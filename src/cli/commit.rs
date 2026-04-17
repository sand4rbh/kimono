use anyhow::{bail, Result};

use crate::config;
use crate::git;
use crate::ui;

use super::find_worktrees;

pub fn run(repos: &[String], feature: Option<&str>, message: Option<&str>) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;

    let worktrees = find_worktrees(&root, &cfg, repos, feature)?;

    if worktrees.is_empty() {
        ui::warn("No worktrees found to commit in.");
        return Ok(());
    }

    let msg = match message {
        Some(m) => m,
        None => bail!("commit message required (-m)"),
    };

    ui::header(&format!("Committing across {} repos", worktrees.len()));

    let mut succeeded: u32 = 0;
    let mut skipped: u32 = 0;
    let mut failed: u32 = 0;

    for (repo_name, wt_path) in &worktrees {
        // Check for staged changes
        let has_staged = match git::has_staged_changes(wt_path) {
            Ok(v) => v,
            Err(e) => {
                ui::error(&format!(
                    "{}: failed to check staged changes: {}",
                    repo_name, e
                ));
                failed += 1;
                continue;
            }
        };

        if !has_staged {
            ui::skip(&format!("{}: no staged changes", repo_name));
            skipped += 1;
            continue;
        }

        match git::commit(wt_path, msg) {
            Ok(()) => {
                ui::success(&format!("{}: committed", repo_name));
                succeeded += 1;
            }
            Err(e) => {
                ui::error(&format!("{}: commit failed: {}", repo_name, e));
                failed += 1;
            }
        }
    }

    let total = succeeded + skipped + failed;
    eprintln!();
    ui::summary(total, succeeded, skipped, failed);
    Ok(())
}
