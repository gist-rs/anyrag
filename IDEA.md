# Anyrag Developer Guide

This document outlines the architectural vision, core principles, and fundamental development strategies for the `anyrag` project. It serves as a blueprint for ongoing development, ensuring a modular, scalable, and maintainable system.

---

## 1. Core Philosophy

The fundamental goal is to create a robust ecosystem for Retrieval-Augmented Generation (RAG) by adhering to these principles:

-   **Modularity**: Each distinct functionality, especially data ingestion, should be isolated in its own crate, acting as a "plugin."
-   **Separation of Concerns**: The API server should only handle web concerns (requests, responses, auth), while the core library orchestrates the business logic.
-   **Extensibility**: The architecture must make it simple to add new data sources or AI providers without modifying core components.
-   **Clarity**: Code and data structures should be organized logically to be easily understood and maintained.

---

## 2. High-Level Architecture

The `anyrag` workspace is composed of several key components:

1.  **`anyrag-server`**: The public-facing API built with `axum`. Its sole responsibility is to handle HTTP requests, perform validation and authentication, and pass sanitized data to `anyrag-lib`. It contains zero business logic.

2.  **`anyrag-lib`**: The heart of the application. This library crate orchestrates all core processes, including ingestion and search pipelines. It exposes a clear public API for consumption by `anyrag-server` and `anyrag-cli`.

3.  **`anyrag-cli`**: A command-line interface for administrative and ingestion tasks. It provides a direct way to interact with the functionalities exposed by `anyrag-lib`.

4.  **Ingestion Crates (Plugins)**: A collection of specialized crates, each responsible for a single data source (e.g., `anyrag-github`, `anyrag-html`, `anyrag-pdf`). These crates implement a common trait from `anyrag-lib`, making them pluggable.

---

## 3. Key Architectural Concepts

### a. Server vs. Library Responsibility

-   **`anyrag-server`**:
    -   Defines API routes and handlers.
    -   Handles request parsing, validation, and serialization.
    -   Manages authentication and authorization.
    -   Translates HTTP requests into calls to `anyrag-lib`.
    -   Formats responses for the client.

-   **`anyrag-lib`**:
    -   Contains all business logic for RAG pipelines.
    -   Defines traits for pluggable components (e.g., `Ingestor`).
    -   Orchestrates the flow of data from ingestion to storage and from query to synthesis.
    -   Is completely agnostic of the web server.

### b. Plugin-Based Ingestion via Traits

To achieve true modularity, ingestion is handled by a generic trait-based system.

-   An `Ingestor` trait is defined in `anyrag-lib`.
-   Each ingestion crate (`anyrag-github`, `anyrag-pdf`, etc.) provides a struct that implements this `Ingestor` trait.
-   `anyrag-lib`'s ingestion pipeline operates on a generic `T: Ingestor`, making it easy to swap or add new data sources.

### c. Feature Flags

Both `anyrag-server` and `anyrag-lib` use feature flags to control which ingestion plugins are compiled and enabled.

-   Each ingestion feature (e.g., `ingest-pdf`, `ingest-github`) is gated by a Cargo feature.
-   By default, all features are enabled.
-   This allows users of the library or server to build a smaller, more specialized binary by disabling features they don't need.

### d. Centralized Types

To avoid circular dependencies and maintain a single source of truth for data models, shared types are consolidated.

-   A `types.rs` module is created within `anyrag-lib`.
-   This module contains core data structures used across the application (e.g., `SearchResult`, `Document`, configuration structs).
-   This separation ensures that any component can depend on the core types without pulling in unnecessary logic.

---

## 4. Fundamental Data Strategy: Structured Contextual Chunking

### The Rationale

Storing entire documents as single, monolithic entries is inefficient for Retrieval-Augmented Generation (RAG). When a user asks a question, the system retrieves the whole document, forcing the Language Model (LLM) to sift through potentially irrelevant information to find the answer. This leads to:

-   **Reduced Accuracy**: The LLM can get lost in the noise of a large, unfocused context, resulting in less precise answers.
-   **Increased Latency & Cost**: Processing larger contexts is slower and more expensive.
-   **Inconsistency**: Different data sources would otherwise require different handling logic.

The solution is to **break documents into smaller, structured YAML chunks**. For example, a web page or PDF is restructured into a standardized format of sections and FAQ pairs. Each section is then stored as an independent "document" in the database.

This strategy enables the retrieval of **highly-focused context**. The RAG pipeline can find and use the exact section or FAQ that answers the user's question. This provides the LLM with precisely the information it needs, leading to **faster, more accurate, and cheaper responses**, and ensures a consistent data model across all ingestion sources.