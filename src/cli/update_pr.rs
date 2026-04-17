use anyhow::Result;

use crate::config;
use crate::git;
use crate::github;
use crate::ui;

use super::find_worktrees;

pub fn run(repos: &[String], feature: Option<&str>) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;

    github::gh_available()?;

    let worktrees = find_worktrees(&root, &cfg, repos, feature)?;

    if worktrees.is_empty() {
        ui::warn("No worktrees found to update PRs for.");
        return Ok(());
    }

    ui::header(&format!("Updating PRs across {} repos", worktrees.len()));

    let mut succeeded: u32 = 0;
    let mut skipped: u32 = 0;
    let mut failed: u32 = 0;

    for (repo_name, wt_path) in &worktrees {
        // Get current branch
        let branch = match git::current_branch(wt_path) {
            Ok(b) => b,
            Err(e) => {
                ui::error(&format!(
                    "{}: failed to get current branch: {}",
                    repo_name, e
                ));
                failed += 1;
                continue;
            }
        };

        // Get the default branch for this repo from config
        let default_branch = cfg
            .repos
            .get(repo_name)
            .map(|r| r.branch.as_str())
            .unwrap_or("main");

        // Skip if on the default branch
        if branch == default_branch {
            ui::skip(&format!(
                "{}: on default branch '{}', skipping",
                repo_name, branch
            ));
            skipped += 1;
            continue;
        }

        // Push latest commits
        let sp = ui::spinner(&format!("{}: pushing {}...", repo_name, branch));
        if let Err(e) = git::push(wt_path, &branch) {
            sp.finish_and_clear();
            ui::error(&format!("{}: push failed: {}", repo_name, e));
            failed += 1;
            continue;
        }
        sp.finish_and_clear();

        // Check if PR exists
        match github::pr_view(wt_path) {
            Ok(Some(pr)) => {
                ui::success(&format!(
                    "{}: PR #{} ({}) — {} — {}",
                    repo_name, pr.number, pr.state, pr.title, pr.url
                ));
                succeeded += 1;
            }
            Ok(None) => {
                ui::warn(&format!(
                    "{}: no PR found for branch '{}'. Use `create-pr` first.",
                    repo_name, branch
                ));
                skipped += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: failed to check PR: {}", repo_name, e));
                failed += 1;
            }
        }
    }

    let total = succeeded + skipped + failed;
    eprintln!();
    ui::summary(total, succeeded, skipped, failed);
    Ok(())
}
