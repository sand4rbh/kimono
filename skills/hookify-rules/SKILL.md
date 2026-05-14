---
name: hookify-rules
description: Use when a kimono workspace needs the read-only master pattern enforced at the tool level. Generates `.claude/hookify/` rule files. Requires the hookify Claude Code plugin to be installed for the rules to take effect.
---

# Kimono Hookify-Rules

## Trigger

Invoke when the user says any of: "set up hookify rules", "enforce the read-only master pattern", "generate hookify rules for this workspace".

## Checklist

1. Confirm the hookify plugin is installed. Look for `hookify` in `~/.claude/settings.json` under `enabledPlugins` or in `~/.claude/plugins/`. If absent, print: "hookify plugin not installed — enable it via Claude Code's plugin manager, then re-run this skill." Continue anyway (the files are still valid).
2. Read `.kimono/config.yml` to learn `workspace.apps_dir` (default `apps`).
3. Ensure `.claude/hookify/` exists. Then write these rule files:

   **`.claude/hookify/no-commit-in-apps.yml`**:
   ```yaml
   name: no-commit-in-apps
   description: Block any git commit whose target path is under apps/ — read-only masters
   matchers:
     - tool: Bash
       command_regex: '^git\s+commit'
       cwd_glob: '<apps_dir>/**'
   action: deny
   message: |
     `apps/` is read-only. Use a worktree under `.worktrees/<repo>--<branch>/` instead.
     Create one with: kimono wt feature <name> <repo> --new
   ```

   **`.claude/hookify/no-checkout-in-apps.yml`**:
   ```yaml
   name: no-checkout-in-apps
   description: Block git checkout in apps/ to keep masters on the default branch
   matchers:
     - tool: Bash
       command_regex: '^git\s+checkout'
       cwd_glob: '<apps_dir>/**'
   action: deny
   message: |
     `apps/<repo>/` must stay on its default branch. Use a worktree:
       kimono wt add <repo> <branch> --new
   ```
   (Substitute `<apps_dir>` with the actual value from config, e.g. `apps`.)
4. Preserve any pre-existing files in `.claude/hookify/` that this skill didn't author — list them in the report and leave them untouched.
5. Report which files were created vs preserved.

## Anti-patterns

- Hardcoding `apps` if the user customised `apps_dir`.
- Adding rules outside the read-only-master scope. Hookify supports a lot — keep this skill focused.
- Deleting user-authored rules in `.claude/hookify/`.

## Output expected

```
Created:
  .claude/hookify/no-commit-in-apps.yml
  .claude/hookify/no-checkout-in-apps.yml
Preserved:
  .claude/hookify/my-custom-rule.yml (not authored by this skill)
```
