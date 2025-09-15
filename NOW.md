# Plan to Fix Ignored `instruction` in RAG Pipeline

## 1. Problem Diagnosis

The `instruction` field provided in the `SearchRequest` payload is not being passed to the final answer synthesis step in the RAG pipeline. This causes the AI to ignore specific formatting requests, such as the one for Question 3 in the `knowledge_prompt2` example.

The root cause is that the `ExecutePromptOptions` struct is being created without forwarding the `instruction` from the incoming request.

## 2. Locate the Bug

The error is located in the `knowledge_search_handler` function within the file:
`anyrag/crates/server/src/handlers/knowledge.rs`

## 3. Implement the Fix

I will modify the instantiation of `ExecutePromptOptions` inside the `knowledge_search_handler` to correctly pass the `instruction` from the `payload`.

**Current (Incorrect) Code:**
```rust
let mut options = ExecutePromptOptions {
    prompt: payload.query.clone(),
    content_type: Some(ContentType::Knowledge),
    context: Some(context.clone()),
    // instruction is missing or hardcoded to None here
    ..Default::default()
};
```

**Proposed (Correct) Code:**
```rust
let mut options = ExecutePromptOptions {
    prompt: payload.query.clone(),
    content_type: Some(ContentType::Knowledge),
    context: Some(context.clone()),
    instruction: payload.instruction, // This line will be added/corrected
    ..Default::default()
};
```

## 4. Verification

After applying the fix, I will run the `knowledge_prompt2` example again:
`cargo run -p anyrag-server --example knowledge_prompt2`

The expected outcome is that the answer to Question 3 will now correctly start with the phrase specified in the instruction ("สรุปเงื่อนไขได้ว่า..."), confirming that the instruction is no longer being ignored.