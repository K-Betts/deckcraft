# DeckCraft maths edition — plan and status

A free, Mac-first PowerPoint alternative for maths teachers, forked from DeckCraft (MIT or
Apache-2.0). Later, the same for Word (WordCraft) and Excel (GridCraft). Owner: the repository owner
(GitHub `K-Betts`).

**Success test:** a deck opened and saved by this app opens in PowerPoint with the same fonts,
layout and equations as the original.

Test material lives outside the repo (the owner's `test-decks` folder). It is never edited and never
committed.

---

## Phases (rebuilt 2026-10-24, revised 2026-10-25)

Rebuilt from an audit of what exists. The phase table was written for the owner's own use; since
2026-11-27 next term's users are the maths department (see the audit section). The original phase tables are summarised under "Done so far"; their
detail is in the [status log](docs/maths-history.md).

**Decision 2026-10-25: the app stays called DeckCraft.** The EduDeck rebrand is cancelled. Its
code commits (stages 1–12, `d90694c` to `2f813ad`) were undone in `39cad33`; the history is kept.
The rebrand plan (`PLAN-rebrand-full.md`) was moved to the archive folder outside the repo; it is still in git history.

| Phase | Work | Done when | State |
| --- | --- | --- | --- |
| E1. Rebrand, part 1 | Cancelled on 2026-10-25 (staying DeckCraft). | | Cancelled |
| E2. Rebrand, part 2 | Cancelled on 2026-10-25 (staying DeckCraft). | | Cancelled |
| E3. PowerPoint fidelity | Run `deckcraft-cli fidelity` on more of the owner's real decks (copies only); fix each difference or list it as a known gap. Old Phase 2. | Every deck in the test folder has a difference report; each difference is fixed or listed; the owner confirms in PowerPoint. | Tool built; not run on more decks |
| E4. iPad and touch | Touch maps to pointer events; Apple Pencil draws on the slide; palm wipe checked on real hardware. Old W2. | The owner draws on the iPad (web app) with Pencil and fingers during a lesson. | Partly built, unchecked on hardware |
| E5. Live projection | Presenter URL (editable) and viewer URL (read-only live view) through a tiny stateless relay; no slide content stored or logged. Old W3. | The owner edits on the iPad and the board shows the change in real time. | Not started |
| Later | Editable number lines and coordinate grids as slide shapes; high-contrast projector mode; colour-blind palettes (rest of old Phase 3). | | Not started |
| Later | Sharing with colleagues: signed, notarised `.dmg` (Apple Developer account), app sandbox, `cargo audit`, privacy statement (rest of old Phase 4). | | Not started |
| Later | Word and Excel: fork WordCraft and GridCraft, repeat E1–E3 for `.docx` and `.xlsx` (old Phase 5). | | Not started |

**Done so far (old phases):** baseline build (0); the real deck's equations and notes (1); web build
in the browser (W1); installable, offline web app (W4); Teacher Mode with maths tools; Get Ready
starters (about 3,700 question templates, Years 7 to 11, curriculum mode); equation dialog with a
live preview; Reveal Step by Step; Exam Question slides; local Mac install and in-app updates.

---

## Audit, 2026-11-27: proposed next direction

The owner's answers: the users for next term are **the maths department**, and what has stopped real
use is **files looking wrong** and **crashes or bugs**. The web app has been
confirmed to work on a school laptop. The order below is a proposal until the owner agrees it.

1. **Stop the known crash.** PR #97 (Save As on macOS) is merged and the Mac app was rebuilt with it on 27 November.
   Not yet tried by hand.
2. **Make delivery work.** GitHub Actions refuses to run on this fork, so merges publish nothing.
   Until that is fixed, publish by hand and check what version teachers actually get.
3. **Protect what teachers type.** Back up the private server's data before inviting colleagues.
4. **Narrow what colleagues are asked to do.** Web app: make a Get Ready or exam slide, download
   it, open it in PowerPoint. Not yet: editing the department's existing decks.
5. **Branding.** Decided 27 November: keep the ArtCraft branding for now.
6. **Prove it on school machines.** Open those files in PowerPoint on Windows, then one colleague
   for two weeks, with each failure written down. Fix only those.
7. **Fidelity (E3)** on copies of real decks, run locally, reporting counts only. Running the same
   decks through this build and an upstream build also informs the open `craft-pptx` decision.

Paused until the pilot ends: new features, iPad (E4), live projection (E5), Word and Excel, and a
paid Apple account. Upstream merges continue (many are PowerPoint fidelity fixes), but not in the
two weeks before the pilot starts.

---

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

The security notes for the school context are in [docs/security.md](docs/security.md).

---

## Status

Full status log, one row per landed task: [docs/maths-history.md](docs/maths-history.md) (search it; do not read it whole).

- Teacher Mode with maths tools, and Get Ready starters (about 3,700 question templates, Years 7 to 11, in curriculum mode).
- Equations keep their size, font and colour, and survive a save: a real 54-slide deck came back with all 275 equations unchanged (2026-10-16).
- Equation dialog with a live preview, and Reveal Step by Step.
- Exam Question slides and the Question Library, on Insert (Maths pack), on the Mac and the web.
- Web build runs in the browser and installs as an offline web app; a private copy runs on a Raspberry Pi behind a staff password.
- Local Mac install (`packaging/macos/install-local.sh`) with in-app updates.

## Known gaps

- The fidelity report has not yet been run on additional real decks from the owner's private test
  corpus; it compares imported models, not rendered pixels.
- STIX Two Math isn't bundled yet: macOS has it installed, other systems may not. Bundling it means
  adding it to a fork of `storytold/craft-fonts` (fonts never live in this repo, AGENTS.md §1.3).
