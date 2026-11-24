# JackiOh: the climb to Jlorious (plan)

## What it is

- **One sentence.** JackiOh is a free 1v1 card game in the browser. Its ranked ladder runs from Raisin through five Grape tiers to Jlorious, the top 100.
- **For** card-game players who like a ladder to climb and a trophy to show for it.
- **What sets it apart.** The ladder's own words: you start as a *Raisin*. The top is a numbered top 100, and it pays a card back that nothing else gives.
- **Visual hook.** The real Raisin crest slams in with "You start as a Raisin.", and its placement pips fill on the beat.
- **Share caption.** "You start as a Raisin. Only 100 players get to be Jlorious."

## Angle

The game's own joke carries the hook. Then the climb, crest by crest on the beat, with every division ticking III, II, I. It ends on JLORIOUS, then the trophy: the Jlorious card back. A real win sits between the hook and the climb, because winning is how you climb.

- **Tone:** `default`: punchy and playful, with a slam on every tier and dips between sections.
- **Landscape:** 1920×1080, 30 fps, 19.2 s (576 frames) = 8 bars.
- **Vertical:** 1080×1920, 30 fps, 14.4 s (432 frames) = 6 bars. It loops.

## Facts on screen, and their sources

| On screen | Source |
|---|---|
| "You start as a Raisin." and 5 placement pips | spec/09-architecture.md §9 "The visible tiers" (Raisin until placements are done); `RANK_PLACEMENT_GAMES = 5` (crates/shared/src/config.rs) |
| "Win to climb." | §9 "How pips move": a win gives pips |
| Rotten, Normal, Large, Golden, Mythic Grape, each III → II → I | §9 "The visible tiers" (three divisions each, III at the bottom) |
| "The top 1%" under Mythic Grape | §9: the tiers target percentiles, Mythic 1% (R605). It is the design target, not a measured share |
| "Jlorious", "#1", "Only the top 100 players." | §9 "Jlorious" (R608); `JLORIOUS_SIZE = 100` |
| "Mythic card back", "Earned at Jlorious.", "Yours to keep." | spec/14-economy.md §14.2 "The Jlorious card back" (R2755000): Mythic, owned only by reaching Jlorious, kept after falling out |
| jackioh.com | The only address shown |

## Footage (all real, from the served client)

- **Crests.** `apps/web/src/rank/RankCrest.tsx`, mounted on a 5173 page with the app's own React (`work/pw.cjs`, `work/real.cjs`). Each tier and division and Jlorious #1 is rendered by the component, and its markup goes to `comp/assets/crests.json`. The composition draws that markup with the component's own `rankCrest.css`, sized up. Its Jlorious shimmer runs on the CSS animation, driven by time.
- **The Jlorious card back.** `CardBack.tsx` with `back="card-back-jlorious"`, rendered the same way. Its 4 s `cosmetic-back-breathe` animation is sampled at 30 fps by seeking the Web Animation (`back/`).
- **The win.** `/dev/hotseat?seed=28`, from p1's seat. The decks are the launch video's vetted ones: Mr. Vanilla, Greaser, Group leader, Selfless Protector, Clobberer, Deft Duelist against My Pawn, Sheepish, Rampaging Rhino, Lone Drifter, Plate Packer, Arch Mage. Both seats play with `manaBonus: 3`; p2 has `heroHealth: 3`.
  - Turn 1: Greaser, Mr. Vanilla and Group leader.
  - Turn 2: the coin, Rampaging Rhino and Lone Drifter, with no trap set.
  - Turn 3: Mr. Vanilla is dragged onto the hero, the real declare with the game's own arrow. The hit, the VICTORY banner and the WIN panel follow.
  - The device wears the Jlorious card back: `jackioh.cosmetics.v1`, and the hotseat counts every cosmetic as owned.
  - Recorded with the page slowed to 0.2× (clocks, timers, rAF, CSS/Web animations) as a 60 fps screencast. Landscape is 1920×1080. Vertical is the client's phone layout: 540×960 CSS at 2×, with Chrome on `--force-device-scale-factor=2`, 1080×1920, never cropped.
- No account, leaderboard or real player appears. Hotseat seats are "You" and "Opponent". No voice lines.

## Visual identity

The tavern backdrop (`apps/web/src/auth/tavern.css`). Gold display type is Alegreya 800 with the wordmark's gradient. Body text is Inter 600 in parchment `#efe0bf`; small caps are Alegreya 700 in brass `#c9953f`. The logo is `apps/web/public/brand/jackioh-logo.svg`.

## Music and sound

- `apps/web/public/audio/music/epic-1.m4a`, the game's own track (D minor, 100 bpm, `tracks.mjs`). Its A section, i VI III VII i VI iv V+, takes one bar of 2.4 s per chord.
  - **Landscape:** bars 1–8, fading over the last beat.
  - **Vertical:** bars 1–4, then bars 7–8 through a 40 ms crossfade at 9.6 s. It ends on V+, so the loop resolves into the opening i.
- A soft swish peaks on Jlorious's beat and on the card back's flip. A faint D-minor chime (D5, A5, D6) sits under Jlorious, about 7 dB under the music.

## Storyboard: landscape (19.2 s)

| Time | On screen | Text (in at) |
|---|---|---|
| 0.0–2.4 | Raisin crest slams in; its 5 placement pips fill on the eighths (0.9–2.1) | caps "Ranked" 0.1; "You start as a Raisin." 0.18 |
| 2.4–4.8 | The real win, page 0.8–3.2 s: the arrow, the hit, VICTORY | "Win to climb." 2.46 |
| 4.8–9.6 | Rotten, Normal, Large, Golden Grape, a bar of 1.2 s each; division III on the beat, II and I on the next eighths; the ladder of real crests fills on the left | the tier's name and numeral |
| 9.6–12.0 | Mythic Grape (bar 5 downbeat) | "Mythic Grape", caps "The top 1%" 9.9 |
| 12.0–14.4 | Jlorious #1 slams in with a gold flash and sparks (bar 6 downbeat); the ladder's top lights | "Jlorious" 12.06; "Only the top 100 players." 12.6 |
| 14.4–16.8 | The real Jlorious card back flips in, breathing | caps "Mythic card back" 14.52; "Earned at Jlorious." 14.6; "Yours to keep." 15.0 |
| 16.8–19.2 | Wordmark | "Start your climb." 17.1; "jackioh.com" 17.4 |

## Storyboard: vertical (14.4 s, loops)

| Time | On screen | Text (in at) |
|---|---|---|
| 0.0–1.8 | Raisin slams in at frame 1; pips fill 0.3–1.5 | caps "Ranked"; "You start as a / Raisin." 0.18 |
| 1.8–3.6 | The real win in the phone layout, page 1.1–2.9 s | "Win to climb." 1.86 |
| 3.6–9.6 | Rotten → Mythic, 1.2 s each, III → II → I; a row of real crests fills below | the tier's name, numeral; "The top 1%" |
| 9.6–12.0 | Jlorious #1 (music: bar 7's iv) | "Jlorious"; "Only the top 100 players." 10.2 |
| 12.0–14.4 | The card back (V+); dims over the last 0.2 s into the first frame | caps "Mythic card back"; "Earned at Jlorious."; "jackioh.com" |

Every read line sits in y 250–1536 and left of x 940 below y 960, checked against a still with the bands drawn. Read times run about 0.3 s a word from when a line has settled. "Only the top 100 players." gets 1.8 s in landscape and 1.6 s in vertical.

## Posters

- **Landscape:** t = 13.6, Jlorious #1 with its line.
- **Vertical:** t = 1.6, "You start as a Raisin." with the pips full. It flows into frame 1's Raisin slam, so the loop has no flash.
