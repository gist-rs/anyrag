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

/// The default user prompt for the query generation stage.
///
/// This template defines how the user's specific question and any table schema
/// context are presented to the AI.
///
/// Placeholders: `{language}`, `{context}`, `{prompt}`, `{alias_instruction}`
pub const DEFAULT_QUERY_USER_PROMPT: &str = "Follow these rules to create production-grade {language}:\n\
    1. For questions about \"who\", \"what\", or \"list\", use DISTINCT to avoid duplicate results.\n\
    2. When filtering, always explicitly exclude NULL values (e.g., `your_column IS NOT NULL`).\n\
    3. For date filtering, prefer using `EXTRACT(YEAR FROM your_column)` over functions like `FORMAT_TIMESTAMP`.\n\
    4. For searches involving a person's name, use a `LIKE` clause for partial matching (e.g., `name_column LIKE 'John%'`).\n\
    5. If a Japanese name includes an honorific like \"さん\", remove the honorific before using the name in the query.\n\
    6. For keyword searches (e.g., 'Python'), it is vital to search across multiple fields. Your `WHERE` clause must use `LIKE` and `OR` to check for the keyword in all plausible text columns based on the schema. For example, you should check fields like `subject_name`, `class_name`, and `memo`.\n\n\
    {alias_instruction}\n\n\
    Use the provided table schema to ensure the query is correct. Do not use placeholders for table or column names.\n\n\
    # Context\n{context}\n\n# User question\n{prompt}";

// --- Response Formatting Prompts ---

/// The default system prompt for the response formatting stage.
///
/// This prompt sets the persona for the AI when it's formatting the final
/// response from the query results.
pub const DEFAULT_FORMAT_SYSTEM_PROMPT: &str = "You are a helpful AI assistant. Your purpose is to answer the user's #PROMPT based on the provided #INPUT data, following the #OUTPUT instructions. If the user's question can be answered with a 'yes' or asks for a list, you must first provide a count and then list the items in a bulleted format. For example: 'Yes, there are 3 Python classes:\\n- Class A\\n- Class B\\n- Class C'. Do not add any explanations or text that is not directly derived from the input data.";

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
