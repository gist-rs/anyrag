# Plan: Implement Batch Embedding for Gemini

## 1. Goal

The current embedding process sends one API request for each document, which is inefficient. The goal is to implement batch embedding for the Gemini provider to significantly improve performance and reduce the number of API calls, as supported by the Gemini API.

## 2. Diagnosis

The inefficiency lies in two key areas:
1.  The `embed_new_handler` in `anyrag/crates/server/src/handlers/knowledge.rs` loops through each document and calls `generate_embedding` individually.
2.  The `generate_embedding` function in `anyrag/crates/lib/src/providers/ai/embedding.rs` is designed to accept only a single text input.

## 3. Implementation Plan

### 3.1. Modify the Core Embedding Function (`generate_embedding`)

The `generate_embedding` function will be refactored to handle batches of text.

**File to Modify**: `anyrag/crates/lib/src/providers/ai/embedding.rs`

**Changes**:
1.  **Update Function Signature**: Change the function to accept a slice of strings (`&[&str]`) and return a vector of embeddings (`Vec<Vec<f32>>`).
2.  **Update Gemini Request/Response Structs**: Modify the JSON structures to match the batch format.

**Current (Simplified) Gemini Structs:**
```rust
struct GeminiEmbeddingRequest<'a> {
    model: String,
    content: GeminiEmbeddingContent<'a>,
}

struct GeminiEmbeddingContent<'a> {
    parts: Vec<GeminiEmbeddingPart<'a>>,
}
```

**Proposed (Batch-enabled) Gemini Structs:**
```rust
// The top-level request will now contain a Vec of content items
struct GeminiBatchEmbeddingRequest<'a> {
    model: String,
    content: Vec<GeminiEmbeddingContent<'a>>,
}

// The response will contain a Vec of embeddings
struct GeminiBatchEmbeddingResponse {
    embeddings: Vec<GeminiEmbeddingValue>,
}
```
3. **Update Logic**: Modify the function's implementation to build the batch request and parse the batch response correctly.

### 3.2. Update the Embedding Handler (`embed_new_handler`)

The handler will be updated to collect all document texts into a batch before making a single API call.

**File to Modify**: `anyrag/crates/server/src/handlers/knowledge.rs`

**Current (Incorrect) Logic:**
```rust
// Simplified for clarity
for (doc_id, title, content) in docs_to_embed {
    let text_to_embed = format!("{title}. {content}");
    // This is called inside a loop
    match generate_embedding(api_url, model, &text_to_embed, api_key).await {
        // ... store single embedding
    }
}
```

**Proposed (Correct) Logic:**
```rust
// Simplified for clarity
// 1. Collect all texts into a batch
let texts_to_embed: Vec<String> = docs_to_embed
    .iter()
    .map(|(_, title, content)| format!("{title}. {content}"))
    .collect();

let text_slices: Vec<&str> = texts_to_embed.iter().map(AsRef::as_ref).collect();

// 2. Make a single batch API call
let embeddings = generate_embedding(api_url, model, &text_slices, api_key).await?;

// 3. Loop through the results to store them
for ((doc_id, _, _), vector) in docs_to_embed.iter().zip(embeddings) {
    // ... store the embedding for the corresponding doc_id
}
```

## 4. Verification

After implementing the changes, I will run the `knowledge_prompt2` example to verify the fix:

`RUST_LOG=info cargo run -p anyrag-server --example knowledge_prompt2`

The expected outcome is that the example completes successfully, and the logs from `embed_new_handler` will show that all documents were found and processed in a single batch operation, confirming the new logic is working correctly.