---
name: Cloudberry
description: A tiny shared desk radio for a handful of coworkers, drawn as the faceplate of a small hardware device.
colors:
  cloudberry-orange: "#f28322"
  cloudberry-orange-hover: "#e2701a"
  lamp-orange: "#ff6a13"
  lamp-rim: "#b84d00"
  orange-ink: "#a34400"
  blush: "#e0553a"
  danger: "#b02a12"
  chassis: "#e3e3df"
  face: "#f3f3f0"
  well: "#e9e9e5"
  key-cap: "#fbfbf9"
  key-lip: "#b4b4ae"
  graphite: "#151515"
  graphite-2: "#45453f"
  graphite-3: "#5f5f59"
  rule: "#cdcdc7"
  rule-strong: "#84847e"
  lamp-off: "#c4c4be"
  chassis-dark: "#111111"
  face-dark: "#191918"
  well-dark: "#222220"
  key-cap-dark: "#262624"
  ink-dark: "#ecece7"
  ink-2-dark: "#b3b3ac"
  ink-3-dark: "#94948d"
  rule-dark: "#2c2c2a"
  rule-strong-dark: "#76766f"
  orange-ink-dark: "#ff9d55"
typography:
  title:
    fontFamily: "Host Grotesk Variable, Helvetica Neue, Arial, sans-serif"
    fontSize: "1.75rem"
    fontWeight: 600
    lineHeight: 1.08
    letterSpacing: "-0.03em"
  body:
    fontFamily: "Host Grotesk Variable, Helvetica Neue, Arial, sans-serif"
    fontSize: "15px"
    fontWeight: 400
    lineHeight: 1.45
  row-title:
    fontFamily: "Host Grotesk Variable, Helvetica Neue, Arial, sans-serif"
    fontSize: "15px"
    fontWeight: 550
  label:
    fontFamily: "Martian Mono Variable, ui-monospace, monospace"
    fontSize: "0.6875rem"
    fontWeight: 500
    letterSpacing: "0.06em"
    fontVariation: "'wdth' 87.5"
  data:
    fontFamily: "Martian Mono Variable, ui-monospace, monospace"
    fontSize: "0.75rem"
    fontWeight: 400
    fontFeature: "'tnum'"
  readout:
    fontFamily: "Doto Variable, Martian Mono Variable, monospace"
    fontSize: "2.25rem"
    fontWeight: 800
    lineHeight: 0.8
rounded:
  sm: "2px"
  md: "4px"
  lg: "8px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
  2xl: "40px"
components:
  key-play:
    backgroundColor: "{colors.cloudberry-orange}"
    textColor: "{colors.graphite}"
    rounded: "{rounded.md}"
    width: "88px"
    height: "56px"
  key:
    backgroundColor: "{colors.key-cap}"
    textColor: "{colors.graphite}"
    rounded: "{rounded.md}"
    width: "56px"
    height: "56px"
  button-graphite:
    backgroundColor: "{colors.graphite}"
    textColor: "{colors.face}"
    rounded: "{rounded.sm}"
    padding: "0 16px"
  input-slot:
    backgroundColor: "{colors.key-cap}"
    textColor: "{colors.graphite}"
    rounded: "{rounded.md}"
    height: "52px"
  deck:
    backgroundColor: "{colors.face}"
    rounded: "{rounded.lg}"
    padding: "12px"
  toast:
    backgroundColor: "{colors.graphite}"
    textColor: "{colors.face}"
    rounded: "{rounded.md}"
---

# Design System: Cloudberry

## Overview

**Creative North Star: "The Desk Radio Faceplate"**

Cloudberry is a small shared radio that sits in a background tab all day, so the screen is drawn as the faceplate of a piece of hardware in the Teenage Engineering / Nothing tradition: an aluminium-grey chassis, one raised deck object holding the playing track, keys and readouts, and quiet ruled lists beside it. Precision does the work that decoration did before: a strict 4/8px rhythm, 1px rules, small radii, monospaced numbers that never jitter, and exactly one hot colour.

The world lends type, palette, density and one signature move (physical transport keys, a fader, lamps and a dot-matrix readout). It does not lend costume: no screws, grilles, bezels or fake LCD glass. Controls stay real buttons, inputs and lists.

Replaces the Phase 3–4 sticker look (bog green, cream, white die-cut outlines, hard offset drops, tilted covers, Fredoka, drawn emoji stickers). Those are anti-references.

**Key Characteristics:**
- One raised object (the deck); everything else lives on the chassis, divided by hairlines.
- Graphite on grey/off-white in light; graphite faces on near-black in dark.
- Cloudberry orange only on the play key, lamps, the playhead, the "next" index and your own log ticks.
- Grotesk for words, condensed mono for labels/numbers, dot-matrix only for big time readouts.
- Monochrome 1.75-stroke pictograms with square terminals.
- Motion is mechanical: keys dip, digits roll, lamps blink in steps; nothing bounces.

## Colors

Restrained: tinted neutrals plus one orange that always means "live / yours / act here".

### Primary
- **Cloudberry Orange** (#f28322): the play and tune-in keys, the playhead, lit hold-to-skip fill, the poke toast, Undo. Graphite text on it (7:1).
- **Lamp Orange** (#ff6a13) with **Lamp Rim** (#b84d00, light theme only): lit LEDs, the "you" tick and tile corner. Always beside a text label; the rim keeps the lamp at 3:1 on the light faces.
- **Orange Ink** (#a34400 light / #ff9d55 dark): orange used as text (queue index 01, "Next", countdown digits, "You").
- **Blush** (#e0553a): heat only (hold-to-skip near the end, a hot combo lamp).

### Neutral
- **Chassis** (#e3e3df / #111111): the page.
- **Face** (#f3f3f0 / #191918): the deck, chat log, dialogs.
- **Well** (#e9e9e5 / #222220): the recessed screen and hover fills.
- **Key Cap** (#fbfbf9 / #262624) with **Key Lip** (#b4b4ae / near-black): raised keys and fields.
- **Graphite** (#151515 / #ecece7), **Graphite 2** (#45453f / #b3b3ac), **Graphite 3** (#5f5f59 / #94948d): text steps; Graphite 3 is the floor for any readable text (≥4.9:1 on every face).
- **Rule** (#cdcdc7 / #2c2c2a): decorative hairlines. **Rule Strong** (#84847e / #76766f): borders that identify a control (≥3:1).

### Named Rules
**The One Orange Rule.** Orange marks what is live, yours, or the next action. If an element is neither, it is graphite.

**The Labelled Lamp Rule.** A lamp never carries meaning alone; it sits beside a word (On air, Next, Halo, You).

## Typography

**UI Font:** Host Grotesk (self-hosted, @fontsource-variable/host-grotesk)
**Label/Mono Font:** Martian Mono at 87.5% width (@fontsource-variable/martian-mono)
**Readout Font:** Doto, dot-matrix (@fontsource-variable/doto), weight 800

**Character:** A plain, slightly squared grotesk for titles and messages; a narrow mono for everything printed on the panel and every number; dot-matrix only where a device would have a display.

### Hierarchy
- **Title** (600, 1.75rem, 1.08, -0.03em): the now-playing title; 1.375rem on phones. Title card slab: clamp(2.2rem…4.5rem).
- **Row title** (550, 15px): queue and library rows.
- **Body** (400, 14–15px, 1.45): chat text, hints.
- **Label** (mono 500, 11px, 0.06em, uppercase, 87.5% width): panel labels (Up next, Library, Chat, Sync, Next in, Added by) and key captions.
- **Data** (mono, 11–12px, tabular): durations, times, counts, list indices (01, 02…).
- **Readout** (Doto 800, 2.25rem): elapsed time; 1.375rem for the countdown. Colons are drawn by `DotTime.svelte` as two single dots.

### Named Rules
**The Printed Label Rule.** Uppercase mono labels name a module or control, like printing next to a knob. They are never decoration above a heading; the label is the heading.

## Layout

Desktop: header strip (64px, 1px rule) then a two-column grid, deck `minmax(340px, 400px)` + lists `1fr`, gap 40px, max width 1240px with 32px gutters; the chat log docks as a full-height 360px column with a single left hairline at ≥1180px. The deck is sticky. Lists sit directly on the chassis, rows separated by hairlines, sections 40px apart; each section opens with a label + 2-digit count over a rule.

Phone (≤560px): one column, 16px gutters; the screen becomes 16:11; volume is hidden (hardware volume); transport keys move to a fixed bottom bar (lyrics | play | skip). Chat becomes a bottom sheet (a side sheet from 640px).

## Elevation & Depth

Flat by default: depth comes from tone (chassis → face → well → key cap) and 1px rules. Keys get a 1px lower lip (`0 1px 0` key-lip; the play key `0 2px 0` darkened orange) that collapses when pressed. Only objects lifted off the panel get a real shadow.

### Shadow Vocabulary
- **Lift** (`0 10px 24px -10px rgb(0 0 0 / .28), 0 2px 6px -2px rgb(0 0 0 / .12)`): drag clone, toasts, dialogs, title card, chat sheet.

### Named Rules
**The Lip Not Drop Rule.** Raised means a 1–2px lower lip in the key's own darker tone. No offset block shadows, no glows.

## Shapes

Small, consistent radii by role: 8px modules (deck, gate plate), 4px keys/fields/screen, 2px tiles, art, chips. Album art is square, crisp and untilted everywhere except Halo mode, where it becomes the round label of a disc inside the spectrum ring. Lamps are 6px circles; the "you" tick is a 6px square.

## Components

### Keys
- **Play key:** orange, 88×56, graphite glyph, 2px darker lip; press sinks 2px. Phone: full-width third of the bottom bar.
- **Key:** off-white cap, inset 1px rule, 1px lip; press sinks 1px. Toggled keys (lyrics) light a 5px lamp in the corner.
- **Hold-to-skip key:** a key whose fill rises from the bottom while held (raw, no easing; it is the timer), orange → blush.
- **Captions:** 10px mono under transport keys (Pause, Hold skip, Lyrics, Vol).

### Fader
Volume: a 2px graphite track over 10% tick marks, a 14×28 key-cap thumb with an orange index line.

### Screen
A recessed 4px well holding the art and the in-card visualizer; status lamp + label top-left; combo readout top-right; visualizer soft keys (Halo / Bloom / Bars with lamps + lean-back) printed along its bottom edge.

### Meter
Progress is a segmented meter (3px cells, 1px gaps) filled graphite, with a 4px orange playhead. Driven by raw rAF transforms.

### Lists
Queue rows: index (01…, orange for the next one) that swaps to a grip on hover, 40px art, title + mono meta, mono duration, actions on hover/focus. Library: ruled table with an outlined + key per row. Just played: 96px square covers with a mono -01 index.

### Chat log
Grid of time (24h mono) | who | text. Own lines: orange square tick + "You"; pending: hollow tick blinking in steps; echoed: tick latches solid; failed: blush tick, struck text, Retry. Events are single mono lines. Composer: `>` prompt, graphite send key.

### Inputs
Key-cap field with a Rule Strong 1px border that turns graphite on focus; the add slot has a printed "Link" tag and a graphite Add key.

### Toast / Title card / Tooltip
Graphite strips with face-coloured text; Undo is an orange key. Pokes use an orange strip. Tooltips are 11px mono on graphite.

### Pictograms
`web/src/lib/icons/Icon.svelte`: 24 grid, 1.75 stroke, square caps, mitred joins, currentColor; transport glyphs solid. Reactions (flame, laugh, heart, clap, party, zzz) are the same set.

## Do's and Don'ts

### Do:
- **Do** keep orange to live/yours/next-action elements (The One Orange Rule).
- **Do** set every number in mono with tabular figures; big time readouts in Doto via `DotTime`.
- **Do** separate modules with 1px rules and tone, not cards and shadows.
- **Do** keep motion mechanical: 50–360ms, outQuart/outExpo/linear/steps, no overshoot; every helper honours `prefers-reduced-motion`.

### Don't:
- **Don't** bring back stickers, die-cut outlines, hard offset drops, tilts, bog green, cream or Fredoka.
- **Don't** use emoji or a stock icon library; draw new pictograms in the Icon set's grammar.
- **Don't** add costume hardware (screws, grilles, bezels, fake LCD glare).
- **Don't** use `outBack`/`outElastic` or any bouncing ease.
