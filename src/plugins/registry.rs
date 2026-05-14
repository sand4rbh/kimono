use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

pub const DEFAULT_REGISTRY_URL: &str = "https://github.com/sand4rbh/kimono";

#[derive(Debug, Clone, Deserialize)]
pub struct Index {
    pub schema_version: u32,
    pub skills: Vec<SkillEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub path: String,
    #[serde(default)]
    pub default: Option<bool>,
}

/// Filesystem location of the registry cache.
/// `$XDG_CACHE_HOME/kimono/registry/` on Linux, or platform equivalent.
pub fn cache_dir() -> Result<PathBuf> {
    let base = dirs::cache_dir().context("could not determine user cache dir")?;
    Ok(base.join("kimono").join("registry"))
}

/// Parse a registry's `skills/index.json` from a cache root.
pub fn load_index(cache: &Path) -> Result<Index> {
    let path = cache.join("skills").join("index.json");
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let index: Index = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(index)
}

/// Ensure the registry cache exists and is up to date.
///
/// If the cache directory does not contain a git repo, shallow-clone `url`
/// into it. Otherwise, `git pull --ff-only` to refresh.
pub fn ensure_cache(url: &str) -> Result<PathBuf> {
    let cache = cache_dir()?;
    let cache_str = cache
        .to_str()
        .context("registry cache path is not valid UTF-8")?;
    if let Some(parent) = cache.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let git_dir = cache.join(".git");
    if git_dir.is_dir() {
        // Existing cache — pull
        let status = std::process::Command::new("git")
            .args(["-C", cache_str, "pull", "--ff-only", "--quiet"])
            .status()
            .with_context(|| format!("failed to run `git pull` in {}", cache.display()))?;
        if !status.success() {
            anyhow::bail!("`git pull` failed for registry cache at {}", cache.display());
        }
    } else {
        // Fresh clone
        if cache.exists() {
            // Non-git directory exists — remove it so clone can run
            std::fs::remove_dir_all(&cache)
                .with_context(|| format!("failed to remove stale cache {}", cache.display()))?;
        }
        let status = std::process::Command::new("git")
            .args(["clone", "--depth", "1", url, cache_str])
            .status()
            .context("failed to run `git clone` for registry")?;
        if !status.success() {
            anyhow::bail!("`git clone` failed for registry URL {}", url);
        }
    }
    Ok(cache)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_index_parses_valid_json() {
        let dir = TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();
        let json = r#"{
          "schema_version": 1,
          "skills": [
            {"name": "bootstrap", "description": "init", "version": "1.0.0", "tags": ["init"], "path": "skills/bootstrap", "default": true},
            {"name": "discover", "description": "scan", "version": "1.0.0", "tags": ["context"], "path": "skills/discover"}
          ]
        }"#;
        std::fs::write(skills_dir.join("index.json"), json).unwrap();

        let idx = load_index(dir.path()).unwrap();
        assert_eq!(idx.schema_version, 1);
        assert_eq!(idx.skills.len(), 2);
        assert_eq!(idx.skills[0].name, "bootstrap");
        assert_eq!(idx.skills[0].default, Some(true));
        assert_eq!(idx.skills[1].default, None);
    }

    #[test]
    fn test_load_index_missing_file_errors() {
        let dir = TempDir::new().unwrap();
        let result = load_index(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_dir_under_user_cache() {
        let dir = cache_dir().unwrap();
        let dir_str = dir.display().to_string();
        assert!(dir_str.contains("kimono"), "{dir_str}");
        assert!(dir_str.ends_with("registry"), "{dir_str}");
    }
}
