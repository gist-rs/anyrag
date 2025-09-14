//! # Example Extractor
//!
//! This module is responsible for finding and extracting code examples from the
//! files of a cloned repository. It identifies potential source files based on
//! naming conventions and location, then parses them to extract code blocks.

use super::types::{ExampleSourceType, GeneratedExample, GitHubIngestError};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

/// A container for all discovered source files, categorized by their type.
#[derive(Default)]
struct DiscoveredSources {
    readmes: Vec<PathBuf>,
    example_files: Vec<PathBuf>,
    tests: Vec<PathBuf>,
    doc_comments: Vec<PathBuf>,
}

/// The main struct for the extraction process.
pub struct Extractor;

impl Extractor {
    /// Extracts all potential code examples from a given repository directory.
    pub fn extract(
        repo_path: &Path,
        version: &str,
    ) -> Result<Vec<GeneratedExample>, GitHubIngestError> {
        info!(
            "Starting example extraction from path: {}",
            repo_path.display()
        );

        let mut sources = DiscoveredSources::default();
        Self::discover_files_recursive(repo_path, &mut sources)?;

        info!(
            "Discovered {} READMEs, {} example files, {} tests, and {} source files for doc comments.",
            sources.readmes.len(),
            sources.example_files.len(),
            sources.tests.len(),
            sources.doc_comments.len()
        );

        let mut all_examples = Vec::new();

        // The extraction will happen in order of priority (lowest to highest).
        all_examples.extend(Self::parse_readme_files(
            repo_path,
            &sources.readmes,
            version,
        )?);
        all_examples.extend(Self::parse_example_files(
            repo_path,
            &sources.example_files,
            version,
        )?);
        all_examples.extend(Self::parse_doc_comments(
            repo_path,
            &sources.doc_comments,
            version,
        )?);
        all_examples.extend(Self::parse_test_files(repo_path, &sources.tests, version)?);

        info!(
            "Resolving conflicts for {} discovered examples.",
            all_examples.len()
        );
        let resolved_examples = Self::resolve_conflicts(all_examples);
        info!(
            "Conflict resolution complete. {} unique examples remain.",
            resolved_examples.len()
        );

        Ok(resolved_examples)
    }

    /// Recursively walks a directory to discover and categorize source files.
    fn discover_files_recursive(
        dir: &Path,
        sources: &mut DiscoveredSources,
    ) -> Result<(), GitHubIngestError> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_lowercase();

            // Skip hidden directories and files, especially .git
            if file_name.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                Self::discover_files_recursive(&path, sources)?;
            } else {
                let path_str = path.to_string_lossy();
                if file_name == "readme.md" {
                    sources.readmes.push(path.clone());
                } else if path_str.contains("/examples/") && file_name.ends_with(".rs") {
                    sources.example_files.push(path.clone());
                } else if (path_str.contains("/tests/") || file_name.ends_with("_test.rs"))
                    && file_name.ends_with(".rs")
                {
                    sources.tests.push(path.clone());
                } else if file_name.ends_with(".rs") {
                    sources.doc_comments.push(path.clone());
                }
            }
        }
        Ok(())
    }

    /// Parses `README.md` files to extract Rust code blocks.
    fn parse_readme_files(
        repo_path: &Path,
        files: &[PathBuf],
        version: &str,
    ) -> Result<Vec<GeneratedExample>, GitHubIngestError> {
        let mut examples = Vec::new();
        // Regex to find ```rust ... ``` blocks. `(?s)` enables `.` to match newlines.
        let re = Regex::new(r"(?s)```rust\s*\n(.*?)\n```")?;

        for file_path in files {
            let content = fs::read_to_string(file_path)?;
            let relative_path = file_path
                .strip_prefix(repo_path)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string();

            for (i, cap) in re.captures_iter(&content).enumerate() {
                if let Some(code_match) = cap.get(1) {
                    let code_block = code_match.as_str().trim().to_string();
                    if code_block.is_empty() {
                        continue;
                    }

                    // Estimate the line number by counting newlines before the match.
                    let line_number = content[..code_match.start()].lines().count() + 1;

                    examples.push(GeneratedExample {
                        example_handle: format!(
                            "{}:{}:{}:{}",
                            ExampleSourceType::Readme,
                            relative_path,
                            line_number,
                            i // Use capture index for uniqueness
                        ),
                        content: code_block,
                        source_file: relative_path.clone(),
                        source_type: ExampleSourceType::Readme,
                        version: version.to_string(),
                    });
                }
            }
        }
        Ok(examples)
    }

    /// Parses files from `/examples` directories, treating each file as a single example.
    fn parse_example_files(
        repo_path: &Path,
        files: &[PathBuf],
        version: &str,
    ) -> Result<Vec<GeneratedExample>, GitHubIngestError> {
        let mut examples = Vec::new();
        for file_path in files {
            let relative_path = file_path
                .strip_prefix(repo_path)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string();

            let content = fs::read_to_string(file_path)?;
            if content.trim().is_empty() {
                continue;
            }

            examples.push(GeneratedExample {
                example_handle: format!("{}:{}", ExampleSourceType::ExampleFile, relative_path),
                content,
                source_file: relative_path,
                source_type: ExampleSourceType::ExampleFile,
                version: version.to_string(),
            });
        }
        Ok(examples)
    }

    /// Parses Rust source files for doc comments containing Rust code blocks.
    fn parse_doc_comments(
        repo_path: &Path,
        files: &[PathBuf],
        version: &str,
    ) -> Result<Vec<GeneratedExample>, GitHubIngestError> {
        let mut examples = Vec::new();
        // This regex finds blocks of `///` or `//!` lines.
        // `(?m)` enables multi-line mode, so `^` matches the start of each line.
        let doc_block_re = Regex::new(r"(?m)((?:^\s*(?:///|//!)[^\n]*\n?)+)")?;
        // This regex finds ```rust ... ``` blocks inside a larger string.
        let code_block_re = Regex::new(r"(?s)```rust\s*\n(.*?)\n```")?;

        for file_path in files {
            let content = fs::read_to_string(file_path)?;
            let relative_path = file_path
                .strip_prefix(repo_path)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string();

            for doc_cap in doc_block_re.captures_iter(&content) {
                if let Some(doc_comment_block) = doc_cap.get(1) {
                    // Clean the prefixes from the doc comment block to treat it as Markdown.
                    let markdown_content = doc_comment_block
                        .as_str()
                        .lines()
                        .map(|line| {
                            line.trim_start()
                                .strip_prefix("///")
                                .or_else(|| line.trim_start().strip_prefix("//!"))
                                .map(|s| s.trim_start())
                                .unwrap_or(line) // Keep original line if no prefix
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

                    // Now find Rust code blocks within the cleaned Markdown content.
                    for (i, code_cap) in code_block_re.captures_iter(&markdown_content).enumerate()
                    {
                        if let Some(code_match) = code_cap.get(1) {
                            let code_block = code_match.as_str().trim().to_string();
                            if code_block.is_empty() {
                                continue;
                            }

                            let line_number =
                                content[..doc_comment_block.start()].lines().count() + 1;

                            examples.push(GeneratedExample {
                                example_handle: format!(
                                    "{}:{}:{}:{}",
                                    ExampleSourceType::DocComment,
                                    relative_path,
                                    line_number,
                                    i
                                ),
                                content: code_block,
                                source_file: relative_path.clone(),
                                source_type: ExampleSourceType::DocComment,
                                version: version.to_string(),
                            });
                        }
                    }
                }
            }
        }
        Ok(examples)
    }

    /// Parses Rust test files for functions annotated with `#[test]`.
    fn parse_test_files(
        repo_path: &Path,
        files: &[PathBuf],
        version: &str,
    ) -> Result<Vec<GeneratedExample>, GitHubIngestError> {
        let mut examples = Vec::new();
        // This regex captures the name and body of functions marked with `#[test]`.
        // It handles both sync and async test functions.
        let re = Regex::new(r"(?s)#\[test\]\s*(?:async\s+)?fn\s+(\w+)\s*\(\)\s*\{(.*?)\}")?;

        for file_path in files {
            let content = fs::read_to_string(file_path)?;
            let relative_path = file_path
                .strip_prefix(repo_path)
                .unwrap_or(file_path)
                .to_string_lossy()
                .to_string();

            for cap in re.captures_iter(&content) {
                if let (Some(fn_name), Some(fn_body)) = (cap.get(1), cap.get(2)) {
                    let code_block = fn_body.as_str().trim().to_string();
                    if code_block.is_empty() {
                        continue;
                    }

                    examples.push(GeneratedExample {
                        example_handle: format!(
                            "{}:{}:{}",
                            ExampleSourceType::Test,
                            relative_path,
                            fn_name.as_str()
                        ),
                        content: code_block,
                        source_file: relative_path.clone(),
                        source_type: ExampleSourceType::Test,
                        version: version.to_string(),
                    });
                }
            }
        }
        Ok(examples)
    }
    /// Resolves conflicts by keeping only the highest-priority source for identical code blocks.
    fn resolve_conflicts(examples: Vec<GeneratedExample>) -> Vec<GeneratedExample> {
        let mut best_examples: HashMap<String, GeneratedExample> = HashMap::new();

        for example in examples {
            // Normalize content by trimming whitespace to improve matching of code blocks.
            let key = example.content.trim().to_string();
            if key.is_empty() {
                continue;
            }

            best_examples
                .entry(key)
                .and_modify(|existing| {
                    // The `Ord` derive on `ExampleSourceType` ensures Test > DocComment > etc.
                    if example.source_type > existing.source_type {
                        info!(
                            "Conflict resolved: Upgrading example from '{}' ({:?}) to '{}' ({:?})",
                            existing.source_file,
                            existing.source_type,
                            &example.source_file,
                            &example.source_type
                        );
                        *existing = example.clone();
                    }
                })
                .or_insert(example);
        }

        best_examples.into_values().collect()
    }
}
