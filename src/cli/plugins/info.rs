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

    println!("{}", style(&entry.name).bold());
    println!("  Version:     {}", entry.version);
    println!("  Tags:        {}", entry.tags.join(", "));
    println!("  Path:        {}", entry.path);
    println!("  Description: {}", entry.description);
    if entry.default.unwrap_or(false) {
        println!("  Default:     yes (auto-installed by `kimono init`)");
    }
    Ok(())
}
