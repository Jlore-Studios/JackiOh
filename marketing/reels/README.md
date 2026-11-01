# Reels (proof of concept, #2616)

Makes the raw material for short 9:16 marketing reels, for [Reel Studio](https://github.com/AidenJohnston/reel-studio-app)
to edit. Reel Studio turns a raw talking-head video into a finished reel: it transcribes the voice, cuts the
pauses, adds captions, puts a song under the voice, adds b-roll and a cover, and writes the post caption.
Then you can type notes like "music too loud at the start" to get a new version. This folder gives it JackiOh
footage to work with:

| Step | Script | Output |
|---|---|---|
| 1. Film a game | `capture.mjs` | `out/gameplay.mp4`: 1080x1920, silent. The practice AI plays both seats on the dev hotseat board, shown from p1's side with the dev bar hidden |
| 2. Build the kit | `kit.mjs` | `out/Reel Studio/Library/`: `Raw takes/<script>.mp4` (the gameplay with the script read over it), `B-roll/<script>-gameplay.mp4`, `Music/jackioh-<track>.m4a` |
| 3. Edit | Reel Studio | the finished reel, its cover and its post caption |

```sh
pnpm --dir apps/web dev --port 5173 --strictPort        # a dev build: capture needs /dev/hotseat and the source modules
node marketing/reels/capture.mjs --seed 42 --from-turn 5 --seconds 45
node marketing/reels/kit.mjs --script marketing/reels/scripts/first-look.txt --music edm-start
```

Then in Reel Studio, click **New video** and drop in `out/Reel Studio/Library/Raw takes/first-look.mp4`. Or pass
`--root ~/Documents/Reel\ Studio` to `kit.mjs` to put the files straight into Reel Studio's own Library, where its
AI editor finds the b-roll and the song.

- **Capture** needs Playwright with Chromium (`require('playwright')`; it is not a dependency of this repo) and
  ffmpeg. It plays the turns before `--from-turn` off camera with reduced motion so the clip opens on a busy
  board, then films the AI's moves at the phone layout, with a 0.7 s pause after each one. `--a`/`--b` pick
  the decks (`DECK_IDS`, `apps/web/src/game/decks.ts`), and `--seed` picks the game. The same seed, decks and
  turn always give the same game, because `decide` has no clock (deadline 0).
- **Kit** reads the script one line at a time with the machine's text to speech (macOS `say`, else
  `espeak-ng`). Reel Studio sorts a video as a raw take only if it is at least 12 s long and the voice is above
  -35 dB for at least 35% of it (`probe.py`'s `guess_kind`). `first-look.txt` measures 0.81. A robot voice is
  enough to prove the pipeline works. A real reel needs a person reading the script, on camera or as a
  voice-over, saved as a new file in `Raw takes/`.
- **Music** is the game's own (`apps/web/public/audio/music/`). It was composed for this repository and
  rendered with an MIT SoundFont (`assets/music/LICENSES.md`), so it is safe to post.
- **Scripts** (`scripts/*.txt`) state only facts from the top-level README. Every new claim needs checking
  against the spec.

## What is left open

- Reel Studio's engine (`python -m reelengine init`, `transcribe`, `analyze`, `autocut`, `render --first-look
  --music Library/Music/...`) could run steps 3's no-AI edit from a script, but this proof of concept did not run
  it. Its speech model downloads from Hugging Face, which the cloud session that built this could not reach.
- A real voice, more scripts (a card spotlight, a Radiant reveal, a trap springing), and footage aimed at
  those moments: a fixture board through `window.__jackiohE2E`, not a seeded game.
