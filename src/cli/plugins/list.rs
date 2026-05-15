use std::collections::HashMap;

use anyhow::Result;
use console::style;

use crate::config;
use crate::plugins::registry;
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    All,
    Installed,
    Available,
}

pub fn run(scope: Scope) -> Result<()> {
    let cfg = config::load_and_validate().ok();
    let installed: HashMap<String, String> = cfg
        .as_ref()
        .and_then(|c| c.plugins.as_ref())
        .map(|p| p.installed.clone())
        .unwrap_or_default();

    // --installed renders from local config alone, no network or registry needed.
    if scope == Scope::Installed {
        ui::header("Installed skills");
        if installed.is_empty() {
            println!("  (none) — run `kimono plugins install` to pick from the catalog");
            return Ok(());
        }
        let mut names: Vec<&String> = installed.keys().collect();
        names.sort();
        for name in &names {
            println!(
                "  {} {} {}",
                style("✓").green(),
                style(name).bold(),
                style(&installed[*name]).dim()
            );
        }
        println!();
        ui::info(&format!("{} skills installed", names.len()));
        return Ok(());
    }

    // --available and the default scope both need the registry index.
    let registry_url = cfg
        .as_ref()
        .and_then(|c| c.plugins.as_ref())
        .and_then(|p| p.registry.as_deref())
        .unwrap_or(registry::DEFAULT_REGISTRY_URL);

    let cache = match registry::ensure_cache(registry_url) {
        Ok(c) => c,
        Err(e) => {
            ui::error(&format!(
                "Registry cache empty and `git clone` failed — run with internet to populate.\n  {e}"
            ));
            return Err(e);
        }
    };
    let index = registry::load_index(&cache)?;

    ui::header(&format!("Skills (registry: {registry_url})"));

    let filtered: Vec<&registry::SkillEntry> = index
        .skills
        .iter()
        .filter(|s| match scope {
            Scope::Available => !installed.contains_key(&s.name),
            _ => true,
        })
        .collect();

    let total_count = index.skills.len();
    let installed_count = installed.len();
    let available_count = total_count.saturating_sub(installed_count);

    println!();
    for skill in &filtered {
        let is_installed = installed.contains_key(&skill.name);
        let status_glyph = if is_installed {
            style("✓").green().to_string()
        } else {
            style("○").dim().to_string()
        };
        let name_styled = if is_installed {
            style(&skill.name).bold().to_string()
        } else {
            style(&skill.name).bold().dim().to_string()
        };
        let tags = if skill.tags.is_empty() {
            String::new()
        } else {
            format!(
                " {}",
                style(format!("[{}]", skill.tags.join(", "))).dim()
            )
        };
        println!(
            "  {} {}  {}{}",
            status_glyph,
            name_styled,
            style(&skill.version).dim(),
            tags,
        );
        println!("      {}", skill.description);
    }

    println!();
    ui::info(&format!(
        "{} total · {} installed · {} available",
        total_count, installed_count, available_count
    ));
    Ok(())
}
