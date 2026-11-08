# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users
Toby and 3–4 coworkers. They have Cloudberry open in a browser tab while working,
usually at their desks, sometimes on a phone. Job: listen to the same music at the same moment,
throw songs into a shared queue, react and chat about what's playing, without leaving their work.

## Product Purpose
A tiny self-hosted shared radio. Open it, pick a name, press **Tune in**, and hear exactly what
everyone else hears, in sync. Anyone can add (paste a YouTube link), skip, reorder, pause. Success:
it feels like sitting in the same room with a shared speaker, and it's fun enough that people keep
it open all day.

## Positioning
Private, tiny and tight: a handful of friends, perfectly synced playback (within a few ms) on a
home server, with personality. Not a streaming service, not a public social app.

## Operating Context
- Lives in a background browser tab most of the day; glanced at, then interacted with in bursts
  (adding a song, reacting, skipping).
- Desktop first, phone width must work. Light and dark mode.
- Self-hosted on Toby's home server (Dockge, `http://10.0.0.163:2279`), private network only;
  identity is just a display name, no accounts.

## Capabilities and Constraints
- One shared station: now playing, upcoming queue, library, recently played, listeners present.
- Paste a YouTube link → server downloads MP3 → library + queue. Spotify is out of scope for now.
- Everyone can play/pause/skip/reorder/remove; no DJ role.
- Side chat with action events, emoji reactions, pokes, away state, DJ of the day, lyrics when found.
- Audio visualizer (WASM analysis) with Halo / Bloom / Bars modes and a lean-back fullscreen mode.
- Stack (fixed): Rust back end, SvelteKit SPA (Svelte 5), Bits UI, plain CSS tokens, anime.js v4 for
  motion, Rust→WASM for audio analysis and "smear" motion physics. Server RAM must stay tiny.
- Browsers need a user gesture before audio, so "Tune in" is a real step.

## Brand Commitments
- Name: **Cloudberry** (not "Cloud Bear"). The cloudberry fruit is the motif.
- Colour: true cloudberry amber-orange as the single accent (`#F0912B` family) with an "unripe
  blush" red (`#E0553A`) for heated moments. Not generic orange.
- Toby's direction, verbatim themes: "clean aesthetic", "a lot of motion", "a shit load of motion
  using anime.js", "smear motion using wasm", "should feel like a game UI, super clear".

## Evidence on Hand
No real users' content, testimonials or metrics exist; never fabricate any. Track content comes
from whatever YouTube links people paste.

## Product Principles
1. Sync is sacred: nothing visual may compromise audio timing or main-thread smoothness.
2. Clear first, playful second: a glance tells you what's playing, who's here, what's next.
3. Every action answers back: immediate, visible feedback for every press, add, react.
4. Tiny footprint: lean server, no needless dependencies, pause work when the tab is hidden.
5. Made for a few friends: warmth and in-jokes over growth features.

## Accessibility & Inclusion
Respect `prefers-reduced-motion` everywhere (heavy motion is opt-out by OS setting), full keyboard
use, readable contrast for accent text in both themes, chunky hit targets.
