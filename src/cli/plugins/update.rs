use anyhow::Result;

use crate::plugins::install;
use crate::ui;

pub fn run(names: &[String]) -> Result<()> {
    install::update(names)?;
    if names.is_empty() {
        ui::success("All installed skills checked for updates");
    } else {
        ui::success(&format!("Checked: {}", names.join(", ")));
    }
    Ok(())
}
