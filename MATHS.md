# DeckCraft maths edition — plan and status

A free, Mac-first PowerPoint alternative for maths teachers, forked from DeckCraft (MIT or
Apache-2.0). Later, the same for Word (WordCraft) and Excel (GridCraft). Owner: Kieran
(GitHub `K-Betts`).

**Success test:** a deck opened and saved by this app opens in PowerPoint with the same fonts,
layout and equations as the original.

Test material lives outside the repo (Kieran's `test-decks` folder). It is never edited and never
committed.

---

## Phases — fidelity and Mac app

| Phase | Work | Done when |
| --- | --- | --- |
| 0. Baseline | Build and test on the Mac; know where the PPTX code lives (`crates/pptx`). | Build and tests pass on the Mac. ✅ |
| 1. Fix the real deck | Equations in table cells, equation fallback text size/font/alignment, the slide 53 notes page. | Open-save-compare shows no lost equations; each fix has a test; Kieran confirms in PowerPoint. ✅ (equations done; notes page known gap) |
| 2. Fidelity checks | A repeatable script comparing an original deck with the saved copy; run on more real decks. | Every deck has a difference report; each difference is fixed or listed as a known gap. |
| 3. Maths features | Equations drawn properly in the editor, LaTeX input, review upstream PR #55 (OMML editor), graphs and number lines, step-by-step reveals, exam-question layouts. | Each works in a sample lesson deck that saves and reopens in PowerPoint. |
| 4. Own app | New name and icon, ArtCraft branding removed, native Mac menu bar, signed .dmg (needs an Apple Developer account). | A colleague can install it. |
| 5. Word and Excel | Fork WordCraft and GridCraft; repeat phases 0–2. | Same fidelity reports for .docx and .xlsx. |

---

## Phases — web and cross-device

| Phase | Work | Done when |
| --- | --- | --- |
| W1. wasm build | The app compiles to wasm32 and runs in a browser on desktop (egui already has an eframe web target; check `cargo build --target wasm32-unknown-unknown`). | Can open and present a deck in Chrome/Safari. |
| W2. iPad / touch | Touch input maps to pointer events; Apple Pencil draws on the slide (freehand ink → shape, or ink stays as annotation). | Teacher can draw on the iPad while the same URL projects on a Mac or Windows PC. |
| W3. Live projection | Two-tab or two-device session: a presenter URL (editable) and a viewer URL (read-only live view). WebSockets or SSE to push slide changes; no server needed beyond static hosting + a tiny relay. | Kieran can open the editor on his iPad, project the viewer on a Mac via the same URL, and have changes appear in real time. |
| W4. PWA / offline | Service worker caches the wasm bundle and the STIX font. App works without a network after first load; files are stored in the browser's origin (OPFS) or opened from local disk via the File System Access API. | Works fully offline on iPad and Mac. |

## Platform targets

| Platform | Build target | Status |
| --- | --- | --- |
| Mac | Native egui app | Local build and install: `packaging/macos/install-local.sh`. |
| Windows | Native egui app (`x86_64-pc-windows-gnu`) | Build: `packaging/windows/build-windows.sh`. Cross-compiling from Mac requires the MinGW-w64 toolchain. |
| Web | WASM (`wasm32-unknown-unknown`) | `packaging/web/package.sh`; browser startup verified, see W1 above. |
| iOS | TBD | No build target or supported workflow yet. |
| Chrome OS | Web build | Use the WASM build in Chrome. |

---

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

---

## Maths teacher use-case features (Phase 3 detail)

These are the features that would make this genuinely better than PowerPoint for a maths lesson,
roughly in priority order.

| Feature | Notes |
| --- | --- |
| **Equations drawn in the editor** | Fractions, superscripts, radicals rendered as maths, not plain text. Upstream PR #55 (OMML editor) is the starting point; coordinate with upstream before duplicating work. |
| **LaTeX / text-math input** | Type `$x^2 + y^2 = r^2$` in a text box and press Enter to insert a rendered equation. Saves as OMML so it round-trips to PowerPoint. |
| **Number lines and coordinate grids** | A shape type (saved as a drawing in the PPTX) that teachers adjust with controls (range, step, labels). |
| **Geometric constructions** | Circles, arcs, perpendicular bisectors — drawn with the pointer tool or inserted from a panel. |
| **Step-by-step reveal** | Click on a working-out step to toggle visibility; Tab cycles through steps. Saves as PowerPoint animations so it works in Slide Show. |
| **Exam-question layout templates** | A "New slide" gallery with GCSE/A-level layouts: question + marks box, answer lines, diagrams. |
| **High-contrast presenter mode** | A colour theme optimised for projectors (white-on-dark or high-contrast accents), toggled from the title bar. |
| **Colour-blind safe palettes** | The Design Ideas / theme panel suggests palettes tested against deuteranopia and protanopia. |

---

## How this work benefits WordCraft and GridCraft

When we fork WordCraft and GridCraft (Phase 5), the following work from this fork carries over
directly without duplication.

| Work done here | Carries to |
| --- | --- |
| OMML equation round-trip fixes (PRs #1, #4) | **WordCraft** — Word documents use the same OMML format. The same fixes (fallback text size/font, alignment, table cells) apply to .docx. |
| STIX Two Math / font substitution chain (PR #3) | **WordCraft** — equations in .docx use Cambria Math too. Same `fontdb.rs` substitution applies. |
| Missing-font toast suppression (fix/cambria-math-toast) | **WordCraft** — same engine code; no extra work. |
| No-spellcheck in equations (PR #5) | **WordCraft** — same engine spell checker; the `RunKind::Math` skip applies. |
| UI theme tokens and spacing (PR #6, ui/calmer-look) | **WordCraft and GridCraft** — same `crates/ui-egui/src/theme.rs`; the calmer look applies to all three apps in the suite immediately. |
| CI / branch-per-task rules (CLAUDE.md, AGENTS.md) | All forks — copy the fork rules into each repo's CLAUDE.md at setup. |
| wasm build and web phases (W1–W4) | **WordCraft and GridCraft** — the web architecture (eframe web target, relay, PWA) is the same for all three apps. Build it once in this fork and port the infrastructure. |
| Equation editor (Phase 3) | **WordCraft** — teachers write worked examples in Word documents too; LaTeX input and step-by-step reveal are useful there. |

---

## Status

| Date | What | Where |
| --- | --- | --- |
| 2026-10-16 | Equations keep size, font, colour and alignment in the text fallback; equations in table cells survive saving; a14 declared on the wrapper. Real 54-slide deck: all 275 equations, positions, text, pictures and animations unchanged after a save. Kieran: "much better" in PowerPoint. | PR #1 |
| 2026-10-16 | Equation letters (x, y) draw in STIX Two Math, an open maths font, when Cambria Math isn't installed. | PR #3 |
| 2026-10-16 | Display equations are placed by their own justification (centred by default), in the editor and in the saved text fallback, even when the paragraph says left. Fixes slides 3 and 6 in DeckCraft. | `fix/display-equation-alignment` |
| 2026-10-16 | Empty notes pages survive PPTX and native-format round trips, including notes masters with slide-number placeholders. | `fix/empty-notes-page-round-trip` |
| 2026-10-16 | PPTX notes import skips an empty body placeholder instead of dropping later speaker-note text; an engine open-save test verifies the text in the saved XML. | `fix/notes-page-round-trip` |
| 2026-10-16 | `deckcraft-cli fidelity ORIGINAL SAVED` emits a JSON model-difference report without modifying either deck. | `feat/fidelity-comparison` |
| 2026-10-16 | Equations no longer trigger spell-check red squiggles. | PR #5 |
| 2026-10-16 | UI calmer look: softer greys, blue accent, better spacing. Kieran: "working better". | PR #6 |
| 2026-10-16 | Missing-font toast no longer lists Cambria Math when STIX Two Math is installed. | PR #7 |
| 2026-10-16 | Teacher Mode (Cmd+Shift+T, View menu): fullscreen overlay with pointer, laser, pen, highlighter, shapes, eraser, clear, revealer, spotlight and countdown timers. Marks are ephemeral and never saved; they clear on slide change (timers stay). Ruler and protractor not built yet; not yet checked by eye on a projector. | PR #11 |
| 2026-10-16 | Teacher Mode polish: tools hide behind a left-edge arrow; timers are set like a board timer: big HH:MM:SS with up and down arrows on each pair and 1, 2, 5, 10 min quick buttons (5 min default) and can be dragged; Teacher Mode button in the status bar; UK English is the default proofing language. | `feature/teacher-mode-polish` |
| 2026-10-16 | Teacher Mode: eraser is now a pixel eraser by default (cuts a gap, shapes become strokes when bitten; a stroke-eraser option stays in the palette). Four fingers down at once acts as a palm wipe whatever tool is on (egui gives no contact size, so this is a finger-count approximation; untested on real touch hardware). Laser and spotlight removed from Teacher Mode. | `feature/teacher-eraser-palm` |
| 2026-10-16 | Web build (W1) now starts in the browser: fixed two startup panics (process id and font access before the first frame), and presenter view is off by default there (one screen). Checked in headless Chromium over WebGL: the sample deck, ribbon, thumbnails and notes draw. No automated test covers these wasm-only crashes; touch, iPad and Safari unchecked. Local run: `packaging/web/package.sh`, then serve `dist/web`. | `feat/web-local-build` |
| 2026-10-16 | Local Mac app: `packaging/macos/install-local.sh` builds the release app and installs it to /Applications (ad-hoc signed, runs on this Mac only; no more `cargo run`). Still named DeckCraft; a signed .dmg for other Macs needs an Apple Developer account. | `feat/local-mac-app` |
| 2026-10-16 | Teacher Mode gains an optional maths-tools panel with per-user local settings and draggable ruler, protractor, compass, number-line, coordinate-grid, fraction-wall, algebra-tile, hundred-square and calculator widgets. These aids are ephemeral and are not saved with slides. | `feat/maths-tools` |
| 2026-10-16 | `packaging/macos/update-local.sh` brings the installed app up to date with `main` in one command (`--check` only reports). It builds in a temporary checkout, so your own branch is untouched. This is a manual update, not automatic; in-app auto-update needs signed releases and a host (planned with the rebrand). | `feat/update-local-app` |
| 2026-10-16 | Added a local Windows GNU-target build script and documented the supported build targets. | `feat/update-local-app` |
| 2026-10-16 | Equations imported from PowerPoint: each equation of a display group is its own line (`y = 2x + 1` over `y = 3 − 2x`, not run together), and relations and binary operators are spaced as PowerPoint draws them. Checked on slides 10 and 12 of a real 54-slide deck against PowerPoint's own PDF export; shapes and pictures already matched. Fractions, powers and roots are not yet checked. | `fix/equation-spacing-and-breaks` |
| 2026-10-17 | UX review and build (`docs/design/ux-review.md`): neutral install with maths as an opt-in subject pack (Preferences, Home card); upstream Discord/website/source links hidden by default; Teacher Mode palette regrouped with 44pt targets and a tile-based maths panel; timer reads as big digits only while running (the HH:MM:SS row used to wrap), amber at 30 s and red at 10 s; Insert > Get Ready Starter inserts four questions as plain text boxes (answers in the notes; 20 PLACEHOLDER questions in `assets/questions/bank.json`, to be replaced by Kieran's bank); optional Google Drive (off by default, `drive.file` scope only; see Security posture). Not checked: projector, touch hardware, PowerPoint opening a Get Ready slide, Drive against real Google, `cargo audit`. Diagram questions are a follow-up. | `feat/ux-review` |
| 2026-10-17 | Get Ready starter: each question now has its own topic and level (Secure / Core / Stretch), 2 to 6 questions, a "Set all to" row, and the last choices are remembered; `slide.getReady` takes `slots`. Not checked: opening the slide in PowerPoint. | `feat/get-ready-per-question` |
| 2026-10-17 | In-app updates (Mac): *Update available* pill in the status bar and a Home card; *Update now* downloads, checks and swaps the app, then *Restart now* (refuses with unsaved work). Off by default (Options ▸ Updates). New workflow `mac-update.yml` publishes a release on a manual run. Not checked: a real update end to end (no release exists yet), Intel Macs, Windows/Linux. | `feat/in-app-update` |
| 2026-10-17 | Web version, rollout-ready: installable and offline (manifest, icons, service worker that picks up each published build the next time the app is opened online); keeps preferences (incl. subject packs) in browser local storage; starts on the Home screen with an empty deck (`?sample` for the demo); no Drive switch where there is no Drive. `docs/web-hosting.md` compares free hosts; `.github/workflows/web.yml` publishes to GitHub Pages on a manual run only. Checked: loads and runs on WebGPU in Chromium, settings survive a reload, service-worker logic in a Node test. Not checked: the service worker in a real browser (the built-in pane has none and Chrome was not connected), Safari/iPad, install on a device, a real Pages deploy. | `feat/web-pwa-pr` |
| 2026-10-17 | UI redesign, step 1 of 5 ("Fresh Paper", chosen from three directions): new colour tokens (airy neutrals, one cobalt accent, matching dark mode), rounder corners, softer shadows, a type/space/radius scale in `theme.rs`, and `docs/design/design-system.md` (the spec for the next steps: roomy floating ribbon, Home and dialogs, Teacher Mode). A test fails if text contrast drops below the promised ratios. No commands, layouts or shortcuts changed. Not done yet: the ribbon's new shape, Home, Teacher Mode. | `feat/ui-foundation` |
| 2026-10-17 | UI redesign, step 2 of 5: the ribbon is now a floating, rounded, softly shadowed card with more padding; tabs are pills (current tab white with accent text) instead of an underline; groups are separated by space instead of rules; ribbon hover washes are rounder. Home and Design tabs still fit at the default 1440pt window (font-name box slimmed). Not changed: any command or shortcut. Known limit: below about 1400pt the right-hand groups are cut off, as before; a responsive ribbon is the next ribbon step. | `feat/ui-chrome` |
| 2026-10-17 | UI redesign, step 3 of 5: Home and dialogs. The File/Home side menu is white with a soft accent pill on the current page; the page is the quiet grey with white raised cards (theme and template tiles lift on hover); the "Teaching a subject?" and update banners are rounded accent-tinted cards with a filled button; dialogs get 20pt padding, a filled accent main button and a quiet Cancel. No commands or shortcuts changed. Not done: task panes, thumbnails and notes (step 3 tail), Teacher Mode (step 4). | `feat/ui-home-dialogs` |
| 2026-10-17 | UI redesign, step 2b: the ribbon is responsive. Groups that do not fit the window fold, whole, into a **More** menu at the right of the card (a button with three dots); the menu shows them with their normal buttons. This also makes reachable some buttons that were cut off at the default 1440pt width (Home: Selection Pane and Design Ideas; Design: Slide Size, Format Background). Widths are learned as groups are drawn (no invisible layout, so no phantom controls for screen readers). Known trade-off: on Design at 1440pt, Colors/Fonts/Effects now sit in More. | `feat/ui-responsive-ribbon` |
| 2026-10-17 | Equation dialog rebuilt PowerPoint-style: live picture of the equation as you type (fractions stacked, powers raised, roots drawn), a button palette (fraction, squared, power, subscript, roots, brackets, sum, integral, symbols, Greek) that types empty boxes, Tab to move between boxes, Insert off until the boxes are filled, and plain-English typing (`x^2/3`, `sqrt(x)`, `pi`, `<=`). Inserts a real equation (OMML) that saves to PowerPoint. Double-click an equation (or Insert › Equation with it selected) to reopen it in the dialog and Update it in place; equations from PowerPoint open too. Not yet: drawing equations as maths on the slide itself (still the plain-text stand-in, Phase 3). | `insert.equation` |
| 2026-10-18 | Get Ready starter reworked: always four questions in a 2×2 grid of coloured cards (number badge, question, answer box that appears on click and is also in the notes). `$…$` LaTeX in the bank (`\frac`, `\sqrt`, powers, `\times`, units) is drawn as real equations, fractions and roots on their own centred line. Bank grown to about 70 KS3/KS4 questions across Number, Algebra, Ratio, Geometry and Statistics. Nineteen figure types are drawn from shapes (triangles, right triangles, similar triangles, angles on a line and at a point, parallel lines, circles and sectors, regular polygons, rectangle, L-shape, trapezium, parallelogram, cuboid, bar chart, number line, coordinate grid, Venn, probability tree); the bank has about 100 questions, 38 with figures. *Manage questions…* (in the Get Ready dialog) adds your own questions and removes or restores built-in ones; kept with the app's settings, no file needed. Laid out like MathsWhiteboard's starter: Last lesson / week / topic / year headings and an *Expand* button per card that opens the question full screen on its own slide (with a *Back* button; both are slide links in the saved file); Teacher Mode's pen works over it. Not built: swapping one question without redoing the slide, MathsWhiteboard's topic search. Not checked: PowerPoint opening the slide, the Manage dialog by eye, a real Slide Show click-through. Equations inside a sentence (`x²`, a fraction) are now laid out as maths by the text engine too, and survive saving and reopening. | `feat/get-ready-revamp` |
| 2026-10-18 | Question templates: a question can be a recipe (`templates` in the bank file) that picks new numbers and letters each time and works out its answer and figure, so Reroll never repeats. Nine examples ship (both-sides equations, two-step, expanding, angles on a line and in a triangle drawn to true size, Pythagoras, circle area, percentages, ratio). Each Get Ready question has a *Search a topic* box (topic, subtopic, keywords, text; shows how many match). Manage Questions can import a question file (checks it first). `deckcraft-cli bank check FILE` validates a file and prints examples. `docs/question-writing-guide.md` holds the format and a prompt for a local AI to write more. Angle and triangle figures now draw to the real sizes. Not checked: PowerPoint, the dialogs by eye, real AI-written files. | `feat/question-templates` |
| 2026-10-18 | Manage Questions: *Add questions written by an AI*: paste the AI's answer (fences and chatter are fine), Check shows three example questions with answers per recipe, orange warnings (a minus after a plus, money without two decimals, unused variables, no check, a question that never changes) and plain-English JSON errors with the line; Add puts them in the bank under fresh ids and skips repeats. Templates can carry `check` sums that must hold for every question made (a wrong answer formula is refused). Import from a file uses the same path. Not checked: the dialog by eye, a long AI file. | `feat/paste-editor` |
| 2026-10-18 | Year 7 Autumn term, all 7 blocks and 55 small steps of the White Rose scheme of learning (`WRE_Maths_v3`), 200 templates in `assets/questions/y7-autumn-*.json`: each step has a Support, Core, Stretch and Exam-style template (levels are now 1 to 4), tagged fluency / reasoning / problem and with its step as `source` (searchable). Five new figures (function machine, sequence arrows, pictorial pattern, bar model, table) and a richer number line; template filters `words`, `group`, `sgn`, `if:` and functions `sf`, `log10`. Fixed an endless loop that large numbers with rounding error could cause. Every template carries a `check` sum; a test requires all four levels for every step. For review: read the sample questions with `deckcraft-cli bank check FILE --samples 3`. Not done: Spring and Summer; exam style means original questions in the style of GCSE papers (not copied). Not checked: by a teacher, PowerPoint. | `feat/year7-autumn-steps` |
| 2026-10-18 | Skill picker: each Get Ready question has a *Choose a skill* button that opens a searchable list of skills (the small steps) with year group, term, block and how many questions exist at each level; search by name, topic, keyword or year ("fractions", "y8 angles"); year chips; *Any skill* to go back to random. A chosen skill fixes what the question is about (levels still choose the difficulty); *none at this level* is flagged. Questions and templates carry an explicit `year` (falls back to the year in `source`). Not checked: the dialogs by eye. | `feat/skill-picker` |
| 2026-10-18 | Year 7 fractions cluster (108 templates, 27 steps, four levels each): Spring *Fractions, decimals and percentages* (13 steps), Spring *Fractions and percentages of amounts* (6) and Summer *Add and subtract fractions* (8), from the White Rose scheme. New figures `fraction-bar` and `shade-grid`, and number-line tick labels. Maths inside a sentence now stays inline (stacked fractions and mixed numbers among words) instead of breaking onto its own line; only a lone fraction at the end of a question is set larger on its own line. Still to write for Year 7: Spring *Graphing data*, *Directed number*, *Perimeter and area*; Summer *Speed, distance and time*, *Properties of number*, *Angles and polygons*. | `feat/y7-fractions` |
| 2026-10-18 | Year 7 finished: Spring *Graphing data*, *Directed number*, *Perimeter and area*; Summer *Speed, distance and time*, *Properties of number*, *Angles and polygons* (all four levels per small step, so Year 7 now covers all three terms of the White Rose scheme). New figures: pictogram, scatter, line graph, polygon shape, polygon angles (true-size), ruler, factor tree, counters, crossing lines and dual/stacked bars. New template functions `isprime`, `nextprime`, `ndiv`; `\|\|` in expressions. A test checks every template makes different questions and valid numbers over 300 seeds. Next: Year 8. Not checked: by a teacher, PowerPoint. | `feat/y7-rest` |

## Known gaps

- DeckCraft's editor draws equations as their plain-text stand-in (no fractions, layout or
  stacking). Real equation drawing is Phase 3.
- The fidelity report has not yet been run on additional real decks from Kieran's private test
  corpus; it compares imported models, not rendered pixels.
- STIX Two Math isn't bundled yet: macOS has it installed, other systems may not. Bundling it means
  adding it to a fork of `storytold/craft-fonts` (fonts never live in this repo, AGENTS.md §1.3).
| 2026-10-17 | UI redesign, step 4 of 5: Teacher Mode on the design system. Palette, timer and maths tiles use named ink, accent, danger and success colours instead of one-off RGB values, 28pt rounded floating surfaces, the accent for the active tool and quick-time chip, a darker success green so white text stays readable. Touch targets stay 44pt. No behaviour change. Checked in the app (palette) and a rendered test (timer); not checked on a projector, interactive board or touch screen. | `feat/ui-teacher-mode` |
| 2026-10-17 | UI redesign, step 5 of 5 (polish): DeckCraft now follows the computer's light/dark setting by default; File > Account and Preferences have Appearance: Match this computer / Light / Dark (new command `view.appearance`; `view.dark` still works and pins the choice). Dark-mode pass fixed unchecked checkboxes, text fields and slider rails that disappeared (also in light), and low-contrast text on dark accent buttons. Checked in the app with screenshots of editor, Home and Preferences in dark and Preferences in light; the system-follow logic is tested by feeding the theme to the app, not by switching macOS. Not done: icon redesign, Figtree, ribbon group labels, dark Teacher/maths panel comparison. | `feat/ui-polish` |
| 2026-10-18 | Year 8 Autumn term, all 6 blocks (*Ratio*, *Proportion and scale*, *Algebraic manipulation*, *Coordinates and graphs*, *Multiply and divide fractions*, *Symmetry and reflection*): 52 small steps, 216 templates in `assets/questions/y8-autumn-*.json`, four levels each, ids `y8au…`. The coordinate grid can now draw several lines (including `x = a`, `y = b`, dashed mirror lines), segments and coloured polygons, so reflection and graph questions have real pictures. The coordinate grid labels every 2nd number on wide grids; graph titles no longer sit on the plot. Next: Year 8 Spring and Summer. Not checked: by a teacher, PowerPoint. | `feat/y8-autumn` |
| 2026-10-18 | Year 8 Spring term, all 6 blocks (*Area, volume and density*, *Equations and inequalities*, *Percentages*, *Indices*, *Standard form*, *Interpret and represent data*): 52 small steps, 204 templates in `assets/questions/y8-spring-*.json` (ids `y8sp…`), four levels each. Number lines can show two-sided inequalities; tables size their columns to the text (long class intervals are written down the page). Next: Year 8 Summer. Not checked: by a teacher, PowerPoint. | `feat/y8-spring` |
