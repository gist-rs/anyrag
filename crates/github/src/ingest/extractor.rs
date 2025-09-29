//! # Example Extractor
//!
//! This module is responsible for finding and extracting code examples from the
//! files of a cloned repository. It identifies potential source files based on
//! naming conventions and location, then parses them to extract code blocks.

use super::types::{ExampleSourceType, GeneratedExample, GitHubIngestError};
use glob::Pattern;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;

/// A container for all discovered source files, categorized by their type.
#[derive(Default)]
struct DiscoveredSources {
    readmes: Vec<PathBuf>,
    text_files: Vec<PathBuf>,
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
        extract_included_files: bool,
    ) -> Result<Vec<GeneratedExample>, GitHubIngestError> {
        info!(
            "Starting example extraction from path: {}",
            repo_path.display()
        );

        let mut sources = DiscoveredSources::default();
        Self::discover_files_recursive(repo_path, &mut sources)?;

        info!(
            "Discovered {} READMEs, {} text files, {} example files, {} tests, and {} source files for doc comments.",
            sources.readmes.len(),
            sources.text_files.len(),
            sources.example_files.len(),
            sources.tests.len(),
            sources.doc_comments.len()
        );

        let mut all_examples = Vec::new();

        // The extraction will happen in order of priority (lowest to highest),
        // matching the Ord derive on ExampleSourceType.
        all_examples.extend(Self::parse_readme_files(
            repo_path,
            &sources.readmes,
            version,
        )?);
        all_examples.extend(Self::parse_text_files(
            repo_path,
            &sources.text_files,
            version,
        )?);
        all_examples.extend(Self::parse_example_files(
            repo_path,
            &sources.example_files,
            version,
            extract_included_files,
        )?);
        all_examples.extend(Self::parse_doc_comments(
            repo_path,
            &sources.doc_comments,
            version,
            extract_included_files,
        )?);
        all_examples.extend(Self::parse_test_files(
            repo_path,
            &sources.tests,
            version,
            extract_included_files,
        )?);

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

    /// Recursively walks a directory to discover and categorize source files for 'examples' dump.
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

            if file_name.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                Self::discover_files_recursive(&path, sources)?;
            } else {
                let path_str = path.to_string_lossy();
                if file_name == "readme.md" {
                    sources.readmes.push(path.clone());
                } else if (path_str.contains("/tests/") || file_name.ends_with("_test.rs"))
                    && file_name.ends_with(".rs")
                {
                    sources.tests.push(path.clone());
                } else if path_str.contains("/examples/") && file_name.ends_with(".rs") {
                    sources.example_files.push(path.clone());
                } else if file_name == "cargo.toml"
                    || file_name.ends_with(".txt")
                    || file_name.ends_with(".json")
                    || file_name.ends_with(".yaml")
                    || file_name.ends_with(".yml")
                {
                    sources.text_files.push(path.clone());
                } else if file_name.ends_with(".rs") {
                    sources.doc_comments.push(path.clone());
                }
            }
        }
        Ok(())
    }

    /// Extracts all source files from a repository for the 'src' dump, applying ignore patterns.
    pub fn extract_all_sources(
        repo_path: &Path,
        ignore_patterns: &[String],
    ) -> Result<Vec<(PathBuf, String)>, GitHubIngestError> {
        info!(
            "Starting source file extraction from path: {} with ignore patterns: {:?}",
            repo_path.display(),
            ignore_patterns
        );

        let default_patterns = ["*.lock", "LICENSE*"];
        let all_patterns_str: Vec<String> = default_patterns
            .iter()
            .map(|s| s.to_string())
            .chain(ignore_patterns.iter().cloned())
            .collect();

        let compiled_patterns: Vec<Pattern> = all_patterns_str
            .iter()
            .filter_map(|s| match Pattern::new(s) {
                Ok(p) => Some(p),
                Err(e) => {
                    info!("Invalid glob pattern '{}': {}", s, e);
                    None
                }
            })
            .collect();

        let mut files = Vec::new();
        Self::discover_all_source_files_recursive(
            repo_path,
            repo_path,
            &mut files,
            &compiled_patterns,
        )?;

        let mut results = Vec::new();
        for file_path in files {
            let relative_path = file_path.strip_prefix(repo_path).unwrap_or(&file_path);
            match fs::read_to_string(&file_path) {
                Ok(content) => {
                    results.push((relative_path.to_path_buf(), content));
                }
                Err(e) => {
                    info!(
                        "Skipping file {} due to read error: {}",
                        file_path.display(),
                        e
                    );
                }
            }
        }
        Ok(results)
    }

    /// Recursively walks a directory to discover all non-hidden source files, checking against ignore patterns.
    fn discover_all_source_files_recursive(
        base_dir: &Path,
        current_dir: &Path,
        files: &mut Vec<PathBuf>,
        ignore_patterns: &[Pattern],
    ) -> Result<(), GitHubIngestError> {
        if !current_dir.is_dir() {
            return Ok(());
        }

        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = entry.file_name();
            let file_name_lossy = file_name.to_string_lossy();
            let file_name = file_name_lossy.as_ref();

            if file_name.starts_with('.') || path.to_string_lossy().contains("/.git/") {
                continue;
            }

            if path.is_dir() {
                Self::discover_all_source_files_recursive(base_dir, &path, files, ignore_patterns)?;
            } else {
                let relative_path = path.strip_prefix(base_dir).unwrap_or(&path);

                let is_ignored_by_glob = ignore_patterns
                    .iter()
                    .any(|p| p.matches_path(relative_path));

                let has_no_extension = path.extension().is_none()
                    && !path.file_name().unwrap_or_default().eq("Makefile");

                if !is_ignored_by_glob && !has_no_extension {
                    files.push(path);
                } else if is_ignored_by_glob {
                    info!("Ignoring file due to glob pattern: {}", path.display());
                } else {
                    info!("Ignoring file with no extension: {}", path.display());
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
                    let line_number = content[..code_match.start()].lines().count() + 1;
                    examples.push(GeneratedExample {
                        example_handle: format!(
                            "{}:{}:{}:{}",
                            ExampleSourceType::Readme,
                            relative_path,
                            line_number,
                            i
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

    /// Parses generic text files (`.txt`, `.json`, etc.), treating each file as a single example.
    fn parse_text_files(
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
                example_handle: format!("{}:{}", ExampleSourceType::TextFile, relative_path),
                content,
                source_file: relative_path,
                source_type: ExampleSourceType::TextFile,
                version: version.to_string(),
            });
        }
        Ok(examples)
    }

    /// Parses files from `/examples` directories, treating each file as a single example.
    fn parse_example_files(
        repo_path: &Path,
        files: &[PathBuf],
        version: &str,
        extract_included_files: bool,
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
                content: content.clone(),
                source_file: relative_path,
                source_type: ExampleSourceType::ExampleFile,
                version: version.to_string(),
            });

            if extract_included_files {
                Self::add_included_bytes_examples(
                    repo_path,
                    file_path,
                    &content,
                    version,
                    &mut examples,
                )?;
            }
        }
        Ok(examples)
    }

    /// Parses Rust source files for doc comments containing Rust code blocks.
    fn parse_doc_comments(
        repo_path: &Path,
        files: &[PathBuf],
        version: &str,
        extract_included_files: bool,
    ) -> Result<Vec<GeneratedExample>, GitHubIngestError> {
        let mut examples = Vec::new();
        let doc_block_re = Regex::new(r"(?m)((?:^\s*(?:///|//!)[^\n]*\n?)+)")?;
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
                    let markdown_content = doc_comment_block
                        .as_str()
                        .lines()
                        .map(|line| {
                            line.trim_start()
                                .strip_prefix("///")
                                .or_else(|| line.trim_start().strip_prefix("//!"))
                                .map(|s| s.trim_start())
                                .unwrap_or(line)
                        })
                        .collect::<Vec<_>>()
                        .join("\n");

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
                                content: code_block.clone(),
                                source_file: relative_path.clone(),
                                source_type: ExampleSourceType::DocComment,
                                version: version.to_string(),
                            });

                            if extract_included_files {
                                Self::add_included_bytes_examples(
                                    repo_path,
                                    file_path,
                                    &code_block,
                                    version,
                                    &mut examples,
                                )?;
                            }
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
        extract_included_files: bool,
    ) -> Result<Vec<GeneratedExample>, GitHubIngestError> {
        let mut examples = Vec::new();
        let re = Regex::new(r#"(?s)#\[test\]\s*(?:async\s+)?fn\s+(\w+)\s*\(\)\s*\{(.*?)\}"#)?;

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
                        content: code_block.clone(),
                        source_file: relative_path.clone(),
                        source_type: ExampleSourceType::Test,
                        version: version.to_string(),
                    });

                    if extract_included_files {
                        Self::add_included_bytes_examples(
                            repo_path,
                            file_path,
                            &code_block,
                            version,
                            &mut examples,
                        )?;
                    }
                }
            }
        }
        Ok(examples)
    }

    /// Resolves conflicts by keeping only the highest-priority source for a given code block.
    fn resolve_conflicts(examples: Vec<GeneratedExample>) -> Vec<GeneratedExample> {
        let mut best_examples: HashMap<String, GeneratedExample> = HashMap::new();

        for example in examples {
            let normalized_content: String = example
                .content
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            let content_hash = format!("{:x}", md5::compute(normalized_content.as_bytes()));

            best_examples
                .entry(content_hash)
                .and_modify(|existing| {
                    if example.source_type > existing.source_type {
                        info!(
                            "Conflict resolved for content: Upgrading from {:?} in '{}' to {:?} in '{}'",
                            existing.source_type, existing.source_file, example.source_type, example.source_file
                        );
                        *existing = example.clone();
                    }
                })
                .or_insert(example);
        }

        best_examples.into_values().collect()
    }

    /// Helper to find `include_bytes!` macros, read the referenced files, and add them as examples.
    fn add_included_bytes_examples(
        repo_path: &Path,
        source_file_path: &Path,
        code_block: &str,
        version: &str,
        examples: &mut Vec<GeneratedExample>,
    ) -> Result<(), GitHubIngestError> {
        let re = Regex::new(r#"include_bytes!\("([^"]+)"\)"#)?;
        let source_dir = source_file_path.parent().unwrap_or(repo_path);

        for cap in re.captures_iter(code_block) {
            if let Some(relative_path_match) = cap.get(1) {
                let included_path_str = relative_path_match.as_str();
                let included_path = source_dir.join(included_path_str);

                if included_path.exists() {
                    let included_content = fs::read_to_string(&included_path)?;
                    if included_content.trim().is_empty() {
                        continue;
                    }

                    let relative_included_path = included_path
                        .strip_prefix(repo_path)
                        .unwrap_or(&included_path)
                        .to_string_lossy()
                        .to_string();

                    examples.push(GeneratedExample {
                        example_handle: format!(
                            "{}:{}",
                            ExampleSourceType::IncludedFile,
                            relative_included_path
                        ),
                        content: included_content,
                        source_file: relative_included_path,
                        source_type: ExampleSourceType::IncludedFile,
                        version: version.to_string(),
                    });
                }
            }
        }
        Ok(())
    }
}
