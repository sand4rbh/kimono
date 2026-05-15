use anyhow::{bail, Result};
use console::style;

use crate::config;
use crate::plugins::registry;

pub fn run(name: &str) -> Result<()> {
    let cfg = config::load_and_validate().ok();
    let registry_url = cfg
        .as_ref()
        .and_then(|c| c.plugins.as_ref())
        .and_then(|p| p.registry.as_deref())
        .unwrap_or(registry::DEFAULT_REGISTRY_URL);

    let cache = registry::ensure_cache(registry_url)?;
    let index = registry::load_index(&cache)?;

    let entry = match index.skills.iter().find(|s| s.name == name) {
        Some(e) => e,
        None => bail!("skill '{name}' not found in registry at {registry_url}"),
    };

    let installed_version = cfg
        .as_ref()
        .and_then(|c| c.plugins.as_ref())
        .and_then(|p| p.installed.get(&entry.name).cloned());

    println!();
    println!(
        "  {} {}",
        style(&entry.name).bold().green(),
        style(&entry.version).dim()
    );
    println!("  {}", entry.description);
    println!();
    if !entry.tags.is_empty() {
        println!("  {} {}", style("Tags:").bold(), entry.tags.join(", "));
    }
    println!("  {} {}", style("Path:").bold(), entry.path);
    if entry.default.unwrap_or(false) {
        println!(
            "  {} yes (auto-installed by `kimono init`)",
            style("Default:").bold()
        );
    }
    match installed_version {
        Some(v) if v == entry.version => {
            println!(
                "  {} {} (up to date)",
                style("Installed:").bold(),
                style(&v).green()
            );
        }
        Some(v) => {
            println!(
                "  {} {} (registry has {} — run `kimono plugins update {}` to refresh)",
                style("Installed:").bold(),
                style(&v).yellow(),
                entry.version,
                entry.name
            );
        }
        None => {
            println!(
                "  {} {} — install with `kimono plugins install {}`",
                style("Installed:").bold(),
                style("no").dim(),
                entry.name
            );
        }
    }
    println!();

    Ok(())
}
