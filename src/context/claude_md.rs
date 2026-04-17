use anyhow::{Context, Result};
use tera::Tera;

use crate::config::KimonoConfig;

/// Generate the root CLAUDE.md content from the workspace config.
pub fn generate(config: &KimonoConfig, tera: &Tera) -> Result<String> {
    let mut ctx = tera::Context::new();

    ctx.insert("workspace", &config.workspace);
    ctx.insert("repos", &config.repos);

    // Determine if dispatch is enabled.
    let dispatch = config.claude.as_ref().map(|c| c.dispatch).unwrap_or(false);
    ctx.insert("dispatch", &dispatch);

    let rendered = tera
        .render("claude_md", &ctx)
        .context("failed to render claude_md template")?;

    Ok(rendered)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ClaudeConfig, KimonoConfig, Repo, Workspace};
    use std::collections::HashMap;

    fn fixture_config() -> KimonoConfig {
        let mut repos = HashMap::new();
        repos.insert(
            "backend".to_string(),
            Repo {
                remote: "git@github.com:myorg/backend.git".to_string(),
                branch: "main".to_string(),
                description: Some("NestJS GraphQL API".to_string()),
                tech: vec![
                    "nestjs".to_string(),
                    "typescript".to_string(),
                    "graphql".to_string(),
                ],
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
                description: Some("Next.js web application".to_string()),
                tech: vec![
                    "nextjs".to_string(),
                    "react".to_string(),
                    "typescript".to_string(),
                ],
                package_manager: Some("pnpm".to_string()),
                depends_on: vec!["backend".to_string()],
                commands: {
                    let mut cmds = HashMap::new();
                    cmds.insert("dev".to_string(), "pnpm dev".to_string());
                    cmds
                },
            },
        );

        KimonoConfig {
            workspace: Workspace {
                name: "my-platform".to_string(),
                apps_dir: "apps".to_string(),
                worktree_dir: ".worktrees".to_string(),
            },
            repos,
            claude: Some(ClaudeConfig {
                dispatch: true,
                agents: true,
                skills: true,
                context_style: "detailed".to_string(),
            }),
        }
    }

    #[test]
    fn test_generate_produces_valid_content() {
        let config = fixture_config();
        let tera = crate::context::create_tera().expect("should create tera");
        let content = generate(&config, &tera).expect("should render");

        // Should contain the workspace name.
        assert!(
            content.contains("my-platform"),
            "should contain workspace name"
        );

        // Should contain repo names.
        assert!(content.contains("backend"), "should contain backend repo");
        assert!(content.contains("frontend"), "should contain frontend repo");

        // Should contain section markers.
        assert!(content.contains("<!-- kimono:start:overview -->"));
        assert!(content.contains("<!-- kimono:end:overview -->"));
        assert!(content.contains("<!-- kimono:start:registry -->"));
        assert!(content.contains("<!-- kimono:end:registry -->"));
        assert!(content.contains("<!-- kimono:start:dependencies -->"));
        assert!(content.contains("<!-- kimono:end:dependencies -->"));
        assert!(content.contains("<!-- kimono:start:commands -->"));
        assert!(content.contains("<!-- kimono:end:commands -->"));
        assert!(content.contains("<!-- kimono:start:worktrees -->"));
        assert!(content.contains("<!-- kimono:end:worktrees -->"));

        // Should contain dispatch section (dispatch is enabled).
        assert!(content.contains("<!-- kimono:start:dispatch -->"));
        assert!(content.contains("<!-- kimono:end:dispatch -->"));

        // Should contain the registry table headers.
        assert!(content.contains("| Repo | Description | Tech Stack | Default Branch |"));

        // Should contain dependency info.
        assert!(content.contains("frontend"));
        assert!(content.contains("backend"));

        // Should contain worktree info.
        assert!(content.contains(".worktrees"));
    }

    #[test]
    fn test_generate_without_dispatch() {
        let mut config = fixture_config();
        config.claude = Some(ClaudeConfig {
            dispatch: false,
            agents: true,
            skills: true,
            context_style: "detailed".to_string(),
        });

        let tera = crate::context::create_tera().expect("should create tera");
        let content = generate(&config, &tera).expect("should render");

        // Dispatch section should not appear.
        assert!(
            !content.contains("<!-- kimono:start:dispatch -->"),
            "dispatch section should not appear when dispatch is disabled"
        );
    }

    #[test]
    fn test_generate_minimal_config() {
        let config = KimonoConfig {
            workspace: Workspace {
                name: "minimal".to_string(),
                apps_dir: "apps".to_string(),
                worktree_dir: ".worktrees".to_string(),
            },
            repos: {
                let mut m = HashMap::new();
                m.insert(
                    "api".to_string(),
                    Repo {
                        remote: "git@github.com:me/api.git".to_string(),
                        branch: "main".to_string(),
                        description: None,
                        tech: vec![],
                        package_manager: None,
                        depends_on: vec![],
                        commands: HashMap::new(),
                    },
                );
                m
            },
            claude: None,
        };

        let tera = crate::context::create_tera().expect("should create tera");
        let content = generate(&config, &tera).expect("should render");

        assert!(content.contains("minimal"), "should contain workspace name");
        assert!(content.contains("api"), "should contain repo name");
        assert!(
            !content.contains("<!-- kimono:start:dispatch -->"),
            "dispatch should not appear with no claude config"
        );
    }
}
