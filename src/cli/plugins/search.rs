use anyhow::Result;
use console::style;

use crate::config;
use crate::plugins::registry;
use crate::ui;

pub fn run(query: &str) -> Result<()> {
    let cfg = config::load_and_validate().ok();
    let registry_url = cfg
        .as_ref()
        .and_then(|c| c.plugins.as_ref())
        .and_then(|p| p.registry.as_deref())
        .unwrap_or(registry::DEFAULT_REGISTRY_URL);

    let cache = registry::ensure_cache(registry_url)?;
    let index = registry::load_index(&cache)?;

    let q = query.to_lowercase();
    let mut hits = Vec::new();
    for skill in &index.skills {
        if skill.name.to_lowercase().contains(&q)
            || skill.description.to_lowercase().contains(&q)
            || skill.tags.iter().any(|t| t.to_lowercase().contains(&q))
        {
            hits.push(skill);
        }
    }

    ui::header(&format!(
        "Search results for '{query}' ({} hits)",
        hits.len()
    ));
    for skill in hits {
        println!(
            "  {:<18} {:<8} {}",
            style(&skill.name).bold(),
            skill.version,
            skill.description
        );
    }
    Ok(())
}
