# The AI's training

The lanes that trained the AI are retired, and #4701 removed what ran them: the two LLM lanes,
`improve` and `unban` (a Muse session after another on the training box, stopped on 2026-10-25 when
#1681's audit found they made no fair promotion in about 427 hours, #1677), and the `learn` lane, which
retrained the learned evaluation on GitHub's runners until #4159 retired it (R4159000). Git history
holds their prompts, `training/loop.sh`, `training/ml/` and `learn.yml`. An idea for the AI now goes
through the night bot as an issue with a measured acceptance, measured with the screens
[`improve.md`](improve.md) describes (How you measure). What is left here:

- **The gather lane** (`.github/workflows/gather.yml`, [`gather.md`](gather.md)): development games of
  the newest version for the card statistics (The gather lane, below).
- **The sweep of record** (`.github/workflows/sweep.yml`, R1678000): the shadow ban measured again after
  every promotion and every set that ships, so the table keeps step with the AI on `main`. #1675's took
  it from 17 cards to 1 and #1889's to none.
- **The arena** (`cargo jackioh arena`, `agent`), where a change to the AI is screened, below.
- **The history** of every promotion (`training/history/`, Reading the history).

The design is docs/v0.3.0/README.md §8; the interfaces are docs/v0.3.0/SURFACE.md §14.

Generation 8 (`training/history/unban.jsonl`, 2026-10-10) removed the last ban on the earlier sets,
Core, Classic and Classic+. Generation 9 (#732, `training/history/meditative.jsonl`) brought the
Meditative set into the AI's decks: it lifted R1390's gate after a sweep of record over the set's 102
cards, which shadow-banned the ones the AI misplays (at most half the set). Those bans were the unban
lane's work until #1677; a sweep of record measures them now (#1675). The shadow ban holds no card of a set before Meditative and at most half of Meditative's (R732000), and since #1186's sweep, which banned none,
Meditative+ cards on the same terms (R1186000), and since #1885 Stinkyscape cards, at most 35 of them
(R1885000), and since #2203 Training Grounds cards, at most half of them (R2203000): no lane, sweep or person adds a card of an earlier set to
it, and the lanes only ever removed entries; the sweep after a promotion may ban a Meditative or
Meditative+ card again when the new AI is slow with it (R1678000).

The lanes played 100 games against SPEC §10.7's random policy too, needing 90, until generation 9,
the first AI dealt the Meditative set, beat random in at least 90 of 100 (R732001). Since then they
measure a candidate against its parent alone: `unban` at 60 of 100, and `improve` at 65 of 100 until
issue #850 lowered its bar to 220 of 400 paired games (R850000), so that the few points a change
gains can bank and compound. From #1679 every lane's gate was a sequential test on paired deals
(R1679000), which banked a 53% gain about nine times in ten where 220 of 400 banked it about one time
in seven. The daily quality gates (`cargo jackioh gate --full`) still hold the AI
on `main` to their floor against random, so an AI that forgets how to beat random fails the daily
run.

Generation 14 is no lane's either. R4159000's retirement of the learned evaluation moved the AI on
2026-11-19 without moving the number, so the sweep of record, the ladder bots' re-settling and the
records' builds read 13 for another AI. It is recorded by hand in lane `hand`
(`training/history/hand.jsonl`), its parent the last commit that held the evaluation. It has no
`vsRandom` and no `vsParent`, and its `shadowBan` and `parentShadowBan` are 0 (R4921000).

## The practice-claim fixture

`crates/server/tests/fixtures/practice-claim.json` is a practice game the web's claim test plays move
for move and the server's test folds and pays (R2560000). Every change to the AI plays it differently,
so the change records it again (`JACKIOH_RECORD_PRACTICE_CLAIM=1 pnpm --dir apps/web exec vitest run
src/practice/claim.test.ts`, after `sh scripts/build-wasm.sh`) and commits it with the change.

## The arena and the agent

- `cargo jackioh arena --a self|random|bin:<path> --b self|random|bin:<path> --games N --seed <base>
  [--paired] [--out <dir>]` plays games `1..=N` with seeds `<base>:<n>`, agent `a` on p1 in odd
  games, on any base of your own.
  With `--paired` (R850000) it plays deals instead: deal `d` is `<base>:<d>`, played twice with the
  seats swapped, so `N` is even. Per game it prints duel.ts's line (who won, why, the hash, whether
  the log replays, decision times) and a tally at the end: the wins, draws and games without a
  result, and agent `a`'s win share with its 95% Wilson interval (`aShare`, `aLow`, `aHigh`); a
  paired series adds `deals` and `edge`, the mean over deals of `a`'s wins less `b`'s over the deal's
  two games (−1 to 1), with its 95% interval (`edgeLow`, `edgeHigh`), then `decided` (the deals
  whose edge is not 0, split into `wonBoth` and `lostBoth`) and `diverged` (the deals whose two
  games end on different hashes). A base that is a retired lane's name, or one followed by `:`, is
  refused: those were the gates' own seeds (R850001).
- `cargo jackioh arena --a … --b … --set improve-dev [--games N] [--extend]` plays the improve
  lane's fixed dev set, deals `improve-dev:1` to `:200`, paired, 400 games unless `--games` says
  more, never fewer (R850001). A screen with fewer than `SCREEN_MIN_DECIDED` (20) decided deals
  prints `inconclusive: true` and says nothing: it neither keeps nor drops the change (R4319000).
  `--extend` plays on in 100-deal batches until `SCREEN_MIN_DECIDED` deals are decided or
  `SCREEN_MAX_GAMES` (2,000) games are played, reading the edge only at the end. A change to the AI
  screens on it (`improve.md`, How you measure); the gate's hold-out deals are never a screen's.
- `cargo jackioh arena --a … --b … --set improve-dev --forks [--extend] [--save <path>]` is
  the fork screen (R4397000): the trunk plays each dev-set deal once with the control on both
  seats, and wherever the candidate would have made a different move that position is played out
  `FORK_ROLLOUTS` (8) pairs both ways, both arms on with the control. The tally's `forkEdge`
  measures the change at the moves it changes; it keeps a change when `forkEdgeLow` is above 0 and
  reads `inconclusive: true` below `SCREEN_MIN_DECIDED` live forks. `--save <path>` (R4607000)
  appends each finished deal to the file as soon as it is done, and a rerun of the same command
  plays only the deals still missing.
- `cargo jackioh agent` is this build's AI speaking JSON lines on stdin and stdout (`info`,
  `decide`, `quit`; SURFACE §14.1). `bin:<path>` spawns `<path> agent`. The referee holds the true
  state and sends each agent only `redact(state, seat)` (R185). The agent's answers name its rng
  cursor, so `self` and `bin:<the same build>` play identical games.

## The gather lane

`.github/workflows/gather.yml` plays without a pause since #3740
(it played 5,000 games of each new patch or set, once, on the PC's gather half, before): five of
GitHub's runners (two until 2026-11-26) each play `GATHER_HOURS` (5) hours of the newest version on
`main` in one `stats --out-every` stopped at the deadline, which writes each `GATHER_FILE_GAMES` (50)
games to a file of their own as soon as all of them have finished, and file each file the same way as
it appears, so the card statistics take fresh games of the version that is live every few minutes. As a run ends it starts the next on `main` as
it is then, so a version shipped meanwhile is played from the next run on, within five hours, and a
schedule every six hours starts the lane again should that chain stop. It runs only while the
repository variable `GATHER_ON_PATCH` is `true`: each version's development totals take about 3 MB of
the database however few its games, and the lane files every version it plays, micros among them.

5,000 games were too few for the card win rates (#2888). A card's rate counts only the games
where exactly one seat holds it (R1284000), about 375 of the 5,000 for a typical card (150 of 2,000,
the number before), so its 95% interval is about ±5 points. `cargo jackioh stats sample-size` prints the games a run needs at
this build's deal pool, and the number grows with the catalog: 29000 at 633 cards when it landed.
`--explain` shows the working, `--sigma` and `--rho` set its two guesses, and
`crates/tools/README.md` has the formula. The lane playing without a pause reaches it for a version
that stays the newest about 12 hours (five runners at about 500 games an hour, #5103). A change to `gather.yml` is a person's, since the bots never edit `.github/`;
so is growing `GATHER_STRIDE` (100,000), which a run's games pass only past 20,000 a part.

The games are `stats --series gather` (R378): the AI on `main` against itself on R258's All Random
deals at the browser's budget, each finished game a record with `source: "dev"`, mode `random`, both
pilots `ai` and the id `dev:<patch>:gather:<n>` ([`gather.md`](gather.md) has the rest). Nothing feeds
the records back to the AI, the gate or the shadow ban (`gather.md`, What it never does).

### Setting it up

A person does this once for each project's database: create a login that may act as `service_role`
(Supabase's SQL editor: `create role stats_importer login noinherit password '…'; grant service_role
to stats_importer;`) and save its session-pooler connection string
(`postgresql://stats_importer.<project-ref>:<password>@aws-0-<region>.pooler.supabase.com:5432/postgres`,
the pooler because GitHub's runners reach no IPv6-only direct host) as the repository secret
`STATS_IMPORT_DATABASE_URL` (staging's, set on 2026-10-26, #1976) or
`PRODUCTION_STATS_IMPORT_DATABASE_URL` (production's, set on 2026-11-03 by `cutover.yml`'s step
`stats-logins`, R1933001). The read-only `stats_reader` is production's other login. Then set the
repository variable `GATHER_ON_PATCH` to `true`.

### Reading the figures

Each run keeps its record files as artifacts, `gather-<run>-<part>`
(`gh run download <run> --pattern 'gather-*'`):

```sh
cargo jackioh stats report --source=dev --patch <patch> gather-*/*.jsonl
cargo jackioh stats report --games --source=dev --patch <patch> gather-*/*.jsonl
jackioh-server stats-cards --source=dev --patch=<patch>           # the same, from Postgres
jackioh-server stats-cards --source=dev --patch=<patch> --card=<id> --json
jackioh-server stats-cards --patch=<patch>                        # that patch's live games, human seats
jackioh-server stats-cards --patch=<patch> --pilot=ai             # the ladder bots' sides of them
```

Each row of the report is a card: the win rate, with the games behind it, of the decks that held it
(in deck), held it in the opening hand, went first or second with it, played it, or drew it and never
played it, the decks that never drew it, and its drawn delta (the drawn win rate less the never-drawn one, in points). A
card's play rate is how often it was played when seen: its played games over its seen ones (played plus drawn and
not played), each printed in the row, as `/stats` reads it (R3950000). `--json` has the
same figures as numbers, and `stats-cards --source=all` reads both sources at once. Development and
live games are never added together unless asked (R378, R1451).

The game figures say how often the first seat won (with the 95% interval of that share), how the games
ended and how many turns they took, and, for the games whose records carry R1256000's counts, how many games had a
draw from an empty library and how many Traps fired of those set, and the position switches and Radiant upgrades in all and a game; an older record is unknown for those, not
zero. They are AI-against-AI figures: the first seat's edge between two AIs is not the people's, so a seat
decision rests on them only with that in view.

### Live games

The live games' records are what the gather lane's figures sit beside, and they are filed already:

- Every finished live match files a record, and no switch turns that off: `api::results::record_result`
  calls `api::game_records::record_finished_game` once the result is in, and `LADDER_BOTS` only decides
  who is seated, not whether the game is filed. The ones that file none are a match no series, room or
  queue ticket made, a log that never reaches a result and one already filed; a failure to file is
  logged as `game.record.failed` and the result stands.
- A seat is `ai` when its profile is a ladder bot's and `human` otherwise, so a game against a bot has
  one of each.
- A query counts the human seats of live data by default and reads the bots' sides only when asked
  (`--pilot=ai` or `--pilot=unified`; R1451). Development data counts every seat.
- Proving tests, all under `crates/server/`: `tests/api/game_records.rs`
  `r376_files_a_finished_match_once_its_result_is_in_its_mode_the_patch_two_human_pilots_and_the_game`,
  `tests/api/bots.rs` `r1451_the_bot_side_is_ai_and_the_human_side_counts`, `tests/actor/glitch.rs`
  `r1451_r677_a_bots_pilot_is_filed_on_the_seat_it_ended_in`, and in `tests/store/card_stats.rs`
  `r1451_counts_every_seat_of_development_data_unless_a_pilot_is_named` and
  `r1451_reads_a_ladder_bots_side_of_a_live_game_only_when_asked`.

## The ladder bots after a change to the AI

The ladder bots' strengths are measured against each other on the AI on `main` (R748000,
`docs/bot-calibration.md`), beside the skill curve their seed ratings are read from (R747000), so a
merged change to the AI is followed by that document's re-calibration: `cargo run --release -p
jackioh-server -- bot-calibrate`, run by a person or the night bot, whose report is compared with the
curve and, where it should win, written into `crates/server/src/config.rs` with `--write` in a pull request. The
change to the AI does not wait for it.

## Where the numbers go

| What | Where |
|---|---|
| Every game the arena plays: one `GameRecord` (R376) per line, `source: "dev"`, `mode: "random"` | `$JACKIOH_TRAINING_OUT/<date>.jsonl`; without it, stdout |
| The gather lane's records, a file each 50 games | the run's artifacts, `gather-<run>-<part>`, and both projects' card statistics as totals (`stats-import --totals-only`, R1976000) |
| A retired lane's promotions | `training/history/<lane>.jsonl` and `crates/ai/generation.json` |

## Reading the history

`crates/ai/generation.json` is the AI on `main`, as the last promotion left it. `training/history/<lane>.jsonl`
holds one line per promotion of that lane, oldest first; no lane promotes since #4701, and the gate
that measured them (`promote`) is gone since #4702 (R4702000), in git history:

```json
{"generation":8,"lane":"improve","parent":"<main commit>","tree":"<crates/ai/src tree>","vsRandom":"93/100","vsParent":"87/100","shadowBan":11,"parentShadowBan":11,"date":"2026-11-02"}
```

- `generation`: the parent's plus one. Generations count across both lanes: they share `main`.
- `parent`: the `main` commit the parent was built from; `tree`: the AI's source tree, which named
  the gate's seeds (`<lane>:<tree>:1` … `:100`, or the improve lane's deals `:1` … `:200`). The arena
  and self-play still refuse a base that names a lane (R850001, R1006002).
- `vsRandom`, `vsParent`: the candidate's wins out of the games played, `"<wins>/<games>"`: out of
  400 for the improve lane since issue #850 (R850000), out of 100 for the unban lane and every
  promotion before. `vsRandom` is `null` from generation 10 on: the lanes play no random games
  (R732001).
- `shadowBan`, `parentShadowBan`: how many cards each AI keeps out of its own decks (R186).

```sh
jq -r '[.generation, .date, .vsRandom, .vsParent, .shadowBan] | @tsv' training/history/*.jsonl
git log --oneline -- crates/ai/generation.json   # every generation's pull request
```

Generation 9 is not a lane's: it is #732's one-off measurement of the AI with the Meditative set in
its decks (lane `meditative`, `training/history/meditative.jsonl`), against random and against
generation 8, recorded when it lifted R1390's gate.

Generation 0 is the port of the last TypeScript AI (before v0.3.0, `packages/ai` at `91cc43c`, which won 94 of 100
against random, 35 of 50 against greedy and 47 of 50 as Hard against Easy). Its record says
`"lane": "port"` until part 40 measures it in Rust; the histories start empty.
