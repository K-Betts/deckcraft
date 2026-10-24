# PLAN.md — DeckCraft Maths Edition: the About box says EduDeck

> Written by Claude for a local model (running in OpenCode via LM Studio).
> The local model must follow this plan exactly, one stage at a time.

---

## 0. Rules for the local model (read first, every stage)

1. Do **only the stage you are told to do**. Never start the next stage.
2. Only edit the files listed under **Files** for that stage. If another file seems to need changing, stop and report it instead.
3. Do not rename, move or delete files unless the stage says so.
4. Do not add new dependencies (crates, packages) unless the stage says so.
5. Keep changes small. Do not refactor or "tidy up" code outside the stage.
6. When finished, run the stage's **Check** command.
7. If the check fails, read the error, fix it, and run the check again. **Maximum 3 attempts.**
8. After 3 failed attempts, or if anything in this plan is unclear: **stop** and write a short report (what you tried, the exact error). Do not guess.
9. Do not repeat your plan back to me. Make the changes, then summarise them.
10. End every stage with the **Stage report** (format in section 6).
11. **Never find-and-replace `DeckCraft` across files.** Change only the exact lines a stage names. Most `DeckCraft` text in this project must stay for now.
12. **Start each stage in a new OpenCode session** (ctrl+p → new session). Never continue an earlier stage's session.
13. **Edit only with the exact text the stage gives.** When a step says "Find" and "Replace with", the Find text appears once in the file; replace only that. Never retype, move or delete the lines around it. When a step gives a shell command, run it exactly as written.
14. **After every edit, run `git diff --stat -- crates/`** and compare it with the numbers the stage gives. If they differ, run `git checkout -- <that file>` and do the step again.
15. **STATUS: PASSED only if** the check output contains `test result: ok` and `0 failed`. Paste that line into CHECK OUTPUT. Always use the report format in section 6.
16. **Trust the progress log (section 7).** A stage marked Done is finished: never redo it. If you are asked for a stage that is not in section 4, stop and report `STATUS: STOPPED (no such stage)`.
17. **Every stage starts with an "already done?" command.** If it says the work is already there, report `STATUS: ALREADY DONE` and stop.
18. **Stop if you are going round in circles.** If you run the same command twice and get the same result, or you have used 15 tool calls in this stage, stop and report what you saw.
19. **Do not commit.** Claude checks your work and commits it.

---

## 1. Goal

**What we are building:** Help ▸ About shows the name **EduDeck** and the line "Based on DeckCraft by the ArtCraft Team".

**Why:** It is the first, smallest step of renaming the app to EduDeck, and it shows whether the local model can do a stage of this project well.

**Done means:** the new test `about_shows_edudeck_and_credits_deckcraft` passes, all the other tests in the `backstage` group still pass, and Kieran sees "EduDeck" in Help ▸ About.

**Out of scope (do not do):**
- Changing "DeckCraft" anywhere a stage does not name (menus, other dialogs, the title drawn on the toolbar, the theme called DeckCraft, folders, file names).
- Changing the logo, links or anything else in the About box.

---

## 2. Project context

**Language / framework:** Rust (edition 2024), egui/eframe 0.36, a Cargo workspace.

**Folder:** `/Users/kieranbetts/maths-office/deckcraft-fix`

**How to build:** `cargo build -p deckcraft`

**How to run tests:** `cargo test -p deckcraft-ui-egui backstage` (only the `backstage` tests, which takes a few minutes)

**Full check (used at the end of every stage):** given in each stage.

**Key folders and files:**

| Path | What it contains |
|---|---|
| `crates/engine/src/lib.rs` | The engine crate's list of modules (`pub mod …;` lines near the top) |
| `crates/engine/src/brand.rs` | **New in stage 1**: the app's name |
| `crates/ui-egui/src/dialogs.rs` | All dialogs, including the About box (`pub fn about`, around line 3749) |
| `crates/ui-egui/src/backstage_tests.rs` | UI tests for the File menu pages and dialogs |

**Conventions to follow:**
- No `unwrap()` outside tests.
- Tests in the engine go in a `#[cfg(test)] mod tests { use super::*; … }` at the bottom of the file.
- After editing, run `cargo fmt -p <package>`. Never reformat by hand.

**Existing code to reuse (do not rewrite):**
- `harness(app)` and `new_app()` in `backstage_tests.rs`: build the app for a test.
- `h.state_mut().run("app.about", json!({}))`: opens the About box in a test.

---

## 3. Design decisions (already made — do not change)

- The name lives in one place, `deckcraft_engine::brand`, so later stages can reuse it.
- The About box credits the original project: "Based on DeckCraft by the ArtCraft Team".
- Exact code:

```rust
// crates/engine/src/brand.rs  (the complete file)
//! The app's name as teachers see it.

/// The display name.
pub const APP_NAME: &str = "EduDeck";

/// The credit line in the About box.
pub const BASED_ON: &str = "Based on DeckCraft by the ArtCraft Team";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_is_edudeck_and_deckcraft_is_credited() {
        assert_eq!(APP_NAME, "EduDeck");
        assert!(BASED_ON.contains("DeckCraft"));
    }
}
```

```rust
// crates/ui-egui/src/backstage_tests.rs  (add at the very end of the file)
#[test]
fn about_shows_edudeck_and_credits_deckcraft() {
    let mut h = harness(new_app());
    h.state_mut().run("app.about", json!({})).unwrap();
    h.run_steps(3);
    assert!(h.query_all_by_label("EduDeck").next().is_some(), "the name is shown");
    assert!(h.query_by_label_contains("Based on DeckCraft").is_some(), "the original is credited");
}
```

---

## 4. Stages

### Stage 1 — Add the name

**Goal:** The engine has a `brand` module holding the name EduDeck.

**Files:**
- `crates/engine/src/brand.rs` — create, with exactly the content in section 3.
- `crates/engine/src/lib.rs` — edit: add one line.

**Steps:**
1. Create `crates/engine/src/brand.rs` with exactly the code in section 3.
2. In `crates/engine/src/lib.rs`, find the line `pub mod cmd;` (near line 10). Add `pub mod brand;` on a new line **directly above it**.
3. Run `cargo fmt -p deckcraft-engine`.

**Check:** `cargo test -p deckcraft-engine brand`

**Success looks like:** the output shows `test brand::tests::the_name_is_edudeck_and_deckcraft_is_credited ... ok`.

**Watch out for:**
- Write `pub mod brand;`, not `mod brand;`. Stage 2 uses it from another crate.
- Put the line with the other `pub mod` lines at the top, not near the `mod tests` lines at the bottom of `lib.rs`.

---

### Stage 2 — Show it in the About box

**Goal:** Help ▸ About says EduDeck and credits DeckCraft, and a test proves it.

**Files:**
- `crates/ui-egui/src/dialogs.rs` — edit: two lines in `pub fn about`.
- `crates/ui-egui/src/backstage_tests.rs` — edit: add one test.

**Steps:** (do them in this order; check `git diff --stat -- crates/` after each)

1. Edit `crates/ui-egui/src/dialogs.rs`.
   Find (exactly, 8 spaces at the start):
   ```
           ui.label(egui::RichText::new("DeckCraft").font(theme::bold(22.0)));
   ```
   Replace with:
   ```
           ui.label(egui::RichText::new(deckcraft_engine::brand::APP_NAME).font(theme::bold(22.0)));
   ```
   Then `git diff --stat -- crates/` must show: `1 file changed, 1 insertion(+), 1 deletion(-)`.

2. Edit `crates/ui-egui/src/dialogs.rs` again.
   Find (exactly):
   ```
           ui.label("An open-source, clean-room project. Not affiliated with Microsoft.");
   ```
   Replace with these two lines:
   ```
           ui.label("An open-source, clean-room project. Not affiliated with Microsoft.");
           ui.label(egui::RichText::new(deckcraft_engine::brand::BASED_ON).color(t.text_dim));
   ```
   Then `git diff --stat -- crates/` must show: `1 file changed, 2 insertions(+), 1 deletion(-)`.

3. Add the test. **Do not use the edit tool for this.** Run this exact command, which adds it to the end of the file:
   ```bash
   cat >> crates/ui-egui/src/backstage_tests.rs <<'EOF'

   #[test]
   fn about_shows_edudeck_and_credits_deckcraft() {
       let mut h = harness(new_app());
       h.state_mut().run("app.about", json!({})).unwrap();
       h.run_steps(3);
       assert!(h.query_all_by_label("EduDeck").next().is_some(), "the name is shown");
       assert!(h.query_by_label_contains("Based on DeckCraft").is_some(), "the original is credited");
   }
   EOF
   ```
   Then `git diff --stat -- crates/` must show: `2 files changed, 11 insertions(+), 1 deletion(-)`.
   If `backstage_tests.rs` shows any deletion (`-`), run `git checkout -- crates/ui-egui/src/backstage_tests.rs` and run the command again.

4. Run `cargo fmt -p deckcraft-ui-egui`.

**Check:** `cargo test -p deckcraft-ui-egui backstage`

**Success looks like:** the last `test result:` line says `ok` and `0 failed`, and the list includes `about_shows_edudeck_and_credits_deckcraft ... ok`.

**Watch out for:**
- Stage 1 is already done. Do not touch `crates/engine/` in this stage.
- `dialogs.rs` has many other `"DeckCraft"` strings (the theme list, dialog titles). Leave all of them.
- The test checks the Account page for the text "About DeckCraft". That still passes, because you did not change it. Don't change it.
- If the compiler says `t` is not found, stop and report. Do not add it yourself.

---

### Stage 3 — The window's name

**Goal:** the Mac window is named EduDeck (what you see in Mission Control and the Window menu).

**Files:**
- `apps/deckcraft/src/main.rs` — edit: one line.

**Steps:**

0. Already done? Run:
   ```bash
   grep -c 'with_title(deckcraft_engine::brand::APP_NAME)' apps/deckcraft/src/main.rs
   ```
   If it prints `1`, report `STATUS: ALREADY DONE` and stop. If it prints `0`, carry on.

1. Edit `apps/deckcraft/src/main.rs`.
   Find (exactly, 12 spaces at the start; it appears once):
   ```
               .with_title("DeckCraft")
   ```
   Replace with:
   ```
               .with_title(deckcraft_engine::brand::APP_NAME)
   ```
   Then `git diff --stat -- apps/ crates/` must show: `1 file changed, 1 insertion(+), 1 deletion(-)`.

2. Run `cargo fmt -p deckcraft`. The diff stat must still be the same.

**Check:** `cargo check -p deckcraft && grep -c 'with_title(deckcraft_engine::brand::APP_NAME)' apps/deckcraft/src/main.rs`

**Success looks like:** `cargo check` ends with `Finished` and no `error`, and the grep prints `1`. (This stage has no test of its own, because a test cannot open the real Mac window. In CHECK OUTPUT, paste the `Finished` line and the `1`.)

**Watch out for:**
- `main.rs` has other `"DeckCraft"` strings (a folder name on line ~65, a file-type filter on line ~115, the app name near line ~247). Leave all of them. Only the `.with_title(` line changes.
- Do not add a `use` line. The full path `deckcraft_engine::brand::APP_NAME` works as it is.
- Do not touch `crates/`. The title drawn on the toolbar is a later stage.

---

### Stage 4 — The Presenter View window's name

**Goal:** during a slide show, the Presenter View window is called "Presenter View — EduDeck".

**Files:**
- `crates/ui-egui/src/show.rs` — edit: one line.

**Steps:**

0. Already done? Run:
   ```bash
   grep -c 'Presenter View — {}", deckcraft_engine::brand::APP_NAME' crates/ui-egui/src/show.rs
   ```
   If it prints `1`, report `STATUS: ALREADY DONE` and stop. If it prints `0`, carry on.

1. Edit `crates/ui-egui/src/show.rs`.
   Find (it appears once; copy it exactly — the dash is a long dash `—`, not `-`):
   ```
   .with_title("Presenter View — DeckCraft")
   ```
   Replace with:
   ```
   .with_title(format!("Presenter View — {}", deckcraft_engine::brand::APP_NAME))
   ```
   Change only those characters. The rest of that line (`egui::ViewportBuilder::default()` before it and `.with_inner_size([1100.0, 700.0]),` after it) stays as it is.
   Then `git diff --stat -- apps/ crates/` must show: `1 file changed, 1 insertion(+), 1 deletion(-)`.

2. Run `cargo fmt -p deckcraft-ui-egui`. This splits the line into three, which is expected.
   Then `git diff --stat -- apps/ crates/` must show: `1 file changed, 3 insertions(+), 1 deletion(-)`.

**Check:** `cargo check -p deckcraft-ui-egui && grep -c 'Presenter View — {}", deckcraft_engine::brand::APP_NAME' crates/ui-egui/src/show.rs`

**Success looks like:** `cargo check` ends with `Finished` and no `error`, and the grep prints `1`. (No test of its own, like Stage 3: a test cannot open the real window. In CHECK OUTPUT, paste the `Finished` line and the `1`.)

**Watch out for:**
- `show.rs` mentions "Presenter View" in other places (labels and comments). Change only the `.with_title(` one.
- Use `format!(…)`, exactly as given. `"Presenter View — " + APP_NAME` does not compile in Rust.
- Do not add a `use` line. Do not touch `apps/`.

---

### Stage 5 — The name on File ▸ Home and File ▸ Account

**Goal:** the big name next to the logo on the File ▸ Home page and on the File ▸ Account page says EduDeck, and a test proves it.

This stage is a little bigger: a test first, then one change in two places. Do the steps in order.

**Files:**
- `crates/ui-egui/src/backstage_tests.rs` — add one test (by a shell command).
- `crates/ui-egui/src/backstage.rs` — edit: the same line, in two places.

**Steps:**

0. Already done? Run:
   ```bash
   grep -c 'RichText::new(deckcraft_engine::brand::APP_NAME).font(theme::bold(24.0))' crates/ui-egui/src/backstage.rs
   ```
   If it prints `2`, report `STATUS: ALREADY DONE` and stop. If it prints `0`, carry on. Any other number: stop and report it.

1. Add the test. **Do not use the edit tool for this.** Run this exact command, which adds it to the end of the file:
   ```bash
   cat >> crates/ui-egui/src/backstage_tests.rs <<'EOF'

   #[test]
   fn file_home_and_account_show_the_edudeck_name() {
       let mut h = harness(new_app());
       for p in [Page::Home, Page::Account] {
           h.state_mut().run("app.backstage", json!({"page": p.id()})).unwrap();
           h.run_steps(3);
           assert_eq!(page(&h), Some(p));
           assert!(h.query_all_by_label("EduDeck").next().is_some(), "{} shows EduDeck", p.label());
       }
   }
   EOF
   ```
   Then `git diff --stat -- apps/ crates/` must show: `1 file changed, 11 insertions(+)`.

2. Run the new test. **It must fail at this point** — that proves it tests something:
   ```bash
   cargo test -p deckcraft-ui-egui file_home_and_account
   ```
   Expected: `test result: FAILED. 0 passed; 1 failed` and the message `Home shows EduDeck`.
   If it passes instead, or fails with a different message (for example a compile error), stop and report the output.

3. Edit `crates/ui-egui/src/backstage.rs`. This line appears **exactly twice** (once on the Home page, once on the Account page), and both must change. Use the edit tool's **replace all** option.
   Find:
   ```
   ui.label(RichText::new("DeckCraft").font(theme::bold(24.0)).color(t.text));
   ```
   Replace with:
   ```
   ui.label(RichText::new(deckcraft_engine::brand::APP_NAME).font(theme::bold(24.0)).color(t.text));
   ```
   Then run the step 0 command again: it must print `2`.
   And `git diff --stat -- apps/ crates/` must show: `2 files changed, 13 insertions(+), 2 deletions(-)`.

4. Run `cargo fmt -p deckcraft-ui-egui`. The diff stat must stay the same.

**Check:** `cargo test -p deckcraft-ui-egui backstage`

**Success looks like:** the last `test result:` line says `ok. 21 passed; 0 failed`, and the list includes `file_home_and_account_show_the_edudeck_name ... ok`.

**Watch out for:**
- Step 2 failing is correct. Do not "fix" the test. Move on to step 3.
- `backstage.rs` has other `"DeckCraft"` text (a theme name near line 549, comments). Change only the two `ui.label(RichText::new("DeckCraft")` lines.
- If your edit tool has no replace-all option, do the same edit twice, then check that the step 0 command prints `2`.
- The test file already has a test that looks for "About DeckCraft" on the Account page. It must still pass. Don't change it.

---

## 5. Final check (after the last stage)

- [ ] `cargo test -p deckcraft-engine brand` passes
- [ ] `cargo test -p deckcraft-ui-egui backstage` passes
- [ ] `cargo clippy -p deckcraft-engine -p deckcraft-ui-egui --all-targets -- -D warnings` shows no warnings
- [ ] Manual test (Kieran): `cargo run -p deckcraft`, then Help ▸ About shows "EduDeck" and "Based on DeckCraft by the ArtCraft Team"
- [ ] Manual test (Kieran): with the app running, the Window menu and Mission Control show "EduDeck"

---

## 6. Stage report format (the local model fills this in after each stage)

```
STAGE: [number and title]
STATUS: PASSED / FAILED / STOPPED (unclear)
FILES CHANGED: [list]
WHAT I DID: [2–4 lines]
CHECK OUTPUT: [last few lines of the check command]
PROBLEMS / QUESTIONS: [anything Claude or Kieran should look at]
```

---

## 7. Progress log (update after each stage)

| Stage | Status | Commit | Notes |
|---|---|---|---|
| 1 Add the name | **Done** — do not redo | d90694c | Gemma 4 26B |
| 2 Show it in the About box | **Done** — do not redo | beafb0f | Gemma 4 26B, second try |
| 3 The window's name | **Done** — do not redo | 254dce5 | Qwen 3.6 35B, first try, 43 s |
| 4 The Presenter View window's name | **Done** — do not redo | 8167349 | Qwen 3.6 35B, first try, 50 s |
| 5 The name on File ▸ Home and Account | Not started | | |
