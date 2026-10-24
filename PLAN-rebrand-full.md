# PLAN.md — DeckCraft Maths Edition: rebrand to EduDeck (part 1 of 2)

> Written by Claude for a local model (running in OpenCode via LM Studio).
> The local model must follow this plan exactly, one stage at a time.
>
> **This is part 1 of the rebrand.** It changes the name a teacher *sees* inside the app (window
> title, About, menus, dialogs, messages, the drawn logo, the web app's name, the CLI and MCP
> text). It deliberately does **not** rename anything that existing installs, saved files or the
> updater depend on. Those are listed in section 8 as part 2, which needs Kieran's decisions first.
>
> Line numbers below were correct on 2026-10-24 (branch `feat/get-ready-curriculum`). They are
> hints only: always find the line by searching for the quoted text.

---

## 0. Rules for the local model (read first, every stage)

1. Do **only the stage you are told to do**. Never start the next stage.
2. Only edit the files listed under **Files** for that stage. If another file seems to need changing, stop and report it instead.
3. Do not rename, move or delete files unless the stage says so.
4. Do not add new dependencies (crates, packages) unless the stage says so. **No stage in this plan adds one.**
5. Keep changes small. Do not refactor or "tidy up" code outside the stage.
6. When finished, run the stage's **Check** command.
7. If the check fails, read the error, fix it, and run the check again. **Maximum 3 attempts.**
8. After 3 failed attempts, or if anything in this plan is unclear: **stop** and write a short report (what you tried, the exact error). Do not guess.
9. Do not repeat your plan back to me. Make the changes, then summarise them.
10. End every stage with the **Stage report** (format in section 6).

**Rules special to this plan (the most likely way to break things):**

11. **Never do a global find-and-replace of `DeckCraft` or `deckcraft`.** Change only the exact
    strings each stage lists. Many `DeckCraft` strings must stay (see section 3, "Names that stay").
12. Never change anything spelled in lower case: crate names (`deckcraft-engine`,
    `deckcraft_engine::…`), the `.deckcraft` file extension, `ai.storyteller.deckcraft`,
    the `deckcraft-app` cache name, `deckcraft-web-*.js` file names, the `"deckcraft"` MCP server
    name. All of these stay exactly as they are.
13. Do not change code comments (`//` and `//!` lines) that say DeckCraft. Only strings the user
    can see.
14. Never touch `LICENSE-APACHE`, `LICENSE-MIT`, `NOTICE`, `ATTRIBUTION.md` or `docs/brand/`.
15. Commit after each stage passes, on the rebrand branch only, never on `main`:
    `git commit -am "rebrand stage N: <title>"`. Do not push. (Stage 1 creates a new file, so
    stage 1 also runs `git add crates/engine/src/brand.rs` before committing.)

---

## 1. Goal

**What we are building:** Everything a teacher sees in the desktop app, the web app, the CLI and
the MCP server calls the app **EduDeck** instead of DeckCraft, with a new EduDeck logo drawn in
code. Every feature keeps working exactly as before.

**Why:** The fork is becoming its own app for teachers; the ArtCraft/DeckCraft name and logo must
go before a build is shared (CLAUDE.md fork rules).

**Done means:**
- The window title of an untitled deck, Help ▸ About, File ▸ Account, the update and Google Drive
  dialogs, Presenter View and the web page title all say **EduDeck**.
- About shows the new logo (a cobalt card with a white slide carrying an equals sign), the line
  "Based on DeckCraft by the ArtCraft Team", and a "Source on GitHub" link to the fork.
- Existing presentations, settings, templates, the teacher's own questions, Google Drive sign-in
  and the updater all still work (none of their names changed).
- `cargo xtask ci` passes.

**Out of scope (do not do):**
- Renaming crates, folders, binaries, the `.app` bundle, the bundle id or the `.deckcraft` extension.
- Changing where settings live (`~/Library/Application Support/DeckCraft`, `%APPDATA%\DeckCraft`).
- Changing the keychain service name, the Google Drive folder name, or update release file names.
- Renaming the built-in theme called "DeckCraft", or the fonts "DeckCraft Legacy" /
  "DeckCraft Colr Squares".
- New app icon files (`assets/app-icon/*`), Linux/Windows packaging, README or other docs
  (except the `MATHS.md` row in stage 10).
- PDF / PPTX / `.deckcraft` file metadata (`creator`, `Application`, `generator`).

---

## 2. Project context

**Language / framework:** Rust edition 2024 (rust-version 1.95), egui/eframe 0.36, Cargo workspace.
The web app is the same code compiled to `wasm32-unknown-unknown`.

**How to build:** `cargo build -p deckcraft`

**How to run tests:** `cargo test -p <package>` (package names: `deckcraft-engine`,
`deckcraft-ui-egui`, `deckcraft-mcp`, `deckcraft-cli`, `deckcraft`, `deckcraft-web`)

**Full check (used at the end of the plan, and by Kieran before a pull request):** `cargo xtask ci`
(fmt --check, pdf-deps, clippy `-D warnings`, test, assets, layers, wasm). If the wasm part cannot
run on this Mac, run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`
and `cargo test --workspace` and say that wasm was skipped.

**Before stage 1 (Kieran does this, not the local model):**
```bash
git switch main && git pull && git switch -c feat/rebrand-edudeck
```
(Make sure `feat/get-ready-curriculum` is merged first, or branch from it instead.)

**Key folders and files:**

| Path | What it contains |
|---|---|
| `crates/engine/src/lib.rs` | Engine crate root; list of `pub mod …;` lines (around line 10–35) |
| `crates/engine/src/links.rs` | URL constants used by About, Help, CLI and MCP |
| `crates/engine/src/brand.rs` | **New in stage 1**: the app's display name and credit line |
| `crates/ui-egui/src/dialogs.rs` | Dialog titles (`fn title`-style `match`, ~line 45–80), the About dialog (`pub fn about`, ~line 3749), the logo (`pub fn paint_logo`, ~line 3786), Preferences and Manage Questions text |
| `crates/ui-egui/src/backstage.rs` | File menu pages (Home, Save As types, Print, Account) |
| `crates/ui-egui/src/backstage_tests.rs` | UI tests for those pages (egui_kittest `Harness`) |
| `crates/ui-egui/src/lib.rs` | `SlideApp`; `UI_COMMANDS` table (~line 458) with each command's label and help |
| `crates/ui-egui/src/ribbon.rs` | Title bar text (~line 82); Slide Show tab "Learn More" button (~line 2230) |
| `crates/ui-egui/src/menus.rs` | Native-style menu bar (Help menu ~line 294) |
| `crates/ui-egui/src/show.rs` | Presenter View window (~line 1974) |
| `crates/ui-egui/src/update_ui.rs`, `update_tests.rs` | In-app update dialogs and their tests |
| `crates/ui-egui/src/drive_ui.rs` | Google Drive section in Account |
| `apps/deckcraft/src/main.rs` | Desktop app entry point (window title, file dialog filters) |
| `packaging/macos/Info.plist.in` | Mac app bundle template |
| `apps/deckcraft-web/index.html`, `manifest.webmanifest`, `src/web.rs` | Web app page, install name, start-up error |
| `crates/mcp/src/server.rs`, `backend.rs`, `tools.rs` | MCP server text for AI agents |
| `apps/deckcraft-cli/src/help.rs`, `main.rs` | `deckcraft-cli` help and messages |
| `MATHS.md` | Plan and status table (one row added in stage 10) |

**Conventions to follow:**
- Errors in the UI are `Result<_, String>` with plain-English messages; no `unwrap()` outside tests
  (AGENTS.md §3 "Never crash").
- UI tests use `egui_kittest`: build a `Harness`, run a command with
  `h.state_mut().run("app.about", json!({})).unwrap()`, step with `h.run_steps(3)`, then look for
  text with `h.query_by_label("…")` / `h.query_by_label_contains("…")` /
  `h.query_all_by_label_contains("…")`.
- Unit tests in the engine live in a `#[cfg(test)] mod tests { use super::*; … }` at the bottom of
  the file.
- `rustfmt.toml` sets a long line width; run `cargo fmt -p <package>` after editing, never reformat by hand.
- UK English in user-facing text.

**Existing code to reuse (do not rewrite):**
- `deckcraft_engine::links::GITHUB` — repo URL (stage 1 points it at the fork).
- `crate::dialogs::paint_logo(painter, rect)` — draws the logo; About and Account both call it, so
  redrawing it once (stage 2) changes both.
- `crate::dialogs::version_line()` — the "Version X.Y.Z" line. Leave it as it is.
- `backstage_tests.rs` helpers `harness(app)` and `new_app()` — reuse them for new tests.

---

## 3. Design decisions (already made — do not change)

- **One display name, in one place:** `deckcraft_engine::brand::APP_NAME = "EduDeck"`. Use it in
  `format!(…)` strings. In `&'static str` tables and `match` arms that cannot call `format!`, type the
  literal `"EduDeck"` (or the `brand` constants below that hold whole phrases).
- **Credit the original.** About says "Based on DeckCraft by the ArtCraft Team". MIT/Apache
  notices stay in the licence files. This is honest and required by CLAUDE.md (keep LICENSE,
  NOTICE, ATTRIBUTION).
- **Links:** `GITHUB`, `ISSUES` and `APP_PAGE` point to the fork. `DISCORD`, `WEBSITE`, `APPS` stay
  (upstream community links, already hidden unless the teacher turns them on). The About dialog
  stops showing the "getartcraft.com/apps/deckcraft" link.
- **The logo** is drawn in code (no image asset, so no `ATTRIBUTION.md` row is needed): a rounded
  cobalt card (`#2B59FF`, the theme accent) with a darker shadow (`#1B3BB8`), a white rounded
  "slide" on it, and a cobalt equals sign on the slide. Exact code is in stage 2.
- **Names that stay `DeckCraft` in part 1** (do not change any of these):
  - `.deckcraft` extension and the Save As type id `"deckcraft"`; settings folders in
    `apps/deckcraft/src/main.rs` (`Library/Application Support/DeckCraft`, `APPDATA … DeckCraft`);
    `APP_ID = "ai.storyteller.deckcraft"`; `eframe::run_native("DeckCraft", …)`.
  - Everything in `crates/update/` (`APP_NAME = "DeckCraft.app"`, `DeckCraft-macos-…` zip names,
    user agent, messages that name `DeckCraft.app` — the bundle really is still called that).
  - Everything in `crates/drive/` (`SERVICE = "DeckCraft Google Drive"` keychain entry,
    `APP_FOLDER = "DeckCraft"` Drive folder, the sign-in page).
  - The built-in theme name `"DeckCraft"` everywhere (`model/src/theme.rs`, `ALBUM_THEMES` and
    `"DeckCraft"` defaults in `dialogs.rs`, `backstage.rs` line ~549, MCP `tools.rs` theme list,
    `effects_tests.rs`, `theme_colors_tests.rs`).
  - Font family names containing `DeckCraft` (`crates/fonts`, `render/src/tests.rs`, `engine/src/tests.rs`).
  - PDF/PPTX/format metadata and test authors (`"DeckCraft tests"`, `DeckCraft.Note.1`).
  - Info.plist keys `CFBundleExecutable`, `CFBundleIdentifier`, `CFBundleIconFile`,
    `DeckCraftVersion`, `DeckCraftBuildCommit`, and the document type's UTI and extension.
  - The MCP server's machine name `"name": "deckcraft"`.
- **Exact new code the stages rely on:**

```rust
// crates/engine/src/brand.rs  (complete file, stage 1)
//! The app's name as teachers see it. Internal names (crates, the `.deckcraft` extension, the
//! settings folder, the bundle id, the updater's file names) are unchanged: see PLAN.md §3.

/// The display name: window titles, About, menus, messages.
pub const APP_NAME: &str = "EduDeck";

/// Help ▸ About and its menu item, and the About dialog's title.
pub const ABOUT_TITLE: &str = "About EduDeck";

/// The update dialog's title and the `update.install` command's label.
pub const UPDATE_TITLE: &str = "Update EduDeck";

/// The credit line under the About text.
pub const BASED_ON: &str = "Based on DeckCraft by the ArtCraft Team";

/// The small print at the bottom of About.
pub const COPYRIGHT: &str = "MIT OR Apache-2.0 · © 2026 the EduDeck and DeckCraft contributors";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_phrases_use_the_app_name() {
        assert_eq!(APP_NAME, "EduDeck");
        assert!(ABOUT_TITLE.ends_with(APP_NAME));
        assert!(UPDATE_TITLE.ends_with(APP_NAME));
        assert!(COPYRIGHT.contains(APP_NAME));
        assert!(BASED_ON.contains("DeckCraft"), "the original is credited");
    }
}
```

```rust
// crates/engine/src/links.rs  (stage 1: these three lines change, the rest stay)
pub const APP_PAGE: &str = "https://github.com/K-Betts/deckcraft";
pub const GITHUB: &str = "https://github.com/K-Betts/deckcraft";
pub const ISSUES: &str = "https://github.com/K-Betts/deckcraft/issues";
```

```rust
// crates/ui-egui/src/dialogs.rs  (stage 2: replaces the whole body of paint_logo)
pub fn paint_logo(p: &egui::Painter, r: Rect) {
    let cobalt = Color32::from_rgb(0x2B, 0x59, 0xFF);
    let deep = Color32::from_rgb(0x1B, 0x3B, 0xB8);
    let w = r.width();
    let card = r.shrink(w * 0.08);
    let round = CornerRadius::same((w * 0.14) as u8);
    p.rect_filled(card.translate(vec2(w * 0.04, w * 0.05)), round, deep);
    p.rect_filled(card, round, cobalt);
    // The slide: a white rounded rectangle, a little above the centre.
    let slide = Rect::from_center_size(card.center() - vec2(0.0, w * 0.04), vec2(card.width() * 0.66, card.height() * 0.48));
    p.rect_filled(slide, CornerRadius::same((w * 0.05) as u8), Color32::WHITE);
    // An equals sign on the slide: two cobalt bars.
    let bar = vec2(slide.width() * 0.52, slide.height() * 0.13);
    let gap = slide.height() * 0.13;
    for dy in [-gap, gap] {
        p.rect_filled(Rect::from_center_size(slide.center() + vec2(0.0, dy), bar), CornerRadius::same((w * 0.02) as u8), cobalt);
    }
}
```

---

## 4. Stages

> Each stage should take a local model under ~30 minutes, touch 1–3 files,
> and end with a passing check. Order matters: each stage builds on the last.

### Stage 1 — Brand constants and fork links

**Goal:** The engine has a `brand` module with the EduDeck name, and the GitHub links point at the fork.

**Files:**
- `crates/engine/src/brand.rs` — create (complete file given in section 3).
- `crates/engine/src/lib.rs` — edit: register the module.
- `crates/engine/src/links.rs` — edit: three URL constants.

**Steps:**
1. Create `crates/engine/src/brand.rs` with exactly the content in section 3 (including its test).
2. In `crates/engine/src/lib.rs`, find the line `pub mod cmd;` (~line 10). Add a new line
   `pub mod brand;` **directly above it** (the list is alphabetical: `brand` comes before `cmd`).
3. In `crates/engine/src/links.rs`, replace the values of `APP_PAGE`, `GITHUB` and `ISSUES` with the
   three lines in section 3. Do not change `DISCORD`, `WEBSITE` or `APPS`, and keep the `//!` comment.
4. Run `cargo fmt -p deckcraft-engine`.

**Check:** `cargo check -p deckcraft-engine && cargo test -p deckcraft-engine brand`

**Success looks like:** `brand::tests::brand_phrases_use_the_app_name` passes.

**Watch out for:**
- Writing `mod brand;` (private) instead of `pub mod brand;` — later stages use `deckcraft_engine::brand::…` from other crates.
- Putting the `pub mod` line inside a `#[cfg(...)]` block or after the `mod tests` lines near the bottom of `lib.rs` (~line 784). It goes with the other `pub mod` lines at the top.
- The test filter `brand` runs only matching tests; that is intended (the full engine suite is slow).
- `git add crates/engine/src/brand.rs` before committing, or the new file is left out.

---

### Stage 2 — New logo and the About dialog

**Goal:** Help ▸ About shows the EduDeck logo, name, credit line and the fork's GitHub link, and a test proves it.

**Files:**
- `crates/ui-egui/src/dialogs.rs` — edit: `paint_logo`, `about`, two dialog titles.
- `crates/ui-egui/src/backstage_tests.rs` — edit: add one test.

**Steps:**
1. In `dialogs.rs`, replace the whole body of `pub fn paint_logo(p: &egui::Painter, r: Rect)`
   (~line 3786, it currently draws an orange card with a white triangle) with the code in section 3.
   Keep the signature exactly as it is.
2. In `pub fn about` (~line 3749):
   - `RichText::new("DeckCraft")` → `RichText::new(deckcraft_engine::brand::APP_NAME)`.
   - Keep the two description lines ("Presentations and slide shows, …" and "An open-source, clean-room project. …").
   - Directly after the "An open-source, clean-room project…" label, add
     `ui.label(egui::RichText::new(deckcraft_engine::brand::BASED_ON).color(t.text_dim));`
   - Inside `if app.ui.show_community_links { … }` **delete** the line
     `ui.hyperlink_to("getartcraft.com/apps/deckcraft", deckcraft_engine::links::APP_PAGE);`.
     Keep the Discord line inside the `if`.
   - **Move** `ui.hyperlink_to("Source on GitHub", deckcraft_engine::links::GITHUB);` out of the
     `if` block so it always shows, directly after the closing `}` of the `if`.
   - Replace the small-print string `"MIT OR Apache-2.0 · © 2026 ArtCraft Team and the DeckCraft contributors"`
     with `deckcraft_engine::brand::COPYRIGHT`.
3. In the dialog-title `match` (~line 45–80): `"about" => "About DeckCraft"` →
   `"about" => deckcraft_engine::brand::ABOUT_TITLE`; `"update" => "Update DeckCraft"` →
   `"update" => deckcraft_engine::brand::UPDATE_TITLE`; `"quit" => "DeckCraft"` and
   `_ => "DeckCraft"` → `deckcraft_engine::brand::APP_NAME`.
4. In `backstage_tests.rs`, add at the end of the file:
   ```rust
   #[test]
   fn about_shows_edudeck_and_credits_deckcraft() {
       let mut h = harness(new_app());
       h.state_mut().run("app.about", json!({})).unwrap();
       h.run_steps(3);
       assert!(h.query_all_by_label("EduDeck").next().is_some(), "the name is shown");
       assert!(h.query_by_label_contains("Based on DeckCraft").is_some(), "the original is credited");
       assert!(h.query_by_label_contains("getartcraft.com").is_none(), "the upstream app page link is gone");
   }
   ```
5. Run `cargo fmt -p deckcraft-ui-egui`.

**Check:** `cargo check -p deckcraft-ui-egui && cargo test -p deckcraft-ui-egui backstage`

**Success looks like:** all `backstage` tests pass, including `about_shows_edudeck_and_credits_deckcraft`.
(The existing test that expects "About DeckCraft" on the Account page still passes because the
Account page is changed in stage 4, not here. If it fails, stop and report.)

**Watch out for:**
- The match arms return `&'static str`; the `brand` constants are `&'static str`, so they fit. Do not use `format!` there.
- `Rect::from_center_size`, `vec2`, `CornerRadius` and `Color32` are already imported in `dialogs.rs` (the old `paint_logo` uses them). If the compiler says `Rect` is not found, use `egui::Rect`.
- `CornerRadius::same` takes a `u8`; keep the `as u8` casts.
- Do **not** touch `ALBUM_THEMES` or the `"DeckCraft"` theme defaults lower down in `dialogs.rs` (section 3, names that stay).
- `query_by_label` panics if more than one widget matches; that is why the name check uses `query_all_by_label(…).next()`.

---

### Stage 3 — Other dialog text, menu bar and Presenter View

**Goal:** The remaining DeckCraft strings in dialogs, the Help menu and the Presenter View window title say EduDeck.

**Files:**
- `crates/ui-egui/src/dialogs.rs` — edit: four strings.
- `crates/ui-egui/src/menus.rs` — edit: one string.
- `crates/ui-egui/src/show.rs` — edit: one string.

**Steps:**
1. `dialogs.rs`:
   - `"Check for a newer version when DeckCraft starts (contacts github.com)"` → `"Check for a newer version when EduDeck starts (contacts github.com)"` (~line 2198).
   - `"A question file (.json) made for DeckCraft"` → `"A question file (.json) made for EduDeck"` (~line 2832).
   - Two places (~lines 4366 and 4381): `format!("DeckCraft can't find “{text}”.")` →
     `format!("{} can't find “{text}”.", deckcraft_engine::brand::APP_NAME)`. Keep the curly quotes “ ” exactly.
2. `menus.rs` (~line 294): `("About DeckCraft", "app.about")` → `(deckcraft_engine::brand::ABOUT_TITLE, "app.about")`.
3. `show.rs` (~line 1974): `.with_title("Presenter View — DeckCraft")` → `.with_title("Presenter View — EduDeck")`. Keep the em dash `—`.
4. Run `cargo fmt -p deckcraft-ui-egui`.

**Check:** `cargo check -p deckcraft-ui-egui && cargo clippy -p deckcraft-ui-egui --all-targets -- -D warnings`

**Success looks like:** both commands finish with no errors or warnings.

**Watch out for:**
- The command id `"app.about"` stays. Only the label changes.
- If `menus.rs` builds the menu as a `const`/`static`, a `pub const` from another crate still works there. If the compiler complains, type `"About EduDeck"` instead.
- Do not change any `DeckCraft` theme names in `dialogs.rs` (`ALBUM_THEMES`, `d.get("theme", "DeckCraft")`, `album_choice(…, "DeckCraft", …)`, `.unwrap_or_else(|| "DeckCraft".into())`).
- Do not "fix" `—`, `“`, `”` or `…` into plain ASCII.

---

### Stage 4 — File menu pages (backstage)

**Goal:** File ▸ Account, Save As, Print and Home say EduDeck, and the existing test is updated.

**Files:**
- `crates/ui-egui/src/backstage.rs` — edit.
- `crates/ui-egui/src/backstage_tests.rs` — edit: one marker string.

**Steps:**
1. `backstage.rs`, change only these strings:
   - Save As type (~line 216): `("deckcraft", "DeckCraft Presentation (*.deckcraft)", "DeckCraft's own format: everything is kept")`
     → `("deckcraft", "EduDeck Presentation (*.deckcraft)", "EduDeck's own format: everything is kept")`.
     The first element `"deckcraft"` and `(*.deckcraft)` stay.
   - New page heading (~line 731): `RichText::new("DeckCraft")` → `RichText::new(deckcraft_engine::brand::APP_NAME)`.
   - Discord link (~line 803): `"Join the ArtCraft community on Discord"` → `"Join the DeckCraft community on Discord"`
     (it is still the upstream community, shown only when the teacher turns links on).
   - Print text (~lines 1117 and 1119): replace the leading `DeckCraft prints through a PDF` with `EduDeck prints through a PDF` in both strings.
   - Account (~line 1418): `"DeckCraft has no accounts of its own, …"` → `"EduDeck has no accounts of its own, …"` (rest unchanged).
   - Account Product Information (~line 1429): `RichText::new("DeckCraft")` → `RichText::new(deckcraft_engine::brand::APP_NAME)`.
   - Account task (~lines 1437–1438): both `"About DeckCraft"` → `deckcraft_engine::brand::ABOUT_TITLE`.
   - Below that, add `dim(ui, deckcraft_engine::brand::BASED_ON);` directly after the line
     `dim(ui, "Open source under MIT OR Apache-2.0. Not affiliated with Microsoft.");`.
2. **Leave** `find(|t| t.name == "DeckCraft")` (~line 549) unchanged: it is the theme name.
3. `backstage_tests.rs` (~line 169): `(Page::Account, "About DeckCraft")` → `(Page::Account, "About EduDeck")`.
4. Leave the test that expects `"not a presentation DeckCraft can read"` (~line 457) as it is. That
   message comes from `crates/engine/src/cmd/file.rs` (~line 220), which is not in this plan; it is
   listed for part 2 (section 8).
5. Run `cargo fmt -p deckcraft-ui-egui`.

**Check:** `cargo check -p deckcraft-ui-egui && cargo test -p deckcraft-ui-egui backstage`

**Success looks like:** all `backstage` tests pass, including the one with `(Page::Account, "About EduDeck")`.

**Watch out for:**
- `dim()` takes `&str`; the constant is `&str`, so pass it directly (no `&`).
- `task(…)` may expect `&str` arguments; constants fit.
- Changing the theme lookup on line ~549 breaks the New page (no theme found). Leave it.

---

### Stage 5 — Command labels, title bar and Learn More

**Goal:** Command Palette labels, status messages and the untitled window title say EduDeck; Learn More opens the fork's page.

**Files:**
- `crates/ui-egui/src/lib.rs` — edit.
- `crates/ui-egui/src/ribbon.rs` — edit.

**Steps:**
1. `lib.rs`, in `UI_COMMANDS` (~line 458 onwards), replace `DeckCraft` with `EduDeck` inside these
   string literals only:
   - `"{on?: bool}: ask GitHub for a newer version when DeckCraft starts (off by default)"`
   - `"Update DeckCraft"` (label of `update.install`)
   - `"{}: opens your browser; asks only for files DeckCraft saves"`
   - `"About DeckCraft"` (label of `app.about`)
   Do **not** change `"{}: saves the open presentation as .pptx in your DeckCraft folder, …"` (~line 502):
   the Drive folder really is still called DeckCraft.
2. `lib.rs` (~line 210): `format!("{name}: DeckCraft can't open this kind of file")` →
   `format!("{name}: {} can't open this kind of file", deckcraft_engine::brand::APP_NAME)`.
3. `lib.rs` (~line 907): `format!("DeckCraft can't export “{other}”")` →
   `format!("{} can't export “{other}”", deckcraft_engine::brand::APP_NAME)`.
4. `ribbon.rs` (~line 82): `.unwrap_or_else(|| "DeckCraft".into())` (window title with no deck open) →
   `.unwrap_or_else(|| deckcraft_engine::brand::APP_NAME.into())`.
5. `ribbon.rs` "Learn More" (~line 2231) already uses `links::APP_PAGE`, which stage 1 changed. No edit needed; just confirm it.
6. Run `cargo fmt -p deckcraft-ui-egui`.

**Check:** `cargo check -p deckcraft-ui-egui && cargo test -p deckcraft-ui-egui`

**Success looks like:** the whole `deckcraft-ui-egui` test suite passes (about 380+ tests; can take several minutes).

**Watch out for:**
- `UI_COMMANDS` is a `const` table of `&str`: use literals there (not `format!`, not `concat!` with a `const`).
- Some tests look commands up by label. If a test fails with "About DeckCraft" or "Update DeckCraft", update **that test's string** only if the test file is in this stage's list; otherwise stop and report the test name.
- Inside a `format!`, a literal `{` must be written `{{`. The strings above have none, but `UI_COMMANDS` help text like `"{on?: bool}…"` is **not** a `format!` string, so leave its braces alone.

---

### Stage 6 — Update and Google Drive screens

**Goal:** The in-app update dialogs and the Drive section say EduDeck.

**Files:**
- `crates/ui-egui/src/update_ui.rs` — edit.
- `crates/ui-egui/src/update_tests.rs` — edit.
- `crates/ui-egui/src/drive_ui.rs` — edit.

**Steps:**
1. `update_ui.rs`, change `DeckCraft` → `EduDeck` in exactly these strings:
   - `"save your work first: restarting closes DeckCraft"`
   - `"Check GitHub for a newer version of DeckCraft? This sends no personal information."`
   - `"A new version of DeckCraft is ready."`
   - `"Restart DeckCraft to use the new version."`
   - `"A newer version of DeckCraft is ready"` (hover text)
   - `format!("A new version of DeckCraft is available ({sha}).")` → `format!("A new version of EduDeck is available ({sha}).")`
2. `update_tests.rs` (~line 132): `"A new version of DeckCraft is ready."` → `"A new version of EduDeck is ready."`.
   Search the file for any other `DeckCraft` **user-text** expectation that matches a string you changed in step 1, and update it the same way.
3. `drive_ui.rs`:
   - `"Optional. DeckCraft can only see files it saved to your Drive. …"` → `"Optional. EduDeck can only see files it saved to your Drive. …"` (rest unchanged).
   - `"Nothing saved from DeckCraft yet."` → `"Nothing saved from EduDeck yet."`
   - Any text that names the Drive **folder** (for example "your DeckCraft folder") stays, because the folder is still called DeckCraft.
4. Run `cargo fmt -p deckcraft-ui-egui`.

**Check:** `cargo check -p deckcraft-ui-egui && cargo test -p deckcraft-ui-egui update && cargo test -p deckcraft-ui-egui drive`

**Success looks like:** all `update` and `drive` tests pass.

**Watch out for:**
- Do **not** edit `crates/update/` (its messages say "installed DeckCraft.app", which is still the real bundle name). That is part 2.
- A test string that still says DeckCraft and fails means you missed one in step 2; a test string that says DeckCraft and passes must be left alone.

---

### Stage 7 — Desktop app shell and Mac bundle names

**Goal:** The desktop window, the Open dialog filter and the Mac menu-bar name say EduDeck, without moving settings or changing the bundle.

**Files:**
- `apps/deckcraft/src/main.rs` — edit.
- `packaging/macos/Info.plist.in` — edit.

**Steps:**
1. `apps/deckcraft/src/main.rs`:
   - `.with_title("DeckCraft")` (~line 212) → `.with_title(deckcraft_engine::brand::APP_NAME)`.
   - `.add_filter("DeckCraft", &["deckcraft"])` (~line 115) → `.add_filter("EduDeck", &["deckcraft"])`. The `&["deckcraft"]` extension stays.
   - `log::info!("DeckCraft {} ({} {})", …)` (~line 197) → `log::info!("{} {} ({} {})", deckcraft_engine::brand::APP_NAME, env!("CARGO_PKG_VERSION"), std::env::consts::OS, std::env::consts::ARCH)`.
   - `eprintln!("DeckCraft needs a graphical session: …")` (~line 232) → `"EduDeck needs a graphical session: …"` (rest unchanged).
   - **Do not change**: the settings paths on lines ~63–65 (`Application Support/DeckCraft`, `APPDATA … DeckCraft`), `APP_ID`, `eframe::run_native("DeckCraft", …)` (~line 247), or the `include_bytes!` icon path.
2. `packaging/macos/Info.plist.in`:
   - `CFBundleDisplayName` value `DeckCraft` → `EduDeck`.
   - `CFBundleName` value `DeckCraft` → `EduDeck`.
   - `NSHumanReadableCopyright` (~line 46) `Copyright © the DeckCraft authors. MIT OR Apache-2.0.` → `Copyright © the EduDeck and DeckCraft authors. MIT OR Apache-2.0.`
   - The document type's name `DeckCraft Presentation` (~line 55) → `EduDeck Presentation`.
   - **Do not change** `CFBundleExecutable`, `CFBundleIdentifier`, `CFBundleIconFile`, the `DeckCraftVersion`/`DeckCraftBuildCommit` keys, `ai.storyteller.deckcraft.presentation`, `deckcraft`, `application/x-deckcraft`, or the `@VERSION@`-style placeholders.
3. Run `cargo fmt -p deckcraft`.

**Check:** `cargo check -p deckcraft && cargo clippy -p deckcraft --all-targets -- -D warnings && plutil -lint packaging/macos/Info.plist.in`

**Success looks like:** both cargo commands pass and `plutil` prints `packaging/macos/Info.plist.in: OK`.

**Watch out for:**
- `apps/deckcraft` already depends on `deckcraft-engine` (see its `Cargo.toml`), so `deckcraft_engine::brand` works without adding anything.
- Changing the settings folder would make every teacher lose their preferences, templates and own questions on update. Leave it.
- `CFBundleExecutable` must match the binary inside the bundle (`Contents/MacOS/DeckCraft`), and the updater checks for it. Leave it.
- `.with_title(…)` takes `impl Into<String>`; a `&'static str` constant works.

---

### Stage 8 — Web app name

**Goal:** The browser tab, the loading text, the install name and the start-up error say EduDeck.

**Files:**
- `apps/deckcraft-web/index.html` — edit.
- `apps/deckcraft-web/manifest.webmanifest` — edit.
- `apps/deckcraft-web/src/web.rs` — edit.

**Steps:**
1. `index.html`: `<title>DeckCraft</title>` → `<title>EduDeck</title>`; `Loading DeckCraft&hellip;` → `Loading EduDeck&hellip;`.
   Keep the element id `deckcraft_loading` and every file name exactly as they are.
2. `manifest.webmanifest`: `"name": "DeckCraft"` → `"name": "EduDeck"`; `"short_name": "DeckCraft"` → `"short_name": "EduDeck"`. Change nothing else (keep `start_url`, `id`, icons, colours).
3. `web.rs` (~line 64): `"<p>DeckCraft failed to start: {e:?}</p>…"` → `"<p>EduDeck failed to start: {e:?}</p>…"` (rest unchanged).
4. Do **not** edit `sw.js` (its cache name `deckcraft-app` must stay, or every installed copy re-downloads and the old cache is orphaned).

**Check:** `node apps/deckcraft-web/tests/sw.test.mjs && cargo check -p deckcraft-web --target wasm32-unknown-unknown`
(If the wasm target is not installed, the second command fails with "can't find crate for `core`": report that and run `cargo fmt --all -- --check` instead. Do not install anything.)

**Success looks like:** the Node test prints no assertion error, and the wasm check passes (or is reported as skipped).

**Watch out for:**
- `manifest.webmanifest` is JSON: keep the commas and quotes valid. You can check it with `python3 -I -m json.tool apps/deckcraft-web/manifest.webmanifest`.
- If the manifest has an `"id"` field containing `deckcraft`, leave it: changing it makes browsers treat EduDeck as a different installed app.

---

### Stage 9 — MCP server text

**Goal:** AI agents that connect through MCP are told the app is EduDeck; the server's machine name stays `deckcraft` so existing MCP configs keep working.

**Files:**
- `crates/mcp/src/server.rs` — edit.
- `crates/mcp/src/backend.rs` — edit.
- `crates/mcp/src/tools.rs` — edit.

**Steps:**
1. `server.rs`:
   - `const INSTRUCTIONS` (~line 21): the opening `"DeckCraft is a presentation app (a PowerPoint clone)."` → `"EduDeck is a presentation app for teachers (a PowerPoint clone)."`. Change no other word of `INSTRUCTIONS`, unless it says `DeckCraft` again as the app's name, in which case write `EduDeck`.
   - `"serverInfo": {"name": "deckcraft", "title": "DeckCraft", …` (~line 139): change only `"title": "DeckCraft"` → `"title": "EduDeck"`. **`"name": "deckcraft"` stays.**
2. `backend.rs` (~lines 130, 136, 147): in the three `format!` strings, `the DeckCraft app` / `DeckCraft app` → `the EduDeck app` / `EduDeck app`. Leave the `///` comment on line ~24.
3. `tools.rs` (~line 282): `"Screenshot of the whole DeckCraft window (UI included).{APP_ONLY}"` → `"Screenshot of the whole EduDeck window (UI included).{APP_ONLY}"`.
   **Leave line ~97** (`Optional theme: DeckCraft, Harbor, …`): that is the theme name.
4. Run `cargo fmt -p deckcraft-mcp`.

**Check:** `cargo check -p deckcraft-mcp && cargo test -p deckcraft-mcp`

**Success looks like:** all `deckcraft-mcp` tests pass.

**Watch out for:**
- A test may check the server title or a message. If one fails because it expects "DeckCraft", stop and report its name and file (test files are not in this stage's list unless they are inside these three files' `mod tests`, in which case update the expected string).
- `INSTRUCTIONS` is long; edit in place, do not rewrite it.

---

### Stage 10 — CLI text and the status row

**Goal:** `deckcraft-cli` help and messages name EduDeck (the command itself stays `deckcraft-cli`), and `MATHS.md` records part 1.

**Files:**
- `apps/deckcraft-cli/src/help.rs` — edit.
- `apps/deckcraft-cli/src/main.rs` — edit.
- `MATHS.md` — edit: one new status row.

**Steps:**
1. `help.rs`, in user-visible strings only:
   - `"run a command in the DeckCraft app started with --control"` → `"run a command in the EduDeck app started with --control"`.
   - `"Sends one request to the DeckCraft app started with `deckcraft --control PORT` …"` → `"Sends one request to the EduDeck app started with `deckcraft --control PORT` …"`. The command `deckcraft --control PORT` stays (the binary is still `deckcraft`).
   - `"… on the DeckCraft app started with --control PORT."` (~line 169) → `"… on the EduDeck app started with --control PORT."`.
   - `"print the DeckCraft community and project links"` → `"print the EduDeck community and project links"`; `"Prints the DeckCraft community and project links."` → `"Prints the EduDeck community and project links."`.
   - `"deckcraft-cli works on DeckCraft presentations (.deckcraft, .pptx) without a window."` → `"deckcraft-cli works on EduDeck presentations (.deckcraft, .pptx) without a window."`.
2. `main.rs` (~line 702): `format!("no DeckCraft app on port {port} ({e}); start it with `deckcraft --control {port}`")` → `format!("no EduDeck app on port {port} ({e}); start it with `deckcraft --control {port}`")`. Leave the `//!` comment on line 1.
3. `MATHS.md`: in the `## Status` table, add one row **at the bottom of the table** (directly above the blank line before `## Known gaps`):
   ```
   | 2026-10-24 | **Rebrand to EduDeck, part 1.** Everything a teacher sees says EduDeck: window title, About (new logo drawn in code, "Based on DeckCraft by the ArtCraft Team", Source on GitHub link to the fork), Account, Save As, Print, update and Drive screens, Presenter View, the web app's title and install name, CLI help and MCP text. Unchanged on purpose: crate and binary names, the `.deckcraft` extension, the settings folder, bundle id and `DeckCraft.app` name, keychain entry, Drive folder, updater file names, the theme called DeckCraft, file metadata. Part 2 (app icon, bundle rename with an updater that accepts both names, settings migration) is in PLAN.md §8. Not checked: the logo by eye, the Mac menu-bar name in a packaged build. | `feat/rebrand-edudeck` |
   ```
   Replace the date with today's date if it is not 2026-10-24.
4. Run `cargo fmt -p deckcraft-cli`.

**Check:** `cargo check -p deckcraft-cli && cargo test -p deckcraft-cli`

**Success looks like:** all `deckcraft-cli` tests pass.

**Watch out for:**
- Some help text is in `r"…"` or multi-line strings with backticks inside; change only the word `DeckCraft`, keep backticks and line breaks.
- If a CLI test compares the help output with a saved snapshot file, the snapshot needs the same change. If that snapshot file is not in this stage's list, stop and report it.
- In the `MATHS.md` row, `|` separates columns: do not add any other `|` inside the text.

---

## 5. Final check (after the last stage)

- [ ] Full check command passes: `cargo xtask ci` (or the fallback in section 2, saying what was skipped)
- [ ] No user-visible DeckCraft left in the files this plan touched. Run this and read every hit; each one must be on the "names that stay" list in section 3:
  ```bash
  grep -rn '"[^"]*DeckCraft[^"]*"' --include='*.rs' crates/ui-egui/src crates/mcp/src apps/deckcraft/src apps/deckcraft-cli/src apps/deckcraft-web/src
  ```
- [ ] Manual test (Kieran): `cargo run -p deckcraft` → the window title says EduDeck; Help ▸ About shows the cobalt logo with the equals sign, "EduDeck", "Based on DeckCraft by the ArtCraft Team" and "Source on GitHub"; File ▸ Account shows the same logo; your recent files, preferences and own questions are all still there; open and save a `.pptx` and a `.deckcraft` file.
- [ ] Manual test (Kieran): `packaging/macos/install-local.sh` → the menu bar shows EduDeck; the app in /Applications is still `DeckCraft.app` (expected until part 2).
- [ ] Everything in "Done means" (section 1) is true
- [ ] Nothing in "Out of scope" was touched: `git diff --stat main` lists only the files named in stages 1–10.

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
| 1 Brand constants and fork links | Not started | | |
| 2 New logo and About | Not started | | |
| 3 Dialog text, menu bar, Presenter View | Not started | | |
| 4 File menu pages | Not started | | |
| 5 Command labels, title bar | Not started | | |
| 6 Update and Drive screens | Not started | | |
| 7 Desktop shell and Mac bundle names | Not started | | |
| 8 Web app name | Not started | | |
| 9 MCP server text | Not started | | |
| 10 CLI text and status row | Not started | | |

---

## 8. Part 2 — not planned yet (needs Kieran's decisions first)

These change names that installed copies, saved settings or the updater depend on, so each needs a
migration or a decision. Claude will write a separate plan for them.

| Item | Why it is risky | Decision needed |
|---|---|---|
| New app icon (`assets/app-icon/*`, `.icns`, `.ico`, PWA icons, `icon-source.deckcraft`) | It is an asset: needs a design you approve and an `ATTRIBUTION.md` row (contributor-original). | Use the in-code logo from stage 2 as the icon, or a different design? |
| `DeckCraft.app` → `EduDeck.app`, executable name | The updater (`crates/update`) only replaces an app called `DeckCraft.app` with `Contents/MacOS/DeckCraft`; `install-local.sh` and `update-local.sh` too. | Ship one release whose updater accepts both names, then rename. |
| Release file names `DeckCraft-macos-…zip` | Installed copies look for that prefix; renaming breaks their update check. | Publish both names for one release, or keep the old prefix. |
| Bundle id `ai.storyteller.deckcraft` (also Linux `.desktop`, Windows `.wxs`) | macOS treats a new id as a different app: file associations and permissions reset. | New id (e.g. `uk.edudeck.app`), or keep. |
| Settings folder `Application Support/DeckCraft` | Moving it loses preferences, templates and own questions unless migrated. | Migrate on first launch (copy old to new), or keep. |
| Keychain `DeckCraft Google Drive`, Drive folder `DeckCraft` | Renaming signs teachers out and starts a second Drive folder. | Keep (recommended), or migrate. |
| Built-in theme "DeckCraft" | The name is saved in decks and used to look themes up. | Rename to "EduDeck" with the old name still recognised, or keep. |
| PDF creator, PPTX `Application`, `.deckcraft` `generator` | Harmless metadata, but tests check it. | Change to EduDeck. |
| Engine messages (`engine/src/cmd/file.rs` "not a presentation DeckCraft can read", others in `engine/src`, `format/src/lib.rs` errors) | Shown to teachers, but tests in several crates expect the old text. | Change to EduDeck (no decision, just a later stage). |
| `docs/brand/` ArtCraft logos, README, docs, Linux/Windows packaging text | Brand files must go before sharing (CLAUDE.md), but `ATTRIBUTION.md` rows and `cargo xtask assets` must stay in step. | Remove now or with the icon. |
| Crate names `deckcraft-*`, `.deckcraft` extension | Huge, mechanical, and the extension is in teachers' files. | Probably never; confirm. |
