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
