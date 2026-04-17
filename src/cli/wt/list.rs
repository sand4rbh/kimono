use anyhow::Result;
use std::collections::HashMap;
use std::fs;

use crate::config;
use crate::git;
use crate::ui;

pub fn run(repo: Option<&str>) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;

    // Validate repo if provided
    if let Some(r) = repo {
        if !cfg.repos.contains_key(r) {
            anyhow::bail!(
                "repo '{}' not found in config. Available repos: {}",
                r,
                cfg.repos.keys().cloned().collect::<Vec<_>>().join(", ")
            );
        }
    }

    let apps_dir = root.join(&cfg.workspace.apps_dir);
    let wt_dir = root.join(&cfg.workspace.worktree_dir);

    if !wt_dir.is_dir() {
        ui::info("No worktrees found.");
        return Ok(());
    }

    // Canonicalize the workspace worktree dir so we can reliably compare
    // against the absolute paths returned by `git worktree list --porcelain`.
    let canonical_wt_dir = fs::canonicalize(&wt_dir).unwrap_or_else(|_| wt_dir.clone());

    // Read and filter directory entries
    let mut entries: Vec<(String, String, std::path::PathBuf)> = Vec::new();

    let dir_entries = fs::read_dir(&wt_dir)?;
    for entry in dir_entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if !entry.path().is_dir() {
            continue;
        }

        // Parse repo name and branch slug from "<repo>--<branch-slug>"
        let (entry_repo, entry_branch) = match name.split_once("--") {
            Some((r, b)) => (r.to_string(), b.to_string()),
            None => continue, // Skip entries that don't match the naming convention
        };

        // Filter by repo if specified
        if let Some(r) = repo {
            if entry_repo != r {
                continue;
            }
        }

        // Verify the repo is known in config
        if !cfg.repos.contains_key(&entry_repo) {
            continue;
        }

        entries.push((entry_repo, entry_branch, entry.path()));
    }

    if entries.is_empty() {
        if let Some(r) = repo {
            ui::info(&format!("No worktrees found for repo '{}'.", r));
        } else {
            ui::info("No worktrees found.");
        }
        return Ok(());
    }

    // Sort entries by repo name, then branch
    entries.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

    // For each distinct repo appearing in `entries`, query `git worktree list
    // --porcelain` once and build a map from canonicalized worktree path to
    // WorktreeInfo. This lets us report authoritative branch/commit info
    // straight from git rather than re-deriving it via per-worktree calls.
    let mut repo_names_present: Vec<String> = entries.iter().map(|(r, _, _)| r.clone()).collect();
    repo_names_present.sort();
    repo_names_present.dedup();

    let mut wt_info: HashMap<std::path::PathBuf, git::WorktreeInfo> = HashMap::new();
    for repo_name in &repo_names_present {
        let repo_path = apps_dir.join(repo_name);
        if !repo_path.is_dir() {
            continue;
        }
        let Ok(infos) = git::worktree_list(&repo_path) else {
            continue;
        };
        for info in infos {
            // Keep only worktrees that live under the workspace worktree dir;
            // the main checkout in apps/ and any stray external worktrees are
            // not relevant to `kimono wt list`.
            let canonical = fs::canonicalize(&info.path).unwrap_or_else(|_| info.path.clone());
            if canonical.starts_with(&canonical_wt_dir) {
                wt_info.insert(canonical, info);
            }
        }
    }

    // Print header
    let title = match repo {
        Some(r) => format!("Worktrees for '{}'", r),
        None => "Worktrees".to_string(),
    };
    ui::header(&title);

    // Print each entry with status info
    for (entry_repo, entry_branch, path) in &entries {
        let dir_name = format!("{}--{}", entry_repo, entry_branch);
        let canonical_path = fs::canonicalize(path).unwrap_or_else(|_| path.clone());

        // Prefer the branch name reported by git worktree list (authoritative,
        // handles slashes in branch names correctly). Fall back to the slug
        // parsed from the directory name.
        let branch = wt_info
            .get(&canonical_path)
            .map(|i| i.branch.clone())
            .unwrap_or_else(|| entry_branch.clone());

        let (ahead, _behind) = git::ahead_behind(path).unwrap_or((0, 0));
        let clean = git::is_clean(path).unwrap_or(true);

        let mut status_parts: Vec<String> = Vec::new();
        if !clean {
            status_parts.push("modified".to_string());
        }
        if ahead > 0 {
            status_parts.push(format!("{} ahead", ahead));
        }
        let status = if status_parts.is_empty() {
            "clean".to_string()
        } else {
            status_parts.join(", ")
        };

        let line = ui::format_worktree_entry(&dir_name, &branch, &status);
        eprintln!("{}", line);
    }

    // Group by branch slug to show "Features" summary
    let mut features: HashMap<String, Vec<String>> = HashMap::new();
    for (entry_repo, entry_branch, _) in &entries {
        features
            .entry(entry_branch.clone())
            .or_default()
            .push(entry_repo.clone());
    }

    // Only show features summary if there are branches spanning multiple repos
    let multi_repo_features: Vec<(&String, &Vec<String>)> = features
        .iter()
        .filter(|(_, repos)| repos.len() > 1)
        .collect();

    if !multi_repo_features.is_empty() {
        ui::header("Features (branches across multiple repos)");
        let mut sorted_features: Vec<_> = multi_repo_features;
        sorted_features.sort_by_key(|(name, _)| name.to_string());
        for (branch_slug, repos) in sorted_features {
            ui::info(&format!("  {} -> {}", branch_slug, repos.join(", ")));
        }
    }

    Ok(())
}
