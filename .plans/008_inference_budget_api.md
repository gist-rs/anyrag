# Plan 008: Inference Budget API — Serve Domain Compute Parameters

> **Status: COMPLETE**
> **Cross-Ref:** `riir-ai/.plans/026_autotts_dynamic_inference_budget.md` (consumer), `anyrag/.plans/007_catalog_driven_domain_shaping.md` (sibling)
> **Research:** `microgpt-rs/.research/16_AutoTTS_Dynamic_Test_Time_Scaling.md`

**Branch:** `develop/feature/008_inference_budget_api`
**Depends on:** Plan 005 (Domain Classifier API — ✅ Complete)

---

## Summary

Extend `anyrag` to serve per-domain inference budget parameters alongside domain classification. When `microgpt-rs` asks "what domain is this prompt?", anyrag answers "py2rs" **and** tells it how much compute to spend: `tree_budget=5000, draft_lookahead=12, screening_threshold=0.3`.

This is the **online API** counterpart to `riir-ai` Plan 026's offline TOML approach. Together they provide two deployment modes:

| Mode | Config Source | Use Case |
|---|---|---|
| **Offline TOML** (Plan 026) | `riir-router` reads `domains.toml` from disk | Single-node, low-latency, no network |
| **Online API** (this plan) | `microgpt-rs` calls `anyrag /classify/domain` | SaaS, multi-tenant, dynamic config |

### Why anyrag Needs This

Currently the `/classify/domain` response is:

```json
{ "domain": "py2rs", "confidence": 0.92, "matched_slots": ["apis", "types"], "alternatives": [...] }
```

The consumer (`riir-router`) knows **which** domain, but not **how** to configure inference for that domain. It falls back to `Config::draft()` defaults for every domain.

AutoTTS showed this wastes compute. A `sudoku` domain needs `tree_budget=100` (pruner dominates). A `py2rs` domain needs `tree_budget=5000` (complex async/lifetime translations). The signal is already in the domain name — we just need to carry the budget metadata through the API.

---

## Architecture

### Extended `/classify/domain` Response

```json
{
  "domain": "py2rs",
  "confidence": 0.92,
  "matched_slots": ["apis", "types"],
  "inference": {
    "tree_budget": 5000,
    "draft_lookahead": 12,
    "screening_threshold": 0.3
  },
  "alternatives": [
    { "domain": "rust_code", "confidence": 0.67, "inference": { "tree_budget": 3000 } }
  ]
}
```

`inference` is optional — domains without explicit budget return `null`, and the consumer uses its defaults.

### `/v1/models/{domain}` Endpoint (Coordinates with Plan 007)

When Plan 007 is implemented, the `/v1/models/{domain}` endpoint will also serve inference budget:

```json
{
  "id": "py2rs",
  "keywords": ["python", "rewrite", "translate"],
  "truncation": { "mode": "tokens", "limit": 10000 },
  "inference": { "tree_budget": 5000, "draft_lookahead": 12 }
}
```

Both endpoints read from the same `DomainMapping.inference` config field.

---

## Tasks

### Phase 1: Types & Config

- [x] **Task 1: Add `InferenceBudget` struct** (`crates/lib/src/router/types.rs`)
  - Optional fields: `tree_budget: Option<usize>`, `draft_lookahead: Option<usize>`, `screening_threshold: Option<f32>`, `temperature: Option<f32>`, `beta: Option<f32>`
  - `#[serde(default)]` on all fields — domains without budget get `None`
  - `#[derive(Debug, Clone, Serialize, Deserialize)]`
  - Unit tests: serde round-trip, `None` defaults, partial fields

- [x] **Task 2: Add `inference` field to `DomainMapping`** (`crates/lib/src/types.rs`)
  - `pub inference: Option<InferenceBudget>` with `#[serde(default)]`
  - Re-export `InferenceBudget` from `crate::router::types`
  - Backward compatible — existing configs without `[domain.inference]` work unchanged
  - Update `default_domain_mappings()` with inference budgets:
    - `sudoku`: `{ tree_budget: 100 }` (constrained, pruner dominates)
    - `pathfinding`: `{ tree_budget: 1000 }` (medium search)
    - `rust_code`: `{ tree_budget: 3000, draft_lookahead: 10 }`
    - `py2rs`: `{ tree_budget: 5000, draft_lookahead: 12, screening_threshold: 0.3 }`
    - `general`: `None` (consumer uses defaults)
  - Unit tests: mapping with budget, mapping without, config TOML parse

### Phase 2: API Response Extension

- [x] **Task 3: Add `inference` to `ClassificationResult`** (`crates/lib/src/router/types.rs`)
  - `pub inference: Option<InferenceBudget>` field
  - `#[serde(skip_serializing_if = "Option::is_none")]` — clean JSON when absent
  - Backward compatible — existing clients ignore unknown fields

- [x] **Task 4: Add `inference` to `DomainScore`** (`crates/lib/src/router/types.rs`)
  - `pub inference: Option<InferenceBudget>` field
  - Each alternative domain carries its own budget parameters

- [x] **Task 5: Wire budget through classify handler** (`crates/server/src/handlers/classify.rs`)
  - When building `ClassificationResult`, look up the winning domain's `inference` from config
  - For alternatives, look up each domain's `inference` from config
  - `resolve_candidate_domains()` already converts `DomainMapping` → `DomainDefinition`; extend to carry `inference`
  - Integration test: classify returns inference budget for matching domain

### Phase 3: β Parameterization (Optional Convenience)

- [x] **Task 6: Add `InferenceBudget::resolve()` method** (`crates/lib/src/router/types.rs`)
  - If `beta` is set and explicit fields are `None`, derive from beta
  - If explicit fields are set, use them (ignore beta)
  - Same monotonic mapping as `riir-ai` Plan 026:
    ```rust
    impl InferenceBudget {
        pub fn resolve(&self) -> InferenceBudget {
            match self.beta {
                Some(beta) if self.tree_budget.is_none() => InferenceBudget::from_beta(beta),
                _ => self.clone(),
            }
        }
    }
    ```
  - This lets TOML authors write either explicit values or `beta = 0.8`

### Phase 4: Docs & Tests

- [x] **Task 7: Integration tests** (`crates/server/tests/classify_test.rs`)
  - Test: classify "translate FastAPI to Axum" → domain "py2rs" with `inference.tree_budget == 5000`
  - Test: classify "solve this sudoku" → domain "sudoku" with `inference.tree_budget == 100`
  - Test: classify "hello world" → domain "general" with `inference == None`
  - Test: alternatives carry their own inference budgets
  - Test: TOML with `beta = 0.8` resolves to correct explicit values

- [x] **Task 8: Update README.md**
  - Add `Inference Budget API` section
  - Show extended `/classify/domain` response with inference field
  - Document TOML config format: explicit values vs `beta`
  - Cross-reference `riir-ai` Plan 026

---

## File Changes

| File | Action | Description |
|---|---|---|
| `crates/lib/src/router/types.rs` | Edit | Add `InferenceBudget` struct, `inference` on `ClassificationResult` and `DomainScore` |
| `crates/lib/src/types.rs` | Edit | Add `inference: Option<InferenceBudget>` to `DomainMapping`, update defaults |
| `crates/server/src/handlers/classify.rs` | Edit | Wire budget from config into response |
| `crates/server/tests/classify_test.rs` | Edit | Add inference budget assertions |
| `README.md` | Edit | Add inference budget API section |

---

## TOML Config Examples

### Explicit Values

```toml
[[domain_mapping]]
domain = "py2rs"
slots = ["apis", "types"]
keywords = ["python", "rewrite", "fastapi", "flask", "translate"]

[domain_mapping.inference]
tree_budget = 5000
draft_lookahead = 12
screening_threshold = 0.3
```

### β Shorthand

```toml
[[domain_mapping]]
domain = "py2rs"
slots = ["apis", "types"]
keywords = ["python", "rewrite", "fastapi", "flask", "translate"]

[domain_mapping.inference]
beta = 0.8
```

### No Budget (Consumer Uses Defaults)

```toml
[[domain_mapping]]
domain = "general"
slots = []
keywords = []
# No [domain_mapping.inference] → inference = null in API response
```

---

## Design Decisions

1. **`Option<InferenceBudget>` everywhere** — `None` means "consumer decides". This avoids anyrag needing to know microgpt-rs defaults. The consumer (`riir-router`) merges: `null` from API → use local `Config::draft()` defaults.

2. **Same struct in response and config** — `InferenceBudget` is both the config type (TOML) and the API type (JSON). No mapping layer needed.

3. **`beta` is a convenience, not required** — domains can specify exact values OR a single scalar. The `resolve()` method expands beta to explicit values before serving.

4. **No breaking change** — `inference` is a new optional field on existing types. Clients that don't read it are unaffected.

5. **Independent of Plan 007** — this plan adds inference budget to `/classify/domain`. Plan 007 adds it to `/v1/models/{domain}`. Both read from the same config. No dependency ordering required.

---

## Cross-Project Coordination

| Project | Plan | Relationship |
|---|---|---|
| `riir-ai` | Plan 026 (AutoTTS Dynamic Budget) | **Consumer.** `riir-router` calls `/classify/domain`, reads `inference` field, passes to `Config::with_overrides()` |
| `anyrag` | Plan 005 (Domain Classifier) | **Foundation.** Built on — this plan extends its response |
| `anyrag` | Plan 007 (Catalog-Driven Shaping) | **Sibling.** Will serve same inference budget via `/v1/models/{domain}` |
| `microgpt-rs` | Plan 021 (ScreeningPruner) | **Verifier.** `screening_threshold` from budget controls pruning aggressiveness |
| `microgpt-rs` | `.research/16_AutoTTS_...` | **Research.** Distilled rationale for dynamic budget |

### Offline vs Online Deployment

```
Offline (Plan 026 only):
  riir-router reads domains.toml → InferenceBudget → Config::with_overrides()
  No network call. Budget is static until restart.

Online (Plan 026 + this plan):
  microgpt-rs → anyrag /classify/domain → { domain, inference }
  riir-router reads inference from API response → Config::with_overrides()
  Budget can change without restart (update anyrag config + reload).

Hybrid (both):
  riir-router has local domains.toml (fallback)
  anyrag serves inference budget (preferred when available)
  Three-tier: API budget → TOML budget → Config defaults
```

---

## Expected Outcomes

| Metric | Before | After |
|---|---|---|
| `/classify/domain` response | `{ domain, confidence, slots }` | `{ domain, confidence, slots, inference }` |
| Budget for "solve sudoku" | N/A (consumer decides) | `{ tree_budget: 100 }` |
| Budget for "translate FastAPI" | N/A (consumer decides) | `{ tree_budget: 5000, lookahead: 12 }` |
| Config format | Keywords + slots only | Keywords + slots + inference |
| Breaking change | N/A | None — optional field |

---

## Success Criteria

- [ ] `InferenceBudget` serde round-trips correctly
- [ ] `/classify/domain` returns `inference` for domains with configured budget
- [ ] `/classify/domain` returns `inference: null` for domains without budget
- [ ] Alternatives carry their own inference budgets
- [ ] `beta = 0.8` in TOML resolves to correct explicit values in API response
- [ ] Existing classify tests pass unchanged
- [ ] Default domain mappings include inference budgets for known domains

---

## Out of Scope

- Consumer-side wiring (that's `riir-ai` Plan 026)
- `/v1/models/{domain}` endpoint (that's `anyrag` Plan 007)
- Adaptive budget learning (future — bandit could learn optimal β over time)
- Budget negotiation protocol (consumer accepting/rejecting suggested budget)
- Per-request budget overrides (API consumer specifies budget, not anyrag)