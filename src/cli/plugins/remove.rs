use anyhow::Result;

use crate::plugins::install;
use crate::ui;

pub fn run(names: &[String]) -> Result<()> {
    if names.is_empty() {
        anyhow::bail!("provide at least one skill name to remove");
    }
    for name in names {
        install::remove(name)?;
        ui::success(&format!("Removed {name}"));
    }
    Ok(())
}
