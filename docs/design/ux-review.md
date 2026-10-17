# DeckCraft for teachers: UI/UX review

Written overnight, 16–17 Oct 2026, for Kieran. Everything here was checked against the running app
(screenshots in [`img/`](img/)), not just the source. Where I could not check something, it says so.

**If you read only one section, read "Things that surprised me" and "What I need you to decide".**

## Things that surprised me

1. **The crowded Teacher Mode palette is not in `main`.** The maths tools panel you described lives
   in open PR #20 (`feat/maths-tools`). `main` has only pointer, pen, highlighter, shapes, eraser,
   revealer and timer. I built this branch on top of #20 (so the PR here carries its two commits
   until #20 merges; merge #20 first).
2. **`drive.file` and "browse everything in Drive" cannot both be true.** With the `drive.file`
   scope, Drive's API only returns files the app created or the user picked through Google's
   *Picker* widget. So the in-app list shows what DeckCraft itself saved, not a teacher's whole
   Drive. Opening a deck that already lives in Drive is already possible without any sign-in,
   because Google Drive for Desktop puts Drive on disk (your Recent list is full of those paths).
   Details in area G. This needs your decision.
3. **The timer bug is a layout bug, not a style problem.** The `HH:MM:SS` row was wider than its
   200pt card, so egui wrapped it: a running 4:48 read `:0 :5 / 4 8`. The 1/2/10 buttons and the ✕
   were dark grey on dark grey. Fixed at the root, not restyled.
4. **A dependency trap:** `keyring` v3 has no default credential store. With no platform feature it
   falls back to a mock that forgets the token, and it compiles fine. I enabled the real store per
   platform and added a (manual) test against the real Keychain.
5. **Google's Desktop-app OAuth client comes with a "client secret"** and its token endpoint
   expects it, even with PKCE. Google documents that this secret cannot be confidential in an
   installed app. So the config takes an optional `client_secret`; security comes from PKCE and the
   loopback redirect.
6. **The "ArtCraft" product is still in the fork's DNA.** Home, About and Account all say
   "DeckCraft" with the upstream logo. Per the fork rules I only moved links; the rename is Phase 4.

## What is in the pull request (and what is not)

| # | Commit | Area | State |
|---|---|---|---|
| 1 | This review | all | done |
| 2 | Hide upstream community links | A | done |
| 3 | Subject-pack mechanism | F | done |
| 4 | Teacher Mode palette | B, C | done |
| 5 | Timer | D | done |
| 6 | Get Ready starter | E | done for text questions; diagram questions are a documented follow-up |
| 7 | Settings surface | H | done |
| 8 | Google Drive | G | built and unit-tested against a fake Google; **not tried against real Google** (no client ID). Off by default. |

Not built, deliberately: hiding ribbon tabs (see A), a first-run subject picker (you ruled it out),
the Google Picker (see G), diagram questions (see E), a Get Ready entry in the New Slide gallery or
the Lesson template (see E).

---

## A. The top layer: what a teacher sees first

**What is there now.** Ten ribbon tabs (Home, Insert, Draw, Design, Transitions, Animations, Slide
Show, Record, Review, View), a title bar (Home, Save, Export, Undo, Redo, Start, more; a theme
palette button, comments, search, Present), a status bar (Notes, Comments, a pen icon that is
Teacher Mode, three view buttons, zoom), and the File view (Home, New, Open, Info, Save, Save As,
Print, Export, Close, Account, Options). Screenshots: `ribbon-*-before.png`.

I scored each against "most teachers need it, most lessons". The honest finding is that the
ribbon is PowerPoint's, so the *tabs* are familiar (principle 5) and I left them. What fails:

| Thing | Who needs it | How often | Verdict |
|---|---|---|---|
| **Record** tab: Cameo (disabled), From Beginning, From Current Slide, Learn More | nearly nobody (narration) | rarely | Fails principle 2: a whole tab for three buttons, one permanently greyed. Proposed: fold into Slide Show. *Not moved tonight.* |
| **Draw** tab: pens, eraser, and *Ink to Shape / Text / Math* (all greyed) | few | rarely | Two problems. The greyed buttons are dead furniture. And these are *authoring* ink, while Teacher Mode has *presenting* ink; the same pens in two places with different meanings is exactly what principle 3 warns about. Proposed: keep, drop the three disabled buttons. *Not moved tonight.* |
| Title bar theme-palette button, comments, search | some | some | Fine. |
| Status bar pen icon (Teacher Mode) | most, in lessons | daily | Right place. Keep. |
| **Discord link** on Home, in File ▸ Info, and in About | no teacher | never | Fails. **Moved.** |
| "Source on GitHub", "getartcraft.com/apps/deckcraft" in About/Info | no teacher | never | Fails. **Moved.** |
| `app.links` command, CLI and MCP link output | developers | n/a | Left (not teacher-facing). |

**What I changed (commit 2).** Home, Info and About show the upstream links only if
**Preferences ▸ Links ▸ "Show community links"** is on. It defaults to off. The constants and
code are all still there. New command `app.communityLinks`. Nothing deleted.

**For you to object to:** the About dialog still says "© 2026 ArtCraft Team and the DeckCraft
contributors" (a licence notice; kept) and the app is still named DeckCraft with the upstream icon
(Phase 4).

## B. Authoring tools versus presenting tools

Rule: *authoring* tools create slide content saved in the deck; *presenting* tools are ephemeral,
drawn over a running show, and die with it.

| Tool | Side | Where it lives now | Where it should live |
|---|---|---|---|
| Pen, highlighter, shapes, eraser, revealer, timer | presenting | Teacher Mode ✓ | Teacher Mode ✓ |
| Ruler, protractor, compass, calculator | presenting | Teacher Mode (PR #20) ✓ | Teacher Mode ✓ |
| Number line, coordinate grid, fraction wall, algebra tiles, hundred square | **both**, and today only the presenting version exists | Teacher Mode (ephemeral) | Teacher Mode *as board aids* **and** an authoring version (Insert, saved as native shapes) so a teacher can prepare one. The authoring version is Phase 3 in `MATHS.md`; not built tonight. |
| Equation (LaTeX / OCR import) | authoring | Insert ✓ | Insert ✓ |
| Exam-question layouts | authoring | not built | New Slide gallery |
| **Get Ready starter** | authoring (it is slide content) | was proposed for Teacher Mode | Insert + (later) New Slide gallery. See E. |
| Draw tab pens | authoring ink | Draw ✓ | Draw ✓ |

**Wrong side today:** nothing is *wrongly placed*; the problem is the one tool family that is
needed on both sides (number line, grid) has only the board half. That is a gap, not a mistake, and
it is why a teacher would rebuild a number line on every slide.

## C. Teacher Mode: the palette

**What is there now** (`teacher-palette-before.png`, `teacher-maths-panel-before.png`). The
diagnosis "flat palette growing without structure" was only half right:

- The palette itself is small and fine in length: 12 icons. But they are **34pt**, below the
  roughly 44pt a fingertip needs, **icon-only** (names only on hover, and a touchscreen has no
  hover), with Clear wedged between Eraser and Revealer and no grouping.
- The show/hide tab is **22 × 56 pt** at the screen edge: the control a teacher must find first is
  the smallest.
- The real mess is the **maths panel** (PR #20): "Open Ruler / Open Protractor …" as text links
  *and*, in the same window, a checklist titled "Choose which tools are available". Doing and
  configuring in one box. It also opens on top of the timer.

**What I built (commit 4)** (`teacher-palette-after.png`, `teacher-maths-panel-after.png`):

- Every control is now **44pt**; the tab is 34 × 72; colour swatches 36pt.
- Tools are **grouped with separators**: look (pointer) · mark up (pen, highlighter, shapes) · rub
  out (eraser, clear) · run the lesson (revealer, timer, maths) · move (previous, next) · exit.
- The maths panel is **big toggle tiles** (108 × 52, three across); tap to bring a tool onto the
  slide, tap again to put it away. "Choose tools" is a collapsed section underneath, closed by
  default. The panel opens beside the palette, clear of the timer.
- The maths button exists only with the Maths pack on (area F).

**Considered and not done:** remembering the last tool across sessions (the overlay already keeps
the tool while Teacher Mode is on; cross-session memory risks starting a lesson with a pen armed);
a collapsed "essentials only" palette (once the maths button is pack-gated and the groups exist, 12
icons did not seem to need it; revisit after a real lesson on a board).

**Not checked:** real touch hardware. The four-finger palm wipe from earlier work is still an
untested finger-count approximation.

## D. The timer

**What was wrong** (`teacher-timer-setting-before.png`, `teacher-timer-running-before.png`):
(1) the digit row did not fit the card and **wrapped** (`:0 :5 / 4 8` while running); (2) the
monospace font drew **slashed zeros** and the colons landed outside their columns; (3) quick-set
buttons 1/2/10, "Reset", and ✕ were **dark on dark**; (4) digits were 34pt in a 200pt card; (5)
buttons stayed on screen while counting.

**What I built (commit 5)** (`teacher-timer-*-after.png`):

- Each digit sits in a **fixed-width cell**, so the line cannot wrap or wobble (a test asserts the
  card is the same height at 5:00 and 0:48).
- **Running = digits only**, sized to the screen (16% of its height, 72–200pt). Tap the digits to
  pause. Pause/Reset/✕ come back when the pointer is over it or it is paused.
- **Last 30 s amber, last 10 s red**, then the existing red flash and Glass beep at zero.
- Setting: 52pt dials with 34pt arrows, 1/2/5/10 quick buttons (selected one is blue), a 150 × 52
  green Start. The ✕ glyph is missing from the bundled font and showed as an empty box; it is now
  drawn as lines.
- Default position top-centre, not top-left under the palette.

**Judgement call:** the running timer sits over the slide's top edge. It is draggable. A teacher who
wants it elsewhere drags it once per lesson; I did not add memory of position.

**Not checked:** how it reads from ten metres on a real projector.

## E. The Get Ready starter: where it lives

**Options weighed**

| Option | For | Against |
|---|---|---|
| New Slide gallery layout | familiar PowerPoint idiom | a layout is static; the point is *choosing* topic and difficulty and *rerolling* |
| Lesson template whose slide 1 is a starter | zero-click for the common case | puts maths content in a template shown to every teacher on a neutral install (breaks principle 4) |
| Default deck on launch | maximum discoverability | wrong for non-maths; a surprise |
| **A command + dialog on Insert (Maths pack), producing ordinary slide content** | choice of topic/difficulty, reroll, round-trips | one more button on Insert (only with Maths on) |

**Chosen:** the command/dialog, `slide.getReady`, behind **Insert ▸ Get Ready Starter** when the
Maths pack is on. A template and a gallery entry can both be built *on top of it* later (a template
would simply run the command); I did not, for the principle-4 reason above.

**What it makes.** A slide named "Get Ready" with the title and **four ordinary text boxes** (28pt,
numbered `1.  …`), 2 × 2. They save as normal shapes, so the deck opens in PowerPoint like any
other. The answers go in the **speaker notes** (not on the slide). That is the "flatten on insert"
trade the brief allows: you cannot reroll in PowerPoint itself, but you can in DeckCraft. A test
saves the slide as .pptx, reopens it, and checks all four questions survive
(`a_get_ready_slide_survives_a_pptx_round_trip_as_plain_text`).

**Topic and difficulty** are choosable in the dialog; with no topic chosen the four are spread
across four different topics (a retrieval starter should revisit, not drill). **Reroll** replaces
the Get Ready slide in place with a new random pick (it replaces the whole slide, so edits you made
to it are lost: reroll before you tidy up).

**The bank format** (`assets/questions/bank.json`; the parser is `crates/engine/src/questions.rs`):

```json
{
  "version": 1,
  "questions": [
    {
      "id": "alg-003",            // unique, required
      "topic": "Algebra",         // required; the dialog lists the distinct topics
      "subtopic": "Factorising",  // optional
      "difficulty": 2,            // required: 1 secure, 2 core, 3 stretch
      "text": "Factorise x² + 7x + 12.",   // required, ≤ 600 characters, plain text
      "answer": "(x + 3)(x + 4)",          // optional; goes in the speaker notes
      "needsDiagram": false,      // optional; true = skipped until diagrams exist
      "diagram": { "kind": "triangle-angles", "values": [50, 60, null] }  // optional, reserved
    }
  ]
}
```

Validation refuses a bank with a wrong version, a repeated id, difficulty outside 1–3, empty text,
or more than 20,000 questions / 4 MB. The same command accepts `bank: "/path/to/bank.json"` to use
another file.

**The shipped questions are placeholders.** `assets/questions/bank.json` has 20 questions I wrote
so the feature works (plus one deliberately diagram-dependent one to prove the skip). They are
**not** the ones you approved. Replace the file with your export; it is compiled in, so rebuild.
(If your export's field names differ, tell me or adjust the parser; it is deliberately small.)

**Diagrams.** Many questions mean nothing without a figure, and the bank's figures are
parameterised (the angles in the text must appear on the drawing). Two ways forward:

1. *Native shapes built on insert* (a triangle from `shape.insert` lines plus labelled text boxes,
   so it round-trips and a teacher can drag a label). Achievable for a closed set of diagram kinds
   (angles on a line / in a triangle / at a point, right-angled triangles, circles with a radius,
   number lines, bar charts). Each kind is its own small builder with its own test.
2. *Text only for now.* What I built.

I chose 2 and made the data model ready for 1: `needsDiagram` questions are skipped, never shown
broken, and `diagram` carries the parameters. **Follow-up:** one diagram builder per kind, starting
with the 3–4 kinds your bank uses most. I need to see the real bank to choose them.

## F. Subject packs

**Mechanism** (commit 3; `crates/ui-egui/src/packs.rs`):

- A **pack** is `{id, label, blurb, slots}`; `slots` says what it contributes: Teacher Mode tools,
  New Slide layouts, a ribbon tab, templates. There is one pack, `maths`.
- **Declared** in the `PACKS` table; **switched** by the `app.pack {id, on}` command and by
  *Preferences ▸ Subjects*; **persisted** as a list of ids in `ui.json`; unknown ids in a
  hand-edited or newer file are dropped.
- **Gated** with `app.pack_on("maths")`. Today the Maths pack gates the Teacher Mode maths button and
  panel and the Insert ▸ Get Ready button.
- **Fresh install:** no pack on, so no maths anywhere. The ribbon is exactly PowerPoint's.
- **Principle 4:** packs only *add*. A test switches Maths on then off and asserts the whole UI
  state is byte-identical to before.
- **Adding science / languages / music / PE:** add one entry to `PACKS` and gate its contributions
  on `pack_on`. No other file needs to know.

**Discoverability** (you chose neutral over a first-run picker). A dismissible card at the top of
the Home screen: *"Teaching a subject? Switch on its tools… [Turn on Maths] [Not now]"*
(`home-after.png`). One tap turns Maths on and the card goes; "Not now" hides it for good; it is
also in Preferences ▸ Subjects. A teacher who never opens Home would still find it in Preferences.
**Migration effect:** anyone who used PR #20's maths tools will find them gone until they tap the
card (including you; this is the cost of the neutral default).

## G. Google Drive

**Design (not up for debate, as you said): built as specified.**

- Scope `drive.file` only. Loopback redirect (`127.0.0.1`, random port, one connection) with PKCE
  (S256) and a random `state`; a redirect with the wrong `state` is refused and never reaches the
  token endpoint. Client ID from `DECKCRAFT_GOOGLE_CLIENT_ID` or `~/.config/deckcraft/drive.toml`,
  never compiled in. Tokens in the macOS Keychain; sign-out deletes them and asks Google to revoke.
- **Opt-in at two levels:** Preferences ▸ Google Drive must be ticked, then the teacher must press
  *Sign in with Google…* on File ▸ Account. Until then nothing contacts Google and the Keychain is
  not read (a test with a tripwire keychain proves it). **Off and signed out on a fresh install.**
- **Nothing is uploaded silently.** Only the *Save to Google Drive* button uploads (`.pptx`, into a
  "DeckCraft" folder the app creates).
- The security-posture section of `MATHS.md` now lists every host, when, and what is sent.

**What you need to create** (the run stalls here, as your note said):

1. Google Cloud Console → new project → enable **Google Drive API** → OAuth consent screen
   (*Internal* if your school is Workspace and you are an admin, else *External* with you as a
   test user) → Credentials → **OAuth client ID → Desktop app**.
2. Put the client ID (and the secret Google shows, if any) in `~/.config/deckcraft/drive.toml`:

```toml
client_id = "1234-abc.apps.googleusercontent.com"
client_secret = "GOCSPX-..."   # optional; Google issues one for Desktop clients
```

   or in `DECKCRAFT_GOOGLE_CLIENT_ID` / `DECKCRAFT_GOOGLE_CLIENT_SECRET`. Then Preferences ▸ Use
   Google Drive, File ▸ Account ▸ Sign in.

**The big limitation: `drive.file` cannot browse a Drive.** It returns only files DeckCraft
created or the user picked via Google's Picker. So File ▸ Account lists *what DeckCraft saved*,
not your `02_Algebra/…` folders. Options:

- *Keep as built* + rely on **Google Drive for Desktop** for existing decks. It already works: the
  native Open dialog shows Drive, and your Recent list is full of
  `~/Library/CloudStorage/GoogleDrive-…/My Drive/…` paths. For teachers who have it (your staff do)
  this is better than OAuth: no sign-in, no network code, instant, works offline.
- *Add the Google Picker.* Needs an API key and a small web page served from the loopback listener,
  and it is a second consent surface. Possible; not tonight. Say if you want it.
- *Widen the scope to `drive` (full read).* **Not recommended** and I did not: it lets the app read
  the whole school Drive.

**The unglamorous cases** (each has a test unless noted):

| Case | What happens |
|---|---|
| No network | `Offline`; the teacher stays signed in (a dead network is not a sign-out); a message on File ▸ Account |
| Access token expired | Refreshed silently a minute early; on a 401 refresh once and retry |
| Grant revoked / refresh token dead (`invalid_grant`) | Signed out, keychain cleared, "Not signed in" |
| Same file open in two places | Opening a Drive file that is already open is refused with a message (no duplicate copy); saving to a file that changed in Drive since it was opened is refused (`Conflict`) and uploads nothing. The teacher is told to save a copy |
| Save fails halfway | Uploads are *resumable*: Drive creates/replaces the file only when the last byte lands, so a dropped connection leaves no half-file and the original is untouched; the error is shown and nothing is retried silently. The local deck is never touched by a failed upload |
| Sign-in window closed / denied / 5 min timeout | "Sign-in did not finish"; back to signed out |
| Hostile file name from Drive | Cache path built from the file id plus a sanitised name; `../` cannot escape (tested) |
| Real Keychain access | **Not tested**: the test exists (`--ignored real_keychain`) but touching the Keychain from an unattended run would raise a permission prompt |

**Not checked:** the whole flow against real Google (no client ID); any real Drive response shape
beyond the documented v3 fields; the browser consent screen itself.

**Dependencies added** (all in `crates/drive`; none touch the wasm build): `ureq` 3 (HTTPS client,
rustls, no async runtime), `sha2` 0.10 (PKCE SHA-256), `base64` 0.22 (URL-safe encoding),
`getrandom` 0.3 (secure random for verifier and state), `keyring` 3 (system credential store).
25 packages total enter `Cargo.lock`, mostly rustls/ring and their parts. **`cargo audit` was not
run**: it is not installed on this Mac and installing it is outside the project folder (a fork
rule says to ask first). Please run `cargo install cargo-audit && cargo audit`.

## H. Where the switches live

One settings surface, instead of each feature inventing its own:

| Kind of thing | Where | Items |
|---|---|---|
| **Switches that change what the app offers** | **File ▸ Options → Preferences** | Subjects (Maths tools), Google Drive, Links (community links), plus the existing editing options. Cancel undoes them all. |
| **Things that need an action** | File ▸ Account | Drive sign-in, sign-out, Save to Google Drive, Open from Drive |
| **First-run discovery** | Home card (dismissible) | "Turn on Maths" |
| **Always-visible, one tap** | Status bar | Teacher Mode (unchanged) |
| **Chosen per tool** | Teacher Mode ▸ Maths tools ▸ "Choose tools" | which of the nine maths aids appear (kept from PR #20) |

*What a first run looks like:* a neutral PowerPoint-like app; a card on Home offering subject
tools; nothing about Discord; no Drive anywhere (it is off); no network.

Preferences is getting long (Subjects, Drive, Links, Editing, name). It should get tabs before the
next pack lands. Not tonight.

---

## What I need you to decide

1. **Merge PR #20 first**, or tell me to rebase this onto `main` and drop its two commits.
2. **Drive: keep as built** (app-saved files only + Drive for Desktop for existing decks), **add the
   Picker**, or drop OAuth and ship only a "Google Drive folder" shortcut? (I recommend keeping as
   built; the Picker only if staff really want in-app browsing.)
3. **Maths is now opt-in.** Anyone with PR #20 loses the tools until they tap *Turn on Maths*. OK?
4. **Hide Record and the three greyed Ink buttons?** I proposed it and did not do it.
5. **Get Ready:** I did not add it to the Lesson template or the New Slide gallery (principle 4).
   Do you want it there for maths teachers if the pack is on?
6. **Questions:** drop your bank at `assets/questions/bank.json` (and tell me if the field names
   differ), plus which diagram kinds matter most.

## What I could not check

- How anything looks **projected in a real classroom**, or from ten metres.
- How the palette and timer behave on **real touch hardware or an interactive whiteboard**.
- How an inserted Get Ready slide **opens in PowerPoint itself** (the round trip is tested through
  DeckCraft's own PPTX reader and writer, not through PowerPoint).
- The **Drive flow against real Google** (no client ID), including the real Keychain.
- The **Get Ready dialog and Preferences screens at the very end**: the screen locked in the small
  hours and `ui.screenshot` stopped working, so the Get Ready dialog has tests but no screenshot.
  The Insert tab, Home card, Account page and Preferences were captured.
- **`cargo audit`** (not installed).
- I did not read every ribbon tab closely (Home, Insert, Draw, Slide Show, Record were read by eye;
  Design, Transitions, Animations, Review and View are screenshots in `img/` that I did not
  audit), so the inventory in A is thorough for the tabs I read and a skim for the rest.

## Evidence

Before/after pairs are in [`img/`](img/): `ribbon-*-before`, `about-before`,
`teacher-palette-before/after`, `teacher-maths-panel-before/after`,
`teacher-timer-setting-before/after`, `teacher-timer-running-before/after`, `home-after`,
`account-drive-after`, `preferences-after`, `ribbon-insert-after`, `get-ready-slide`.
I did not commit a "Home before" because that screenshot's Recent list shows a school email address
in file paths; the Discord link it showed is described in A. The screenshots show only the sample
deck (no pupil data).
