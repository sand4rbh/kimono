use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Message functions — all print to stderr
// ---------------------------------------------------------------------------

/// Print a success message with green checkmark prefix.
pub fn success(msg: &str) {
    eprintln!("  {} {}", style("✓").green(), style(msg).green());
}

/// Print a warning message with yellow warning prefix.
pub fn warn(msg: &str) {
    eprintln!("  {} {}", style("⚠").yellow(), style(msg).yellow());
}

/// Print an error message with red X prefix.
pub fn error(msg: &str) {
    eprintln!("  {} {}", style("✗").red(), style(msg).red());
}

/// Print a skip message in dim/gray.
pub fn skip(msg: &str) {
    eprintln!("  {} {}", style("·").dim(), style(msg).dim());
}

/// Print an info message (neutral).
pub fn info(msg: &str) {
    eprintln!("  {msg}");
}

/// Print a section header in bold.
pub fn header(msg: &str) {
    eprintln!("\n{}", style(msg).bold());
}

// ---------------------------------------------------------------------------
// Status formatting
// ---------------------------------------------------------------------------

/// Format a single repo's status line for the status dashboard.
///
/// Returns a formatted string (does not print). When `stash_count > 0`, a
/// trailing `N stashed` annotation is appended.
pub fn format_repo_status(
    name: &str,
    branch: &str,
    is_clean: bool,
    ahead: u32,
    behind: u32,
    worktree_count: usize,
    stash_count: u32,
) -> String {
    let status_text = if is_clean {
        style("clean".to_string()).green().to_string()
    } else {
        style("modified".to_string()).yellow().to_string()
    };

    let sync_text = if ahead == 0 && behind == 0 {
        style("✓ up to date".to_string()).green().to_string()
    } else {
        let mut parts = Vec::new();
        if ahead > 0 {
            parts.push(style(format!("↑ {} ahead", ahead)).yellow().to_string());
        }
        if behind > 0 {
            parts.push(style(format!("↓ {} behind", behind)).red().to_string());
        }
        parts.join(" ")
    };

    let wt_label = if worktree_count == 1 {
        "worktree"
    } else {
        "worktrees"
    };

    let stash_suffix = if stash_count > 0 {
        format!("  {}", style(format!("{} stashed", stash_count)).cyan())
    } else {
        String::new()
    };

    format!(
        "  {:<16}{:<12}{:<14}  {:<18}{} {}{}",
        style(name).bold(),
        branch,
        status_text,
        sync_text,
        worktree_count,
        wt_label,
        stash_suffix,
    )
}

// ---------------------------------------------------------------------------
// Summary
// ---------------------------------------------------------------------------

/// Print an operation summary (e.g., after clone, sync).
pub fn summary(total: u32, succeeded: u32, skipped: u32, failed: u32) {
    let line = format!(
        "  Done: {} succeeded, {} skipped, {} failed ({} total)",
        succeeded, skipped, failed, total
    );

    if failed > 0 {
        eprintln!("{}", style(line).red());
    } else if skipped > 0 {
        eprintln!("{}", style(line).yellow());
    } else {
        eprintln!("{}", style(line).green());
    }
}

// ---------------------------------------------------------------------------
// Spinner
// ---------------------------------------------------------------------------

/// Create a spinner for long-running operations (clone, fetch, etc.).
///
/// Returns the `ProgressBar` so the caller can `.finish_with_message()` or
/// `.finish_and_clear()`.
pub fn spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"])
            .template("  {spinner} {msg}")
            .expect("invalid spinner template"),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

// ---------------------------------------------------------------------------
// Table helper
// ---------------------------------------------------------------------------

/// Print a simple aligned table to stderr.
///
/// `headers` — column header names.
/// `rows` — each row is a vector of cell strings.
///
/// Column widths are derived from the widest content in each column
/// (including the header). At least 2 spaces separate columns.
pub fn table(headers: &[&str], rows: &[Vec<String>]) {
    if headers.is_empty() {
        return;
    }

    let col_count = headers.len();
    let gap = 2;

    // Compute max width per column.
    let mut widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < col_count {
                let stripped = console::strip_ansi_codes(cell);
                if stripped.len() > widths[i] {
                    widths[i] = stripped.len();
                }
            }
        }
    }

    // Print header row.
    let header_line: String = headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            let padded = format!("{:width$}", h, width = widths[i]);
            style(padded).bold().to_string()
        })
        .collect::<Vec<_>>()
        .join(&" ".repeat(gap));
    eprintln!("  {header_line}");

    // Print data rows.
    for row in rows {
        let cells: Vec<String> = (0..col_count)
            .map(|i| {
                let cell = row.get(i).map(|s| s.as_str()).unwrap_or("");
                let visible_len = console::strip_ansi_codes(cell).len();
                let padding = widths[i].saturating_sub(visible_len);
                format!("{}{}", cell, " ".repeat(padding))
            })
            .collect();
        let line = cells.join(&" ".repeat(gap));
        eprintln!("  {line}");
    }
}

// ---------------------------------------------------------------------------
// Worktree list formatting
// ---------------------------------------------------------------------------

/// Format a worktree entry for the `wt list` command.
///
/// Returns a formatted string.
pub fn format_worktree_entry(name: &str, branch: &str, status: &str) -> String {
    format!("  {:<32}{:<20}{}", style(name).bold(), branch, status)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_repo_status_clean() {
        let output = format_repo_status("backend", "main", true, 0, 0, 2, 0);
        let plain = console::strip_ansi_codes(&output).to_string();

        assert!(plain.contains("backend"), "should contain the repo name");
        assert!(plain.contains("main"), "should contain the branch");
        assert!(plain.contains("clean"), "should say clean");
        assert!(
            plain.contains("up to date"),
            "should say up to date when ahead=0 and behind=0"
        );
        assert!(plain.contains("2 worktrees"), "should list worktree count");
        assert!(
            !plain.contains("stashed"),
            "should omit stash annotation when stash_count=0"
        );
    }

    #[test]
    fn test_format_repo_status_dirty() {
        let output = format_repo_status("frontend", "develop", false, 1, 3, 1, 0);
        let plain = console::strip_ansi_codes(&output).to_string();

        assert!(plain.contains("frontend"), "should contain the repo name");
        assert!(plain.contains("develop"), "should contain the branch");
        assert!(
            plain.contains("modified"),
            "should say modified for dirty repo"
        );
        assert!(plain.contains("1 ahead"), "should report ahead count");
        assert!(plain.contains("3 behind"), "should report behind count");
        assert!(
            plain.contains("1 worktree"),
            "should list worktree count (singular)"
        );
    }

    #[test]
    fn test_format_repo_status_with_stashes() {
        let output = format_repo_status("backend", "main", true, 0, 0, 0, 2);
        let plain = console::strip_ansi_codes(&output).to_string();

        assert!(
            plain.contains("2 stashed"),
            "should show stash count when stash_count > 0"
        );
    }

    #[test]
    fn test_format_worktree_entry() {
        let output =
            format_worktree_entry("backend--feature-payments", "feature-payments", "3 ahead");
        let plain = console::strip_ansi_codes(&output).to_string();

        assert!(
            plain.contains("backend--feature-payments"),
            "should contain worktree name"
        );
        assert!(
            plain.contains("feature-payments"),
            "should contain branch name"
        );
        assert!(plain.contains("3 ahead"), "should contain status");
    }

    #[test]
    fn test_summary_no_failures() {
        // Should not panic; output goes to stderr.
        summary(4, 3, 1, 0);
    }

    #[test]
    fn test_spinner_creation() {
        let pb = spinner("testing spinner");
        pb.finish_and_clear();
        // No panic means success.
    }
}
