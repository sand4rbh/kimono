use anyhow::Result;

use crate::plugins::claude_shell;
use crate::ui;

pub fn run() -> Result<()> {
    ui::header("Running bootstrap skill via Claude Code");
    ui::warn(
        "This launches an interactive Claude Code session and consumes API tokens. Ctrl+C to abort.",
    );
    claude_shell::run_skill("bootstrap")?;
    ui::success("Bootstrap complete");
    Ok(())
}
