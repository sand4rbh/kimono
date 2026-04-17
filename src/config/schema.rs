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

fn default_true() -> bool {
    true
}

fn default_context_style() -> String {
    "detailed".to_string()
}

/// Top-level kimono configuration parsed from .kimono/config.yml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KimonoConfig {
    pub workspace: Workspace,
    pub repos: HashMap<String, Repo>,
    pub claude: Option<ClaudeConfig>,
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

/// Claude-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeConfig {
    #[serde(default = "default_true")]
    pub dispatch: bool,
    #[serde(default = "default_true")]
    pub agents: bool,
    #[serde(default = "default_true")]
    pub skills: bool,
    #[serde(default = "default_context_style")]
    pub context_style: String,
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

        // Workspace
        assert_eq!(config.workspace.name, "my-platform");
        assert_eq!(config.workspace.apps_dir, "apps");
        assert_eq!(config.workspace.worktree_dir, ".worktrees");

        // Repos
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
        assert_eq!(backend.commands.get("lint").unwrap(), "yarn lint");
        assert_eq!(
            backend.commands.get("typecheck").unwrap(),
            "npx tsc --noEmit"
        );
        assert_eq!(backend.commands.get("test").unwrap(), "yarn test");

        let frontend = config.repos.get("frontend").expect("missing frontend");
        assert_eq!(frontend.remote, "git@github.com:myorg/frontend.git");
        assert_eq!(frontend.depends_on, vec!["backend"]);
        assert_eq!(frontend.package_manager.as_deref(), Some("pnpm"));
        assert_eq!(frontend.commands.get("dev").unwrap(), "pnpm dev");

        let mobile = config.repos.get("mobile").expect("missing mobile");
        assert_eq!(mobile.remote, "git@github.com:myorg/mobile.git");
        assert_eq!(mobile.depends_on, vec!["backend"]);
        assert_eq!(mobile.tech, vec!["react-native", "typescript", "expo"]);

        // Claude
        let claude = config.claude.as_ref().expect("missing claude config");
        assert!(claude.dispatch);
        assert!(claude.agents);
        assert!(claude.skills);
        assert_eq!(claude.context_style, "detailed");
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
        assert_eq!(config.workspace.apps_dir, "apps");
        assert_eq!(config.workspace.worktree_dir, ".worktrees");

        assert_eq!(config.repos.len(), 1);
        let api = config.repos.get("api").expect("missing api repo");
        assert_eq!(api.remote, "git@github.com:me/api.git");
        assert_eq!(api.branch, "main");
        assert!(api.description.is_none());
        assert!(api.tech.is_empty());
        assert!(api.package_manager.is_none());
        assert!(api.depends_on.is_empty());
        assert!(api.commands.is_empty());

        assert!(config.claude.is_none());
    }

    #[test]
    fn test_required_fields_error() {
        // Missing workspace.name
        let yaml = r#"
workspace:
  apps_dir: apps
repos:
  api:
    remote: git@github.com:me/api.git
"#;
        let result: Result<KimonoConfig, _> = serde_yaml::from_str(yaml);
        assert!(
            result.is_err(),
            "should error when workspace.name is missing"
        );
    }

    #[test]
    fn test_defaults() {
        let yaml = r#"
workspace:
  name: test-project
repos:
  svc:
    remote: git@github.com:me/svc.git
claude:
  dispatch: false
"#;
        let config: KimonoConfig = serde_yaml::from_str(yaml).expect("failed to parse");

        // Workspace defaults
        assert_eq!(config.workspace.apps_dir, "apps");
        assert_eq!(config.workspace.worktree_dir, ".worktrees");

        // Repo defaults
        let svc = config.repos.get("svc").unwrap();
        assert_eq!(svc.branch, "main");
        assert!(svc.tech.is_empty());
        assert!(svc.depends_on.is_empty());
        assert!(svc.commands.is_empty());

        // Claude defaults (dispatch explicitly false, rest default to true)
        let claude = config.claude.as_ref().unwrap();
        assert!(!claude.dispatch);
        assert!(claude.agents);
        assert!(claude.skills);
        assert_eq!(claude.context_style, "detailed");
    }
}
