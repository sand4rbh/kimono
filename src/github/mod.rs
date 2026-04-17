// GitHub CLI operations — wraps the `gh` CLI via std::process::Command.
// Pattern mirrors src/git/mod.rs: shell out rather than using a library so
// behaviour matches what users see on the command line.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PrInfo {
    pub number: u32,
    pub url: String,
    pub state: String,
    pub title: String,
    pub head_branch: String,
}

#[derive(Debug, Clone)]
pub struct ReviewComment {
    pub author: String,
    pub path: String,
    pub line: Option<u32>,
    pub body: String,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Run a `gh` command inside `repo_path` and return its stdout on success.
fn run_gh(repo_path: &Path, args: &[&str]) -> Result<String> {
    let output = Command::new("gh")
        .current_dir(repo_path)
        .args(args)
        .output()
        .context("Failed to execute gh — is GitHub CLI installed?")?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if !output.status.success() {
        bail!("gh {} failed: {}", args.join(" "), stderr);
    }

    Ok(stdout)
}

// ---------------------------------------------------------------------------
// Availability check
// ---------------------------------------------------------------------------

/// Verify that `gh` is available on PATH and the user is authenticated.
pub fn gh_available() -> Result<()> {
    which::which("gh").context(
        "GitHub CLI (gh) is not installed or not found in PATH. \
         Install it from https://cli.github.com/",
    )?;

    let output = Command::new("gh")
        .args(["auth", "status"])
        .output()
        .context("Failed to execute gh auth status")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!(
            "GitHub CLI is not authenticated. Run `gh auth login` first.\n{}",
            stderr
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// PR operations
// ---------------------------------------------------------------------------

/// Create a pull request and return the PR URL.
pub fn pr_create(
    repo_path: &Path,
    base: &str,
    title: &str,
    body: &str,
    draft: bool,
) -> Result<String> {
    let mut args = vec![
        "pr", "create", "--base", base, "--title", title, "--body", body,
    ];
    if draft {
        args.push("--draft");
    }

    let url = run_gh(repo_path, &args).context("Failed to create pull request")?;
    Ok(url)
}

/// Get PR info for the current branch. Returns None if no PR exists.
pub fn pr_view(repo_path: &Path) -> Result<Option<PrInfo>> {
    let output = Command::new("gh")
        .current_dir(repo_path)
        .args(["pr", "view", "--json", "number,url,state,title,headRefName"])
        .output()
        .context("Failed to execute gh pr view")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        // gh returns a non-zero exit when no PR is found for the current branch
        if stderr.contains("no pull requests found")
            || stderr.contains("Could not resolve")
            || stderr.contains("no open pull requests")
        {
            return Ok(None);
        }
        bail!("gh pr view failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Ok(None);
    }

    let v: serde_json::Value =
        serde_json::from_str(&stdout).context("Failed to parse gh pr view JSON")?;

    let number = v["number"].as_u64().unwrap_or(0) as u32;
    let url = v["url"].as_str().unwrap_or("").to_string();
    let state = v["state"].as_str().unwrap_or("").to_string();
    let title = v["title"].as_str().unwrap_or("").to_string();
    let head_branch = v["headRefName"].as_str().unwrap_or("").to_string();

    if number == 0 {
        return Ok(None);
    }

    Ok(Some(PrInfo {
        number,
        url,
        state,
        title,
        head_branch,
    }))
}

/// Get review comments for a PR by number.
pub fn pr_comments(repo_path: &Path, pr_number: u32) -> Result<Vec<ReviewComment>> {
    let number_str = pr_number.to_string();

    // Use gh pr view with --json to get review comments
    let output = Command::new("gh")
        .current_dir(repo_path)
        .args(["pr", "view", &number_str, "--json", "reviews,comments"])
        .output()
        .context("Failed to execute gh pr view for comments")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        bail!("gh pr view --json reviews,comments failed: {}", stderr);
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Ok(Vec::new());
    }

    let v: serde_json::Value =
        serde_json::from_str(&stdout).context("Failed to parse PR comments JSON")?;

    let mut comments = Vec::new();

    // Parse review comments from "reviews" array
    if let Some(reviews) = v["reviews"].as_array() {
        for review in reviews {
            let author = review["author"]["login"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let body = review["body"].as_str().unwrap_or("").to_string();

            if !body.is_empty() {
                comments.push(ReviewComment {
                    author,
                    path: String::new(),
                    line: None,
                    body,
                });
            }
        }
    }

    // Parse issue-level comments from "comments" array
    if let Some(issue_comments) = v["comments"].as_array() {
        for comment in issue_comments {
            let author = comment["author"]["login"]
                .as_str()
                .unwrap_or("unknown")
                .to_string();
            let body = comment["body"].as_str().unwrap_or("").to_string();

            if !body.is_empty() {
                comments.push(ReviewComment {
                    author,
                    path: String::new(),
                    line: None,
                    body,
                });
            }
        }
    }

    // Also fetch inline review comments via the API
    let api_output = Command::new("gh")
        .current_dir(repo_path)
        .args([
            "api",
            &format!("repos/{{owner}}/{{repo}}/pulls/{}/comments", pr_number),
        ])
        .output();

    if let Ok(api_out) = api_output {
        if api_out.status.success() {
            let api_stdout = String::from_utf8_lossy(&api_out.stdout).trim().to_string();
            if let Ok(arr) = serde_json::from_str::<serde_json::Value>(&api_stdout) {
                if let Some(items) = arr.as_array() {
                    for item in items {
                        let author = item["user"]["login"]
                            .as_str()
                            .unwrap_or("unknown")
                            .to_string();
                        let path = item["path"].as_str().unwrap_or("").to_string();
                        let line = item["line"].as_u64().map(|n| n as u32);
                        let body = item["body"].as_str().unwrap_or("").to_string();

                        if !body.is_empty() {
                            comments.push(ReviewComment {
                                author,
                                path,
                                line,
                                body,
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(comments)
}

/// Update a PR's description body.
pub fn pr_update_body(repo_path: &Path, pr_number: u32, body: &str) -> Result<()> {
    let number_str = pr_number.to_string();
    run_gh(repo_path, &["pr", "edit", &number_str, "--body", body])
        .context("Failed to update PR body")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gh_available_returns_result() {
        // Just verify it doesn't panic. gh may or may not be installed in
        // the test environment.
        let _ = gh_available();
    }
}
