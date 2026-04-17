use anyhow::Result;
use serde_json::json;

use crate::config::KimonoConfig;

/// Generate `.claude/settings.json` content.
///
/// The settings file lists all repo directories as `additionalDirectories`
/// so Claude Code can navigate across the workspace.
pub fn generate(config: &KimonoConfig) -> Result<String> {
    let mut dirs: Vec<String> = config
        .repos
        .keys()
        .map(|name| format!("{}/{}", config.workspace.apps_dir, name))
        .collect();

    // Sort for deterministic output.
    dirs.sort();

    let settings = json!({
        "additionalDirectories": dirs,
    });

    let rendered = serde_json::to_string_pretty(&settings)?;
    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{KimonoConfig, Repo, Workspace};
    use std::collections::HashMap;

    #[test]
    fn test_generate_settings() {
        let mut repos = HashMap::new();
        repos.insert(
            "backend".to_string(),
            Repo {
                remote: "git@github.com:x/backend.git".to_string(),
                branch: "main".to_string(),
                description: None,
                tech: vec![],
                package_manager: None,
                depends_on: vec![],
                commands: HashMap::new(),
            },
        );
        repos.insert(
            "frontend".to_string(),
            Repo {
                remote: "git@github.com:x/frontend.git".to_string(),
                branch: "main".to_string(),
                description: None,
                tech: vec![],
                package_manager: None,
                depends_on: vec![],
                commands: HashMap::new(),
            },
        );

        let config = KimonoConfig {
            workspace: Workspace {
                name: "test".to_string(),
                apps_dir: "apps".to_string(),
                worktree_dir: ".worktrees".to_string(),
            },
            repos,
            claude: None,
        };

        let content = generate(&config).expect("should generate settings");

        // Parse back as JSON to verify structure.
        let parsed: serde_json::Value =
            serde_json::from_str(&content).expect("output should be valid JSON");

        let dirs = parsed["additionalDirectories"]
            .as_array()
            .expect("should have additionalDirectories array");

        assert_eq!(dirs.len(), 2);

        let dir_strs: Vec<&str> = dirs.iter().map(|v| v.as_str().unwrap()).collect();
        assert!(dir_strs.contains(&"apps/backend"));
        assert!(dir_strs.contains(&"apps/frontend"));

        // Should be sorted.
        assert_eq!(dir_strs[0], "apps/backend");
        assert_eq!(dir_strs[1], "apps/frontend");
    }

    #[test]
    fn test_generate_settings_custom_apps_dir() {
        let mut repos = HashMap::new();
        repos.insert(
            "svc".to_string(),
            Repo {
                remote: "git@github.com:x/svc.git".to_string(),
                branch: "main".to_string(),
                description: None,
                tech: vec![],
                package_manager: None,
                depends_on: vec![],
                commands: HashMap::new(),
            },
        );

        let config = KimonoConfig {
            workspace: Workspace {
                name: "test".to_string(),
                apps_dir: "services".to_string(),
                worktree_dir: ".worktrees".to_string(),
            },
            repos,
            claude: None,
        };

        let content = generate(&config).expect("should generate settings");
        assert!(
            content.contains("services/svc"),
            "should use the custom apps_dir"
        );
    }
}
