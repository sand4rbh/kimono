pub mod add;
pub mod branch;
pub mod clone;
pub mod context;
pub mod exec;
pub mod init;
pub mod plugins;
pub mod remove;
pub mod status;
pub mod sync;
pub mod wt;

use crate::config::{KimonoConfig, Repo};
use anyhow::{bail, Result};

/// Filter repos from config by the names the user provided on the CLI.
///
/// If `repos` is empty, returns all repos from the config.
/// If `repos` is non-empty, returns only the matching repos and errors if
/// any requested name is not found in the config.
pub fn filter_repos<'a>(
    config: &'a KimonoConfig,
    repos: &[String],
) -> Result<Vec<(&'a String, &'a Repo)>> {
    if repos.is_empty() {
        let mut all: Vec<(&String, &Repo)> = config.repos.iter().collect();
        all.sort_by_key(|(name, _)| name.as_str().to_owned());
        return Ok(all);
    }

    let mut result = Vec::new();
    for name in repos {
        match config.repos.get_key_value(name) {
            Some((k, v)) => result.push((k, v)),
            None => bail!(
                "repo '{}' not found in config. Available repos: {}",
                name,
                config.repos.keys().cloned().collect::<Vec<_>>().join(", ")
            ),
        }
    }
    Ok(result)
}
