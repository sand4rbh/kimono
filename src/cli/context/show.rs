use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use console::style;

use crate::config;
use crate::ui;

pub fn run() -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;

    let installed = cfg
        .plugins
        .as_ref()
        .map(|p| p.installed.clone())
        .unwrap_or_default();

    ui::header(&format!(
        "Workspace: {} ({} repos)",
        cfg.workspace.name,
        cfg.repos.len()
    ));

    // Installed skills
    println!("\nInstalled skills:");
    if installed.is_empty() {
        println!("  (none)");
    } else {
        let mut names: Vec<_> = installed.keys().collect();
        names.sort();
        for name in names {
            println!("  {:<18} {}", name, installed[name]);
        }
    }

    // Context files
    println!("\nContext files:");
    let claude_md = root.join("CLAUDE.md");
    print_status(&claude_md, &root, "CLAUDE.md");

    let agents_dir = root.join(".claude").join("agents");
    let mut agent_files: Vec<std::path::PathBuf> = Vec::new();
    if agents_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&agents_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) == Some("md") {
                    agent_files.push(p);
                }
            }
        }
    }
    agent_files.sort();
    for path in &agent_files {
        let rel = path
            .strip_prefix(&root)
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| path.display().to_string());
        println!("  {} {}", style("✓ present").green(), rel);
    }

    let settings_json = root.join(".claude").join("settings.json");
    print_status(&settings_json, &root, ".claude/settings.json");

    // Hints
    let hints = hints(&claude_md, &settings_json, &agent_files, &installed);
    if !hints.is_empty() {
        println!("\nHints:");
        for h in hints {
            println!("  - {h}");
        }
    }

    Ok(())
}

fn print_status(path: &Path, root: &Path, label: &str) {
    let rel = label.to_string();
    let _ = root;
    if path.is_file() {
        println!("  {} {}", style("✓ present").green(), rel);
    } else {
        println!("  {} {}", style("✗ missing").red(), rel);
    }
}

fn hints(
    claude_md: &Path,
    settings_json: &Path,
    agent_files: &[std::path::PathBuf],
    installed: &HashMap<String, String>,
) -> Vec<String> {
    let mut out = Vec::new();
    if !claude_md.is_file() {
        out.push("CLAUDE.md not found — run `kimono bootstrap`".to_string());
    }
    if !settings_json.is_file() {
        out.push(".claude/settings.json not found — run `kimono bootstrap`".to_string());
    }
    if agent_files.is_empty() {
        out.push("No agent files in .claude/agents/ — run `kimono bootstrap`".to_string());
    }
    if !installed.contains_key("discover") {
        out.push(
            "discover skill not installed — run `kimono plugins install discover` for deeper context"
                .to_string(),
        );
    }
    if installed.contains_key("hookify-rules") {
        let hookify_dir = claude_md
            .parent()
            .unwrap_or(Path::new("."))
            .join(".claude/hookify");
        if !hookify_dir.is_dir() {
            out.push(
                "hookify-rules installed but .claude/hookify/ empty — run discover or invoke the skill manually".to_string(),
            );
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_hints_missing_claude_md() {
        let tmp = TempDir::new().unwrap();
        let claude_md = tmp.path().join("CLAUDE.md");
        let settings = tmp.path().join(".claude/settings.json");
        let installed = HashMap::new();
        let agent_files: Vec<std::path::PathBuf> = Vec::new();
        let h = hints(&claude_md, &settings, &agent_files, &installed);
        assert!(h.iter().any(|s| s.contains("CLAUDE.md not found")));
        assert!(h.iter().any(|s| s.contains("settings.json not found")));
        assert!(h.iter().any(|s| s.contains("No agent files")));
        assert!(h.iter().any(|s| s.contains("discover skill not installed")));
    }

    #[test]
    fn test_hints_no_warnings_when_present() {
        let tmp = TempDir::new().unwrap();
        let claude_md = tmp.path().join("CLAUDE.md");
        std::fs::write(&claude_md, "# workspace").unwrap();
        std::fs::create_dir_all(tmp.path().join(".claude")).unwrap();
        let settings = tmp.path().join(".claude/settings.json");
        std::fs::write(&settings, "{}").unwrap();
        let agent_files = vec![tmp.path().join("agent.md")];
        let mut installed = HashMap::new();
        installed.insert("discover".to_string(), "1.0.0".to_string());
        let h = hints(&claude_md, &settings, &agent_files, &installed);
        assert!(h.is_empty(), "expected no hints, got {:?}", h);
    }
}
