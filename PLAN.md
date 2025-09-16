# Master Plan: Refactoring the RAG Ingestion Pipeline

This document outlines the master plan for refactoring the `anyrag` library's knowledge base ingestion and Retrieval-Augmented Generation (RAG) process. The goal is to move from a disconnected, FAQ-based model to a structured, context-aware system that improves retrieval accuracy and simplifies dataset creation for fine-tuning.

## 1. Core Problem with the Current Approach

The existing pipeline over-segments information. It uses an LLM to distill a single source document into dozens of individual question-answer pairs, each stored as a separate document. This approach has several major drawbacks:

-   **Loss of Context:** Each answer is stored in isolation, stripped of its surrounding context. This makes it difficult for the RAG system to understand the nuance of a retrieved answer, leading to less accurate synthesis.
-   **Brittle Retrieval:** The system heavily relies on matching the user's query to the exact `title` (the question) of a stored document.
-   **Inefficient Processing:** It requires multiple LLM calls (distillation, augmentation) for a single source, which is slow and costly.

## 2. The New Vision: Structured Data and Contextual RAG

We will refactor the pipeline to create a single, well-structured document for each source. This document will serve as the "source of truth" and will be optimized for both RAG and future fine-tuning tasks.

The new workflow will be as follows:

### 2.1. Ingestion: LLM-Powered Restructuring

1.  **Fetch & Convert:** The initial step remains the same: fetch the raw content from a URL and perform a basic HTML-to-Markdown conversion.
2.  **LLM Restructuring:** The messy, converted Markdown will be sent to an LLM with a new, sophisticated prompt. The LLM's task is to intelligently reformat the content into a structured YAML format. This YAML will group related FAQs under semantic headings, preserving the original document's hierarchy.
3.  **Store Single Source of Truth:** The resulting YAML document will be stored as a single entry in the `documents` table. This becomes the canonical, clean representation of the knowledge source.

**Example of the Target YAML Structure:**

```yaml
section:
  title: Section Title (e.g., "การออมต่อ")
  faqs:
    - question: "A specific question from the section."
      answer: |
        A multi-line answer that preserves formatting.
    - question: "Another question from the same section."
      answer: |
        Its corresponding answer.
```

### 2.2. RAG: Chunking on Structure

1.  **On-the-Fly Parsing:** During a search, the RAG system will fetch the YAML content from the database.
2.  **Contextual Chunking:** Instead of chunking by arbitrary text length, we will parse the YAML and treat each `section` as a single, semantic chunk. This ensures that the context (the section title) is always included with its corresponding Q&A pairs.
3.  **Embedding & Retrieval:** These context-rich chunks will be embedded and indexed for vector search.
4.  **Informed Synthesis:** When a chunk is retrieved, the final LLM will receive the section title, the question, and the answer, giving it all the necessary context to synthesize a high-quality response.

### 2.3. Fine-Tuning: Simple and Clean

The structured YAML stored in the database is already a clean, high-quality dataset. A simple script can parse these documents and export the question-answer pairs in the required format for model fine-tuning, with no additional LLM calls or complex processing needed.

## 3. Future Integration: Knowledge Graph

This new structured data model provides a perfect foundation for future knowledge graph integration. The entities, keyphrases, and relationships captured in the YAML can be programmatically extracted to populate a graph database, further enhancing the system's reasoning capabilities. This will be addressed after the core RAG pipeline is successfully refactored.