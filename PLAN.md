# PLAN.md — DeckCraft maths edition: high-contrast projector theme

> Written by Claude for a local model (Qwen 3.6 35B in OpenCode via LM Studio).
> The local model follows this plan exactly, one stage at a time.
> The app stays called **DeckCraft**. The EduDeck rebrand was cancelled on 2026-10-25 (see MATHS.md).

---

## 0. Rules for the local model (read first, every stage)

1. Do **only the stage you are told to do**. Never start the next stage.
2. Only edit the files named under **Files** for that stage. If another file seems to need changing, stop and report it.
3. Do not rename, move or delete files. Do not add dependencies.
4. Do not refactor or tidy code outside the stage.
5. Before the first edit of a stage, run the stage's **Already done?** command. If it says the work is there, report `STATUS: ALREADY DONE` and stop.
6. **Edit only with the exact text a stage gives.** The Find text appears once in the file. Never retype, move or delete the lines around it.
7. After every edit, run the stage's **diff check** and compare it with the numbers given. If they differ, run `git checkout -- <that file>` and do the step again. Maximum 3 attempts at any step.
8. If anything is unclear, or a check fails 3 times: **stop** and report the exact output. Do not guess.
9. **STATUS: PASSED only if** the check output contains `test result: ok` and `0 failed`.
10. Stop if you go round in circles: the same command giving the same result twice, or more than 15 tool calls in one stage. Report what you saw.
11. **Commit each stage yourself, but only if every check matched** (the already-done result, each diff count, the test result). Use the exact commit command in the stage. Never use `git add -A`, `git add .`, `git commit -a`, `--amend`, `git push`, `git reset`, `git checkout` (except rule 7), or `git stash`. After committing, `git status --short` must print nothing.
12. **If you do anything differently from this plan, say so**, even if it worked. Put it under DIFFERENT FROM THE PLAN in the report; write "None" if true.
13. **Report format** (after each stage):

```
STAGE: [number and title]
STATUS: PASSED / FAILED / STOPPED / ALREADY DONE
FILES CHANGED: [list]
MUST-FAIL RESULT: [the FAILED line and the panic message, or "not applicable"]
CHECK OUTPUT: [the final test result line]
COMMIT: [the short hash from `git log --oneline -1`, or "not committed"]
GIT STATUS AFTER COMMIT: [empty, or what it printed]
DIFFERENT FROM THE PLAN: [or "None"]
PROBLEMS / QUESTIONS: [or "None"]
```

---

## 1. Goal

**What we are building:** a built-in theme called **High Contrast** for projectors: pure black text on pure white, and strong accent colours. Teachers can pick it in the theme list like any other theme.

**Why:** a projector washes out pale colours; the default themes have softer greys.

**Done means:** the High Contrast theme appears in the theme list, its text is at least 7:1 contrast on its background, its accents are at least 4.5:1, and all existing tests still pass.

**Out of scope (do not do):**
- A title-bar switch for projector mode (a later stage; it needs a design decision).
- Colour-blind palettes (a later stage).
- Changing any existing theme.

---

## 2. Project context

**Language / framework:** Rust 2024, egui 0.36, a Cargo workspace.
**Folder:** `/Users/kieranbetts/maths-office/deckcraft-fix`
**Build:** `cargo build -p deckcraft`
**Model tests:** `cargo test -p deckcraft-model`
**UI tests:** `cargo test -p deckcraft-ui-egui`

| Path | What it contains |
|---|---|
| `crates/model/src/theme.rs` | `builtin_themes()` (about line 178): the list of built-in themes, each made by a `t(...)` block. The last entry is "Slate". |
| `crates/model/src/tests.rs` | Model tests. Uses `use super::*;`. |
| `crates/color/src/lib.rs` | `Rgba` (with `contrast`, which returns the WCAG contrast ratio as an `f64`) and `SchemeSlot` (with `Dk1`, `Lt1`, `Accent1`…). |

**Useful facts:**
- `ColorScheme::get(slot)` returns the `Rgba` for a slot. A theme's colours are in `theme.colors`.
- A theme's name is the `name` field of the `Theme` in its `t(...)` block.
- The theme list in the UI reads `builtin_themes()`, so a new theme appears there with no UI change.

---

## 3. Design decisions (already made — do not change)

- The new theme goes **before** "Slate" in the list, so "Slate" stays the last theme.
- Text colour is pure black `000000` on pure white `FFFFFF`.
- Font: Arial for both headings and body.
- The 12 colours, in this order: Dk1, Lt1, Dk2, Lt2, Accent1 to Accent6, Hlink, FolHlink:

```
["000000", "FFFFFF", "000000", "FFFFFF", "0047AB", "B33A00", "006B3C", "6A1B9A", "A4123B", "8A6100", "0047AB", "6A1B9A"]
```

---

## 4. Stages

### Stage 1 — Add the High Contrast theme and test it

**Goal:** `builtin_themes()` includes "High Contrast", and a test proves its contrast.

**Files:**
- `crates/model/src/theme.rs` — edit: add one theme block.
- `crates/model/src/tests.rs` — add one test (by a shell command).

**Steps:**

0. Already done? Run:
   ```bash
   grep -c '"High Contrast",' crates/model/src/theme.rs
   ```
   If it prints `1`, report `STATUS: ALREADY DONE` and stop. If `0`, carry on.

1. Edit `crates/model/src/theme.rs`.
   Find (it appears once; the line before `"Slate",` is `        t(`):
   ```
           t(
               "Slate",
   ```
   Replace with (the new block, then the same two lines again):
   ```
           t(
               "High Contrast",
               scheme(
                   "High Contrast",
                   ["000000", "FFFFFF", "000000", "FFFFFF", "0047AB", "B33A00", "006B3C", "6A1B9A", "A4123B", "8A6100", "0047AB", "6A1B9A"],
               ),
               "Arial",
               "Arial",
           ),
           t(
               "Slate",
   ```
   Then `git diff --stat -- crates/` must show: `1 file changed, 10 insertions(+)`.

2. Add the test. **Do not use the edit tool.** Run exactly:
   ```bash
   cat >> crates/model/src/tests.rs <<'EOF'

   #[test]
   fn high_contrast_theme_text_is_at_least_seven_to_one() {
       let t = theme::builtin_themes().into_iter().find(|t| t.name == "High Contrast").expect("the High Contrast theme exists");
       let paper = t.colors.get(SchemeSlot::Lt1);
       let ink = t.colors.get(SchemeSlot::Dk1);
       assert!(ink.contrast(paper) >= 7.0, "text on paper is {:.1}:1", ink.contrast(paper));
       for slot in [SchemeSlot::Accent1, SchemeSlot::Accent2, SchemeSlot::Accent3, SchemeSlot::Accent4] {
           let c = t.colors.get(slot);
           assert!(c.contrast(paper) >= 4.5, "{slot:?} on paper is {:.1}:1", c.contrast(paper));
       }
   }
   EOF
   ```
   Then `git diff --stat -- crates/` must show: `2 files changed, 21 insertions(+)` (10 in theme.rs, 11 in tests.rs).

   **If the compiler says `SchemeSlot` is not found:** stop and report it. Do not add an import; a person will fix the import path.

3. Run the test. It **passes** here (unlike the rebrand stages, the theme is new, so there is no "must fail" step):
   ```bash
   cargo test -p deckcraft-model high_contrast_theme
   ```
   Expected: `high_contrast_theme_text_is_at_least_seven_to_one ... ok` and `test result: ok. 1 passed; 0 failed`.
   If it fails with `text on paper is …:1` or an accent message, stop and report the exact numbers. Do not change the colours.

4. `cargo fmt -p deckcraft-model`. Diff stat must not change.

**Check:** `cargo test -p deckcraft-model`

**Success looks like:** `test result: ok.` and `0 failed`, and the list includes `high_contrast_theme_text_is_at_least_seven_to_one ... ok`. Also run `cargo test -p deckcraft-ui-egui backstage` and paste its `test result:` line: it must be `ok` with 0 failed (the New page and theme list tests run against the new theme list).

**Commit** (only if every check above matched):
```bash
git add crates/model/src/theme.rs crates/model/src/tests.rs && git commit -m "High Contrast theme for projectors, with a contrast test"
```
Then `git status --short` must print nothing.

**Watch out for:**
- The Find text must match the file exactly, including the 8, 12 and 16 leading spaces. Copy it as given.
- Do not add the theme after "Slate". Slate must stay the last entry.
- Do not change any other colour or any existing theme.

---

## 5. Final check (after the last stage)

- [ ] `cargo test -p deckcraft-model` passes
- [ ] `cargo test -p deckcraft-ui-egui` passes
- [ ] `cargo clippy -p deckcraft-model -p deckcraft-ui-egui --all-targets -- -D warnings` shows no warnings
- [ ] Manual (Kieran): Design tab ▸ Themes shows High Contrast; on a projector it reads clearly

---

## 6. Progress log (update after each stage)

| Stage | Status | Commit | Notes |
|---|---|---|---|
| 1 High Contrast theme and test | Not started | | |

---

## 7. Not in this plan (needs Kieran's decision first)

| Item | Why it is not planned |
|---|---|
| E3 PowerPoint fidelity on more real decks | Needs Kieran's private test decks, and a person to judge the output in PowerPoint. |
| E4 iPad and touch | Needs real hardware. |
| E5 Live projection (relay) | Needs a design decision about the relay server and what may be stored. |
| Projector-mode switch in the title bar | Needs a design choice about where it sits. |
| Colour-blind palettes | Needs the palettes chosen and tested by a person. |
| Number lines and coordinate grids as slide shapes | Large; needs the design first. |
| Sharing: signed `.dmg`, sandbox, privacy statement | Needs an Apple Developer account. |
| Word and Excel forks | Later phase. |
