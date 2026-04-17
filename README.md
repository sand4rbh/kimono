# kimono

> Multi-repo aggregator for AI-assisted development with Claude Code.

Kimono wraps multiple git repositories into a single workspace with shared context, worktree-based development, and generated Claude Code configuration. You get monorepo-like AI context while keeping polyrepo independence for CI/CD, deployment, and ownership.

## What it does

1. Clones your repos into `apps/` (read-only, always on default branch — the source of truth AI agents read from)
2. All active development happens in git worktrees under `.worktrees/`
3. Generates Claude Code configuration (`CLAUDE.md`, agents, skills, hookify rules) from a single YAML config
4. Provides CLI commands for cross-repo operations: clone, sync, status, branching, worktrees, commits, PRs

## Status

Pre-alpha. Core features implemented, not yet released.

## Install

Requires Rust 1.75+. For now, build from source:

```bash
git clone <this-repo> kimono
cd kimono
cargo install --path .
```

This installs the `kimono` binary to `~/.cargo/bin/`.

Also requires `git` (for all repo operations) and `gh` (for PR commands).

## Quick start

```bash
# In an empty directory, set up a new workspace
mkdir my-platform && cd my-platform
kimono init

# Or migrate from an existing ofmono-style setup
kimono init --from ~/path/to/ofmono

# Add a repo later
kimono add backend git@github.com:myorg/backend.git

# Clone all repos
kimono clone

# Check status across all repos (like git status, but multi-repo)
kimono status

# Sync all repos with their remotes
kimono sync

# Create a cross-repo feature with coordinated worktrees
kimono wt feature payments backend frontend --new

# Commit across all worktrees for a feature
kimono commit --feature payments -m "Add payment processing"

# Create PRs across all repos in the feature
kimono create-pr --feature payments

# See PR review comments (for AI agents to address)
kimono pr-review-comments --feature payments

# Regenerate Claude Code context files after config changes
kimono context generate

# Clean up merged worktrees
kimono wt clean --merged
```

## Config

Create `.kimono/config.yml` (or use `kimono init` to generate it):

```yaml
workspace:
  name: my-platform

repos:
  backend:
    remote: git@github.com:myorg/backend.git
    branch: main
    description: "NestJS GraphQL API"
    tech: [nestjs, typescript, graphql]
    package_manager: yarn
    commands:
      dev: yarn dev
      test: yarn test

  frontend:
    remote: git@github.com:myorg/frontend.git
    description: "Next.js web app"
    tech: [nextjs, react, typescript]
    package_manager: pnpm
    depends_on: [backend]
    commands:
      dev: pnpm dev
```

Only `workspace.name` and `repos.<name>.remote` are required. Everything else has sensible defaults.

## Commands

### Workspace

| Command | Description |
|---------|-------------|
| `kimono init [--from <path>] [--bare]` | Interactive workspace setup |
| `kimono add <name> <remote> [--branch <branch>]` | Add a repo to the workspace |
| `kimono remove <name> [--delete]` | Remove a repo from the workspace |

### Repos

| Command | Description |
|---------|-------------|
| `kimono clone [repos...]` | Clone repos from config |
| `kimono sync [repos...] [--fetch-only]` | Fetch + rebase all repos |
| `kimono status [repos...]` | Git status across repos (local-only, like `git status`) |
| `kimono exec <cmd> [repos...]` | Run a shell command in each repo |
| `kimono branch <name> [repos...] [--new] [--from-master] [--list]` | Branch across repos |

### Worktrees

| Command | Description |
|---------|-------------|
| `kimono wt add <repo> <branch> [--new]` | Create a worktree |
| `kimono wt remove <repo> <branch>` | Remove a worktree |
| `kimono wt list [repo]` | List worktrees |
| `kimono wt feature <name> [repos...] [--new] [--remove]` | Cross-repo feature worktrees |
| `kimono wt clean [--merged] [--dry-run]` | Clean merged/stale worktrees |

### Git workflow

| Command | Description |
|---------|-------------|
| `kimono commit [repos...] [--feature <name>] [-m <msg>]` | Commit across worktrees |
| `kimono create-pr [repos...] [--feature <name>] [--draft]` | Create PRs |
| `kimono update-pr [repos...] [--feature <name>]` | Push + report PR status |
| `kimono pr-review-comments [repos...] [--feature <name>]` | Show PR review comments |

### Claude Code context

| Command | Description |
|---------|-------------|
| `kimono context generate [--force]` | Generate Claude Code config files |
| `kimono context diff` | Show what would change |
| `kimono context show` | Show which context files exist |

## How it works

A kimono workspace is a git repo that acts as an orchestrator:

```
my-platform/
├── .kimono/
│   └── config.yml              # Workspace config
├── CLAUDE.md                   # Generated root context (preserves custom sections)
├── .claude/
│   ├── settings.json           # additionalDirectories for Claude Code
│   ├── agents/<repo>/AGENT.md  # Per-repo agent definitions
│   ├── skills/<repo>/SKILL.md  # Per-repo dispatcher skills
│   ├── skills/commit/SKILL.md  # Workflow skills (commit, create-pr, etc.)
│   └── hookify/                # Read-only master enforcement
├── apps/                       # Read-only clones (always on default branch)
│   ├── backend/
│   └── frontend/
└── .worktrees/                 # All active development
    ├── backend--feature-payments/
    └── frontend--feature-payments/
```

### Read-only master pattern

`apps/` is never directly modified — it stays on the default branch and serves as a clean reference. AI agents read cross-repo context from `apps/` (e.g., frontend reads backend's API types) while writing code in `.worktrees/`. Generated hookify rules enforce this at the tool level.

### Preserved custom sections

`kimono context generate` uses marker comments (`<!-- kimono:start:section -->` / `<!-- kimono:end:section -->`) so you can add your own content to `CLAUDE.md` without it being overwritten on regeneration.

## Docs

- [Full feature spec](docs/specs/20260417-v1-spec.md) — config format, all commands, generated files
- [Implementation plan](docs/plans/20260417-v1-implementation-plan.md) — phased build, parallelization, dependency graph
- [Rust learning notes](docs/learning/) — notes written as we built kimono

## License

TBD
