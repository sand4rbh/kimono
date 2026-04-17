use std::collections::HashMap;

use anyhow::{Context, Result};
use tera::Tera;

use crate::config::KimonoConfig;

/// Generate hookify rule files.
///
/// Returns a map of filename -> rendered content for each hookify rule.
pub fn generate_all(config: &KimonoConfig, tera: &Tera) -> Result<HashMap<String, String>> {
    let mut rules = HashMap::new();

    let mut ctx = tera::Context::new();
    ctx.insert("workspace", &config.workspace);
    ctx.insert("apps_dir", &config.workspace.apps_dir);
    ctx.insert("worktree_dir", &config.workspace.worktree_dir);

    let no_commit = tera
        .render("hookify_no_commit_apps", &ctx)
        .context("failed to render hookify no-commit-apps rule")?;
    rules.insert(
        "hookify.no-direct-apps-commit.local.md".to_string(),
        no_commit,
    );

    let no_checkout = tera
        .render("hookify_no_checkout_apps", &ctx)
        .context("failed to render hookify no-checkout-apps rule")?;
    rules.insert(
        "hookify.no-direct-apps-checkout.local.md".to_string(),
        no_checkout,
    );

    Ok(rules)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{KimonoConfig, Repo, Workspace};

    fn fixture_config() -> KimonoConfig {
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

        KimonoConfig {
            workspace: Workspace {
                name: "test".to_string(),
                apps_dir: "apps".to_string(),
                worktree_dir: ".worktrees".to_string(),
            },
            repos,
            claude: None,
        }
    }

    #[test]
    fn test_generate_hookify_rules() {
        let config = fixture_config();
        let tera = crate::context::create_tera().expect("should create tera");
        let rules = generate_all(&config, &tera).expect("should generate hookify rules");

        assert_eq!(rules.len(), 2);

        // No-commit rule.
        let no_commit = &rules["hookify.no-direct-apps-commit.local.md"];
        assert!(no_commit.contains("name: no-direct-apps-commit"));
        assert!(no_commit.contains("event: PreToolUse"));
        assert!(no_commit.contains("pattern: Bash"));
        assert!(no_commit.contains("action: block"));
        assert!(no_commit.contains("apps/"));

        // No-checkout rule.
        let no_checkout = &rules["hookify.no-direct-apps-checkout.local.md"];
        assert!(no_checkout.contains("name: no-direct-apps-checkout"));
        assert!(no_checkout.contains("event: PreToolUse"));
        assert!(no_checkout.contains("action: block"));
        assert!(no_checkout.contains("apps/"));
    }

    #[test]
    fn test_hookify_rules_use_custom_dirs() {
        let config = KimonoConfig {
            workspace: Workspace {
                name: "test".to_string(),
                apps_dir: "services".to_string(),
                worktree_dir: ".wt".to_string(),
            },
            repos: HashMap::new(),
            claude: None,
        };

        let tera = crate::context::create_tera().expect("should create tera");
        let rules = generate_all(&config, &tera).expect("should generate hookify rules");

        let no_commit = &rules["hookify.no-direct-apps-commit.local.md"];
        assert!(
            no_commit.contains("services/"),
            "should use custom apps_dir"
        );
        assert!(no_commit.contains(".wt/"), "should use custom worktree_dir");
    }
}
