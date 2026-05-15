use std::collections::HashSet;

use anyhow::Result;
use dialoguer::{theme::ColorfulTheme, MultiSelect};

use crate::config;
use crate::plugins::{install, registry};
use crate::ui;

pub fn run(names: &[String], version: Option<&str>) -> Result<()> {
    if !names.is_empty() {
        for name in names {
            install::install(name, version)?;
            ui::success(&format!("Installed {name}"));
        }
        return Ok(());
    }
    run_picker()
}

fn run_picker() -> Result<()> {
    let cfg = config::load_and_validate().ok();
    let installed: HashSet<String> = cfg
        .as_ref()
        .and_then(|c| c.plugins.as_ref())
        .map(|p| p.installed.keys().cloned().collect())
        .unwrap_or_default();

    let registry_url = cfg
        .as_ref()
        .and_then(|c| c.plugins.as_ref())
        .and_then(|p| p.registry.as_deref())
        .unwrap_or(registry::DEFAULT_REGISTRY_URL);

    let cache = registry::ensure_cache(registry_url)?;
    let index = registry::load_index(&cache)?;

    let installable: Vec<&registry::SkillEntry> = index
        .skills
        .iter()
        .filter(|s| !installed.contains(&s.name))
        .collect();

    if installable.is_empty() {
        ui::info(
            "All skills are already installed. Run `kimono plugins update` to refresh them.",
        );
        return Ok(());
    }

    let items: Vec<String> = installable
        .iter()
        .map(|s| format!("{} ({})  —  {}", s.name, s.version, s.description))
        .collect();
    let defaults: Vec<bool> = installable
        .iter()
        .map(|s| s.default.unwrap_or(false))
        .collect();

    ui::header("Install skills");
    let selection = MultiSelect::with_theme(&ColorfulTheme::default())
        .with_prompt("Space to toggle, enter to confirm")
        .items(&items)
        .defaults(&defaults)
        .interact()?;

    if selection.is_empty() {
        ui::info("No skills selected — nothing installed.");
        return Ok(());
    }

    for idx in selection {
        let entry = installable[idx];
        install::install(&entry.name, None)?;
        ui::success(&format!("Installed {} {}", entry.name, entry.version));
    }
    Ok(())
}
