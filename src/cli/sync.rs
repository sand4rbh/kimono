use anyhow::Result;

use crate::config;
use crate::git;
use crate::ui;

use super::filter_repos;

pub fn run(repos: &[String], fetch_only: bool) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;
    let selected = filter_repos(&cfg, repos)?;

    let apps_dir = root.join(&cfg.workspace.apps_dir);

    let total = selected.len() as u32;
    let mut succeeded: u32 = 0;
    let mut skipped: u32 = 0;
    let mut failed: u32 = 0;

    let mode = if fetch_only { "Fetching" } else { "Syncing" };
    ui::header(&format!("{} {} repos", mode, total));

    for (name, repo) in &selected {
        let repo_path = apps_dir.join(name);

        if !repo_path.exists() {
            ui::skip(&format!("{} — not cloned", name));
            skipped += 1;
            continue;
        }

        let sp = ui::spinner(&format!("Fetching {}...", name));
        if let Err(e) = git::fetch(&repo_path) {
            sp.finish_and_clear();
            ui::error(&format!("{}: fetch failed — {}", name, e));
            failed += 1;
            continue;
        }
        sp.finish_and_clear();

        if fetch_only {
            ui::success(&format!("{} fetched", name));
            succeeded += 1;
            continue;
        }

        // Check if on default branch
        let current = match git::current_branch(&repo_path) {
            Ok(b) => b,
            Err(e) => {
                ui::warn(&format!("{}: could not determine branch — {}", name, e));
                succeeded += 1; // fetch succeeded at least
                continue;
            }
        };

        if current != repo.branch {
            ui::warn(&format!(
                "{} on '{}', not default '{}' — skipping rebase",
                name, current, repo.branch
            ));
            succeeded += 1; // fetch succeeded
            continue;
        }

        // Check for clean working tree
        match git::is_clean(&repo_path) {
            Ok(true) => {}
            Ok(false) => {
                ui::warn(&format!(
                    "{} has uncommitted changes — skipping rebase",
                    name
                ));
                succeeded += 1; // fetch succeeded
                continue;
            }
            Err(e) => {
                ui::warn(&format!("{}: could not check status — {}", name, e));
                succeeded += 1;
                continue;
            }
        }

        // Rebase onto origin/default-branch
        let onto = format!("origin/{}", repo.branch);
        let sp = ui::spinner(&format!("Rebasing {}...", name));
        match git::rebase(&repo_path, &onto) {
            Ok(()) => {
                sp.finish_and_clear();
                ui::success(&format!("{} synced", name));
                succeeded += 1;
            }
            Err(e) => {
                sp.finish_and_clear();
                ui::error(&format!("{}: rebase failed — {}", name, e));
                failed += 1;
            }
        }
    }

    ui::summary(total, succeeded, skipped, failed);
    Ok(())
}
