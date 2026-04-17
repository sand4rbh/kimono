use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};
use dialoguer::{Confirm, Input};

use crate::config::schema::{KimonoConfig, Repo, Workspace};
use crate::ui;

/// Run the `init` command.
///
/// Two modes:
/// - `--from <path>`: migrate from an ofmono `.repos.conf` file.
/// - Interactive (default): guided wizard to build config from scratch.
///
/// When `bare` is true, only the config is written (no cloning or context
/// generation).
pub fn run(from: Option<&Path>, bare: bool) -> Result<()> {
    let workspace_root =
        std::env::current_dir().context("failed to determine current directory")?;

    let config = if let Some(conf_dir) = from {
        build_config_from_repos_conf(conf_dir, &workspace_root)?
    } else {
        build_config_interactive()?
    };

    // Write .kimono/config.yml
    let kimono_dir = workspace_root.join(".kimono");
    fs::create_dir_all(&kimono_dir).context("failed to create .kimono directory")?;

    let yaml = serde_yaml::to_string(&config).context("failed to serialize config to YAML")?;
    let config_path = kimono_dir.join("config.yml");
    fs::write(&config_path, &yaml)
        .with_context(|| format!("failed to write {}", config_path.display()))?;
    ui::success("Created .kimono/config.yml");

    // Write .gitignore using the Tera template
    let tera = crate::context::create_tera()?;
    let mut ctx = tera::Context::new();
    ctx.insert("workspace", &config.workspace);
    let gitignore_content = tera
        .render("gitignore", &ctx)
        .context("failed to render .gitignore template")?;
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

        // Generate context files
        generate_context(&config, &workspace_root)?;

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

    Ok(())
}

/// Parse a `.repos.conf` file from an ofmono workspace and build a KimonoConfig.
///
/// Format: `name:remote:branch` (colon-separated), lines starting with `#` are
/// comments, empty lines are skipped.
fn build_config_from_repos_conf(conf_dir: &Path, workspace_root: &Path) -> Result<KimonoConfig> {
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

    // Derive workspace name from the directory name or ask
    let default_name = workspace_root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("my-workspace")
        .to_string();

    let name: String = Input::new()
        .with_prompt("Workspace name")
        .default(default_name)
        .interact_text()
        .context("failed to read workspace name")?;

    Ok(KimonoConfig {
        workspace: Workspace {
            name,
            apps_dir: "apps".to_string(),
            worktree_dir: ".worktrees".to_string(),
        },
        repos,
        claude: None,
    })
}

/// Interactive wizard to build a KimonoConfig from user prompts.
fn build_config_interactive() -> Result<KimonoConfig> {
    ui::header("Kimono workspace setup");

    let name: String = Input::new()
        .with_prompt("Workspace name")
        .interact_text()
        .context("failed to read workspace name")?;

    let apps_dir: String = Input::new()
        .with_prompt("Apps directory")
        .default("apps".into())
        .interact_text()
        .context("failed to read apps directory")?;

    let worktree_dir: String = Input::new()
        .with_prompt("Worktree directory")
        .default(".worktrees".into())
        .interact_text()
        .context("failed to read worktree directory")?;

    let mut repos = HashMap::new();

    loop {
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
        claude: None,
    })
}

/// Generate all context files for the workspace.
///
/// This is a shared helper also used by add and remove commands.
pub fn generate_context(config: &KimonoConfig, workspace_root: &Path) -> Result<()> {
    let tera = crate::context::create_tera()?;
    let claude_dir = workspace_root.join(".claude");

    ui::header("Generating context files");

    // 1. CLAUDE.md
    let claude_md = crate::context::claude_md::generate(config, &tera)
        .context("failed to generate CLAUDE.md")?;
    let claude_md_path = workspace_root.join("CLAUDE.md");
    if claude_md_path.is_file() {
        let existing = fs::read_to_string(&claude_md_path)
            .with_context(|| format!("failed to read {}", claude_md_path.display()))?;
        let merged = crate::context::preserve::merge(&claude_md, &existing);
        fs::write(&claude_md_path, &merged)?;
        ui::success("Updated CLAUDE.md");
    } else {
        fs::write(&claude_md_path, &claude_md)?;
        ui::success("Created CLAUDE.md");
    }

    // 2. Agent files
    let agents = crate::context::agents::generate_all(config, &tera)
        .context("failed to generate agent files")?;
    for (name, content) in &agents {
        let dir = claude_dir.join("agents").join(name);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("AGENT.md"), content)?;
        ui::success(&format!("Created .claude/agents/{}/AGENT.md", name));
    }

    // 3. Skill files — repo skills
    let repo_skills = crate::context::skills::generate_repo_skills(config, &tera)
        .context("failed to generate repo skills")?;
    for (name, content) in &repo_skills {
        let dir = claude_dir.join("skills").join(name);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("SKILL.md"), content)?;
        ui::success(&format!("Created .claude/skills/{}/SKILL.md", name));
    }

    // 3b. Skill files — workflow skills
    let workflow_skills = crate::context::skills::generate_workflow_skills(config, &tera)
        .context("failed to generate workflow skills")?;
    for (name, content) in &workflow_skills {
        let dir = claude_dir.join("skills").join(name);
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("SKILL.md"), content)?;
        ui::success(&format!("Created .claude/skills/{}/SKILL.md", name));
    }

    // 4. settings.json
    let settings =
        crate::context::settings::generate(config).context("failed to generate settings.json")?;
    fs::create_dir_all(&claude_dir)?;
    let settings_path = claude_dir.join("settings.json");
    fs::write(&settings_path, &settings)?;
    ui::success("Created .claude/settings.json");

    // 5. Hookify rules
    let rules = crate::context::hookify::generate_all(config, &tera)
        .context("failed to generate hookify rules")?;
    let hookify_dir = claude_dir.join("hookify");
    fs::create_dir_all(&hookify_dir)?;
    for (filename, content) in &rules {
        fs::write(hookify_dir.join(filename), content)?;
        ui::success(&format!("Created .claude/hookify/{}", filename));
    }

    Ok(())
}
