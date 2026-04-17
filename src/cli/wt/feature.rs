use anyhow::{bail, Context, Result};
use std::collections::HashSet;
use std::fs;

use crate::config;
use crate::git;
use crate::ui;

pub fn run(name: &str, repos: &[String], new: bool, remove: bool) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;

    // Determine which repos to operate on
    let target_repos: Vec<String> = if repos.is_empty() {
        let mut all: Vec<String> = cfg.repos.keys().cloned().collect();
        all.sort();
        all
    } else {
        // Validate that all specified repos exist
        for r in repos {
            if !cfg.repos.contains_key(r) {
                bail!(
                    "repo '{}' not found in config. Available repos: {}",
                    r,
                    cfg.repos.keys().cloned().collect::<Vec<_>>().join(", ")
                );
            }
        }
        repos.to_vec()
    };

    if remove {
        run_remove(name, &target_repos, &root, &cfg)
    } else {
        run_create(name, &target_repos, &root, &cfg, new)
    }
}

/// Topological sort: repos with no dependencies first, then repos whose
/// dependencies are already sorted. Deps outside the feature set are ignored.
fn topo_sort(repos: &[String], cfg: &config::KimonoConfig) -> Vec<String> {
    let repo_set: HashSet<&str> = repos.iter().map(|s| s.as_str()).collect();
    let mut sorted: Vec<String> = Vec::new();
    let mut remaining: Vec<String> = repos.to_vec();

    // Simple iterative approach — keep pulling out repos whose deps are satisfied
    let max_iterations = remaining.len() + 1;
    for _ in 0..max_iterations {
        if remaining.is_empty() {
            break;
        }

        let sorted_set: HashSet<&str> = sorted.iter().map(|s| s.as_str()).collect();
        let mut next_batch: Vec<String> = Vec::new();

        for r in &remaining {
            if let Some(repo_config) = cfg.repos.get(r) {
                let deps_satisfied = repo_config.depends_on.iter().all(|dep| {
                    // Dep is satisfied if it's already sorted OR it's not in our feature set
                    sorted_set.contains(dep.as_str()) || !repo_set.contains(dep.as_str())
                });
                if deps_satisfied {
                    next_batch.push(r.clone());
                }
            } else {
                // Unknown repo — just add it
                next_batch.push(r.clone());
            }
        }

        if next_batch.is_empty() {
            // Can't make progress — add remaining in original order to avoid infinite loop
            sorted.append(&mut remaining);
            break;
        }

        for r in &next_batch {
            remaining.retain(|x| x != r);
        }
        sorted.extend(next_batch);
    }

    sorted
}

fn run_create(
    name: &str,
    repos: &[String],
    root: &std::path::Path,
    cfg: &config::KimonoConfig,
    new: bool,
) -> Result<()> {
    let sorted = topo_sort(repos, cfg);
    let slug = git::branch_slug(name);
    let wt_dir = root.join(&cfg.workspace.worktree_dir);

    // Ensure the worktree directory exists
    fs::create_dir_all(&wt_dir)
        .with_context(|| format!("failed to create worktree directory {}", wt_dir.display()))?;

    ui::header(&format!(
        "Creating feature '{}' across {} repos",
        name,
        sorted.len()
    ));

    let mut succeeded: u32 = 0;
    let mut skipped: u32 = 0;
    let mut failed: u32 = 0;
    let mut created_paths: Vec<(String, std::path::PathBuf)> = Vec::new();

    for repo in &sorted {
        let repo_config = match cfg.repos.get(repo) {
            Some(rc) => rc,
            None => {
                ui::error(&format!("{}: not found in config", repo));
                failed += 1;
                continue;
            }
        };

        let wt_path = wt_dir.join(format!("{}--{}", repo, slug));
        let repo_path = root.join(&cfg.workspace.apps_dir).join(repo);

        if !repo_path.exists() {
            ui::skip(&format!("{}: not cloned, skipping", repo));
            skipped += 1;
            continue;
        }

        if wt_path.exists() {
            ui::skip(&format!("{}: worktree already exists", repo));
            created_paths.push((repo.clone(), wt_path));
            skipped += 1;
            continue;
        }

        let sp = ui::spinner(&format!("{}...", repo));

        let result = if new {
            git::fetch(&repo_path).and_then(|()| {
                let base = format!("origin/{}", repo_config.branch);
                git::worktree_add(&repo_path, &wt_path, name, Some(&base))
            })
        } else {
            git::worktree_add(&repo_path, &wt_path, name, None)
        };

        sp.finish_and_clear();

        match result {
            Ok(()) => {
                ui::success(&format!("{}: worktree created", repo));
                created_paths.push((repo.clone(), wt_path));
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: {}", repo, e));
                failed += 1;
            }
        }
    }

    let total = succeeded + skipped + failed;
    eprintln!();
    ui::summary(total, succeeded, skipped, failed);

    if !created_paths.is_empty() {
        eprintln!();
        ui::info(&format!(
            "Feature '{}' ready across {} repos. cd commands:",
            name,
            created_paths.len()
        ));
        for (repo, path) in &created_paths {
            ui::info(&format!("  cd {}  # {}", path.display(), repo));
        }
    }

    Ok(())
}

fn run_remove(
    name: &str,
    repos: &[String],
    root: &std::path::Path,
    cfg: &config::KimonoConfig,
) -> Result<()> {
    let slug = git::branch_slug(name);
    let wt_dir = root.join(&cfg.workspace.worktree_dir);

    ui::header(&format!(
        "Removing feature '{}' across {} repos",
        name,
        repos.len()
    ));

    let mut succeeded: u32 = 0;
    let mut skipped: u32 = 0;
    let mut failed: u32 = 0;

    for repo in repos {
        let wt_path = wt_dir.join(format!("{}--{}", repo, slug));
        let repo_path = root.join(&cfg.workspace.apps_dir).join(repo);

        if !wt_path.exists() {
            ui::skip(&format!("{}: no worktree found", repo));
            skipped += 1;
            continue;
        }

        if !repo_path.exists() {
            ui::error(&format!(
                "{}: repo not cloned, cannot remove worktree",
                repo
            ));
            failed += 1;
            continue;
        }

        let sp = ui::spinner(&format!("{}...", repo));

        match git::worktree_remove(&repo_path, &wt_path, false) {
            Ok(()) => {
                sp.finish_and_clear();
                ui::success(&format!("{}: worktree removed", repo));
                succeeded += 1;
            }
            Err(e) => {
                sp.finish_and_clear();
                ui::error(&format!("{}: {}", repo, e));
                failed += 1;
            }
        }
    }

    let total = succeeded + skipped + failed;
    eprintln!();
    ui::summary(total, succeeded, skipped, failed);

    if succeeded > 0 {
        ui::info(&format!(
            "  Note: branch '{}' still exists in the repos. Use `git branch -d {}` to delete it.",
            name, name
        ));
    }

    Ok(())
}
