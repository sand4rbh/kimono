# Kimono v2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Pivot kimono from a Rust-and-Tera context generator to a thin workspace/git/worktree manager plus an installable skill catalog. Delete v1's context-generation Rust code and templates; add a plugin system that fetches skills from this same repo's `skills/` directory; author 7 default skills.

**Architecture:** The Rust binary shrinks to workspace + git + worktree operations plus a small plugin client. Context generation and workflow commands move into Claude skills authored as markdown files under `skills/<name>/SKILL.md`. The plugin client shallow-clones the kimono repo to `~/.cache/kimono/registry/` and reads `skills/index.json` to enumerate skills. Installing a skill copies it into the workspace's `.claude/skills/<name>/` and records its version in `.kimono/config.yml > plugins.installed`. Two CLI shortcuts (`kimono bootstrap`, `kimono discover`) shell out to `claude -p "…"` to run the corresponding skill non-interactively.

**Tech Stack:** Rust (edition 2021), `clap` (derive macros), `serde` + `serde_yaml`, `serde_json`, `anyhow`, `dialoguer`, `console`, `indicatif`, `which`, `dirs`, `tempfile` (dev). `tera` is removed.

**Reference spec:** [`docs/specs/20260421-v2-spec.md`](../specs/20260421-v2-spec.md).

---

## File Structure

### Files deleted

```
src/context/                          # entire directory
  ├── mod.rs
  ├── agents.rs
  ├── claude_md.rs
  ├── hookify.rs
  ├── preserve.rs
  ├── settings.rs
  └── skills.rs
src/cli/context/generate.rs
src/cli/context/diff.rs
src/cli/commit.rs
src/cli/create_pr.rs
src/cli/update_pr.rs
src/cli/pr_review_comments.rs
src/github/                           # entire directory (mod.rs)
templates/                            # entire directory (11 .tera files)
```

### Files created (Rust)

```
src/plugins/mod.rs                    # plugin system entry; re-exports
src/plugins/registry.rs               # cache dir, clone/pull, index.json parsing
src/plugins/install.rs                # copy skill into workspace; update config
src/plugins/claude_shell.rs           # shell out to `claude -p "..."`
src/cli/plugins/mod.rs                # plugins subcommand module
src/cli/plugins/list.rs               # kimono plugins list
src/cli/plugins/search.rs             # kimono plugins search
src/cli/plugins/info.rs               # kimono plugins info
src/cli/plugins/install.rs            # kimono plugins install
src/cli/plugins/remove.rs             # kimono plugins remove
src/cli/plugins/update.rs             # kimono plugins update
src/cli/bootstrap.rs                  # kimono bootstrap (shell-out shortcut)
src/cli/discover.rs                   # kimono discover (shell-out shortcut)
```

### Files created (skills + catalog)

```
skills/index.json                     # catalog manifest, 7 skills
skills/bootstrap/SKILL.md
skills/discover/SKILL.md
skills/git-commit/SKILL.md
skills/create-pr/SKILL.md
skills/update-pr/SKILL.md
skills/code-review/SKILL.md
skills/hookify-rules/SKILL.md
```

### Files modified

```
Cargo.toml                            # remove `tera` dependency
src/main.rs                           # drop removed clap commands, add new ones
src/cli/mod.rs                        # mod declarations
src/cli/init.rs                       # rewrite wizard for plugin install + bootstrap shell-out
src/cli/add.rs                        # stop calling context::generate
src/cli/remove.rs                     # stop calling context::generate
src/cli/context/mod.rs                # only re-export `show`
src/cli/context/show.rs               # rewrite as AI-layer status display
src/config/schema.rs                  # drop ClaudeConfig, add PluginsConfig
tests/fixtures/config.yml             # update example: drop claude, add plugins
tests/fixtures/config_minimal.yml     # (unchanged or trivial)
tests/e2e_test.rs                     # update to test v2 flow
CLAUDE.md                             # v2 model description
README.md                             # v2 model description
```

### Naming conventions (must match across tasks)

- Config struct names: `KimonoConfig`, `Workspace`, `Repo`, `PluginsConfig` (new). `ClaudeConfig` is deleted.
- Plugin module functions (used across tasks):
  - `crate::plugins::registry::cache_dir() -> PathBuf`
  - `crate::plugins::registry::ensure_cache(url: &str) -> Result<PathBuf>` — clones or pulls
  - `crate::plugins::registry::load_index(cache: &Path) -> Result<Index>`
  - `crate::plugins::registry::DEFAULT_REGISTRY_URL: &str = "https://github.com/sand4rbh/kimono"`
  - `crate::plugins::install::install(name: &str, version: Option<&str>) -> Result<()>`
  - `crate::plugins::install::remove(name: &str) -> Result<()>`
  - `crate::plugins::install::update(names: &[String]) -> Result<()>`
  - `crate::plugins::claude_shell::is_available() -> bool`
  - `crate::plugins::claude_shell::run_skill(skill_name: &str) -> Result<()>`
- Index types (in `registry.rs`):
  - `pub struct Index { pub schema_version: u32, pub skills: Vec<SkillEntry> }`
  - `pub struct SkillEntry { pub name: String, pub description: String, pub version: String, pub tags: Vec<String>, pub path: String, pub default: Option<bool> }`

---

## Task 1: Delete v1 context generation paths

Strip the v1 template-and-context infrastructure. After this task the binary compiles, runs, but `kimono context generate/diff` and the four workflow commands no longer exist. `kimono context show` is stubbed (rewrite comes in Task 9).

**Files:**
- Delete: `src/context/` (whole directory), `src/cli/context/generate.rs`, `src/cli/context/diff.rs`, `src/cli/commit.rs`, `src/cli/create_pr.rs`, `src/cli/update_pr.rs`, `src/cli/pr_review_comments.rs`, `src/github/` (whole directory), `templates/` (whole directory)
- Modify: `Cargo.toml`, `src/main.rs`, `src/cli/mod.rs`, `src/cli/context/mod.rs`, `src/cli/context/show.rs`, `src/cli/add.rs`, `src/cli/remove.rs`

- [ ] **Step 1: Delete directories and files**

```bash
rm -r src/context
rm src/cli/context/generate.rs src/cli/context/diff.rs
rm src/cli/commit.rs src/cli/create_pr.rs src/cli/update_pr.rs src/cli/pr_review_comments.rs
rm -r src/github
rm -r templates
```

- [ ] **Step 2: Remove `tera` from `Cargo.toml`**

Edit `Cargo.toml`, delete the line `tera = "1"`.

- [ ] **Step 3: Strip context/github mod declarations and removed clap subcommands from `src/main.rs`**

Open `src/main.rs`. Remove these top-level `mod` declarations: `mod context;`, `mod github;`.

In the `Commands` enum, delete these variants: `Commit`, `CreatePr`, `UpdatePr`, `PrReviewComments`.

In the `ContextCommands` enum, delete `Generate { force: bool }` and `Diff`. Keep only `Show`.

In the `match cli.command { ... }` block in `fn main()`, delete the arms for `Commands::Commit`, `Commands::CreatePr`, `Commands::UpdatePr`, `Commands::PrReviewComments`. In the `Commands::Context { command } => match command { ... }` block, delete the `ContextCommands::Generate` and `ContextCommands::Diff` arms.

- [ ] **Step 4: Strip removed modules from `src/cli/mod.rs`**

Open `src/cli/mod.rs`. Delete these lines:

```rust
pub mod commit;
pub mod create_pr;
pub mod pr_review_comments;
pub mod update_pr;
```

- [ ] **Step 5: Stub out `src/cli/context/mod.rs` to only expose `show`**

Replace the contents of `src/cli/context/mod.rs` with:

```rust
pub mod show;
```

- [ ] **Step 6: Stub `src/cli/context/show.rs` with a placeholder that compiles**

This temporary stub keeps the binary buildable. Task 9 rewrites it. Replace the contents of `src/cli/context/show.rs` with:

```rust
use anyhow::Result;

pub fn run() -> Result<()> {
    println!("(context show is being rewritten — see docs/plans/20260514-v2-implementation-plan.md Task 9)");
    Ok(())
}
```

- [ ] **Step 7: Stop calling context::generate from `src/cli/add.rs` and `src/cli/remove.rs`**

Open `src/cli/add.rs`. Find any call site like `crate::cli::context::generate::run(...)?;` and delete that statement (and any surrounding ui message about regenerating context). Do the same in `src/cli/remove.rs`. If a `use crate::cli::context;` import becomes unused, remove it. If a function becomes empty / trivially returns, that's fine.

- [ ] **Step 8: Run cargo build to confirm it compiles**

Run: `cargo build`
Expected: builds successfully. If there are unused-import warnings about `tera` or `context` or `github`, fix them by removing the imports.

- [ ] **Step 9: Run cargo test to confirm tests still pass**

Run: `cargo test`
Expected: most tests pass. Some tests under `tests/e2e_test.rs` referencing `context generate` will fail — that's expected and will be fixed in Task 12. For now, run with the existing E2E tests skipped:

Run: `cargo test --lib`
Expected: all unit tests pass.

- [ ] **Step 10: Commit**

```bash
git add -A
git commit -m "$(cat <<'EOF'
refactor: delete v1 context-generation paths

Removes src/context/, templates/, src/github/, the four workflow CLI
commands (commit, create-pr, update-pr, pr-review-comments), and the
context generate/diff subcommands. Stubs context/show.rs and removes
the tera dependency.

Part of the v2 pivot — these paths are replaced by installable skills
(see docs/specs/20260421-v2-spec.md).
EOF
)"
```

---

## Task 2: Update config schema — drop `claude`, add `plugins`

Replace the v1 `claude:` config section with a `plugins:` section that holds the registry URL and a map of installed skill names → semver strings.

**Files:**
- Modify: `src/config/schema.rs`, `tests/fixtures/config.yml`, `tests/fixtures/config_minimal.yml`
- Test: same file (`schema.rs` has inline `mod tests`)

- [ ] **Step 1: Write failing test for plugins config parsing**

In `src/config/schema.rs`'s `mod tests`, add this test (before the final closing `}` of the `mod tests`):

```rust
#[test]
fn test_parse_plugins_config() {
    let yaml = r#"
workspace:
  name: test-project
repos:
  svc:
    remote: git@github.com:me/svc.git
plugins:
  registry: github.com/myorg/my-registry
  installed:
    bootstrap: 1.0.0
    discover: 1.0.0
"#;
    let config: KimonoConfig = serde_yaml::from_str(yaml).expect("failed to parse");

    let plugins = config.plugins.as_ref().expect("missing plugins config");
    assert_eq!(
        plugins.registry.as_deref(),
        Some("github.com/myorg/my-registry")
    );
    assert_eq!(plugins.installed.get("bootstrap").unwrap(), "1.0.0");
    assert_eq!(plugins.installed.get("discover").unwrap(), "1.0.0");
}

#[test]
fn test_plugins_optional() {
    let yaml = r#"
workspace:
  name: test-project
repos:
  svc:
    remote: git@github.com:me/svc.git
"#;
    let config: KimonoConfig = serde_yaml::from_str(yaml).expect("failed to parse");
    assert!(config.plugins.is_none());
}
```

- [ ] **Step 2: Run the new tests, confirm they fail**

Run: `cargo test --lib test_parse_plugins_config test_plugins_optional`
Expected: FAIL — `KimonoConfig` has no `plugins` field, `ClaudeConfig` still present.

- [ ] **Step 3: Replace the schema**

Replace the contents of `src/config/schema.rs` with:

```rust
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_apps_dir() -> String {
    "apps".to_string()
}

fn default_worktree_dir() -> String {
    ".worktrees".to_string()
}

fn default_branch() -> String {
    "main".to_string()
}

/// Top-level kimono configuration parsed from .kimono/config.yml
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KimonoConfig {
    pub workspace: Workspace,
    pub repos: HashMap<String, Repo>,
    #[serde(default)]
    pub plugins: Option<PluginsConfig>,
}

/// Workspace-level settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub name: String,
    #[serde(default = "default_apps_dir")]
    pub apps_dir: String,
    #[serde(default = "default_worktree_dir")]
    pub worktree_dir: String,
}

/// Configuration for a single repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Repo {
    pub remote: String,
    #[serde(default = "default_branch")]
    pub branch: String,
    pub description: Option<String>,
    #[serde(default)]
    pub tech: Vec<String>,
    pub package_manager: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub commands: HashMap<String, String>,
}

/// Plugin (skill) configuration. Defaults are implicit: `registry` falls
/// back to the binary's hardcoded URL; `installed` defaults to empty.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginsConfig {
    pub registry: Option<String>,
    #[serde(default)]
    pub installed: HashMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_full_config() {
        let yaml = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/config.yml"
        ))
        .expect("failed to read full config fixture");
        let config: KimonoConfig =
            serde_yaml::from_str(&yaml).expect("failed to parse full config");

        assert_eq!(config.workspace.name, "my-platform");
        assert_eq!(config.workspace.apps_dir, "apps");
        assert_eq!(config.workspace.worktree_dir, ".worktrees");

        assert_eq!(config.repos.len(), 3);

        let backend = config.repos.get("backend").expect("missing backend");
        assert_eq!(backend.remote, "git@github.com:myorg/backend.git");
        assert_eq!(backend.branch, "main");
        assert_eq!(backend.description.as_deref(), Some("NestJS GraphQL API"));
        assert_eq!(
            backend.tech,
            vec!["nestjs", "typescript", "graphql", "postgresql"]
        );
        assert_eq!(backend.package_manager.as_deref(), Some("yarn"));
        assert!(backend.depends_on.is_empty());
        assert_eq!(backend.commands.get("dev").unwrap(), "yarn dev");

        let frontend = config.repos.get("frontend").expect("missing frontend");
        assert_eq!(frontend.depends_on, vec!["backend"]);

        // Plugins section
        let plugins = config.plugins.as_ref().expect("missing plugins config");
        assert_eq!(plugins.installed.get("bootstrap").unwrap(), "1.0.0");
    }

    #[test]
    fn test_parse_minimal_config() {
        let yaml = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/config_minimal.yml"
        ))
        .expect("failed to read minimal config fixture");
        let config: KimonoConfig =
            serde_yaml::from_str(&yaml).expect("failed to parse minimal config");

        assert_eq!(config.workspace.name, "my-project");
        assert_eq!(config.repos.len(), 1);
        assert!(config.plugins.is_none());
    }

    #[test]
    fn test_required_fields_error() {
        let yaml = r#"
workspace:
  apps_dir: apps
repos:
  api:
    remote: git@github.com:me/api.git
"#;
        let result: Result<KimonoConfig, _> = serde_yaml::from_str(yaml);
        assert!(result.is_err(), "should error when workspace.name is missing");
    }

    #[test]
    fn test_defaults() {
        let yaml = r#"
workspace:
  name: test-project
repos:
  svc:
    remote: git@github.com:me/svc.git
"#;
        let config: KimonoConfig = serde_yaml::from_str(yaml).expect("failed to parse");

        assert_eq!(config.workspace.apps_dir, "apps");
        assert_eq!(config.workspace.worktree_dir, ".worktrees");

        let svc = config.repos.get("svc").unwrap();
        assert_eq!(svc.branch, "main");
    }

    #[test]
    fn test_parse_plugins_config() {
        let yaml = r#"
workspace:
  name: test-project
repos:
  svc:
    remote: git@github.com:me/svc.git
plugins:
  registry: github.com/myorg/my-registry
  installed:
    bootstrap: 1.0.0
    discover: 1.0.0
"#;
        let config: KimonoConfig = serde_yaml::from_str(yaml).expect("failed to parse");

        let plugins = config.plugins.as_ref().expect("missing plugins config");
        assert_eq!(
            plugins.registry.as_deref(),
            Some("github.com/myorg/my-registry")
        );
        assert_eq!(plugins.installed.get("bootstrap").unwrap(), "1.0.0");
        assert_eq!(plugins.installed.get("discover").unwrap(), "1.0.0");
    }

    #[test]
    fn test_plugins_optional() {
        let yaml = r#"
workspace:
  name: test-project
repos:
  svc:
    remote: git@github.com:me/svc.git
"#;
        let config: KimonoConfig = serde_yaml::from_str(yaml).expect("failed to parse");
        assert!(config.plugins.is_none());
    }
}
```

- [ ] **Step 4: Update the full-config fixture to have a `plugins` section instead of `claude`**

Open `tests/fixtures/config.yml`. Delete the `claude:` block at the bottom. Append a `plugins:` block:

```yaml
plugins:
  installed:
    bootstrap: 1.0.0
```

- [ ] **Step 5: Run all schema tests**

Run: `cargo test --lib config::schema`
Expected: all 6 tests pass.

- [ ] **Step 6: Run the whole unit-test suite**

Run: `cargo test --lib`
Expected: all unit tests pass. If `src/config/mod.rs` references `ClaudeConfig` anywhere, remove those references.

- [ ] **Step 7: Commit**

```bash
git add src/config/schema.rs tests/fixtures/config.yml tests/fixtures/config_minimal.yml
git commit -m "$(cat <<'EOF'
feat(config): replace `claude` config section with `plugins`

Adds PluginsConfig struct holding optional registry URL and installed
skill versions map. Drops ClaudeConfig and its associated default fns.
EOF
)"
```

---

## Task 3: Registry foundation — cache dir, clone/pull, index.json parsing

Create the `plugins` module with the registry client. The client locates the cache directory under `~/.cache/kimono/registry/`, shallow-clones or `git pull`s the registry repo, and parses `skills/index.json`.

**Files:**
- Create: `src/plugins/mod.rs`, `src/plugins/registry.rs`
- Modify: `src/main.rs` (add `mod plugins;`)

- [ ] **Step 1: Create `src/plugins/mod.rs`**

```rust
pub mod claude_shell;
pub mod install;
pub mod registry;
```

(We declare all three submodules up front so subsequent tasks just fill them in — but for Task 3 they need to be empty stubs.)

- [ ] **Step 2: Create stub `src/plugins/install.rs` and `src/plugins/claude_shell.rs`**

```rust
// src/plugins/install.rs
// Filled in by Task 4.
```

```rust
// src/plugins/claude_shell.rs
// Filled in by Task 5.
```

- [ ] **Step 3: Add `mod plugins;` to `src/main.rs`**

Open `src/main.rs`. After the existing `mod` declarations near the top, add:

```rust
mod plugins;
```

- [ ] **Step 4: Write failing test for `index.json` parsing in `src/plugins/registry.rs`**

Create `src/plugins/registry.rs` with this content (test only — types and impls in next step will make it compile):

```rust
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

pub const DEFAULT_REGISTRY_URL: &str = "https://github.com/sand4rbh/kimono";

#[derive(Debug, Clone, Deserialize)]
pub struct Index {
    pub schema_version: u32,
    pub skills: Vec<SkillEntry>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SkillEntry {
    pub name: String,
    pub description: String,
    pub version: String,
    #[serde(default)]
    pub tags: Vec<String>,
    pub path: String,
    #[serde(default)]
    pub default: Option<bool>,
}

/// Filesystem location of the registry cache.
/// `$XDG_CACHE_HOME/kimono/registry/` on Linux, or platform equivalent.
pub fn cache_dir() -> Result<PathBuf> {
    let base = dirs::cache_dir().context("could not determine user cache dir")?;
    Ok(base.join("kimono").join("registry"))
}

/// Parse a registry's `skills/index.json` from a cache root.
pub fn load_index(cache: &Path) -> Result<Index> {
    let path = cache.join("skills").join("index.json");
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    let index: Index = serde_json::from_str(&raw)
        .with_context(|| format!("failed to parse {}", path.display()))?;
    Ok(index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_index_parses_seven_skills() {
        let dir = TempDir::new().unwrap();
        let skills_dir = dir.path().join("skills");
        std::fs::create_dir_all(&skills_dir).unwrap();
        let json = r#"{
          "schema_version": 1,
          "skills": [
            {"name": "bootstrap", "description": "init", "version": "1.0.0", "tags": ["init"], "path": "skills/bootstrap", "default": true},
            {"name": "discover", "description": "scan", "version": "1.0.0", "tags": ["context"], "path": "skills/discover"}
          ]
        }"#;
        std::fs::write(skills_dir.join("index.json"), json).unwrap();

        let idx = load_index(dir.path()).unwrap();
        assert_eq!(idx.schema_version, 1);
        assert_eq!(idx.skills.len(), 2);
        assert_eq!(idx.skills[0].name, "bootstrap");
        assert_eq!(idx.skills[0].default, Some(true));
        assert_eq!(idx.skills[1].default, None);
    }

    #[test]
    fn test_load_index_missing_file_errors() {
        let dir = TempDir::new().unwrap();
        let result = load_index(dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_dir_under_user_cache() {
        let dir = cache_dir().unwrap();
        let dir_str = dir.display().to_string();
        assert!(dir_str.contains("kimono"), "{dir_str}");
        assert!(dir_str.ends_with("registry"), "{dir_str}");
    }
}
```

- [ ] **Step 5: Run the new tests, confirm they pass**

Run: `cargo test --lib plugins::registry`
Expected: all 3 tests pass.

- [ ] **Step 6: Add `ensure_cache` (clone-or-pull)**

In `src/plugins/registry.rs`, after the `load_index` function, add:

```rust
/// Ensure the registry cache exists and is up to date.
///
/// If the cache directory does not contain a git repo, shallow-clone `url`
/// into it. Otherwise, `git pull --ff-only` to refresh.
pub fn ensure_cache(url: &str) -> Result<PathBuf> {
    let cache = cache_dir()?;
    if let Some(parent) = cache.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let git_dir = cache.join(".git");
    if git_dir.is_dir() {
        // Existing cache — pull
        let status = std::process::Command::new("git")
            .args(["-C", cache.to_str().unwrap(), "pull", "--ff-only", "--quiet"])
            .status()
            .with_context(|| format!("failed to run `git pull` in {}", cache.display()))?;
        if !status.success() {
            anyhow::bail!("`git pull` failed for registry cache at {}", cache.display());
        }
    } else {
        // Fresh clone
        if cache.exists() {
            // Non-git directory exists — remove it so clone can run
            std::fs::remove_dir_all(&cache)
                .with_context(|| format!("failed to remove stale cache {}", cache.display()))?;
        }
        let status = std::process::Command::new("git")
            .args([
                "clone",
                "--depth",
                "1",
                url,
                cache.to_str().unwrap(),
            ])
            .status()
            .context("failed to run `git clone` for registry")?;
        if !status.success() {
            anyhow::bail!("`git clone` failed for registry URL {}", url);
        }
    }
    Ok(cache)
}
```

- [ ] **Step 7: Run cargo build to ensure ensure_cache compiles**

Run: `cargo build`
Expected: builds successfully.

- [ ] **Step 8: Commit**

```bash
git add src/plugins/ src/main.rs
git commit -m "$(cat <<'EOF'
feat(plugins): add registry module — cache dir, clone/pull, index parsing

Introduces src/plugins/{mod,registry,install,claude_shell}.rs. registry.rs
provides cache_dir(), ensure_cache(url), load_index(cache), plus Index +
SkillEntry serde types and the DEFAULT_REGISTRY_URL constant. install.rs
and claude_shell.rs are empty stubs filled in by later tasks.
EOF
)"
```

---

## Task 4: Install / remove / update operations on installed skills

Implement the operations that copy skill directories from the cache into the workspace's `.claude/skills/<name>/` and keep `.kimono/config.yml > plugins.installed` in sync.

**Files:**
- Modify: `src/plugins/install.rs`
- Test: same file (inline `mod tests`)

- [ ] **Step 1: Write failing test for install**

Replace the contents of `src/plugins/install.rs` with:

```rust
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config;
use crate::config::{KimonoConfig, PluginsConfig};
use crate::plugins::registry;

/// Install a skill from the cached registry into the workspace's
/// `.claude/skills/<name>/` and record the version in
/// `.kimono/config.yml > plugins.installed`.
///
/// If `version` is None, installs the version listed in the cache's
/// `index.json`. If `version` is Some, the function still installs the
/// version present in the cache (it does not support multiple versions
/// in v2) and errors if `version` doesn't match.
pub fn install(name: &str, version: Option<&str>) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;

    let registry_url = cfg
        .plugins
        .as_ref()
        .and_then(|p| p.registry.as_deref())
        .unwrap_or(registry::DEFAULT_REGISTRY_URL);

    let cache = registry::ensure_cache(registry_url)?;
    let index = registry::load_index(&cache)?;

    let entry = index
        .skills
        .iter()
        .find(|s| s.name == name)
        .with_context(|| format!("skill '{name}' not found in registry"))?;

    if let Some(requested) = version {
        if requested != entry.version {
            anyhow::bail!(
                "skill '{name}' is version {} in the registry; requested version {} not available",
                entry.version,
                requested
            );
        }
    }

    let src = cache.join(&entry.path);
    if !src.is_dir() {
        anyhow::bail!(
            "registry layout error: index entry points to {} but that directory does not exist",
            src.display()
        );
    }

    let dest = root.join(".claude").join("skills").join(name);
    if dest.is_dir() {
        std::fs::remove_dir_all(&dest)
            .with_context(|| format!("failed to remove existing {}", dest.display()))?;
    }
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    copy_dir_recursive(&src, &dest)?;

    record_installed(&root, &cfg, name, &entry.version)?;
    Ok(())
}

/// Remove a skill: delete `.claude/skills/<name>/` and the config entry.
pub fn remove(name: &str) -> Result<()> {
    let root = config::workspace_root()?;
    let cfg = config::load_and_validate()?;

    let skill_dir = root.join(".claude").join("skills").join(name);
    if skill_dir.is_dir() {
        std::fs::remove_dir_all(&skill_dir)
            .with_context(|| format!("failed to remove {}", skill_dir.display()))?;
    }

    erase_installed(&root, &cfg, name)?;
    Ok(())
}

/// Update one or more installed skills: re-run install if the registry's
/// version is higher than the installed version. Empty `names` updates all
/// installed skills.
pub fn update(names: &[String]) -> Result<()> {
    let cfg = config::load_and_validate()?;
    let installed = cfg
        .plugins
        .as_ref()
        .map(|p| p.installed.clone())
        .unwrap_or_default();

    let targets: Vec<String> = if names.is_empty() {
        installed.keys().cloned().collect()
    } else {
        for n in names {
            if !installed.contains_key(n) {
                anyhow::bail!("skill '{n}' is not installed");
            }
        }
        names.to_vec()
    };

    let registry_url = cfg
        .plugins
        .as_ref()
        .and_then(|p| p.registry.as_deref())
        .unwrap_or(registry::DEFAULT_REGISTRY_URL);
    let cache = registry::ensure_cache(registry_url)?;
    let index = registry::load_index(&cache)?;

    for name in &targets {
        if let Some(entry) = index.skills.iter().find(|s| s.name == *name) {
            let current = installed.get(name).cloned().unwrap_or_default();
            if entry.version != current {
                install(name, None)?;
            }
        }
    }
    Ok(())
}

fn record_installed(root: &Path, cfg: &KimonoConfig, name: &str, version: &str) -> Result<()> {
    let mut new_cfg = cfg.clone();
    let plugins = new_cfg.plugins.get_or_insert_with(PluginsConfig::default);
    plugins.installed.insert(name.to_string(), version.to_string());
    write_config(root, &new_cfg)
}

fn erase_installed(root: &Path, cfg: &KimonoConfig, name: &str) -> Result<()> {
    let mut new_cfg = cfg.clone();
    if let Some(plugins) = new_cfg.plugins.as_mut() {
        plugins.installed.remove(name);
    }
    write_config(root, &new_cfg)
}

fn write_config(root: &Path, cfg: &KimonoConfig) -> Result<()> {
    let path = root.join(".kimono").join("config.yml");
    let yaml = serde_yaml::to_string(cfg).context("failed to serialize config")?;
    std::fs::write(&path, yaml).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest)
        .with_context(|| format!("failed to create {}", dest.display()))?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else if file_type.is_file() {
            std::fs::copy(&src_path, &dest_path).with_context(|| {
                format!(
                    "failed to copy {} -> {}",
                    src_path.display(),
                    dest_path.display()
                )
            })?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, content: &str) {
        if let Some(p) = path.parent() {
            std::fs::create_dir_all(p).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    fn make_cache(root: &Path) -> PathBuf {
        let cache = root.join("cache");
        let skills = cache.join("skills");
        std::fs::create_dir_all(&skills).unwrap();
        write(
            &skills.join("index.json"),
            r#"{
              "schema_version": 1,
              "skills": [
                {"name":"bootstrap","description":"init","version":"1.0.0","tags":[],"path":"skills/bootstrap","default":true},
                {"name":"discover","description":"deep","version":"1.0.0","tags":[],"path":"skills/discover"}
              ]
            }"#,
        );
        write(&skills.join("bootstrap").join("SKILL.md"), "# bootstrap\n");
        write(&skills.join("discover").join("SKILL.md"), "# discover\n");
        cache
    }

    #[test]
    fn test_copy_dir_recursive_preserves_structure() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cache = make_cache(tmp.path());
        let dest = tmp.path().join("installed");
        copy_dir_recursive(&cache.join("skills").join("bootstrap"), &dest).unwrap();
        assert!(dest.join("SKILL.md").is_file());
    }
}
```

- [ ] **Step 2: Run the new test**

Run: `cargo test --lib plugins::install::tests::test_copy_dir_recursive_preserves_structure`
Expected: PASS.

- [ ] **Step 3: Verify the whole module compiles**

Run: `cargo build`
Expected: builds successfully.

The integration-level tests for `install`/`remove`/`update` need a real workspace + registry — those run via the E2E test in Task 12. The unit test for `copy_dir_recursive` is enough here.

- [ ] **Step 4: Commit**

```bash
git add src/plugins/install.rs
git commit -m "$(cat <<'EOF'
feat(plugins): add install, remove, and update operations

install() copies a skill from the cached registry into
.claude/skills/<name>/ and records the version in the config.
remove() deletes the directory and config entry. update() re-runs
install when the registry has a higher version. Bumps to the
.kimono/config.yml file are written via serde_yaml round-trip.
EOF
)"
```

---

## Task 5: claude_shell — shell out to `claude -p "…"` to run a skill

Implements `is_available()` (checks `PATH`) and `run_skill(skill_name)` (spawns `claude -p` with the appropriate prompt). Used by `kimono init`, `kimono bootstrap`, and `kimono discover`.

**Files:**
- Modify: `src/plugins/claude_shell.rs`

- [ ] **Step 1: Write the module**

Replace the contents of `src/plugins/claude_shell.rs` with:

```rust
use anyhow::{Context, Result};

/// Returns true if `claude` is on PATH.
pub fn is_available() -> bool {
    which::which("claude").is_ok()
}

/// Shell out to `claude -p "Run the kimono <name> skill for this workspace"`
/// non-interactively. Stdout/stderr stream to the parent terminal.
///
/// Returns an error if `claude` is not available, or if the invocation
/// exits non-zero.
pub fn run_skill(skill_name: &str) -> Result<()> {
    if !is_available() {
        anyhow::bail!(
            "`claude` CLI not found on PATH. Install Claude Code from \
             https://claude.com/claude-code and re-run."
        );
    }

    let prompt = format!(
        "Run the kimono {skill_name} skill for this workspace, applied to the current directory."
    );

    let status = std::process::Command::new("claude")
        .arg("-p")
        .arg(&prompt)
        .status()
        .with_context(|| format!("failed to spawn `claude -p` for skill {skill_name}"))?;

    if !status.success() {
        anyhow::bail!(
            "`claude -p` for skill '{skill_name}' exited with status {}",
            status
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_available_does_not_panic() {
        let _ = is_available();
    }
}
```

- [ ] **Step 2: Run cargo build + the smoke test**

Run: `cargo test --lib plugins::claude_shell`
Expected: 1 test passes.

- [ ] **Step 3: Commit**

```bash
git add src/plugins/claude_shell.rs
git commit -m "$(cat <<'EOF'
feat(plugins): add claude_shell — shell-out to `claude -p` for skill runs

is_available() probes PATH via the `which` crate. run_skill(name) spawns
`claude -p "Run the kimono <name> skill for this workspace, applied to
the current directory."` non-interactively, streaming stdout/stderr to
the parent terminal. Used by init, bootstrap, and discover shortcuts.
EOF
)"
```

---

## Task 6: CLI plugins subcommands — list, search, info, install, remove, update

Wire the plugin operations to a `kimono plugins` subcommand tree. Alias: `kimono skills`.

**Files:**
- Create: `src/cli/plugins/mod.rs`, `src/cli/plugins/list.rs`, `src/cli/plugins/search.rs`, `src/cli/plugins/info.rs`, `src/cli/plugins/install.rs`, `src/cli/plugins/remove.rs`, `src/cli/plugins/update.rs`
- Modify: `src/cli/mod.rs`, `src/main.rs`

- [ ] **Step 1: Create `src/cli/plugins/mod.rs`**

```rust
pub mod info;
pub mod install;
pub mod list;
pub mod remove;
pub mod search;
pub mod update;
```

- [ ] **Step 2: Create `src/cli/plugins/list.rs`**

```rust
use anyhow::Result;
use console::style;

use crate::config;
use crate::plugins::registry;
use crate::ui;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    All,
    Installed,
    Available,
}

pub fn run(scope: Scope) -> Result<()> {
    let cfg = config::load_and_validate().ok();
    let installed = cfg
        .as_ref()
        .and_then(|c| c.plugins.as_ref())
        .map(|p| p.installed.clone())
        .unwrap_or_default();

    if scope == Scope::Installed {
        ui::header("Installed skills");
        if installed.is_empty() {
            ui::info("(none)");
            return Ok(());
        }
        let mut names: Vec<_> = installed.keys().collect();
        names.sort();
        for name in names {
            println!("  {} {}", style(name).green(), installed[name]);
        }
        return Ok(());
    }

    let registry_url = cfg
        .as_ref()
        .and_then(|c| c.plugins.as_ref())
        .and_then(|p| p.registry.as_deref())
        .unwrap_or(registry::DEFAULT_REGISTRY_URL);

    let cache = match registry::ensure_cache(registry_url) {
        Ok(c) => c,
        Err(e) => {
            ui::error(&format!(
                "Registry cache empty and `git clone` failed — run with internet to populate.\n  {e}"
            ));
            return Err(e);
        }
    };
    let index = registry::load_index(&cache)?;

    ui::header(&format!("Available skills (registry: {registry_url})"));
    for skill in &index.skills {
        if scope == Scope::Available && installed.contains_key(&skill.name) {
            continue;
        }
        let marker = if installed.contains_key(&skill.name) {
            style("(installed)").dim().to_string()
        } else {
            String::new()
        };
        println!(
            "  {:<18} {:<8} {}  {}",
            style(&skill.name).bold(),
            skill.version,
            marker,
            skill.description
        );
    }
    Ok(())
}
```

- [ ] **Step 3: Create `src/cli/plugins/search.rs`**

```rust
use anyhow::Result;
use console::style;

use crate::config;
use crate::plugins::registry;
use crate::ui;

pub fn run(query: &str) -> Result<()> {
    let cfg = config::load_and_validate().ok();
    let registry_url = cfg
        .as_ref()
        .and_then(|c| c.plugins.as_ref())
        .and_then(|p| p.registry.as_deref())
        .unwrap_or(registry::DEFAULT_REGISTRY_URL);

    let cache = registry::ensure_cache(registry_url)?;
    let index = registry::load_index(&cache)?;

    let q = query.to_lowercase();
    let mut hits = Vec::new();
    for skill in &index.skills {
        if skill.name.to_lowercase().contains(&q)
            || skill.description.to_lowercase().contains(&q)
            || skill.tags.iter().any(|t| t.to_lowercase().contains(&q))
        {
            hits.push(skill);
        }
    }

    ui::header(&format!("Search results for '{query}' ({} hits)", hits.len()));
    for skill in hits {
        println!(
            "  {:<18} {:<8} {}",
            style(&skill.name).bold(),
            skill.version,
            skill.description
        );
    }
    Ok(())
}
```

- [ ] **Step 4: Create `src/cli/plugins/info.rs`**

```rust
use anyhow::{bail, Result};
use console::style;

use crate::config;
use crate::plugins::registry;

pub fn run(name: &str) -> Result<()> {
    let cfg = config::load_and_validate().ok();
    let registry_url = cfg
        .as_ref()
        .and_then(|c| c.plugins.as_ref())
        .and_then(|p| p.registry.as_deref())
        .unwrap_or(registry::DEFAULT_REGISTRY_URL);

    let cache = registry::ensure_cache(registry_url)?;
    let index = registry::load_index(&cache)?;

    let entry = match index.skills.iter().find(|s| s.name == name) {
        Some(e) => e,
        None => bail!("skill '{name}' not found in registry at {registry_url}"),
    };

    println!("{}", style(&entry.name).bold());
    println!("  Version:     {}", entry.version);
    println!("  Tags:        {}", entry.tags.join(", "));
    println!("  Path:        {}", entry.path);
    println!("  Description: {}", entry.description);
    if entry.default.unwrap_or(false) {
        println!("  Default:     yes (auto-installed by `kimono init`)");
    }
    Ok(())
}
```

- [ ] **Step 5: Create `src/cli/plugins/install.rs`**

```rust
use anyhow::Result;

use crate::plugins::install;
use crate::ui;

pub fn run(names: &[String], version: Option<&str>) -> Result<()> {
    if names.is_empty() {
        anyhow::bail!("provide at least one skill name to install");
    }
    for name in names {
        install::install(name, version)?;
        ui::success(&format!("Installed {name}"));
    }
    Ok(())
}
```

- [ ] **Step 6: Create `src/cli/plugins/remove.rs`**

```rust
use anyhow::Result;

use crate::plugins::install;
use crate::ui;

pub fn run(names: &[String]) -> Result<()> {
    if names.is_empty() {
        anyhow::bail!("provide at least one skill name to remove");
    }
    for name in names {
        install::remove(name)?;
        ui::success(&format!("Removed {name}"));
    }
    Ok(())
}
```

- [ ] **Step 7: Create `src/cli/plugins/update.rs`**

```rust
use anyhow::Result;

use crate::plugins::install;
use crate::ui;

pub fn run(names: &[String]) -> Result<()> {
    install::update(names)?;
    if names.is_empty() {
        ui::success("All installed skills checked for updates");
    } else {
        ui::success(&format!("Checked: {}", names.join(", ")));
    }
    Ok(())
}
```

- [ ] **Step 8: Register `plugins` module in `src/cli/mod.rs`**

Open `src/cli/mod.rs`. Among the `pub mod ...;` declarations at the top, add (alphabetical order):

```rust
pub mod plugins;
```

- [ ] **Step 9: Add the `Plugins` clap subcommand to `src/main.rs`**

In `src/main.rs`, after the `Wt` variant in `enum Commands`, add:

```rust
    /// Manage installable Claude skills (alias: `skills`)
    #[command(alias = "skills")]
    Plugins {
        #[command(subcommand)]
        command: PluginsCommands,
    },
```

After the existing `enum WtCommands { ... }` block, add:

```rust
#[derive(Subcommand)]
enum PluginsCommands {
    /// List available or installed skills
    List {
        /// Show only installed skills
        #[arg(long, conflicts_with = "available")]
        installed: bool,
        /// Show only skills not yet installed
        #[arg(long)]
        available: bool,
    },
    /// Search the registry by name, description, or tag
    Search { query: String },
    /// Show details for a single skill
    Info { name: String },
    /// Install one or more skills into this workspace
    Install {
        names: Vec<String>,
        /// Pin to a specific version (must match registry)
        #[arg(long)]
        version: Option<String>,
    },
    /// Remove one or more installed skills
    Remove { names: Vec<String> },
    /// Update installed skills against the registry
    Update { names: Vec<String> },
}
```

In `fn main()`'s `match cli.command { ... }`, add a new arm after the `Wt` arm:

```rust
        Commands::Plugins { command } => match command {
            PluginsCommands::List { installed, available } => {
                let scope = if installed {
                    cli::plugins::list::Scope::Installed
                } else if available {
                    cli::plugins::list::Scope::Available
                } else {
                    cli::plugins::list::Scope::All
                };
                cli::plugins::list::run(scope)
            }
            PluginsCommands::Search { query } => cli::plugins::search::run(&query),
            PluginsCommands::Info { name } => cli::plugins::info::run(&name),
            PluginsCommands::Install { names, version } => {
                cli::plugins::install::run(&names, version.as_deref())
            }
            PluginsCommands::Remove { names } => cli::plugins::remove::run(&names),
            PluginsCommands::Update { names } => cli::plugins::update::run(&names),
        },
```

- [ ] **Step 10: Build and verify the subcommand parses**

Run: `cargo build && ./target/debug/kimono plugins --help`
Expected: clap prints the plugins help text listing `list`, `search`, `info`, `install`, `remove`, `update`.

- [ ] **Step 11: Run unit tests**

Run: `cargo test --lib`
Expected: all pass.

- [ ] **Step 12: Commit**

```bash
git add src/cli/plugins/ src/cli/mod.rs src/main.rs
git commit -m "$(cat <<'EOF'
feat(cli): add `kimono plugins` subcommand tree (alias `skills`)

Six subcommands: list, search, info, install, remove, update. list
supports --installed and --available scoping. install/remove accept
multiple skill names. Wired into main.rs.
EOF
)"
```

---

## Task 7: `kimono bootstrap` and `kimono discover` shortcuts

Thin CLI wrappers that call `crate::plugins::claude_shell::run_skill(...)` with the corresponding skill name.

**Files:**
- Create: `src/cli/bootstrap.rs`, `src/cli/discover.rs`
- Modify: `src/cli/mod.rs`, `src/main.rs`

- [ ] **Step 1: Create `src/cli/bootstrap.rs`**

```rust
use anyhow::Result;

use crate::plugins::claude_shell;
use crate::ui;

pub fn run() -> Result<()> {
    ui::header("Running bootstrap skill via Claude Code");
    claude_shell::run_skill("bootstrap")?;
    ui::success("Bootstrap complete");
    Ok(())
}
```

- [ ] **Step 2: Create `src/cli/discover.rs`**

```rust
use anyhow::Result;

use crate::plugins::claude_shell;
use crate::ui;

pub fn run() -> Result<()> {
    ui::header("Running discover skill via Claude Code");
    claude_shell::run_skill("discover")?;
    ui::success("Discover complete");
    Ok(())
}
```

- [ ] **Step 3: Register both modules in `src/cli/mod.rs`**

Open `src/cli/mod.rs`. Add (alphabetically):

```rust
pub mod bootstrap;
pub mod discover;
```

- [ ] **Step 4: Add `Bootstrap` and `Discover` clap variants to `src/main.rs`**

In `enum Commands`, after `Plugins { ... }`, add:

```rust
    /// Run the bootstrap skill via Claude Code (shell-out)
    Bootstrap,
    /// Run the discover skill via Claude Code (shell-out)
    Discover,
```

In `fn main()`'s match, add:

```rust
        Commands::Bootstrap => cli::bootstrap::run(),
        Commands::Discover => cli::discover::run(),
```

- [ ] **Step 5: Build + verify help**

Run: `cargo build && ./target/debug/kimono --help`
Expected: `bootstrap` and `discover` listed in the top-level subcommands.

- [ ] **Step 6: Commit**

```bash
git add src/cli/bootstrap.rs src/cli/discover.rs src/cli/mod.rs src/main.rs
git commit -m "$(cat <<'EOF'
feat(cli): add `kimono bootstrap` and `kimono discover` shortcuts

Thin wrappers around plugins::claude_shell::run_skill that spawn
`claude -p` non-interactively to run the corresponding skill. Useful
for regeneration without re-running the init wizard.
EOF
)"
```

---

## Task 8: Repurpose `kimono context show` — AI-layer status

Rewrite `src/cli/context/show.rs` to print installed skills, presence/absence of CLAUDE.md and per-agent files, and next-move hints.

**Files:**
- Modify: `src/cli/context/show.rs`

- [ ] **Step 1: Write failing unit test for the hint logic**

Add a helper function `hints(...)` that produces the list of hint strings from inputs. Test it directly so we don't have to set up a filesystem to test hints.

Replace the contents of `src/cli/context/show.rs` with:

```rust
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
    let _ = root; // reserved for future relative-path formatting
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
        let hookify_dir = claude_md.parent().unwrap_or(Path::new(".")).join(".claude/hookify");
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
```

- [ ] **Step 2: Run the new tests**

Run: `cargo test --lib cli::context::show`
Expected: 2 tests pass.

- [ ] **Step 3: Run the binary against a workspace to eyeball output**

Run: `cargo build && cd /tmp && rm -rf show-test && mkdir show-test && cd show-test && /Users/sandarbh/data/code/kimono/target/debug/kimono init demo --bare && /Users/sandarbh/data/code/kimono/target/debug/kimono context show`
Expected: prints workspace name, empty "Installed skills", red "missing" for CLAUDE.md + settings.json, and hints to run `kimono bootstrap`. Then clean up: `cd / && rm -rf /tmp/show-test`.

- [ ] **Step 4: Commit**

```bash
git add src/cli/context/show.rs
git commit -m "$(cat <<'EOF'
feat(cli): rewrite `kimono context show` as AI-layer status

Lists installed skills with versions, shows presence/absence of
CLAUDE.md, settings.json, and per-agent files under .claude/agents/,
and emits actionable hints (run bootstrap, install discover, etc.)
when context is missing. Drops the v1 file-comparison logic entirely.
EOF
)"
```

---

## Task 9: Rewrite the init wizard

Update `src/cli/init.rs` to:
1. Skip the v1 "Claude options" prompts (they're gone).
2. After cloning, offer the default skill bundle (single Y/n).
3. Install `bootstrap` unconditionally; install bundle on confirmation.
4. Prompt to run bootstrap via shell-out if `claude` is available.

**Files:**
- Modify: `src/cli/init.rs`

- [ ] **Step 1: Read the current init.rs to understand structure**

Run: `wc -l src/cli/init.rs`
Then open it and locate (a) the section that handles the `--bare` branch, (b) the section that calls into `context::generate::run`, (c) the wizard's per-prompt block. These are the regions to rewrite. The rest (mode selection, repo prompts, config write) stays.

- [ ] **Step 2: Replace the post-clone hook with the new install + bootstrap flow**

In `src/cli/init.rs`:

1. Delete any code that calls `crate::cli::context::generate::run(...)`. Remove its `use` statement if now unused.
2. After repos have been cloned (`crate::cli::clone::run(&[])?;` or equivalent), insert the new flow:

```rust
    // Required: install bootstrap skill
    ui::info("Installing required skill: bootstrap…");
    if let Err(e) = crate::plugins::install::install("bootstrap", None) {
        ui::warn(&format!(
            "Failed to install bootstrap from the registry ({e}). \
             Run `kimono plugins install bootstrap` once you have network access."
        ));
    } else {
        ui::success("Installed bootstrap");
    }

    // Optional: default skill bundle
    let install_defaults = dialoguer::Confirm::new()
        .with_prompt(
            "Install default skills (discover, git-commit, create-pr, update-pr, code-review, hookify-rules)?",
        )
        .default(true)
        .interact()
        .unwrap_or(true);
    if install_defaults {
        let bundle = [
            "discover",
            "git-commit",
            "create-pr",
            "update-pr",
            "code-review",
            "hookify-rules",
        ];
        for name in bundle {
            match crate::plugins::install::install(name, None) {
                Ok(()) => ui::success(&format!("Installed {name}")),
                Err(e) => ui::warn(&format!("Failed to install {name}: {e}")),
            }
        }
    }

    // Optional: shell out to claude to run bootstrap right now
    if crate::plugins::claude_shell::is_available() {
        let run_now = dialoguer::Confirm::new()
            .with_prompt("Run bootstrap skill now to generate AI context?")
            .default(true)
            .interact()
            .unwrap_or(false);
        if run_now {
            if let Err(e) = crate::plugins::claude_shell::run_skill("bootstrap") {
                ui::warn(&format!("Bootstrap shell-out failed: {e}"));
            }
        } else {
            ui::info("Run `kimono bootstrap` later to generate AI context.");
        }
    } else {
        ui::info(
            "Claude Code CLI not detected on PATH. Run `kimono bootstrap` after installing it.",
        );
    }

    ui::info("Tip: run `kimono discover` later to enrich AI context with deep codebase analysis.");
```

3. In the `--bare` branch (which skips clone), do not run any plugin install — bare workspaces are config-only. Make sure the new install block above is *not* reachable when `bare == true`.

- [ ] **Step 3: Add the missing imports**

At the top of `src/cli/init.rs`, ensure these are imported (add if not present):

```rust
use crate::plugins;
use crate::ui;
```

(`dialoguer` is referenced fully-qualified above so no import is needed; `dialoguer` is already a top-level dependency.)

- [ ] **Step 4: Build to confirm it compiles**

Run: `cargo build`
Expected: builds successfully.

- [ ] **Step 5: Smoke-test the bare path interactively**

Run: `cd /tmp && rm -rf init-bare-test && /Users/sandarbh/data/code/kimono/target/debug/kimono init init-bare-test --bare`
Expected: creates `init-bare-test/` containing `.kimono/config.yml`, no apps, no skill install attempts. Clean up: `cd / && rm -rf /tmp/init-bare-test`.

- [ ] **Step 6: Run unit tests**

Run: `cargo test --lib`
Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add src/cli/init.rs
git commit -m "$(cat <<'EOF'
feat(init): rewrite wizard for v2 — install skills + bootstrap shell-out

After repos are cloned, the wizard installs the required `bootstrap`
skill, offers the default bundle (discover + 4 workflow skills +
hookify-rules) behind a single Y/n, then optionally shells out to
`claude -p` to run the bootstrap skill immediately. Bare workspaces
skip the install phase. The old context::generate call site is removed.
EOF
)"
```

---

## Task 10: Author `skills/index.json` and the 7 SKILL.md files

Write the catalog manifest and the 7 default skill bodies under `skills/`. Each skill is a markdown file with YAML frontmatter following the descriptive (superpowers-style) pattern: explicit "Use when…" trigger, a checklist, anti-patterns.

**Files:**
- Create: `skills/index.json`, `skills/bootstrap/SKILL.md`, `skills/discover/SKILL.md`, `skills/git-commit/SKILL.md`, `skills/create-pr/SKILL.md`, `skills/update-pr/SKILL.md`, `skills/code-review/SKILL.md`, `skills/hookify-rules/SKILL.md`

- [ ] **Step 1: Create `skills/index.json`**

```json
{
  "schema_version": 1,
  "skills": [
    {
      "name": "bootstrap",
      "description": "Initial AI context setup — minimal CLAUDE.md and per-project AGENT.md from config + manifests.",
      "version": "1.0.0",
      "tags": ["init", "context", "setup"],
      "path": "skills/bootstrap",
      "default": true
    },
    {
      "name": "discover",
      "description": "Deep codebase scan that augments CLAUDE.md and AGENT.md with architecture, patterns, and domain vocabulary.",
      "version": "1.0.0",
      "tags": ["context", "analysis"],
      "path": "skills/discover"
    },
    {
      "name": "git-commit",
      "description": "Stage and commit changes in worktrees with contextual messages.",
      "version": "1.0.0",
      "tags": ["git", "workflow"],
      "path": "skills/git-commit"
    },
    {
      "name": "create-pr",
      "description": "Push branch and open a PR via gh, populating title/description from commits and cross-repo context.",
      "version": "1.0.0",
      "tags": ["git", "github", "workflow"],
      "path": "skills/create-pr"
    },
    {
      "name": "update-pr",
      "description": "Push commits and update an existing PR; report check and review state.",
      "version": "1.0.0",
      "tags": ["git", "github", "workflow"],
      "path": "skills/update-pr"
    },
    {
      "name": "code-review",
      "description": "Fetch PR review comments and structure them for the agent to address.",
      "version": "1.0.0",
      "tags": ["github", "workflow"],
      "path": "skills/code-review"
    },
    {
      "name": "hookify-rules",
      "description": "Generate .claude/hookify/ rules that enforce the read-only master pattern. Depends on the hookify plugin.",
      "version": "1.0.0",
      "tags": ["hooks", "enforcement"],
      "path": "skills/hookify-rules"
    }
  ]
}
```

- [ ] **Step 2: Create `skills/bootstrap/SKILL.md`**

````markdown
---
name: bootstrap
description: Use when a kimono workspace needs initial AI context — fresh init, after adding a repo, or when CLAUDE.md / agent files are missing. Writes minimal CLAUDE.md and per-project AGENT.md from config + manifests.
---

# Kimono Bootstrap

You are setting up the initial AI layer for a kimono workspace. Keep it fast and shallow — `discover` is the deep-scan companion.

## Trigger

Invoke when the user says any of: "bootstrap kimono context", "run kimono bootstrap", "set up the AI layer", or when `CLAUDE.md` / files under `.claude/agents/` are missing for a kimono workspace.

## Checklist

1. Read `.kimono/config.yml` from the workspace root. Extract `workspace.name`, `workspace.apps_dir` (default `apps`), `workspace.worktree_dir` (default `.worktrees`), and the `repos` map.
2. For each repo under `<apps_dir>/<name>/`:
   - **Multi-project detection** via file-existence checks: `nx.json`, `turbo.json`, `pnpm-workspace.yaml`, `lerna.json`, top-level `[workspace]` block in `Cargo.toml`, or conventional subdirs (`packages/`, `apps/`, `libs/`) containing manifests.
   - If multi-project, enumerate the projects (one per subdir with a manifest).
   - For each project (or the whole repo if single-project): read its top-level `README.md` and primary manifest only. No deep tree walks.
3. Write `CLAUDE.md` at the workspace root. Required sections:
   - Workspace overview (1 paragraph)
   - Repo / project registry table: name, description, tech, default branch
   - Cross-repo dependencies (from `repos.<name>.depends_on`)
   - Agent dispatch rules: one row per project, pointing at the agent's `name`
   - Worktree conventions: `.worktrees/<repo>--<branch-slug>/`
   - Read-only master guidance: never modify files under `apps/`
4. Write one agent file per project:
   - **Single-project repo**: `.claude/agents/<repo>.md`, frontmatter `name: <repo>`.
   - **Multi-project**: `.claude/agents/<repo>--<project>.md`, frontmatter `name: <repo>--<project>`.
   - Each contains description, tech stack, package manager, commands (dev/lint/test/typecheck if known), working directory note.
5. Write `.claude/settings.json` with `additionalDirectories` covering each `apps/<repo>/`.
6. **Marker preservation**: when rewriting any file, preserve content outside `<!-- kimono:start:... -->` / `<!-- kimono:end:... -->` markers. Replace only the body inside markers.
7. Report a summary: which files were created vs updated, and any project where tech inference was uncertain.

## Anti-patterns

- **Walking source trees beyond top-level manifest reads.** That's `discover`'s job. If you find yourself opening `src/services/*.ts` to figure out what the code does, stop — write what you know and let discover fill the rest.
- **Inventing tech stacks not evidenced by manifests or README.** If `package.json` doesn't list React, don't write "React".
- **Overwriting content outside markers.** User-authored sections must survive a re-run.
- **Skipping multi-project detection.** A repo with `pnpm-workspace.yaml` and three packages must produce three agent files, not one.

## Output expected

A summary report including:
- `Created: CLAUDE.md, .claude/agents/<...>.md, .claude/settings.json`
- `Updated: <list>` (when re-running)
- `Uncertain inference: <project> (reason)`
````

- [ ] **Step 3: Create `skills/discover/SKILL.md`**

````markdown
---
name: discover
description: Use when a kimono workspace needs deeper AI context — after working in the codebase, when bootstrap output feels too thin, or when architecture has evolved. Walks source trees and augments CLAUDE.md / agent files with architecture, patterns, and domain vocabulary.
---

# Kimono Discover

You are enriching AI context with a deep codebase scan. **Bootstrap must have run first.**

## Trigger

Invoke when the user says any of: "run kimono discover", "deep-scan this codebase", "enrich the AI context", "augment CLAUDE.md with architecture details".

## Checklist

1. Read `.kimono/config.yml` plus the existing `CLAUDE.md` and `.claude/agents/<name>.md` files. If `CLAUDE.md` does not exist, stop and tell the user to run `kimono bootstrap` first.
2. For each project listed in the existing `CLAUDE.md` registry:
   - Walk the source tree under `apps/<repo>/<project-path>/`. Identify entry points, module boundaries, public APIs.
   - Detect architectural patterns: DDD, feature folders, event-sourcing, layered, MVC, hexagonal, microservices, etc.
   - Extract domain vocabulary from filenames, type definitions, ADRs in `docs/adr/` or `architecture/decisions/`, and deeper README content.
   - Identify common workflows beyond `package.json` scripts: test runners, env vars, deployment hints, CI config.
   - Map cross-project data flow: shared types, API contracts, message schemas.
   - Surface hotspots from `git log` (most-changed paths). Concretely: `git -C apps/<repo> log --pretty=format:'' --name-only | sort | uniq -c | sort -rn | head -50`.
3. Augment `CLAUDE.md` and per-project `.claude/agents/<name>.md` by writing into **new marker sections**, never overwriting bootstrap sections:
   - `<!-- kimono:start:discover:architecture --> … <!-- kimono:end:discover:architecture -->`
   - `<!-- kimono:start:discover:vocabulary --> … <!-- kimono:end:discover:vocabulary -->`
   - `<!-- kimono:start:discover:hotspots --> … <!-- kimono:end:discover:hotspots -->`
4. Report a per-project summary: what patterns were detected, what's been added, and which areas remain unclear.

## Anti-patterns

- **Overwriting bootstrap output.** Add new marker sections; never edit content inside `<!-- kimono:start:overview -->`, `<!-- kimono:start:registry -->`, etc.
- **Inventing architecture descriptions not evidenced by the source.** If you can't point at a directory or file that shows DDD, don't write "uses Domain-Driven Design".
- **Reading every file.** Sample representative files per directory. A typical project should produce a 100-300 line augmentation, not 5000.
- **Ignoring multi-project structure.** Each sub-project gets its own augmentation in its own agent file.

## Output expected

A per-project summary like:
- `<project> — added: architecture (3 patterns detected), vocabulary (28 terms), hotspots (10 paths)`
- `<project> — unclear: <reason>`
````

- [ ] **Step 4: Create `skills/git-commit/SKILL.md`**

````markdown
---
name: git-commit
description: Use when the user has staged or unstaged changes in a kimono workspace's worktrees and wants them committed with a contextual message. Refuses to commit inside `apps/` (read-only master).
---

# Kimono Git-Commit

## Trigger

Invoke when the user says any of: "commit my changes", "commit the worktree", "make a commit", in a kimono workspace context.

## Checklist

1. List all worktrees: `ls -1 .worktrees/` (or use `kimono wt list`). For each, run `git -C <worktree> status --porcelain`.
2. Refuse to operate on directories under `apps/` — those are read-only masters. If the user explicitly asks to commit in `apps/`, decline and point them to `kimono wt feature <name> --new`.
3. For each worktree with changes (staged or unstaged):
   - If the user supplied `-m <message>`, use it.
   - Otherwise, read the diff (`git -C <worktree> diff --cached` then `git -C <worktree> diff`), and draft a 1-2 sentence Conventional-Commit-style message that captures the *why* of the change.
   - Stage tracked unstaged files (`git -C <worktree> add -u`) unless the user asks for a more selective stage.
   - Commit with the drafted (or user-supplied) message.
4. Report per worktree: `<worktree> ✓ committed (N files)` or `<worktree> — no changes`.

## Anti-patterns

- Committing in `apps/<repo>/` — never do this.
- Force-adding untracked files (`git add .`) — explicit unstaged tracked only unless the user asks otherwise.
- Long commit messages — keep them tight.

## Output expected

```
backend--feature-payments   ✓ committed (3 files): feat: add payment processing endpoint
frontend--feature-payments  ✓ committed (5 files): feat: wire payment form to backend
mobile--feature-payments    — no changes
```
````

- [ ] **Step 5: Create `skills/create-pr/SKILL.md`**

````markdown
---
name: create-pr
description: Use when the user wants to open a pull request from a kimono worktree. Pushes the branch, creates the PR via `gh`, and links cross-repo PRs when the same feature spans multiple worktrees.
---

# Kimono Create-PR

## Trigger

Invoke when the user says any of: "create a PR", "open a pull request", "push and open PRs", in a kimono workspace context.

## Checklist

1. Identify target worktrees (all in `.worktrees/`, or those matching `--feature <name>` if specified).
2. For each target worktree:
   - Ensure the branch is pushed: `git -C <worktree> push -u origin HEAD` if no upstream is configured, else `git -C <worktree> push`.
   - Draft a PR title from the most recent commit subject (or a synthesis of commits ahead of the base branch).
   - Draft a PR description summarising the diff. Include cross-repo PR links if other worktrees in the same feature already have PRs open.
   - Open the PR: `gh pr create --base <default-branch> --head <branch> --title <title> --body <body>`. Append `--draft` if the user asked for draft.
3. Collect PR URLs and emit a final block listing them grouped by feature.

## Anti-patterns

- Force-pushing — never; the worktree's branch might be shared.
- Long PR descriptions — keep them 5-10 bullet points max plus a test plan.
- Auto-merging — do not invoke `gh pr merge` from this skill.

## Output expected

```
Feature 'payments' PRs:
  backend  https://github.com/myorg/backend/pull/142
  frontend https://github.com/myorg/frontend/pull/89
```
````

- [ ] **Step 6: Create `skills/update-pr/SKILL.md`**

````markdown
---
name: update-pr
description: Use when the user has new commits in a kimono worktree whose branch already has a PR open. Pushes the commits, refreshes the PR description if cross-repo PRs have been added since, and reports check/review status.
---

# Kimono Update-PR

## Trigger

Invoke when the user says any of: "update the PR", "push and update", "what's the PR status".

## Checklist

1. Identify target worktrees (all in `.worktrees/`, or those matching `--feature <name>` if specified).
2. For each target worktree:
   - `git -C <worktree> push` to push unpushed commits.
   - Find the PR number: `gh -R <repo> pr view --json number --jq .number` (or list PRs by head branch if `pr view` is ambiguous).
   - If the feature spans multiple worktrees and any sibling worktree has a PR opened since this PR was last edited, refresh the description (`gh pr edit <num> --body <new-body>`).
   - Fetch check and review state: `gh -R <repo> pr view <num> --json statusCheckRollup,reviewDecision`.
3. Report per worktree: pushed-commit-count, PR URL, check summary, review state.

## Anti-patterns

- Force-pushing — never.
- Re-opening closed PRs — out of scope.
- Editing PRs you didn't author without confirmation — confirm first.

## Output expected

```
backend--feature-payments  ✓ pushed 2 commits, PR #142 (checks: passing, review: approved)
frontend--feature-payments — nothing to push, PR #89 (checks: pending, review: changes requested)
```
````

- [ ] **Step 7: Create `skills/code-review/SKILL.md`**

````markdown
---
name: code-review
description: Use when the user wants to address PR review comments in a kimono workspace. Fetches review comments across worktrees, groups them by file and reviewer, and structures them for the agent to act on.
---

# Kimono Code-Review

## Trigger

Invoke when the user says any of: "fetch the review comments", "what did reviewers say", "address the PR comments", "respond to feedback".

## Checklist

1. Identify target worktrees (all in `.worktrees/`, or those matching `--feature <name>` if specified).
2. For each worktree with an open PR:
   - Fetch review comments: `gh -R <repo> api repos/<owner>/<repo>/pulls/<num>/comments`.
   - Filter to unresolved comments only.
   - Group by file, then by line, then by reviewer.
3. Render a structured block per worktree:
   ```
   <repo> PR #<num> (<N> open comments):
     <reviewer> on <file>:<line>
       "<comment body>"
   ```
4. After rendering, ask the user which comments to address. For each chosen comment:
   - Read the file, propose a fix, and apply it in the worktree (not `apps/`).
   - Reply to the comment with a brief summary of the fix: `gh -R <repo> api repos/<owner>/<repo>/pulls/comments/<id>/replies -f body=...`.
5. Hand off to `git-commit` and `update-pr` to ship the fixes.

## Anti-patterns

- Marking comments resolved without an actual fix.
- Editing comments other reviewers wrote.
- Pushing the fix straight to main — always via the PR branch.

## Output expected

A grouped comment listing, followed by the fixes applied (or "left for the user").
````

- [ ] **Step 8: Create `skills/hookify-rules/SKILL.md`**

````markdown
---
name: hookify-rules
description: Use when a kimono workspace needs the read-only master pattern enforced at the tool level. Generates `.claude/hookify/` rule files. Requires the hookify Claude Code plugin to be installed for the rules to take effect.
---

# Kimono Hookify-Rules

## Trigger

Invoke when the user says any of: "set up hookify rules", "enforce the read-only master pattern", "generate hookify rules for this workspace".

## Checklist

1. Confirm the hookify plugin is installed. Look for `hookify` in `~/.claude/settings.json` under `enabledPlugins` or in `~/.claude/plugins/`. If absent, print: "hookify plugin not installed — enable it via Claude Code's plugin manager, then re-run this skill." Continue anyway (the files are still valid).
2. Read `.kimono/config.yml` to learn `workspace.apps_dir` (default `apps`).
3. Ensure `.claude/hookify/` exists. Then write these rule files:

   **`.claude/hookify/no-commit-in-apps.yml`**:
   ```yaml
   name: no-commit-in-apps
   description: Block any git commit whose target path is under apps/ — read-only masters
   matchers:
     - tool: Bash
       command_regex: '^git\s+commit'
       cwd_glob: '<apps_dir>/**'
   action: deny
   message: |
     `apps/` is read-only. Use a worktree under `.worktrees/<repo>--<branch>/` instead.
     Create one with: kimono wt feature <name> <repo> --new
   ```

   **`.claude/hookify/no-checkout-in-apps.yml`**:
   ```yaml
   name: no-checkout-in-apps
   description: Block git checkout in apps/ to keep masters on the default branch
   matchers:
     - tool: Bash
       command_regex: '^git\s+checkout'
       cwd_glob: '<apps_dir>/**'
   action: deny
   message: |
     `apps/<repo>/` must stay on its default branch. Use a worktree:
       kimono wt add <repo> <branch> --new
   ```
   (Substitute `<apps_dir>` with the actual value from config, e.g. `apps`.)
4. Preserve any pre-existing files in `.claude/hookify/` that this skill didn't author — list them in the report and leave them untouched.
5. Report which files were created vs preserved.

## Anti-patterns

- Hardcoding `apps` if the user customised `apps_dir`.
- Adding rules outside the read-only-master scope. Hookify supports a lot — keep this skill focused.
- Deleting user-authored rules in `.claude/hookify/`.

## Output expected

```
Created:
  .claude/hookify/no-commit-in-apps.yml
  .claude/hookify/no-checkout-in-apps.yml
Preserved:
  .claude/hookify/my-custom-rule.yml (not authored by this skill)
```
````

- [ ] **Step 9: Verify `skills/index.json` parses with `jq`**

Run: `jq . skills/index.json > /dev/null`
Expected: no output (silent success).

- [ ] **Step 10: Verify file count**

Run: `ls skills/*/SKILL.md | wc -l`
Expected: `7`.

- [ ] **Step 11: Run the registry unit tests against the on-disk catalog**

Add an integration-style test to verify the on-disk `skills/index.json` parses. Append to `src/plugins/registry.rs`'s `mod tests`:

```rust
    #[test]
    fn test_load_index_from_repo_catalog() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let repo_root = std::path::Path::new(manifest_dir);
        let index = load_index(repo_root).expect("failed to parse repo skills/index.json");
        assert_eq!(index.schema_version, 1);
        assert_eq!(index.skills.len(), 7, "expected 7 skills in catalog");
        let names: Vec<&str> = index.skills.iter().map(|s| s.name.as_str()).collect();
        for required in &[
            "bootstrap",
            "discover",
            "git-commit",
            "create-pr",
            "update-pr",
            "code-review",
            "hookify-rules",
        ] {
            assert!(names.contains(required), "missing skill: {required}");
        }
        let bootstrap = index.skills.iter().find(|s| s.name == "bootstrap").unwrap();
        assert_eq!(bootstrap.default, Some(true));
    }
```

Run: `cargo test --lib plugins::registry::tests::test_load_index_from_repo_catalog`
Expected: PASS.

- [ ] **Step 12: Commit**

```bash
git add skills/ src/plugins/registry.rs
git commit -m "$(cat <<'EOF'
feat(skills): add 7 default skills and catalog manifest

Authors skills/index.json plus skills/{bootstrap,discover,git-commit,
create-pr,update-pr,code-review,hookify-rules}/SKILL.md following the
descriptive (superpowers-style) format — explicit trigger, checklist,
anti-patterns. Adds an integration test that parses the on-disk
catalog and confirms all 7 skills are present.
EOF
)"
```

---

## Task 11: Update E2E test for the v2 flow

The v1 E2E test exercises `kimono context generate` and asserts on generated CLAUDE.md/AGENT.md content. Rewrite it for v2: init → clone → status → wt feature → context show → wt feature --remove. The plugin install step is skipped because the registry repo isn't necessarily reachable in CI; instead, the test seeds a fake `.kimono/config.yml` `plugins.installed` map and asserts that `context show` reports it correctly.

**Files:**
- Modify: `tests/e2e_test.rs`

- [ ] **Step 1: Read the current e2e test**

Run: `wc -l tests/e2e_test.rs` then open the file. Identify the section that calls `kimono context generate` and the assertions on generated files. Those go.

- [ ] **Step 2: Replace the post-clone section**

Find the `context generate` call site in the test. Replace it with a `context show` call and assertions on its stdout. The replacement block looks like (adapt variable names to the existing helper functions):

```rust
    // Seed a plugins.installed entry so `context show` has something to print
    {
        let cfg_path = workspace.join(".kimono").join("config.yml");
        let raw = std::fs::read_to_string(&cfg_path).unwrap();
        let with_plugins = format!("{raw}\nplugins:\n  installed:\n    bootstrap: 1.0.0\n");
        std::fs::write(&cfg_path, with_plugins).unwrap();
    }

    let show_output = run_kimono(&workspace, &["context", "show"]);
    assert!(
        show_output.contains("bootstrap"),
        "context show should list installed skills, got:\n{show_output}"
    );
    assert!(
        show_output.contains("CLAUDE.md not found")
            || show_output.contains("CLAUDE.md"),
        "context show should mention CLAUDE.md status, got:\n{show_output}"
    );
```

- [ ] **Step 3: Remove any assertions on Tera-generated content**

Delete lines that:
- check for the existence of `.claude/agents/<repo>/AGENT.md` (v1 subdir layout) — v2 layout differs and is generated by the bootstrap skill, not by the binary
- assert specific marker comments or Tera-rendered table rows in CLAUDE.md
- exercise `context generate --force` idempotency

The remaining assertions (init creates `.kimono/config.yml`, `clone` populates `apps/`, `wt feature` creates worktrees, `wt feature --remove` cleans them) carry over unchanged.

- [ ] **Step 4: Run the E2E test**

Run: `cargo test --test e2e_test -- --nocapture`
Expected: PASS. If a helper function references a removed module, update or delete it.

- [ ] **Step 5: Run the full test suite**

Run: `cargo test`
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add tests/e2e_test.rs
git commit -m "$(cat <<'EOF'
test: rewrite E2E test for v2 — drop context generate assertions

Replaces the v1 `kimono context generate` + content-assertion section
with a `context show` exercise that seeds plugins.installed and asserts
the status output mentions installed skills and CLAUDE.md presence.
Worktree, clone, and init checks carry over unchanged.
EOF
)"
```

---

## Task 12: Update CLAUDE.md and README to describe v2

The project's `CLAUDE.md` and `README.md` still describe v1 — Tera templates, generated context files, the four workflow Rust commands. Rewrite both to describe the v2 model: plugin system, skills under `skills/`, `kimono bootstrap` / `kimono discover` / `kimono plugins`, etc.

**Files:**
- Modify: `CLAUDE.md`, `README.md`

- [ ] **Step 1: Read both files to understand what's there**

Run: `wc -l CLAUDE.md README.md`. Open both. Note the sections that mention "tera", "context generate", "workflow commands", and the four removed CLI commands — those are the v1 fingerprints.

- [ ] **Step 2: Rewrite the "Tech Stack" section in CLAUDE.md**

Find and update the table to:
- Remove `tera` row
- Add a row for the plugin system: `serde_json — index.json parsing`
- Update "Templates: Tera (Jinja2-like, compiled into binary via include_str!())" — delete this line

- [ ] **Step 3: Rewrite the "Rust Project Structure" section in CLAUDE.md**

Replace the existing tree with the v2 layout. Key changes:
- Remove `src/context/*.rs`, `src/github/`, `templates/`
- Remove `commit.rs, create_pr.rs, update_pr.rs, pr_review_comments.rs` from `cli/`
- Remove `generate.rs, diff.rs` from `cli/context/`
- Add `src/plugins/{mod,registry,install,claude_shell}.rs`
- Add `src/cli/plugins/{mod,list,search,info,install,remove,update}.rs`
- Add `src/cli/{bootstrap,discover}.rs`
- Add `skills/` (one entry per default skill)

- [ ] **Step 4: Rewrite the "CLI Commands" section in CLAUDE.md**

Update the command listing to drop the four workflow commands and `context generate`/`context diff`, and add:

```
kimono plugins list [--installed | --available]
kimono plugins search <query>
kimono plugins info <name>
kimono plugins install <name>... [--version <x>]
kimono plugins remove <name>...
kimono plugins update [<name>...]
kimono bootstrap
kimono discover
```

- [ ] **Step 5: Rewrite the "Config Format" section in CLAUDE.md**

Replace the `claude:` block in the example config with:

```yaml
plugins:
  installed:
    bootstrap: 1.0.0
    discover: 1.0.0
    git-commit: 1.0.0
    create-pr: 1.0.0
    update-pr: 1.0.0
    code-review: 1.0.0
    hookify-rules: 1.0.0
```

- [ ] **Step 6: Rewrite the "Implementation Plan" section in CLAUDE.md**

The CLAUDE.md currently lists v1 phases. Replace with:

```
## Implementation Status

v2 (2026-05-14) — spec: docs/specs/20260421-v2-spec.md, plan: docs/plans/20260514-v2-implementation-plan.md

The v2 pivot is complete:
- Plugin system: kimono plugins {list, search, info, install, remove, update}
- Bootstrap and discover skills replace Rust-and-Tera context generation
- Workflow commands (commit, create-pr, update-pr, code-review) are skills
- Hookify rules are a separate skill (hookify-rules)
- 7 default skills live under skills/ in this repo and are fetched via shallow clone
```

- [ ] **Step 7: Rewrite README.md**

Replace the README's command list and feature description with v2 language. The "Quick Start" section should read:

```
## Quick Start

```bash
# Install
cargo install --path .

# Initialize a workspace (interactive)
kimono init my-workspace
cd my-workspace

# After init, generate AI context
kimono bootstrap

# Later, when you want richer context
kimono discover
```

## Commands

| Command | What it does |
|---------|--------------|
| `kimono init [name]` | Interactive workspace setup |
| `kimono add <name> <remote>` | Add a repo |
| `kimono clone` | Clone all repos from config |
| `kimono sync` | Fetch + rebase all repos |
| `kimono status` | Working tree state across repos |
| `kimono exec <cmd>` | Run a shell command per repo |
| `kimono branch <name>` | Branch across repos |
| `kimono wt feature <name>` | Create cross-repo worktrees for a feature |
| `kimono bootstrap` | Generate initial AI context via Claude Code |
| `kimono discover` | Deep codebase scan for richer AI context |
| `kimono plugins list` | List available / installed skills |
| `kimono plugins install <name>...` | Install one or more skills |
| `kimono context show` | Status of installed skills + context files |
```

- [ ] **Step 8: Verify both files parse as valid markdown**

Run: `cargo build` (sanity — unrelated, but confirms nothing in the docs breaks the rust workspace).
Open both files in a markdown previewer or read them top-to-bottom to confirm there are no broken code fences.

- [ ] **Step 9: Commit**

```bash
git add CLAUDE.md README.md
git commit -m "$(cat <<'EOF'
docs: rewrite CLAUDE.md and README for v2

Updates the tech stack (drops tera), project structure (drops
src/context/, src/github/, templates/; adds src/plugins/ and skills/),
CLI command list (drops the four workflow commands and context
generate/diff; adds plugins, bootstrap, discover), config schema
(drops claude:, adds plugins:), and implementation status. README
quickstart reflects the bootstrap-via-Claude flow.
EOF
)"
```

---

## Task 13: Final verification + push

Confirm the whole pipeline works end-to-end and push to remote.

- [ ] **Step 1: Clean build**

Run: `cargo clean && cargo build --release`
Expected: builds successfully with zero warnings.

- [ ] **Step 2: Run the full test suite**

Run: `cargo test`
Expected: all tests pass.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy --all-targets -- -D warnings`
Expected: no clippy errors.

- [ ] **Step 4: Run cargo fmt --check**

Run: `cargo fmt --check`
Expected: no changes needed. If formatting drift exists, run `cargo fmt` and amend the most recent commit.

- [ ] **Step 5: Manual smoke test of the binary**

Run:
```bash
cd /tmp && rm -rf v2-smoke
/path/to/kimono/target/release/kimono init v2-smoke --bare
cd v2-smoke
/path/to/kimono/target/release/kimono context show
/path/to/kimono/target/release/kimono plugins list --installed
cd / && rm -rf /tmp/v2-smoke
```
Expected:
- `kimono init … --bare` creates `v2-smoke/.kimono/config.yml` and chdirs into the dir
- `context show` prints workspace name, "(none)" for installed skills, and missing-file warnings
- `plugins list --installed` prints "(none)"

- [ ] **Step 6: Push to remote**

```bash
git push origin master
```
Expected: branch-protection allows the push (admin bypass). If it fails, do not force — investigate.

- [ ] **Step 7: Final report**

Verify against the spec (`docs/specs/20260421-v2-spec.md`):
- §4.3 registry layout — `skills/index.json` exists with 7 entries ✓ (Task 10)
- §4.4 bootstrap skill — `skills/bootstrap/SKILL.md` exists ✓ (Task 10)
- §4.5 discover skill — `skills/discover/SKILL.md` exists ✓ (Task 10)
- §4.6 default skill catalog — all 7 present ✓ (Task 10)
- §4.7 flat agent layout — bootstrap skill writes `.claude/agents/<repo>.md` flat ✓ (Task 10 SKILL body)
- §5.1 `kimono plugins` subcommands — wired ✓ (Task 6)
- §5.2 `kimono bootstrap`, `kimono discover` — wired ✓ (Task 7)
- §5.3 removed commands — deleted ✓ (Task 1)
- §5.4 `context show` redefined — done ✓ (Task 8)
- §5.5 unchanged commands — preserved ✓
- §5.6 init wizard — rewritten ✓ (Task 9)
- §5.7 offline behavior — list/install/update guarded ✓ (Tasks 4, 6)
- §6 config schema — `claude` dropped, `plugins` added ✓ (Task 2)
- §7.1 removed modules — done ✓ (Task 1)
- §7.2 added modules — done ✓ (Tasks 3, 4, 5, 6, 7)
- §7.3 modified modules — done ✓ (Tasks 2, 8, 9)
- §7.4 dependencies — tera removed ✓ (Task 1)

Print a summary message: "v2 implementation complete. All 13 tasks done. Push at <SHA>."

---

## Verification commands (per phase)

After **any task**, you can run:

```bash
cargo build           # compiles cleanly
cargo test            # all tests pass
cargo clippy          # no warnings
./target/debug/kimono --help   # CLI surface is intact
```

If any of those break, the task is not done — fix in the same task.
