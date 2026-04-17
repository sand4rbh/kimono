use anyhow::{bail, Context, Result};
use std::fs;

use crate::config;
use crate::git;
use crate::ui;

pub fn run(repo: &str, branch: &str, new: bool) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;

    let repo_config = cfg.repos.get(repo).ok_or_else(|| {
        anyhow::anyhow!(
            "repo '{}' not found in config. Available repos: {}",
            repo,
            cfg.repos.keys().cloned().collect::<Vec<_>>().join(", ")
        )
    })?;

    let slug = git::branch_slug(branch);
    let wt_dir = root.join(&cfg.workspace.worktree_dir);
    let wt_path = wt_dir.join(format!("{}--{}", repo, slug));
    let repo_path = root.join(&cfg.workspace.apps_dir).join(repo);

    if !repo_path.exists() {
        bail!(
            "repo '{}' is not cloned yet. Run `kimono clone {}` first.",
            repo,
            repo
        );
    }

    if wt_path.exists() {
        bail!("worktree already exists at {}", wt_path.display());
    }

    // Ensure the worktree directory exists
    fs::create_dir_all(&wt_dir)
        .with_context(|| format!("failed to create worktree directory {}", wt_dir.display()))?;

    let sp = ui::spinner(&format!(
        "Creating worktree for {} on branch {}...",
        repo, branch
    ));

    if new {
        // Fetch first, then create a new branch from origin/<default_branch>
        git::fetch(&repo_path).with_context(|| format!("failed to fetch origin for {}", repo))?;
        let base = format!("origin/{}", repo_config.branch);
        git::worktree_add(&repo_path, &wt_path, branch, Some(&base)).with_context(|| {
            format!(
                "failed to add worktree for {} on new branch {} from {}",
                repo, branch, base
            )
        })?;
    } else {
        git::worktree_add(&repo_path, &wt_path, branch, None).with_context(|| {
            format!(
                "failed to add worktree for {} on existing branch {}",
                repo, branch
            )
        })?;
    }

    sp.finish_and_clear();
    ui::success(&format!(
        "{}: worktree created at {}",
        repo,
        wt_path.display()
    ));

    Ok(())
}
