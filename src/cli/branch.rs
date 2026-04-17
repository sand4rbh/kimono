use anyhow::Result;
use console::style;
use std::path::Path;

use crate::config::{KimonoConfig, Repo};
use crate::git;
use crate::ui;

use super::filter_repos;

pub fn run(
    name: &str,
    repos: &[String],
    new: bool,
    list: bool,
    from_master: bool,
    no_checkout: bool,
) -> Result<()> {
    let config = crate::config::load_and_validate()?;
    let workspace_root = crate::config::workspace_root()?;
    let selected_repos = filter_repos(&config, repos)?;

    if list {
        run_list(name, &selected_repos, &workspace_root, &config)
    } else if from_master {
        run_from_master(name, &selected_repos, &workspace_root, &config, no_checkout)
    } else {
        run_create_or_checkout(name, &selected_repos, &workspace_root, &config, new)
    }
}

/// Mode 1: List branch existence across repos.
fn run_list(
    name: &str,
    repos: &[(&String, &Repo)],
    workspace_root: &Path,
    config: &KimonoConfig,
) -> Result<()> {
    ui::header(&format!("Branch '{}' across repos", name));

    let mut rows: Vec<Vec<String>> = Vec::new();

    for (repo_name, _repo) in repos {
        let repo_path = workspace_root
            .join(&config.workspace.apps_dir)
            .join(repo_name);

        if !repo_path.exists() {
            rows.push(vec![
                repo_name.to_string(),
                style("--").dim().to_string(),
                style("--").dim().to_string(),
                style("not cloned").dim().to_string(),
            ]);
            continue;
        }

        let local = match git::branch_exists_local(&repo_path, name) {
            Ok(true) => style("yes").green().to_string(),
            Ok(false) => style("no").dim().to_string(),
            Err(_) => style("error").red().to_string(),
        };

        let remote = match git::branch_exists_remote(&repo_path, name) {
            Ok(true) => style("yes").green().to_string(),
            Ok(false) => style("no").dim().to_string(),
            Err(_) => style("error").red().to_string(),
        };

        let current = match git::current_branch(&repo_path) {
            Ok(b) => b,
            Err(_) => "unknown".to_string(),
        };

        rows.push(vec![repo_name.to_string(), local, remote, current]);
    }

    ui::table(&["Repo", "Local", "Remote", "Current"], &rows);
    Ok(())
}

/// Mode 2: Create branch from remote default branch (--from-master).
fn run_from_master(
    name: &str,
    repos: &[(&String, &Repo)],
    workspace_root: &Path,
    config: &KimonoConfig,
    no_checkout: bool,
) -> Result<()> {
    ui::header(&format!("Creating branch '{}' from default branch", name));

    let mut succeeded: u32 = 0;
    let mut skipped: u32 = 0;
    let mut failed: u32 = 0;

    for (repo_name, repo) in repos {
        let repo_path = workspace_root
            .join(&config.workspace.apps_dir)
            .join(repo_name);

        if !repo_path.exists() {
            ui::skip(&format!("{}: not cloned, skipping", repo_name));
            skipped += 1;
            continue;
        }

        // Fetch first
        if let Err(e) = git::fetch(&repo_path) {
            ui::error(&format!("{}: fetch failed: {}", repo_name, e));
            failed += 1;
            continue;
        }

        // Create branch from origin/<default_branch>
        let base = format!("origin/{}", repo.branch);
        match git::create_branch(&repo_path, name, Some(&base)) {
            Ok(()) => {
                if no_checkout {
                    // checkout -b already left us on the new branch;
                    // switch back to default branch
                    if let Err(e) = git::checkout(&repo_path, &repo.branch) {
                        ui::warn(&format!(
                            "{}: branch created but failed to switch back to {}: {}",
                            repo_name, repo.branch, e
                        ));
                    }
                    ui::success(&format!(
                        "{}: created '{}' from '{}' (stayed on {})",
                        repo_name, name, base, repo.branch
                    ));
                } else {
                    ui::success(&format!(
                        "{}: created and checked out '{}' from '{}'",
                        repo_name, name, base
                    ));
                }
                succeeded += 1;
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("already exists") {
                    ui::skip(&format!("{}: branch '{}' already exists", repo_name, name));
                    skipped += 1;
                } else {
                    ui::error(&format!("{}: {}", repo_name, e));
                    failed += 1;
                }
            }
        }
    }

    let total = succeeded + skipped + failed;
    ui::summary(total, succeeded, skipped, failed);
    Ok(())
}

/// Mode 3: Create branch from HEAD (--new) or checkout existing branch.
fn run_create_or_checkout(
    name: &str,
    repos: &[(&String, &Repo)],
    workspace_root: &Path,
    config: &KimonoConfig,
    new: bool,
) -> Result<()> {
    if new {
        ui::header(&format!("Creating branch '{}' from HEAD", name));
    } else {
        ui::header(&format!("Checking out branch '{}'", name));
    }

    let mut succeeded: u32 = 0;
    let mut skipped: u32 = 0;
    let mut failed: u32 = 0;

    for (repo_name, _repo) in repos {
        let repo_path = workspace_root
            .join(&config.workspace.apps_dir)
            .join(repo_name);

        if !repo_path.exists() {
            ui::skip(&format!("{}: not cloned, skipping", repo_name));
            skipped += 1;
            continue;
        }

        if new {
            match git::create_branch(&repo_path, name, None) {
                Ok(()) => {
                    ui::success(&format!(
                        "{}: created and checked out '{}'",
                        repo_name, name
                    ));
                    succeeded += 1;
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    if err_msg.contains("already exists") {
                        ui::skip(&format!("{}: branch '{}' already exists", repo_name, name));
                        skipped += 1;
                    } else {
                        ui::error(&format!("{}: {}", repo_name, e));
                        failed += 1;
                    }
                }
            }
        } else {
            match git::checkout(&repo_path, name) {
                Ok(()) => {
                    ui::success(&format!("{}: checked out '{}'", repo_name, name));
                    succeeded += 1;
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    if err_msg.contains("did not match any")
                        || err_msg.contains("pathspec")
                        || err_msg.contains("not found")
                    {
                        ui::error(&format!(
                            "{}: branch '{}' not found (use --new to create)",
                            repo_name, name
                        ));
                    } else {
                        ui::error(&format!("{}: {}", repo_name, e));
                    }
                    failed += 1;
                }
            }
        }
    }

    let total = succeeded + skipped + failed;
    ui::summary(total, succeeded, skipped, failed);
    Ok(())
}
