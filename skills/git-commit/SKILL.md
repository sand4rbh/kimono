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
