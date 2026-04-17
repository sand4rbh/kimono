# Kimono

Multi-repo aggregator CLI for AI-assisted development with Claude Code. Wraps multiple git repos into a single workspace with shared context, worktree-based development, and generated Claude Code configuration.

## Project State

This is a **greenfield Rust project** — no code written yet, currently in planning phase. The developer (Sandarbh) is learning Rust for the first time through this project.

## What Kimono Does

Kimono gives polyrepo teams monorepo-like AI context while keeping repo independence. A kimono workspace:

1. Clones repos into `apps/` (read-only, always on default branch)
2. Uses git worktrees in `.worktrees/` for all feature development
3. Generates Claude Code config (CLAUDE.md, agents, skills, hookify rules) from a single YAML config
4. Provides CLI commands for cross-repo operations (clone, sync, status, branching, worktrees, PRs)

## Tech Stack

- **Language**: Rust
- **CLI framework**: clap (derive macros)
- **Config**: serde + serde_yaml (`.kimono/config.yml`)
- **Templates**: Tera (Jinja2-like, compiled into binary via `include_str!`)
- **Interactive prompts**: dialoguer
- **Terminal output**: console + indicatif
- **Error handling**: anyhow
- **Git operations**: `std::process::Command` calling `git` CLI directly
- **GitHub operations**: `gh` CLI wrapper

## Key Files

| File | Purpose |
|------|---------|
| `docs/specs/` | Feature specs (current: `20260417-v1-spec.md`) |
| `docs/plans/` | Implementation plans (current: `20260417-v1-implementation-plan.md`) |
| `docs/learning/` | Rust learning docs tied to code being written |
| `docs.local/` | **Gitignored**. Personal notes/research that shouldn't be published (current: initial idea, competitive analysis) |
| `README.md` | Public-facing project readme |

## Doc Naming Convention

All docs under `docs/` use the format `YYYYMMDD-kebab-case-title.md` — timestamp prefix (UTC date of creation) followed by a descriptive kebab-case title. This keeps docs naturally sortable and tells you at a glance when something was written.

Examples:
- `docs/specs/20260417-v1-spec.md`
- `docs/plans/20260417-v1-implementation-plan.md`
- `docs/20260418-worktree-cleanup-benchmark.md` (a future research doc)
- `docs/learning/20260417-rust-foundations.md`

Applies to new docs only — existing third-party files (like `README.md`, `CLAUDE.md`) keep their conventional names.

## Reference Implementation

The ofmono prototype at `~/data/work/ofmono` is the reference. It has:
- 5 bash scripts (of-init, of-status, of-sync, of-branch, of-worktree) doing what kimono will do in Rust
- `.repos.conf` (colon-delimited `name:remote:branch`) — kimono uses `.kimono/config.yml` (YAML) instead
- 5 repos: NestJS backend, 2 Next.js frontends, React Native mobile, GitOps infra
- `.claude/` with agents, skills, hookify rules — the patterns kimono will auto-generate

## Config Format

`.kimono/config.yml` — the central config file:

```yaml
workspace:
  name: my-platform
  apps_dir: apps              # default
  worktree_dir: .worktrees    # default

repos:
  backend:
    remote: git@github.com:myorg/backend.git
    branch: main              # default
    description: "NestJS GraphQL API"
    tech: [nestjs, typescript, graphql]
    package_manager: yarn
    depends_on: []
    commands:
      dev: yarn dev
      lint: yarn lint
      test: yarn test

  frontend:
    remote: git@github.com:myorg/frontend.git
    branch: main
    description: "Next.js web app"
    tech: [nextjs, react, typescript]
    package_manager: pnpm
    depends_on: [backend]
    commands:
      dev: pnpm dev

claude:
  dispatch: true
  agents: true
  skills: true
  context_style: detailed
```

Only `workspace.name` and `repos.<name>.remote` are required. Everything else has defaults.

## CLI Commands

```
kimono init [--from <path>] [--bare]     # Interactive workspace setup
kimono add <name> <remote>                # Add repo to workspace
kimono remove <name> [--delete]           # Remove repo
kimono clone [repos...]                   # Clone repos from config
kimono sync [repos...] [--fetch-only]     # Fetch + rebase all repos
kimono status [repos...]                  # Git status across repos (local-only, no fetch)
kimono exec <command> [repos...]          # Run command in each repo
kimono branch <name> [repos...] [--new]   # Branch across repos

kimono wt add <repo> <branch> [--new]     # Create worktree
kimono wt remove <repo> <branch>          # Remove worktree
kimono wt list [repo]                     # List worktrees
kimono wt feature <name> [repos...] [--new] [--remove]  # Cross-repo feature worktrees
kimono wt clean [--merged] [--dry-run]    # Clean merged worktrees

kimono context generate [--force]         # Generate Claude Code config files
kimono context diff                       # Show what would change
kimono context show                       # Show context file status

kimono commit [repos...] [--feature <name>] [-m <msg>]   # Commit in worktrees
kimono create-pr [repos...] [--feature <name>] [--draft]  # Create PRs
kimono update-pr [repos...] [--feature <name>]             # Push + update PRs
kimono pr-review-comments [repos...] [--feature <name>]    # Fetch PR review comments
```

## Workspace Directory Structure

```
my-platform/                        # Workspace root (git repo)
├── .kimono/
│   └── config.yml                  # Workspace config
├── CLAUDE.md                       # Generated root context
├── .claude/
│   ├── settings.json               # additionalDirectories
│   ├── agents/<repo>/AGENT.md      # Per-repo agent definitions
│   ├── skills/<repo>/SKILL.md      # Per-repo dispatcher skills
│   ├── skills/commit/SKILL.md      # Workflow skills
│   ├── skills/create-pr/SKILL.md
│   ├── skills/update-pr/SKILL.md
│   ├── skills/pr-review-comments/SKILL.md
│   └── hookify/                    # Read-only master enforcement
├── apps/                           # Read-only clones (always on default branch)
├── .worktrees/                     # Active development (<repo>--<branch-slug>)
└── docs/                           # Optional documentation
```

## Rust Project Structure

```
src/
├── main.rs                         # Clap CLI definition + command dispatch
├── cli/
│   ├── mod.rs
│   ├── init.rs, clone.rs, sync.rs, status.rs, add.rs, remove.rs
│   ├── exec.rs, branch.rs, commit.rs, create_pr.rs, update_pr.rs, pr_review_comments.rs
│   ├── wt/{mod,add,remove,list,feature,clean}.rs
│   └── context/{mod,generate,diff,show}.rs
├── config/
│   ├── mod.rs                      # Load, find, validate config
│   └── schema.rs                   # Serde structs for .kimono/config.yml
├── git/
│   └── mod.rs                      # Git CLI wrappers (clone, fetch, worktree, status, etc.)
├── github/
│   └── mod.rs                      # gh CLI wrappers (PR create, view, comments)
├── context/
│   ├── mod.rs                      # Tera template engine setup
│   ├── claude_md.rs, agents.rs, skills.rs, settings.rs, hookify.rs
│   └── preserve.rs                 # Custom section preservation (<!-- kimono:start/end -->)
└── ui/
    └── mod.rs                      # Colored output, tables, spinners
templates/                          # Tera templates (compiled into binary)
```

## Implementation Plan

7 phases — see `impl-plan.md` for full details:

0. **Bootstrap** — cargo init, clap skeleton, module stubs
1. **Foundation** (3 parallel) — config schema, git wrapper, UI helpers
2. **First commands** (2 parallel) — clone/status/sync/exec + branch
3. **Worktree commands** — add/remove/list/feature/clean
4. **Context generation** (2 sequenced) — templates, then generation engine
5. **CLI + workflow** (2 parallel) — context CLI + commit/PR commands
6. **Init wizard** — interactive setup, ofmono migration
7. **Polish** — wire everything, E2E tests, clippy

## Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Name | kimono | Wraps multiple layers into one cohesive garment |
| Language | Rust | serde_yaml for config, single-binary distribution, no rewrite later |
| AI scope | Claude Code only (v1) | Focus. Architecture allows adding Cursor/AGENTS.md later |
| Config location | `.kimono/config.yml` | Directory allows future expansion |
| Worktree format | `<repo>--<branch-slug>` | Flat, scannable, proven in ofmono |
| Status behavior | Local-only (like `git status`) | No surprise network calls |
| Hooks | Hookify | More ergonomic than raw hooks.json, fallback to CLAUDE.md prose |
| Templates | Compiled into binary | No external files to distribute |
| Git operations | Shell out to `git` CLI | Same behavior users expect, no libgit2 dependency |

## Working Style

**Background-first execution.** When given a task, prioritize dispatching it to a background agent rather than running it in the current session thread. The main session acts as an orchestrator — it plans, dispatches, and reviews. Actual implementation work should run in background agents (or worktree agents when parallel work is possible). This keeps the main thread responsive and available for the user.

Exceptions: trivial one-line edits, quick reads, and planning/discussion stay in the main thread.

**Research and analysis goes to docs/.** When asked to research, analyze, or investigate something, write the output as a new markdown file in `docs/` (not to plans, memory, or terminal output). Use the `YYYYMMDD-kebab-case-title.md` convention. Specs go in `docs/specs/`, implementation plans go in `docs/plans/`, general research and learning docs go at the `docs/` root or `docs/learning/`.

## Conventions

- **Learning docs** go in `docs/learning/` — one per topic, numbered, tied to code being written
- **Worktree naming**: `<repo>--<branch-slug>` (slashes become dashes)
- **Generated file markers**: `<!-- kimono:start:section -->` / `<!-- kimono:end:section -->` — content between markers is regenerated, content outside is preserved
- **Error handling**: use `anyhow::Result` with `.context()` for human-readable error chains
- **No `unwrap()`** in production code — use `?` or handle the error explicitly
