use anyhow::{Context, Result};

/// Returns true if `claude` is on PATH.
pub fn is_available() -> bool {
    which::which("claude").is_ok()
}

/// Shell out to `claude -p "Run the kimono <name> skill for this workspace"`
/// non-interactively. Stdout/stderr stream to the parent terminal.
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

    let status = std::process::Command::new("claude")
        .arg("-p")
        .arg(&prompt)
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
