use anyhow::Result;
use std::fs;

use crate::config;
use crate::git;
use crate::ui;

use super::filter_repos;

pub fn run(repos: &[String]) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;
    let selected = filter_repos(&cfg, repos)?;

    let apps_dir = root.join(&cfg.workspace.apps_dir);
    let worktree_dir = root.join(&cfg.workspace.worktree_dir);

    ui::header(&format!(
        "Workspace: {} ({} repos)",
        cfg.workspace.name,
        selected.len()
    ));

    for (name, _repo) in &selected {
        let repo_path = apps_dir.join(name);

        if !repo_path.exists() {
            ui::skip(&format!("{} — not cloned", name));
            continue;
        }

        let branch = git::current_branch(&repo_path).unwrap_or_else(|_| "???".to_string());
        let clean = git::is_clean(&repo_path).unwrap_or(false);
        let (ahead, behind) = git::ahead_behind(&repo_path).unwrap_or((0, 0));

        // Count worktrees for this repo by scanning the worktree directory
        let wt_count = count_worktrees_for_repo(&worktree_dir, name);

        let line = ui::format_repo_status(name, &branch, clean, ahead, behind, wt_count);
        eprintln!("{}", line);
    }

    // List worktrees if the worktree directory exists
    if worktree_dir.is_dir() {
        let has_worktrees = list_worktrees(&worktree_dir, &selected);
        if has_worktrees {
            // header already printed inside list_worktrees
        }
    }

    Ok(())
}

/// Count worktree directories matching `<repo>--*` in the worktree dir.
fn count_worktrees_for_repo(worktree_dir: &std::path::Path, repo_name: &str) -> usize {
    let prefix = format!("{}--", repo_name);
    match fs::read_dir(worktree_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|n| n.starts_with(&prefix))
                    .unwrap_or(false)
            })
            .count(),
        Err(_) => 0,
    }
}

/// Scan the worktree directory for entries matching `<repo>--*` and print them.
/// Returns true if any worktrees were printed.
fn list_worktrees(
    worktree_dir: &std::path::Path,
    selected: &[(&String, &crate::config::Repo)],
) -> bool {
    let repo_names: Vec<&str> = selected.iter().map(|(n, _)| n.as_str()).collect();

    let mut entries: Vec<(String, std::path::PathBuf)> = Vec::new();

    if let Ok(dir) = fs::read_dir(worktree_dir) {
        for entry in dir.filter_map(|e| e.ok()) {
            let name = entry.file_name().to_string_lossy().to_string();
            // Check if this worktree belongs to one of our selected repos
            let belongs = repo_names
                .iter()
                .any(|repo| name.starts_with(&format!("{}--", repo)));
            if belongs && entry.path().is_dir() {
                entries.push((name, entry.path()));
            }
        }
    }

    if entries.is_empty() {
        return false;
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));

    ui::header("Worktrees");

    for (name, path) in &entries {
        let branch = git::current_branch(path).unwrap_or_else(|_| "???".to_string());
        let (ahead, _behind) = git::ahead_behind(path).unwrap_or((0, 0));
        let status = if ahead > 0 {
            format!("{} ahead", ahead)
        } else {
            "up to date".to_string()
        };
        let line = ui::format_worktree_entry(name, &branch, &status);
        eprintln!("{}", line);
    }

    true
}
