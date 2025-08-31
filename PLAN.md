# Application Architectural Roadmap

This document outlines the high-level strategic plan for evolving the `anyrag` application into a secure, scalable, and production-grade multi-tenant RAG platform.

For the detailed, in-progress implementation plan for the current development phase, please see [`NOW.md`](./NOW.md).

## 1. Guiding Principles

-   **Privacy by Design**: Integrate privacy and security controls into every component.
-   **Separation of Concerns**: Keep components modular and single-purpose.
-   **Scalability**: Architect for growth in users, data, and features.
-   **Data Governance**: Implement clear rules for data retention, ownership, and deletion.

## 2. Target Architecture Overview

The final application will be composed of several key architectural pillars:

-   **`core-access` Crate**: A dedicated workspace crate that will serve as the central authority for all identity, authentication (AuthN), and authorization (AuthZ) logic.

-   **Metadata-Driven Search Engine**: A high-performance, multi-stage search pipeline that ensures speed and relevance. The flow is:
    1.  *Query Analysis*: An LLM extracts key entities from a user's query.
    2.  *Tag-Based Pre-Filtering*: A fast, indexed search on pre-extracted metadata finds a small set of relevant document candidates.
    3.  *Final Ranking & Synthesis*: The candidate documents are used for a final, precise vector search and/or LLM-based answer generation.

-   **Asynchronous Job Processing System**: A robust background worker system (backed by a database job queue) to handle long-running, resource-intensive tasks like document ingestion, metadata extraction, and embedding generation, ensuring the API remains fast and responsive.

-   **Multi-Tenant Data & Sharing Model**: A comprehensive data model that supports:
    -   **Private Content**: Owned by individual authenticated users.
    -   **Public Content**: Ingested by guests (if enabled).
    -   **Group Collaboration**: The ability for users to form groups and share resources with them.
    -   **Direct Sharing**: The ability to share resources with specific individual users.

## 3. Implementation Roadmap

The project will be implemented in a phased approach to manage complexity and deliver value incrementally.

-   **Phase 1 (Current Focus): Database & Search Architecture Refactoring**
    -   **Goal**: Restructure the database schema to normalize data, separating content, metadata (tags), and embeddings for optimal performance and security. Re-architect the search flow to be metadata-driven.
    -   **Details**: **See [`NOW.md`](./NOW.md) for the detailed implementation plan for this phase.**

-   **Phase 2: Core Access, Authentication & Ownership**
    -   **Goal**: Implement the `core-access` crate, JWT authentication, user management, and the core ownership model (public vs. private content).

-   **Phase 3: Groups, Sharing, & Collaboration**
    -   **Goal**: Build the complete feature set for creating and managing groups, and sharing resources with both groups and individual users.

-   **Phase 4: Governance & Operational Excellence**
    -   **Goal**: Implement data lifecycle features (TTL, content wipeout), privacy tools (PII scanner, Right to be Forgotten), and observability (metrics, tracing).

