# NOW: Parallel Hybrid Search Strategy

## Objective

The previous hybrid search implementation used a sequential, pre-filtering approach where a metadata search was performed first, and its results were used to constrain the subsequent keyword and vector searches. This was causing issues where relevant documents were being filtered out too early if the initial metadata search wasn't precise enough.

The new strategy addresses this by running all retrieval methods in parallel and combining their results for a more robust and comprehensive search.

## New Workflow

1.  **Query Analysis**: An LLM extracts key entities and concepts from the user's query. This step remains the same.

2.  **Parallel Retrieval**: The following three searches are executed concurrently, each searching the entire relevant corpus without pre-filtering:
    *   **Metadata Search**: Finds documents tagged with the extracted entities and keyphrases.
    *   **Keyword Search**: Performs a traditional `LIKE` search using the extracted keyphrases.
    *   **Vector Search**: Performs a semantic similarity search using an embedding of the original user query.

3.  **Reciprocal Rank Fusion (RRF)**:
    *   All candidate documents from the three parallel searches are collected.
    *   The results are combined and re-ranked using the RRF algorithm.
    *   A boost is applied to candidates found via the metadata search to give them a slight priority, as they are often more precise.

This parallel approach ensures that a relevant document will be found as long as at least one of the search methods can identify it, significantly improving the accuracy of the context provided to the RAG pipeline.
