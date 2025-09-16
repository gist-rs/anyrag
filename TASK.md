# Task: Refactor the Knowledge Ingestion Pipeline

## Goal
The primary goal is to fix the RAG system's failure to retrieve correct context for user queries. The root cause has been identified as improper document chunking, where entire webpages are stored as a single large document, leading to context truncation when sent to the LLM.

## Strategy
The correct strategy is to leverage the existing LLM-powered restructuring step, which already converts raw content into structured YAML. We will use this structured YAML to create smaller, semantically coherent document chunks. Each chunk will be a self-contained, valid YAML document representing a single section of the original content.

---

## Detailed Plan

The following changes will be made exclusively in `anyrag/crates/lib/src/ingest/knowledge.rs`.

### 1. Define Global Structs for YAML Parsing
At the top of the file, define the necessary structs to represent the YAML structure. This makes them available to all functions in the module.

- Add `#[derive(Debug, Deserialize, Serialize, Clone)]` to `Faq` and `Section`.
- Add `#[derive(Debug, Deserialize, Serialize)]` to `YamlContent`.

```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
struct Faq {
    question: String,
    answer: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Section {
    title: String,
    faqs: Vec<Faq>,
}

#[derive(Debug, Deserialize, Serialize)]
struct YamlContent {
    sections: Vec<Section>,
}
```

### 2. Verify `ingest_and_cache_url`
Ensure this function is in its original, simple state. Its only responsibility is to fetch and cache the raw content from a URL as a **single document**.

- It should **NOT** perform any chunking.
- Its return type must be `Result<(String, IngestedDocument), KnowledgeError>`.

### 3. Refactor `run_ingestion_pipeline`
This is the core of the change. The function will be rewritten to orchestrate the new chunking workflow.

- **Stage 1: Ingest Raw Content**
  - Call `ingest_and_cache_url` to get the initial temporary document.

- **Stage 2: Restructure to YAML**
  - Call `restructure_with_llm` to convert the raw content into a large `structured_yaml` string.
  - Add a check to handle cases where the LLM returns empty or invalid YAML (e.g., `""` or `"[]"`).

- **Stage 3: Chunk from YAML and Store**
  - **Delete the original raw document**. It is no longer needed.
  - Parse the `structured_yaml` string into the `YamlContent` struct.
  - Iterate through `yaml_content.sections`.
  - In each iteration:
    1. Create a new `YamlContent` object containing only the current `section`.
    2. Serialize this new object back into a small, self-contained `yaml_chunk` string.
    3. Generate a unique `chunk_id` for the new document.
    4. `INSERT` the `yaml_chunk` into the `documents` table. The document's `title` should be the `section.title`.

- **Stage 4: Extract Metadata**
  - Inside the loop, immediately after inserting a chunk, call `extract_and_store_metadata` for that specific `chunk_id` and `yaml_chunk`.

### 4. Cleanup `export_for_finetuning`
- Remove the local, redundant `Faq`, `Section`, and `YamlContent` struct definitions from within this function, as they are now defined globally at the top of the file.

---

## Verification
Once all code changes are complete, run the example test to confirm the fix:
```bash
RUST_LOG=info cargo run -p anyrag-server --example knowledge_prompt2
```
The test should now pass, as the RAG system will be able to retrieve the correct, specific YAML chunk containing the answer about "ออมต่อ".