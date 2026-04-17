use anyhow::Result;
use std::process::Command;

use crate::config;
use crate::ui;

use super::filter_repos;

pub fn run(command: &str, repos: &[String]) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;
    let selected = filter_repos(&cfg, repos)?;

    let apps_dir = root.join(&cfg.workspace.apps_dir);

    let total = selected.len() as u32;
    let mut succeeded: u32 = 0;
    let mut failed: u32 = 0;
    let mut skipped: u32 = 0;

    for (name, _repo) in &selected {
        let repo_path = apps_dir.join(name);

        if !repo_path.exists() {
            ui::skip(&format!("{} — not cloned", name));
            skipped += 1;
            continue;
        }

        ui::header(&format!("[{}]", name));

        let output = Command::new("sh")
            .arg("-c")
            .arg(command)
            .current_dir(&repo_path)
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let stderr = String::from_utf8_lossy(&out.stderr);

                if !stdout.is_empty() {
                    eprint!("{}", stdout);
                }
                if !stderr.is_empty() {
                    eprint!("{}", stderr);
                }

                if out.status.success() {
                    succeeded += 1;
                } else {
                    let code = out.status.code().unwrap_or(-1);
                    ui::error(&format!("{} exited with code {}", name, code));
                    failed += 1;
                }
            }
            Err(e) => {
                ui::error(&format!("{}: failed to execute — {}", name, e));
                failed += 1;
            }
        }
    }

    eprintln!();
    ui::summary(total, succeeded, skipped, failed);
    Ok(())
}
