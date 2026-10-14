# Calibrating the ladder bots (R748000)

A ladder bot's rating starts at its strength's seed (`BOT_SEED_RATINGS`, `crates/server/src/config.rs`;
R1454), and its own games move it from there (R610). This is how the five strengths are measured
against each other, how that measurement meets the skill curve the seeds are read from (R747000), and
how it is done again when the AI changes. Issue #748, part of #636; the ruling is
[R748000](../spec/rulings/R748000.md).

Two measurements stand behind the seeds, and they measure different things. The skill curve
(`SKILL_ELO_TABLE`, `crates/server/README.md`, "Skill to Elo") is the AI at the arena's budget playing
at each skill against the reference and against the random policy; each strength's seed is the curve's
reading at its skill, and a test (`r747000_strength_seed_ratings_match_skill_curve`) holds the two equal.
The calibration here plays the five strengths as they are built, each with its own search budget,
softmax temperature, missed-lethal chance and skill, against one another, which is what the curve does
not measure. The seeds are the curve's until a person moves them on the calibration's evidence, and
then the calibration's rows and the curve's move together (below).

## The reference and the offsets

The strongest of the five strengths (`BOT_STRENGTHS`, the last: the largest budget, full skill, no
softmax temperature and no missed lethal) is the reference bot. The other four are measured as Elo
offsets from it.

```sh
cargo run --release -p jackioh-server -- bot-calibrate
```

plays every pair of strengths `BOT_CALIBRATION_GAMES_PER_PAIR` (100) games, ten pairs and 1000 games in
all, on every core the machine has. Each game is two strengths with All Random decks (R258), the
lower strength of the pair in the first seat in the odd games and the second in the even ones; each
seat plays as a ladder bot's seat does (`bots::seat`): the skill's roll first, on the seat's own
stream, then `decide_styled` at its strength on its node budget when the AI has the decision and the
random policy's move (`random_action`) when it does not, with no wall clock. A game is a function
of its seed (`bot-calibration:gen<generation>:<a>v<b>:<n>`, `--seed` changes it), so the same command
on the same commit plays the same games on any number of threads. A game that reaches no result in
`BOT_CALIBRATION_MAX_ACTIONS` moves, or in which one turn takes more than
`BOT_CALIBRATION_MAX_TURN_ACTIONS` (30) actions, is counted as unfinished and left out. The second
limit stands in for the acting clock a live turn has (R79): without it a game can loop for good (see
Results).

The fit is Bradley-Terry over all ten pairs at once, with one drawn game added to every pair (a draw is
half a win each): it ties a strength that loses every game to the reference to the strengths between,
and a pair that sweeps stays finite. The offsets come out in Elo points (`GLICKO_SCALE · ln(strength)`),
the reference at 0, each with its standard error. `--games` can change the 100 and never lower it under
`BOT_CALIBRATION_MIN_GAMES` (30).

## The anchor

The offsets say how far apart the strengths are, not where they stand among people. The anchor is the
rating the reference holds, and a seed is the anchor plus the strength's offset, rounded.

- **Provisional.** Until the reference has played people, the anchor is the seed of record,
  `SKILL_ELO_ANCHOR` (1200): the value the strongest strength was given by hand, and the one the skill
  curve is held at (R747000). `bot-calibrate` prints it as its anchor, shifted by the
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
2. Compare the printed seeds with the skill curve's (Results below shows how). Put the report into
   Results either way, and `BOT_CALIBRATION_GENERATION` at the printed generation when the seeds stay
   at the curve. To move them, paste the printed `BOT_SEED_RATINGS` row over the one in
   `crates/server/src/config.rs` and the same readings over the rows of `SKILL_ELO_TABLE` (the
   strengths' skills are the rows' skills, and the test that holds a strength's seed to the curve
   fails until they agree). Both measurements carry error, the curve's about 50 Elo (95%) for a run
   of 200 games near an even match and the calibration's the ± of its table, so a person judges
   whether a difference is the strengths' own budgets and temperatures or noise, and a longer run
   (`--games=300`) settles a doubt. A person or the night bot makes the pull request: the training
   lanes never edit `crates/server` (`training/README.md`), and a promotion does not wait for it. The arena's own protocol for the curve (`crates/server/README.md`, "Skill to Elo") is the
   other half of the same job after a new best AI; the two need not be run together.

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
- **All Random decks.** Both seats get a deck dealt as All Random deals one (R258), passing no
  strength, so no card is weighted by its win rate. A live bot plays the decks the pool dealt it, many
  of them people's own lists, and the random ones lean on the cards that win in proportion to its
  strength (R1454, R750000), which a bot's rating takes in over its own games and a calibration does
  not.
- **AI against AI.** The offsets are measured between strengths of the same AI. A person is not another
  strength, and the gap a bigger search opens over a smaller one need not be the gap it opens over a
  person, so the offsets are a first estimate of the spread. A promotion's `vsParent` has the
  same fault: it is the new AI against the old one, at least 220 of 400 (improve) or 60 of 100
  (unban) to be promoted, and overstates its gain against people. R610 corrects both in live play,
  and `bot-calibrate-live` replaces the anchor as soon as people have played the reference.

## Results

Run on 2026-10-13 on `main` at `a4b29f814` (R747000's skills, R750000's decks and the gather lane in) with
the code of this change, from the repo root:

```sh
cargo run --release -p jackioh-server -- bot-calibrate --games=100
```

(the default, `BOT_CALIBRATION_GAMES_PER_PAIR`), on 4 threads, in about 14 minutes. The anchor is the
seed of record, 1200 (`SKILL_ELO_ANCHOR`): `bot-calibrate-live` has not been run, since production has no
rated games of the reference against people yet (it needs `LADDER_BOTS=on`). The report is the command's
output, unedited.

## Ladder bot calibration (R748000)

Generation 9, seed `bot-calibration:gen9`, 100 games per pair, anchor 1200.0 (the seed of record 1200.0, no promotion since generation 9).

Wins, draws and losses are the lower strength's.

| Pair | Wins | Draws | Losses | Unfinished |
| --- | ---: | ---: | ---: | ---: |
| 0 v 1 | 27 | 0 | 73 | 0 |
| 0 v 2 | 14 | 0 | 86 | 0 |
| 0 v 3 | 7 | 0 | 93 | 0 |
| 0 v 4 | 3 | 0 | 97 | 0 |
| 1 v 2 | 41 | 0 | 59 | 0 |
| 1 v 3 | 16 | 0 | 84 | 0 |
| 1 v 4 | 9 | 0 | 91 | 0 |
| 2 v 3 | 40 | 0 | 60 | 0 |
| 2 v 4 | 16 | 0 | 84 | 0 |
| 3 v 4 | 33 | 0 | 67 | 0 |

| Strength | Offset | ± | Seed |
| --- | ---: | ---: | ---: |
| 0 | -564.5 | 26.4 | 636.0 |
| 1 | -377.9 | 21.1 | 822.0 |
| 2 | -266.0 | 20.3 | 934.0 |
| 3 | -141.8 | 21.1 | 1058.0 |
| 4 (reference) | +0.0 | 0.0 | 1200.0 |

// jackioh-server bot-calibrate: paste over the rows of these names, crates/server/src/config.rs
pub const BOT_SEED_RATINGS: [f64; 5] = [636.0, 822.0, 934.0, 1058.0, 1200.0];
pub const BOT_CALIBRATION_GENERATION: i64 = 9;

### Reading it against the curve

The seeds in `config.rs` are the skill curve's: 570, 740, 900, 1080 and 1200. Strength by strength, the
calibration reads:

| Strength | Curve | Calibration | Difference | Difference in its ±'s |
| --- | ---: | ---: | ---: | ---: |
| 0 (skill 0) | 570 | 636 | +66 | 2.5 |
| 1 (skill 0.25) | 740 | 822 | +82 | 3.9 |
| 2 (skill 0.5) | 900 | 934 | +34 | 1.7 |
| 3 (skill 0.75) | 1080 | 1058 | -22 | 1.0 |
| 4 (skill 1) | 1200 | 1200 | 0 | |

- **They agree on the order and the spread.** Every strength is below the next, from 636 to 1200 where
  the curve runs from 570 to 1200, and no pair of neighbours is closer than 100 Elo. The skills do most
  of the separating, as R747000 set them to (the earlier runs below, without a skill, spread the
  strengths far less). No game was unfinished.
- **The two weakest read higher than the curve.** Strengths 0 and 1 measure 66 and 82 above it, 2.5 and
  3.9 times the calibration's standard error. The curve's own error is about 50 Elo (95%) for a run of 200
  games near an even match and more for a lopsided one (`crates/server/README.md`, "Skill to Elo"), and
  the games differ: the curve is the AI at the arena's budget, the calibration plays the strengths'
  own budgets, with the reference at the largest one. Strength 0 is the random policy in both, and its
  three wins in 100 against the reference match the arena's 21 of 800; the added drawn game and its
  other pairs pull its offset up from the -604 those three wins alone read.
- **The seeds stay at the curve.** This change does not move them. The test that holds a strength's
  seed to the curve's reading (`r747000_strength_seed_ratings_match_skill_curve`) would fail on the
  printed row until `SKILL_ELO_TABLE` agrees, and R747000 measured and set that table; whether its two
  lowest rows move 66 and 82 Elo is a person's decision, taken with the games above and a longer run if
  wanted (`--games=300` cuts the errors to between 12 and 15). The ratings of the bots in the pool do not wait
  for it: R610 moves each bot's from the games it plays.

### Earlier runs, and the University loop

Before R747000 gave the strengths a skill, the same command on the same strengths, with the budgets,
temperatures and missed-lethal chances alone, read offsets of -126.1, -37.0, -16.9, -4.2 and 0 (100
games, seeds 1074, 1163, 1183, 1196 and 1200) and, in a first run of 40 games per pair, errors of about
28 Elo, wider than the gap between three of the strengths. Those numbers do not describe the strengths
as they are now and are not kept as results; they say that a search budget and a temperature alone
separate the strengths by little, which is why the skill exists.

The first attempt at 100 games hung. In game 66 of pair 0 v 1 a University (Meditative 97.6) Buffs
itself, which raises its own "Activate N" limit, so the AI can activate it for ever and every step costs
half as much again as the last. A live game ends such a turn with the acting clock (R79); a calibration
game has none, hence the turn limit above. That a card can do this is an engine and card design question
for a person: nothing here changes it.
