use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, MultiSelect};

use crate::config;
use crate::plugins::install;
use crate::ui;

pub fn run(names: &[String]) -> Result<()> {
    if !names.is_empty() {
        for name in names {
            install::remove(name)?;
            ui::success(&format!("Removed {name}"));
        }
        return Ok(());
    }
    run_picker()
}

fn run_picker() -> Result<()> {
    let cfg = config::load_and_validate()?;
    let installed_map = cfg
        .plugins
        .as_ref()
        .map(|p| p.installed.clone())
        .unwrap_or_default();

    if installed_map.is_empty() {
        ui::info("No skills installed.");
        return Ok(());
    }

    let mut entries: Vec<(String, String)> = installed_map.into_iter().collect();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    let items: Vec<String> = entries
        .iter()
        .map(|(name, version)| format!("{} ({})", name, version))
        .collect();

    ui::header("Remove skills");
    let selection = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Space to toggle, enter to confirm")
        .items(&items)
        .interact()?;

    if selection.is_empty() {
        ui::info("No skills selected — nothing removed.");
        return Ok(());
    }

    for idx in selection {
        let name = &entries[idx].0;
        install::remove(name)?;
        ui::success(&format!("Removed {name}"));
    }
    Ok(())
}
