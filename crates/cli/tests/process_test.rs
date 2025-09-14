//! # CLI Process Command Tests
//!
//! This file contains tests for the `process` command of the `anyrag-cli`.

use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

/// Helper to create a temporary fixture file within a given directory.
fn create_fixture_file(dir: &std::path::Path, content: &str) -> std::path::PathBuf {
    let file_path = dir.join("sample.md");
    let mut file = fs::File::create(&file_path).expect("Failed to create fixture file");
    file.write_all(content.as_bytes())
        .expect("Failed to write to fixture file");
    file_path
}

#[test]
fn test_process_file_command_success() {
    // Arrange
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_process.db");
    let fixture_content =
        "This is the first chunk.\n---\nThis is the second one.\n---\nAnd a third.";
    let fixture_path = create_fixture_file(temp_dir.path(), fixture_content);

    // Act
    let mut cmd = Command::cargo_bin("cli").unwrap();
    cmd.arg("process")
        .arg("file")
        .arg(fixture_path.to_str().unwrap())
        .arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("--separator")
        .arg("\n---\n");

    // Assert
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Successfully ingested 3 chunks"));
}

#[test]
fn test_process_file_command_no_file() {
    // Arrange
    let mut cmd = Command::cargo_bin("cli").unwrap();

    // Act & Assert
    cmd.arg("process")
        .arg("file")
        .arg("a/non/existent/file.md")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Process failed"));
}

#[test]
fn test_process_file_command_custom_separator() {
    // Arrange
    let temp_dir = tempdir().unwrap();
    let db_path = temp_dir.path().join("test_separator.db");
    let fixture_content = "Part 1|||Part 2";
    let fixture_path = create_fixture_file(temp_dir.path(), fixture_content);

    // Act
    let mut cmd = Command::cargo_bin("cli").unwrap();
    cmd.arg("process")
        .arg("file")
        .arg(fixture_path.to_str().unwrap())
        .arg("--db-path")
        .arg(db_path.to_str().unwrap())
        .arg("--separator")
        .arg("|||");

    // Assert
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Successfully ingested 2 chunks"));
}
