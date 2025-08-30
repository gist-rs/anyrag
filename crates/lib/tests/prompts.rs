//! # Prompt Generation Logic Tests
//!
//! This test suite validates the helper functions in `anyrag::prompts::core`
//! that dynamically generate instructional snippets for the main prompt templates.
//! These tests ensure that user-provided options correctly translate into
//! specific, clear instructions for the AI.

use anyrag::prompts::core::{get_alias_instruction, get_select_instruction};

// --- Tests for `get_select_instruction` ---

/// Verifies that a specific, non-empty instruction generates the detailed prompt variant.
#[test]
fn test_get_select_with_valid_instruction() {
    let instruction = "Show me the names and ages of all users.";
    let expected = format!(
        "The user's ultimate goal is to receive an answer that follows this #OUTPUT instruction: \"{instruction}\". You MUST select all columns from the schema that are necessary to fulfill this final request. For example, if the instruction is to 'summarize the body', you MUST select the 'body' column. Do not use `SELECT *`."
    );
    assert_eq!(get_select_instruction(Some(instruction)), expected);
}

/// Verifies that a `None` value for the instruction results in the generic fallback prompt.
#[test]
fn test_get_select_with_none_instruction() {
    let expected = "Unless the user asks for 'everything' or 'all details', select only the most relevant columns to answer the question, not `SELECT *`.";
    assert_eq!(get_select_instruction(None), expected.to_string());
}

/// Verifies that an empty string `""` is considered empty and results in the generic prompt.
#[test]
fn test_get_select_with_empty_string_instruction() {
    let expected = "Unless the user asks for 'everything' or 'all details', select only the most relevant columns to answer the question, not `SELECT *`.";
    assert_eq!(get_select_instruction(Some("")), expected.to_string());
}

/// Verifies that a string containing only whitespace is correctly trimmed, considered empty,
/// and results in the generic prompt.
#[test]
fn test_get_select_with_whitespace_instruction() {
    let expected = "Unless the user asks for 'everything' or 'all details', select only the most relevant columns to answer the question, not `SELECT *`.";
    assert_eq!(
        get_select_instruction(Some("   \t\n  ")),
        expected.to_string()
    );
}

// --- Tests for `get_alias_instruction` ---

/// Verifies that providing a specific `answer_key` generates the aliasing instruction
/// with the correct key.
#[test]
fn test_get_alias_with_valid_key() {
    let answer_key = "total_users";
    let expected = "If the query uses an aggregate function or returns a single column, alias the result with `AS total_users`.";
    assert_eq!(get_alias_instruction(Some(answer_key)), expected);
}

/// Verifies that a `None` value for the `answer_key` results in the generic instruction
/// for the AI to choose its own alias.
#[test]
fn test_get_alias_with_none_key() {
    let expected = "If the query uses an aggregate function or returns a single column, choose a descriptive, single-word, lowercase alias for the result based on the user's question (e.g., for 'how many users', use `count`; for 'who is the manager', use `manager`).";
    assert_eq!(get_alias_instruction(None), expected.to_string());
}

/// Verifies that an empty string `""` for the `answer_key` still produces a grammatically
/// correct (though likely useless) instruction. This confirms the function doesn't panic on edge cases.
#[test]
fn test_get_alias_with_empty_string_key() {
    let answer_key = "";
    let expected = "If the query uses an aggregate function or returns a single column, alias the result with `AS `.";
    assert_eq!(get_alias_instruction(Some(answer_key)), expected);
}
