# Future Architectural Goals (TOHAVE)

This document outlines planned architectural enhancements to elevate the `anyrag` application to a production-grade, scalable, and operationally excellent service. These goals build upon the core application logic defined in `PLAN.md`.

The guiding principle for these enhancements is to maintain a lean, manageable technology stack, leveraging Turso/SQLite where feasible to minimize external dependencies.

## 1. Metadata-Driven Search Architecture

To achieve superior search performance and relevance, the system will be enhanced with a metadata-first search strategy.

-   **Problem**: Full-text and vector search across large documents can be slow and inefficient for targeted queries, and they may miss context.
-   **Solution**:
    1.  **LLM-based Entity & Tag Extraction**: During ingestion, an asynchronous background job will analyze the content and extract key entities (e.g., people, products, locations, concepts).
    2.  **Structured Metadata Storage**: A new, indexed table, `content_tags`, will store these extracted entities. Each tag will be linked to its source document and will inherit the `owner_id` to ensure security is maintained.
        -   **Schema**: `content_id`, `owner_id`, `tag_type`, `tag_value`.
    3.  **Multi-Stage Search Flow**: The primary RAG search will be upgraded:
        a.  **Query Analysis**: An initial LLM call will extract key entities from the user's query.
        b.  **Tag Filtering**: A fast, indexed SQL query will run against the `content_tags` table to find documents that contain these entities and are accessible to the user. This creates a small set of highly relevant candidates.
        c.  **Final RAG**: The candidate documents will then be used for the final vector search and LLM synthesis step.
-   **Benefit**: This creates a highly efficient pre-filtering layer, dramatically improving search speed and relevance while reducing the workload on the more expensive vector search and LLM components.

## 2. Background Job Processing System

To ensure the API remains fast and responsive, long-running and resource-intensive tasks will be moved to a background processing system.

-   **Problem**: Synchronous processing of large PDFs, web scraping, and multiple LLM calls for distillation and entity extraction will lead to HTTP timeouts and a poor user experience.
-   **Solution**:
    1.  **Job Queue with Turso**: A `jobs` table will be created in the Turso database to serve as the job queue.
        -   **Schema**: `job_id` (PK), `job_type` (e.g., 'ingest_pdf', 'extract_entities'), `payload` (JSON), `status` ('pending', 'processing', 'completed', 'failed'), `result` (JSON), `created_at`, `updated_at`.
    2.  **Asynchronous API Flow**: An API endpoint (e.g., `POST /ingest/file`) will create a job and immediately return a `202 Accepted` response with the `job_id`. A separate `GET /jobs/{job_id}` endpoint will allow clients to poll for the status.
    3.  **Worker Processes**: A separate pool of worker processes will poll the `jobs` table, transactionally lock and execute pending jobs, and update their status upon completion.
-   **Benefit**: Decouples the API server from heavy processing, creating a resilient, scalable, and responsive system.

## 3. Advanced Role-Based Access Control (RBAC)

The current sharing model will be enhanced with a formal role system for more granular and scalable permissions management.

-   **Problem**: Managing permissions on a per-user, per-resource basis becomes unmanageable in a collaborative environment.
-   **Solution**:
    1.  **Define Roles**: Introduce roles at both the group level (e.g., `admin`, `editor`, `viewer` in `group_memberships`) and the system level (e.g., `super_admin` in a `user_roles` table).
    2.  **Granular Permissions**: The `core-access` crate's authorization engine will be expanded to check these roles. For example, the `super_admin` role will be required for system-wide administrative API endpoints.
-   **Benefit**: RBAC provides a powerful and flexible way to manage permissions that scales with the number of users and resources.

## 4. Observability

To run a reliable service, we need deep visibility into its performance and behavior.

-   **Problem**: Basic logging is insufficient for diagnosing complex issues in a distributed system.
-   **Solution**:
    1.  **Metrics**: Instrument the application with a library compatible with **Prometheus**. We will export key metrics such as API request latency/error rates, job queue depth, and job execution times.
    2.  **Distributed Tracing**: Integrate **OpenTelemetry** to trace requests as they flow from the API server, into the Turso job queue, are picked up by a worker, and result in calls to external AI providers.
    3.  **Structured Logging**: Ensure all logs are emitted in a structured format (e.g., JSON) and include the `trace_id` from OpenTelemetry to allow for easy correlation and analysis.
-   **Benefit**: Provides the necessary tools to proactively monitor system health, debug performance bottlenecks, and resolve errors quickly.

## 5. Caching Layer

Repetitive database queries, especially for authorization checks (e.g., looking up a user's group memberships), can create unnecessary load.

-   **Problem**: Frequent, identical queries for slow-changing data can impact performance.
-   **Solution**:
    1.  **Cache with Turso**: A dedicated table in Turso will be used as a simple key-value cache.
        -   **Schema**: `cache_key` (PK), `value` (BLOB/TEXT), `expires_at` (DATETIME).
    2.  **Cache Targets**: The system will cache data that is frequently read but infrequently written, such as a user's roles and permissions, table schemas, and group membership lists.
-   **Benefit**: Reduces database load, decreases latency for common operations, and improves overall application responsiveness.