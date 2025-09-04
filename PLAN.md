### **Revised Plan: From TUI to a Powerful `clap`-based CLI**

This plan outlines the migration from the `Ratatui` TUI to a `clap`-based command-line interface, focusing on a robust, multi-stage data processing pipeline from Firebase to local enrichment and fine-tuning.

#### **1. Core Objective**

To create a powerful, scriptable CLI for authenticating, dumping, and processing data. The CLI will feature two distinct, decoupled stages: `dump` (retrieving data from a source like Firebase) and `process` (enriching that data for RAG and preparing it for fine-tuning).

#### **2. Core Technologies**

*   **CLI Framework**: `clap` (replacing `ratatui` and `crossterm`).
*   **Firebase/GCP Integration**: A suitable Rust client for Firestore (e.g., `gcp_sdk` or direct REST API calls with `reqwest`) to interact with the targeted Firebase project.
*   (The rest of the technology stack remains the same: `tokio`, `keyring`, `reqwest`, `anyhow`, etc.)

---

#### **3. Feature Roadmap & CLI Structure**

The new CLI structure will be command-based, like this:

```sh
anyrag <COMMAND>
```

##### **Phase 1: Migration to `clap` and Core Structure**

This phase establishes the new foundation.

1.  **Dependency Swap**:
    *   Remove `ratatui` and `crossterm` from `crates/cli/Cargo.toml`.
    *   Add `clap` with the `derive` feature.
2.  **Refactor `main.rs`**:
    *   Replace the entire TUI event loop with a `clap` parser.
    *   Define the main `Cli` struct with subcommands for `login`, `dump`, and `process`.
    *   The existing logging-to-file setup will be preserved, which is perfect for a CLI.

##### **Phase 2: Firebase-style Authentication**

This phase implements the `login` command, reusing our existing browser-based flow.

1.  **Define the `login` Command**:
    *   Create a `login` subcommand: `anyrag login`.
2.  **Integrate Existing Auth Logic**:
    *   The `login` command handler will call the existing `auth::login()` function.
    *   Upon successful authentication, it will use the `keyring` crate to securely store the received JWT, just as the TUI version did.
    *   It will print a "Login successful" message to the console. All subsequent commands that require authentication will automatically and silently read the token from the keychain.

##### **Phase 3: Data Dumping from Firebase**

This is the first major new feature, focusing on getting data *out* of Firebase.

1.  **Define the `dump` Command**:
    *   Create a `dump` subcommand with a nested `firebase` command: `anyrag dump firebase`.
    *   Add a required `--project-id` option to specify the target Firebase project.
    *   Add an optional `--collection` option to specify which Firestore collections to dump.
2.  **Implement the Logic**:
    *   The handler will require an authenticated user (reading the token from the keychain).
    *   It will use a Firestore client to connect to the specified project.
    *   It will fetch all documents from the specified collection(s).
    *   It will map the Firestore document structure to a new table in the local SQLite database and insert the data. This creates a local, decoupled copy of the source data.

##### **Phase 4: Data Enrichment for RAG**

This phase re-implements the existing ingestion pipeline as a dedicated `process` command.

1.  **Define the `process` Command**:
    *   Create a `process` subcommand: `anyrag process`.
    *   This command will operate entirely on the local SQLite database.
2.  **Implement the Logic**:
    *   The handler will read the raw data from the tables created by the `dump` command.
    *   For each raw data entry, it will execute the existing `run_ingestion_pipeline` (distill, augment, create metadata, embed vectors).
    *   This populates the `documents`, `faq_items`, `document_embeddings`, and `content_metadata` tables, making the data ready for RAG queries.

##### **Phase 5: Storytelling Fine-Tuning Format**

This phase extends the `process` command to generate a new fine-tuning format.

1.  **Extend the `process` Command**:
    *   Add an optional `--format` flag, which can be `faq` (default) or `storytelling`.
    *   `anyrag process --format storytelling`
2.  **Plan the Storytelling Training Data Format**:
    *   **Goal**: To train a model to take raw, structured data (like a JSON object) and turn it into a narrative.
    *   **Model**: `qwen3-30b-a3b-thinking-2507` and similar models are trained on conversational formats. We will generate data that fits this pattern.
    *   **Process**: The `--format storytelling` option will trigger a new LLM-based transformation step. For each raw data entry, it will:
        1.  Make an LLM call with a "storyteller" persona.
        2.  The LLM's task is to convert the raw JSON data into a compelling narrative paragraph.
        3.  The output will be saved as a JSONL file, with each line being a training example.

3.  **Proposed Training Data Structure (`storytelling_finetune.jsonl`)**:

    ```json
    {"messages": [{"role": "system", "content": "You are a master storyteller. Your task is to weave compelling, engaging narratives from the raw, structured data you are given. Focus on clarity, tone, and creating a story that is easy to understand."}, {"role": "user", "content": "Here is some data about an event:\n\n```json\n{\"eventName\": \"Project Phoenix Launch\", \"date\": \"2024-08-15\", \"lead\": \"Dr. Aris Thorne\", \"outcome\": \"Successful deployment of solar sails\", \"notes\": \"Minor telemetry glitch post-deployment, resolved within 2 hours.\"}\n```\n\nPlease tell me a story about this event."}, {"role": "assistant", "content": "On August 15th, 2024, the ambitious Project Phoenix reached a major milestone under the leadership of Dr. Aris Thorne. The team successfully deployed the advanced solar sails, marking a new era in stellar exploration. While a minor telemetry glitch occurred shortly after deployment, the skilled engineering team quickly resolved it within two hours, ensuring the mission's continued success."}]}
    {"messages": [{"role": "system", "content": "You are a master storyteller..."}, {"role": "user", "content": "Here is some data about a person:\n\n```json\n{\"name\": \"Kaelen\", \"role\": \"Lead Archivist\", \"specialty\": \"Pre-digital cartography\", \"achievement\": \"Rediscovered the lost maps of the Sunken City of Aeridor.\"}\n```\n\nPlease tell me a story about this person."}, {"role": "assistant", "content": "Kaelen, the Lead Archivist, was a true master of their craft, specializing in the delicate art of pre-digital cartography. Their dedication culminated in a monumental achievement: the rediscovery of the long-lost maps of the Sunken City of Aeridor, a find that reshaped the known history of the world."}]}
    ```
