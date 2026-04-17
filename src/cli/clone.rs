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
    fs::create_dir_all(&apps_dir)?;

    let total = selected.len() as u32;
    let mut succeeded: u32 = 0;
    let mut skipped: u32 = 0;
    let mut failed: u32 = 0;

    ui::header(&format!("Cloning {} repos", total));

    for (name, repo) in &selected {
        let repo_path = apps_dir.join(name);

        if repo_path.exists() {
            ui::skip(&format!("{} already cloned", name));
            skipped += 1;
            continue;
        }

        let sp = ui::spinner(&format!("Cloning {}...", name));
        match git::clone(&repo.remote, &repo_path, &repo.branch) {
            Ok(()) => {
                sp.finish_and_clear();
                ui::success(&format!("{} cloned", name));
                succeeded += 1;
            }
            Err(e) => {
                sp.finish_and_clear();
                ui::error(&format!("{}: {}", name, e));
                failed += 1;
            }
        }
    }

    ui::summary(total, succeeded, skipped, failed);
    Ok(())
}
