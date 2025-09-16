//! # Extractor Tests
//!
//! This file contains tests for the `Extractor` module in the `github_ingest` crate.

use github::ingest::{extractor::Extractor, types::ExampleSourceType};
use std::fs::File;
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;

/// Helper function to create a file with specific content. Panics on error.
fn create_test_file(path: &Path, content: &str) {
    let mut file = File::create(path).expect("Failed to create test file");
    file.write_all(content.as_bytes())
        .expect("Failed to write to test file");
}

#[test]
fn test_extract_rust_code_blocks_from_readme() {
    // Arrange
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();
    let readme_path = repo_path.join("README.md");

    let readme_content = r#"
# My Project

Here is a Rust code example:
```rust
fn main() {
    println!("Hello, from README!");
}
```

And another one with extra whitespace:
```rust

let x = 5;
let y = 10;
assert_eq!(x + y, 15);

```

This is not a rust block:
```python
print("hello")
```
"#;
    create_test_file(&readme_path, readme_content);

    // Act
    let examples = Extractor::extract(repo_path, "v1.0.0").unwrap();

    // Assert
    assert_eq!(examples.len(), 2, "Expected to find 2 Rust code blocks.");

    let expected_code_1 = r#"fn main() {
    println!("Hello, from README!");
}"#;
    let expected_code_2 = r#"let x = 5;
let y = 10;
assert_eq!(x + y, 15);"#;

    // Check for presence of each code block, regardless of order.
    assert!(
        examples.iter().any(|e| e.content == expected_code_1),
        "The 'main' function example was not found"
    );
    assert!(
        examples.iter().any(|e| e.content == expected_code_2),
        "The 'x + y' example was not found"
    );

    // Verify all examples from this test are from the README.
    assert!(
        examples
            .iter()
            .all(|e| e.example_handle.contains("README.md"))
    );
}

#[test]
fn test_readme_with_no_rust_blocks() {
    // Arrange
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();
    let readme_path = repo_path.join("README.md");
    create_test_file(
        &readme_path,
        "# Title\n\nNo rust code here.\n```javascript\nconsole.log('hello');\n```",
    );

    // Act
    let examples = Extractor::extract(repo_path, "v1.0.0").unwrap();

    // Assert
    assert!(examples.is_empty(), "Expected no examples to be found.");
}

#[test]
fn test_empty_readme() {
    // Arrange
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();
    let readme_path = repo_path.join("README.md");
    create_test_file(&readme_path, "");

    // Act
    let examples = Extractor::extract(repo_path, "v1.0.0").unwrap();

    // Assert
    assert!(
        examples.is_empty(),
        "Expected no examples from an empty file."
    );
}

#[test]
fn test_extract_example_files() {
    // Arrange
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // Create an /examples directory
    let examples_dir = repo_path.join("examples");
    std::fs::create_dir(&examples_dir).expect("Failed to create examples dir");

    let example1_path = examples_dir.join("simple.rs");
    let example1_content = r#"fn main() {
    println!("This is a simple example.");
}"#;
    create_test_file(&example1_path, example1_content);

    let example2_path = examples_dir.join("another.rs");
    let example2_content = "pub fn do_something() -> bool { true }";
    create_test_file(&example2_path, example2_content);

    // Create a non-rs file that should be ignored
    let non_example_path = examples_dir.join("notes.txt");
    create_test_file(&non_example_path, "this is not a rust file");

    // Act
    let examples = Extractor::extract(repo_path, "v1.0.0").unwrap();

    // Assert
    assert_eq!(examples.len(), 2, "Expected to find 2 example files.");

    // Sort by handle to ensure consistent order
    let mut sorted_examples = examples;
    sorted_examples.sort_by(|a, b| a.example_handle.cmp(&b.example_handle));

    assert_eq!(sorted_examples[0].content, example2_content);
    assert_eq!(sorted_examples[0].source_file, "examples/another.rs");
    assert_eq!(
        sorted_examples[0].example_handle,
        "example_file:examples/another.rs"
    );

    assert_eq!(sorted_examples[1].content, example1_content);
    assert_eq!(sorted_examples[1].source_file, "examples/simple.rs");
    assert_eq!(
        sorted_examples[1].example_handle,
        "example_file:examples/simple.rs"
    );
}

#[test]
fn test_extract_from_doc_comments() {
    // Arrange
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();
    let src_dir = repo_path.join("src");
    std::fs::create_dir(&src_dir).unwrap();
    let lib_rs_path = src_dir.join("lib.rs");

    let lib_rs_content = r#"
//! # Crate Documentation
//!
//! This is a crate-level doc comment with an example.
//!
//! ```rust
//! let a = 1;
//! assert_eq!(a, 1);
//! ```

/// # Function Documentation
///
/// This is a function-level doc comment with an example.
///
/// ```rust
/// let b = 2;
/// assert_eq!(b, 2);
/// ```
fn some_function() {}
"#;
    create_test_file(&lib_rs_path, lib_rs_content);

    // Act
    let examples = Extractor::extract(repo_path, "v1.0.0").unwrap();

    // Assert
    assert_eq!(
        examples.len(),
        2,
        "Expected two examples from doc comments."
    );

    let mut sorted_examples = examples;
    sorted_examples.sort_by(|a, b| a.example_handle.cmp(&b.example_handle));

    let expected_code_1 = r#"let a = 1;
assert_eq!(a, 1);"#;
    assert_eq!(sorted_examples[0].content, expected_code_1);
    assert_eq!(
        sorted_examples[0].source_type,
        ExampleSourceType::DocComment
    );

    let expected_code_2 = r#"let b = 2;
assert_eq!(b, 2);"#;
    assert_eq!(sorted_examples[1].content, expected_code_2);
    assert_eq!(
        sorted_examples[1].source_type,
        ExampleSourceType::DocComment
    );
}

#[test]
fn test_extract_from_test_files() {
    // Arrange
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();
    let tests_dir = repo_path.join("tests");
    std::fs::create_dir(&tests_dir).unwrap();
    let test_file_path = tests_dir.join("integration_test.rs");

    let test_file_content = r#"
#[test]
fn simple_addition() {
    assert_eq!(2 + 2, 4);
}

// This should be ignored
fn helper_function() {}

#[test]
async fn async_test_example() {
    // some async setup
    let result = true;
    assert!(result);
}
"#;
    create_test_file(&test_file_path, test_file_content);

    // Act
    let examples = Extractor::extract(repo_path, "v1.0.0").unwrap();

    // Assert
    assert_eq!(
        examples.len(),
        2,
        "Expected two examples from test functions."
    );

    let mut sorted_examples = examples;
    sorted_examples.sort_by(|a, b| a.example_handle.cmp(&b.example_handle));

    assert_eq!(
        sorted_examples[0].content,
        "// some async setup\n    let result = true;\n    assert!(result);"
    );
    assert_eq!(
        sorted_examples[0].example_handle,
        "test:tests/integration_test.rs:async_test_example"
    );

    assert_eq!(sorted_examples[1].content, "assert_eq!(2 + 2, 4);");
    assert_eq!(
        sorted_examples[1].example_handle,
        "test:tests/integration_test.rs:simple_addition"
    );
}

#[test]
fn test_conflict_resolution_priority() {
    // Arrange
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let repo_path = temp_dir.path();

    // The same code block will be placed in multiple files with different source types.
    let common_code = "let a = 1;\nassert_eq!(a, 1);";

    // 1. README (lowest priority)
    let readme_path = repo_path.join("README.md");
    create_test_file(&readme_path, &format!("```rust\n{common_code}\n```"));

    // 2. Doc Comment (medium priority)
    let src_dir = repo_path.join("src");
    std::fs::create_dir(&src_dir).unwrap();
    let lib_rs_path = src_dir.join("lib.rs");
    create_test_file(
        &lib_rs_path,
        &format!(
            "/// ```rust\n/// {}\n/// ```\nfn dummy() {{}}",
            common_code.replace('\n', "\n/// ")
        ),
    );

    // 3. Test file (highest priority)
    let tests_dir = repo_path.join("tests");
    std::fs::create_dir(&tests_dir).unwrap();
    let test_file_path = tests_dir.join("test.rs");
    create_test_file(
        &test_file_path,
        &format!("#[test]\nfn my_test() {{\n{common_code}\n}}"),
    );

    // Act
    let examples = Extractor::extract(repo_path, "v1.0.0").unwrap();

    // Assert
    assert_eq!(
        examples.len(),
        1,
        "Expected only one example after conflict resolution."
    );

    let final_example = &examples[0];
    assert_eq!(
        final_example.source_type,
        ExampleSourceType::Test,
        "The final example should be from the highest priority source (Test)."
    );
    assert!(final_example.source_file.contains("tests/test.rs"));
    assert_eq!(final_example.content, common_code);
}
