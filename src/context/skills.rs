use std::collections::HashMap;

use anyhow::{Context, Result};
use tera::Tera;

use crate::config::KimonoConfig;

/// Generate a SKILL.md dispatcher for each repo.
///
/// Returns a map of repo_name -> rendered skill markdown.
pub fn generate_repo_skills(config: &KimonoConfig, tera: &Tera) -> Result<HashMap<String, String>> {
    let mut skills = HashMap::new();

    for (name, repo) in &config.repos {
        let mut ctx = tera::Context::new();
        ctx.insert("name", name);
        ctx.insert("repo", repo);
        ctx.insert("apps_dir", &config.workspace.apps_dir);
        ctx.insert("worktree_dir", &config.workspace.worktree_dir);

        let rendered = tera
            .render("skill_repo", &ctx)
            .with_context(|| format!("failed to render skill_repo for '{}'", name))?;

        skills.insert(name.clone(), rendered);
    }

    Ok(skills)
}

/// Generate workflow skills (commit, create-pr, update-pr, pr-review-comments).
///
/// Returns a map of skill_name -> rendered skill markdown.
pub fn generate_workflow_skills(
    config: &KimonoConfig,
    tera: &Tera,
) -> Result<HashMap<String, String>> {
    let mut skills = HashMap::new();

    let workflow_templates = [
        ("commit", "skill_commit"),
        ("create-pr", "skill_create_pr"),
        ("update-pr", "skill_update_pr"),
        ("pr-review-comments", "skill_pr_review"),
    ];

    for (skill_name, template_name) in &workflow_templates {
        let mut ctx = tera::Context::new();
        ctx.insert("workspace", &config.workspace);
        ctx.insert("repos", &config.repos);
        ctx.insert("apps_dir", &config.workspace.apps_dir);
        ctx.insert("worktree_dir", &config.workspace.worktree_dir);

        let rendered = tera
            .render(template_name, &ctx)
            .with_context(|| format!("failed to render workflow skill '{}'", skill_name))?;

        skills.insert(skill_name.to_string(), rendered);
    }

    Ok(skills)
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
                description: Some("API server".to_string()),
                tech: vec!["nestjs".to_string()],
                package_manager: Some("yarn".to_string()),
                depends_on: vec![],
                commands: {
                    let mut cmds = HashMap::new();
                    cmds.insert("dev".to_string(), "yarn dev".to_string());
                    cmds
                },
            },
        );

        KimonoConfig {
            workspace: Workspace {
                name: "test-ws".to_string(),
                apps_dir: "apps".to_string(),
                worktree_dir: ".worktrees".to_string(),
            },
            repos,
            claude: None,
        }
    }

    #[test]
    fn test_generate_repo_skills() {
        let config = fixture_config();
        let tera = crate::context::create_tera().expect("should create tera");
        let skills = generate_repo_skills(&config, &tera).expect("should generate repo skills");

        assert_eq!(skills.len(), 1);
        assert!(skills.contains_key("backend"));

        let backend = &skills["backend"];
        assert!(
            backend.contains("name: backend"),
            "should have YAML frontmatter"
        );
        assert!(
            backend.contains("user-invocable: true"),
            "should be user-invocable"
        );
        assert!(backend.contains("agent: backend"), "should reference agent");
        assert!(backend.contains("API server"), "should contain description");
        assert!(backend.contains("nestjs"), "should list tech");
        assert!(backend.contains("yarn dev"), "should list commands");
    }

    #[test]
    fn test_generate_workflow_skills() {
        let config = fixture_config();
        let tera = crate::context::create_tera().expect("should create tera");
        let skills =
            generate_workflow_skills(&config, &tera).expect("should generate workflow skills");

        assert_eq!(skills.len(), 4);
        assert!(skills.contains_key("commit"));
        assert!(skills.contains_key("create-pr"));
        assert!(skills.contains_key("update-pr"));
        assert!(skills.contains_key("pr-review-comments"));

        // Each should have YAML frontmatter.
        for (name, content) in &skills {
            assert!(
                content.contains("user-invocable: true"),
                "skill '{}' should be user-invocable",
                name
            );
            assert!(
                content.contains(&format!("name: {}", name)),
                "skill '{}' should have correct name in frontmatter",
                name
            );
        }

        // Commit skill should reference apps_dir.
        assert!(skills["commit"].contains("apps/"));
    }
}
