pub mod schema;
pub use schema::*;

use anyhow::{Context, Result};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Load and parse a KimonoConfig from the given YAML file path.
pub fn load(path: &Path) -> Result<KimonoConfig> {
    let contents = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read config file at {}", path.display()))?;
    let config: KimonoConfig = serde_yaml::from_str(&contents)
        .with_context(|| format!("failed to parse YAML in {}", path.display()))?;
    Ok(config)
}

/// Walk up from the current directory looking for `.kimono/config.yml`.
/// Returns the path to the config file if found.
pub fn find_config() -> Result<PathBuf> {
    let mut dir = std::env::current_dir().context("failed to determine current directory")?;
    loop {
        let candidate = dir.join(".kimono").join("config.yml");
        if candidate.is_file() {
            return Ok(candidate);
        }
        if !dir.pop() {
            break;
        }
    }
    anyhow::bail!(
        "could not find .kimono/config.yml in the current directory or any parent directory. \
         Run `kimono init` to create a new workspace."
    )
}

/// Validate the config for logical consistency.
///
/// Checks:
/// - No circular dependencies between repos
/// - All `depends_on` entries reference existing repo keys
/// - Every repo has a non-empty `remote`
pub fn validate(config: &KimonoConfig) -> Result<()> {
    let repo_names: HashSet<&str> = config.repos.keys().map(|s| s.as_str()).collect();

    for (name, repo) in &config.repos {
        // Non-empty remote
        if repo.remote.trim().is_empty() {
            anyhow::bail!("repo '{}' has an empty remote URL", name);
        }

        // All depends_on refs must exist
        for dep in &repo.depends_on {
            if !repo_names.contains(dep.as_str()) {
                anyhow::bail!(
                    "repo '{}' depends on '{}', but '{}' is not defined in the config",
                    name,
                    dep,
                    dep
                );
            }
        }
    }

    // Circular dependency check via DFS
    for start in config.repos.keys() {
        let mut visited = HashSet::new();
        let mut stack = HashSet::new();
        if has_cycle(start, &config.repos, &mut visited, &mut stack) {
            anyhow::bail!("circular dependency detected involving repo '{}'", start);
        }
    }

    Ok(())
}

/// Depth-first cycle detection.
fn has_cycle(
    node: &str,
    repos: &std::collections::HashMap<String, schema::Repo>,
    visited: &mut HashSet<String>,
    stack: &mut HashSet<String>,
) -> bool {
    if stack.contains(node) {
        return true;
    }
    if visited.contains(node) {
        return false;
    }
    visited.insert(node.to_string());
    stack.insert(node.to_string());

    if let Some(repo) = repos.get(node) {
        for dep in &repo.depends_on {
            if has_cycle(dep, repos, visited, stack) {
                return true;
            }
        }
    }

    stack.remove(node);
    false
}

/// Return the workspace root directory (parent of `.kimono/`).
pub fn workspace_root() -> Result<PathBuf> {
    let config_path = find_config()?;
    // config_path is `.kimono/config.yml`, parent is `.kimono/`, grandparent is workspace root
    let root = config_path
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| anyhow::anyhow!("unexpected config path structure"))?;
    Ok(root.to_path_buf())
}

/// Convenience: find config, load it, validate it, and return.
pub fn load_and_validate() -> Result<KimonoConfig> {
    let path = find_config()?;
    let config = load(&path)?;
    validate(&config)?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// Helper to build a KimonoConfig from a list of (name, remote, depends_on) tuples.
    fn make_config(repos: Vec<(&str, &str, Vec<&str>)>) -> KimonoConfig {
        let mut map = HashMap::new();
        for (name, remote, deps) in repos {
            map.insert(
                name.to_string(),
                schema::Repo {
                    remote: remote.to_string(),
                    branch: "main".to_string(),
                    description: None,
                    tech: vec![],
                    package_manager: None,
                    depends_on: deps.into_iter().map(|s| s.to_string()).collect(),
                    commands: HashMap::new(),
                },
            );
        }
        KimonoConfig {
            workspace: schema::Workspace {
                name: "test".to_string(),
                apps_dir: "apps".to_string(),
                worktree_dir: ".worktrees".to_string(),
            },
            repos: map,
            plugins: None,
        }
    }

    #[test]
    fn test_validate_catches_circular_deps() {
        let config = make_config(vec![
            ("a", "git@github.com:x/a.git", vec!["b"]),
            ("b", "git@github.com:x/b.git", vec!["a"]),
        ]);
        let result = validate(&config);
        assert!(result.is_err(), "should detect circular dependency");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("circular dependency"),
            "error message should mention circular dependency, got: {}",
            msg
        );
    }

    #[test]
    fn test_validate_catches_missing_dep_ref() {
        let config = make_config(vec![("a", "git@github.com:x/a.git", vec!["nonexistent"])]);
        let result = validate(&config);
        assert!(result.is_err(), "should catch missing dependency reference");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("nonexistent"),
            "error should mention the missing dep, got: {}",
            msg
        );
    }

    #[test]
    fn test_validate_catches_empty_remote() {
        let config = make_config(vec![("a", "", vec![])]);
        let result = validate(&config);
        assert!(result.is_err(), "should catch empty remote");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("empty remote"),
            "error should mention empty remote, got: {}",
            msg
        );
    }

    #[test]
    fn test_validate_passes_valid_config() {
        let config = make_config(vec![
            ("backend", "git@github.com:x/backend.git", vec![]),
            ("frontend", "git@github.com:x/frontend.git", vec!["backend"]),
            ("mobile", "git@github.com:x/mobile.git", vec!["backend"]),
        ]);
        let result = validate(&config);
        assert!(
            result.is_ok(),
            "valid config should pass validation: {:?}",
            result
        );
    }
}
