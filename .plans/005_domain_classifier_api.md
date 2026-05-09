# Plan 005: Domain Classifier API — Embedding-Based Routing for microgpt-rs

> **Status: BLOCKED** — Depends on Plan 004 (Raven Routed Slots) integration tests (tasks 5.8–5.10, 6.6–6.7) and benchmarks (7.1–7.3). The slot system infrastructure exists but is not fully validated. Do not start this plan until Plan 004's unchecked tasks are complete. `microgpt-rs` Plan 023 (KeywordRouter V1) is the working fallback.

**Branch:** `develop/feature/005_domain_classifier`
**Depends on:** Plan 004 (Raven Routed Slots — keyword router + slot system)
**Cross-Ref:** `microgpt-rs/.plans/023_prompt_router.md` (consumer of this API)

---

## Summary

Expose anyrag's slot-based semantic routing as a `DomainClassifier` API that `microgpt-rs` can query to upgrade from keyword-based routing to embedding-based domain classification. This is the V2 router backend — `microgpt-rs` sends a prompt, anyrag returns a domain classification with confidence scores.

The keyword router in microgpt-rs Plan 023 is ~80% accurate. This API targets ~95% by using anyrag's existing vector embeddings + slot keyword overlap to classify prompts into the same domains defined in `domains.toml`.

---

## Architecture

### API Endpoint

```text
POST /classify/domain
Body:  { "prompt": "Rewrite this FastAPI endpoint to Axum", "candidate_domains": ["rust_code", "py2rs", "sudoku", "pathfinding", "general"] }
Resp:  { "domain": "py2rs", "confidence": 0.92, "matched_slots": ["apis", "types"], "alternatives": [{"domain": "rust_code", "confidence": 0.67}] }
```

### Classification Pipeline

```text
Prompt
  ↓
1. Embed prompt via configured AI provider (Gemini/local)
  ↓
2. Search all slots for top-K similar documents
  ↓
3. Score each candidate_domain:
     score = slot_keyword_overlap(prompt, domain.keywords) * 0.3
           + embedding_similarity(prompt, slot_documents) * 0.7
  ↓
4. Return domain with highest score + alternatives
```

### DomainClassifier Trait

```rust
// crates/lib/src/router/classifier.rs

/// Classifies a prompt into a domain using semantic search + keyword overlap.
#[async_trait]
pub trait DomainClassifier: Send + Sync {
    /// Classify a prompt against candidate domains.
    /// Returns ranked domain scores.
    async fn classify(
        &self,
        prompt: &str,
        candidate_domains: &[DomainDefinition],
    ) -> Result<ClassificationResult, ClassifyError>;
}

pub struct DomainDefinition {
    pub name: String,
    pub keywords: Vec<String>,
}

pub struct ClassificationResult {
    pub domain: String,
    pub confidence: f32,
    pub matched_slots: Vec<String>,
    pub alternatives: Vec<DomainScore>,
}

pub struct DomainScore {
    pub domain: String,
    pub confidence: f32,
}
```

### Weighted Scoring

The classifier blends two signals:

1. **Keyword overlap (30% weight):** Same keyword matching as Plan 004's `KeywordRouter`, but applied to the prompt against domain keywords.
2. **Embedding similarity (70% weight):** Prompt embedding compared against document embeddings in each slot. Slots that have high similarity to the prompt indicate the prompt belongs to that slot's domain.

```rust
fn compute_domain_score(
    keyword_score: f32,    // [0.0, 1.0] from keyword overlap
    embedding_score: f32,  // [0.0, 1.0] from vector similarity
) -> f32 {
    keyword_score * 0.3 + embedding_score * 0.7
}
```

### Slot → Domain Mapping

The mapping between anyrag slots and microgpt-rs domains is configured in anyrag's config:

```toml
[[domain_mapping]]
domain = "rust_code"
slots = ["apis", "types", "architecture"]
keywords = ["rust", "cargo", "axum", "tokio", "trait", "impl", "compile"]

[[domain_mapping]]
domain = "py2rs"
slots = ["apis", "types"]
keywords = ["python", "rewrite", "fastapi", "flask", "translate"]

[[domain_mapping]]
domain = "sudoku"
slots = ["tests"]
keywords = ["sudoku", "puzzle", "grid", "9x9"]
```

---

## Tasks

### Phase 1: Types & Trait

- [ ] **Task 1: Add classification types** (`crates/lib/src/router/types.rs`)
  - `DomainDefinition`, `ClassificationResult`, `DomainScore`, `ClassifyError`
  - Shared between classifier impl and API handler

- [ ] **Task 2: Define `DomainClassifier` trait** (`crates/lib/src/router/classifier.rs`)
  - Async trait with `classify()` method
  - `Send + Sync` for use in axum handlers

### Phase 2: Keyword + Embedding Hybrid Classifier

- [ ] **Task 3: Implement `HybridClassifier`** (`crates/lib/src/router/hybrid.rs`)
  - Combines keyword overlap (from Plan 004's `KeywordRouter`) with embedding similarity
  - Keyword scoring: reuse `KeywordRouter` logic
  - Embedding scoring: use existing vector search against slot documents
  - Weighted blend: 30% keyword + 70% embedding
  - Falls back to keyword-only if AI provider is unavailable

### Phase 3: REST API Endpoint

- [ ] **Task 4: Add `POST /classify/domain` endpoint** (`crates/server/src/handlers/classify.rs`)
  - Request: `{ prompt, candidate_domains }`
  - Response: `{ domain, confidence, matched_slots, alternatives }`
  - Calls `HybridClassifier::classify()` through state
  - Error handling: provider unavailable → fall back to keyword-only

- [ ] **Task 5: Add domain mapping config** (`crates/server/src/config.rs` or config file)
  - `DomainMapping` struct with domain name, slots, keywords
  - Loaded from config file at startup
  - Used to build `DomainDefinition` list for classifier

### Phase 4: Integration Test

- [ ] **Task 6: Integration test**
  - Test: classify "solve this sudoku" → domain "sudoku"
  - Test: classify "write Rust HTTP server" → domain "rust_code"
  - Test: classify "translate FastAPI to Axum" → domain "py2rs"
  - Test: fallback to keyword-only when AI provider unavailable
  - Test: confidence scores are reasonable (best > 0.5, worst < 0.3)

### Phase 5: microgpt-rs Integration Docs

- [ ] **Task 7: Document microgpt-rs integration** (`README.md` or `HOW.md`)
  - How to configure `domains.toml` to match anyrag's `domain_mapping`
  - How to call `/classify/domain` from microgpt-rs REST client
  - Fallback behavior when anyrag is unavailable

---

## Key Risks & Mitigations

| Risk | Mitigation |
|------|-----------|
| AI provider latency slows classification | Cache recent classifications. Set timeout (200ms). Fall back to keyword-only. |
| Embedding quality insufficient for domain separation | Tune keyword weight up (50/50 split). Add more domain-specific keywords. |
| Slot-document overlap across domains | Use slot-specific documents for scoring, not all documents. Domain mapping is explicit, not inferred. |
| anyrag unavailable | microgpt-rs `KeywordRouter` is the V1 fallback. This API is additive, not required. |

---

## Expected Outcomes

| Metric | Keyword Router (V1) | Hybrid Classifier (V2) |
|--------|---------------------|----------------------|
| Accuracy (obvious domains) | ~80% | ~95% |
| Accuracy (ambiguous prompts) | ~50% | ~80% |
| Latency per classification | <1μs (in-process) | ~50-200ms (API call) |
| Dependency | None | anyrag server running |

---

## Files to Create

| File | Purpose |
|------|---------|
| `crates/lib/src/router/types.rs` | Classification types |
| `crates/lib/src/router/classifier.rs` | `DomainClassifier` trait |
| `crates/lib/src/router/hybrid.rs` | `HybridClassifier` implementation |
| `crates/server/src/handlers/classify.rs` | REST endpoint handler |

## Files to Modify

| File | Change |
|------|--------|
| `crates/lib/src/router/mod.rs` | Add new modules |
| `crates/server/src/routes.rs` (or equivalent) | Add `/classify/domain` route |
| `crates/server/src/state.rs` (or equivalent) | Add classifier to app state |
| `README.md` | Add Domain Classifier API section |

---

## Out of Scope

- Training a custom classifier model (future research)
- Multi-label classification (one prompt → multiple domains)
- Streaming classification (not needed — single response)
- Auto-sync of domain config between microgpt-rs and anyrag (manual for now)

---

## Cross-Project References

| Project | Plan | Relationship |
|---------|------|-------------|
| `microgpt-rs` | `.plans/023_prompt_router.md` | Consumer of this API. `KeywordRouter` is V1, this is V2. |
| `anyrag` | `.plans/004_raven_routed_slots.md` | Slot system + KeywordRouter reused for classification |
| `riir-validator-sdk` | N/A | Curators build validators; this classifies which one to use |