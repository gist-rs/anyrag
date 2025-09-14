//! # Git Repository Crawler
//!
//! This module provides the functionality to clone a public Git repository
//! into a temporary local directory for analysis.

use super::types::{GitHubIngestError, IngestionTask};
use semver::Version;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{tempdir, TempDir};
use tokio::process::Command;
use tracing::{info, warn};

/// Represents the result of a successful crawl operation.
/// It holds a `TempDir` guard, which ensures that the temporary directory
/// is automatically cleaned up when this struct is dropped.
pub struct CrawlResult {
    pub temp_dir: TempDir,
    pub path: PathBuf,
    pub version: String, // The actual version that was checked out
}

/// The main crawler structure.
pub struct Crawler;

impl Crawler {
    /// Clones a Git repository for a given ingestion task and returns the path
    /// to the temporary directory where it was cloned.
    pub async fn crawl(task: &IngestionTask) -> Result<CrawlResult, GitHubIngestError> {
        info!("Starting crawl for repository: {}", task.url);
        let temp_dir = tempdir().map_err(GitHubIngestError::Io)?;
        let repo_path = temp_dir.path().to_path_buf();

        // 1. Clone the repository
        let clone_status = Command::new("git")
            .arg("clone")
            .arg("--depth")
            .arg("1") // Start with a shallow clone for speed
            .arg(&task.url)
            .arg(&repo_path)
            .status()
            .await
            .map_err(|e| GitHubIngestError::Git(format!("Failed to execute git clone: {e}")))?;

        if !clone_status.success() {
            return Err(GitHubIngestError::Git(
                "git clone command failed".to_string(),
            ));
        }
        info!("Successfully cloned repository to: {:?}", repo_path);

        let version = if let Some(version_spec) = &task.version {
            // 2. If a specific version is requested, fetch and check it out.
            // We need to unshallow the repo to fetch all tags/branches.
            let unshallow_status = Command::new("git")
                .arg("-C")
                .arg(&repo_path)
                .arg("fetch")
                .arg("--unshallow")
                .status()
                .await
                .map_err(|e| GitHubIngestError::Git(format!("Failed to unshallow repo: {e}")))?;

            if !unshallow_status.success() {
                warn!("Failed to unshallow the repository. Will proceed with the default branch.");
            } else {
                let fetch_tags_status = Command::new("git")
                    .arg("-C")
                    .arg(&repo_path)
                    .arg("fetch")
                    .arg("--tags")
                    .status()
                    .await
                    .map_err(|e| GitHubIngestError::Git(format!("Failed to fetch tags: {e}")))?;

                if !fetch_tags_status.success() {
                    warn!("Failed to fetch tags. Proceeding without them.");
                }
            }

            let checkout_status = Command::new("git")
                .arg("-C")
                .arg(&repo_path)
                .arg("checkout")
                .arg(version_spec)
                .status()
                .await
                .map_err(|e| {
                    GitHubIngestError::Git(format!("Failed to execute git checkout: {e}"))
                })?;

            if !checkout_status.success() {
                return Err(GitHubIngestError::Git(format!(
                    "git checkout command failed for version '{version_spec}'"
                )));
            }
            info!("Successfully checked out version: {}", version_spec);
            version_spec.clone()
        } else {
            // 3. If no version is specified, determine the latest version.
            info!("No version specified, attempting to determine the latest version.");

            // To find the latest tag, we need to fetch all tags first.
            let unshallow_status = Command::new("git")
                .arg("-C")
                .arg(&repo_path)
                .arg("fetch")
                .arg("--unshallow")
                .status()
                .await
                .map_err(|e| GitHubIngestError::Git(format!("Failed to unshallow repo: {e}")))?;

            if !unshallow_status.success() {
                warn!("Failed to unshallow the repository. Will proceed with the default branch commit hash.");
            }

            let fetch_tags_status = Command::new("git")
                .arg("-C")
                .arg(&repo_path)
                .arg("fetch")
                .arg("--tags")
                .status()
                .await
                .map_err(|e| GitHubIngestError::Git(format!("Failed to fetch tags: {e}")))?;

            if !fetch_tags_status.success() {
                warn!("Failed to fetch tags. Proceeding without them.");
            }

            let latest_tag = Self::get_latest_semver_tag(&repo_path).await?;

            if let Some(tag) = latest_tag {
                info!("Found latest semver tag: {}. Checking it out.", tag);
                let checkout_status = Command::new("git")
                    .arg("-C")
                    .arg(&repo_path)
                    .arg("checkout")
                    .arg(&tag)
                    .status()
                    .await
                    .map_err(|e| {
                        GitHubIngestError::Git(format!("Failed to execute git checkout: {e}"))
                    })?;

                if !checkout_status.success() {
                    warn!(
                        "Failed to checkout latest tag '{}'. Using HEAD commit hash instead.",
                        tag
                    );
                    Self::get_head_commit(&repo_path).await?
                } else {
                    info!("Successfully checked out latest tag: {}", tag);
                    tag
                }
            } else {
                info!("No semver tags found. Attempting to read version from Cargo.toml.");
                if let Some(version) = Self::get_version_from_cargo_toml(&repo_path).await? {
                    version
                } else {
                    info!("No version found in Cargo.toml. Using the latest commit on the default branch.");
                    Self::get_head_commit(&repo_path).await?
                }
            }
        };

        Ok(CrawlResult {
            temp_dir,
            path: repo_path,
            version,
        })
    }

    /// Gets the commit hash of the current HEAD.
    async fn get_head_commit(repo_path: &Path) -> Result<String, GitHubIngestError> {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("rev-parse")
            .arg("HEAD")
            .output()
            .await
            .map_err(|e| GitHubIngestError::Git(format!("Failed to get HEAD commit: {e}")))?;

        if !output.status.success() {
            return Err(GitHubIngestError::Git(
                "Failed to get HEAD commit hash".to_string(),
            ));
        }
        Ok(String::from_utf8(output.stdout).unwrap().trim().to_string())
    }

    /// Finds the latest semantic version tag in the repository.
    async fn get_latest_semver_tag(repo_path: &Path) -> Result<Option<String>, GitHubIngestError> {
        let output = Command::new("git")
            .arg("-C")
            .arg(repo_path)
            .arg("tag")
            .arg("-l")
            .arg("--sort=-v:refname") // Sorts by version name descending
            .output()
            .await
            .map_err(|e| GitHubIngestError::Git(format!("Failed to list tags: {e}")))?;

        if !output.status.success() {
            warn!("`git tag` command failed. Cannot determine latest tag.");
            return Ok(None);
        }

        let tags = String::from_utf8(output.stdout).unwrap();
        // Find the first tag that looks like a semver tag.
        for tag in tags.lines() {
            let trimmed_tag = tag.trim().strip_prefix('v').unwrap_or(tag.trim());
            if Version::parse(trimmed_tag).is_ok() {
                return Ok(Some(tag.trim().to_string()));
            }
        }

        Ok(None)
    }

    /// Reads the version from the `[package]` section of a `Cargo.toml` file.
    async fn get_version_from_cargo_toml(
        repo_path: &Path,
    ) -> Result<Option<String>, GitHubIngestError> {
        let cargo_toml_path = repo_path.join("Cargo.toml");
        if !cargo_toml_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(cargo_toml_path)?;
        let value: toml::Value = toml::from_str(&content).map_err(|e| {
            GitHubIngestError::VersionParsing(format!("Failed to parse Cargo.toml: {e}"))
        })?;

        if let Some(package) = value.get("package") {
            if let Some(version) = package.get("version") {
                if let Some(version_str) = version.as_str() {
                    info!("Found version in Cargo.toml: {}", version_str);
                    return Ok(Some(version_str.to_string()));
                }
            }
        }

        Ok(None)
    }
}
