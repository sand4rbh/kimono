# Kimono

Multi-repo aggregator CLI for AI-assisted development with Claude Code. Wraps multiple git repos into a single workspace with shared context, worktree-based development, and an installable plugin (skill) system for Claude Code.

## Project State

Rust binary for v2 is complete: plugin system, bootstrap/discover shortcuts that shell out to `claude -p`, and seven default skills live under `skills/` in this repo. The developer (Sandarbh) is learning Rust for the first time through this project.

## What Kimono Does

Kimono gives polyrepo teams monorepo-like AI context while keeping repo independence. A kimono workspace:

1. Clones repos into `apps/` (read-only, always on default branch)
2. Uses git worktrees in `.worktrees/` for all feature development
3. Installs Claude Code skills (plugins) from a registry into `.claude/skills/` — the binary doesn't generate context itself, the skills do
4. Provides CLI commands for cross-repo operations (clone, sync, status, branching, worktrees) and shell-out shortcuts that run skills via `claude -p` (`bootstrap`, `discover`)

## Tech Stack

- **Language**: Rust
- **CLI framework**: clap (derive macros)
- **Config**: serde + serde_yaml (`.kimono/config.yml`)
- **Registry catalog**: serde_json — `index.json` parsing for the plugin catalog
- **Interactive prompts**: dialoguer
- **Terminal output**: console + indicatif
- **Error handling**: anyhow
- **Git operations**: `std::process::Command` calling `git` CLI directly
- **Claude Code integration**: shell-out to `claude -p` for skill execution

## Key Files

| File | Purpose |
|------|---------|
| `docs/specs/` | Feature specs (current: `20260421-v2-spec.md`) |
| `docs/plans/` | Implementation plans (current: `20260514-v2-implementation-plan.md`) |
| `docs/learning/` | Rust learning docs tied to code being written |
| `docs.local/` | **Gitignored**. Personal notes/research that shouldn't be published (current: initial idea, competitive analysis) |
| `README.md` | Public-facing project readme |

## Doc Naming Convention

All docs under `docs/` use the format `YYYYMMDD-kebab-case-title.md` — timestamp prefix (UTC date of creation) followed by a descriptive kebab-case title. This keeps docs naturally sortable and tells you at a glance when something was written.

Examples:
- `docs/specs/20260421-v2-spec.md`
- `docs/plans/20260514-v2-implementation-plan.md`
- `docs/20260418-worktree-cleanup-benchmark.md` (a future research doc)
- `docs/learning/20260417-rust-foundations.md`

Applies to new docs only — existing third-party files (like `README.md`, `CLAUDE.md`) keep their conventional names.

## Reference Implementation

The ofmono prototype at `~/data/work/ofmono` is the reference. It has:
- 5 bash scripts (of-init, of-status, of-sync, of-branch, of-worktree) doing what kimono will do in Rust
- `.repos.conf` (colon-delimited `name:remote:branch`) — kimono uses `.kimono/config.yml` (YAML) instead
- 5 repos: NestJS backend, 2 Next.js frontends, React Native mobile, GitOps infra
- `.claude/` with agents, skills, hookify rules — the patterns kimono's bundled skills generate today

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

kimono plugins list [--installed | --available]   # List available / installed skills
kimono plugins search <query>                     # Search the registry
kimono plugins info <name>                        # Show details for one skill
kimono plugins install <name>... [--version <x>]  # Install one or more skills
kimono plugins remove <name>...                   # Remove installed skills
kimono plugins update [<name>...]                 # Update installed skills

kimono bootstrap                          # Run the bootstrap skill via `claude -p`
kimono discover                           # Run the discover skill via `claude -p`

kimono context show                       # Status of installed skills + context files
```

## Workspace Directory Structure

```
my-platform/                        # Workspace root (git repo)
├── .kimono/
│   └── config.yml                  # Workspace config
├── CLAUDE.md                       # Authored or generated by the bootstrap skill
├── .claude/
│   ├── settings.json               # additionalDirectories
│   ├── skills/                     # Installed skills (one directory per skill)
│   │   ├── bootstrap/SKILL.md
│   │   ├── discover/SKILL.md
│   │   ├── git-commit/SKILL.md
│   │   ├── create-pr/SKILL.md
│   │   ├── update-pr/SKILL.md
│   │   ├── code-review/SKILL.md
│   │   └── hookify-rules/SKILL.md
│   └── hookify/                    # Optional — produced by the hookify-rules skill
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
│   ├── exec.rs, branch.rs
│   ├── bootstrap.rs                # Shell-out to `claude -p` running the bootstrap skill
│   ├── discover.rs                 # Shell-out to `claude -p` running the discover skill
│   ├── wt/{mod,add,remove,list,feature,clean}.rs
│   ├── plugins/{mod,list,search,info,install,remove,update}.rs
│   └── context/{mod,show}.rs
├── config/
│   ├── mod.rs                      # Load, find, validate config
│   └── schema.rs                   # Serde structs for .kimono/config.yml
├── git/
│   └── mod.rs                      # Git CLI wrappers (clone, fetch, worktree, status, etc.)
├── plugins/
│   ├── mod.rs                      # Public plugin API surface
│   ├── registry.rs                 # Registry cache, shallow clone/pull, index.json parsing
│   ├── install.rs                  # Install / remove / update operations
│   └── claude_shell.rs             # Shell out to `claude -p`
└── ui/
    └── mod.rs                      # Colored output, tables, spinners
skills/                             # Default skills bundled in this repo (fetched via shallow clone)
├── index.json                      # Catalog read by the registry layer
├── bootstrap/SKILL.md
├── discover/SKILL.md
├── git-commit/SKILL.md
├── create-pr/SKILL.md
├── update-pr/SKILL.md
├── code-review/SKILL.md
└── hookify-rules/SKILL.md
```

## Implementation Status

v2 (2026-05-14) — spec: docs/specs/20260421-v2-spec.md, plan: docs/plans/20260514-v2-implementation-plan.md

The v2 pivot is complete:
- Plugin system: kimono plugins {list, search, info, install, remove, update}
- Bootstrap and discover skills replace Rust-and-Tera context generation
- Workflow commands (commit, create-pr, update-pr, code-review) are skills
- Hookify rules are a separate skill (hookify-rules)
- 7 default skills live under skills/ in this repo and are fetched via shallow clone

## Key Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Name | kimono | Wraps multiple layers into one cohesive garment |
| Language | Rust | serde_yaml for config, single-binary distribution, no rewrite later |
| AI scope | Claude Code only (v1) | Focus. Architecture allows adding other agents later |
| Config location | `.kimono/config.yml` | Directory allows future expansion |
| Worktree format | `<repo>--<branch-slug>` | Flat, scannable, proven in ofmono |
| Status behavior | Local-only (like `git status`) | No surprise network calls |
| Context generation | Skills via `claude -p` | The agent itself authors context; Rust just stages the prompt |
| Skill distribution | Shallow git clone of a registry | No proprietary protocol; any GitHub repo with `index.json` works |
| Git operations | Shell out to `git` CLI | Same behavior users expect, no libgit2 dependency |

## Working Style

**Background-first execution.** When given a task, prioritize dispatching it to a background agent rather than running it in the current session thread. The main session acts as an orchestrator — it plans, dispatches, and reviews. Actual implementation work should run in background agents (or worktree agents when parallel work is possible). This keeps the main thread responsive and available for the user.

Exceptions: trivial one-line edits, quick reads, and planning/discussion stay in the main thread.

**Research and analysis goes to docs/.** When asked to research, analyze, or investigate something, write the output as a new markdown file in `docs/` (not to plans, memory, or terminal output). Use the `YYYYMMDD-kebab-case-title.md` convention. Specs go in `docs/specs/`, implementation plans go in `docs/plans/`, general research and learning docs go at the `docs/` root or `docs/learning/`.

## Conventions

- **Learning docs** go in `docs/learning/` — one per topic, numbered, tied to code being written
- **Worktree naming**: `<repo>--<branch-slug>` (slashes become dashes)
- **Generated file markers**: `<!-- kimono:start:section -->` / `<!-- kimono:end:section -->` — skills that emit shared files use these so user content outside the markers is preserved
- **Error handling**: use `anyhow::Result` with `.context()` for human-readable error chains
- **No `unwrap()`** in production code — use `?` or handle the error explicitly
