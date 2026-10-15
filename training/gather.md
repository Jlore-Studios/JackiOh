# Standing prompt: the gather lane

No model runs this lane. `training/loop.sh gather` does every step itself, with no Devin session and no
pull request, so this file is the lane's contract, not a prompt anyone reads: a change to the lane
updates this file, `loop.sh` and `training/README.md` (The gather lane) together.

The lane is the data side of #636. The two other lanes improve the AI; this one improves nothing. It
gathers the development games the card win rates (SPEC §9.11, R376–R378) are read from, so a patch has
AI-against-AI figures for every card beside its live games, whether or not anyone ever trains on them.

## What it plays

`jackioh stats --series gather` (R378), in batches:

- The AI on `main` plays itself. It is the best AI there is, since every promotion lands on `main`, and
  the lane builds `jackioh` from `main` at the start of every cycle.
- Both decks are dealt by R258's All Random deal (`build_ai_deck` with nothing banned, less the AI's
  soft gate, R1390), at the browser's budget (`AI_BUDGET`) and with no handicap.
- Game `n` of the series has the seed `gather:<n>`, so the lane's games are not the gate's
  (`<lane>:<tree>:<k>`), not a person's pre-release run's (`dev:<n>`) and not any tuning run's.
- Each finished game is filed as a record with `source: "dev"`, `mode: "random"`, both pilots `ai`, the
  id `dev:<patch>:gather:<n>` and the patch `cargo jackioh catalog-version` names for the checkout. A game
  that does not finish (the action limit, a panic) files nothing.

The lane keeps `~/training-out/gather/cursor-<patch>`, the number of the next game of that patch, so a
patch's games are played once each and a new patch starts again at 1. It plays a patch without end,
until `main` ships another.

## What it never does

1. It changes no file in the checkout, commits nothing, pushes nothing and opens no pull request. It
   needs no GitHub login beyond reading the repository.
2. Nothing reads its records back into the AI, the gate or the shadow ban. A lane's gate plays its own
   seeds (`<lane>:<tree>:<k>`) and every sweep its own; this lane's numbers inform a person, and no
   loop closes on them.
3. It never mixes live and development data (R378, R1451). Its records are `source: "dev"` all of
   them, `stats-import` refuses a file that holds any other, and no live figure counts a development
   record unless the query asks for `--source=all`.
4. It does not reach players except as R654 says: until a patch has `PUBLIC_STATS_MIN_LIVE_GAMES` live
   games, the public card statistics are padded with development games and labelled provisional, and
   once it has them the live games alone are read.

## What it leaves

`~/training-out/gather/`, under the lane's own Linux user:

- the records, a file a day (`<date>.jsonl`);
- `cursor-<patch>`, the number of the next game of that patch;
- `cards-<patch>.txt` and `.json`, the patch's card figures from every game held, rewritten each cycle:
  per card, the win rate and the games behind it for the decks that held it (in deck), held it in the
  opening hand, went first or second with it, played it, or drew it and never played it, and the played
  delta; a card's play rate is its played games over its in-deck games;
- `imported.txt`, the days already loaded into Postgres when the box has `DATABASE_URL`.

`training/README.md` (The gather lane) says how to read them.
