# Current Action Sub-Tasks: RAG Pipeline Refactoring

This file breaks down the master plan for refactoring the RAG ingestion pipeline into immediate, actionable sub-tasks. Each of these tasks should be completed in order to achieve the goals outlined in `PLAN.md`.

## Task List

-   [ ] **1. Enhance HTML to Markdown Conversion**
    -   [ ] Modify the `html` crate to correctly extract the content of the `<title>` tag from an HTML document.
    -   [ ] Ensure that the extracted title is prepended to the final Markdown output as a level 1 header (e.g., `# Page Title`).
    -   [ ] Add a test case to the `html` crate to verify this functionality.

-   [ ] **2. Develop the LLM YAML Restructuring Prompt**
    -   [ ] Create a new, sophisticated system prompt in `anyrag/crates/lib/src/prompts/knowledge.rs`.
    -   [ ] This prompt will instruct the LLM to take messy, unstructured Markdown as input.
    -   [ ] The LLM's goal will be to identify semantic sections, clean up the text, and reformat the entire document into the specified structured YAML format.

-   [ ] **3. Refactor the Core Ingestion Logic (`run_ingestion_pipeline`)**
    -   [ ] Remove the call to the old `distill_and_augment` function.
    -   [ ] Create a new function, perhaps named `restructure_with_llm`, that takes the messy Markdown and uses the new prompt to generate the structured YAML.
    -   [ ] Modify the pipeline to store this single YAML string as the `content` in the `documents` table. The original, messy Markdown will no longer be stored.
    -   [ ] The concept of "augmenting" new FAQs will be removed, as the new structure preserves all content.

-   [ ] **4. Update the RAG and Search Logic (`hybrid_search`)**
    -   [ ] The search process will no longer query for individual FAQ documents.
    -   [ ] It will now fetch the full YAML `content` from relevant parent documents.
    -   [ ] Implement a YAML parsing step within the search function.
    -   [ ] Create a "chunking" logic that treats each `section` of the YAML as a single, context-rich document for embedding and retrieval.

-   [ ] **5. Update the Fine-Tuning Export Logic (`export_for_finetuning`)**
    -   [ ] Modify this function to query the `documents` table for the YAML content.
    -   [ ] Add logic to parse the YAML string.
    -   [ ] Iterate through the parsed structure to extract the `question` and `answer` pairs for the export dataset.

-   [ ] **6. Update and Create New Tests**
    -   [ ] The existing tests for the old `distill_and_augment` function will need to be removed or completely refactored.
    -   [ ] Create a new integration test that verifies the end-to-end process: from a URL to a structured YAML document in the database.
    -   [ ] Update the `knowledge_search_logic_test` to reflect the new RAG-on-YAML-chunks logic and ensure it retrieves the correct context.