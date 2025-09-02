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
7. **Crucially, do not format data in the query** (e.g., using `TO_CHAR` or `FORMAT`). Return raw numbers and dates. Formatting is handled separately.

{select_instruction}
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
pub const SQLITE_QUERY_USER_PROMPT: &str = r#"Your task is to write a single, read-only SQLite query based on the provided schema and question.

# Primary Goal
{select_instruction}
{alias_instruction}

# User Question
{prompt}

# Context
{context}
"#;

// --- Response Formatting Prompts ---

/// The default system prompt for the response formatting stage.
///
/// This prompt sets the persona for the AI when it's formatting the final
/// response from the query results.
pub const DEFAULT_FORMAT_SYSTEM_PROMPT: &str = "You are a strict data processor. Your only purpose is to answer the user's #PROMPT by strictly using the provided #INPUT data and following the #OUTPUT instructions. You MUST NOT use any external knowledge or make any assumptions. Your response must only contain information directly present in the #INPUT. If the #INPUT is empty or `[]`, you MUST state that no information was found to answer the question, and nothing else. If the user's question can be answered with a 'yes' or asks for a list, first provide a count and then list the items in a bulleted format (e.g., 'Yes, there are 3 items:\\n- Item A\\n- Item B\\n- Item C'). Do not add any explanations or text that is not directly derived from the input data.";

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
/// otherwise it defaults to `result`. This ensures a predictable column name.
pub fn get_alias_instruction(answer_key: Option<&str>) -> String {
    let key = answer_key.unwrap_or("result");
    format!(
        "In the SELECT clause, if you are selecting an aggregate function or a single column, you MUST alias it with `AS {key}`."
    )
}

/// Generates the instruction for selecting specific columns in a query.
///
/// This function returns a specific instruction if a user `instruction` is provided,
/// otherwise it returns a general instruction to avoid using `SELECT *`.
pub fn get_select_instruction(instruction: Option<&str>) -> String {
    match instruction {
        Some(inst) if !inst.trim().is_empty() => format!(
            "The user's ultimate goal is to receive an answer that follows this #OUTPUT instruction: \"{inst}\". You MUST select all columns from the schema that are necessary to fulfill this final request. For example, if the instruction is to 'summarize the body', you MUST select the 'body' column. Do not use `SELECT *`."
        ),
        _ => "Unless the user asks for 'everything' or 'all details', select only the most relevant columns to answer the question, not `SELECT *`.".to_string(),
    }
}
