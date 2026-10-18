# How DeckCraft fits together

One page for the owner. The idea: **one engine, several front ends**. Anything a teacher can do is
a *command* in the engine, so the Mac app, the web version and the command line all get a feature
at the same time.

## The layers (bottom to top)

| Layer | Crates | What it does |
| --- | --- | --- |
| Data | `model`, `geom`, `color` | What a presentation *is*: slides, shapes, text, colours. No drawing, no files. |
| Files | `pptx`, `format`, `pdf` | Read and write `.pptx`, the native format and PDF. |
| Text and maths | `text` (`equation.rs`, `math.rs`) | Lays text out; draws equations, including ones inside a sentence. |
| Drawing | `render` | Turns a slide into pixels. |
| Engine | `engine` | **The commands** (`engine/src/cmd/*`): insert, format, Get Ready, undo, the question bank. Every user action is one command with tests. |
| Screens | `ui-egui` | The windows, ribbon and dialogs. Calls engine commands; holds no presentation logic. |
| Apps | `apps/deckcraft` (Mac/Windows), `apps/deckcraft-web` (browser), `apps/deckcraft-cli` (terminal) | Thin shells around the engine and screens. |
| Extras | `drive`, `update` | Google Drive and self-update. Mac app only today. |

Rule of thumb: **new behaviour goes in the engine as a command; the screen only asks for it.**
Then the web version gets it for free.

## Get Ready starter (where its code lives)

- `engine/src/questions.rs`: the bank (loading, merging the teacher's own, picking).
- `engine/src/template.rs`: question templates (recipes that make new numbers each time) and their checks. Format: `docs/question-writing-guide.md`.
- `engine/src/latex.rs`: turns `$\frac{3}{4}$` into the equation parser's syntax.
- `engine/src/cmd/get_ready.rs`: builds the slide: cards, full-screen slides, answers on click.
- `engine/src/cmd/figures.rs`: draws the diagrams from shapes.
- `ui-egui/src/dialogs.rs`: the Get Ready dialog and Manage Questions.
- `assets/questions/bank.json`: the built-in questions.

## How a change reaches teachers

1. Work happens on a branch; a pull request is merged into `main`.
2. The merge starts two GitHub workflows by themselves (`.github/workflows/`):
   **Publish Mac update** (a release the app's *Update available* check finds) and
   **Publish web app** (GitHub Pages). See `docs/updating.md`.
3. The Mac app offers the update next time it starts; the web app updates on its next online load.

`packaging/macos/update-local.sh` is only a developer shortcut that builds `main` on your own Mac.

## Not shared between the web and Mac versions yet

Settings, including the teacher's own questions, are kept on each device. Google Drive works in the
Mac app only. Sharing a question bank through Drive is a planned later step.
