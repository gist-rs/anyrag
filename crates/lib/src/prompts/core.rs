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

/// A shared set of rules for query construction to be used by multiple prompts.
pub const QUERY_CONSTRUCTION_RULES: &str = r#"# Query Construction Rules
1.  **Top-N Requests**: For requests asking for "top N", "highest", "best", or "most popular" items, you MUST use an `ORDER BY` clause on the relevant metric (e.g., `rating`, `views`) in descending order (`DESC`) and a `LIMIT` clause to restrict the number of results.
2.  **Column Specificity**: When the user's prompt specifies a column to filter on (e.g., "where the `topic_detail` contains..."), you MUST use that exact column in your `WHERE` clause. Do not substitute it with another column like `title`.
3. For questions about "who", "what", or "list", use DISTINCT to avoid duplicate results.
4. When filtering, always explicitly exclude NULL values (e.g., `your_column IS NOT NULL`).
5. For questions about "today", you MUST use one of the formats provided in the # TODAY context. Choose the format that matches the data in the relevant date column. If the column is TEXT, you may need to use string matching (e.g., `your_column LIKE 'YYYY-MM-DD%'`).
6. For searches involving a person's name, use a `LIKE` clause for partial matching (e.g., `name_column LIKE 'John%'`).
7. If a Japanese name includes an honorific like "さん", remove the honorific before using the name in the query.
8. For keyword searches (e.g., 'Rust') where no specific column is mentioned, it is vital to search across multiple fields. Your `WHERE` clause must use `LIKE` and `OR` to check for the keyword in all plausible text columns based on the schema. For example, you should check fields like `subject_name`, `class_name`, and `memo`.
9. **Crucially, do not format data in the query** (e.g., using `TO_CHAR` or `FORMAT`). Return raw numbers and dates. Formatting is handled separately.
10. **Compatibility Constraint**: You MUST NOT use subqueries (e.g., `SELECT ... FROM (SELECT ...)` or `WHERE col IN (SELECT ...)`). Use `JOIN`s or simplified `WHERE` clauses instead.
11. Use the provided table schema to ensure the query is correct. Do not use placeholders for table or column names.
12. **SQLite Compatibility Error**: The database will fail if you use `ORDER BY` on a compound `SELECT` (like `UNION` or `EXCEPT`). You MUST write a query that avoids this pattern. For example, do not combine `EXCEPT` and `ORDER BY`.
13. **Exclusion Logic (Availability)**: For questions about "who is available" or "who is not busy", you MUST use the `EXCEPT` clause to find the correct set of results. First, select all unique names/items. Then, `EXCEPT` the names/items that match the exclusion criteria. For example: `SELECT DISTINCT name FROM your_table EXCEPT SELECT name FROM your_table WHERE busy_date = 'YYYY-MM-DD'`."#;

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
            "The user's ultimate goal is to receive an answer that follows this #OUTPUT instruction: \"{inst}\". You MUST select all columns from the schema that are necessary to fulfill this final request. For example, if the instruction is to 'summarize the email body', you MUST select both the 'email_subject' and 'email_body' columns to provide sufficient context. Do not use `SELECT *`."
        ),
        _ => "Unless the user asks for 'everything' or 'all details', select only the most relevant columns to answer the question, not `SELECT *`.".to_string(),
    }
}
//
