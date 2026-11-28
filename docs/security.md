## Security posture (school context)

The app is local-first and is designed to handle no pupil data, so the attack surface is small.
These are the practical things to keep in mind and fix over time.

### Already good
- **No network calls unless the teacher switches Google Drive on and signs in.** DeckCraft (and this
  fork) makes no HTTP requests during normal use. Fonts load from disk; decks open and save
  locally. The one exception is the optional Google Drive feature, below.
- **No telemetry or analytics.** The app collects nothing.
- **Sandboxed renderer.** The render and pptx crates never read or write files; only the
  CLI/UI layer touches the filesystem.

### Updates from GitHub (optional, off by default)
`crates/update` (native builds only). Off until the teacher turns on *Options ▸ Updates ▸ Check for a
newer version when DeckCraft starts*, or presses *Check for updates now…*.

| When | Host | What is sent |
| --- | --- | --- |
| Only on that switch or button | `api.github.com` | One GET for this fork's latest release. No id, no version, no account (the usual HTTP headers only). |
| Only after the teacher presses *Update now* | `github.com` (release download, which redirects to GitHub's storage) | Two GETs: the app zip and its `.sha256` file. |

The zip is kept only if its SHA-256 matches the checksum file; the new app's code signature is
checked before it replaces the old one; if any step fails the old app stays. The checksum guards
against a damaged download, not a hostile release: trust rests on HTTPS and on who can publish
releases to this repository. Downloads are only accepted from `github.com`. See `docs/updating.md`.

### Google Drive (optional, off by default)
Added in `crates/drive` (native builds only; never in the web build). What it talks to, when, and
what it sends:

| When | Host | What is sent |
| --- | --- | --- |
| Only after Preferences ▸ Use Google Drive is on **and** the teacher presses *Sign in with Google* | `accounts.google.com` (in the teacher's own browser) | The sign-in request: the OAuth client ID, a PKCE challenge, a random `state`, scope `drive.file`. The browser redirects back to a one-shot listener on `127.0.0.1` (random port, loopback only). |
| Right after sign-in, and when a token is about to expire | `oauth2.googleapis.com` | The one-time code and PKCE verifier; later the refresh token. Signing out also sends a revoke request. |
| Only when the teacher presses *Refresh list*, *Open* or *Save to Google Drive* | `www.googleapis.com` | File list / download requests; on save, the presentation as .pptx bytes plus its name. |

- **Scope is `drive.file` only:** DeckCraft sees only files it saved to Drive (in a folder it creates
  called "DeckCraft") or files the teacher explicitly picks. It cannot read the rest of a Drive.
- **Signed out on a fresh install.** Nothing contacts Google, and the keychain is not read, until the
  switch is on and the button pressed.
- **Tokens are in the system keychain** (macOS Keychain), never in `prefs.json`. Signing out deletes
  them and asks Google to revoke the grant.
- **Nothing is uploaded silently.** Uploads happen only from the *Save to Google Drive* button. A
  save to a file that changed in Drive since it was opened is refused (no overwriting a colleague).
- **Client ID is configuration**, never compiled in: `DECKCRAFT_GOOGLE_CLIENT_ID` or
  `~/.config/deckcraft/drive.toml`. Google's Desktop-app client also has a secret that cannot be
  kept confidential in an installed app; it is optional config, protected by PKCE not secrecy.
- **No pupil data** is read or sent by DeckCraft itself; what goes to Drive is the deck the teacher
  chose to save, so the rule "no pupil data in decks" applies to what is uploaded.
- Dependencies added for this: `ureq` (HTTPS client, rustls), `sha2` (PKCE), `base64`, `getrandom`,
  `keyring` (system credential store). Run `cargo audit` before sharing a build (not run in the
  overnight build: the tool is not installed on this Mac).

### To do before sharing builds
- **Apple notarization.** Without a signed, notarised `.dmg`, macOS Gatekeeper blocks the app for
  staff who don't have admin rights. Needs an Apple Developer membership (~£79/yr). Do this in
  Phase 4.
- **App sandbox entitlements.** When the app is sandboxed, file access is limited to what the
  user explicitly opens (no silently reading `/Documents`). Good default for a school app.
- **No pupil data rule.** Enforced by project rules: nothing in commits, logs or tests. The app
  only opens files the user chooses; it never crawls a directory automatically.
- **Dependency audit.** Run `cargo audit` (add `cargo-audit` to the xtask) before any build that
  goes to staff. The current dependency tree is all Rust (no npm, no Python); this is low risk
  but worth checking on each release.

### For the web build (phases W1–W4)
- **HTTPS only.** Any web hosting must serve over TLS; wasm won't load over plain HTTP in browsers.
- **CSP.** The page should set a strict Content-Security-Policy: no inline scripts, no external
  origins for scripts or fonts beyond your own domain.
- **File API.** The File System Access API (used for open/save) is gated behind a user gesture
  (click); the browser never grants silent filesystem access.
- **Relay server for live projection.** Even a tiny relay is a server you operate. Keep it
  stateless (just fan-out WebSocket messages; never log or store slide content). A hosted option
  like Cloudflare Workers (free tier) avoids running anything yourself.
- **No pupil data in the web build either.** The app renders decks; it must never send slide
  content to a third party. Make this explicit in the privacy statement before staff use it.
