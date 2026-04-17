// Git operations — wraps the git CLI via std::process::Command.
// We intentionally shell out to `git` rather than using libgit2 so that
// behavior matches what users see on the command line.

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: PathBuf,
    pub branch: String,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

struct CommandOutput {
    stdout: String,
}

/// Run a git command inside `repo_path` via `git -C <path> <args…>`.
fn run_git(repo_path: &Path, args: &[&str]) -> Result<CommandOutput> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(args)
        .output()
        .context("Failed to execute git")?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!("git {} failed: {}", args.join(" "), stderr);
    }

    Ok(CommandOutput { stdout })
}

/// Run a git command without a repo path (e.g. `git clone`).
fn run_git_raw(args: &[&str]) -> Result<CommandOutput> {
    let output = Command::new("git")
        .args(args)
        .output()
        .context("Failed to execute git")?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!("git {} failed: {}", args.join(" "), stderr);
    }

    Ok(CommandOutput { stdout })
}

// ---------------------------------------------------------------------------
// Availability check
// ---------------------------------------------------------------------------

/// Verify that `git` is available on PATH.
pub fn git_available() -> Result<()> {
    which::which("git").context("git is not installed or not found in PATH")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Clone / Fetch / Rebase
// ---------------------------------------------------------------------------

/// Clone a remote repository into `path`, checking out `branch`.
pub fn clone(remote: &str, path: &Path, branch: &str) -> Result<()> {
    let path_str = path.to_str().context("Path contains invalid UTF-8")?;
    run_git_raw(&["clone", "--branch", branch, remote, path_str])
        .with_context(|| format!("Failed to clone {} into {}", remote, path_str))?;
    Ok(())
}

/// Fetch from origin.
pub fn fetch(repo_path: &Path) -> Result<()> {
    run_git(repo_path, &["fetch", "origin"]).context("Failed to fetch from origin")?;
    Ok(())
}

/// Rebase the current branch onto `onto` (e.g. `origin/main`).
pub fn rebase(repo_path: &Path, onto: &str) -> Result<()> {
    run_git(repo_path, &["rebase", onto])
        .with_context(|| format!("Failed to rebase onto {}", onto))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Status queries
// ---------------------------------------------------------------------------

/// Return the name of the currently checked-out branch.
pub fn current_branch(repo_path: &Path) -> Result<String> {
    let out = run_git(repo_path, &["branch", "--show-current"])
        .context("Failed to determine current branch")?;
    Ok(out.stdout)
}

/// Return `true` if the working tree has no uncommitted changes.
pub fn is_clean(repo_path: &Path) -> Result<bool> {
    let out = run_git(repo_path, &["status", "--porcelain"])
        .context("Failed to check working tree status")?;
    Ok(out.stdout.is_empty())
}

/// Return `(ahead, behind)` relative to the upstream tracking branch.
/// Returns `(0, 0)` when there is no upstream configured.
pub fn ahead_behind(repo_path: &Path) -> Result<(u32, u32)> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["rev-list", "--left-right", "--count", "@{u}...HEAD"])
        .output()
        .context("Failed to execute git rev-list")?;

    if !output.status.success() {
        // No upstream tracking branch — treat as (0, 0).
        return Ok((0, 0));
    }

    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let parts: Vec<&str> = text.split('\t').collect();
    if parts.len() != 2 {
        return Ok((0, 0));
    }

    let behind: u32 = parts[0].parse().unwrap_or(0);
    let ahead: u32 = parts[1].parse().unwrap_or(0);
    Ok((ahead, behind))
}

/// Return the number of stash entries.
pub fn stash_count(repo_path: &Path) -> Result<u32> {
    let out = run_git(repo_path, &["stash", "list"]).context("Failed to list stashes")?;
    if out.stdout.is_empty() {
        Ok(0)
    } else {
        Ok(out.stdout.lines().count() as u32)
    }
}

// ---------------------------------------------------------------------------
// Worktree operations
// ---------------------------------------------------------------------------

/// Add a worktree at `wt_path`.
///
/// If `new_from` is `Some`, create a new branch from that ref:
///   `git worktree add <wt_path> -b <branch> <new_from>`
///
/// If `new_from` is `None`, attach to an existing branch:
///   `git worktree add <wt_path> <branch>`
pub fn worktree_add(
    repo_path: &Path,
    wt_path: &Path,
    branch: &str,
    new_from: Option<&str>,
) -> Result<()> {
    let wt_str = wt_path
        .to_str()
        .context("Worktree path contains invalid UTF-8")?;

    match new_from {
        Some(base) => {
            run_git(repo_path, &["worktree", "add", wt_str, "-b", branch, base]).with_context(
                || format!("Failed to add worktree {} on new branch {}", wt_str, branch),
            )?;
        }
        None => {
            run_git(repo_path, &["worktree", "add", wt_str, branch]).with_context(|| {
                format!("Failed to add worktree {} on branch {}", wt_str, branch)
            })?;
        }
    }
    Ok(())
}

/// Remove a worktree.
pub fn worktree_remove(repo_path: &Path, wt_path: &Path, force: bool) -> Result<()> {
    let wt_str = wt_path
        .to_str()
        .context("Worktree path contains invalid UTF-8")?;

    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(wt_str);

    run_git(repo_path, &args).with_context(|| format!("Failed to remove worktree {}", wt_str))?;
    Ok(())
}

/// List all worktrees by parsing `git worktree list --porcelain`.
pub fn worktree_list(repo_path: &Path) -> Result<Vec<WorktreeInfo>> {
    let out = run_git(repo_path, &["worktree", "list", "--porcelain"])
        .context("Failed to list worktrees")?;
    Ok(parse_worktree_list(&out.stdout))
}

/// Parse the porcelain output of `git worktree list --porcelain` into a
/// vector of `WorktreeInfo`.
///
/// Each worktree block looks like:
/// ```text
/// worktree /path/to/repo
/// HEAD abc123def456
/// branch refs/heads/main
/// ```
/// Blocks are separated by blank lines. The `HEAD` line is ignored — we only
/// extract `worktree` (path) and `branch`.
fn parse_worktree_list(output: &str) -> Vec<WorktreeInfo> {
    let mut results = Vec::new();
    let mut path: Option<PathBuf> = None;
    let mut branch: Option<String> = None;

    for line in output.lines() {
        if line.is_empty() {
            // End of a block — flush if we have both fields.
            if let (Some(p), Some(b)) = (path.take(), branch.take()) {
                results.push(WorktreeInfo { path: p, branch: b });
            }
            // Reset for next block even if we didn't have all fields
            // (e.g. a bare worktree or detached HEAD).
            path = None;
            branch = None;
            continue;
        }

        if let Some(rest) = line.strip_prefix("worktree ") {
            path = Some(PathBuf::from(rest));
        } else if let Some(rest) = line.strip_prefix("branch ") {
            // Strip `refs/heads/` prefix to give a short branch name.
            branch = Some(rest.strip_prefix("refs/heads/").unwrap_or(rest).to_string());
        }
    }

    // Flush the last block (porcelain output may not end with a blank line).
    if let (Some(p), Some(b)) = (path, branch) {
        results.push(WorktreeInfo { path: p, branch: b });
    }

    results
}

// ---------------------------------------------------------------------------
// Branch operations
// ---------------------------------------------------------------------------

/// Return `true` if a local branch with the given name exists.
pub fn branch_exists_local(repo_path: &Path, branch: &str) -> Result<bool> {
    let refspec = format!("refs/heads/{}", branch);
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["show-ref", "--verify", "--quiet", &refspec])
        .output()
        .context("Failed to execute git show-ref")?;
    Ok(output.status.success())
}

/// Return `true` if a remote branch with the given name exists on origin.
pub fn branch_exists_remote(repo_path: &Path, branch: &str) -> Result<bool> {
    let out = run_git(repo_path, &["ls-remote", "--heads", "origin", branch])
        .context("Failed to query remote branches")?;
    Ok(!out.stdout.is_empty())
}

/// Create a new branch, optionally from a specific ref.
pub fn create_branch(repo_path: &Path, name: &str, from: Option<&str>) -> Result<()> {
    let mut args = vec!["checkout", "-b", name];
    if let Some(base) = from {
        args.push(base);
    }
    run_git(repo_path, &args).with_context(|| format!("Failed to create branch {}", name))?;
    Ok(())
}

/// Check out an existing branch.
pub fn checkout(repo_path: &Path, branch: &str) -> Result<()> {
    run_git(repo_path, &["checkout", branch])
        .with_context(|| format!("Failed to checkout branch {}", branch))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Push
// ---------------------------------------------------------------------------

/// Push a branch to origin with upstream tracking.
pub fn push(repo_path: &Path, branch: &str) -> Result<()> {
    run_git(repo_path, &["push", "-u", "origin", branch])
        .with_context(|| format!("Failed to push branch {}", branch))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Utility
// ---------------------------------------------------------------------------

/// Convert a branch name into a worktree-safe slug by replacing `/` with `-`.
pub fn branch_slug(branch: &str) -> String {
    branch.replace('/', "-")
}

/// Create a commit with the given message.
pub fn commit(repo_path: &Path, message: &str) -> Result<()> {
    run_git(repo_path, &["commit", "-m", message]).context("Failed to commit")?;
    Ok(())
}

/// Return `true` if there are staged changes ready to commit.
///
/// `git diff --cached --quiet` exits with code 1 when staged changes exist.
pub fn has_staged_changes(repo_path: &Path) -> Result<bool> {
    let output = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["diff", "--cached", "--quiet"])
        .output()
        .context("Failed to execute git diff --cached")?;

    // Exit code 0 → no staged changes; exit code 1 → has staged changes.
    Ok(!output.status.success())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_branch_slug() {
        assert_eq!(branch_slug("feature/payments"), "feature-payments");
        assert_eq!(branch_slug("fix/auth-bug"), "fix-auth-bug");
        assert_eq!(branch_slug("simple"), "simple");
        assert_eq!(branch_slug("a/b/c"), "a-b-c");
    }

    #[test]
    fn test_git_available() {
        // Just verify this doesn't panic — git should be installed in CI
        // and on developer machines.
        let result = git_available();
        assert!(result.is_ok(), "git should be available: {:?}", result);
    }

    #[test]
    fn test_worktree_info_parse() {
        let porcelain = "\
worktree /path/to/main
HEAD abc123def456789012345678901234567890abcd
branch refs/heads/main

worktree /path/to/feature
HEAD def456abc789012345678901234567890abcd1234
branch refs/heads/feature-x
";
        let worktrees = parse_worktree_list(porcelain);
        assert_eq!(worktrees.len(), 2);

        assert_eq!(worktrees[0].path, PathBuf::from("/path/to/main"));
        assert_eq!(worktrees[0].branch, "main");

        assert_eq!(worktrees[1].path, PathBuf::from("/path/to/feature"));
        assert_eq!(worktrees[1].branch, "feature-x");
    }

    #[test]
    fn test_worktree_info_parse_no_trailing_newline() {
        // Porcelain output may not end with a blank line.
        let porcelain = "\
worktree /only/one
HEAD 0000000000000000000000000000000000000000
branch refs/heads/solo";
        let worktrees = parse_worktree_list(porcelain);
        assert_eq!(worktrees.len(), 1);
        assert_eq!(worktrees[0].branch, "solo");
    }

    #[test]
    fn test_worktree_info_parse_empty() {
        let worktrees = parse_worktree_list("");
        assert!(worktrees.is_empty());
    }
}
