# JackiOh lethal shorts: plan

Three vertical cuts of the launch video's hook, "You had lethal. They had My Pawn." The short-form way to find the
opening that holds people is to test several on one body. So the cuts share everything after their first ~2 s and
differ only in the hook.

## What it is

- **One sentence.** JackiOh is a 1v1 card game of lanes, mana and hidden traps, played in the browser at jackioh.com.
- **The claim, in the card's own words.** My Pawn (core-096, a Mythic trap): "Reveals when your opponent declares
  a lethal attack on your hero: Cancel it. The rest of their turn is played at random."
- **Shape.** 1080×1920, 30 fps, 14.0 s = 420 frames, each one a loop: the last 0.25 s dips out of the outro and
  back into the hook's first frame.

## Footage (real, seed 28, the phone layout)

The same game as the launch video: decks `brag-a` and `brag-b`, the same handicaps (4 mana from turn 1, p2's hero
at 3) and the same turns. This time it's played at a portrait viewport, 540×960 CSS px at deviceScaleFactor 2, so
the client draws its phone layout: the hero bars top and bottom, five lanes of two rows each side, the hand and
End turn below. Nothing is cropped from the landscape captures.

- The declare is a real drag from Mr. Vanilla onto the opponent's portrait. On the phone layout the portrait sits
  at the bar's left end, so the drag aims 24 px in from the bar's edge. The arrow is the game's own.
- The reveal and the random turn are a 60 fps screencast with the page slowed to 0.2×. Chrome runs with
  `--force-device-scale-factor=2`, since without it the screencast sends 540×960 frames.
  - Release at page time 1.61 s; the takeover's first frame is 107 (1.78 s).
- Board stills after turn 1 (`board-t1`) and turn 2 (`board-t2`, two face-down traps).
- Faces from `/almanac` (the launch video's set; the same vetted names).
- Board footage sits 180 px down, so the hero at 3 clears the platforms' top bar.

## Hooks (0–2.0 s)

| Cut | Hook | What it tests |
|---|---|---|
| A | The aimed arrow on the hero at 3, a slow push in, "LETHAL." slams on the 0.5 s beat | stakes first |
| B | The same footage, with two caption boxes in the platforms' own style: "You swing for lethal." (0.05 s), "They flip **this**." (1.0 s). They hold into the takeover, to 2.3 s | text first, read before anything moves |
| C | Cold open on the takeover (0–1.0 s), a rewind with ◀◀ (1.0–1.5 s), then the aim plays from "One second earlier" (1.0–2.15 s) into the release | curiosity gap |

## Body (shared, 2.0–14.0 s; cuts on the 120 bpm beat)

| Time | On screen | Text (in at) |
|---|---|---|
| 2.0–3.0 | My Pawn's own Mythic takeover, full screen at 1× | |
| 3.0–6.5 | The random turn keeps playing behind, blurred; My Pawn's face rises in | "TRAP · MY PAWN", "Cancel it." 2.9; "The rest of their turn is played at random." 3.3 |
| 6.5–7.5 | The board after turn 1; the five lanes light up one by one | "Five lanes." 6.5 |
| 7.5–8.5 | The board after turn 2, pushing in on the two face-down traps | "Hidden traps." 7.5 |
| 8.5–10.0 | Mr. Vanilla's base face (4/4) flips at 9.0 to Radiant (12/12) | "Every card goes Radiant." 8.55 |
| 10.0–11.5 | A tilted, drifting wall of real faces from all eight sets | "800 cards." 10.0; "Eight sets." 10.5 |
| 11.5–14.0 | The wordmark on the tavern backdrop | "Play online or against the AI." 11.7; "jackioh.com" 12.0 |

- **Read times** (0.3 s a word, from when the line is settled):
  - "LETHAL." 1.5 s;
  - B's lines 2.1 s and 1.2 s;
  - "One second earlier" 1.0 s;
  - "Cancel it." 3.4 s;
  - the random line 2.9 s;
  - the four quick beats 0.8–1.3 s, each of two to four words;
  - the outro 1.9 s;
  - the address 1.7 s.
- **Safe zone.** Every line meant to be read sits between y 250 and 1536 and out of the right 140 px of the lower
  half. This was checked on banded stills of every scene. The client's own "My Pawn" title sits lower, at y ≈ 1660, as
  part of its takeover; the video's own text says it again inside the zone.

## Sound

`edm-1.m4a`, the game's own track (D minor, 120 bpm, its kick on every 0.5 s from 0), runs from 0 s and fades over
its last 0.4 s so the loop restarts clean. A soft swish peaks on each flip's beat (2.0 s, 9.0 s), with a faint
D-minor chime under the Radiant flip. In C, a reversed swish sits under the rewind. There is no game audio and no
voice line.

## Facts on screen

- Five shared lanes (§1.1).
- Every card has a Radiant face (§1.1).
- Mr. Vanilla is 4/4 base and 12/12 Radiant.
- 800 cards in eight sets.
- My Pawn's text is quoted from its face.
- There is no claim of price ("free" is not on the landing page) and no address but jackioh.com.
- There is no account: the hotseat seats are "You" and "Opponent".
