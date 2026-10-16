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

---

## Security posture (school context)

The app is local-first and is designed to handle no pupil data, so the attack surface is small.
These are the practical things to keep in mind and fix over time.

### Already good
- **No network calls from the app.** DeckCraft (and this fork) makes no HTTP requests during normal
  use. Fonts load from disk; decks open and save locally.
- **No telemetry or analytics.** The app collects nothing.
- **Sandboxed renderer.** The render and pptx crates never read or write files; only the
  CLI/UI layer touches the filesystem.

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
| 2026-10-16 | `packaging/macos/update-local.sh` brings the installed app up to date with `main` in one command (`--check` only reports). It builds in a temporary checkout, so your own branch is untouched. This is a manual update, not automatic; in-app auto-update needs signed releases and a host (planned with the rebrand). | `feat/update-local-app` |

## Known gaps

- DeckCraft's editor draws equations as their plain-text stand-in (no fractions, layout or
  stacking). Real equation drawing is Phase 3.
- The fidelity report has not yet been run on additional real decks from Kieran's private test
  corpus; it compares imported models, not rendered pixels.
- STIX Two Math isn't bundled yet: macOS has it installed, other systems may not. Bundling it means
  adding it to a fork of `storytold/craft-fonts` (fonts never live in this repo, AGENTS.md §1.3).
