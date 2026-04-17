use anyhow::{bail, Context, Result};

use crate::config;
use crate::git;
use crate::ui;

pub fn run(repo: &str, branch: &str) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;

    if !cfg.repos.contains_key(repo) {
        bail!(
            "repo '{}' not found in config. Available repos: {}",
            repo,
            cfg.repos.keys().cloned().collect::<Vec<_>>().join(", ")
        );
    }

    let slug = git::branch_slug(branch);
    let wt_dir = root.join(&cfg.workspace.worktree_dir);
    let wt_path = wt_dir.join(format!("{}--{}", repo, slug));
    let repo_path = root.join(&cfg.workspace.apps_dir).join(repo);

    if !wt_path.exists() {
        bail!("worktree does not exist at {}", wt_path.display());
    }

    if !repo_path.exists() {
        bail!("repo '{}' is not cloned yet. Cannot remove worktree.", repo);
    }

    let sp = ui::spinner(&format!(
        "Removing worktree for {} on branch {}...",
        repo, branch
    ));

    git::worktree_remove(&repo_path, &wt_path, false)
        .with_context(|| format!("failed to remove worktree at {}", wt_path.display()))?;

    sp.finish_and_clear();
    ui::success(&format!("{}: worktree removed (branch {})", repo, branch));
    ui::info(&format!(
        "  Note: branch '{}' still exists. Use `git branch -d {}` in the repo to delete it.",
        branch, branch
    ));

    Ok(())
}
