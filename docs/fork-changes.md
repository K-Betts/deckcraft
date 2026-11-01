# Fork changes inside upstream files

Every change the maths edition makes **inside a file that also exists upstream**
(`storytold/deckcraft`), grouped by feature, with where its code now lives. This is the checklist
for merging upstream: each row is either a small hook to carry across, or an inline change to
re-apply (or drop, when upstream fixed the same thing).

The fork's own files (about 29,000 lines: `crates/engine/src/{questions,template,curriculum,latex,exam_library}.rs`,
`crates/engine/src/cmd/{get_ready,exam,exam_library,figures}.rs`, `crates/text/src/{equation,math}.rs`,
`crates/drive`, `crates/update`, `crates/ui-egui/src/maths/…`, the web app's PWA and school files…)
merge without conflicts and are not listed one by one.

Base: the last upstream merge, `5d9c3818`. Measure with
`git diff --numstat --diff-filter=M 5d9c3818 HEAD -- crates apps`.

| | Files | Lines added | Lines removed |
| --- | --- | --- | --- |
| Before this refactor (main at `fe3f240a`) | 62 | 4,293 | 592 |
| After (`refactor-maths-modules`) | 62 | 1,225 | 548 |

**Kinds of change.** *Hook*: one line (a `mod`, a field, a call, a match arm) that reaches fork code.
*Inline*: a change to upstream behaviour or look that stays where it is. *Inline fix*: a fix to an
upstream bug; when merging, check whether upstream fixed it too and keep only one.

## Where the fork code lives now

| Fork module | What it holds | Reached from (hook) |
| --- | --- | --- |
| `ui-egui/src/maths/` | Calculator, Teacher Mode overlay, maths tools, subject packs, Drive and update screens, equation drawing, and their tests | `pub mod maths;` in `lib.rs` |
| `ui-egui/src/maths/app.rs` | `MathsUi` (saved settings), `MathsApp` (running state), `MathsServices` (host services), the fork's UI commands (`run_maths`), Teacher Mode on/off, Save format default, follow-the-computer appearance, `is_compact` | `UiState::maths` (`#[serde(flatten)]`, so `ui.json` keeps its keys), `SlideApp::maths`, `Services::maths`; `id if maths::app::owns(id)` in `run_ui`; `maths_quit_if_asked`, `poll_maths`, `follow_system_theme` in the frame loop |
| `ui-egui/src/maths/logging.rs` | Slow-frame and AutoRecover logging | `note_frame_time`, `save_recovery_logged` in `lib.rs` |
| `ui-egui/src/maths/appearance.rs` | The Appearance drop-down (Match this computer, Light, Dark) | Preferences and File ▸ Account |
| `ui-egui/src/dialogs/maths/` | Get Ready, Choose a Skill, Manage Questions, Exam Question, Question Library, Taught, the fork's Equation dialog, Update; the Preferences sections and their Cancel; the dialog buttons | `pub(crate) mod maths;`, `maths::title`, `maths::sized`, `maths::body` (first line of `body`), `maths::buttons_if` (in `buttons`), `preferences_snapshot`/`restore_preferences`/`appearance_row`/`preferences_sections` |
| `ui-egui/src/show/steps.rs` | Reveal Step by Step in the show (click a line, Tab, Shift+Tab) and its tests | `mod steps;`, `Show::steps`, `Key::marks`, `render_frame(…, marks, …)` |
| `ui-egui/src/show/teacher.rs` | Teacher Mode overlay in the show: pointer and wheel, drawing, ending the show | `teacher::pointer`, `teacher::overlay`, `teacher::stash_on_exit` |
| `ui-egui/src/ribbon/fold.rs` | The responsive ribbon (groups fold into More) and the group pass state | `fold::responsive_tab` in `ribbon::show`; `widgets::group` calls `fold::group` |
| `ui-egui/src/ribbon/maths.rs` | Get Ready, Reveal Steps and Teacher Mode ribbon groups | `maths::insert_group`, `maths::animations_group`, `maths::view_group` |
| `ui-egui/src/backstage/maths.rs` | Home's update and subject cards; personal templates list and folder hint | `maths::home_cards`, `maths::personal_templates`, `maths::templates_folder_hint` |
| `ui-egui/src/settings/{maths,text}.rs` | `maths-tools.json`; settings as text for the browser | `mod maths; pub mod text; pub use maths::…` |
| `ui-egui/src/theme/design.rs` | Radius, spacing and type scales; the contrast test | `mod design; pub use design::{radius, size, space};` |
| `engine/src/cmd/animation/steps.rs` | `animation.steps` (Reveal Step by Step) and its tests | `mod steps;`, `steps::spec()` in the spec list (same place as before) |
| `engine/src/cmd/insert/equation.rs` | `insert.equation`, `equation.source` | `mod equation;`, `equation::spec()`, `equation::source_spec()` |
| `engine/src/cmd/file/personal.rs` | Personal templates: `file.new {templateFile}`, `file.saveTemplate` | `mod personal;`, `personal::new(…)` at the top of `new`, `personal::save_template` |
| `engine/src/templates/personal.rs` | The personal templates folder | `mod personal; pub use personal::{…};` |
| `engine/src/tests/fixes.rs` | Regression tests for the fork's font and notes fixes | `mod fixes;` at the end of `tests.rs` |
| `text/src/math_layout.rs` | Equation stand-in text, equations drawn as maths in a paragraph or a sentence | `mod math_layout; use math_layout::{…};` |
| `text/src/tests/equations.rs` | Tests of the above | `mod equations;` at the end of `tests.rs` |
| `pptx/src/read/dml/math.rs` | An equation's plain text and the formatting it keeps on import | `mod math; use math::{…};` |
| `apps/deckcraft/src/maths/` | Drive and update backends, the folder picker, `maths-tools.json` load and save | `mod maths;`, `maths: maths::services()`, `maths::load_prefs`, `maths::save_prefs` |
| `apps/deckcraft-web/src/web/settings.rs` | Settings in the browser's local storage | `pub(crate) mod settings;`, `WebShell::settings` |
| `apps/deckcraft-cli/src/maths.rs` | `fidelity`, `bank check`, `curriculum check` and their tests | `mod maths;`, one match arm |

## By feature: what is still inside upstream files

### Get Ready, curriculum, question bank, Exam Question, Question Library
- `engine/src/lib.rs`: `pub mod curriculum, exam_library, latex, questions, template;` (hooks).
- `engine/src/cmd/mod.rs`: `pub mod exam, exam_library, figures, get_ready;` and their `specs()` (hooks).
- `engine/src/io.rs`: `list_files`, `create_dir` (for the Question Library and personal templates; wasm stubs). Inline.
- `engine/tests/it/main.rs`: `mod deck_scale;` (hook).
- `ui-egui/src/lib.rs`: `app.dialog` passes its params to the dialog (Choose a Skill uses `slot`). Inline.
- `apps/deckcraft/src/main.rs`: `questions` and `curriculum` file filters in the open dialog. Inline.
- `apps/deckcraft-cli/src/help.rs`: help pages for `fidelity`, `bank`, `curriculum` (data, in the help table's order). Inline.

### Teacher Mode and maths tools
- `ui-egui/src/show.rs`: `Show::teacher`; calculator keys are not show shortcuts; Cmd+P/Cmd+L give way to Teacher Mode; E clears and Cmd+Z undoes its ink; the show toolbar hides; `on_slide_change`. Inline (keys) and hooks.
- `ui-egui/src/lib.rs`: `show.end` and closing the window put the overlay aside (`stash_teacher`). Hooks.
- `ui-egui/src/menus.rs`: Cmd+Shift+T and the Teacher Mode menu item. Inline.
- `ui-egui/src/status.rs`: Teacher Mode button. Inline.
- `UI_COMMANDS` in `lib.rs`: `show.teacher`, `show.teacherKeep`. Inline (table rows).

### Reveal Step by Step
- `anim/src/timeline.rs`: `Timeline::step_lines`; `anim/src/tests.rs`: its test. Inline (an addition to upstream's type).
- `render/src/placed.rs`: `PlacedText::shape` (which shape a text belongs to). Inline.
- `ui-egui/src/show.rs`: click on a line or Tab reveals it (`step_line_under_pointer`, `tab_step`), frame cache key and `render_frame` carry the marks. Inline.

### Equations (editor, display, PowerPoint round trip)
- `text/src/lib.rs`: `pub mod equation, math;`; `ParaShaped::{math, inline_math}`; stand-in text; display alignment; line height and drawing of equations. Inline (layout integration) plus hooks.
- `model/src/text.rs`: `Paragraph::display_math_align`. Inline.
- `pptx/src/read/dml.rs`: equation runs keep their stand-in text and formatting. Hook.
- `pptx/src/write/dml.rs`: a display equation's fallback text keeps the equation's alignment. Inline.
- `pptx/src/write/shapes.rs`: `xmlns:a14` declared on `mc:AlternateContent`, where PowerPoint declares it. **Inline fix.**
- `pptx/tests/it/roundtrip.rs`: operators are spaced in an equation's stand-in text (`x = 1`). Inline (test follows the change).
- `engine/src/spell.rs`: equations are not spell-checked (with a test). Inline.
- `engine/src/cmd/text.rs`: `text.set {maths}` turns `$…$` into equations. Inline; `apps/deckcraft-cli/tests/it/cli.rs` follows its description.
- `ui-egui/src/canvas.rs`: double-clicking an equation opens the Equation dialog. Inline.
- `ui-egui/src/dialogs.rs`: upstream's own Equation dialog arm and `pretty_equation` are kept but unreachable (`maths::body` answers `equation` first), so the merge can take upstream's version untouched.

### Fonts
- `fonts/src/fontdb.rs`: Cambria Math is drawn in STIX Two Math first; `glyph_bounds`. **Inline fix** (with tests in `fonts/src/lib.rs`).
- `engine/src/fonts.rs`: a family with an installed open substitute is not reported missing. **Inline fix** (test in `engine/src/tests/fixes.rs`).

### Notes pages
- `model/src/lib.rs`: `Slide::notes_page_present`. `pptx/src/read/mod.rs`: set on import; an empty body placeholder no longer hides the one with the notes. `pptx/src/write/mod.rs`: such pages are written back. `engine/src/compare.rs`: reports notes page and notes formatting changes. **Inline fix** (tests in `engine/src/tests/fixes.rs` and `compare.rs`).

### Subject packs and Preferences
- `ui-egui/src/dialogs.rs`: hooks only (see the table). The Preferences snapshot keeps the fork's part under `maths`.
- `ui-egui/src/preferences_tests.rs`: uses the Appearance drop-down. Inline (test follows the change).
- `UI_COMMANDS`: `app.pack`, `app.dismissSubjectsHint`. Inline (table rows).

### Appearance and the calmer look (theme changes)
- `ui-egui/src/theme.rs`: the light and dark token colours ("Fresh Paper", cobalt accent); `apply` split into `visuals` (also used by the light Get Ready cards); larger spacing and radii; controls have an outline at rest (its test changed to match). Inline.
- `ui-egui/src/lib.rs`: `view.dark` stops following the computer; `view.appearance` in `UI_COMMANDS`. Inline.
- `ui-egui/src/ribbon.rs`: tabs as pills, the ribbon as a floating card with a shadow; the scroll-and-chevron overflow replaced by the folding ribbon (`ribbon/fold.rs`). Inline.
- `ui-egui/src/widgets.rs`: control radius from the theme; groups separated by space instead of a rule; `group` calls the fold pass (and needs `R: Default`). Inline.
- `ui-egui/src/backstage.rs`: menu and page colours swapped, pill nav items, card tiles. Inline.
- `ui-egui/src/dialogs.rs`: OK/Cancel restyled (`maths::buttons_if`). Hook.
- `ui-egui/src/replacefonts_tests.rs`, `ui-egui/src/control.rs` (`ribbonFolded` in the UI state report): follow the folding ribbon. Inline.

### Phone and tablet widths (web app)
- `ui-egui/src/ribbon.rs` (title bar), `status.rs`, `panes.rs`, `thumbs.rs`, `lib.rs`: below `maths::app::COMPACT_WIDTH` (640 pt) panels give way to the slide. Inline (layout code, re-indented in places).
- `ui-egui/src/lib.rs`: presenter view off by default in the browser. Inline.

### Updates and Google Drive
- `ui-egui/src/lib.rs`: hooks only (`Services::maths`, `poll_maths`, `maths_quit_if_asked`); `UI_COMMANDS` rows `app.updates`, `update.*`, `app.drive`, `drive.*`.
- `ui-egui/src/status.rs`: the update chip. Hook. `backstage.rs`: Drive section on Account. Hook.
- `apps/deckcraft/Cargo.toml`: `deckcraft-drive`, `deckcraft-update`. Inline.
- **Upstream now has its own updater** (`craft_update`, `ui-egui/src/dialogs/updates_ui.rs`, `app.checkUpdates`, `UPDATE_APP` in `lib.rs`). Step 2 must decide which one the Mac app keeps; the fork's publishes from `K-Betts/deckcraft` releases (`docs/updating.md`).

### Community links, UK English, saving as PowerPoint
- `ui-egui/src/dialogs.rs` (About), `backstage.rs` (Home, Account): upstream links only when `show_community_links`. Inline.
- `model/src/defaults.rs`: `DEFAULT_LANG = "en-GB"`; used in `dialogs.rs`, `status.rs`, `squiggles.rs`, `engine/src/cmd/review.rs`; tests in `datetime_tests.rs`, `spelling_tests.rs`, `units_tests.rs`, `status.rs` follow. Inline.
- `ui-egui/src/lib.rs`: Save and Save As suggest `.pptx` for a new presentation (`default_save_format`, in `maths/app.rs`). Hooks.
- `engine/src/cmd/file.rs`: `file.new` and `file.saveTemplate` descriptions, `file.templates` lists personal templates. Inline plus hooks.

### Deck logging (counts and timings only)
- `engine/src/lib.rs` (documents added and closed), `engine/src/recovery.rs` (bytes written), `ui-egui/src/opening.rs` (background opens), `ui-egui/src/lib.rs` (file opened on the UI thread; `Perf::slow_logged_ms`). Inline, plus hooks into `maths/logging.rs`.

### Clipboard shortcuts
- `ui-egui/src/lib.rs`: Cmd+C, Cmd+X and Cmd+V work while a ribbon button has keyboard focus (`clipboard_event`, `paste_released_image`); a failed copy says why. **Inline fix** (test in `maths/clipboard_tests.rs`).

### Wasm build
- `ui-egui/src/lib.rs`: `opening_generation` is unused in the browser (`expect(dead_code)`). Inline.

### Web shell
- `apps/deckcraft-web/src/web.rs`: settings in local storage (hook), the school curriculum (`school::apply`), starts on Home with an empty presentation, `?sample` for the demo deck. Inline plus hooks.
- `apps/deckcraft-web/src/main.rs`: `mod school;` and the doc comment. `index.html`: PWA manifest, icons, service worker. `Cargo.toml`: the `Storage` web-sys feature. Inline.

### Tests that moved without changing
Moved into fork files: the Reveal Step by Step show tests (`show/steps.rs`), the `animation.steps` tests
(`cmd/animation/steps.rs`), personal templates (`cmd/file/personal.rs`, `templates/personal.rs`),
equation layout (`text/src/tests/equations.rs`), font and notes fixes (`engine/src/tests/fixes.rs`), the
CLI's `fidelity` tests (`apps/deckcraft-cli/src/maths.rs`; the `fidelity --help` check became its own
test), settings (`settings/maths.rs`, `settings/text.rs`), the clipboard test and the contrast test.
`ui-egui/src/backstage_tests.rs` still holds two fork tests (personal templates on File ▸ New, the
`.pptx` default), added at one place.
