use std::fs;

use anyhow::{bail, Context, Result};

use crate::config;
use crate::ui;

/// Remove a repository from the workspace.
///
/// - Removes the repo entry from `.kimono/config.yml`.
/// - If `--delete` is set, also removes the cloned directory and any worktrees.
/// - Regenerates context files.
pub fn run(name: &str, delete: bool) -> Result<()> {
    let config_path = config::find_config()?;
    let workspace_root = config_path
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| anyhow::anyhow!("unexpected config path structure"))?
        .to_path_buf();

    let mut cfg = config::load(&config_path)?;

    // Check that the repo exists
    if !cfg.repos.contains_key(name) {
        bail!(
            "repo '{}' not found in config. Available repos: {}",
            name,
            cfg.repos.keys().cloned().collect::<Vec<_>>().join(", ")
        );
    }

    // Remove from config
    cfg.repos.remove(name);

    // Also remove this repo from any depends_on lists in other repos
    for (_repo_name, repo) in cfg.repos.iter_mut() {
        repo.depends_on.retain(|dep| dep != name);
    }

    // Write updated config back to disk
    let yaml = serde_yaml::to_string(&cfg).context("failed to serialize config to YAML")?;
    fs::write(&config_path, &yaml)
        .with_context(|| format!("failed to write {}", config_path.display()))?;
    ui::success(&format!("Removed '{}' from .kimono/config.yml", name));

    if delete {
        // Remove the apps/<name>/ directory
        let repo_dir = workspace_root.join(&cfg.workspace.apps_dir).join(name);
        if repo_dir.is_dir() {
            fs::remove_dir_all(&repo_dir)
                .with_context(|| format!("failed to remove {}", repo_dir.display()))?;
            ui::success(&format!("Deleted {}", repo_dir.display()));
        }

        // Remove any worktrees matching <name>--*
        let wt_dir = workspace_root.join(&cfg.workspace.worktree_dir);
        if wt_dir.is_dir() {
            let prefix = format!("{}--", name);
            if let Ok(entries) = fs::read_dir(&wt_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let entry_name = entry.file_name().to_string_lossy().to_string();
                    if entry_name.starts_with(&prefix) && entry.path().is_dir() {
                        fs::remove_dir_all(entry.path()).with_context(|| {
                            format!("failed to remove worktree {}", entry.path().display())
                        })?;
                        ui::success(&format!("Deleted worktree {}", entry_name));
                    }
                }
            }
        }

        // Remove agent/skill directories for this repo
        let claude_dir = workspace_root.join(".claude");
        let agent_dir = claude_dir.join("agents").join(name);
        if agent_dir.is_dir() {
            fs::remove_dir_all(&agent_dir)?;
            ui::success(&format!("Deleted .claude/agents/{}", name));
        }
        let skill_dir = claude_dir.join("skills").join(name);
        if skill_dir.is_dir() {
            fs::remove_dir_all(&skill_dir)?;
            ui::success(&format!("Deleted .claude/skills/{}", name));
        }
    }

    // Regenerate context files
    super::init::generate_context(&cfg, &workspace_root)?;

    ui::header("Done");
    ui::info(&format!("Repo '{}' removed from workspace", name));

    Ok(())
}
