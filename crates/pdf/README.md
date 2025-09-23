# Anyrag PDF Ingestor (`anyrag-pdf`)

This crate provides the ingestion logic for PDF documents, acting as a plugin for the `anyrag` ecosystem. It implements the `Ingestor` trait from `anyrag-lib`.

## Ingestion Strategy: Contextual Chunking

This ingestor follows the project's standard data strategy of structured contextual chunking. The pipeline is as follows:

1.  **Text Extraction**: The raw text content is extracted from the PDF file.
2.  **LLM-Powered Restructuring**: The raw text is sent to a Language Model (LLM) which intelligently reformats the messy content into a structured YAML document, identifying logical sections and question/answer pairs.
3.  **Chunking and Storing**: The pipeline parses the structured YAML and iterates through each top-level `section`. Each section is then stored as a separate, independent "document" (a chunk) in the database.
4.  **Metadata Extraction**: Metadata (Entities and Keyphrases) is extracted by an LLM for *each individual chunk*, allowing for highly precise retrieval during a search.

This strategy ensures that when a user query is received, the RAG pipeline can retrieve only the most relevant, focused chunks of the original PDF, leading to faster, more accurate, and cheaper responses.

## Refactoring Status: Complete

This crate was the subject of a significant refactoring to align it with the project's modern, chunk-based ingestion strategy.

-   The `run_pdf_ingestion_pipeline` function was updated to parse the restructured YAML from the LLM.
-   It now successfully iterates through each section, storing it as an independent, chunked document.
-   Integration tests were updated to validate this new behavior, ensuring the correct number of chunks are created with their respective metadata.
-   The `MockAiProvider` in the test utilities was refactored to a FIFO queue to correctly handle the sequential metadata extraction calls required by the chunking logic.

This refactoring is now complete, bringing the PDF ingestor in line with the more advanced strategies used elsewhere in the project.