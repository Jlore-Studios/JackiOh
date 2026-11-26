# The gather lane

No model runs this lane. `.github/workflows/gather.yml` does every step itself on GitHub's runners,
with no model session and no pull request, so this file is the lane's contract: a change to the lane
updates this file, `gather.yml` and `training/README.md` (The gather lane) together.

The lane is the data side of #636. It improves nothing: it gathers the development games the card win
rates (SPEC §9.11, R376–R378) are read from, so a patch has AI-against-AI figures for every card beside
its live games.

## How it runs

On two of GitHub's runners since #3740, `gather.yml` plays without a pause, `stats --series gather` on
the newest version on `main`, game `n` of run `r` being `gather:<(r - 1) × 100,000 + n>` and each of a
run's two parts playing its own half of that range, while the repository variable `GATHER_ON_PATCH` is
`true`. Each part plays five hours (`GATHER_HOURS`) and files its games as totals only into both
projects' databases 400 at a time as they finish (`GATHER_FILE_GAMES`, about an hour), through
`training/file-records.sh` (R1976000, R1933001), so the newest version's figures grow every hour; each
run starts the next as it ends. Until #3740 it played 5,000 games of each new patch or set (never a
micro) once, on the owner's PC, and until 2026-11-04 the training box ran it too (`loop.sh gather`,
which #4701 removed).

## What it plays

`jackioh stats --series gather` (R378):

- The AI on `main` plays itself, built from `main` as it was when the run started.
- Both decks are dealt by R258's All Random deal (`build_ai_deck` with nothing banned, less the AI's
  soft gate, R1390), at the browser's budget (`AI_BUDGET`) and with no handicap.
- A game's seed is `gather:<n>`, so the lane's games are not the gate's (`<lane>:<tree>:<k>`), not a
  person's pre-release run's (`dev:<n>`) and not any tuning run's, and no two runs or parts play the
  same game (R1976000).
- Each finished game is filed as a record with `source: "dev"`, `mode: "random"`, both pilots `ai`, the
  id `dev:<patch>:gather:<n>` and the patch `cargo jackioh catalog-version` names for the checkout. A game
  that does not finish (the action limit, a panic) files nothing.

## What it never does

1. It changes no file in the repository, commits nothing, pushes nothing and opens no pull request.
2. Nothing reads its records back into the AI, the gate or the shadow ban. The gate plays its own
   seeds (`<lane>:<tree>:<k>`) and every sweep its own; this lane's numbers inform a person, and no
   loop closes on them.
3. It never mixes live and development data (R378, R1451). Its records are `source: "dev"` all of
   them, `stats-import` refuses a file that holds any other, and no live figure counts a development
   record unless the query asks for `--source=all`.
4. It does not reach players except as R654 says: until a patch has `PUBLIC_STATS_MIN_LIVE_GAMES` live
   games, the public card statistics are padded with development games and labeled provisional, and
   once it has them the live games alone are read.

## What it leaves

- Both projects' card statistics, as totals: no row per game (`stats-import --totals-only`, R1976000).
- Each batch's records file, a `GameRecord` per line, kept as the run's artifact `gather-<run>-<part>`
  for `GATHER_RETENTION_DAYS`.

`training/README.md` (The gather lane) says how to read them.
