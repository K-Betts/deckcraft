# DeckCraft maths edition — working notes for Claude

This is Kieran's fork of DeckCraft (K-Betts/deckcraft), being turned into a free Mac slides app for
maths teachers. Read in this order:

1. **[MATHS.md](MATHS.md)**: the goal, the phase plan and where things stand. Start there.
2. **[docs/how-it-fits-together.md](docs/how-it-fits-together.md)**: one page on how the crates fit and how a change reaches teachers.
3. **[AGENTS.md](AGENTS.md)**: the project's rules (asset policy, clean room, never crash,
   architecture). They bind you, except where the fork rules below change them.

## Fork rules (these win over AGENTS.md and craftrules on git, publishing and asking)

- **Branch per task.** Never commit or push to `main`. Push your branch to `origin` (the fork) and
  open a pull request into `main`; Kieran reviews and merges it.
- **Never push to, or open issues or pull requests on, the original `storytold` repositories.**
  Offering a fix upstream happens only when Kieran asks.
- **Ask Kieran before** large rewrites, deleting features, adding dependencies or anything
  installed outside the project folder, and publishing or sharing a build.
- **Never edit his test decks.** Work on copies (in a scratch or `out/` folder) and write results
  beside them. Never commit any PowerPoint-made deck (AGENTS.md §1.1); tests build decks in code.
- **No pupil data** (names, marks, photos) in decks, tests, logs, screenshots or commit messages.
- **Keep** `LICENSE*`, `NOTICE` and `ATTRIBUTION.md`. The ArtCraft name, wordmark and logos
  (`docs/brand/`) must be replaced before any build is shared with anyone else.
- **Report honestly.** Back a fix with evidence (a test, an XML comparison of the original and the
  saved file, a render). Say plainly what you could not check, above all how a slide looks in
  PowerPoint itself, which only Kieran can confirm.
- When you're unattended and a decision is genuinely Kieran's, take the safe reading, note it in
  the pull request and in `MATHS.md`, and carry on with work that doesn't depend on it.

## Habits

- Before a pull request: `cargo xtask ci` (fmt, clippy `-D warnings`, tests, layering, assets,
  wasm). If something can't run (e.g. missing system libraries), run the parts that can, at least
  `cargo fmt --check`, `cargo clippy -p <crate> --all-targets` and `cargo test -p <crate>` for every
  crate you touched, and say what was skipped.
- Every user-visible action is a command with tests (`crates/engine/src/cmd/*`). Every bug fix gets
  a regression test.
- Look at what you changed: `deckcraft-cli render FILE --slide N --scale 1 out/sN.png`, or
  `deckcraft --sample --control 7990` + `ui.screenshot` for UI work.
- Fonts come from a `craft-fonts` checkout next to this repo (`.cargo/config.toml`); renders made
  without it use system fonts and can look different.
- Update the status table in `MATHS.md` when a task lands. The upstream progress docs
  (`ROADMAP.md`, `docs/gaps.md`, …) only need updating when you change what they describe.
- `plan/` is gitignored upstream and won't exist on a fresh clone; `MATHS.md` replaces it here.
