# JackiOh "Flex" video: plan

## What it is

- **One sentence.** JackiOh is a 1v1 card game of lanes, mana and hidden traps. Every card has a ladder of looks
  to climb, and your account dresses its board, backdrop and card back.
- **For** card-game players who like to show off a collection.
- **What sets it apart.** You don't buy the looks. Spec §14's opening says "Real money is out of scope: nothing
  here is sold, and nothing is bought but with Jlore". §14.1 says Jlore is "earned only by playing … never sold,
  and no money becomes Jlore".
- **Hook.** One card climbing its looks on the beat: Default, then Alt art, Alt full art, Animated and Signed.
- **Punchline.** The Jlorious card back. Nothing drops it; only reaching Jlorious, the ladder's top 100, earns it.
- **Share caption.** "Level up your cards. Earned by playing, never sold."

## Tone, shape, sound

- **Tone:** `default`, punchy and quick, with cuts and switches on the beat.
- **Landscape:** 1920×1080, 30 fps, 20.0 s (600 frames).
- **Vertical:** 1080×1920, 30 fps, 13.7 s (411 frames), looping.
- **Music:** `apps/web/public/audio/music/edm-5.m4a`, the game's own trap track (`edmTrap` in
  `apps/web/scripts/music/tracks.mjs`: 140 bpm, G harmonic minor).
  - Its first 32 beats are a quiet intro and the drop lands on beat 32 (13.714 s, checked from the file's
    envelope).
  - Both cuts start the track at its beat 30 (12.857 s), so the drop hits on the hook's first change, at 0.857 s.
  - Every beat sits on k × 0.4286 s from frame 0.
- **Effects:** a soft swish peaks on each section join's beat. A faint G-minor chime (G5, D6, G6) sits under the
  Signed reveal and the Jlorious back.
- **Ending:** the landscape cut fades out from 19 s. The vertical cut loops, so its ending turns back into its
  first frame.
- No game audio and no voice lines.

## Visual identity (from the repo)

- **Backdrop:** `.tavern`'s gradients and planks (`apps/web/src/auth/tavern.css`).
- **Type:** gold display type in Alegreya 800 (the wordmark's gradient), Inter 600 parchment `#efe0bf`, and
  Alegreya 700 brass `#c9953f` caps. Fonts from `apps/web/public/fonts/`.
- **Logo:** `apps/web/public/brand/jackioh-logo.svg`.
- **Rarity colours:** the client's (`apps/web/src/cards/cards.css`):
  - Common `#d9dde6`
  - Rare `#3d8bfd`
  - Epic `#a45cf0`
  - Legendary `#ff9f1c`
  - Mythic `#ff5fd2`

## Footage (all real UI)

- **Collection, signed in.** Served locally (`pnpm --dir apps/web dev`) against the `E2E=1` test server, signed in
  as the fixture account `e2e-p1`. No account name appears in any shot: only card faces and cosmetic pictures are
  filmed, never the page's header.
- **Card view.** Nature Titan's card view (`collection-card-view-face`). Each look is shown by focusing its rung
  or strip item ("Hover or focus a cosmetic to see it on the card").
- **Cosmetics tab.** The Cosmetics tab's fullscreen previews (`collection-preview__picture`) for:
  - the Astral playmat;
  - the Cosmic drift backdrop;
  - the Nebula and Jlorious card backs.
- **How it was recorded.** The page's clocks, timers, frames and CSS/Web animations were slowed to 0.1×, and each
  look recorded as a 30 fps page-time sequence of element shots. So the animated level, the signed level and the
  chase foils, finishes and borders move as they do in the client.
- **Resolution, by cut:**
  - Landscape: at 1920×1080, deviceScaleFactor 2 (faces 630×880).
  - Vertical: at 540×960 (the client's phone layout), deviceScaleFactor 4 (faces 860×1200).
- **Card names.** Nature Titan is the only card shown. Its name is generic.
- **What stays out.** The Collection's grid of cards stays out of every shot: its names include ones that point
  at a real person or a company. So do the hero portraits ("Tempo Timmy", "Jlockeed Shredder-10").

## Facts on screen and their sources

| On screen | Source |
|---|---|
| Ladder: Default (every card), Alt art Rare, Alt full art Legendary, Animated Mythic, Signed Mythic | §14.2 table, the Alt art row; "the Default on every non-token card" (§14.2, offers); Nature Titan's own rungs in its card view |
| Cracked ice (Legendary), Molten gold (Legendary), Void shimmer (Mythic), Galaxy (Epic) foils; Gilded edge (Legendary), Starlit (Mythic) finishes; Filigree (Legendary), Celestial (Mythic) borders | `crates/shared/cosmetics.json` (via `apps/web/src/wire/generated/cosmetics.ts`), as the card view's strips print them |
| "One card. Eight looks." | Nature Titan's card view lists eight variants: Default, Full art, Alt art, Comic, Out of frame, Night, Watercolor, Tarot |
| Playmat Astral, Backdrop Cosmic drift, Card back Nebula, all Mythic | the registry and the Cosmetics tab (it calls tables "Playmat" and fields "Backdrop") |
| "Earned by playing. Never sold." | §14's opening ("nothing here is sold, and nothing is bought but with Jlore"); §14.1 (Jlore "earned only by playing … never sold, and no money becomes Jlore"); §14.6 (a cosmetic pack is "earned only … nothing sells one") |
| "The Jlorious back? Top 100 only." | §14.2: `card-back-jlorious` "is owned only by reaching Jlorious"; §9 (`spec/09-architecture.md`): Jlorious is "the top 100 placed Mythic Grape players by hidden rating" (`JLORIOUS_SIZE` = 100) |
| jackioh.com | the only address shown |

## Landscape storyboard (beats of 0.4286 s)

| # | Time | Beats | On screen | Text |
|---|---|---|---|---|
| 1 | 0.00–3.43 | 0–8 | Nature Titan's face, left, with a slow push-in. On the right, its Alt art ladder as five rungs. The face switches look on beats 2 (the drop), 4, 5 and 6, and the current rung lights in its rarity's colour | "Level up your cards." from frame 0 (the rungs are labels) |
| 2 | 3.43–6.86 | 8–16 | The same face, dressed one beat at a time: Cracked ice, Molten gold, Void shimmer and Galaxy foils, Gilded edge and Starlit finishes, Filigree and Celestial borders. A chip names each with its rarity | "Foils. Finishes. Borders." at 3.43 |
| 3 | 6.86–10.29 | 16–24 | Dip, then the card's eight variants deal in, one every half beat, in a 4 × 2 grid with their names | "One card. Eight looks." at 6.88 |
| 4 | 10.29–13.71 | 24–32 | Dip, then the Astral playmat slides in. At beat 26 it becomes the Cosmic drift backdrop. At beat 28 the Nebula card back flips in | "Dress your board too." at 10.31; chips under each |
| 5 | 13.71–18.00 | 32–42 | Dip, then the Jlorious card back rises in, violet glow and sparks, floating | "Earned by playing. Never sold." at 13.76; "The Jlorious back? Top 100 only." at 15.43 |
| 6 | 18.00–20.00 | 42–46.7 | The wordmark | "Flex it at jackioh.com" at 18.12–18.2 |

**Read times.** At 0.3 s a word, from settled:

| Line | Time on screen |
|---|---|
| Hook (4 words) | 3.4 s |
| Foils (3) | 3.2 s |
| Eight looks (4) | 3.2 s |
| Board (4) | 3.2 s |
| Earned (5) | 1.5 s |
| Jlorious (6) | 2.4 s |
| Outro (4) | 1.6 s |

**Poster:** frame 96 (3.2 s). It holds the Signed alt full art, every rung lit and the caption settled.

## Vertical storyboard (the short-form cut, a loop)

| # | Time | Beats | On screen | Text |
|---|---|---|---|---|
| 1 | 0.00–3.43 | 0–8 | The face, large and centred; the look switches on beats 2 (0.857 s, the hook's first beat), 4, 5 and 6 | "Level up your cards." from frame 0; a chip with the level and its rarity |
| 2 | 3.43–5.57 | 8–13 | Dressed one beat at a time: Cracked ice, Void shimmer, Gilded edge, Starlit, Celestial | "Foils. Finishes. Borders." |
| 3 | 5.57–8.14 | 13–19 | Dip, then the eight variants deal in, 3 / 3 / 2 | "One card. Eight looks." |
| 4 | 8.14–12.00 | 19–28 | Dip, then the Jlorious card back | "Earned by playing. Never sold." at 8.19; "The Jlorious back? Top 100 only." at 9.86 |
| 5 | 12.00–13.70 | 28–32 | The wordmark and the url. From 13.3 s a dip, then the hook's opening fades back in | "Flex it at jackioh.com" |

- **Safe zone.** Everything meant to be read sits between y 262 and 1530. Nothing readable sits in the right
  140 px of the lower half (checked on a still with the bands drawn).
- **Poster:** frame 0, the loop's own first frame, so the seam shows nothing new.
