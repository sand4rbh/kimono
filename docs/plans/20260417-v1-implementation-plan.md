# Kimono Implementation Plan

## Context

Building a Rust CLI tool called "kimono" that wraps multiple git repos into a single workspace with shared context, worktree-based development, and generated Claude Code configuration. Extracting from the ofmono prototype at `~/data/work/ofmono`. The user is learning Rust for the first time — phases are ordered by increasing Rust complexity.

## Current State

- Repo: `~/data/code/kimono` — has planning docs only (initial.md, spec-v1.md, analysis doc, learning/)
- No Rust code, no Cargo.toml, no .gitignore, no commits, no remote
- Rust toolchain needs to be installed via `rustup`

## Dependency Graph

```
config/schema.rs  ──┐
config/mod.rs     ──┤
git/mod.rs        ──┤──> cli/clone,status,sync,exec,branch ──> cli/wt/* ──┐
github/mod.rs     ──┤                                                      │
ui/mod.rs         ──┘                                                      │
                                                                           v
templates/*.tera ──> context/* ──> cli/context/* ──> cli/init ──> main.rs (final)
                                   cli/commit,create-pr,update-pr,pr-review-comments
```

## Parallelization Diagram

```
Phase 0: Bootstrap (1 agent)
  │
  ▼
Phase 1: Foundation ─────────────────────────────
  │           │              │              │
  │     [Track 1A]     [Track 1B]     [Track 1C]
  │      Config          Git            UI
  │     schema.rs       mod.rs         mod.rs
  │      mod.rs
  │           │              │              │
  ├───────────┴──────────────┴──────────────┘
  ▼                                  MERGE
Phase 2: First Commands ─────────────────────────
  │           │                        │
  │     [Track 2A]               [Track 2B]
  │   clone/status/sync/exec      branch
  │           │                        │
  ├───────────┴────────────────────────┘
  ▼                                  MERGE
Phase 3: Worktree Commands (1 agent, sequential)
  │   add → list → remove → feature → clean
  │
  ▼
Phase 4: Context ────────────────────────────────
  │           │                        │
  │     [Track 4A]               [Track 4B]
  │     Templates              Generation Engine
  │   (must finish            (depends on 4A)
  │    before 4B)
  │           │                        │
  ├───────────┴────────────────────────┘
  ▼                                  MERGE
Phase 5: CLI + Workflow ─────────────────────────
  │           │                        │
  │     [Track 5A]               [Track 5B]
  │   context generate/         GitHub wrapper +
  │   diff/show                 commit/PR/review
  │           │                        │
  ├───────────┴────────────────────────┘
  ▼                                  MERGE
Phase 6: Init + Add/Remove (1 agent)
  │
  ▼
Phase 7: Polish + E2E Tests (1 agent)
```

### Max Parallel Agents Per Phase

| Phase | Agents | Worktree Branches |
|-------|--------|-------------------|
| 0 | 1 | `main` |
| 1 | **3** | `feat/config-schema`, `feat/git-wrapper`, `feat/ui-helpers` |
| 2 | **2** | `feat/repo-commands`, `feat/branch-command` |
| 3 | 1 | `feat/worktree-commands` |
| 4 | **2** (sequenced) | `feat/templates`, then `feat/context-engine` |
| 5 | **2** | `feat/context-cli`, `feat/workflow-commands` |
| 6 | 1 | `feat/init-wizard` |
| 7 | 1 | `feat/polish-integration` |

---

## Phase 0: Project Bootstrap

**Goal:** Empty CLI that compiles and prints help.

**Steps:**
1. Install Rust via `rustup` (if not installed)
2. `cargo init` inside `~/data/code/kimono`
3. Add dependencies to `Cargo.toml`:
   - `clap = { version = "4", features = ["derive"] }`
   - `serde = { version = "1", features = ["derive"] }`
   - `serde_yaml = "0.9"`, `serde_json = "1"`, `anyhow = "1"`
   - `console = "0.15"`, `indicatif = "0.17"`, `dialoguer = "0.11"`
   - `tera = "1"`, `which = "7"`, `dirs = "6"`
4. Create module skeleton (all `mod` declarations, empty files):
   - `src/main.rs` — clap `Cli` enum with all commands stubbed as `todo!()`
   - `src/cli/mod.rs`, `src/config/{mod,schema}.rs`, `src/git/mod.rs`
   - `src/github/mod.rs`, `src/ui/mod.rs`, `src/context/mod.rs`
5. Create `.gitignore`: `/target`, `.DS_Store`
6. `cargo build` must succeed
7. `cargo run -- --help` prints all subcommands

**Verify:** `cargo run -- --help`
**Commit:** `bootstrap: cargo init with clap skeleton and all module stubs`
**Learning doc:** `learning/01-rust-foundations.md` (already written)

---

## Phase 1: Foundation Layer (3 parallel tracks)

### Track 1A: Config Schema + Loader

**Files:** `src/config/schema.rs`, `src/config/mod.rs`, `tests/fixtures/config.yml`, `tests/fixtures/config_minimal.yml`

**Build:**
- `schema.rs` — serde structs: `KimonoConfig`, `Workspace`, `Repo`, `ClaudeConfig`
  - All derive `Debug, Clone, Serialize, Deserialize`
  - `#[serde(default)]` for optional fields, `Option<T>` for nullable
  - Default functions for `apps_dir` ("apps"), `worktree_dir` (".worktrees"), `branch` ("main")
- `mod.rs` — config operations:
  - `load(path: &Path) -> Result<KimonoConfig>` — read + parse YAML
  - `find_config() -> Result<PathBuf>` — walk up from cwd looking for `.kimono/config.yml`
  - `validate(config: &KimonoConfig) -> Result<()>` — no circular deps, no unknown dep refs
  - `workspace_root() -> Result<PathBuf>` — parent of `.kimono/`

**Tests:** Parse full config, parse minimal config, verify defaults, catch circular deps, catch missing dep refs
**Rust concepts:** serde, `Option<T>`, `HashMap`, `Vec`, `Result`, `?`, file I/O

### Track 1B: Git Wrapper

**Files:** `src/git/mod.rs`

**Build:** Functions wrapping `git` CLI via `std::process::Command`:
- `git_available()`, `clone()`, `fetch()`, `rebase()`
- `current_branch()`, `is_clean()`, `status_porcelain()`, `ahead_behind()`, `stash_count()`
- `worktree_add()`, `worktree_remove()`, `worktree_list()` → `Vec<WorktreeInfo>`
- `branch_exists_local()`, `branch_exists_remote()`, `create_branch()`, `checkout()`
- `push()`, `log_oneline()`
- `branch_slug(branch: &str) -> String` — convert `/` to `-`
- Internal: `run_git(repo_path, args) -> Result<CommandOutput>` helper

**Tests:** Unit test `branch_slug`, integration tests with `tempfile` creating real git repos
**Rust concepts:** `std::process::Command`, `PathBuf` vs `Path`, `String::from_utf8`

### Track 1C: UI Helpers

**Files:** `src/ui/mod.rs`

**Build:** Output formatting with `console` crate:
- `success()`, `warn()`, `error()`, `skip()`, `info()`, `header()`
- `repo_status_line()` — formatted status matching ofmono's output
- `summary()` — cloned/skipped/failed counts
- `spinner()` — `indicatif` ProgressBar for long operations

**Tests:** Smoke tests (no panics)
**Rust concepts:** `&str` vs `String`, closures, the `console` crate API

**Merge commit:** `feat: add config schema, git wrapper, and UI helpers`

---

## Phase 2: First Working Commands (2 parallel tracks)

### Track 2A: clone + status + sync + exec

**Files:** `src/cli/{clone,status,sync,exec}.rs`, update `src/cli/mod.rs` and `src/main.rs`

**Build — each is a direct port of the ofmono script:**
- `clone.rs` — load config, skip if exists, `git::clone()`, print summary
- `status.rs` — load config, per-repo: branch, dirty state, ahead/behind, stash count, worktree list. Local-only (no fetch).
- `sync.rs` — load config, fetch, check clean + on default branch, rebase. `--fetch-only` flag.
- `exec.rs` — load config, run shell command in each repo's `apps/` dir

**Tests:** Integration tests with temp workspace + bare git repos

### Track 2B: branch

**Files:** `src/cli/branch.rs`

**Build:** Port of `of-branch`. Create/checkout branch across repos. `--from-master`, `--list`, `--no-checkout` flags.

**Tests:** Create branch across 2 repos, verify exists, list mode

**Merge commit:** `feat: add clone, status, sync, exec, and branch commands`

**Milestone:** Replicates 100% of ofmono script functionality:
```
cargo run -- clone
cargo run -- status
cargo run -- sync
cargo run -- exec "git log --oneline -3"
cargo run -- branch my-feature backend frontend --from-master
```

---

## Phase 3: Worktree Commands (sequential)

**Files:** `src/cli/wt/{mod,add,remove,list,feature,clean}.rs`

**Build order:**
1. `add.rs` — create worktree, `--new` creates branch from `origin/<default>`
2. `list.rs` — enumerate worktrees, format output
3. `remove.rs` — remove worktree, optionally delete branch
4. `feature.rs` — **killer command**: create/remove worktrees across repos for a feature. Respects `depends_on` order.
5. `clean.rs` — find merged/deleted branches, remove their worktrees. `--dry-run` flag.

**Key details:**
- Worktree path: `.worktrees/<repo>--<branch_slug>/`
- `feature.rs` sorts repos by dependency order (no-deps first, then dependents)
- `clean.rs` uses `git branch --merged` and `git ls-remote --heads`

**Tests:** Create worktrees, list, verify paths, remove, feature across 2 repos, clean after merge
**Commit:** `feat: add worktree management commands (add, remove, list, feature, clean)`

**Milestone:**
```
cargo run -- wt feature payments backend frontend --new
cargo run -- wt list
cargo run -- wt feature payments --remove
cargo run -- wt clean --merged --dry-run
```

---

## Phase 4: Context Generation (2 tracks, sequenced)

### Track 4A: Templates

**Files:** `templates/*.tera`, `src/context/mod.rs` (template engine setup)

**Build:**
- `claude_md.tera` — workspace overview, repo registry table, deps, commands, agent dispatch. Uses `<!-- kimono:start:section -->` markers.
- `agent_md.tera` — YAML frontmatter + markdown. Based on ofmono `.claude/agents/backend/AGENT.md`.
- `skill_repo.tera` — repo dispatcher skill. Based on ofmono `.claude/skills/backend/SKILL.md`.
- `skill_commit.tera`, `skill_create_pr.tera`, `skill_update_pr.tera`, `skill_pr_review.tera` — workflow skills
- `hookify_no_commit_apps.tera`, `hookify_no_checkout_apps.tera` — hookify rules
- `settings_json.tera`, `gitignore.tera`
- `context/mod.rs` — `create_tera()` loading all templates via `include_str!()`

### Track 4B: Generation Engine (after 4A merges)

**Files:** `src/context/{claude_md,agents,skills,settings,hookify,preserve}.rs`

**Build:**
- `preserve.rs` — extract custom sections from existing files (content outside `<!-- kimono:start/end -->` markers), merge back after regeneration. Algorithmically the trickiest module.
- `claude_md.rs` — render root CLAUDE.md, merge with existing customs
- `agents.rs` — render per-repo AGENT.md files
- `skills.rs` — render repo dispatcher + workflow skill files
- `settings.rs` — render `.claude/settings.json` with `additionalDirectories`
- `hookify.rs` — render hookify rule files

**Tests:** `preserve.rs` round-trip test, template rendering with fixture config, valid output
**Commit:** `feat: add context generation engine with Tera templates`

---

## Phase 5: CLI + Workflow (2 parallel tracks)

### Track 5A: Context CLI Commands

**Files:** `src/cli/context/{mod,generate,diff,show}.rs`

- `generate.rs` — load config, call generators, write files, preserve customs. `--force` flag.
- `diff.rs` — generate in memory, compare to disk, print changes
- `show.rs` — list expected context files, show exists/customized/missing status

### Track 5B: GitHub Wrapper + Workflow Commands

**Files:** `src/github/mod.rs`, `src/cli/{commit,create_pr,update_pr,pr_review_comments}.rs`

**GitHub wrapper:**
- `gh_available()`, `pr_create()`, `pr_view()`, `pr_comments()`, `pr_update_body()`

**Workflow commands (all support `--feature <name>`):**
- `commit.rs` — find worktrees with staged changes, commit with message
- `create_pr.rs` — push + `gh pr create`, `--draft` flag, cross-repo PR links
- `update_pr.rs` — push + update PR, report check status
- `pr_review_comments.rs` — fetch and display review comments grouped by repo/PR

**Tests:** GitHub tests `#[ignore]` (require `gh` auth), workflow argument parsing tested directly
**Commit:** `feat: add context CLI commands and git workflow commands`

---

## Phase 6: Init + Add/Remove

**Files:** `src/cli/{init,add,remove}.rs`

- `init.rs` — interactive wizard using `dialoguer`. Creates `.kimono/config.yml`, `.gitignore`, clones repos, generates context, inits git repo. `--from <path>` reads ofmono `.repos.conf`. `--bare` skips clone + context.
- `add.rs` — add repo to config YAML, clone, regenerate context
- `remove.rs` — remove from config, optionally delete files, regenerate

**Tests:** Test `--bare` and `--from` paths (non-interactive)
**Commit:** `feat: add init wizard, add, and remove commands`

---

## Phase 7: Polish + E2E

- Wire all commands in `main.rs` — no more `todo!()`
- End-to-end test: init → add repos → clone → status → wt feature → context generate
- `cargo clippy`, `cargo fmt`
- Error message polish with `anyhow::Context`
- Optional: `clap_complete` for shell completions

**Commit:** `feat: wire all commands, add integration tests, polish`

---

## Critical Path

```
Phase 0 → Phase 1A (config) → Phase 2A (clone/status) → Phase 3 (worktrees)
  → Phase 4A (templates) → Phase 4B (context engine) → Phase 5A (context CLI)
  → Phase 6 (init) → Phase 7 (polish)
```

Config is on the critical path because every command loads config.

## Testing Strategy

| Layer | Approach | Crates |
|-------|----------|--------|
| Config parsing | Unit tests with YAML fixtures | serde_yaml |
| Git operations | Integration tests with temp repos | tempfile |
| UI formatting | Smoke tests (no panics) | - |
| Context preservation | Unit tests with string fixtures | - |
| Template rendering | Unit tests with fixture config | tera |
| CLI commands | Integration tests with temp workspace | assert_cmd, predicates |
| GitHub ops | `#[ignore]` tests (require `gh` auth) | - |
| End-to-end | Full workflow in temp dir | tempfile, assert_cmd |

## Verification

After each phase, run:
```bash
cargo build          # compiles
cargo test           # all tests pass
cargo clippy         # no warnings
cargo run -- --help  # CLI shows all wired commands
```

After Phase 7, full verification:
```bash
mkdir /tmp/test-workspace && cd /tmp/test-workspace
kimono init --bare
# edit .kimono/config.yml to add test repos
kimono clone
kimono status
kimono wt feature test-feature repo1 repo2 --new
kimono context generate
# verify CLAUDE.md, .claude/agents/, .claude/skills/ exist and look correct
kimono wt feature test-feature --remove
```
