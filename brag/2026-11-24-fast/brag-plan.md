# JackiOh "Fast": plan

## Angle
A strategy card game you finish fast, shown rather than claimed. One real game, played start to finish in the client, flies past on the beat. Then the lethal swing at full smoothness, and the rules that make it quick.

- **Hook (first second):** the real board filling up one beat-cut still at a time, under "One game. Start to finish."
- **Highlights:**
  - the lethal swing (Clobberer, 14 attack, into a hero at 9) and the client's own VICTORY;
  - mana tops out at 4;
  - 25-card decks with no duplicates;
  - five lanes and face-down traps.
- **Punchline:** "Short, sharp games." It is the spec's own phrase.
- **Tone:** default, pushed toward chaotic for a drum-and-bass track: hard cuts on beats, a gold flash at each section, a punch-in on every montage cut. Every line holds about 0.3 s a word.

## Facts on screen and their sources
| On screen | Source |
|---|---|
| "Short, sharp games." / "25-card decks." / "No duplicates." | `spec/01-overview.md:7`: "Short, sharp games: 25-card decks with no duplicates, 4 max mana, 30 hero health" |
| "Mana tops out at 4." | `MAX_MANA = 4`, `crates/engine/src/config.rs:41`; §2.3 |
| "Full mana on your fourth turn." | `spec/01-overview.md:24`: "Max mana is the number of turns you have started, at most 4 before modifiers". In the footage p1 shows 1/1, 2/2, 3/3, 4/4 on its four turns (`stills/*/000, 005, 012, 023.json`) |
| "Units fight in five lanes." | The board's five lanes: every play's `zone.lane` is 1–5 in `legal()`, as the board draws them |
| "Traps wait face down." | §1.1, the hidden backrow; in the footage p2 sets Traitorous Blood face down (action 20) |
| "Play the AI in your browser. No account." | `apps/web/index.html:7`, the meta description: "Play the AI in your browser with no account" |
| "jackioh.com" | The only address shown |

Dropped:
- the 75 s turn clock (`TURN_CLOCK_SECONDS`, `crates/shared/src/config.rs:48`). It's true, but it doesn't show speed.
- any game-length number, since the repo doesn't measure one. The client's own "TURN 7" appears in the footage, uncaptioned.
- Hearthstone and Yu-Gi-Oh by name: the landing copy names neither.

## The game (real, seed 12, no handicap)
- `/dev/hotseat?seed=12&a=fast-a&b=fast-b`. Both decks are injected as `seedGame` does (`js/inject.js`, `decks.cjs`). Each is 25 distinct cards with plainly generic names, and every card that can reach a hand or the board was vetted.
  - Cut for their names: Boulder Fist Ogre, Knife Juggler, Flame Tongue Totem, Polymorph, Twisting Nether, Explosive Trap, Explosive Runes, Call to Arms, Reno, The Rock, Gatling Pea, Fed Fauci, Tempo Timmy and the like.
  - Cut because their targets need a choice: the targeted spells.
- Both seats play `js/policy.js`, a deterministic greedy policy:
  - it plays the dearest affordable card into lane 3, then 2, 4, 1, 5;
  - it attacks the hero when it can;
  - otherwise it ends the turn.
- The game:
  - 29 actions over seven turns, from p1's seat;
  - p2 sets Traitorous Blood on turn 6, and it redirects an attack on turn 7;
  - p1's fourth turn (game turn 7, its first at 4 mana) ends it: Pointmaster comes down, three swings, then Clobberer for 14 into 9;
  - final health 17 to −5.
- Footage:
  - a still after every action (`capture.cjs`, landscape 1920×1080 and the phone layout at 540×960 ×2);
  - the last action as the real drag (the client's own arrow), recorded as a 60 fps screencast with the page's clocks, timers, rAF and CSS/Web animations slowed to 0.2×.

## Music
- `edm-2`: `edmDnb`, 172 bpm, F minor (`apps/web/scripts/music/tracks.mjs:2218`). Beat = 0.3488 s.
- Its drums enter at beat 32 (11.163 s; checked from the file's loudness per bar), so the video's audio starts there and every cut falls on the grid from frame 0.
- A low F impact sits under the lethal hit (beat 12), about 2 dB over the music.
- In landscape only, a soft swish peaks on the outro's beat (50).
- No game audio and no voice lines.

## Landscape: 1920×1080, 30 fps, 628 frames (20.93 s = 60 beats)
| Beats | Time | Scene | Text (in at) |
|---|---|---|---|
| 0–8 | 0.00–2.79 | Montage: 16 stills of the real game, one per half beat, from the empty board to the board before lethal; punch-in on each cut | "One game." 0.0; "Start to finish." beat 3 |
| 8–16 | 2.79–5.58 | The lethal swing at 1× page time: the arrow from Clobberer to the hero at 9, release, the hit on beat 12, −14, VICTORY | "LETHAL." slams on beat 12 |
| 16–24 | 5.58–8.37 | p1's four turn starts, cut every 2 beats, ringed on the client's mana counter | "1/1" to "4/4" big; "Mana tops out at 4." beat 16; "Full mana on your fourth turn." beat 18 |
| 24–32 | 8.37–11.16 | Tavern: the 25 faces of the deck that played, landing one by one | "25-card decks." beat 24; "No duplicates." beat 27 |
| 32–42 | 11.16–14.65 | The board mid-game; then p2's face-down trap, ringed | "Units fight in five lanes." beat 32; "Traps wait face down." beat 37 |
| 42–50 | 14.65–17.44 | The VICTORY frame, blurred | "Short, sharp games." beat 42 |
| 50–60 | 17.44–20.93 | Wordmark | "Play the AI in your browser. No account." beat 51; "jackioh.com" beat 53; dip and music fade from 20.25 |

Poster and frame 0: frame 159 (t 5.3 s), "LETHAL." over the client's VICTORY.

## Vertical (short form): 1080×1920, 30 fps, 419 frames (13.97 s = 40 beats), looping
| Beats | Time | Scene | Text (in at) |
|---|---|---|---|
| 0–8 | 0.00–2.79 | Montage of the phone layout's stills | "One game." 0.0; "Start to finish." beat 3 |
| 8–16 | 2.79–5.58 | The lethal swing, phone layout | "LETHAL." beat 12 |
| 16–24 | 5.58–8.37 | Mana 1/1 to 4/4 | "Mana tops out at 4." beat 16; "Full mana on your fourth turn." beat 18 |
| 24–32 | 8.37–11.16 | The deck's 25 faces, 5×5 | "25-card decks." beat 24; "No duplicates." beat 27 |
| 32–40 | 11.16–13.97 | Wordmark; the last beat dissolves into the montage's first board, so the loop flows | "Short, sharp games." beat 32; "Play the AI in your browser." beat 33; "jackioh.com" beat 34 |

- Everything meant to be read sits in y 250–1536 and clear of the right 140 px of the lower half. This was checked with the bands drawn (`render.cjs` with `BANDS=1`).
- Poster and frame 0: frame 159 (t 5.3 s).

## Build
In `work/`:
1. `scan.cjs` (seed search)
2. `capture.cjs land|port`
3. `faces.cjs base <ids>`
4. `comp/land.html`, `comp/vert.html` (`renderAt(t)`, pure in t)
5. `render.cjs`
6. `audio.sh land|vert`
7. `encode.sh land|vert <poster>`
