//! # Text Ingestion and Chunking Tests
//!
//! This file contains integration tests for the text chunking logic
//! provided in the `anyrag` library.

// The chunking logic is in a submodule, so we need to specify the path.
use anyrag::ingest::text::{chunk_text, IngestError};

// It's good practice to define constants used in tests, especially if they
// mirror constants in the implementation, to catch accidental changes.
const CHUNK_SIZE_LIMIT: usize = 4096;
const CHUNK_OVERLAP: usize = 200;

/// Verifies that a simple text with two distinct paragraphs is split correctly.
#[test]
fn test_chunk_text_simple() {
    let text = "This is a short text.\n\nIt has two paragraphs.";
    let chunks = chunk_text(text).unwrap();
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0], "This is a short text.");
    assert_eq!(chunks[1], "It has two paragraphs.");
}

/// Verifies that an empty input string results in the expected `EmptyContent` error.
#[test]
fn test_chunk_text_empty_input() {
    let text = "";
    let result = chunk_text(text);
    assert!(matches!(result, Err(IngestError::EmptyContent)));
}

/// Verifies that an input string containing only whitespace is correctly identified
/// as empty and results in an `EmptyContent` error.
#[test]
fn test_chunk_text_whitespace_input() {
    let text = "   \t\n  ";
    let result = chunk_text(text);
    assert!(matches!(result, Err(IngestError::EmptyContent)));
}

/// Verifies that a single, very long paragraph that exceeds the `CHUNK_SIZE_LIMIT`
/// is correctly split into multiple chunks with the specified overlap.
#[test]
fn test_long_paragraph_gets_split() {
    let long_paragraph = "a".repeat(5000);
    let chunks = chunk_text(&long_paragraph).unwrap();

    // The logic should be:
    // Chunk 1: Chars 0..4096
    // Next start: 4096 - 200 = 3896
    // Chunk 2: Chars 3896..5000
    assert_eq!(
        chunks.len(),
        2,
        "Expected the long paragraph to be split into 2 chunks"
    );

    // Verify the content and length of the first chunk.
    assert_eq!(chunks[0].chars().count(), CHUNK_SIZE_LIMIT);
    assert_eq!(chunks[0], "a".repeat(CHUNK_SIZE_LIMIT));

    // Verify the content and length of the second chunk.
    let expected_second_chunk_len = 5000 - (CHUNK_SIZE_LIMIT - CHUNK_OVERLAP); // 5000 - 3896 = 1104
    assert_eq!(chunks[1].chars().count(), expected_second_chunk_len);
    assert_eq!(chunks[1], "a".repeat(expected_second_chunk_len));
}

/// Verifies that a text containing both a short paragraph and a very long paragraph
/// is chunked correctly, preserving the short paragraph as a single chunk.
#[test]
fn test_mixed_length_paragraphs() {
    let short_paragraph = "This is short.";
    let long_paragraph = "b".repeat(6000);
    let text = format!("{short_paragraph}\n\n{long_paragraph}");

    let chunks = chunk_text(&text).unwrap();

    // Expect 1 chunk for the short paragraph and 2 for the long one.
    // Chunk 1: short_paragraph
    // Chunk 2: long_paragraph[0..4096]
    // Chunk 3: long_paragraph[3896..6000]
    assert_eq!(chunks.len(), 3);
    assert_eq!(chunks[0], short_paragraph);
    assert_eq!(chunks[1].chars().count(), CHUNK_SIZE_LIMIT);
    assert_eq!(
        chunks[2].chars().count(),
        6000 - (CHUNK_SIZE_LIMIT - CHUNK_OVERLAP)
    );
}

/// Verifies an edge case where a long text could potentially cause an infinite loop
/// if the exit condition for the overlap logic is not correctly implemented.
#[test]
fn test_no_infinite_loop_on_overlap() {
    // Create a text that is just slightly longer than the step size (limit - overlap).
    // An incorrect loop condition might fail to terminate here.
    let text = "c".repeat(CHUNK_SIZE_LIMIT - CHUNK_OVERLAP + 1);
    let chunks = chunk_text(&text).unwrap();

    // It should produce only one chunk because the next starting point would be <= the current one.
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].len(), CHUNK_SIZE_LIMIT - CHUNK_OVERLAP + 1);
}
