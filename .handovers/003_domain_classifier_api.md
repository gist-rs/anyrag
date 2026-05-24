# Handover 003: Domain Classifier API (Plan 005)

## What Happened

Implemented Tasks 5, 6, and 7 of Plan 005 (Domain Classifier API). Tasks 1–4 were already implemented in a prior session:

- **Tasks 1–4 (pre-existing):** `DomainDefinition`, `ClassificationResult`, `ClassifyError` types; `DomainClassifier` trait with `async_trait`; `HybridClassifier` with keyword/embedding hybrid scoring; `POST /classify/domain` handler with embedding scoring against slot documents.

- **Task 5 (domain mapping config):** Added `DomainMapping` struct and `default_domain_mappings()` to `crates/lib/src/types.rs`. Default mappings match `katgpt-rs/domains.toml` (sudoku, pathfinding, rust_code, py2rs, general). Added `domain_mappings: Vec<DomainMapping>` field to `AppConfig` with serde default. Updated handler to fall back to config defaults when `candidate_domains` is empty.

- **Task 6 (integration tests):** Created `crates/server/tests/classify_test.rs` with 7 integration tests + 3 unit tests in the handler module. All 10 tests pass.

- **Task 7 (katgpt-rs docs):** Added comprehensive module-level documentation to `crates/server/src/handlers/classify.rs` explaining: how to configure `domains.toml`, how to call `/classify/domain` from katgpt-rs REST client, and fallback behavior when anyrag is unavailable.

## Where Is the Plan/Code/Test

- **Plan:** `.plans/005_domain_classifier_api.md` — status updated to COMPLETE
- **Code:**
  - `crates/lib/src/types.rs` — `DomainMapping` struct + `default_domain_mappings()` + `domain_mappings` field in `AppConfig`
  - `crates/server/src/handlers/classify.rs` — updated handler with `resolve_candidate_domains()` fallback + katgpt-rs integration docs + unit tests
  - `crates/server/Cargo.toml` — `[[test]]` entry for `classify_test`
- **Tests:**
  - `crates/server/tests/classify_test.rs` — 7 integration tests
  - `crates/server/src/handlers/classify.rs` — 3 unit tests (`#[cfg(test)] mod tests`)
  - `crates/lib/src/router/hybrid.rs` — 12 pre-existing unit tests

## Reflection Struggling/Solved

- **No struggles.** The existing Tasks 1–4 were well-implemented and provided a clean foundation. The handler's I/O separation (handler does embeddings/vector search, classifier does pure scoring) made it straightforward to add config fallback without touching the classifier logic.
- **Design decision:** Put `DomainMapping` in `crates/lib/src/types.rs` (not server-only) because it's part of `AppConfig` which is defined in the lib crate. The handler converts `DomainMapping` → `DomainDefinition` via `resolve_candidate_domains()`.
- **Test approach:** Tests send `candidate_domains` with `slots: []` to force keyword-only mode (no embedding mock needed). One test verifies embedding fallback works when mock server has no matching route.

## Remain Work

- Plan 005 is **complete** — all 7 tasks done.
- Plan 004 (Raven Routed Slots) still has unchecked tasks (5.8–5.10, 6.6–6.7, 7.1–7.3 benchmarks). The slot system infrastructure works but integration tests and benchmarks are incomplete.
- katgpt-rs Plan 023 (KeywordRouter V1) is the working fallback. When Plan 004 is validated, the embedding scoring in this classifier will use real slot document embeddings.

## Issues Ref

- No new issues created.

## How to Dev/Test

```bash
# Build check
cargo check --package anyrag --package anyrag-server

# Run all classifier integration tests
cargo test -p anyrag-server --test classify_test

# Run handler unit tests
cargo test -p anyrag-server --lib classify

# Run hybrid classifier unit tests
cargo test -p anyrag --lib hybrid

# Run all tests together
cargo test -p anyrag-server --test classify_test && cargo test -p anyrag-server --lib classify && cargo test -p anyrag --lib hybrid
```

### Adding new domains

1. Add `[[domain_mapping]]` entries to `config.yml`, or rely on defaults in `default_domain_mappings()`.
2. Domain names must match `katgpt-rs/domains.toml` for the two services to share vocabulary.
3. `slots` field maps to anyrag slot names (used for embedding scoring). `keywords` are for keyword overlap scoring.

### Testing with real embeddings

Set up a local embedding service and configure `embedding.api_url` in `config.yml`. The handler will attempt embedding scoring against slot documents and fall back to keyword-only if the service is unavailable.