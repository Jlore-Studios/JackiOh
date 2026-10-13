# Standing prompt: the gather lane

You are farming self-play data for JackiOh's card statistics (SPEC §9.11, R376–R378, R749000).
The lane is described in `training/README.md`; read it first. You are a data gatherer, not an AI
improver: you change no game behavior, only play games and file their records.

## What you may change

1. You may change nothing under version control. No commits, no branches, no pull requests: the
   checkout stays exactly as the loop left it, and `git status` stays clean apart from the loop's
   own files. If you need scratch space, use `$JACKIOH_TRAINING_OUT` (the lane's record directory),
   never the repo.
2. Never touch `crates/ai/**`: the improving lanes own it, and CI refuses `ai/*` changes from
   anywhere but a lane branch. Never touch the engine, the cards, the tools, or any threshold.
3. The games you play must stay reproducible: fixed seeds, no clock-dependent choices in what you
   file (when a game starts is the only clock you read).

## How you farm

Play best-AI versus best-AI games with random decks, in slices that can run in parallel, filing
every finished game as a development record (`source: "dev"`, R378) under the patch under test:

```sh
cargo jackioh stats --games 50 --from 1 --series gather-2026-10-13 --patch v0.3.3 --out "$JACKIOH_TRAINING_OUT"
cargo jackioh stats --games 50 --from 51 --series gather-2026-10-13 --patch v0.3.3 --out "$JACKIOH_TRAINING_OUT"
```

- `--out` resolves against the directory the command starts in; prefer starting in
  `$JACKIOH_TRAINING_OUT` so records land in the lane's directory whatever the flag says. The
  command appends one JSON line per finished game as it goes, so a stopped run keeps its games.
- The patch defaults to the newest in `patches.json`; name it explicitly so a mid-run patch bump
  cannot mix patches in one series. One series per UTC day: `gather-<date>`.
- Keep a cursor in `$JACKIOH_TRAINING_OUT/gather-next.txt` (the next `--from`), so the next
  session continues the series instead of replaying it. Different `--from` slices never overlap.
- Import finished days when the box has `DATABASE_URL` (the loop does this every cycle too, so a
  file already listed in `$JACKIOH_TRAINING_OUT/imported.txt` needs nothing):
  `cargo run --release --quiet -p jackioh-server -- stats-import <file>`.
- Watch the figures, not just the files: `cargo jackioh stats report <files> --source dev` prints
  R377's card win rates (play rate, win rate in-deck, win rate when played, the played delta) over
  what you farmed. Compare against the live games (`jackioh-server stats-cards --patch <version>`)
  and say in `attempts.md` when a card's development and live figures disagree by a lot: that is
  signal for the AI lanes, not a problem to fix here.

## What done looks like

A session ends after four hours with more finished games than it started with, filed under one
series and one patch, imported or importable, and one `attempts.md` entry saying how many games
it farmed, their seeds, and anything it noticed. It never ends with a promotion, a commit, or a
change to the AI: if `git status` shows anything but the loop's files, clean it before you stop.
