# Plan 003: Self-Improving Cycle — 32-Day Runtime LoRA Pipeline

## Objective

Implement the full 32-day self-improving loop described in the research: anyrag collects successful RIIR translations (episodic memory), synthesizes patterns into structured training data, exports JSONL, feeds it into 's wgpu LoRA trainer (Plan 008), and hot-reloads the trained adapter back into the inference engine.

## The Problem

Current state:
1. **No episodic memory**: anyrag ingests documents but doesn't track which translations succeeded (compiled) vs failed
2. **Export is FAQ-only**: `/knowledge/export` only exports YAML FAQ pairs, not code translation pairs with hidden states
3. **No auto-synthesis**: The curator deduplicates but doesn't synthesize patterns into training data
4. **No bridge to LoRA trainer**: No pipeline from anyrag export →  Plan 008 training
5. **No hot-reload**: Once `lora.bin` is trained, there's no mechanism to load it into a running  instance

The research describes a 4-phase cycle:
- **Day 1 (RAG Phase)**: Answer queries via retrieval
- **Day 30 (Synthesis)**: Curator synthesizes common patterns into Q&A pairs
- **Day 31 (Export & Fine-tune)**: `/knowledge/export` → JSONL → LoRA training
- **Day 32 (Upgrade)**: Base model upgraded, episodic memory cleared

## Architecture

### Part 1: Episodic Memory

Track translation success/failure alongside retrieved context.

```rust
// crates/lib/src/types.rs — episodic memory types

/// A single RIIR translation episode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationEpisode {
    pub id: String,                          // UUID v7
    pub source_language: String,             // "python", "typescript", etc.
    pub source_code: String,                 // original input code
    pub generated_rust: String,              // LLM-generated Rust code
    pub retrieved_context: Vec<SearchResult>, // what RAG retrieved
    pub hidden_state: Option<Vec<f64>>,      // embedding at generation time
    pub compilation_result: CompilationResult,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompilationResult {
    Success {
        warnings: u32,
        clippy_lints: u32,
    },
    Failed {
        error_message: String,
        error_code: Option<String>,   // e.g., "E0382"
        suggestion: Option<String>,
    },
    NotCompiled,  // generated but not yet verified
}

/// Statistics for episodic memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpisodicStats {
    pub total_episodes: u64,
    pub successful: u64,
    pub failed: u64,
    pub success_rate: f64,
    pub top_error_codes: Vec<(String, u64)>,
}
```

```rust
// crates/server/src/handlers/ — new endpoints

// POST /episodes — record a translation episode
// GET  /episodes — list episodes with filtering
// GET  /episodes/stats — success/failure statistics
// POST /episodes/{id}/verify — update compilation result
```

```rust
// crates/lib/src/ingest/episodic.rs — new ingestion for episodes

pub struct EpisodicIngester;

impl EpisodicIngester {
    /// Record a new translation episode.
    pub async fn record_episode(
        db: &dyn DbProvider,
        episode: TranslationEpisode,
    ) -> Result<()> {
        // Insert into `episodes` table
        // If successful, also insert hidden state into vector index for REST retrieval
    }
    
    /// Update episode with compilation result (called by external verifier).
    pub async fn verify_episode(
        db: &dyn DbProvider,
        episode_id: &str,
        result: CompilationResult,
    ) -> Result<()> {
        // UPDATE episodes SET compilation_result = ? WHERE id = ?
    }
    
    /// Get episodes for synthesis (successful translations only).
    pub async fn get_successful_episodes(
        db: &dyn DbProvider,
        limit: usize,
        since: Option<NaiveDateTime>,
    ) -> Result<Vec<TranslationEpisode>> {
        // SELECT * FROM episodes WHERE compilation_result = 'Success' ORDER BY created_at DESC
    }
}
```

### Part 2: Training Data Synthesis

```rust
// crates/lib/src/curator.rs — extend curator for training synthesis

impl Curator {
    /// Synthesize successful translation episodes into training pairs.
    /// This is the "Day 30" step — turning episodic memory into fine-tuning data.
    pub async fn synthesize_training_data(
        db: &dyn DbProvider,
        ai_provider: &dyn AiProvider,
        config: &SynthesisConfig,
    ) -> Result<TrainingDataset> {
        // 1. Fetch successful episodes since last synthesis
        let episodes = EpisodicIngester::get_successful_episodes(
            db, config.batch_size, config.last_synthesis,
        ).await?;
        
        // 2. Group by pattern (similar source → similar generated code)
        let groups = Self::group_by_pattern(&episodes);
        
        // 3. For each group, use LLM to create canonical Q&A pair
        let mut training_pairs = Vec::new();
        for group in groups {
            let synthesis = ai_provider.generate(&format!(
                "Synthesize these {} successful Rust translations into one canonical example.\n\
                 Input patterns:\n{}\n\
                 Output as JSONL: {{\"messages\":[{{\"role\":\"system\",\"content\":\"...\"}},...]}}",
                group.len(),
                group.iter().map(|e| format!("---\n{}\n→\n{}", e.source_code, e.generated_rust))
                    .collect::<Vec<_>>().join("\n"),
            )).await?;
            training_pairs.push(synthesis);
        }
        
        Ok(TrainingDataset {
            pairs: training_pairs,
            episode_count: episodes.len(),
            synthesized_at: chrono::Utc::now().naive_utc(),
        })
    }
}
```

### Part 3: Extended Export

```rust
// crates/lib/src/ingest/knowledge.rs — extend export

impl KnowledgeExporter {
    /// Export training data for LoRA fine-tuning.
    /// Extends /knowledge/export to include:
    /// 1. FAQ pairs (existing)
    /// 2. Successful translation episodes (new)
    /// 3. Hidden state vectors for REST retrieval (new)
    pub async fn export_for_lora(
        db: &dyn DbProvider,
        config: &ExportConfig,
    ) -> Result<LoraExport> {
        let mut jsonl_lines = Vec::new();
        
        // 1. FAQ pairs (existing logic)
        let faq_jsonl = Self::export_for_finetuning(db).await?;
        jsonl_lines.extend(faq_jsonl.lines().map(String::from));
        
        // 2. Successful translation episodes
        let episodes = EpisodicIngester::get_successful_episodes(
            db, config.max_episodes, config.since,
        ).await?;
        
        for episode in &episodes {
            let jsonl = serde_json::json!({
                "messages": [
                    {"role": "system", "content": "Rewrite the following code in idiomatic Rust."},
                    {"role": "user", "content": &episode.source_code},
                    {"role": "assistant", "content": &episode.generated_rust},
                ]
            }).to_string();
            jsonl_lines.push(jsonl);
        }
        
        // 3. Hidden states (separate file for REST index)
        let hidden_states: Vec<(String, Vec<f64>)> = episodes.iter()
            .filter_map(|e| e.hidden_state.as_ref().map(|hs| (e.id.clone(), hs.clone())))
            .collect();
        
        Ok(LoraExport {
            training_jsonl: jsonl_lines.join("\n"),
            hidden_states_json: serde_json::to_string(&hidden_states)?,
            stats: ExportStats {
                faq_pairs: /* count */,
                translation_pairs: episodes.len(),
                total_tokens: /* estimate */,
            },
        })
    }
}
```

### Part 4: The 32-Day Cycle Orchestrator

```rust
// crates/lib/src/cycle.rs — new module

/// The self-improving cycle orchestrator.
/// Runs as a background task in the server.
pub struct SelfImprovingCycle {
    db: Box<dyn DbProvider>,
    ai: Box<dyn AiProvider>,
    config: CycleConfig,
    state: CycleState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleConfig {
    /// Minimum episodes before synthesis triggers.
    pub min_episodes_for_synthesis: usize,  // default: 100
    /// Minimum success rate to trigger export.
    pub min_success_rate: f64,              // default: 0.85
    /// Minimum days between synthesis runs.
    pub synthesis_interval_days: u32,       // default: 30
    /// Path to write training JSONL.
    pub export_path: String,               // default: "exports/training.jsonl"
    ///  API URL for hot-reload trigger.
    pub model_api_url: Option<String>,      // e.g., "http://localhost:8080"
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CycleState {
    Collecting,    // Day 1-29: recording episodes
    ReadyToSynthesize, // Day 30: enough episodes collected
    Synthesizing,  // Running LLM synthesis
    ReadyToExport, // Synthesis complete, ready for export
    Exporting,     // Generating JSONL
    Training,      // Waiting for LoRA training to complete
    Upgrading,     // Day 32: hot-reloading trained LoRA
}

impl SelfImprovingCycle {
    /// Check if cycle should advance to next state.
    pub async fn tick(&mut self) -> Result<Option<CycleAction>> {
        match self.state {
            CycleState::Collecting => {
                let stats = EpisodicIngester::get_stats(&*self.db).await?;
                if stats.total_episodes >= self.config.min_episodes_for_synthesis as u64
                    && stats.success_rate >= self.config.min_success_rate {
                    self.state = CycleState::ReadyToSynthesize;
                    return Ok(Some(CycleAction::BeginSynthesis));
                }
                Ok(None)
            }
            CycleState::ReadyToSynthesize => {
                // Trigger synthesis
                let dataset = Curator::synthesize_training_data(
                    &*self.db, &*self.ai, &Default::default(),
                ).await?;
                self.state = CycleState::ReadyToExport;
                Ok(Some(CycleAction::SynthesisComplete(dataset.stats)))
            }
            CycleState::ReadyToExport => {
                let export = KnowledgeExporter::export_for_lora(
                    &*self.db, &Default::default(),
                ).await?;
                // Write JSONL to file
                std::fs::create_dir_all("exports")?;
                std::fs::write(&self.config.export_path, &export.training_jsonl)?;
                self.state = CycleState::Training;
                Ok(Some(CycleAction::ExportComplete(export.stats)))
            }
            CycleState::Training => {
                // Wait for external training (Plan 008 wgpu trainer)
                // Could poll file system for lora.bin, or wait for webhook
                Ok(None)
            }
            CycleState::Upgrading => {
                // POST to  API to hot-reload lora.bin
                if let Some(url) = &self.config.model_api_url {
                    // reqwest::post(format!("{}/reload_lora", url)).send().await?;
                }
                // Clear episodic memory for new cycle
                // db.execute("DELETE FROM episodes WHERE created_at < ?").await?;
                self.state = CycleState::Collecting;
                Ok(Some(CycleAction::CycleComplete))
            }
        }
    }
}

#[derive(Debug)]
pub enum CycleAction {
    BeginSynthesis,
    SynthesisComplete(SynthesisStats),
    ExportComplete(ExportStats),
    TrainingComplete { lora_path: String },
    CycleComplete,
}
```

### Part 5:  Hot-Reload Endpoint

```rust
// This goes in , not anyrag.
//  gets a simple HTTP endpoint that reloads lora.bin:

// /src/server.rs (new, behind "server" feature)
// POST /reload_lora — loads new lora.bin and swaps the adapter
// GET /status — returns current model stats, LoRA version, acceptance rate
```

## Database Schema

```sql
-- New table for translation episodes
CREATE TABLE IF NOT EXISTS episodes (
    id TEXT PRIMARY KEY,
    source_language TEXT NOT NULL,
    source_code TEXT NOT NULL,
    generated_rust TEXT NOT NULL,
    retrieved_context TEXT,         -- JSON array of search results
    hidden_state TEXT,              -- JSON array of f64 (embedding vector)
    compilation_result TEXT NOT NULL, -- JSON: CompilationResult enum
    created_at DATETIME NOT NULL,
    synthesized BOOLEAN DEFAULT FALSE
);

CREATE INDEX idx_episodes_success ON episodes(compilation_result);
CREATE INDEX idx_episodes_created ON episodes(created_at);
```

## 5.3 How microgpt-rs Plan 008 Trainer Consumes the JSONL

### JSONL Format

anyrag's `Curator::export_for_lora()` produces a JSONL file where each line is a JSON object with a `messages` array following the chat completion format:

```jsonl
{"messages":[{"role":"system","content":"Rewrite the following code in idiomatic Rust."},{"role":"user","content":"def hello(): print(\"hello\")"},{"role":"assistant","content":"fn hello() { println!(\"hello\"); }"}]}
{"messages":[{"role":"system","content":"Rewrite the following code in idiomatic Rust."},{"role":"user","content":"class Foo: ..."},{"role":"assistant","content":"struct Foo { ... }"}]}
```

Two sources feed into this JSONL:
1. **FAQ pairs** — structured YAML knowledge documents (existing `/knowledge/export` logic)
2. **Translation episodes** — successful `CompilationResult::Success` episodes from the `episodes` table

### Consumption Pipeline

```
anyrag                          microgpt-rs (Plan 008)
──────                          ────────────────────
Curator::export_for_lora()
        │
        ▼
training.jsonl ──────────────► DataLoader::from_jsonl(path, batch_size, seq_len, pad_id)
                                       │
                                       ▼
                                 Parse each line as TrainingSample { tokens: Vec<usize> }
                                 - system + user content = input_ids
                                 - assistant content = target_ids (shifted by 1)
                                       │
                                       ▼
                                 batches() → Iterator<Item = (Vec<u32>, Vec<u32>)>
                                 - Shuffles samples each epoch
                                 - Pads/truncates to seq_len
                                 - Returns (input_ids, target_ids) pairs
                                       │
                                       ▼
                                 GPU Training Loop
                                 for each (input_ids, target_ids):
                                   1. GpuForwardPass::forward(input_ids) → logits
                                   2. Cross-entropy loss(logits, target_ids)
                                   3. GpuBackwardPass::compute_lora_gradients()
                                   4. AdamW optimizer step
                                       │
                                       ▼
                                 export_lora() → lora.bin (safetensors)
```

### Config Compatibility

Plan 008's `DataLoader` requires `seq_len` and `pad_id` to match the tokenizer used during microgpt-rs inference. For the micro config:
- `vocab_size = 4096` (BPE tokenizer from Plan 007)
- `n_embd = 32`, `n_layer = 1`
- `lora_rank = 4`, `lora_alpha = 8.0`

The JSONL `messages` content must be tokenized by microgpt-rs's BPE tokenizer **before** training. The `DataLoader` expects pre-tokenized `TrainingSample { tokens }` — not raw text. This means either:
1. anyrag exports pre-tokenized JSONL (requires shared tokenizer), or
2. microgpt-rs tokenizes the JSONL at load time in `DataLoader::from_jsonl()`

Current implementation uses option 2: `DataLoader::from_jsonl()` parses raw text messages and microgpt-rs's tokenizer handles encoding.

### File Path Convention

- anyrag writes to: `exports/training.jsonl` (configurable via `CycleConfig::export_path`)
- microgpt-rs reads from: passed as CLI arg `--data training.jsonl`
- The cycle orchestrator's `ReadyToExport` state writes the file; microgpt-rs's CLI consumes it.

## 5.4 How microgpt-rs Hot-Reloads Trained lora.bin

### Hot-Reload Architecture

```
microgpt-rs (Plan 008)                     anyrag Cycle
──────────────────                       ────────────
POST /reload_lora ◄─────────────────── CycleConfig::model_api_url
        │                                         │
        ▼                                         │
load_lora(path, &mut forward)                     │
  1. Read lora.bin (safetensors)                  │
  2. Deserialize tensors                          │
  3. Upload A/B matrices to GPU                   │
  4. Swap into GpuForwardPass.lora.adapters       │
        │                                         │
        ▼                                         │
Next inference uses new LoRA weights              │
```

### The Reload Endpoint

microgpt-rs exposes a simple HTTP server (behind `#[cfg(feature = "server")]`):

```
POST /reload_lora    — Reads lora.bin from disk, uploads to GPU, swaps adapter
GET  /status         — Returns current model stats, LoRA version, acceptance rate
```

The reload is **atomic at the inference level**: the GPU buffers are swapped in a single write, so the next `forward()` call uses the new weights. There is no partial state — the old adapter GPU buffers are simply overwritten.

### lora.bin Format (safetensors)

```rust
// Key naming convention:
// "lora.{layer_idx}.a" — A matrix (down-projection): [n_embd, rank]  f32
// "lora.{layer_idx}.b" — B matrix (up-projection):   [rank, out_dim] f32

// Example for 1-layer micro config with rank=4, targets=["q","k","v","o","mlp1","mlp2"]:
// lora.0.a  → [32, 4]   f32 (q_proj down)
// lora.0.b  → [4, 32]   f32 (q_proj up)
// lora.1.a  → [32, 4]   f32 (k_proj down)
// lora.1.b  → [4, 32]   f32 (k_proj up)
// ... etc for v, o, mlp1, mlp2
```

For WASM targets where safetensors may not compile, a simpler binary format is used:
`[blake3_hash(4B) | n_layers(4B) | rank(4B) | layer_data...]` where each `layer_data` is `[a_len(4B) | a_data | b_len(4B) | b_data]`.

### Trigger Flow

1. anyrag cycle reaches `CycleState::Upgrading`
2. `CycleConfig::model_api_url` points to microgpt-rs's server (e.g., `http://localhost:8080`)
3. anyrag sends `POST {model_api_url}/reload_lora`
4. microgpt-rs reads `lora.bin` from disk (path configured server-side)
5. `load_lora()` deserializes + uploads to GPU
6. Next inference call uses updated LoRA weights
7. anyrag clears episodic memory and returns to `CycleState::Collecting`

### Current State

- `CycleConfig::model_api_url` exists in anyrag's config but the POST call is commented out (placeholder)
- microgpt-rs's server endpoint is not yet implemented (part of Plan 008 Phase 7)
- The `load_lora()` function is defined in Plan 008's Phase 6 but not yet implemented

## Tasks

### Phase 1: Episodic Memory
- [x] 1.1 Add `TranslationEpisode`, `CompilationResult`, `EpisodicStats` to types
- [x] 1.2 Create `crates/lib/src/ingest/episodic.rs`
- [x] 1.3 Add `episodes` table to DB schema
- [x] 1.4 Add `POST /episodes` handler
- [x] 1.5 Add `GET /episodes` handler with filtering
- [x] 1.6 Add `GET /episodes/stats` handler
- [x] 1.7 Add `POST /episodes/{id}/verify` handler
- [x] 1.8 Add tests: record, verify, query episodes

### Phase 2: Training Data Synthesis
- [x] 2.1 Extend `Curator` with `synthesize_training_data()`
- [x] 2.2 Implement pattern grouping (similar source → similar output)
- [x] 2.3 Add LLM prompt for canonical Q&A pair generation
- [x] 2.4 Add test: synthesis produces valid JSONL

### Phase 3: Extended Export
- [x] 3.1 Extend `/knowledge/export` to include translation episodes
- [x] 3.2 Add hidden state export for REST retrieval index
- [x] 3.3 Add `GET /knowledge/export/lora` — LoRA-specific export endpoint
- [x] 3.4 Add test: export produces valid training JSONL
- [x] 3.5 Add test: exported JSONL parses with serde_json

### Phase 4: Cycle Orchestrator
- [x] 4.1 Create `crates/lib/src/cycle.rs`
- [x] 4.2 Implement `SelfImprovingCycle` state machine
- [x] 4.3 Add `CycleConfig` to `AppConfig`
- [x] 4.4 Add background task in server that calls `tick()` periodically
- [x] 4.5 Add `GET /cycle/status` endpoint
- [x] 4.6 Add `POST /cycle/trigger` endpoint (manual trigger)
- [x] 4.7 Add test: state transitions work correctly

### Phase 5: Integration
- [x] 5.1 Wire: episode recording → RAG pipeline → episode storage
- [x] 5.2 Wire: synthesis → export → file system
- [x] 5.3 Document: how  Plan 008 trainer consumes the JSONL
- [x] 5.4 Document: how  hot-reloads trained lora.bin
- [x] 5.5 End-to-end test: record episode → verify → synthesize → export → JSONL

## Key Risks & Mitigations

| Risk | Impact | Mitigation |
|------|--------|-----------|
| LLM synthesis quality varies | Bad training data | Filter by compilation success; validate JSONL format |
| Cycle takes too long | Stale model | Configurable thresholds; manual trigger endpoint |
| Hidden state storage grows large | DB bloat | Compress vectors; limit retention period |
| Export/Train/Reload pipeline fragile | Broken cycle | Each step is independent and can be triggered manually |

## Expected Outcomes

1. **Episodic memory**: Every RIIR translation tracked with success/failure
2. **Training synthesis**: Curator auto-generates training pairs from successful episodes
3. **LoRA export**: `/knowledge/export/lora` produces JSONL for Plan 008 trainer
4. **32-day cycle**: Background orchestrator advances through collection → synthesis → export → upgrade
5. **Hot-reload**:  can swap `lora.bin` without restart

## Files to Create/Modify

| File | Action | Phase |
|------|--------|-------|
| `crates/lib/src/types.rs` | Add episode types | 1 |
| `crates/lib/src/ingest/episodic.rs` | New | 1 |
| `crates/lib/src/curator.rs` | Extend with synthesis | 2 |
| `crates/lib/src/ingest/knowledge.rs` | Extend export | 3 |
| `crates/lib/src/cycle.rs` | New | 4 |
| `crates/server/src/handlers/episodes.rs` | New | 1 |
| `crates/server/src/handlers/cycle.rs` | New | 4 |
| `crates/server/src/router.rs` | Add routes | 1, 4 |
| `crates/lib/src/types.rs` | Add CycleConfig | 4 |

## Cross-Project References

- `/.plans/008_wgpu_lora_training.md` — Consumes the JSONL this plan produces
- `/.plans/009_rest_speculative_decoding.md` — Queries episodic hidden states during inference
- `.research/00_Neuro-Symbolic LLM Architecture.md` — §Runtime LoRA Pipeline (Day 1→32)
- `.research/01_Advanced Neuro-Symbolic Rust Translation.md` — §Continuous Learning Loop
