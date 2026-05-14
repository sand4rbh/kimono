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
