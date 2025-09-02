### **PLAN: Refactor Configuration to a Structured YAML File**

#### **1. Objective**

To replace the current `.env`-based configuration for AI providers and prompts with a more structured, flexible, and maintainable `config.yml` file. This will allow for assigning different AI models and prompts to specific tasks within the application.

#### **2. Proposed `config.yml` Structure**

We will use YAML for its superior readability, especially for multi-line prompts. The configuration will be split into three main sections: `embedding`, `providers`, and `tasks`.

*   **`embedding`**: A special section for the embedding model, which has a different interface than generative models.
*   **`providers`**: A dictionary of reusable provider configurations. This allows us to define a provider once (e.g., "my_gemini_pro") and reference it in multiple tasks, following the Don't Repeat Yourself (DRY) principle. Secrets like `api_key` will be loaded from environment variables.
*   **`tasks`**: Defines the specific AI model and prompts to be used for each distinct task in the application.

To improve user experience, we will provide two ready-to-use templates. The user can copy the one that matches their setup to `config.yml`.

**Example `config.gemini.yml` (for Cloud AI):**
```yaml
# Configuration for the Google Gemini API
embedding:
  api_url: "${EMBEDDINGS_API_URL}"
  model_name: "gemini-embedding-001"

providers:
  gemini_default:
    provider: "gemini"
    api_url: "${AI_API_URL}"
    api_key: "${AI_API_KEY}"
    model_name: "gemini-2.5-flash-lite"

  local_fast:
    provider: "local"
    api_url: "http://localhost:1234/v1/chat/completions"
    api_key: null
    model_name: "gemma-2-9b-it-mlx"

tasks:
  query_generation:
    provider: "gemini_default"
    system_prompt: |
      You are a SQL expert for the specified database. Write a readonly SQL query
      that answers the user's question. Expected output is a single SQL query only.
    user_prompt: |
      Follow these rules to create production-grade SQL...
      # Context
      {context}
      # User question
      {prompt}

  rag_synthesis:
    provider: "gemini_default"
    system_prompt: |
      You are a strict, factual AI. Your sole purpose is to answer the user's
      question based *only* on the provided #Context.
    user_prompt: |
      # User Question
      {prompt}
      # Context
      {context}
      # Your Answer:

  knowledge_distillation:
    provider: "gemini_default"
    system_prompt: |
      You are an expert data extraction and reconciliation agent...
    user_prompt: |
      # Markdown Content to Process:
      {markdown_content}

  query_analysis:
    provider: "local_fast" # Use a faster model for this simple task
    system_prompt: |
      You are an expert query analyst. Your task is to extract key **Entities**
      and **Keyphrases** from the user's query...
    user_prompt: |
      # USER QUERY:
      {prompt}

  llm_rerank:
    provider: "local_fast"
    system_prompt: |
      You are an expert search result re-ranker...
    user_prompt: |
      # User Query:
      {query_text}
      # Articles to Re-rank:
      {articles_context}
```

**Example `config.local.yml` (for Local AI):**
```yaml
# Configuration for a local AI provider (e.g., Ollama, LM Studio)
embedding:
  api_url: "${EMBEDDINGS_API_URL}"
  model_name: "text-embedding-qwen3-embedding-8b"

providers:
  local_default:
    provider: "local"
    api_url: "${AI_API_URL}"
    api_key: null
    model_name: "gemma-2-9b-it-mlx"

tasks:
  query_generation:
    provider: "local_default"
    system_prompt: |
      You are a SQL expert for the specified database. Write a readonly SQL query
      that answers the user's question. Expected output is a single SQL query only.
    user_prompt: |
      Follow these rules to create production-grade SQL...
      # Context
      {context}
      # User question
      {prompt}

  rag_synthesis:
    provider: "local_default"
    system_prompt: |
      You are a strict, factual AI. Your sole purpose is to answer the user's
      question based *only* on the provided #Context.
    user_prompt: |
      # User Question
      {prompt}
      # Context
      {context}
      # Your Answer:

  knowledge_distillation:
    provider: "local_default"
    system_prompt: |
      You are an expert data extraction and reconciliation agent...
    user_prompt: |
      # Markdown Content to Process:
      {markdown_content}

  query_analysis:
    provider: "local_default"
    system_prompt: |
      You are an expert query analyst. Your task is to extract key **Entities**
      and **Keyphrases** from the user's query...
    user_prompt: |
      # USER QUERY:
      {prompt}

  llm_rerank:
    provider: "local_default"
    system_prompt: |
      You are an expert search result re-ranker...
    user_prompt: |
      # User Query:
      {query_text}
      # Articles to Re-rank:
      {articles_context}
```

#### **3. Implementation Plan**

**What Stays in `.env`**

The `.env` file will continue to be used for secrets and environment-specific variables that are not part of the application's logical structure. This includes:

*   **Secrets**: `AI_API_KEY`, `EMBEDDINGS_API_KEY` (if any). These are referenced from the `.yml` config via `${VAR_NAME}`.
*   **Provider URLs**: `AI_API_URL`, `EMBEDDINGS_API_URL`.
*   **Environment Pointers**: `GOOGLE_APPLICATION_CREDENTIALS`.
*   **Operational Variables**: `RUST_LOG`, `PORT`.
*   **Example-Specific Variables**: `BIGQUERY_PROJECT_ID` will be kept for use in client scripts and examples but will not be read by the server at startup.

**Step 1: Add Dependencies and New Config Files**
1.  Add the `config` and `serde_yaml` crates to `anyrag-server/Cargo.toml`.
2.  Create two template files in `anyrag-server/`: `config.local.yml` and `config.gemini.yml`.
3.  Instruct users to copy their preferred template to `config.yml`.
4.  Create a `.gitignore` entry for `config.yml`.

**Step 2: Define New Configuration Structs**
1.  In `anyrag-server/src/config.rs`, create new Rust structs that directly map to the YAML structure:
    *   `AppConfig` (the root object)
    *   `EmbeddingConfig`
    *   `ProviderConfig`
    *   `TaskConfig` (will contain `provider` reference string, `system_prompt`, `user_prompt`)

**Step 3: Refactor the Configuration Loader**
1.  Modify `config::get_config()` to use the `config` crate.
2.  It will be configured to load from `config.yml` and merge in environment variables (for `${VAR}` substitution), ensuring secrets stay out of the config file.
3.  The old logic for loading individual prompt templates from `.env` will be removed.

**Step 4: Update `AppState` and Provider Management**
1.  The `AppState` struct will be simplified. Instead of storing individual prompt strings, it will store an `Arc<AppConfig>`.
2.  `build_app_state` will be updated. It will no longer build a single, default `PromptClient`. Instead, it will create a `HashMap` or a factory that can dynamically create an `AiProvider` instance for each definition in the `providers` section of the config. This map of providers will be stored in `AppState`.

**Step 5: Update Application Logic to Use the New Config**
1.  All handlers and library functions that currently use a hardcoded prompt (e.g., `DEFAULT_QUERY_SYSTEM_PROMPT`) will be refactored.
2.  They will now:
    a.  Determine the current task (e.g., `query_generation`).
    b.  Retrieve the corresponding `TaskConfig` from `app_state.config.tasks`.
    c.  Retrieve the correct `AiProvider` from the provider map in `AppState` using the `provider` key from the `TaskConfig`.
    d.  Use the `system_prompt` and `user_prompt` from the `TaskConfig` for the AI call.

**Step 6: Cleanup and Documentation**
1.  Remove the now-unused `DEFAULT_..._PROMPT` constants from `anyrag/crates/lib/src/prompts/`.
2.  Update `anyrag/crates/server/.env.example` to remove the old prompt variables and instruct the user to configure `config.yml`.
3.  Update the `README.md` to explain the new, more powerful configuration method.
