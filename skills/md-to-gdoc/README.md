# md-to-gdoc — setup

A skill that converts a local markdown file to a Google Doc in your Drive, using the Drive API's native markdown conversion (rolled out July 2024). Re-running on the same file updates the existing Doc — the source `.md` tracks its Doc ID in YAML frontmatter.

## One-time setup (≈ 5 min)

You need a Google Cloud project with the Drive API enabled and a Desktop OAuth client. The skill ships no shared credentials — each user owns their own. The `drive.file` scope keeps the blast radius tight: the skill can only read/write files it created.

### 1. Create or pick a GCP project

- Open [console.cloud.google.com](https://console.cloud.google.com/).
- Top bar → project picker → **New Project**. Name it whatever you like (`md-to-gdoc` is fine). Skip the organization if you're using a personal Google account.

### 2. Enable the Drive API

- Sidebar → **APIs & Services** → **Library** → search "Google Drive API" → **Enable**.

### 3. Configure the OAuth consent screen

- Sidebar → **APIs & Services** → **OAuth consent screen**.
- User type: **External** → **Create**.
- App name: `md-to-gdoc`. User support email and developer email: yours.
- Save and continue.
- **Scopes** step → **Add or remove scopes** → search `drive.file` → tick `…/auth/drive.file` ("See, edit, create, and delete only the specific Google Drive files you use with this app") → **Update** → **Save and continue**.
- **Test users** step → **Add users** → add your own email. **Save and continue**.

> **Note on testing vs production**: the screen stays in "Testing" status. That's fine, but refresh tokens issued in testing mode **expire after 7 days** — you'll need to re-run `md-to-gdoc auth` every week. To remove this expiry, click **Publish app** on the consent screen page. The `drive.file` scope is non-sensitive, so publication is **instant and requires no Google review**.

### 4. Create the Desktop OAuth client

- Sidebar → **APIs & Services** → **Credentials** → **+ Create Credentials** → **OAuth client ID**.
- Application type: **Desktop app**.
- Name: `md-to-gdoc`.
- **Create** → on the resulting dialog, **Download JSON**.

### 5. Drop the JSON in place

```bash
mkdir -p ~/.config/md-to-gdoc
mv ~/Downloads/client_secret_*.json ~/.config/md-to-gdoc/client.json
chmod 600 ~/.config/md-to-gdoc/client.json
```

### 6. Run the OAuth bootstrap

```bash
.claude/skills/md-to-gdoc/bin/md-to-gdoc auth
```

This opens your browser, you click "Allow," and a refresh token is saved to `~/.config/md-to-gdoc/token.json`. You can verify with:

```bash
.claude/skills/md-to-gdoc/bin/md-to-gdoc status
```

You should see both `client.json: present` and `token.json: present`.

## Using the skill

### From the CLI

```bash
# Create a new Google Doc from a markdown file:
.claude/skills/md-to-gdoc/bin/md-to-gdoc upload docs/research/foo.md
# → prints https://docs.google.com/document/d/<id>/edit
# → writes `gdoc-id: <id>` into the file's frontmatter

# Update the same Doc later (just re-run — gdoc-id in frontmatter triggers update):
.claude/skills/md-to-gdoc/bin/md-to-gdoc upload docs/research/foo.md
# → same URL, contents replaced
```

### From a Claude Code session

Trigger phrases:

- "make this markdown a Google Doc"
- "share `<file>` as a gdoc"
- "/md-to-gdoc `<file>`"
- "upload `<file>` to Drive"

Claude will preview whether it's a create or an update, then run the upload and report the Doc URL.

## What gets preserved

Drive's native markdown converter (since July 2024) handles:

- ✅ Headings (`#` → H1, … `######` → H6)
- ✅ Ordered + unordered lists, nested
- ✅ Inline links `[text](url)`
- ✅ Bold, italic, strikethrough
- ✅ Inline `code` and fenced code blocks (rendered as monospace; **no syntax highlighting**)
- ✅ Pipe-syntax tables
- ⚠️ Images: external `![](https://…)` URLs are not reliably fetched; data-URL images come through broken
- ⚠️ Footnotes, bookmarks, cross-references: dropped

YAML frontmatter is stripped before upload so `gdoc-id:` etc. don't appear in the resulting Doc.

## Doc title

The script picks the Doc's name (as it appears in Drive) in this order:

1. `title:` field in YAML frontmatter (if present).
2. First H1 heading (`# Heading`) found in the body, with inline markdown stripped.
3. The filename without `.md`.

The title is applied on every run, so renaming the H1 (or adding/changing `title:` in frontmatter) and re-running will rename the existing Doc.

## Where things live

| Path | Purpose | Tracked in git? |
|------|---------|-----------------|
| `.claude/skills/md-to-gdoc/SKILL.md` | Claude-facing instructions | yes |
| `.claude/skills/md-to-gdoc/bin/md-to-gdoc` | Main bash script | yes |
| `.claude/skills/md-to-gdoc/bin/_auth_listener.py` | OAuth loopback listener | yes |
| `~/.config/md-to-gdoc/client.json` | Your OAuth client (per-user) | **no — never commit** |
| `~/.config/md-to-gdoc/token.json` | Your tokens (per-user) | **no — never commit** |

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `no client.json at …` | Re-do step 5 above. |
| `no token.json at …` | Run `md-to-gdoc auth`. |
| `token refresh failed: … invalid_grant` | Refresh token expired (7-day testing-mode limit). Re-run `md-to-gdoc auth`. To stop this happening, publish the consent screen (step 3 note). |
| `Drive API error: … notFound` on update | The `gdoc-id` in frontmatter no longer resolves (Doc deleted/trashed). Delete the `gdoc-id:` line and re-run to create a fresh Doc. |
| `Drive API error: … insufficientPermissions` on update | The Doc wasn't created via this skill, so `drive.file` scope can't touch it. Either delete the `gdoc-id:` line (create fresh) or change scope to `drive` (adds Google verification overhead). |
| Browser doesn't open during `auth` | Copy the URL printed to stderr into your browser manually. The loopback listener doesn't care how the redirect arrives, only that it lands on `127.0.0.1:<port>`. |

## Dependencies

`bash`, `curl`, `jq`, `openssl`, `python3` — all preinstalled on macOS and standard on any Linux dev box. No `pandoc`, `gdrive`, `rclone`, `gcloud`, or `oauth2l` required.
