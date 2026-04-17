use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

use crate::config;
use crate::git;
use crate::ui;

pub fn run(merged: bool, dry_run: bool) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;

    let wt_dir = root.join(&cfg.workspace.worktree_dir);

    if !wt_dir.is_dir() {
        ui::info("No worktrees found.");
        return Ok(());
    }

    // Collect all worktree entries
    let dir_entries = fs::read_dir(&wt_dir)
        .with_context(|| format!("failed to read worktree directory {}", wt_dir.display()))?;

    let mut candidates: Vec<(String, String, std::path::PathBuf)> = Vec::new();

    for entry in dir_entries.filter_map(|e| e.ok()) {
        let name = entry.file_name().to_string_lossy().to_string();
        if !entry.path().is_dir() {
            continue;
        }

        // Parse repo name and branch slug
        let (entry_repo, entry_branch_slug) = match name.split_once("--") {
            Some((r, b)) => (r.to_string(), b.to_string()),
            None => continue,
        };

        // Check that repo exists in config
        let repo_config = match cfg.repos.get(&entry_repo) {
            Some(rc) => rc,
            None => continue,
        };

        let repo_path = root.join(&cfg.workspace.apps_dir).join(&entry_repo);
        if !repo_path.exists() {
            continue;
        }

        let should_clean = if merged {
            // Check if the branch is merged into the default branch
            is_branch_merged(&repo_path, &entry_branch_slug, &repo_config.branch)
        } else {
            // Without --merged, check if the branch is gone from remote
            is_branch_gone_from_remote(&repo_path, &entry_branch_slug)
        };

        if should_clean {
            candidates.push((entry_repo, entry_branch_slug, entry.path()));
        }
    }

    if candidates.is_empty() {
        ui::info("No worktrees to clean up.");
        return Ok(());
    }

    candidates.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));

    let action = if dry_run { "Would remove" } else { "Cleaning" };
    ui::header(&format!("{} {} worktrees", action, candidates.len()));

    let mut succeeded: u32 = 0;
    let mut failed: u32 = 0;

    for (entry_repo, entry_branch_slug, wt_path) in &candidates {
        let dir_name = format!("{}--{}", entry_repo, entry_branch_slug);

        if dry_run {
            ui::info(&format!("  {} ({})", dir_name, wt_path.display()));
            succeeded += 1;
            continue;
        }

        let repo_path = root.join(&cfg.workspace.apps_dir).join(entry_repo);

        let sp = ui::spinner(&format!("Removing {}...", dir_name));

        match git::worktree_remove(&repo_path, wt_path, false) {
            Ok(()) => {
                sp.finish_and_clear();
                ui::success(&format!("{}: removed", dir_name));
                succeeded += 1;
            }
            Err(e) => {
                sp.finish_and_clear();
                ui::error(&format!("{}: {}", dir_name, e));
                failed += 1;
            }
        }
    }

    eprintln!();
    if dry_run {
        ui::info(&format!(
            "{} worktrees would be removed. Run without --dry-run to remove.",
            succeeded
        ));
    } else {
        ui::summary(succeeded + failed, succeeded, 0, failed);
    }

    Ok(())
}

/// Check if a branch (by slug) has been merged into the default branch.
/// We run `git branch --merged <default_branch>` and check if the branch
/// slug appears in the output.
fn is_branch_merged(repo_path: &std::path::Path, branch_slug: &str, default_branch: &str) -> bool {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["branch", "--merged", default_branch])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // Each line is like "  branch-name" or "* branch-name"
            // The branch slug has / replaced with -, so we check against that
            stdout.lines().any(|line| {
                let trimmed = line.trim().trim_start_matches("* ");
                // Compare slug of branch name to our slug
                git::branch_slug(trimmed) == branch_slug
            })
        }
        _ => false,
    }
}

/// Check if the branch is gone from the remote.
/// Fetch prune first, then check if `origin/<branch>` exists.
fn is_branch_gone_from_remote(repo_path: &std::path::Path, branch_slug: &str) -> bool {
    // Try to fetch with prune to update remote refs
    let _ = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["fetch", "--prune", "origin"])
        .output();

    // Check if there's a remote branch matching this slug
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["branch", "-r"])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // If no remote branch matches the slug, the branch is "gone"
            !stdout.lines().any(|line| {
                let trimmed = line.trim();
                // Remote branches look like "origin/feature/payments"
                if let Some(branch_name) = trimmed.strip_prefix("origin/") {
                    git::branch_slug(branch_name) == branch_slug
                } else {
                    false
                }
            })
        }
        _ => false,
    }
}
