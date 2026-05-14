use std::process::Command;

use anyhow::{Context, Result};

/// Returns true if `claude` is on PATH.
pub fn is_available() -> bool {
    which::which("claude").is_ok()
}

/// Launch an interactive Claude Code session with the kimono skill prompt as
/// the first user message. We deliberately do NOT use `claude -p` here:
/// `-p` buffers the full response before printing, hiding the per-tool
/// progress (file reads, edits, bash) that's the whole point of running a
/// skill in real time. Interactive mode gives the user the full TUI — they
/// see every file Claude reads, every edit, every command — and can
/// intervene or quit at any point.
///
/// All stdio inherits from the parent terminal so the TUI renders directly.
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
        "  Launching Claude Code with the {skill_name} prompt — exit the session with /exit when done."
    );

    let status = Command::new("claude")
        .arg(&prompt)
        .status()
        .with_context(|| format!("failed to spawn `claude` for skill {skill_name}"))?;

    if !status.success() {
        anyhow::bail!(
            "`claude` for skill '{skill_name}' exited with status {}",
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
