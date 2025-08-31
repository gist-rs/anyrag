# Future Architectural Goals (TO HAVE)

This document outlines the key architectural pillars required to evolve `anyrag` into a scalable, secure, and production-grade service. These goals build upon the core application logic defined in `PLAN.md`.

The guiding principle for these enhancements is to maintain a lean, manageable technology stack, leveraging Turso/SQLite where feasible to minimize external dependencies.

## 1. Background Job Processing System

Many core tasks (e.g., ingesting and embedding a large PDF) are too long-running for a standard synchronous HTTP request-response cycle. To ensure the API remains fast and reliable, a background job processing system is required.

-   **Problem**: Long-running ingestion tasks will lead to HTTP timeouts and a poor user experience. It also tightly couples the web server's availability with the processing workload.
-   **Solution**:
    1.  **Job Queue with Turso**: A `jobs` table will be created in the Turso database to serve as the job queue.
        -   **Schema**: `job_id` (PK), `job_type` (e.g., 'ingest_pdf'), `payload` (JSON), `status` ('pending', 'processing', 'completed', 'failed'), `result` (JSON), `created_at`, `updated_at`.
    2.  **Asynchronous API Flow**:
        -   An API endpoint (e.g., `POST /ingest/file`) will receive a request, create a job record in the `jobs` table with a `'pending'` status, and immediately return a `202 Accepted` response with the `job_id`.
        -   A separate endpoint (`GET /jobs/{job_id}`) will allow clients to poll for the job's status.
    3.  **Worker Processes**: A separate, scalable pool of worker processes will be implemented. These workers will poll the `jobs` table for `'pending'` jobs, transactionally lock a job by updating its status to `'processing'`, execute the task, and finally update the status to `'completed'` or `'failed'` with a result.

-   **Benefit**: This creates a resilient, scalable, and responsive system. The API server remains lightweight, and the computationally expensive work can be scaled independently.

## 2. Advanced Role-Based Access Control (RBAC)

The current sharing model is direct. A more scalable system uses roles to define permissions.

-   **Problem**: Managing permissions on a per-user, per-resource basis becomes unmanageable in a collaborative environment.
-   **Solution**:
    1.  **Define Roles**: Introduce roles at both the group and system level.
        -   **Group Roles**: The `group_memberships` table will be modified to include a `role` column (e.g., `'admin'`, `'editor'`, `'viewer'`).
        -   **System Roles**: A `user_roles` table will be created to assign system-wide roles (e.g., `'super_admin'`) to users.
    2.  **Granular Permissions**: The `core-access` crate's authorization engine will be expanded to check these roles. For example, a group `admin` can manage memberships, while a `viewer` cannot. The `super_admin` role will be required for system-wide administrative API endpoints.

-   **Benefit**: RBAC provides a powerful and flexible way to manage permissions that scales with the number of users and resources.

## 3. Observability

To run a reliable service, we need deep visibility into its performance and behavior.

-   **Problem**: Basic logging is insufficient for diagnosing complex issues in a distributed system.
-   **Solution**:
    1.  **Metrics**: Instrument the application using a library compatible with **Prometheus**. We will export key metrics such as API request latency/error rates, job queue depth (`SELECT COUNT(*) FROM jobs WHERE status='pending'`), and job execution times.
    2.  **Distributed Tracing**: Integrate **OpenTelemetry** to trace requests as they flow from the API server, into the Turso job queue, are picked up by a worker, and result in calls to external AI providers.
    3.  **Structured Logging**: Ensure all logs are emitted in a structured format (e.g., JSON) and include the `trace_id` from OpenTelemetry to allow for easy correlation and analysis.

-   **Benefit**: Provides the necessary tools to proactively monitor system health, debug performance bottlenecks, and resolve errors quickly.

## 4. Caching Layer

Repetitive database queries, especially for authorization checks (e.g., looking up a user's group memberships), can create unnecessary load.

-   **Problem**: Frequent, identical queries for slow-changing data can impact performance.
-   **Solution**:
    1.  **Cache with Turso**: A dedicated table in Turso will be used as a simple key-value cache.
        -   **Schema**: `cache_key` (PK), `value` (BLOB/TEXT), `expires_at` (DATETIME).
    2.  **Cache Targets**: The system will cache data that is frequently read but infrequently written.
        -   A user's roles and permissions.
        -   Table schemas.
        -   Group membership lists.
    3.  **Note**: While a dedicated service like Redis is a more conventional choice for caching, using Turso aligns with our goal of maintaining a lean technology stack for the initial phases of the project.

-   **Benefit**: Reduces database load, decreases latency for common operations, and improves overall application responsiveness.