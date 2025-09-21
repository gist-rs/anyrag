//! # `gof` Crate Integration Tests

use anyhow::Result;
use gof::{format_mcp_response, parse_dependencies, McpSearchResult, McpSuccessResponse};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_parse_dependencies_success() -> Result<()> {
    // Arrange
    let dir = tempdir()?;
    let file_path = dir.path().join("Cargo.toml");
    let mut file = File::create(&file_path)?;

    let content = r#"[package]
name = "test-project"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = "1.0.190"
tokio = { version = "1.35.1", features = ["full"] }
thiserror = { version = "1.0" }
"#;
    file.write_all(content.as_bytes())?;

    // Act
    let mut deps = parse_dependencies(&PathBuf::from(file_path))?;
    deps.sort_by(|a, b| a.0.cmp(&b.0)); // Sort for deterministic assertion

    // Assert
    assert_eq!(deps.len(), 3, "Expected to find 3 dependencies");
    assert_eq!(deps[0], ("serde".to_string(), "1.0.190".to_string()));
    assert_eq!(deps[1], ("thiserror".to_string(), "1.0".to_string()));
    assert_eq!(deps[2], ("tokio".to_string(), "1.35.1".to_string()));

    Ok(())
}

#[test]
fn test_parse_dependencies_file_not_found() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("non_existent_cargo.toml");

    // Act
    let result = parse_dependencies(&file_path);

    // Assert
    assert!(result.is_err());
    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("Failed to read Cargo.toml"),
        "Unexpected error message: {}",
        error_message
    );
}

#[test]
fn test_parse_dependencies_malformed_toml() {
    // Arrange
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("Cargo.toml");
    let mut file = File::create(&file_path).unwrap();
    // This is invalid TOML because of the trailing 'a'.
    file.write_all(b"[dependencies]\nserde = \"1.0\"a").unwrap();

    // Act
    let result = parse_dependencies(&file_path);

    // Assert
    assert!(result.is_err());
    let error_message = result.unwrap_err().to_string();
    assert!(
        error_message.contains("Failed to parse TOML"),
        "Unexpected error message: {}",
        error_message
    );
}

#[test]
fn test_parse_dependencies_no_dependencies_section() -> Result<()> {
    // Arrange
    let dir = tempdir()?;
    let file_path = dir.path().join("Cargo.toml");
    let mut file = File::create(&file_path)?;
    let content = r#"[package]
name = "test"
version = "0.1.0"
edition = "2021"
"#;
    file.write_all(content.as_bytes())?;

    // Act
    let deps = parse_dependencies(&file_path)?;

    // Assert
    assert!(
        deps.is_empty(),
        "Expected no dependencies to be found when the section is missing"
    );

    Ok(())
}

#[test]
fn test_mcp_json_formatting() -> Result<()> {
    // Arrange
    let search_results = vec![
        anyrag::SearchResult {
            title: "handle1".to_string(),
            link: "file1.rs".to_string(),
            description: "content1".to_string(),
            score: 0.9,
        },
        anyrag::SearchResult {
            title: "handle2".to_string(),
            link: "file2.rs".to_string(),
            description: "content2".to_string(),
            score: 0.8,
        },
    ];

    // Act
    let json_output = format_mcp_response(search_results)?;
    let parsed: McpSuccessResponse = serde_json::from_str(&json_output)?;

    // Assert
    assert_eq!(parsed.results.len(), 2);
    assert_eq!(
        parsed.results[0],
        McpSearchResult {
            source_file: "file1.rs".to_string(),
            handle: "handle1".to_string(),
            content: "content1".to_string(),
            score: 0.9,
        }
    );
    assert_eq!(
        parsed.results[1],
        McpSearchResult {
            source_file: "file2.rs".to_string(),
            handle: "handle2".to_string(),
            content: "content2".to_string(),
            score: 0.8,
        }
    );

    Ok(())
}
