use anyhow::Result;

use crate::plugins::claude_shell;
use crate::ui;

pub fn run() -> Result<()> {
    ui::header("Running discover skill via Claude Code");
    ui::warn(
        "This launches an interactive Claude Code session and walks the source tree — expect significant token usage. Ctrl+C to abort.",
    );
    claude_shell::run_skill("discover")?;
    ui::success("Discover complete");
    Ok(())
}
