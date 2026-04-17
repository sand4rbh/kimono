use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use console::style;

use crate::config;
use crate::context;
use crate::ui;

pub fn run() -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;
    let tera = context::create_tera()?;

    let claude_dir = root.join(".claude");

    let mut files: Vec<FileEntry> = Vec::new();

    // ── 1. CLAUDE.md ─────────────────────────────────────────────────────
    files.push(FileEntry::new(root.join("CLAUDE.md"), &root));

    // ── 2. Agent files ───────────────────────────────────────────────────
    {
        let agents =
            context::agents::generate_all(&cfg, &tera).context("failed to generate agent files")?;

        let mut names: Vec<&String> = agents.keys().collect();
        names.sort();
        for repo_name in names {
            let target = claude_dir.join("agents").join(repo_name).join("AGENT.md");
            files.push(FileEntry::new(target, &root));
        }
    }

    // ── 3. Skill files ───────────────────────────────────────────────────
    {
        let repo_skills = context::skills::generate_repo_skills(&cfg, &tera)
            .context("failed to generate repo skills")?;

        let mut names: Vec<&String> = repo_skills.keys().collect();
        names.sort();
        for repo_name in names {
            let target = claude_dir.join("skills").join(repo_name).join("SKILL.md");
            files.push(FileEntry::new(target, &root));
        }

        let workflow_skills = context::skills::generate_workflow_skills(&cfg, &tera)
            .context("failed to generate workflow skills")?;

        let mut names: Vec<&String> = workflow_skills.keys().collect();
        names.sort();
        for skill_name in names {
            let target = claude_dir.join("skills").join(skill_name).join("SKILL.md");
            files.push(FileEntry::new(target, &root));
        }
    }

    // ── 4. settings.json ─────────────────────────────────────────────────
    files.push(FileEntry::new(claude_dir.join("settings.json"), &root));

    // ── 5. Hookify rules ─────────────────────────────────────────────────
    {
        let rules = context::hookify::generate_all(&cfg, &tera)
            .context("failed to generate hookify rules")?;

        let mut filenames: Vec<&String> = rules.keys().collect();
        filenames.sort();
        for filename in filenames {
            let target = claude_dir.join("hookify").join(filename);
            files.push(FileEntry::new(target, &root));
        }
    }

    // ── Display ──────────────────────────────────────────────────────────
    ui::header("Context files");

    let headers = &["File", "Status"];
    let rows: Vec<Vec<String>> = files
        .iter()
        .map(|f| vec![f.rel_path.clone(), f.status_display()])
        .collect();

    ui::table(headers, &rows);

    // ── Counts ───────────────────────────────────────────────────────────
    let total = files.len();
    let exists_count = files
        .iter()
        .filter(|f| matches!(f.status, FileStatus::Exists))
        .count();
    let customized_count = files
        .iter()
        .filter(|f| matches!(f.status, FileStatus::Customized))
        .count();
    let missing_count = files
        .iter()
        .filter(|f| matches!(f.status, FileStatus::Missing))
        .count();

    ui::info(&format!(
        "{} total: {} exist, {} customized, {} missing",
        total, exists_count, customized_count, missing_count
    ));

    Ok(())
}

#[derive(Debug)]
enum FileStatus {
    Exists,
    Missing,
    Customized,
}

#[derive(Debug)]
struct FileEntry {
    rel_path: String,
    status: FileStatus,
}

impl FileEntry {
    fn new(abs_path: PathBuf, root: &PathBuf) -> Self {
        let rel_path = abs_path
            .strip_prefix(root)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| abs_path.display().to_string());

        let status = if !abs_path.is_file() {
            FileStatus::Missing
        } else if has_custom_content(&abs_path) {
            FileStatus::Customized
        } else {
            FileStatus::Exists
        };

        FileEntry { rel_path, status }
    }

    fn status_display(&self) -> String {
        match self.status {
            FileStatus::Exists => style("exists").green().to_string(),
            FileStatus::Missing => style("missing").red().to_string(),
            FileStatus::Customized => style("customized").yellow().to_string(),
        }
    }
}

/// Check whether a file has content outside `<!-- kimono:start/end -->` markers,
/// indicating user customization.
fn has_custom_content(path: &PathBuf) -> bool {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let blocks = crate::context::preserve::extract_custom_content(&content);
    !blocks.is_empty()
}
