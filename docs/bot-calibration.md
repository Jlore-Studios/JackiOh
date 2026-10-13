# Calibrating the ladder bots (R748000)

A ladder bot's rating starts at its strength's seed (`BOT_SEED_RATINGS`, `crates/server/src/config.rs`;
R1454), and its own games move it from there (R610). This is how the seeds are measured, and how they
are measured again when the AI changes. Issue #748, part of #636; the ruling is
[R748000](../spec/rulings/R748000.md).

## The reference and the offsets

The strongest of the five strengths (`BOT_STRENGTHS`, the last: the largest budget, no softmax
temperature and no missed lethal) is the reference bot. The other four are measured as Elo offsets
from it.

```sh
cargo run --release -p jackioh-server -- bot-calibrate
```

plays every pair of strengths `BOT_CALIBRATION_GAMES_PER_PAIR` (100) games, ten pairs and 1000 games in
all, on every core the machine has. Each game is two strengths with All Random decks (R258), the
lower strength of the pair in the first seat in the odd games and the second in the even ones; each
seat runs `decide_styled` at its strength on its node budget, with no wall clock. A game is a function
of its seed (`bot-calibration:gen<generation>:<a>v<b>:<n>`, `--seed` changes it), so the same command
on the same commit plays the same games on any number of threads. A game that reaches no result in
`BOT_CALIBRATION_MAX_ACTIONS` moves, or in which one turn takes more than
`BOT_CALIBRATION_MAX_TURN_ACTIONS` (30) actions, is counted as unfinished and left out. The second
limit stands in for the acting clock a live turn has (R79): without it a game can loop for good (see
Results).

The fit is Bradley-Terry over all ten pairs at once, with one drawn game added to every pair (a draw is
half a win each): it ties a strength that loses every game to the reference to the strengths between,
and a pair that sweeps stays finite. The offsets come out in Elo points (`GLICKO_SCALE · ln(strength)`),
the reference at 0, each with its standard error. `--games` can raise the 40 and never lower it under
`BOT_CALIBRATION_MIN_GAMES` (30).

## The anchor

The offsets say how far apart the strengths are, not where they stand among people. The anchor is the
rating the reference holds, and a seed is the anchor plus the strength's offset, rounded.

- **Provisional.** Until the reference has played people, the anchor is the seed of record, 1200: the
  value the strongest strength was given by hand. `bot-calibrate` prints it as its anchor, shifted by the
  AI promotions since the seeds were measured (below).
- **Measured.** With `LADDER_BOTS=on` the reference plays rated games against people. Once it has
  `BOT_CALIBRATION_MIN_GAMES` of them, a person runs, against production's `DATABASE_URL`,

  ```sh
  cargo run --release -p jackioh-server -- bot-calibrate-live --since=<epoch ms of this generation's merge>
  ```

  which reads every rated game with a bot on one side that ended at or after `--since`
  (`ranked_bot_games_since`), keeps the reference's games against people, and prints the rating that
  matches its score against their ratings before each game, with its standard error. `--since` leaves
  out the games the bot played before the last promotion, on the AI it replaced. The next step is the command it
  prints: `bot-calibrate --anchor=<rating>`.

Nothing in a build can reach production, so the first calibration's anchor is the provisional one.

## The ceiling gate

A player rated above every bot in the pool is never given one: `choose_bot` (`bots/fallback.rs`,
R1453) answers none when the player outrates every active bot, and the ticket waits for a person
instead. With a pool seeded from `BOT_SEED_RATINGS` the ceiling is the highest seed. Two tests hold it:
`r1453_above_the_strongest_bot_gets_none` and `r748000_the_strongest_seed_is_the_ceiling`
(`crates/server/tests/api/bots.rs`): a person one point above the highest seed gets no bot, and one
rated exactly at it gets the strongest.

The ceiling is why the seeds matter at the top: a reference seeded too low sends the best players
to wait for people, one seeded too high lets a bot beat players who have outgrown it. Live ratings
(R610, Glicko-2) correct a bot that is off, since each game moves it; the ceiling reads those live
ratings, not the seeds.

## After an AI promotion

A promotion (R732001) replaces the AI the strengths run on, and with it what each strength is worth.
The offsets are measured again, and the anchor moves by what the new AI gained.

1. On the new `main`, run `cargo run --release -p jackioh-server -- bot-calibrate`. It reads
   `crates/ai/generation.json` for the generation it is playing, and `training/history/*.jsonl` for the
   promotions after `BOT_CALIBRATION_GENERATION`, and adds the Elo of each one's `vsParent` (wins out of
   games, a game not won counting as lost) to the reference's seed of record. If `bot-calibrate-live`
   has measured the anchor since, pass it as `--anchor=<rating>` instead.
2. Open a pull request that pastes the two printed rows (`BOT_SEED_RATINGS` and
   `BOT_CALIBRATION_GENERATION`) over those in `crates/server/src/config.rs` and the report into
   Results below. A person or the night bot makes it: the training lanes never edit `crates/server`
   (`training/README.md`), and a promotion does not wait for it.

Rejected: shifting the seeds automatically when the AI is built, or failing a test once the AI is newer
than `BOT_CALIBRATION_GENERATION`. The first would move seeds nobody had read, and the second would stop
every promotion on a number a lane cannot change.

The seeds are for new bots. A bot already in the pool keeps the rating it has earned, so a
re-calibration changes the ratings of the bots made after it, and the ceiling only as bots are replaced
and the live ratings follow the AI.

## How calibration differs from live play

- **No deadline.** A live bot's search stops at `BOT_DECISION_DEADLINE_MS` or its acting clock's margin.
  Calibration counts nodes only, so a slow machine does not weaken the strongest strength. A live
  bot on a slow host plays a little below its strength.
- **No manner.** A live bot concedes, answers a draw offer, lets a turn run out and thinks for a
  person's time (R639000). A calibration game has none of them: no think time, no concession, no idle
  turn.
- **All Random decks.** Both seats get a deck dealt as All Random deals one (R258). A live bot plays
  the decks the pool dealt it, many of them people's own lists (R1454).
- **AI against AI.** The offsets are measured between strengths of the same AI. A person is not another
  strength, and the gap a bigger search opens over a smaller one need not be the gap it opens over a
  person, so the offsets are a first estimate of the spread. A promotion's `vsParent` has the
  same fault: it is the new AI against the old one, at least 65 of 100 to be promoted, and overstates its
  gain against people. R610 corrects both in live play, and `bot-calibrate-live` replaces the anchor as
  soon as people have played the reference.

## Results

Run on 2026-10-13 at base commit `21cfe832a` with the code of this change, from the repo root:

```sh
cargo run --release -p jackioh-server -- bot-calibrate --games=100
```

(the default, `BOT_CALIBRATION_GAMES_PER_PAIR`), on 4 threads, in about 50 minutes. The anchor is the
seed of record, 1200: `bot-calibrate-live` has not been run, since production has no rated games of the
reference against people yet (it needs `LADDER_BOTS=on`). The report is the command's output, unedited,
and its last two lines are what `config.rs` holds.

Generation 9, seed `bot-calibration:gen9`, 100 games per pair, anchor 1200.0 (the seed of record 1200.0, no promotion since generation 9).

Wins, draws and losses are the lower strength's.

| Pair | Wins | Draws | Losses | Unfinished |
| --- | ---: | ---: | ---: | ---: |
| 0 v 1 | 36 | 0 | 63 | 1 |
| 0 v 2 | 39 | 0 | 61 | 0 |
| 0 v 3 | 29 | 0 | 71 | 0 |
| 0 v 4 | 33 | 0 | 67 | 0 |
| 1 v 2 | 50 | 0 | 50 | 0 |
| 1 v 3 | 44 | 0 | 56 | 0 |
| 1 v 4 | 42 | 0 | 58 | 0 |
| 2 v 3 | 56 | 0 | 44 | 0 |
| 2 v 4 | 47 | 0 | 53 | 0 |
| 3 v 4 | 52 | 0 | 48 | 0 |

| Strength | Offset | ± | Seed |
| --- | ---: | ---: | ---: |
| 0 | -126.1 | 18.2 | 1074.0 |
| 1 | -37.0 | 17.5 | 1163.0 |
| 2 | -16.9 | 17.5 | 1183.0 |
| 3 | -4.2 | 17.6 | 1196.0 |
| 4 (reference) | +0.0 | 0.0 | 1200.0 |

// jackioh-server bot-calibrate: paste over the rows of these names, crates/server/src/config.rs
pub const BOT_SEED_RATINGS: [f64; 5] = [1074.0, 1163.0, 1183.0, 1196.0, 1200.0];
pub const BOT_CALIBRATION_GENERATION: i64 = 9;

Reading it:

- The order is the strengths' own: every offset is at or below the reference's, and rises with the
  strength. The three strongest are within one standard error or two of each other (-16.9, -4.2 and 0:
  their softmax temperature and missed-lethal chance differ little), and the weakest is 126 Elo
  below the reference. The seeds therefore span 1074 to 1200, not the 800 to 1200 the hand-set values
  did: one search budget against another is a smaller gap than the guess.
- Pair 0 v 1 has one unfinished game, a turn that went past `BOT_CALIBRATION_MAX_TURN_ACTIONS`.

### Why 100 games, and the University loop

A first run of 40 games per pair (20 minutes, before the turn limit existed) fitted offsets of -147.7,
-69.8, -17.7, -62.9 and 0, each with an error of about 28 Elo: wider than the gap between three of the
strengths, and strength 3 fitted below strength 2 (27-13 in their pair), which is noise. Those numbers
were not kept. At 100 games the errors are about 18 and the order holds.

The first attempt at 100 games hung. In game 66 of pair 0 v 1 a University (Meditative 97.6) Buffs
itself, which raises its own "Activate N" limit, so the AI can activate it for ever and every step costs
half as much again as the last. A live game ends such a turn with the acting clock (R79); a calibration
game has none, hence the turn limit above. That a card can do this is an engine and card design question
for a person: nothing here changes it.
