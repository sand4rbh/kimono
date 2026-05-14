use anyhow::Result;

use crate::plugins::claude_shell;
use crate::ui;

pub fn run() -> Result<()> {
    ui::header("Running discover skill via Claude Code");
    claude_shell::run_skill("discover")?;
    ui::success("Discover complete");
    Ok(())
}
