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
