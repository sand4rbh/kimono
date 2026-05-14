use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_apps_dir() -> String {
    "apps".to_string()
}

fn default_worktree_dir() -> String {
    ".worktrees".to_string()
}

fn default_branch() -> String {
    "main".to_string()
}

/// Top-level kimono configuration parsed from .kimono/config.yml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KimonoConfig {
    pub workspace: Workspace,
    pub repos: HashMap<String, Repo>,
    #[serde(default)]
    pub plugins: Option<PluginsConfig>,
}

/// Workspace-level settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    #[serde(default = "default_apps_dir")]
    pub apps_dir: String,
    #[serde(default = "default_worktree_dir")]
    pub worktree_dir: String,
}

/// Configuration for a single repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub remote: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    pub description: Option<String>,
    #[serde(default)]
    pub tech: Vec<String>,
    pub package_manager: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub commands: HashMap<String, String>,
}

/// Plugin (skill) configuration. Defaults are implicit: `registry` falls
/// back to the binary's hardcoded URL; `installed` defaults to empty.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginsConfig {
    pub registry: Option<String>,
    #[serde(default)]
    pub installed: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_config() {
        let yaml = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/config.yml"
        ))
        .expect("failed to read full config fixture");
        let config: KimonoConfig =
            serde_yaml::from_str(&yaml).expect("failed to parse full config");

        assert_eq!(config.workspace.name, "my-platform");
        assert_eq!(config.workspace.apps_dir, "apps");
        assert_eq!(config.workspace.worktree_dir, ".worktrees");

        assert_eq!(config.repos.len(), 3);

        let backend = config.repos.get("backend").expect("missing backend");
        assert_eq!(backend.remote, "git@github.com:myorg/backend.git");
        assert_eq!(backend.branch, "main");
        assert_eq!(backend.description.as_deref(), Some("NestJS GraphQL API"));
        assert_eq!(
            backend.tech,
            vec!["nestjs", "typescript", "graphql", "postgresql"]
        );
        assert_eq!(backend.package_manager.as_deref(), Some("yarn"));
        assert!(backend.depends_on.is_empty());
        assert_eq!(backend.commands.get("dev").unwrap(), "yarn dev");

        let frontend = config.repos.get("frontend").expect("missing frontend");
        assert_eq!(frontend.depends_on, vec!["backend"]);

        // Plugins section
        let plugins = config.plugins.as_ref().expect("missing plugins config");
        assert_eq!(plugins.installed.get("bootstrap").unwrap(), "1.0.0");
    }

    #[test]
    fn test_parse_minimal_config() {
        let yaml = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/config_minimal.yml"
        ))
        .expect("failed to read minimal config fixture");
        let config: KimonoConfig =
            serde_yaml::from_str(&yaml).expect("failed to parse minimal config");

        assert_eq!(config.workspace.name, "my-project");
        assert_eq!(config.repos.len(), 1);
        assert!(config.plugins.is_none());
    }

    #[test]
    fn test_required_fields_error() {
        let yaml = r#"
workspace:
  apps_dir: apps
repos:
  api:
    remote: git@github.com:me/api.git
"#;
        let result: Result<KimonoConfig, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err(), "should error when workspace.name is missing");
    }

    #[test]
    fn test_defaults() {
        let yaml = r#"
workspace:
  name: test-project
repos:
  svc:
    remote: git@github.com:me/svc.git
"#;
        let config: KimonoConfig = serde_yaml::from_str(yaml).expect("failed to parse");

        assert_eq!(config.workspace.apps_dir, "apps");
        assert_eq!(config.workspace.worktree_dir, ".worktrees");

        let svc = config.repos.get("svc").unwrap();
        assert_eq!(svc.branch, "main");
    }

    #[test]
    fn test_parse_plugins_config() {
        let yaml = r#"
workspace:
  name: test-project
repos:
  svc:
    remote: git@github.com:me/svc.git
plugins:
  registry: github.com/myorg/my-registry
  installed:
    bootstrap: 1.0.0
    discover: 1.0.0
"#;
        let config: KimonoConfig = serde_yaml::from_str(yaml).expect("failed to parse");

        let plugins = config.plugins.as_ref().expect("missing plugins config");
        assert_eq!(
            plugins.registry.as_deref(),
            Some("github.com/myorg/my-registry")
        );
        assert_eq!(plugins.installed.get("bootstrap").unwrap(), "1.0.0");
        assert_eq!(plugins.installed.get("discover").unwrap(), "1.0.0");
    }

    #[test]
    fn test_plugins_optional() {
        let yaml = r#"
workspace:
  name: test-project
repos:
  svc:
    remote: git@github.com:me/svc.git
"#;
        let config: KimonoConfig = serde_yaml::from_str(yaml).expect("failed to parse");
        assert!(config.plugins.is_none());
    }
}
