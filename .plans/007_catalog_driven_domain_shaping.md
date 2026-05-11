# Plan 007: Catalog-Driven Domain Shaping & API Fidelity

> Distilled from [NVIDIA Dynamo Agentic Inference Lessons](../../microgpt-rs/.research/001_nvidia_dynamo_agentic_lessons.md)

## Context

NVIDIA Dynamo demonstrated that **catalog metadata shapes agent behavior as much as the model itself**. In a 50-task SWE-Bench Verified run, wrong catalog metadata caused 50% fewer tool calls (21.0 vs 41.7 per task). Truncation policy (`tokens` vs `bytes`) changed what the model could inspect after failures. Reasoning settings, system prompts, and tool availability all derive from the catalog record.

anyrag already has `domains.toml`-driven routing with keywords and pruners. This plan extends domain configuration to include the production metadata that Dynamo showed matters: truncation policy, reasoning retention, model metadata endpoints, and token counting.

## Tasks

- [x] T1: Add truncation policy to domain config (`mode`: tokens|bytes, `limit`: u32)
- [x] T2: Add reasoning retention policy to domain config (`keep_on_tool_calls`: bool, `keep_on_plain`: bool)
- [x] T3: Add `/v1/models/{domain}` endpoint — returns domain expert metadata (keywords, truncation, reasoning, tools)
- [x] T4: Add `/v1/tokenize` endpoint — wraps existing tokenizer for pre-request token counting
- [x] T5: Add `/v1/detokenize` endpoint — inverse of tokenize
- [ ] T6: Ensure stable prompt prefix — no per-request metadata at position zero that would poison KV cache
- [x] T7: Update README.md with catalog-driven shaping section

## Design Notes

### Dynamo Lesson: Catalog Shapes Behavior

Dynamo's finding: two endpoints serving the **same model** produced **different agent behavior** because Codex attached different catalog metadata. The request schema was identical, but:
- Truncation: `tokens` mode (10K tokens) preserved more context than `bytes` mode (10K bytes)
- Reasoning: catalog-derived reasoning settings enabled/disabled encrypted_content replay
- System prompt: fallback profile used generic instructions, catalog profile used model-specific instructions

For anyrag, each domain is effectively a "catalog entry" that shapes how microgpt-rs uses that expert.

### Domain Config Extension

```toml
[[domain]]
name = "py2rs"
keywords = ["python", "rewrite", "translate"]
pruner = "syn_validator"

# NEW: Truncation policy (Dynamo lesson)
[domain.truncation]
mode = "tokens"    # "tokens" or "bytes"
limit = 10000

# NEW: Reasoning retention (Dynamo lesson)
[domain.reasoning]
keep_on_tool_calls = true   # preserve reasoning behind tool calls
keep_on_plain = false       # drop reasoning on ordinary turns

# NEW: Agent hints (Dynamo lesson)
[domain.hints]
latency_sensitivity = 0.8   # interactive domain
speculative_prefill = true  # enable prompt compression
```

### `/v1/models/{domain}` Endpoint

Returns metadata for a domain expert. This is what Dynamo's `GET /v1/models/{model_id}` does:

```json
{
  "id": "py2rs",
  "name": "Python to Rust Translation",
  "keywords": ["python", "rewrite", "translate"],
  "truncation": { "mode": "tokens", "limit": 10000 },
  "reasoning": { "keep_on_tool_calls": true, "keep_on_plain": false },
  "tools": ["pruner"],
  "context_window": 8192
}
```

### `/v1/tokenize` Endpoint

Wraps the existing tokenizer. Harnesses (microgpt-rs, future agents) use this for context accounting — deciding when to compact conversation before exceeding model window.

```json
POST /v1/tokenize
{ "text": "fn validate_token(" }
→ { "token_count": 4, "tokens": [123, 456, 789, 12] }
```

### Stable Prompt Prefix (Dynamo Lesson #1)

Dynamo showed that varying content at position zero causes 5× TTFT penalty. For anyrag:
- System prompts for each domain should start with a **stable prefix** (domain name, version, fixed instructions)
- Per-request metadata (user ID, session ID) goes at the END, not the beginning
- This preserves KV cache reuse when the same domain expert handles multiple requests

### Embedding Router Validation

Plan 024 (microgpt-rs) uses `POST /search/embedding` for KV cache priming. Dynamo's work validates this pattern — but we should measure:
- Do relevant embeddings actually improve draft acceptance rate?
- What's the latency impact of the embedding lookup vs the TTFT improvement?
- Is the three-tier fallback (embedding → classify → keyword) exercised in practice?

## Scope

- **In scope:** Domain config extensions, API endpoints, stable prefix enforcement
- **Out of scope:** Changing embedding search algorithm, new domain types, microgpt-rs integration (that's Plan 029)

## Dependencies

- microgpt-rs Plan 029 (consumer of domain metadata)
- riir-ai Plan 002 (validator events feed into domain shaping)

## Success Criteria

1. Domain config includes truncation, reasoning, and hints fields
2. `/v1/models/{domain}` returns accurate metadata
3. `/v1/tokenize` returns correct token counts
4. No per-request metadata at prompt position zero
5. All existing tests pass unchanged
