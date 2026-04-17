pub mod agents;
pub mod claude_md;
pub mod hookify;
pub mod preserve;
pub mod settings;
pub mod skills;

use anyhow::Result;
use tera::Tera;

/// Create a Tera instance with all kimono templates compiled in.
///
/// Templates are embedded via `include_str!()` so the binary has no
/// runtime dependency on external template files.
pub fn create_tera() -> Result<Tera> {
    let mut tera = Tera::default();
    tera.add_raw_template("claude_md", include_str!("../../templates/claude_md.tera"))?;
    tera.add_raw_template("agent_md", include_str!("../../templates/agent_md.tera"))?;
    tera.add_raw_template(
        "skill_repo",
        include_str!("../../templates/skill_repo.tera"),
    )?;
    tera.add_raw_template(
        "skill_commit",
        include_str!("../../templates/skill_commit.tera"),
    )?;
    tera.add_raw_template(
        "skill_create_pr",
        include_str!("../../templates/skill_create_pr.tera"),
    )?;
    tera.add_raw_template(
        "skill_update_pr",
        include_str!("../../templates/skill_update_pr.tera"),
    )?;
    tera.add_raw_template(
        "skill_pr_review",
        include_str!("../../templates/skill_pr_review.tera"),
    )?;
    tera.add_raw_template(
        "hookify_no_commit_apps",
        include_str!("../../templates/hookify_no_commit_apps.tera"),
    )?;
    tera.add_raw_template(
        "hookify_no_checkout_apps",
        include_str!("../../templates/hookify_no_checkout_apps.tera"),
    )?;
    tera.add_raw_template(
        "settings_json",
        include_str!("../../templates/settings_json.tera"),
    )?;
    tera.add_raw_template("gitignore", include_str!("../../templates/gitignore.tera"))?;
    Ok(tera)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tera_loads_all_templates() {
        let tera = create_tera().expect("should load all templates");
        let names = tera.get_template_names().collect::<Vec<_>>();
        assert!(names.contains(&"claude_md"));
        assert!(names.contains(&"agent_md"));
        assert!(names.contains(&"skill_repo"));
        assert!(names.contains(&"skill_commit"));
        assert!(names.contains(&"skill_create_pr"));
        assert!(names.contains(&"skill_update_pr"));
        assert!(names.contains(&"skill_pr_review"));
        assert!(names.contains(&"hookify_no_commit_apps"));
        assert!(names.contains(&"hookify_no_checkout_apps"));
        assert!(names.contains(&"settings_json"));
        assert!(names.contains(&"gitignore"));
    }
}
