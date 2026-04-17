use std::fs;
use std::path::{Path, PathBuf};

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
    {
        let generated = context::claude_md::generate(&cfg, &tera)
            .context("failed to generate CLAUDE.md")?;
        files.push(FileEntry::new_merge(
            root.join("CLAUDE.md"),
            &root,
            &generated,
        ));
    }

    // ── 2. Agent files ───────────────────────────────────────────────────
    {
        let agents =
            context::agents::generate_all(&cfg, &tera).context("failed to generate agent files")?;

        let mut names: Vec<&String> = agents.keys().collect();
        names.sort();
        for repo_name in names {
            let target = claude_dir.join("agents").join(repo_name).join("AGENT.md");
            let generated = &agents[repo_name];
            files.push(FileEntry::new_exact(target, &root, generated));
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
            let generated = &repo_skills[repo_name];
            files.push(FileEntry::new_exact(target, &root, generated));
        }

        let workflow_skills = context::skills::generate_workflow_skills(&cfg, &tera)
            .context("failed to generate workflow skills")?;

        let mut names: Vec<&String> = workflow_skills.keys().collect();
        names.sort();
        for skill_name in names {
            let target = claude_dir.join("skills").join(skill_name).join("SKILL.md");
            let generated = &workflow_skills[skill_name];
            files.push(FileEntry::new_exact(target, &root, generated));
        }
    }

    // ── 4. settings.json ─────────────────────────────────────────────────
    {
        let generated =
            context::settings::generate(&cfg).context("failed to generate settings.json")?;
        files.push(FileEntry::new_json(
            claude_dir.join("settings.json"),
            &root,
            &generated,
        ));
    }

    // ── 5. Hookify rules ─────────────────────────────────────────────────
    {
        let rules = context::hookify::generate_all(&cfg, &tera)
            .context("failed to generate hookify rules")?;

        let mut filenames: Vec<&String> = rules.keys().collect();
        filenames.sort();
        for filename in filenames {
            let target = claude_dir.join("hookify").join(filename);
            let generated = &rules[filename];
            files.push(FileEntry::new_exact(target, &root, generated));
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
    /// For files that use marker-based merging (CLAUDE.md). The file is
    /// "exists/clean" if `merge(generated, existing) == existing`, meaning
    /// kimono would produce exactly what is already on disk. Otherwise it is
    /// "customized" — the user edited a generated section or added content
    /// that `merge` would not reproduce.
    fn new_merge(abs_path: PathBuf, root: &Path, generated: &str) -> Self {
        let rel_path = relative_display(&abs_path, root);

        let status = if !abs_path.is_file() {
            FileStatus::Missing
        } else {
            match fs::read_to_string(&abs_path) {
                Ok(existing) => {
                    let merged = context::preserve::merge(generated, &existing);
                    if merged == existing {
                        FileStatus::Exists
                    } else {
                        FileStatus::Customized
                    }
                }
                Err(_) => FileStatus::Missing,
            }
        };

        FileEntry { rel_path, status }
    }

    /// For files that are fully overwritten by generate (no marker-based
    /// merge). "Exists" if on-disk content equals generated content byte-for-
    /// byte; "customized" otherwise.
    fn new_exact(abs_path: PathBuf, root: &Path, generated: &str) -> Self {
        let rel_path = relative_display(&abs_path, root);

        let status = if !abs_path.is_file() {
            FileStatus::Missing
        } else {
            match fs::read_to_string(&abs_path) {
                Ok(existing) => {
                    if existing == generated {
                        FileStatus::Exists
                    } else {
                        FileStatus::Customized
                    }
                }
                Err(_) => FileStatus::Missing,
            }
        };

        FileEntry { rel_path, status }
    }

    /// For settings.json — compare as parsed JSON so whitespace/ordering
    /// differences don't mark the file as customized.
    fn new_json(abs_path: PathBuf, root: &Path, generated: &str) -> Self {
        let rel_path = relative_display(&abs_path, root);

        let status = if !abs_path.is_file() {
            FileStatus::Missing
        } else {
            match fs::read_to_string(&abs_path) {
                Ok(existing) => match json_equivalent(&existing, generated) {
                    Ok(true) => FileStatus::Exists,
                    Ok(false) => FileStatus::Customized,
                    Err(_) => FileStatus::Customized,
                },
                Err(_) => FileStatus::Missing,
            }
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

fn relative_display(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

/// Compare two JSON strings for semantic equality.
fn json_equivalent(a: &str, b: &str) -> Result<bool> {
    let av: serde_json::Value = serde_json::from_str(a).context("failed to parse JSON (a)")?;
    let bv: serde_json::Value = serde_json::from_str(b).context("failed to parse JSON (b)")?;
    Ok(av == bv)
}
