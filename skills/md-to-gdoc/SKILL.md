---
name: md-to-gdoc
description: Use when the user wants to convert, upload, or share a local markdown file as a Google Doc — e.g. "make this a Google Doc", "share as gdoc", "upload to Drive", "/md-to-gdoc <file>". Handles both create (new Doc) and update (re-export the same .md to its existing Doc, preserving the URL). Bash + curl + jq, OAuth on behalf of the current user. Drive API does the markdown conversion natively (since Jul 2024) — headings, lists, links, code blocks, and tables are preserved.
user-invocable: true
---

# md-to-gdoc Skill

Convert a local `.md` file to a Google Doc in the user's Drive, using the Drive API's native markdown conversion. Tracks the resulting Doc ID in the markdown's frontmatter so re-running on the same file updates the existing Doc instead of creating a duplicate.

## Arguments

`$ARGUMENTS` is the path to the markdown file. Examples:

- `docs/research/20260520-md-to-gdoc-skill.md`
- (empty) — ask the user which file via **AskUserQuestion** or a file path prompt

## Prerequisites

The skill ships a bash script at `.claude/skills/md-to-gdoc/bin/md-to-gdoc`. Before first use the user must:

1. Have a Google Cloud project with the Drive API enabled and a **Desktop** OAuth client created.
2. Place the client JSON at `~/.config/md-to-gdoc/client.json`.
3. Run `md-to-gdoc auth` once (opens browser, captures consent, persists refresh token).

See `.claude/skills/md-to-gdoc/README.md` for the 5-minute GCP setup if the user hasn't done it.

## Instructions

### Step 1 — Validate input

1. If `$ARGUMENTS` is empty, ask the user for the file path.
2. Resolve the file: it must exist and be readable. Reject if not `.md`/`.markdown`.

### Step 2 — Check skill state

Run `.claude/skills/md-to-gdoc/bin/md-to-gdoc status`. Read the output:

- If `client.json: MISSING` — stop and point the user at `.claude/skills/md-to-gdoc/README.md` for the GCP setup. Do **not** attempt to create a client for them.
- If `token.json: MISSING` — instruct the user to run `! .claude/skills/md-to-gdoc/bin/md-to-gdoc auth` themselves (the `!` prefix runs it in their shell so the browser handoff works). Wait for them to confirm completion before continuing.
- Otherwise, proceed.

### Step 3 — Preview frontmatter

Read the markdown file (or grep the first 30 lines) and check:

- Is there a `gdoc-id:` field in YAML frontmatter? If yes, this run will **update** the existing Doc — same URL.
- If no, this run will **create** a new Doc and the script will write a `gdoc-id:` line back into the file's frontmatter so future runs update it.

Tell the user which mode this is before uploading. For updates, surface the Doc URL the existing ID resolves to (`https://docs.google.com/document/d/<gdoc-id>/edit`) so they can sanity-check it's the right target.

The Doc title is resolved in this order: (1) frontmatter `title:` field, (2) the file's first H1 (`# Heading`) with inline markdown stripped, (3) basename without `.md`. On update runs, the title is re-applied — so renaming the H1 (or adding a `title:` field) and re-running will rename the Doc.

### Step 4 — Upload

Run:

```bash
.claude/skills/md-to-gdoc/bin/md-to-gdoc upload <file>
```

The script prints the Doc's `webViewLink` on stdout and writes status messages to stderr. On success the file's frontmatter is updated in place (for the create path).

### Step 5 — Report

Show the user:
- The Doc URL (clickable).
- Whether it was a create or update.
- For creates: the fact that `gdoc-id` was added to the file's frontmatter (and that re-running will update the same Doc).

If the upload failed:
- Surface the error message verbatim. Common ones:
  - `Drive API error: ... insufficientPermissions` — the OAuth scope is `drive.file`, which only lets the script touch files it created. For an update, the existing Doc may have been created via a different method; the user will need to recreate it via this skill.
  - `Drive API error: ... notFound` — the `gdoc-id` in frontmatter no longer resolves (Doc deleted/trashed). Suggest removing the `gdoc-id` line and re-running to create fresh.
  - `token refresh failed: ... invalid_grant` — the refresh token expired (testing-mode consent screen has a 7-day TTL). Tell the user to re-run `md-to-gdoc auth`.

## Rules

- **NEVER** call `gcloud`, `gdrive`, `rclone`, `oauth2l`, or any third-party CLI from this skill — the bash script is intentionally dependency-free (curl + jq + openssl + python3 only).
- **NEVER** echo or log token contents, refresh tokens, or client secrets to the conversation.
- **NEVER** modify the markdown file other than via the script's `fm_set` path (which only touches frontmatter). Don't reformat the body.
- **ALWAYS** invoke the script via its absolute path (`.claude/skills/md-to-gdoc/bin/md-to-gdoc`) so it works regardless of the user's cwd.
- For the **auth** subcommand: do not run it from inside this session. Ask the user to run it themselves with `! md-to-gdoc auth` — that way the browser handoff lands in their shell, not a backgrounded agent process.
