use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::config;
use crate::context;
use crate::ui;

pub fn run(force: bool) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;
    let tera = context::create_tera()?;

    let claude_dir = root.join(".claude");
    let mut generated_count: u32 = 0;
    let mut updated_count: u32 = 0;

    ui::header("Generating context files");

    // ── 1. CLAUDE.md ─────────────────────────────────────────────────────
    {
        let content =
            context::claude_md::generate(&cfg, &tera).context("failed to generate CLAUDE.md")?;
        let target = root.join("CLAUDE.md");

        let final_content = if target.is_file() && !force {
            let existing = fs::read_to_string(&target)
                .with_context(|| format!("failed to read {}", target.display()))?;
            let merged = context::preserve::merge(&content, &existing);
            fs::write(&target, &merged)
                .with_context(|| format!("failed to write {}", target.display()))?;
            ui::success(&format!("Updated {}", relative_display(&target, &root)));
            updated_count += 1;
            merged
        } else {
            fs::write(&target, &content)
                .with_context(|| format!("failed to write {}", target.display()))?;
            ui::success(&format!("Created {}", relative_display(&target, &root)));
            generated_count += 1;
            content
        };
        let _ = final_content;
    }

    // ── 2. Agent files ───────────────────────────────────────────────────
    {
        let agents =
            context::agents::generate_all(&cfg, &tera).context("failed to generate agent files")?;

        for (repo_name, content) in &agents {
            let dir = claude_dir.join("agents").join(repo_name);
            fs::create_dir_all(&dir)
                .with_context(|| format!("failed to create directory {}", dir.display()))?;

            let target = dir.join("AGENT.md");
            write_file(
                &target,
                content,
                &root,
                force,
                &mut generated_count,
                &mut updated_count,
            )?;
        }
    }

    // ── 3. Skill files ───────────────────────────────────────────────────
    {
        // Repo skills
        let repo_skills = context::skills::generate_repo_skills(&cfg, &tera)
            .context("failed to generate repo skills")?;

        for (repo_name, content) in &repo_skills {
            let dir = claude_dir.join("skills").join(repo_name);
            fs::create_dir_all(&dir)
                .with_context(|| format!("failed to create directory {}", dir.display()))?;

            let target = dir.join("SKILL.md");
            write_file(
                &target,
                content,
                &root,
                force,
                &mut generated_count,
                &mut updated_count,
            )?;
        }

        // Workflow skills
        let workflow_skills = context::skills::generate_workflow_skills(&cfg, &tera)
            .context("failed to generate workflow skills")?;

        for (skill_name, content) in &workflow_skills {
            let dir = claude_dir.join("skills").join(skill_name);
            fs::create_dir_all(&dir)
                .with_context(|| format!("failed to create directory {}", dir.display()))?;

            let target = dir.join("SKILL.md");
            write_file(
                &target,
                content,
                &root,
                force,
                &mut generated_count,
                &mut updated_count,
            )?;
        }
    }

    // ── 4. settings.json ─────────────────────────────────────────────────
    {
        let content =
            context::settings::generate(&cfg).context("failed to generate settings.json")?;
        let target = claude_dir.join("settings.json");

        fs::create_dir_all(&claude_dir)
            .with_context(|| format!("failed to create directory {}", claude_dir.display()))?;

        if target.is_file() && !force {
            let existing_raw = fs::read_to_string(&target)
                .with_context(|| format!("failed to read {}", target.display()))?;

            let merged = merge_settings_json(&existing_raw, &content)?;
            fs::write(&target, &merged)
                .with_context(|| format!("failed to write {}", target.display()))?;
            ui::success(&format!("Updated {}", relative_display(&target, &root)));
            updated_count += 1;
        } else {
            fs::write(&target, &content)
                .with_context(|| format!("failed to write {}", target.display()))?;
            ui::success(&format!("Created {}", relative_display(&target, &root)));
            generated_count += 1;
        }
    }

    // ── 5. Hookify rules ─────────────────────────────────────────────────
    {
        let rules = context::hookify::generate_all(&cfg, &tera)
            .context("failed to generate hookify rules")?;

        let hookify_dir = claude_dir.join("hookify");
        fs::create_dir_all(&hookify_dir)
            .with_context(|| format!("failed to create directory {}", hookify_dir.display()))?;

        for (filename, content) in &rules {
            let target = hookify_dir.join(filename);
            write_file(
                &target,
                content,
                &root,
                force,
                &mut generated_count,
                &mut updated_count,
            )?;
        }
    }

    // ── Summary ──────────────────────────────────────────────────────────
    ui::header("Summary");
    ui::info(&format!(
        "{} files created, {} files updated",
        generated_count, updated_count
    ));

    Ok(())
}

/// Write a file to disk, respecting the force flag. For non-CLAUDE.md files
/// we simply overwrite when content differs (no marker-based merge needed).
fn write_file(
    target: &PathBuf,
    content: &str,
    root: &PathBuf,
    force: bool,
    generated_count: &mut u32,
    updated_count: &mut u32,
) -> Result<()> {
    if target.is_file() && !force {
        let existing = fs::read_to_string(target)
            .with_context(|| format!("failed to read {}", target.display()))?;
        if existing == content {
            ui::skip(&format!("Unchanged {}", relative_display(target, root)));
            return Ok(());
        }
        fs::write(target, content)
            .with_context(|| format!("failed to write {}", target.display()))?;
        ui::success(&format!("Updated {}", relative_display(target, root)));
        *updated_count += 1;
    } else {
        fs::write(target, content)
            .with_context(|| format!("failed to write {}", target.display()))?;
        if target.is_file() && force {
            ui::success(&format!("Overwritten {}", relative_display(target, root)));
        } else {
            ui::success(&format!("Created {}", relative_display(target, root)));
        }
        *generated_count += 1;
    }
    Ok(())
}

/// Merge settings JSON: preserve existing keys but update additionalDirectories.
fn merge_settings_json(existing_raw: &str, generated_raw: &str) -> Result<String> {
    let mut existing: serde_json::Value =
        serde_json::from_str(existing_raw).context("failed to parse existing settings.json")?;
    let generated: serde_json::Value =
        serde_json::from_str(generated_raw).context("failed to parse generated settings.json")?;

    // Update additionalDirectories from generated into existing.
    if let Some(dirs) = generated.get("additionalDirectories") {
        existing["additionalDirectories"] = dirs.clone();
    }

    let merged = serde_json::to_string_pretty(&existing)?;
    Ok(merged)
}

/// Display a path relative to the workspace root for compact output.
fn relative_display(path: &PathBuf, root: &PathBuf) -> String {
    path.strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}
