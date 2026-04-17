//! End-to-end test: replays a full kimono workspace lifecycle against
//! three local bare git repos, verifying clone, status, worktree management,
//! context generation, and idempotent regeneration.
//!
//! The test creates bare repos on the local filesystem (no network), points
//! a `.kimono/config.yml` at them, and drives the compiled kimono binary via
//! `std::process::Command`. Cargo sets `CARGO_BIN_EXE_kimono` automatically
//! when building integration tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use tempfile::TempDir;

const KIMONO: &str = env!("CARGO_BIN_EXE_kimono");

/// Create a bare git repo at `bare_path` and push a single "README.md"
/// commit on `main` into it. Returns the path to the bare repo.
fn make_bare_repo(root: &Path, name: &str) -> PathBuf {
    let bare_path = root.join("remotes").join(format!("{}.git", name));
    fs::create_dir_all(bare_path.parent().unwrap()).unwrap();

    // git init --bare
    let out = Command::new("git")
        .arg("init")
        .arg("--bare")
        .arg("-b")
        .arg("main")
        .arg(&bare_path)
        .output()
        .expect("git init --bare failed");
    assert!(
        out.status.success(),
        "git init --bare: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Create a seed repo and push an initial commit to the bare repo.
    let seed_path = root.join("seeds").join(name);
    fs::create_dir_all(&seed_path).unwrap();

    git_run(&seed_path, &["init", "-q", "-b", "main"]);
    fs::write(seed_path.join("README.md"), format!("# {}\n", name)).unwrap();
    git_run(&seed_path, &["add", "README.md"]);
    git_run_envs(
        &seed_path,
        &["commit", "-q", "-m", "initial"],
        &[
            ("GIT_AUTHOR_NAME", "test"),
            ("GIT_AUTHOR_EMAIL", "test@example.com"),
            ("GIT_COMMITTER_NAME", "test"),
            ("GIT_COMMITTER_EMAIL", "test@example.com"),
        ],
    );
    git_run(
        &seed_path,
        &[
            "push",
            "-q",
            bare_path.to_str().unwrap(),
            "main",
        ],
    );

    bare_path
}

fn git_run(cwd: &Path, args: &[&str]) {
    let out = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .output()
        .expect("git command failed");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

fn git_run_envs(cwd: &Path, args: &[&str], envs: &[(&str, &str)]) {
    let mut cmd = Command::new("git");
    cmd.current_dir(cwd).args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("git command failed");
    assert!(
        out.status.success(),
        "git {:?} failed: {}",
        args,
        String::from_utf8_lossy(&out.stderr)
    );
}

fn run_kimono(workspace: &Path, args: &[&str]) -> Output {
    Command::new(KIMONO)
        .current_dir(workspace)
        .args(args)
        .output()
        .expect("failed to run kimono")
}

fn assert_success(out: &Output, label: &str) {
    if !out.status.success() {
        panic!(
            "kimono {} failed\nstdout: {}\nstderr: {}",
            label,
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
    }
}

/// Combine stdout and stderr into a single string for `contains` checks.
/// Kimono writes UI messages to stderr, so we merge both streams for the
/// test assertions.
fn combined_output(out: &Output) -> String {
    let mut s = String::new();
    s.push_str(&String::from_utf8_lossy(&out.stdout));
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    s
}

#[test]
fn test_full_lifecycle() {
    let tmp = TempDir::new().expect("failed to create temp dir");
    let root = tmp.path().to_path_buf();

    // Create three bare repos with initial commits on `main`.
    let backend_remote = make_bare_repo(&root, "backend");
    let frontend_remote = make_bare_repo(&root, "frontend");
    let mobile_remote = make_bare_repo(&root, "mobile");

    // Create the workspace directory and seed a .kimono/config.yml.
    let workspace = root.join("workspace");
    fs::create_dir_all(workspace.join(".kimono")).unwrap();

    let config = format!(
        r#"workspace:
  name: e2e-test
repos:
  backend:
    remote: {}
    branch: main
  frontend:
    remote: {}
    branch: main
    depends_on: [backend]
  mobile:
    remote: {}
    branch: main
    depends_on: [backend]
"#,
        backend_remote.display(),
        frontend_remote.display(),
        mobile_remote.display(),
    );
    fs::write(workspace.join(".kimono").join("config.yml"), config).unwrap();

    // ── kimono clone ─────────────────────────────────────────────────────
    let out = run_kimono(&workspace, &["clone"]);
    assert_success(&out, "clone");
    assert!(workspace.join("apps").join("backend").is_dir());
    assert!(workspace.join("apps").join("frontend").is_dir());
    assert!(workspace.join("apps").join("mobile").is_dir());
    let co = combined_output(&out);
    assert!(co.contains("backend"), "clone output missing backend");
    assert!(co.contains("frontend"), "clone output missing frontend");
    assert!(co.contains("mobile"), "clone output missing mobile");

    // ── kimono status ────────────────────────────────────────────────────
    let out = run_kimono(&workspace, &["status"]);
    assert_success(&out, "status");
    let co = combined_output(&out);
    assert!(co.contains("backend"), "status missing backend");
    assert!(co.contains("frontend"), "status missing frontend");
    assert!(co.contains("mobile"), "status missing mobile");
    assert!(co.contains("e2e-test"), "status missing workspace name");

    // ── kimono wt feature payments backend frontend --new ────────────────
    let out = run_kimono(
        &workspace,
        &["wt", "feature", "payments", "backend", "frontend", "--new"],
    );
    assert_success(&out, "wt feature --new");
    assert!(
        workspace.join(".worktrees").join("backend--payments").is_dir(),
        "backend--payments worktree should exist"
    );
    assert!(
        workspace
            .join(".worktrees")
            .join("frontend--payments")
            .is_dir(),
        "frontend--payments worktree should exist"
    );
    assert!(
        !workspace.join(".worktrees").join("mobile--payments").is_dir(),
        "mobile--payments should NOT exist (not requested)"
    );

    // ── kimono wt list ──────────────────────────────────────────────────
    let out = run_kimono(&workspace, &["wt", "list"]);
    assert_success(&out, "wt list");
    let co = combined_output(&out);
    assert!(co.contains("backend--payments"), "wt list missing backend");
    assert!(co.contains("frontend--payments"), "wt list missing frontend");
    assert!(
        co.contains("Features"),
        "wt list should show Features section for multi-repo branch"
    );

    // ── kimono context generate (first run) ──────────────────────────────
    let out = run_kimono(&workspace, &["context", "generate"]);
    assert_success(&out, "context generate (first)");
    let co = combined_output(&out);
    assert!(co.contains("Created CLAUDE.md"), "should create CLAUDE.md");
    assert!(
        co.contains("Created .claude/settings.json"),
        "should create settings.json"
    );

    // Assert all expected generated files exist.
    let expected_files = [
        "CLAUDE.md",
        ".claude/agents/backend/AGENT.md",
        ".claude/agents/frontend/AGENT.md",
        ".claude/agents/mobile/AGENT.md",
        ".claude/skills/backend/SKILL.md",
        ".claude/skills/frontend/SKILL.md",
        ".claude/skills/mobile/SKILL.md",
        ".claude/skills/commit/SKILL.md",
        ".claude/skills/create-pr/SKILL.md",
        ".claude/skills/update-pr/SKILL.md",
        ".claude/skills/pr-review-comments/SKILL.md",
        ".claude/settings.json",
        ".claude/hookify/hookify.no-direct-apps-commit.local.md",
        ".claude/hookify/hookify.no-direct-apps-checkout.local.md",
    ];
    for f in &expected_files {
        assert!(
            workspace.join(f).is_file(),
            "expected generated file missing: {}",
            f
        );
    }

    // ── kimono context generate (second run — should all be Unchanged) ───
    let out = run_kimono(&workspace, &["context", "generate"]);
    assert_success(&out, "context generate (second)");
    let co = combined_output(&out);
    assert!(
        !co.contains("Created"),
        "second generate should not Create any file: {}",
        co
    );
    assert!(
        !co.contains("Updated"),
        "second generate should not Update any file (bug-2 regression): {}",
        co
    );
    assert!(
        co.contains("Unchanged"),
        "second generate should report Unchanged files: {}",
        co
    );
    assert!(
        co.contains("0 files created, 0 files updated"),
        "summary should show all zeros on no-op regenerate: {}",
        co
    );

    // ── kimono context show (all should be "exists") ─────────────────────
    let out = run_kimono(&workspace, &["context", "show"]);
    assert_success(&out, "context show");
    let co = combined_output(&out);
    assert!(
        co.contains("14 total: 14 exist, 0 customized, 0 missing"),
        "show should report all files exist, got: {}",
        co
    );

    // ── Add custom section to CLAUDE.md — show should flag customized ───
    let claude_md_path = workspace.join("CLAUDE.md");
    let mut claude_content = fs::read_to_string(&claude_md_path).unwrap();
    let custom_marker = "## My Custom Section\nThis is user content that must survive.\n";
    claude_content.push_str("\n\n");
    claude_content.push_str(custom_marker);
    fs::write(&claude_md_path, &claude_content).unwrap();

    // show BEFORE regenerating — CLAUDE.md should be flagged as customized,
    // all others "exists".
    let out = run_kimono(&workspace, &["context", "show"]);
    assert_success(&out, "context show (after custom edit, pre-regenerate)");
    let co = combined_output(&out);
    assert!(
        co.contains("1 customized"),
        "show should flag 1 customized file after CLAUDE.md edit, got: {}",
        co
    );
    // Find the CLAUDE.md row and verify it is "customized".
    let claude_line = co
        .lines()
        .find(|l| l.contains("CLAUDE.md"))
        .expect("no CLAUDE.md line in show output");
    assert!(
        claude_line.contains("customized"),
        "CLAUDE.md should be customized, line: {}",
        claude_line
    );
    // Others should still be "exists" (count 13).
    assert!(
        co.contains("13 exist"),
        "13 files should still be 'exists', got: {}",
        co
    );

    // Regenerate — custom content must survive.
    let out = run_kimono(&workspace, &["context", "generate"]);
    assert_success(&out, "context generate (after custom edit)");
    let after = fs::read_to_string(&claude_md_path).unwrap();
    assert!(
        after.contains("My Custom Section"),
        "custom section should be preserved across regenerate"
    );
    assert!(
        after.contains("This is user content that must survive."),
        "custom content should be preserved"
    );

    // ── kimono wt feature payments --remove ──────────────────────────────
    let out = run_kimono(
        &workspace,
        &["wt", "feature", "payments", "backend", "frontend", "--remove"],
    );
    assert_success(&out, "wt feature --remove");
    assert!(
        !workspace.join(".worktrees").join("backend--payments").is_dir(),
        "backend--payments worktree should be removed"
    );
    assert!(
        !workspace
            .join(".worktrees")
            .join("frontend--payments")
            .is_dir(),
        "frontend--payments worktree should be removed"
    );

    // ── kimono wt list after removal ────────────────────────────────────
    let out = run_kimono(&workspace, &["wt", "list"]);
    assert_success(&out, "wt list (post-remove)");
    let co = combined_output(&out);
    assert!(
        !co.contains("backend--payments"),
        "wt list should no longer show backend--payments"
    );
    assert!(
        !co.contains("frontend--payments"),
        "wt list should no longer show frontend--payments"
    );
}
