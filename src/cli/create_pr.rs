use anyhow::Result;

use crate::config;
use crate::git;
use crate::github;
use crate::ui;

use super::find_worktrees;

pub fn run(repos: &[String], feature: Option<&str>, draft: bool) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;

    github::gh_available()?;

    let worktrees = find_worktrees(&root, &cfg, repos, feature)?;

    if worktrees.is_empty() {
        ui::warn("No worktrees found to create PRs for.");
        return Ok(());
    }

    ui::header(&format!(
        "Creating PRs across {} repos{}",
        worktrees.len(),
        if draft { " (draft)" } else { "" }
    ));

    let mut succeeded: u32 = 0;
    let mut skipped: u32 = 0;
    let mut failed: u32 = 0;
    let mut pr_urls: Vec<(String, String)> = Vec::new();

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

        // Push first
        let sp = ui::spinner(&format!("{}: pushing {}...", repo_name, branch));
        if let Err(e) = git::push(wt_path, &branch) {
            sp.finish_and_clear();
            ui::error(&format!("{}: push failed: {}", repo_name, e));
            failed += 1;
            continue;
        }
        sp.finish_and_clear();

        // Build title and body
        let title = feature.unwrap_or(&branch).to_string();
        let body = build_pr_body(repo_name, &branch, feature, &worktrees);

        // Create PR
        match github::pr_create(wt_path, default_branch, &title, &body, draft) {
            Ok(url) => {
                ui::success(&format!("{}: PR created -> {}", repo_name, url));
                pr_urls.push((repo_name.clone(), url));
                succeeded += 1;
            }
            Err(e) => {
                let err_msg = e.to_string();
                if err_msg.contains("already exists") {
                    ui::skip(&format!(
                        "{}: PR already exists for branch '{}'",
                        repo_name, branch
                    ));
                    skipped += 1;
                } else {
                    ui::error(&format!("{}: PR creation failed: {}", repo_name, e));
                    failed += 1;
                }
            }
        }
    }

    let total = succeeded + skipped + failed;
    eprintln!();
    ui::summary(total, succeeded, skipped, failed);

    // Print summary of all PR URLs
    if !pr_urls.is_empty() {
        eprintln!();
        ui::header("Pull Requests");
        for (repo, url) in &pr_urls {
            ui::info(&format!("  {}: {}", repo, url));
        }
    }

    Ok(())
}

/// Build a PR body with cross-repo context.
fn build_pr_body(
    current_repo: &str,
    branch: &str,
    feature: Option<&str>,
    all_worktrees: &[(String, std::path::PathBuf)],
) -> String {
    let mut body = String::new();

    if let Some(feat) = feature {
        body.push_str(&format!("## Feature: {}\n\n", feat));
    }

    body.push_str(&format!("Branch: `{}`\n", branch));

    // List other repos in the same feature for cross-repo context
    let other_repos: Vec<&str> = all_worktrees
        .iter()
        .filter(|(name, _)| name != current_repo)
        .map(|(name, _)| name.as_str())
        .collect();

    if !other_repos.is_empty() {
        body.push_str("\n### Related repositories\n\n");
        body.push_str("This change is part of a multi-repo feature spanning:\n");
        for repo in &other_repos {
            body.push_str(&format!("- `{}`\n", repo));
        }
    }

    body
}
