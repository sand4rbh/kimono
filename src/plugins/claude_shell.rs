use std::process::Stdio;

use anyhow::{Context, Result};

/// Returns true if `claude` is on PATH.
pub fn is_available() -> bool {
    which::which("claude").is_ok()
}

/// Shell out to `claude -p "Run the kimono <name> skill for this workspace"`
/// non-interactively. Stdout/stderr stream to the parent terminal; stdin is
/// closed so the child cannot block waiting on it.
///
/// Pre-allows the file-system and shell tools the bootstrap/discover skills
/// need so the run doesn't gate on an invisible permission prompt.
///
/// Returns an error if `claude` is not available, or if the invocation
/// exits non-zero.
pub fn run_skill(skill_name: &str) -> Result<()> {
    if !is_available() {
        anyhow::bail!(
            "`claude` CLI not found on PATH. Install Claude Code from \
             https://claude.com/claude-code and re-run."
        );
    }

    let prompt = format!(
        "Run the kimono {skill_name} skill for this workspace, applied to the current directory."
    );

    eprintln!(
        "  Spawning `claude -p` to run the {skill_name} skill — this may take a minute as Claude reads your codebase and writes the files."
    );

    let status = std::process::Command::new("claude")
        .arg("-p")
        .arg(&prompt)
        .arg("--allowedTools")
        .arg("Read Write Edit Bash Glob Grep")
        .stdin(Stdio::null())
        .status()
        .with_context(|| format!("failed to spawn `claude -p` for skill {skill_name}"))?;

    if !status.success() {
        anyhow::bail!(
            "`claude -p` for skill '{skill_name}' exited with status {}",
            status
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_available_does_not_panic() {
        let _ = is_available();
    }
}
