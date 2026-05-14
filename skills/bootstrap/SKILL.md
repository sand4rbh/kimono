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
