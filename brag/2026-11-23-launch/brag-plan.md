# JackiOh launch video: plan

## What it is

- **One sentence.** JackiOh is a free 1v1 card game in the browser: Hearthstone-style mana, combat and
  keywords on Yu-Gi-Oh-style lanes, with a hidden backrow of traps.
- **For** card-game players who want a fast duel with real bluffing (the face-down backrow).
- **What sets it apart.** Traps you can't see, and every card has a Radiant form.
- **The funniest claim, in the cards' own words.** My Pawn (core-096): "Reveals when your opponent declares a
  lethal attack on your hero: Cancel it. The rest of their turn is played at random."
- **Visual hook.** Your lethal swing on a hero at 3 health, and the face-down card that answers it.
- **How to get it.** jackioh.com: hotseat, online or against the AI.
- **Share caption.** "You had lethal. They had My Pawn."

## Angle

The hook is a sure win that a face-down card takes away. Open on the real board, the arrow from your unit to a
hero at 3 health, and "LETHAL."; then My Pawn reveals with the client's own Mythic takeover and its rules text
gives the punchline. Then what the game is: lanes, traps, Radiant, 800 cards, and where to play it.

- **Tone:** `default`: punchy, playful, clean, with soft dips between sections and beat cuts inside them.
- **Shape:** 1920×1080, 30 fps, 20.0 s = 600 frames.

## Visual identity (all from the repo)

- **Backdrop:** `.tavern`'s three gradients and `.tavern.app-shell::before`'s planks
  (`apps/web/src/auth/tavern.css`).
- **Display type:** the wordmark's gold gradient (`#fbe7b0 → #ffe39a → #f3c969 → #c9953f → #9b6423 →
  #d9a44c`), clipped to the text with a `#3a1f0c` stroke and the hearth-glow drop shadow, in Alegreya 800.
- **Body:** Inter 600 in parchment `#efe0bf`. **Small caps:** Alegreya 700 in brass `#c9953f`, letter-spaced.
- **Fonts** from `apps/web/public/fonts/`, the wordmark `apps/web/public/brand/jackioh-logo.svg`.
- **Footage:** the real client, served locally. `/dev/hotseat`, from p1's seat at 1920×1080 with the dev bar
  hidden. Card faces from `/almanac?card=<id>` (`inspect-face-base` / `inspect-face-radiant`) at
  deviceScaleFactor 2.

## Music and sound

- `apps/web/public/audio/music/edm-1.m4a`, the game's own track: D minor, 120 bpm (`tracks.mjs`). Its kick
  lands every 0.5 s from 0.000 (checked from the file's peaks).
- 0–20 s of it, fading out from 19.0 s. Every cut is on a beat (0.5 s = 15 frames).
- A soft swish on both flips (2.0 s and 13.0 s), its peak on the beat, about 7 dB under the music. On the
  Radiant flip a faint D-minor chime (D5, A5, D6) sits under the swish, in the track's key.
- No game audio and no voice lines: the browser was muted, and the cards' voice lines stay out (their
  licence isn't cleared).

## The board (real, seed 28)

Injected as `seedGame` does: `window.__jackiohE2E` in localStorage, then `/dev/hotseat?seed=28&a=brag-a&b=brag-b`.
Both seats get `{deckSize: 6, manaBonus: 3, manaCap: 4}`, so 4 mana from turn 1, and p2 gets `heroHealth: 3`.

The suggested decks were changed. From p1's seat its whole hand is on screen, and several suggested names
point at another game or may point at a person ("4-mana 7/7", "Tempo Timmy", "Prem Panther", "Straaza",
"Gravedigger"), so every card in these decks and on the face wall has a plainly generic name:

- **p1 `brag-a`:** Mr. Vanilla (core-008), Greaser (meditative-100), Group leader (traininggrounds-003),
  Selfless Protector (traininggrounds-032), Clobberer (traininggrounds-008), Deft Duelist (core-045).
- **p2 `brag-b`:** My Pawn (core-096), Sheepish (core-041), Rampaging Rhino (meditative-002), Lone Drifter
  (stinkyscapeplus-002), Plate Packer (meditative-073), Arch Mage (meditativeplus-008). No Taunt, no prompts.

Turns: both mulligans kept. Turn 1: p1 plays Greaser, Mr. Vanilla and Group leader, no attack. Turn 2: p2 plays
the coin, Rampaging Rhino and Lone Drifter, then sets My Pawn and Sheepish face down. Turn 3: p1 drags Mr.
Vanilla (5 Attack with Group leader's aura) onto the hero at 3. That's the real declare, through the UI, and
the arrow is the client's own. My Pawn cancels it and plays the rest of the turn at random: Selfless
Protector comes down, Sheepish turns it into a Sheep, and a random swing wins the game anyway. The video
stops on My Pawn's words before that VICTORY.

The declare, the reveal and the random turn were recorded as a 60 fps screencast with the page's clocks,
timers, frames and CSS/Web animations slowed to 0.2×, so the takeover plays back smoothly. Page time 1.61 s is
the release, and 1.75 s is the takeover's first frame.

## Storyboard (cuts on beats)

| # | Time | What's on screen | Text (in at) |
|---|---|---|---|
| 1 | 0.0–2.0 | The real turn-3 board, the arrow from Mr. Vanilla to the hero at 3, pulsing; slow push in. Mr. Vanilla lunges just before 2.0 (page time × 0.875) | "LETHAL." slams at 0.5 |
| 2 | 2.0–6.0 | 2.0: the client's My Pawn takeover, full screen at 1×. From 2.9 the footage keeps playing behind, blurred and dimmed, with the random turn going on in it. My Pawn's real face slides in on the right | small caps "TRAP · MY PAWN" and "Cancel it." at 3.0; "The rest of their turn is played at random." at 3.25; held to 6.0 |
| 3 | 6.0–9.0 | Tavern backdrop; the wordmark rises and glows in | "A duel of lanes, mana and hidden traps" at 6.5 |
| 4 | 9.0–12.0 | Real boards, cut at 9 / 10 / 11: after turn 1 (units in lanes), after turn 2 (two face-down traps), turn 3 live (Sheepish reveals and turns a unit into a Sheep) | "Summon units into lanes." 9.0; "Set traps face down." 10.0 |
| 5 | 12.0–15.0 | Mr. Vanilla's base face (4/4); flips at 13.0 to Radiant (12/12) with a gold glow | small caps "BASE" → "RADIANT"; "Every card has a Radiant form." 13.0 |
| 6 | 15.0–17.0 | A tilted, drifting wall of 56 real faces from all eight sets | "800 cards." 15.0; "Eight sets." 15.5 |
| 7 | 17.0–20.0 | Wordmark | "Play hotseat, online or against the AI." 17.25; "jackioh.com" 17.75 |

Read times (0.3 s a word, from when the line is settled): LETHAL 1.4 s; "Cancel it." 2.9 s; the random line
(9 words) 2.6 s; the tagline (8) 2.3 s; the two captions 2.8 s and 1.9 s; Radiant (6) 1.8 s; "800 cards." /
"Eight sets." 1.9 s / 1.4 s; the outro (7) 2.6 s.

Facts on screen: 800 non-token cards in `crates/cards/catalog.json`, in 8 sets, all shipped. Mr. Vanilla is 4/4
base and 12/12 Radiant. My Pawn's text is quoted from its base face. No address but jackioh.com, no account
(hotseat seats are "You" and "Opponent"), no voice line.

## Build

`brag-output/work/`: `capture.cjs` (board stills and the slowed screencast), `faces.cjs` (almanac faces),
`comp/index.html` (`renderAt(t)`, a pure function of time), `render.cjs` (600 PNG frames), `audio.sh` (the
mix). Stills of every scene and transition are checked before the full render.
