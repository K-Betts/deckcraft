# DeckCraft design system: "Fresh Paper"

The look chosen for DeckCraft: **modern, fresh, clean, but familiar and easy to use.** Light first,
airy, one confident accent colour, and a ribbon with room to breathe. This page is the source of
truth; `crates/ui-egui/src/theme.rs` is its code.

Mockups (private canvas, direction A): https://claude.ai/artifact/VUQnDJTy7f7qU6SkwaUTuB

## Principles

1. **Familiar first.** The ribbon, tabs, thumbnails-left, slide-centre, notes-below layout stays. Anyone
   who has used PowerPoint should not have to learn anything. We change *how it looks and breathes*,
   not where things are.
2. **Room to breathe.** More space between groups, larger hit areas, fewer lines. Groups are
   separated by whitespace and a small label, not by boxes and rules.
3. **One accent.** A single cobalt blue says "this is what you are doing / what you can press".
   Everything else is neutral. Colour in the *content* (the slide) stays the slide's.
4. **Soft, not flat.** Rounded corners and gentle shadows lift the ribbon and dialogs off the
   canvas, so the slide is the brightest thing on screen.
5. **Legible from the back of the room.** Teacher Mode text and controls follow the board rules in
   `ux-review.md`: at least 44pt targets, high contrast, no hover-only information.
6. **Never break the round trip, never lose a feature.** This is a skin: every command, shortcut
   and menu stays.

## Colour

Light is the default; dark is the same system, darker.

| Token | Light | Dark | Used for |
|---|---|---|---|
| `chrome` | `#F6F7F9` | `#16181D` | Window background, title bar, side panes |
| `card` | `#FFFFFF` | `#1F222A` | Ribbon, dialogs, popovers, thumbnails |
| `card_border` | `#E6E8EE` | `#2E323D` | Hairline around cards |
| `canvas` | `#F6F7F9` | `#101216` | The area around the slide (same as chrome: the slide carries the shadow) |
| `text` | `#14161C` | `#ECEEF3` | Body and labels (16:1 on white) |
| `text_dim` | `#5B6070` | `#B3B8C6` | Secondary text (6:1) |
| `text_faint` | `#6A7082` | `#8B91A1` | Group labels, captions (4.5:1: never for essential text) |
| `border` | `#D8DCE6` | `#3A3F4C` | Fields, outlines |
| `separator` | `#E6E8EE` | `#2E323D` | Rules (used sparingly) |
| `hover` | `#EEF2FF` | `#262B3A` | Hover wash: a faint tint of the accent |
| `pressed` | `#E0E7FF` | `#2F3650` | Pressed |
| `selected` | `#E3EAFF` | `#27304F` | Selected tab, thumbnail, tile |
| `accent` | `#2B59FF` | `#6C8CFF` | Primary buttons, active tab text, selection ring, links (white text on it is 5.6:1) |
| `contextual` | `#1F3FBF` | `#8FA8FF` | Contextual tab text |
| `field_bg` | `#FFFFFF` | `#1A1D24` | Text fields |
| `shadow` | black 7% | black 35% | Elevation |

Status colours (for later): success `#16A870`, warning `#F2B01E` (timer amber), danger `#E5484D`.

**Rules:** text on any fill must meet 4.5:1 (3:1 for 18px+ or bold 14px+). Never tell two things
apart by hue alone. Dark mode is a full citizen, not an afterthought.

## Type

One family, **Inter** (already embedded; Regular, SemiBold, Bold). The mockups used Figtree for a
friendlier feel; adding it means forking `craft-fonts` (not done: your call).

| Role | Size | Weight | Use |
|---|---|---|---|
| Display | 22 | Bold | Page titles (Home, dialogs' first line) |
| Heading | 16 | SemiBold | Section titles |
| Body | 13 | Regular | Everything in the chrome |
| Label | 12 | SemiBold | Ribbon button labels |
| Caption | 11 | SemiBold, uppercase, +6% tracking | Ribbon group labels |

Board sizes (Teacher Mode): timer digits 72–200pt, tile labels 15pt, palette labels 11pt under 44pt icons.

## Shape, space, elevation

- **Radius:** 6 (small controls), **8** (buttons, fields), **12** (ribbon buttons, cards, tiles),
  **16** (ribbon card, dialogs), pill (Present, tabs, chips).
- **Space scale:** 4 · 8 · 12 · 16 · 24 · 32. Between ribbon groups: 24–32. Inside a group: 4.
- **Hit targets:** 32pt in the desktop chrome; **44pt** anywhere a finger or a board is expected.
- **Elevation:** *rest* = hairline border; *raised* (ribbon, dialogs) = 0 1 2 / 0 8 24, 5–7%; *floating*
  (Teacher Mode palette, timer) = 0 18 50, 30–35%.

## Components

**Title bar.** 52pt. Quick actions left (icon only, 32pt), document name centred with a quiet
"Saved", search, and **Present** as a filled pill (the one primary action on screen).

**Tabs.** Text only, 14pt SemiBold, 32pt tall; the active tab is a white pill with the accent
colour as text. No underline bar, no tab borders.

**Ribbon.** A floating white card (16 radius, raised shadow, 16pt side margins), about 112pt tall.
Large buttons are a 26pt icon over a 12pt label in a 68pt-wide hit area, hover wash behind. Small
buttons sit in a column beside. Each group has a quiet 11pt uppercase label under it; groups are
separated by space, not rules. A chevron at the right collapses it.

**Thumbnails.** 8pt radius, hairline border; selected = 2pt accent ring with 2pt offset. Section
names are small caps in `text_faint`.

**Slide.** The brightest object on screen: white, 6pt radius, soft raised shadow on the quiet canvas.
**Notes and status** sit in a white strip and a borderless bar.

**Dialogs and popovers.** White, 16 radius, floating shadow, 24pt padding, primary button on the
right as a filled accent button, secondary as plain text.

**Home.** White left rail with the app mark, a friendly greeting, theme and template cards with a
soft shadow (hover lifts), the subject card as a quiet accent-tinted banner.

**Teacher Mode.** Floating dark palette (92% ink, 28pt radius, 52pt buttons, hairline group
separators, accent for the active tool, red exit); timer is digits only on an ink pill; the maths
panel is large white tiles; Maths tiles use the accent when open.

## Motion

Keep it small: 120ms ease-out for hover washes and press; 200ms for panels opening. No bouncing.
Respect the OS "reduce motion" setting.

## Accessibility

Every icon-only button has an accessible name. Focus is always visible (2pt accent ring). Contrast
as above. Nothing is hover-only. Targets as above.

## Rollout order

1. **Foundation** (done): colour, shape, space and type tokens; global widget style; this document.
2. **Chrome** (done): tab pills, the floating roomy ribbon card, group gaps instead of rules. Still to do here: a *responsive* ribbon (groups folding into a "more" menu in narrow windows) and group labels under each group.
3. **Home and dialogs** (done): white side menu with an accent pill for the current page, grey page with white raised cards that lift on hover, a rounded accent banner, filled accent main buttons and quiet Cancel, roomy 20pt window padding. Still to do: thumbnails, notes, task panes.
4. **Teacher Mode:** palette, timer, maths tiles restyled to the system.
5. **Icons and polish:** icon weight and any Figtree upgrade; dark-mode pass; system light/dark.

## Not changing

Fonts used *inside slides* (they belong to the deck), the slide renderer, file formats, commands,
shortcuts, menus.
