# Project: AnyRAG - Intelligent Code Context System

## 1. Goal

The primary goal is to create a system that can crawl public GitHub repositories, intelligently extract versioned "how-to-use examples" from various sources (documentation, code, tests), store them in a queryable format, and serve them through an API. This provides LLMs with accurate, up-to-date context for code generation tasks, overcoming the issue of outdated knowledge. A secondary goal is to provide tooling for ingesting other structured and unstructured data sources (like Firestore and web pages) to build comprehensive knowledge bases.

## 2. Completed Milestones

### 2.1. Core GitHub Ingestion Pipeline
- **Repository Crawler:** Clones public GitHub repositories, handles versioning (tags, branches, commits), and falls back to `Cargo.toml` or latest commit if no version is specified.
- **Example Extractor:** Intelligently extracts code examples from multiple sources within a repository, prioritizing tests > doc comments > example files > READMEs to ensure accuracy.
- **Versioned Storage:** Uses a multi-database SQLite approach. A central metadata DB tracks all repositories, while each repository gets its own dedicated database for storing versioned examples and their embeddings.
- **Consolidated Markdown Export:** Provides a mechanism to generate a single, well-formatted Markdown file containing all extracted examples for a specific repository and version, ready for use as a large context block for LLMs.

### 2.2. Server API
- **GitHub Ingestion API:**
    - `POST /ingest/github`: Triggers the ingestion of a repository.
    - `GET /examples/{repo_name}/{version}`: Generates and returns the consolidated Markdown for a specific version.
- **RAG Query API:**
    - `POST /search/examples`: A RAG endpoint to perform hybrid (keyword + vector) searches across one or more repository databases to find relevant code examples based on a natural language query.

### 2.3. Command-Line Interface (CLI)
- **`dump firebase`:** A robust command to fetch data from a Google Firestore collection and store it in a local SQLite database. It supports full and incremental dumps, field selection, and more.
- **`dump github`:** A command to execute the entire GitHub ingestion pipeline from the command line. It takes a repository URL, automatically ingests the content, and generates the consolidated Markdown context file locally.
- **Utility Commands:** Includes helper commands like `login`, `list`, and `count` for managing authentication and inspecting local databases.

## 3. Future Scope & Unaddressed Considerations

- **Scalability & Rate Limiting:** The current implementation uses unauthenticated Git cloning. For heavy use, integrating GitHub API authentication (Personal Access Tokens) will be necessary to avoid rate limits.
- **Expanded Language Support:** The example extractor is currently optimized for Rust projects (`.rs` files, `Cargo.toml`). The architecture is modular and can be extended to support other languages (e.g., TypeScript, Python) by adding new extraction logic.
- **Code Embedding & Semantic Search:** While the database schema supports storing embeddings for code examples, the pipeline to generate and utilize them for the `/search/examples` endpoint is not fully implemented. This would enable more powerful semantic search for code.
- **Security:** Ingesting content from arbitrary repositories is handled safely by only treating files as text and not executing any code. This principle must be maintained.
- **Advanced RAG Strategies:** The RAG pipeline could be enhanced with more sophisticated strategies, such as re-ranking with a cross-encoder or summarizing retrieved examples before synthesis.