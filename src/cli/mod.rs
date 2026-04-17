pub mod add;
pub mod branch;
pub mod clone;
pub mod commit;
pub mod context;
pub mod create_pr;
pub mod exec;
pub mod init;
pub mod pr_review_comments;
pub mod remove;
pub mod status;
pub mod sync;
pub mod update_pr;
pub mod wt;

use std::path::{Path, PathBuf};

use crate::config::{KimonoConfig, Repo};
use crate::git;
use anyhow::{bail, Result};

/// Filter repos from config by the names the user provided on the CLI.
///
/// If `repos` is empty, returns all repos from the config.
/// If `repos` is non-empty, returns only the matching repos and errors if
/// any requested name is not found in the config.
pub fn filter_repos<'a>(
    config: &'a KimonoConfig,
    repos: &[String],
) -> Result<Vec<(&'a String, &'a Repo)>> {
    if repos.is_empty() {
        let mut all: Vec<(&String, &Repo)> = config.repos.iter().collect();
        all.sort_by_key(|(name, _)| name.as_str().to_owned());
        return Ok(all);
    }

    let mut result = Vec::new();
    for name in repos {
        match config.repos.get_key_value(name) {
            Some((k, v)) => result.push((k, v)),
            None => bail!(
                "repo '{}' not found in config. Available repos: {}",
                name,
                config.repos.keys().cloned().collect::<Vec<_>>().join(", ")
            ),
        }
    }
    Ok(result)
}

/// Find worktrees matching either a feature name, specific repo names, or all
/// worktrees that exist in the worktree directory.
///
/// When `feature` is Some, look for `<repo>--<feature_slug>` directories.
/// When `repos` is non-empty, look for worktrees belonging to those repos.
/// When both are empty, return all worktrees found in the worktree directory.
///
/// Also checks the main repo directories under apps_dir when no feature
/// worktrees are found (i.e., the user is working on branches inside the
/// main clones rather than in separate worktrees).
///
/// Returns `Vec<(repo_name, worktree_path)>`.
pub fn find_worktrees(
    workspace_root: &Path,
    config: &KimonoConfig,
    repos: &[String],
    feature: Option<&str>,
) -> Result<Vec<(String, PathBuf)>> {
    let wt_dir = workspace_root.join(&config.workspace.worktree_dir);
    let apps_dir = workspace_root.join(&config.workspace.apps_dir);
    let mut results: Vec<(String, PathBuf)> = Vec::new();

    if let Some(feat) = feature {
        // Feature mode: look for <repo>--<feature_slug> directories
        let slug = git::branch_slug(feat);

        let target_repos: Vec<String> = if repos.is_empty() {
            let mut all: Vec<String> = config.repos.keys().cloned().collect();
            all.sort();
            all
        } else {
            // Validate repos
            for r in repos {
                if !config.repos.contains_key(r) {
                    bail!(
                        "repo '{}' not found in config. Available repos: {}",
                        r,
                        config.repos.keys().cloned().collect::<Vec<_>>().join(", ")
                    );
                }
            }
            repos.to_vec()
        };

        for repo_name in &target_repos {
            let wt_path = wt_dir.join(format!("{}--{}", repo_name, slug));
            if wt_path.is_dir() {
                results.push((repo_name.clone(), wt_path));
            }
        }
    } else if !repos.is_empty() {
        // Repo mode: find worktrees for specific repos, or fall back to apps_dir
        for repo_name in repos {
            if !config.repos.contains_key(repo_name) {
                bail!(
                    "repo '{}' not found in config. Available repos: {}",
                    repo_name,
                    config.repos.keys().cloned().collect::<Vec<_>>().join(", ")
                );
            }

            // Check if there are any worktrees for this repo
            let mut found = false;
            if wt_dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&wt_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let name = entry.file_name().to_string_lossy().to_string();
                        if entry.path().is_dir() {
                            if let Some((r, _)) = name.split_once("--") {
                                if r == repo_name {
                                    results.push((repo_name.clone(), entry.path()));
                                    found = true;
                                }
                            }
                        }
                    }
                }
            }

            // Fall back to the main repo directory under apps_dir
            if !found {
                let repo_path = apps_dir.join(repo_name);
                if repo_path.is_dir() {
                    results.push((repo_name.clone(), repo_path));
                }
            }
        }
    } else {
        // All mode: find all worktrees, or fall back to all repos in apps_dir
        let mut found_any = false;
        if wt_dir.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&wt_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if entry.path().is_dir() {
                        if let Some((r, _)) = name.split_once("--") {
                            if config.repos.contains_key(r) {
                                results.push((r.to_string(), entry.path()));
                                found_any = true;
                            }
                        }
                    }
                }
            }
        }

        // If no worktrees found, use main repo dirs
        if !found_any {
            let mut all_repos: Vec<String> = config.repos.keys().cloned().collect();
            all_repos.sort();
            for repo_name in all_repos {
                let repo_path = apps_dir.join(&repo_name);
                if repo_path.is_dir() {
                    results.push((repo_name, repo_path));
                }
            }
        }
    }

    // Sort by repo name for consistent output
    results.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(results)
}
