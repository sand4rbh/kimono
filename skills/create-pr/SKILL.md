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
