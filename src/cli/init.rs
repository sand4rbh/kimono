use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};
use dialoguer::{Confirm, Input};

use crate::config::schema::{KimonoConfig, Repo, Workspace};
use crate::ui;

/// Run the `init` command.
///
/// Creates a new directory named after the workspace (like `git clone`) and
/// initializes the kimono workspace inside it.
///
/// Two modes:
/// - `--from <path>`: migrate from an ofmono `.repos.conf` file.
/// - Interactive (default): guided wizard to build config from scratch.
///
/// When `bare` is true, only the config is written (no cloning).
pub fn run(name_arg: Option<&str>, from: Option<&Path>, bare: bool) -> Result<()> {
    let parent_dir =
        std::env::current_dir().context("failed to determine current directory")?;

    let config = if let Some(conf_dir) = from {
        build_config_from_repos_conf(conf_dir, name_arg)?
    } else {
        build_config_interactive(name_arg, bare)?
    };

    // Create the workspace directory (like `git clone <url>` creates `<name>/`)
    let workspace_root = parent_dir.join(&config.workspace.name);
    if workspace_root.exists() {
        let is_empty = fs::read_dir(&workspace_root)
            .with_context(|| format!("failed to read {}", workspace_root.display()))?
            .next()
            .is_none();
        if !is_empty {
            bail!(
                "directory '{}' already exists and is not empty",
                config.workspace.name
            );
        }
    } else {
        fs::create_dir(&workspace_root).with_context(|| {
            format!("failed to create workspace directory '{}'", config.workspace.name)
        })?;
    }
    ui::success(&format!("Created workspace directory: {}", config.workspace.name));

    // chdir into the workspace so later operations resolve paths relative to it.
    std::env::set_current_dir(&workspace_root)
        .with_context(|| format!("failed to cd into {}", workspace_root.display()))?;

    // Write .kimono/config.yml
    let kimono_dir = workspace_root.join(".kimono");
    fs::create_dir_all(&kimono_dir).context("failed to create .kimono directory")?;

    let yaml = serde_yaml::to_string(&config).context("failed to serialize config to YAML")?;
    let config_path = kimono_dir.join("config.yml");
    fs::write(&config_path, &yaml)
        .with_context(|| format!("failed to write {}", config_path.display()))?;
    ui::success("Created .kimono/config.yml");

    // Write .gitignore — a minimal hardcoded version. The full template-driven
    // version is being rewritten as part of the v2 init wizard rework (Task 9).
    let gitignore_content = format!(
        "# Kimono workspace\n{}\n{}\n",
        config.workspace.apps_dir,
        config.workspace.worktree_dir,
    );
    let gitignore_path = workspace_root.join(".gitignore");
    fs::write(&gitignore_path, &gitignore_content)
        .with_context(|| format!("failed to write {}", gitignore_path.display()))?;
    ui::success("Created .gitignore");

    if !bare {
        // Clone all repos
        let apps_dir = workspace_root.join(&config.workspace.apps_dir);
        fs::create_dir_all(&apps_dir).context("failed to create apps directory")?;

        if !config.repos.is_empty() {
            ui::header("Cloning repositories");

            for (name, repo) in &config.repos {
                let repo_path = apps_dir.join(name);
                if repo_path.exists() {
                    ui::skip(&format!("{} already exists", name));
                    continue;
                }

                let sp = ui::spinner(&format!("Cloning {}...", name));
                match crate::git::clone(&repo.remote, &repo_path, &repo.branch) {
                    Ok(()) => {
                        sp.finish_and_clear();
                        ui::success(&format!("{} cloned", name));
                    }
                    Err(e) => {
                        sp.finish_and_clear();
                        ui::error(&format!("{}: {}", name, e));
                    }
                }
            }
        }

        // Initialize git repo (ignore errors if already a git repo)
        let _ = Command::new("git")
            .arg("init")
            .current_dir(&workspace_root)
            .output();
        ui::success("Initialized git repository");
    }

    // Print summary
    ui::header("Workspace initialized");
    ui::info(&format!("  Name:       {}", config.workspace.name));
    ui::info(&format!("  Apps dir:   {}", config.workspace.apps_dir));
    ui::info(&format!("  Repos:      {}", config.repos.len()));
    if bare {
        ui::info("  Mode:       bare (config only, no clones)");
    }
    ui::info("");
    ui::info(&format!("  Next: cd {}", config.workspace.name));

    Ok(())
}

/// Parse a `.repos.conf` file from an ofmono workspace and build a KimonoConfig.
///
/// Format: `name:remote:branch` (colon-separated), lines starting with `#` are
/// comments, empty lines are skipped.
fn build_config_from_repos_conf(conf_dir: &Path, name_arg: Option<&str>) -> Result<KimonoConfig> {
    let repos_conf_path = conf_dir.join(".repos.conf");
    let contents = fs::read_to_string(&repos_conf_path)
        .with_context(|| format!("failed to read {}", repos_conf_path.display()))?;

    let mut repos = HashMap::new();
    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = trimmed.splitn(3, ':').collect();
        if parts.len() < 2 {
            ui::warn(&format!("skipping malformed line: {}", trimmed));
            continue;
        }

        let name = parts[0].to_string();
        let remote = parts[1].to_string();
        let branch = if parts.len() == 3 && !parts[2].is_empty() {
            parts[2].to_string()
        } else {
            "main".to_string()
        };

        repos.insert(
            name,
            Repo {
                remote,
                branch,
                description: None,
                tech: vec![],
                package_manager: None,
                depends_on: vec![],
                commands: HashMap::new(),
            },
        );
    }

    // Use the provided name, or derive from the source directory name, or prompt
    let name = match name_arg {
        Some(n) => n.to_string(),
        None => {
            let default_name = conf_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("my-workspace")
                .to_string();
            Input::new()
                .with_prompt("Workspace name")
                .default(default_name)
                .interact_text()
                .context("failed to read workspace name")?
        }
    };

    Ok(KimonoConfig {
        workspace: Workspace {
            name,
            apps_dir: "apps".to_string(),
            worktree_dir: ".worktrees".to_string(),
        },
        repos,
        plugins: None,
    })
}

/// Interactive wizard to build a KimonoConfig from user prompts.
///
/// When `name_arg` is provided via CLI, skips the workspace-level prompts
/// (apps_dir, worktree_dir) and uses defaults. When `bare` is true, skips
/// the repo prompts entirely (config only, no repos).
fn build_config_interactive(name_arg: Option<&str>, bare: bool) -> Result<KimonoConfig> {
    ui::header("Kimono workspace setup");

    let name: String = match name_arg {
        Some(n) => n.to_string(),
        None => Input::new()
            .with_prompt("Workspace name (also the directory name)")
            .interact_text()
            .context("failed to read workspace name")?,
    };

    // If name was provided via CLI, use defaults for dir settings.
    let (apps_dir, worktree_dir) = if name_arg.is_some() {
        ("apps".to_string(), ".worktrees".to_string())
    } else {
        let apps: String = Input::new()
            .with_prompt("Apps directory")
            .default("apps".into())
            .interact_text()
            .context("failed to read apps directory")?;
        let wt: String = Input::new()
            .with_prompt("Worktree directory")
            .default(".worktrees".into())
            .interact_text()
            .context("failed to read worktree directory")?;
        (apps, wt)
    };

    let mut repos = HashMap::new();

    loop {
        if bare {
            break;
        }
        let add_repo = Confirm::new()
            .with_prompt("Add a repo?")
            .default(repos.is_empty()) // default yes for first repo
            .interact()
            .context("failed to read confirmation")?;

        if !add_repo {
            break;
        }

        let repo_name: String = Input::new()
            .with_prompt("Repo short name")
            .interact_text()
            .context("failed to read repo name")?;

        let remote: String = Input::new()
            .with_prompt("Git remote URL")
            .interact_text()
            .context("failed to read remote URL")?;

        let branch: String = Input::new()
            .with_prompt("Default branch")
            .default("main".into())
            .interact_text()
            .context("failed to read branch")?;

        let description: String = Input::new()
            .with_prompt("Description (optional)")
            .default("".into())
            .interact_text()
            .context("failed to read description")?;

        let desc = if description.is_empty() {
            None
        } else {
            Some(description)
        };

        repos.insert(
            repo_name,
            Repo {
                remote,
                branch,
                description: desc,
                tech: vec![],
                package_manager: None,
                depends_on: vec![],
                commands: HashMap::new(),
            },
        );
    }

    Ok(KimonoConfig {
        workspace: Workspace {
            name,
            apps_dir,
            worktree_dir,
        },
        repos,
        plugins: None,
    })
}

