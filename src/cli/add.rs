use std::collections::HashMap;
use std::fs;

use anyhow::{bail, Context, Result};

use crate::config;
use crate::config::schema::Repo;
use crate::git;
use crate::ui;

/// Add a new repository to the workspace.
///
/// - Validates the repo name is not already in use.
/// - Updates `.kimono/config.yml` with the new repo entry.
/// - Clones the repo into the apps directory.
/// - Regenerates context files.
pub fn run(name: &str, remote: &str, branch: Option<&str>) -> Result<()> {
    let config_path = config::find_config()?;
    let workspace_root = config_path
        .parent()
        .and_then(|p| p.parent())
        .ok_or_else(|| anyhow::anyhow!("unexpected config path structure"))?
        .to_path_buf();

    let mut cfg = config::load(&config_path)?;

    // Check that the repo name doesn't already exist
    if cfg.repos.contains_key(name) {
        bail!("repo '{}' already exists in the workspace config", name);
    }

    let branch = branch.unwrap_or("main");

    // Create the new Repo entry
    let repo = Repo {
        remote: remote.to_string(),
        branch: branch.to_string(),
        description: None,
        tech: vec![],
        package_manager: None,
        depends_on: vec![],
        commands: HashMap::new(),
    };

    // Add to config
    cfg.repos.insert(name.to_string(), repo.clone());

    // Validate updated config
    config::validate(&cfg)?;

    // Write updated config back to disk
    let yaml = serde_yaml::to_string(&cfg).context("failed to serialize config to YAML")?;
    fs::write(&config_path, &yaml)
        .with_context(|| format!("failed to write {}", config_path.display()))?;
    ui::success(&format!("Added '{}' to .kimono/config.yml", name));

    // Clone the repo
    let apps_dir = workspace_root.join(&cfg.workspace.apps_dir);
    fs::create_dir_all(&apps_dir).context("failed to create apps directory")?;

    let repo_path = apps_dir.join(name);
    if repo_path.exists() {
        ui::warn(&format!(
            "{} directory already exists, skipping clone",
            name
        ));
    } else {
        let sp = ui::spinner(&format!("Cloning {}...", name));
        match git::clone(remote, &repo_path, branch) {
            Ok(()) => {
                sp.finish_and_clear();
                ui::success(&format!("{} cloned", name));
            }
            Err(e) => {
                sp.finish_and_clear();
                ui::error(&format!("Failed to clone {}: {}", name, e));
            }
        }
    }

    // Regenerate context files
    super::init::generate_context(&cfg, &workspace_root)?;

    ui::header("Done");
    ui::info(&format!("Repo '{}' added to workspace", name));

    Ok(())
}
