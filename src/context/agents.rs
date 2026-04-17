use std::collections::HashMap;

use anyhow::{Context, Result};
use tera::Tera;

use crate::config::KimonoConfig;

/// Generate AGENT.md content for every repo in the config.
///
/// Returns a map of repo_name -> rendered agent markdown.
pub fn generate_all(config: &KimonoConfig, tera: &Tera) -> Result<HashMap<String, String>> {
    let mut agents = HashMap::new();

    for (name, repo) in &config.repos {
        let mut ctx = tera::Context::new();
        ctx.insert("name", name);
        ctx.insert("repo", repo);
        ctx.insert("apps_dir", &config.workspace.apps_dir);
        ctx.insert("worktree_dir", &config.workspace.worktree_dir);

        let rendered = tera
            .render("agent_md", &ctx)
            .with_context(|| format!("failed to render agent_md for repo '{}'", name))?;

        agents.insert(name.clone(), rendered);
    }

    Ok(agents)
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
                remote: "git@github.com:myorg/backend.git".to_string(),
                branch: "main".to_string(),
                description: Some("NestJS GraphQL API".to_string()),
                tech: vec!["nestjs".to_string(), "typescript".to_string()],
                package_manager: Some("yarn".to_string()),
                depends_on: vec![],
                commands: {
                    let mut cmds = HashMap::new();
                    cmds.insert("dev".to_string(), "yarn dev".to_string());
                    cmds.insert("test".to_string(), "yarn test".to_string());
                    cmds
                },
            },
        );
        repos.insert(
            "frontend".to_string(),
            Repo {
                remote: "git@github.com:myorg/frontend.git".to_string(),
                branch: "main".to_string(),
                description: Some("Next.js web app".to_string()),
                tech: vec!["nextjs".to_string(), "react".to_string()],
                package_manager: Some("pnpm".to_string()),
                depends_on: vec!["backend".to_string()],
                commands: HashMap::new(),
            },
        );

        KimonoConfig {
            workspace: Workspace {
                name: "test-workspace".to_string(),
                apps_dir: "apps".to_string(),
                worktree_dir: ".worktrees".to_string(),
            },
            repos,
            claude: None,
        }
    }

    #[test]
    fn test_generate_all_agents() {
        let config = fixture_config();
        let tera = crate::context::create_tera().expect("should create tera");
        let agents = generate_all(&config, &tera).expect("should generate agents");

        assert_eq!(agents.len(), 2);
        assert!(agents.contains_key("backend"));
        assert!(agents.contains_key("frontend"));

        let backend = &agents["backend"];
        assert!(
            backend.contains("name: backend"),
            "should have YAML frontmatter name"
        );
        assert!(
            backend.contains("NestJS GraphQL API"),
            "should contain description"
        );
        assert!(backend.contains("nestjs"), "should list tech stack");
        assert!(backend.contains("yarn"), "should mention package manager");
        assert!(backend.contains("yarn dev"), "should list commands");
        assert!(
            backend.contains("apps/backend/"),
            "should reference working directory"
        );

        let frontend = &agents["frontend"];
        assert!(frontend.contains("name: frontend"));
        assert!(frontend.contains("Next.js web app"));
        assert!(
            frontend.contains("depends on: backend"),
            "should list dependencies"
        );
    }
}
