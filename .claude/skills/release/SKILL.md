---
name: release
description: Use when the user wants to cut a new release of the kimono repo. Checks for a clean working tree, audits README/llms.txt/CLAUDE.md against current behavior, asks for a version bump (major/minor/patch), commits, tags, pushes, and creates a GitHub release.
---

# Kimono Release

You are cutting a new release of the kimono repo. The workflow has guardrails: don't tag uncommitted work, don't ship a release with stale docs, and never force-push.

## Trigger

Invoke when the user says any of: "release kimono", "cut a release", "bump and release", "new release", "release v0.x.y", in the kimono repo working directory.

## Checklist

1. **Confirm you're on the release branch.** Run `git rev-parse --abbrev-ref HEAD`. Expect `master`. If not, ask the user whether to switch or abort. Run `git fetch origin` and `git rev-list --left-right --count HEAD...origin/master` to confirm local is not behind remote — if behind, ask the user to pull first.

2. **Verify clean working tree.** Run `git status --porcelain`. If non-empty:
   - Show a short summary of the modified/untracked files.
   - Ask: "There are uncommitted changes. Include them in this release commit, stash, or abort?"
   - Wait for the user's answer before proceeding. Do not silently include or discard.

3. **Identify the last release tag.** Run `git describe --tags --abbrev=0 2>/dev/null`. If a tag exists (e.g. `v0.2.0`), record it as `<last>`. If no tags exist, use the root commit.

4. **Summarize changes since `<last>`.** Run `git log <last>..HEAD --oneline`. Group commits by Conventional-Commit prefix:
   - `feat:` → new features (relevant for at least a minor bump)
   - `fix:` → bug fixes (relevant for at least a patch bump)
   - `chore:` / `docs:` / `test:` / `refactor:` → housekeeping
   - anything with `BREAKING CHANGE:` or `!:` → breaking (relevant for a major bump)
   Show the user a brief grouped summary so they have the context to choose the bump kind.

5. **Audit docs for staleness against the current binary.** Run `kimono --help` first to capture the live CLI surface, then for each doc:

   - **`README.md`** — verify:
     - The status section reflects the current released version (typically the last tag).
     - The Commands table lists every subcommand printed by `kimono --help` and only those.
     - The install snippet pins to the *current* (about-to-be-released) version.
   - **`llms.txt`** — verify:
     - The `## Commands` section covers every subcommand and matches `kimono --help`.
     - The `## Skills` section enumerates every entry in `skills/index.json`.
     - Inline config + workspace-layout examples reflect current behavior.
   - **`CLAUDE.md`** — verify the tech stack, project structure, CLI commands, and config schema sections.
   - **`docs/specs/<latest>-spec.md`** and **`docs/plans/<latest>-plan.md`** — skim for obvious staleness against the codebase; spec drift is OK, but if a section explicitly contradicts shipped behavior, flag it.

   For each detected drift, **propose the edit and show a brief diff to the user before applying**. Do not edit silently.

6. **Verify `Cargo.toml` baseline.** Run `grep '^version' Cargo.toml`. Confirm with the user this matches the last tag (minus the `v` prefix), not already bumped past it. If someone bumped without releasing, ask whether to keep that version or roll back.

7. **Ask for the bump kind.** Present the changes summary from step 4 alongside the question:
   - **major** (`X.0.0`) — breaking changes; requires user confirmation since this is a commitment
   - **minor** (`x.Y.0`) — backward-compatible new features
   - **patch** (`x.y.Z`) — backward-compatible bug fixes
   Compute the proposed new version and read it back to the user (e.g. "current is 0.2.0, minor bump → 0.3.0 — confirm?").

8. **Apply doc fixes first, as a separate commit.** If step 5 found any drift, apply the approved edits now. Commit them with a `docs: refresh README/llms.txt for vX.Y.Z` prefix. Run `cargo test --bins` after the edits to confirm nothing structural broke; if it did, stop and report.

9. **Bump version.** Edit `Cargo.toml` to the new `version = "X.Y.Z"`. Run `cargo build --release` to refresh `Cargo.lock`. Stage both `Cargo.toml` and `Cargo.lock`.

10. **Commit the bump.** Use a single commit: subject `chore: release vX.Y.Z`, body listing the bump kind and a one-line summary of headline changes. Do not amend.

11. **Push commits.** Run `git push origin master`. If the push is rejected (e.g. someone landed work in the meantime), stop and ask the user how to proceed; never force-push.

12. **Tag the release.** Run `git tag -a vX.Y.Z -m "Release vX.Y.Z — <one-line summary>"`. Then `git push origin vX.Y.Z`. **Tag push must happen after the commit push** so the tag points at a commit that exists on the remote.

13. **Draft release notes.** Compose a Markdown body with:
    - One opening paragraph summarising the release (one or two sentences).
    - `## Highlights` — 3–7 bullets, drawn from the `feat:` and meaningful `fix:` commits.
    - `## Breaking changes` — only when there are real breakages; list each with the migration path.
    - `## Install` — the `cargo install --git https://github.com/sand4rbh/kimono --tag vX.Y.Z` snippet.
    Show the notes to the user for approval before publishing.

14. **Publish the GitHub release.** Run:
    ```
    gh release create vX.Y.Z \
      --title "vX.Y.Z — <one-line summary>" \
      --notes "$(cat <<'EOF'
    <approved release notes>
    EOF
    )"
    ```
    Report the release URL in the final summary.

15. **Verify the release landed.** Run `gh release view vX.Y.Z` and confirm tag, title, and notes look right.

## Anti-patterns

- **Tagging on a dirty working tree.** Always require clean state or an explicit "commit these as part of the release."
- **Skipping the doc audit.** A release with a stale README or llms.txt sets a trap for the next user — they hit the gap on day one. The audit step is non-negotiable.
- **Pushing the tag before the commit.** Tags reference commits; if the commit isn't on origin yet, the tag is dangling.
- **Force-pushing during a release.** Never. If something is wrong, fix forward with another commit and another release.
- **Editing release notes silently after publication.** If a fix is needed, use `gh release edit` and tell the user.
- **Bumping major without explicit confirmation.** Major releases are commitments. Always confirm with the user.
- **Inferring the bump kind from the commits alone.** Show the user the grouped summary and let them pick; "feat:" commits in a v0.x project may still be patch-level depending on intent.
- **Running `cargo install` as part of the release flow.** The release is the artifact; local install is a separate concern and not part of the release commit/tag/publish path.

## Output expected

End with a tight summary like:

```
Released v0.3.0:
- 12 commits since v0.2.0 (3 feat, 2 fix, 7 chore/docs)
- Docs refreshed: README.md, llms.txt (commit 437e9d6)
- Version bump committed: c12fed5
- Tag pushed: v0.3.0
- Release published: https://github.com/sand4rbh/kimono/releases/tag/v0.3.0
```
