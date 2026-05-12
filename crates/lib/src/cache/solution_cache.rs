//! PUCT-inspired solution cache for TTT feedback loop.
//!
//! Stores high-reward inference results for reuse and export as training data.
//! Uses papaya for lock-free concurrent access (read-heavy pattern).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Cached solution with PUCT-style scoring metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSolution {
    pub query_hash: u64,
    pub domain: String,
    pub output: String,
    pub reward: f32,
    /// Number of times this solution was reused.
    pub reuse_count: usize,
    /// Timestamp of last use.
    pub last_used: i64,
    /// Max reward from children spawned from reusing this solution.
    pub child_max_reward: f32,
}

impl CachedSolution {
    /// PUCT-inspired score: exploit (reward) + explore (under-visited).
    fn puct_score(&self, total_visits: usize, reward_scale: f32, c: f32) -> f32 {
        let q = self.child_max_reward.max(self.reward);
        let exploration =
            c * reward_scale * (1.0 + total_visits as f32).sqrt() / (1.0 + self.reuse_count as f32);
        q + exploration
    }
}

/// Training sample for riir-burner JSONL format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingSample {
    pub instruction: String,
    pub output: String,
    pub reward: f32,
    pub domain: String,
}

/// Inference result from microgpt-rs feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResult {
    pub domain: String,
    pub reward: f32,
    pub tree_budget_used: usize,
    pub budget_level: u8,
    pub prompt_hash: u64,
    pub output: String,
    pub timestamp: i64,
    pub screened: bool,
}

/// Solution cache with PUCT-inspired selection.
pub struct SolutionCache {
    entries: HashMap<u64, CachedSolution>,
    max_entries: usize,
    reward_threshold: f32,
    total_visits: AtomicU64,
}

impl SolutionCache {
    pub fn new(max_entries: usize, reward_threshold: f32) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
            reward_threshold,
            total_visits: AtomicU64::new(0),
        }
    }

    /// Insert an inference result. Only caches if reward > threshold.
    /// Returns true if inserted.
    pub fn insert(&mut self, result: &InferenceResult) -> bool {
        if result.reward < self.reward_threshold {
            return false;
        }

        // Prune if at capacity
        if self.entries.len() >= self.max_entries {
            self.prune();
        }

        let key = result.prompt_hash;
        let solution = CachedSolution {
            query_hash: key,
            domain: result.domain.clone(),
            output: result.output.clone(),
            reward: result.reward,
            reuse_count: 0,
            last_used: result.timestamp,
            child_max_reward: result.reward,
        };

        match self.entries.entry(key) {
            std::collections::hash_map::Entry::Occupied(mut e) => {
                let existing = e.get_mut();
                if result.reward > existing.reward {
                    existing.reward = result.reward;
                    existing.output = result.output.clone();
                }
                existing.last_used = result.timestamp;
                false // Updated, not new insert
            }
            std::collections::hash_map::Entry::Vacant(e) => {
                e.insert(solution);
                true
            }
        }
    }

    /// Lookup a cached solution by query hash and domain.
    pub fn lookup(&self, query_hash: u64, domain: &str) -> Option<&CachedSolution> {
        self.entries.get(&query_hash).filter(|s| s.domain == domain)
    }

    /// Select a solution for reuse using PUCT scoring.
    pub fn select_for_reuse(&mut self, domain: &str) -> Option<&CachedSolution> {
        let total = self.total_visits.load(Ordering::Relaxed) as usize;

        let mut best_key: Option<u64> = None;
        let mut best_score = f32::NEG_INFINITY;

        for (&key, sol) in &self.entries {
            if sol.domain != domain {
                continue;
            }
            let score = sol.puct_score(total, 1.0, 1.414);
            if score > best_score {
                best_score = score;
                best_key = Some(key);
            }
        }

        match best_key {
            Some(key) => {
                self.total_visits.fetch_add(1, Ordering::Relaxed);
                if let Some(sol) = self.entries.get_mut(&key) {
                    sol.reuse_count += 1;
                }
                self.entries.get(&key)
            }
            None => None,
        }
    }

    /// Export cache entries as riir-burner-compatible JSONL.
    pub fn export_jsonl(&self, domain: &str) -> Vec<TrainingSample> {
        self.entries
            .values()
            .filter(|s| s.domain == domain)
            .map(|s| TrainingSample {
                instruction: format!("{:016x}", s.query_hash),
                output: s.output.clone(),
                reward: s.reward,
                domain: s.domain.clone(),
            })
            .collect()
    }

    /// Prune: keep top-K by reward, always keep entries with reuse_count > 0.
    fn prune(&mut self) {
        if self.entries.len() <= self.max_entries / 2 {
            return;
        }

        let keep_count = self.max_entries * 3 / 4; // Keep 75%

        let mut entries: Vec<_> = self.entries.drain().collect();

        // Sort: prefer high reward AND reused entries
        entries.sort_by(|a, b| {
            let score_a = a.1.reward + if a.1.reuse_count > 0 { 1.0 } else { 0.0 };
            let score_b = b.1.reward + if b.1.reuse_count > 0 { 1.0 } else { 0.0 };
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        entries.truncate(keep_count);
        self.entries = entries.into_iter().collect();
    }

    /// Get cache statistics.
    pub fn stats(&self) -> CacheStats {
        let domains: HashMap<String, usize> =
            self.entries.values().fold(HashMap::new(), |mut acc, s| {
                *acc.entry(s.domain.clone()).or_default() += 1;
                acc
            });
        let avg_reward = if self.entries.is_empty() {
            0.0
        } else {
            self.entries.values().map(|s| s.reward).sum::<f32>() / self.entries.len() as f32
        };
        CacheStats {
            entry_count: self.entries.len(),
            max_entries: self.max_entries,
            avg_reward,
            domain_counts: domains,
        }
    }
}

/// Cache statistics for monitoring.
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub entry_count: usize,
    pub max_entries: usize,
    pub avg_reward: f32,
    pub domain_counts: HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(domain: &str, reward: f32, hash: u64) -> InferenceResult {
        InferenceResult {
            domain: domain.to_string(),
            reward,
            tree_budget_used: 100,
            budget_level: 0,
            prompt_hash: hash,
            output: format!("output_{hash}"),
            timestamp: 1000,
            screened: reward < 0.7,
        }
    }

    #[test]
    fn test_insert_below_threshold() {
        let mut cache = SolutionCache::new(100, 0.7);
        let result = make_result("test", 0.5, 1);
        assert!(!cache.insert(&result));
        assert_eq!(cache.entries.len(), 0);
    }

    #[test]
    fn test_insert_above_threshold() {
        let mut cache = SolutionCache::new(100, 0.7);
        let result = make_result("test", 0.9, 1);
        assert!(cache.insert(&result));
        assert_eq!(cache.entries.len(), 1);
    }

    #[test]
    fn test_lookup_by_domain() {
        let mut cache = SolutionCache::new(100, 0.7);
        cache.insert(&make_result("py2rs", 0.9, 42));
        cache.insert(&make_result("other", 0.8, 43));

        assert!(cache.lookup(42, "py2rs").is_some());
        assert!(cache.lookup(42, "other").is_none());
        assert!(cache.lookup(43, "other").is_some());
    }

    #[test]
    fn test_export_jsonl() {
        let mut cache = SolutionCache::new(100, 0.7);
        cache.insert(&make_result("py2rs", 0.9, 1));
        cache.insert(&make_result("py2rs", 0.8, 2));
        cache.insert(&make_result("other", 0.85, 3));

        let samples = cache.export_jsonl("py2rs");
        assert_eq!(samples.len(), 2);
        assert!(samples.iter().all(|s| s.domain == "py2rs"));
    }

    #[test]
    fn test_prune_keeps_reused() {
        let mut cache = SolutionCache::new(4, 0.5);
        // Fill to 3 entries (below capacity, no auto-prune)
        for i in 0..3 {
            cache.insert(&make_result("test", 0.5 + i as f32 * 0.1, i as u64));
        }
        // Mark entry 0 as reused before prune can trigger
        if let Some(s) = cache.entries.get_mut(&0) {
            s.reuse_count = 5;
        }
        // Fill remaining to trigger prune on 5th insert
        cache.insert(&make_result("test", 0.8, 3));
        cache.insert(&make_result("test", 0.9, 4)); // triggers prune
        assert!(cache.entries.len() <= 4);
        // Reused entry should survive pruning
        assert!(cache.entries.contains_key(&0));
    }

    #[test]
    fn test_stats() {
        let mut cache = SolutionCache::new(100, 0.7);
        cache.insert(&make_result("py2rs", 0.9, 1));
        cache.insert(&make_result("py2rs", 0.8, 2));

        let stats = cache.stats();
        assert_eq!(stats.entry_count, 2);
        assert!(stats.avg_reward > 0.8);
        assert_eq!(stats.domain_counts.get("py2rs"), Some(&2));
    }
}
