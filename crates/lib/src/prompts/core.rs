//! # Default Prompt Templates
//!
//! This module contains the default prompt templates used by the `PromptClient`.
//! These can be overridden at runtime via `ExecutePromptOptions` or environment
//! variables in the `anyrag-server`.

// --- Query Generation Prompts ---

/// The default system prompt for the query generation stage.
///
/// This prompt sets the core persona and rules for the AI when it's generating a query.
///
/// Placeholders: `{language}`, `{db_name}`
pub const DEFAULT_QUERY_SYSTEM_PROMPT: &str = "You are a {language} expert for {db_name}. Write a readonly {language} query that answers the user's question. Expected output is a single {language} query only.";

/// The default user prompt for the query generation stage (for BigQuery).
///
/// This template defines how the user's specific question and any table schema
/// context are presented to the AI.
///
/// Placeholders: `{language}`, `{context}`, `{prompt}`, `{alias_instruction}`
pub const DEFAULT_QUERY_USER_PROMPT: &str = r#"Follow these rules to create production-grade {language}:

1. For questions about "who", "what", or "list", use DISTINCT to avoid duplicate results.
2. When filtering, always explicitly exclude NULL values (e.g., `your_column IS NOT NULL`).
3. For questions about "today", you MUST use one of the formats provided in the # TODAY context. Choose the format that matches the data in the relevant date column. If the column is TEXT, you may need to use string matching (e.g., `your_column LIKE 'YYYY-MM-DD%'`).
4. For searches involving a person's name, use a `LIKE` clause for partial matching (e.g., `name_column LIKE 'John%'`).
5. If a Japanese name includes an honorific like "さん", remove the honorific before using the name in the query.
6. For keyword searches (e.g., 'Rust'), it is vital to search across multiple fields. Your `WHERE` clause must use `LIKE` and `OR` to check for the keyword in all plausible text columns based on the schema. For example, you should check fields like `subject_name`, `class_name`, and `memo`.

{alias_instruction}

Use the provided table schema to ensure the query is correct. Do not use placeholders for table or column names.

# Context
{context}

# User question
{prompt}"#;

/// A SQLite-specific user prompt for query generation.
///
/// This is similar to the default prompt but provides rules tailored to SQLite's SQL dialect,
/// especially concerning date functions.
pub const SQLITE_QUERY_USER_PROMPT: &str = r#"Follow these rules to create production-grade SQLite SQL:

1. For questions about "who", "what", or "list", use DISTINCT to avoid duplicate results.
2. When filtering, always explicitly exclude NULL values (e.g., `your_column IS NOT NULL`).
3. For questions about "today", you MUST use one of the formats provided in the # TODAY context. Choose the format that matches the data in the relevant date column. If the column is a DATETIME type, use `date(your_column) = 'YYYY-MM-DD'`. If it is TEXT, you may need to use string matching (e.g., `your_column LIKE 'YYYY-MM-DD%'`). Do not use `date('now')` for this purpose.
4. For searches involving a person's name, use a `LIKE` clause for partial matching (e.g., `name_column LIKE 'John%'`).
5. If a Japanese name includes an honorific like "さん", remove the honorific before using the name in the query.
6. For keyword searches (e.g., 'Rust'), it is vital to search across multiple fields. Your `WHERE` clause must use `LIKE` and `OR` to check for the keyword in all plausible text columns based on the schema. For example, you should check fields like `subject_name`, `class_name`, and `memo`.

{alias_instruction}

Use the provided table schema to ensure the query is correct. Do not use placeholders for table or column names.

# Context
{context}

# User question
{prompt}"#;

// --- Response Formatting Prompts ---

/// The default system prompt for the response formatting stage.
///
/// This prompt sets the persona for the AI when it's formatting the final
/// response from the query results.
pub const DEFAULT_FORMAT_SYSTEM_PROMPT: &str = "You are a helpful AI assistant. Your purpose is to answer the user's #PROMPT based on the provided #INPUT data, following the #OUTPUT instructions. IMPORTANT: If the #INPUT is empty or `[]`, you MUST state that no information was found to answer the question, and nothing else. Otherwise, if the user's question can be answered with a 'yes' or asks for a list, you must first provide a count and then list the items in a bulleted format. For example: 'Yes, there are 3 Rust classes:\\n- Class A\\n- Class B\\n- Class C'. Do not add any explanations or text that is not directly derived from the input data.";

/// The default user prompt for the response formatting stage.
///
/// This template defines how the original question, formatting instructions,
/// and raw data are presented to the AI for the final formatting step.
///
/// Placeholders: `{prompt}`, `{instruction}`, `{content}`
pub const DEFAULT_FORMAT_USER_PROMPT: &str = r#"# PROMPT:
{prompt}

# OUTPUT:
{instruction}

# INPUT:
{content}
"#;

/// Generates the instruction for aliasing a result column in a query.
///
/// This function returns a specific instruction if an `answer_key` (alias) is provided,
/// otherwise it returns a general instruction for the AI to choose a descriptive alias.
pub fn get_alias_instruction(answer_key: Option<&str>) -> String {
    match answer_key {
        Some(key) => format!(
            "If the query uses an aggregate function or returns a single column, alias the result with `AS {key}`."
        ),
        None => "If the query uses an aggregate function or returns a single column, choose a descriptive, single-word, lowercase alias for the result based on the user's question (e.g., for 'how many users', use `count`; for 'who is the manager', use `manager`).".to_string(),
    }
}

// --- Rerank Prompts ---

/// The system prompt for the LLM-based re-ranking stage.
///
/// This prompt instructs the AI to act as an expert re-ranker and defines the
/// expected JSON output format.
pub const DEFAULT_RERANK_SYSTEM_PROMPT: &str = "You are an expert search result re-ranker. Your task is to re-order a list of provided articles based on their relevance to a user's query. Analyze the user's query and the article content (title and description). Return a JSON array containing only the `link` strings of the articles in the new, correctly ordered sequence, from most relevant to least relevant. Do not add any explanation or other text outside of the JSON array.";

/// The user prompt for the LLM-based re-ranking stage.
///
/// This template provides the user's query and the list of candidate articles
/// to the AI for re-ranking.
///
/// Placeholders: `{query_text}`, `{articles_context}`
pub const DEFAULT_RERANK_USER_PROMPT: &str = "# User Query:
    {query_text}n\
    # Articles to Re-rank:
    {articles_context}n\
    # Your Output (JSON array of links only):\n";
