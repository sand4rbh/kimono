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
    let installed = cfg
        .as_ref()
        .and_then(|c| c.plugins.as_ref())
        .map(|p| p.installed.clone())
        .unwrap_or_default();

    if scope == Scope::Installed {
        ui::header("Installed skills");
        if installed.is_empty() {
            ui::info("(none)");
            return Ok(());
        }
        let mut names: Vec<_> = installed.keys().collect();
        names.sort();
        for name in names {
            println!("  {} {}", style(name).green(), installed[name]);
        }
        return Ok(());
    }

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

    ui::header(&format!("Available skills (registry: {registry_url})"));
    for skill in &index.skills {
        if scope == Scope::Available && installed.contains_key(&skill.name) {
            continue;
        }
        let marker = if installed.contains_key(&skill.name) {
            style("(installed)").dim().to_string()
        } else {
            String::new()
        };
        println!(
            "  {:<18} {:<8} {}  {}",
            style(&skill.name).bold(),
            skill.version,
            marker,
            skill.description
        );
    }
    Ok(())
}
