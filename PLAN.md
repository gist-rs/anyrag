# Anyrag Master Plan

This document outlines the architectural vision and core principles for the `anyrag` project. It serves as a blueprint for development, ensuring a modular, scalable, and maintainable system.

## 1. Core Philosophy

The fundamental goal is to create a robust ecosystem for Retrieval-Augmented Generation (RAG) by adhering to these principles:

-   **Modularity**: Each distinct functionality, especially data ingestion, should be isolated in its own crate, acting as a "plugin."
-   **Separation of Concerns**: The API server should only handle web concerns (requests, responses, auth), while the core library orchestrates the business logic.
-   **Extensibility**: The architecture must make it simple to add new data sources or AI providers without modifying core components.
-   **Clarity**: Code and data structures should be organized logically to be easily understood and maintained.

## 2. High-Level Architecture

The `anyrag` workspace is composed of several key components:

1.  **`anyrag-server`**: The public-facing API built with `axum`. Its sole responsibility is to handle HTTP requests, perform validation and authentication, and pass sanitized data to `anyrag-lib`. It contains zero business logic.

2.  **`anyrag-lib`**: The heart of the application. This library crate orchestrates all core processes, including ingestion and search pipelines. It exposes a clear public API for consumption by `anyrag-server` and `anyrag-cli`.

3.  **`anyrag-cli`**: A command-line interface for administrative and ingestion tasks. It provides a direct way to interact with the functionalities exposed by `anyrag-lib`.

4.  **Ingestion Crates (Plugins)**: A collection of specialized crates, each responsible for a single data source (e.g., `anyrag-github`, `anyrag-html`, `anyrag-pdf`). These crates implement a common trait from `anyrag-lib`, making them pluggable.

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

To achieve true modularity, ingestion will be handled by a generic trait-based system.

-   An `Ingestor` trait will be defined in `anyrag-lib`.
-   Each ingestion crate (`anyrag-github`, `anyrag-pdf`, etc.) will provide a struct that implements this `Ingestor` trait.
-   `anyrag-lib`'s ingestion pipeline will operate on a generic `T: Ingestor`, making it easy to swap or add new data sources.

### c. Feature Flags

Both `anyrag-server` and `anyrag-lib` will use feature flags to control which ingestion plugins are compiled and enabled.

-   Each ingestion feature (e.g., `ingest-pdf`, `ingest-github`) will be gated by a Cargo feature.
-   By default, all features will be enabled.
-   This allows users of the library or server to build a smaller, more specialized binary by disabling features they don't need.

### d. Centralized Types

To avoid circular dependencies and maintain a single source of truth for data models, we will consolidate shared types.

-   A `types.rs` module will be created within `anyrag-lib`.
-   This module will contain core data structures used across the application (e.g., `SearchResult`, `Document`, configuration structs).
-   This separation ensures that any component can depend on the core types without pulling in unnecessary logic.

### e. Contextual Configuration

Configuration files, especially prompts, should be organized for clarity and maintainability.

-   **`config.yml`**: Defines providers, models, and global settings.
-   **`prompt.yml`**: An optional, user-provided file for overriding default prompts. This file is git-ignored.
-   **Default Prompts**: Default prompts are hardcoded within the `anyrag-lib` in a dedicated `prompts` module, organized by task (e.g., `prompts/knowledge.rs`, `prompts/pdf.rs`). This keeps the prompts co-located with the logic that uses them.

## 4. Data Flow Overview

### Ingestion Flow

1.  **Trigger**: A user initiates ingestion via an API call (`anyrag-server`) or a CLI command (`anyrag-cli`).
2.  **Orchestration**: The request is passed to `anyrag-lib`, which identifies the correct ingestion plugin based on the source type.
3.  **Execution**: `anyrag-lib` calls the `ingest` method on the specific `Ingestor` implementation (e.g., `github::GithubIngestor`).
4.  **Processing**: The plugin fetches, cleans, and structures the data.
5.  **Storage**: The structured data is stored in the appropriate SQLite database.

### RAG Search Flow

1.  **Request**: A user sends a search query to `anyrag-server`.
2.  **Orchestration**: The server passes the validated query to `anyrag-lib`.
3.  **Hybrid Retrieval**: `anyrag-lib` executes its multi-stage hybrid search pipeline (metadata, keyword, vector) against the database to find relevant document chunks.
4.  **Synthesis**: The retrieved chunks are passed as context to a configured LLM provider, which synthesizes a final answer.
5.  **Response**: The synthesized answer is returned through the library to the server, which then sends it to the client.