use anyhow::Result;

use crate::plugins::claude_shell;
use crate::ui;

pub fn run() -> Result<()> {
    ui::header("Running bootstrap skill via Claude Code");
    claude_shell::run_skill("bootstrap")?;
    ui::success("Bootstrap complete");
    Ok(())
}
