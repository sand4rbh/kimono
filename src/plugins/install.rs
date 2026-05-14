use std::path::Path;

use anyhow::{Context, Result};

use crate::config;
use crate::config::{KimonoConfig, PluginsConfig};
use crate::plugins::registry;

/// Install a skill from the cached registry into the workspace's
/// `.claude/skills/<name>/` and record the version in
/// `.kimono/config.yml > plugins.installed`.
///
/// If `version` is None, installs the version listed in the cache's
/// `index.json`. If `version` is Some, the function still installs the
/// version present in the cache (it does not support multiple versions
/// in v2) and errors if `version` doesn't match.
pub fn install(name: &str, version: Option<&str>) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;

    let registry_url = cfg
        .plugins
        .as_ref()
        .and_then(|p| p.registry.as_deref())
        .unwrap_or(registry::DEFAULT_REGISTRY_URL);

    let cache = registry::ensure_cache(registry_url)?;
    let index = registry::load_index(&cache)?;

    let entry = index
        .skills
        .iter()
        .find(|s| s.name == name)
        .with_context(|| format!("skill '{name}' not found in registry"))?;

    if let Some(requested) = version {
        if requested != entry.version {
            anyhow::bail!(
                "skill '{name}' is version {} in the registry; requested version {} not available",
                entry.version,
                requested
            );
        }
    }

    let src = cache.join(&entry.path);
    if !src.is_dir() {
        anyhow::bail!(
            "registry layout error: index entry points to {} but that directory does not exist",
            src.display()
        );
    }

    let dest = root.join(".claude").join("skills").join(name);
    if dest.is_dir() {
        std::fs::remove_dir_all(&dest)
            .with_context(|| format!("failed to remove existing {}", dest.display()))?;
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    copy_dir_recursive(&src, &dest)?;

    record_installed(&root, &cfg, name, &entry.version)?;
    Ok(())
}

/// Remove a skill: delete `.claude/skills/<name>/` and the config entry.
pub fn remove(name: &str) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;

    let skill_dir = root.join(".claude").join("skills").join(name);
    if skill_dir.is_dir() {
        std::fs::remove_dir_all(&skill_dir)
            .with_context(|| format!("failed to remove {}", skill_dir.display()))?;
    }

    erase_installed(&root, &cfg, name)?;
    Ok(())
}

/// Update one or more installed skills: re-run install if the registry version
/// differs from the recorded installed version. Empty `names` updates all
/// installed skills. Skills listed in the workspace but absent from the
/// registry produce a warning on stderr (or an error if named explicitly).
pub fn update(names: &[String]) -> Result<()> {
    let cfg = config::load_and_validate()?;
    let installed = cfg
        .plugins
        .as_ref()
        .map(|p| p.installed.clone())
        .unwrap_or_default();

    let names_explicit = !names.is_empty();
    let targets: Vec<String> = if names.is_empty() {
        installed.keys().cloned().collect()
    } else {
        for n in names {
            if !installed.contains_key(n) {
                anyhow::bail!("skill '{n}' is not installed");
            }
        }
        names.to_vec()
    };

    let registry_url = cfg
        .plugins
        .as_ref()
        .and_then(|p| p.registry.as_deref())
        .unwrap_or(registry::DEFAULT_REGISTRY_URL);
    let cache = registry::ensure_cache(registry_url)?;
    let index = registry::load_index(&cache)?;

    for name in &targets {
        match index.skills.iter().find(|s| s.name == *name) {
            Some(entry) => {
                let current = installed.get(name).cloned().unwrap_or_default();
                if entry.version != current {
                    install(name, None)?;
                }
            }
            None => {
                if names_explicit {
                    anyhow::bail!(
                        "skill '{name}' is installed locally but not present in the registry; cannot update"
                    );
                } else {
                    eprintln!(
                        "warning: skill '{name}' is installed locally but not present in the registry; skipping"
                    );
                }
            }
        }
    }
    Ok(())
}

fn record_installed(root: &Path, cfg: &KimonoConfig, name: &str, version: &str) -> Result<()> {
    let mut new_cfg = cfg.clone();
    let plugins = new_cfg.plugins.get_or_insert_with(PluginsConfig::default);
    plugins.installed.insert(name.to_string(), version.to_string());
    write_config(root, &new_cfg)
}

fn erase_installed(root: &Path, cfg: &KimonoConfig, name: &str) -> Result<()> {
    let mut new_cfg = cfg.clone();
    if let Some(plugins) = new_cfg.plugins.as_mut() {
        plugins.installed.remove(name);
    }
    write_config(root, &new_cfg)
}

fn write_config(root: &Path, cfg: &KimonoConfig) -> Result<()> {
    let path = root.join(".kimono").join("config.yml");
    let yaml = serde_yaml::to_string(cfg).context("failed to serialize config")?;
    std::fs::write(&path, yaml).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)
        .with_context(|| format!("failed to create {}", dest.display()))?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&src_path, &dest_path).with_context(|| {
                format!(
                    "failed to copy {} -> {}",
                    src_path.display(),
                    dest_path.display()
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn write(path: &Path, content: &str) {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    fn make_cache(root: &Path) -> PathBuf {
        let cache = root.join("cache");
        let skills = cache.join("skills");
        std::fs::create_dir_all(&skills).unwrap();
        write(
            &skills.join("index.json"),
            r#"{
              "schema_version": 1,
              "skills": [
                {"name":"bootstrap","description":"init","version":"1.0.0","tags":[],"path":"skills/bootstrap","default":true},
                {"name":"discover","description":"deep","version":"1.0.0","tags":[],"path":"skills/discover"}
              ]
            }"#,
        );
        write(&skills.join("bootstrap").join("SKILL.md"), "# bootstrap\n");
        write(&skills.join("discover").join("SKILL.md"), "# discover\n");
        cache
    }

    #[test]
    fn test_copy_dir_recursive_preserves_structure() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cache = make_cache(tmp.path());
        let dest = tmp.path().join("installed");
        copy_dir_recursive(&cache.join("skills").join("bootstrap"), &dest).unwrap();
        assert!(dest.join("SKILL.md").is_file());
    }
}
