# Engineering Plan: PDF Ingestion Refactoring

## 1. Goal

Refactor the PDF ingestion pipeline to process and store documents as contextual chunks, aligning it with the more advanced strategy used by the `anyrag-web` ingestor.

## 2. The "How" - Implementation Steps

1.  **Adopt Web Ingestor's Data Structures:**
    *   In `anyrag-pdf`, import and use the `YamlContent`, `Section`, and `Faq` structs from `anyrag-web` to handle the structured data.

2.  **Refactor `run_pdf_ingestion_pipeline` in `anyrag/crates/pdf/src/lib.rs`:**
    *   **Step 1: Extract Raw Text:** Keep the existing logic that uses `extract_text_from_pdf` to get the raw string content.
    *   **Step 2: Restructure to YAML:** Re-introduce the call to `restructure_with_llm`, passing it the raw text to get a structured YAML string.
    *   **Step 3: Parse the YAML:** Use `serde_yaml::from_str` to parse the YAML string into the `YamlContent` struct.
    *   **Step 4: Chunk and Store:**
        *   Iterate through the `sections` in the parsed `YamlContent`.
        *   For each `section`, create a new YAML string representing just that single section (as a `YamlContent` object with one section).
        *   Generate a unique ID for each chunk document.
        *   Insert each chunk into the `documents` table. The `source_url` should be modified slightly to ensure uniqueness (e.g., `test.pdf#section_0`).
        *   The original, full PDF document should *not* be stored. Only the chunks will be saved.
    *   **Step 5: Extract Metadata:** The `extract_and_store_metadata` function should be called for *each chunk* that is created.

3.  **Update the Test (`faq_ingestion_test.rs`):**
    *   The test must be updated to validate the new chunking behavior.
    *   **Database Assertions:** Instead of checking for one document with raw text, the test will need to:
        *   Query the database for multiple documents based on the source PDF name (e.g., using `LIKE 'test.pdf%'`).
        *   Assert that the expected number of chunks were created.
        *   Assert that the content of each chunk is the correct, structured YAML for that section.
    *   **Mock Updates:** The mock for the restructuring call will need to be re-enabled and provide a valid multi-section YAML response for the test to parse.

This plan will result in a more robust, efficient, and consistent ingestion pipeline.