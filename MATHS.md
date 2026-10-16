# DeckCraft maths edition — plan and status

A free, Mac-first PowerPoint alternative for maths teachers, forked from DeckCraft (MIT or
Apache-2.0). Later, the same for Word (WordCraft) and Excel (GridCraft). Owner: Kieran
(GitHub `K-Betts`).

**Success test:** a deck opened and saved by this app opens in PowerPoint with the same fonts,
layout and equations as the original.

Test material lives outside the repo (Kieran's `test-decks` folder). It is never edited and never
committed.

## Phases

| Phase | Work | Done when |
| --- | --- | --- |
| 0. Baseline | Build and test on the Mac; know where the PPTX code lives (`crates/pptx`). | Build and tests pass on the Mac. |
| 1. Fix the real deck | Equations in table cells, equation fallback text size/font/alignment, the slide 53 notes page. | Open-save-compare shows no lost equations; each fix has a test; Kieran confirms in PowerPoint. |
| 2. Fidelity checks | A repeatable script comparing an original deck with the saved copy; run on more real decks. | Every deck has a difference report; each difference is fixed or listed below as a known gap. |
| 3. Maths features | Equations drawn properly in the editor, LaTeX input, review upstream PR #55 (OMML editor), graphs and number lines, step-by-step reveals, exam-question layouts. | Each works in a sample lesson deck that saves and reopens in PowerPoint. |
| 4. Own app | New name and icon, ArtCraft branding removed, native Mac menu bar, signed .dmg (needs an Apple Developer account). | A colleague can install it. |
| 5. Word and Excel | Fork WordCraft and GridCraft; repeat phases 0–2. | Same fidelity reports for .docx and .xlsx. |

## Status

| Date | What | Where |
| --- | --- | --- |
| 2026-10-16 | Equations keep size, font, colour and alignment in the text fallback; equations in table cells survive saving; a14 declared on the wrapper. Real 54-slide deck: all 275 equations, positions, text, pictures and animations unchanged after a save. Kieran: "much better" in PowerPoint. | PR #1 |
| 2026-10-16 | Equation letters (x, y) draw in STIX Two Math, an open maths font, when Cambria Math isn't installed. | PR #3 |
| 2026-10-16 | Display equations are placed by their own justification (centred by default), in the editor and in the saved text fallback, even when the paragraph says left. Fixes slides 3 and 6 in DeckCraft. | `fix/display-equation-alignment` |
| 2026-10-16 | Empty notes pages survive PPTX and native-format round trips, including notes masters with slide-number placeholders. | `fix/empty-notes-page-round-trip` |
| 2026-10-16 | PPTX notes import skips an empty body placeholder instead of dropping later speaker-note text; an engine open-save test verifies the text in the saved XML. | `fix/notes-page-round-trip` |

## Known gaps

- DeckCraft's editor draws equations as their plain-text stand-in (no fractions, layout or
  stacking). Real equation drawing is phase 3.
- STIX Two Math isn't bundled yet: macOS has it installed, other systems may not. Bundling it means
  adding it to a fork of `storytold/craft-fonts` (fonts never live in this repo, AGENTS.md §1.3).
- Not yet built or run on a Mac.
