use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::config;
use crate::context;
use crate::ui;

pub fn run() -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;
    let tera = context::create_tera()?;

    let claude_dir = root.join(".claude");

    let mut new_count: u32 = 0;
    let mut changed_count: u32 = 0;
    let mut unchanged_count: u32 = 0;

    ui::header("Context diff");

    // ── 1. CLAUDE.md ─────────────────────────────────────────────────────
    {
        let content =
            context::claude_md::generate(&cfg, &tera).context("failed to generate CLAUDE.md")?;
        let target = root.join("CLAUDE.md");
        diff_file(
            &target,
            &content,
            &root,
            &mut new_count,
            &mut changed_count,
            &mut unchanged_count,
        )?;
    }

    // ── 2. Agent files ───────────────────────────────────────────────────
    {
        let agents =
            context::agents::generate_all(&cfg, &tera).context("failed to generate agent files")?;

        for (repo_name, content) in &agents {
            let target = claude_dir.join("agents").join(repo_name).join("AGENT.md");
            diff_file(
                &target,
                content,
                &root,
                &mut new_count,
                &mut changed_count,
                &mut unchanged_count,
            )?;
        }
    }

    // ── 3. Skill files ───────────────────────────────────────────────────
    {
        let repo_skills = context::skills::generate_repo_skills(&cfg, &tera)
            .context("failed to generate repo skills")?;

        for (repo_name, content) in &repo_skills {
            let target = claude_dir.join("skills").join(repo_name).join("SKILL.md");
            diff_file(
                &target,
                content,
                &root,
                &mut new_count,
                &mut changed_count,
                &mut unchanged_count,
            )?;
        }

        let workflow_skills = context::skills::generate_workflow_skills(&cfg, &tera)
            .context("failed to generate workflow skills")?;

        for (skill_name, content) in &workflow_skills {
            let target = claude_dir.join("skills").join(skill_name).join("SKILL.md");
            diff_file(
                &target,
                content,
                &root,
                &mut new_count,
                &mut changed_count,
                &mut unchanged_count,
            )?;
        }
    }

    // ── 4. settings.json ─────────────────────────────────────────────────
    {
        let content =
            context::settings::generate(&cfg).context("failed to generate settings.json")?;
        let target = claude_dir.join("settings.json");
        diff_file(
            &target,
            &content,
            &root,
            &mut new_count,
            &mut changed_count,
            &mut unchanged_count,
        )?;
    }

    // ── 5. Hookify rules ─────────────────────────────────────────────────
    {
        let rules = context::hookify::generate_all(&cfg, &tera)
            .context("failed to generate hookify rules")?;

        for (filename, content) in &rules {
            let target = claude_dir.join("hookify").join(filename);
            diff_file(
                &target,
                content,
                &root,
                &mut new_count,
                &mut changed_count,
                &mut unchanged_count,
            )?;
        }
    }

    // ── Summary ──────────────────────────────────────────────────────────
    ui::header("Summary");
    ui::info(&format!(
        "{} files would change, {} new, {} unchanged",
        changed_count, new_count, unchanged_count
    ));

    Ok(())
}

/// Compare a generated file against what exists on disk.
fn diff_file(
    target: &PathBuf,
    generated: &str,
    root: &PathBuf,
    new_count: &mut u32,
    changed_count: &mut u32,
    unchanged_count: &mut u32,
) -> Result<()> {
    let rel = relative_display(target, root);

    if !target.is_file() {
        ui::info(&format!("+ New file: {}", rel));
        *new_count += 1;
        return Ok(());
    }

    let existing = fs::read_to_string(target)
        .with_context(|| format!("failed to read {}", target.display()))?;

    if existing == generated {
        *unchanged_count += 1;
    } else {
        ui::warn(&format!("~ Changed: {}", rel));
        *changed_count += 1;
    }

    Ok(())
}

/// Display a path relative to the workspace root for compact output.
fn relative_display(path: &PathBuf, root: &PathBuf) -> String {
    path.strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}
