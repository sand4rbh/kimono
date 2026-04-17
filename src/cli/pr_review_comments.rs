use anyhow::Result;

use crate::config;
use crate::github;
use crate::ui;

use super::find_worktrees;

pub fn run(repos: &[String], feature: Option<&str>) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;

    github::gh_available()?;

    let worktrees = find_worktrees(&root, &cfg, repos, feature)?;

    if worktrees.is_empty() {
        ui::warn("No worktrees found to check for review comments.");
        return Ok(());
    }

    ui::header(&format!("Review comments across {} repos", worktrees.len()));

    let mut total_comments: u32 = 0;
    let mut repos_with_comments: u32 = 0;
    let mut _skipped: u32 = 0;

    for (repo_name, wt_path) in &worktrees {
        // Get PR info
        let pr_info = match github::pr_view(wt_path) {
            Ok(Some(info)) => info,
            Ok(None) => {
                ui::skip(&format!("{}: no PR found", repo_name));
                _skipped += 1;
                continue;
            }
            Err(e) => {
                ui::error(&format!("{}: failed to get PR info: {}", repo_name, e));
                _skipped += 1;
                continue;
            }
        };

        // Get comments
        let comments = match github::pr_comments(wt_path, pr_info.number) {
            Ok(c) => c,
            Err(e) => {
                ui::error(&format!("{}: failed to fetch comments: {}", repo_name, e));
                continue;
            }
        };

        ui::header(&format!(
            "[{}] PR #{}: {}",
            repo_name, pr_info.number, pr_info.title
        ));

        if comments.is_empty() {
            ui::info("  No review comments.");
            continue;
        }

        repos_with_comments += 1;

        for comment in &comments {
            total_comments += 1;

            let location = if comment.path.is_empty() {
                "general".to_string()
            } else if let Some(line) = comment.line {
                format!("{}:{}", comment.path, line)
            } else {
                comment.path.clone()
            };

            eprintln!();
            ui::info(&format!("  @{} on {}", comment.author, location));

            // Print body with indentation
            for line in comment.body.lines() {
                ui::info(&format!("    {}", line));
            }
        }
    }

    eprintln!();
    if total_comments == 0 {
        ui::info("No review comments found.");
    } else {
        ui::info(&format!(
            "{} comment(s) across {} repo(s).",
            total_comments, repos_with_comments
        ));
    }

    Ok(())
}
