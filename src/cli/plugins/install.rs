use anyhow::Result;

use crate::plugins::install;
use crate::ui;

pub fn run(names: &[String], version: Option<&str>) -> Result<()> {
    if names.is_empty() {
        anyhow::bail!("provide at least one skill name to install");
    }
    for name in names {
        install::install(name, version)?;
        ui::success(&format!("Installed {name}"));
    }
    Ok(())
}
