//! Self-Improving Cycle — 32-Day Runtime LoRA Pipeline Orchestrator

use crate::types::EpisodicStats;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::info;

/// Configuration for the self-improving cycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleConfig {
    /// Minimum episodes before synthesis triggers. Default: 100
    pub min_episodes_for_synthesis: usize,
    /// Minimum success rate to trigger export. Default: 0.85
    pub min_success_rate: f64,
    /// Path to write training JSONL. Default: "exports/training.jsonl"
    pub export_path: String,
    /// microgpt-rs API URL for hot-reload trigger.
    pub model_api_url: Option<String>,
}

impl Default for CycleConfig {
    fn default() -> Self {
        Self {
            min_episodes_for_synthesis: 100,
            min_success_rate: 0.85,
            export_path: "exports/training.jsonl".to_string(),
            model_api_url: None,
        }
    }
}

/// State machine for the 32-day self-improving cycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CycleState {
    /// Day 1-29: recording episodes
    Collecting,
    /// Enough episodes collected, ready for LLM synthesis
    ReadyToSynthesize,
    /// Running LLM synthesis
    Synthesizing,
    /// Synthesis complete, ready for export
    ReadyToExport,
    /// Generating JSONL
    Exporting,
    /// Waiting for LoRA training to complete
    Training,
    /// Hot-reloading trained LoRA
    Upgrading,
}

/// Snapshot of the cycle for API responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleStatus {
    pub state: CycleState,
    pub stats: Option<EpisodicStats>,
    pub config: CycleConfig,
    pub last_action: Option<String>,
}

/// Actions returned by the cycle state machine for the caller to perform.
#[derive(Debug)]
pub enum CycleAction {
    BeginSynthesis,
    SynthesisComplete { pairs_count: usize },
    ExportComplete { path: String, lines: usize },
    TrainingComplete { lora_path: String },
    CycleComplete,
}

/// Orchestrator for the self-improving 32-day cycle.
///
/// Manages transitions: Collecting → Synthesizing → Exporting → Training → Upgrading.
/// The orchestrator only advances the state machine; the caller is responsible for
/// performing the actual synthesis, export, and training work.
pub struct SelfImprovingCycle {
    pub config: CycleConfig,
    pub state: CycleState,
    last_action: Option<String>,
}

impl SelfImprovingCycle {
    /// Create a new cycle orchestrator with the given configuration.
    pub fn new(config: CycleConfig) -> Self {
        Self {
            config,
            state: CycleState::Collecting,
            last_action: None,
        }
    }

    /// Get the current cycle status.
    pub fn status(&self, stats: Option<EpisodicStats>) -> CycleStatus {
        CycleStatus {
            state: self.state.clone(),
            stats,
            config: self.config.clone(),
            last_action: self.last_action.clone(),
        }
    }

    /// Check if cycle should advance to next state.
    /// Returns `Some(CycleAction)` if a state transition occurred.
    /// This is the "tick" of the cycle — it should be called periodically.
    ///
    /// Note: The actual synthesis/export work is NOT done here.
    /// This method only advances the state machine. The caller is responsible
    /// for performing the actual work when an action is returned.
    pub fn tick(&mut self, stats: &EpisodicStats) -> Result<Option<CycleAction>> {
        match self.state {
            CycleState::Collecting => {
                if stats.total_episodes >= self.config.min_episodes_for_synthesis as u64
                    && stats.success_rate >= self.config.min_success_rate
                {
                    info!(
                        "Cycle advancing: Collecting → ReadyToSynthesize ({} episodes, {:.1}% success)",
                        stats.total_episodes,
                        stats.success_rate * 100.0
                    );
                    self.state = CycleState::ReadyToSynthesize;
                    self.last_action = Some("Threshold reached".to_string());
                    return Ok(Some(CycleAction::BeginSynthesis));
                }
                Ok(None)
            }
            CycleState::Synthesizing => {
                // Caller sets this state before calling tick again
                Ok(None)
            }
            CycleState::Exporting => {
                // Caller sets this state before calling tick again
                Ok(None)
            }
            CycleState::Training => {
                // Waiting for external training to complete
                Ok(None)
            }
            CycleState::Upgrading => {
                // Caller sets this state before calling tick again
                Ok(None)
            }
            CycleState::ReadyToSynthesize | CycleState::ReadyToExport => {
                // These are transitional states; caller should perform work
                // and then call advance()
                Ok(None)
            }
        }
    }

    /// Manually advance to a specific state (for external triggers).
    pub fn advance_to(&mut self, state: CycleState, action_desc: &str) {
        info!(
            "Cycle advancing: {:?} → {:?} ({})",
            self.state, state, action_desc
        );
        self.state = state;
        self.last_action = Some(action_desc.to_string());
    }

    /// Complete the full cycle and reset to Collecting.
    pub fn reset(&mut self) {
        info!("Cycle complete. Resetting to Collecting.");
        self.state = CycleState::Collecting;
        self.last_action = Some("Cycle complete, reset".to_string());
    }
}
