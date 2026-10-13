# The AI training lanes

Two lanes that keep improving JackiOh's AI (`crates/ai/`, SPEC §9.9), each a Devin session after
another on the training box, each gated by a fixed number of games against the AI on `main`, and
nothing else: since generation 9 they play no games against random (R732001). They
succeed #55's ladder (the Python `ladder/` that v0.3.0 replaced and deleted). The design is docs/v0.3.0/README.md §8; the interfaces are
docs/v0.3.0/SURFACE.md §14.

| Lane | Goal | Standing prompt |
|---|---|---|
| `improve` | a stronger AI: beat its predecessor in 65 of 100 games | [`improve.md`](improve.md) |
| `unban` | an AI that plays well with fewer shadow-banned cards (R186): strictly fewer bans than its predecessor, while beating it in 60 of 100 | [`unban.md`](unban.md) |
| `gather` | no change to the AI: development records for the card win rates (R378), without end | [`gather.md`](gather.md) |

The `gather` lane runs no Devin session and opens no pull request (The gather lane, below).

Generation 8 (`training/history/unban.jsonl`, 2026-10-10) removed the last ban on the earlier sets,
Core, Classic and Classic+. Generation 9 (#732, `training/history/meditative.jsonl`) brought the
Meditative set into the AI's decks: it lifted R1390's gate after a sweep of record over the set's 102
cards, which shadow-banned the ones the AI misplays (at most half the set). Those bans are the unban
lane's work now. The shadow ban holds Meditative cards only (R732000): no lane, sweep or person adds
a card of an earlier set to it, and the lanes only ever remove entries.

While the table is empty the unban lane idles: no AI can keep strictly fewer cards out of its decks
than none, so `promote --lane unban` refuses to play its games (The gate), and `loop.sh` starts no
session, looking again every `TRAIN_IDLE_SECONDS` (How a lane runs, step 2).

The lanes played 100 games against SPEC §10.7's random policy too, needing 90, until generation 9,
the first AI dealt the Meditative set, beat random in at least 90 of 100 (R732001). Since then they
measure a candidate against its parent alone, at 65 of 100 for `improve` and 60 for `unban`. The
daily quality gates (`cargo jackioh gate --full`) still hold the AI on `main` to their floor against
random, so an AI that forgets how to beat random fails the daily run.

## How a lane runs

The training box (an always-on EC2 `m7i-flex.large`, set up by #306's part 39) runs two systemd services,
`jackioh-train@improve` and `jackioh-train@unban`, each as its own Linux user (`agent-train-improve`,
`agent-train-unban`) with its own checkout, Devin login and GitHub token. Each runs
[`loop.sh`](loop.sh) `<lane>` forever (`Restart=always`).

The steps below are the `improve` and `unban` lanes'. The `gather` lane's cycle is its own (The
gather lane).

Each lane keeps one pull request open from `ai/<lane>`: a draft while Devin works, and ready for
review, with auto-merge on, once a promotion is in it. Each cycle:

1. If the pull request holds a promotion waiting on CI, look after it and do nothing else. When
   `main` moved under it (GitHub calls it behind or conflicting, or a check failed on a `main` that
   has moved on since), land it on `main` again (step 4) and push, so CI runs again. When a required
   check failed, set it back: back to draft, with a comment naming the failing checks, its commit kept
   as a local branch `ai/<lane>-set-back-<t>`, and an entry in `attempts.md` saying why, which the
   next session reads first. Otherwise wait.
2. Otherwise fetch `main`, reset `ai/<lane>` to it, build `jackioh` from it and keep that binary as
   `~/parent-jackioh`. The parent is always the AI on `main`.
   - The `unban` lane stops here while the parent shadow-bans no card. A promotion needs strictly
     fewer bans, so a session could do nothing. The lane retitles its draft
     `AI gen <N> (unban): idle, the AI on main bans no card`, says so once in a comment, and looks
     again every `TRAIN_IDLE_SECONDS` (15 minutes). The first cycle after `main`'s AI bans a card
     again starts a session. A sweep of record adds such bans, as #732's did for the Meditative set,
     and both standing prompts send the lanes to the Meditative cards first.
   - Otherwise push `ai/<lane>` as `main` plus an empty `[skip ci]` commit naming the session, and
     make the lane's pull request this session's draft, titled
     `AI gen <N> (<lane>): in training, nothing promoted yet` (opening one if none is open).
3. Run one Devin session on the lane's standing prompt
   (`devin -p --prompt-file training/<lane>.md --model "$DEVIN_MODEL" --permission-mode dangerous
   --respect-workspace-trust false --export ~/logs/<lane>-<t>.json`). Devin changes the AI, measures
   it with dry runs of the gate, and commits only once a real run of the gate has passed. A session
   ends after a promotion or after four hours. Meanwhile, every 15 minutes
   (`TRAIN_PROGRESS_SECONDS`), the loop pushes the checkout as it is to `ai/<lane>` as one
   `[skip ci]` commit when it changed, and rewrites the session's comment on the draft.
4. When the session ends with a promotion commit, land it on `main`. If `main`'s crates did not move
   since the session's start, the parent and the candidate are the builds the gate measured: rebase
   the commit and re-check it against the parent (`cargo jackioh promote --verify`). If they moved,
   the parent changed: build it from `main` again, apply the promotion's change (less the record the
   gate writes) to `main`'s AI and play the gate again for real, which renumbers the generation.
   Then run the web's tests that play the AI (the tutorial's lessons and practice, as the standing
   prompts' "Beyond the gate" lists them) when the box has `pnpm`, push `ai/<lane>`, retitle the pull
   request with the promotion's subject (`AI gen <N> (<lane>): <what changed>`), give it the gate's
   report as its body, mark it ready and turn on auto-merge (squash). A promotion that fails any of
   that is set back as in 1.
5. Sleep 60 seconds and repeat.

A lane never merges anything itself: CI does, through branch protection, once its checks pass.

### Watching a lane on GitHub

The lane's pull request is the place to follow it. Its **Files changed** is the session's work in
progress against `main` (CI does not run on a `[skip ci]` commit), and its comments say what
happened:

- one comment per session, rewritten every 15 minutes while it runs: how long it has run, the
  checkout's diff against `main` with a link to it, and everything `attempts.md` gained this session
  (each attempt, its dry run's numbers, kept or reverted). The last rewrite says how it ended;
- a comment when a promotion goes up, with the gate's table, and when one is set back, with the
  failing checks.

## What a lane may change

Only `crates/ai/**` and `training/history/**`. A pull request from an `ai/*` branch that touches
anything else is refused by CI, and so is one whose `crates/ai/generation.json` is not `main`'s
plus one (part 30's `training-gate` job). That job builds `main`'s `jackioh` and the branch's, and
runs `promote --lane <lane> --verify` again on the same seeds: a lane's own claim is never trusted.
Branch protection's "require branches to be up to date" holds a promotion whose `main` moved on
until the loop lands it on `main` again (How a lane runs, step 1).

The engine (`crates/engine`), the cards (`crates/cards`), every test outside `crates/ai`, the gate
(`crates/tools/src/promote.rs`, `arena.rs`) and its thresholds (`TRAINING_*` in
`crates/engine/src/config.rs`) are never a lane's to change. Inside `crates/ai`, `SHADOW_BAN` only
shrinks: no lane adds an entry, and no card of an earlier set is ever banned again (R732000;
`crates/ai/tests/ai/shadow_ban.rs` fails on one).

## The gate

`cargo jackioh promote --lane <lane> --parent-bin <parent> [--dry-run] [--verify]` plays
`TRAINING_GAMES` (100) games against the parent:

- game `k`'s seed is `<lane>:<tree>:<k>`, where `<tree>` is `git rev-parse HEAD:crates/ai/src`, so
  the seeds name the AI being measured and committing `generation.json` does not move them;
- the candidate sits p1 on odd games and p2 on even ones;
- each seat's deck is `build_ai_deck` over every set that ships (Meditative included since
  generation 9) minus its own AI's shadow ban, with no handicap (Easy both);
- the AI plays at `AI_GATE_BUDGET`, the browser's own budget, so the gate measures the AI that ships.

| Lane | vs parent | Shadow bans |
|---|---|---|
| `improve` | ≥ 65 of 100 (`TRAINING_IMPROVE.vs_parent`) | — |
| `unban` | ≥ 60 of 100 (`TRAINING_UNBAN.vs_parent`) | strictly fewer than the parent's |

It plays no games against random (R732001), and a promotion records `"vsRandom": null`. A draw is not
a win, and neither is a game that ended without a result. Exit 0 on a pass, 1 on a failed gate. The
report (Markdown) goes to stdout.

Against a parent whose shadow ban is empty, the unban lane's gate cannot pass (strictly fewer than
none), so `promote --lane unban` says the lane has nothing to do and exits 1 before it plays a game,
whatever the flag, writing nothing and printing no report.

- `--dry-run` measures and writes nothing. It runs on a working copy with uncommitted changes in
  `crates/ai/src` (on HEAD's seeds, and says so).
- Without a flag, a pass writes `crates/ai/generation.json` and appends the same object to
  `training/history/<lane>.jsonl`. It refuses to run while `crates/ai/src` has uncommitted changes,
  since the seeds come from the committed tree: commit the change first, then run it, then amend the
  two files into that commit.
- `--verify` (CI, and `loop.sh` before it pushes) recomputes and compares every field of the
  branch's `generation.json` except `date` and `parent`.

The games run on rayon's threads (`RAYON_NUM_THREADS` is respected); no result depends on the
thread count.

### The arena and the agent

- `cargo jackioh arena --a self|random|bin:<path> --b self|random|bin:<path> --games N --seed <base>
  [--out <dir>]` plays games `1..=N` with seeds `<base>:<n>`, agent `a` on p1 in odd games. It is
  the free-form version of the gate, for experiments on other seeds than the gate's. Per game it
  prints duel.ts's line (who won, why, the hash, whether the log replays, decision times) and a
  tally at the end.
- `cargo jackioh agent` is this build's AI speaking JSON lines on stdin and stdout (`info`,
  `decide`, `quit`; SURFACE §14.1). `bin:<path>` spawns `<path> agent`. The referee holds the true
  state and sends each agent only `redact(state, seat)` (R185). The agent's answers name its rng
  cursor, so `self` and `bin:<the same build>` play identical games.

## The gather lane

`loop.sh gather` is the third lane, and it gathers data instead of improving the AI (#749, part of
#636): it plays AI-against-AI All Random games without end and keeps their card figures, so the card
win rates (SPEC §9.11, R376–R378) of every patch have a development source beside the live games. No
model runs it, so [`gather.md`](gather.md) is its contract rather than a prompt, and it needs no Devin
login and opens no pull request. Each cycle:

1. Fetch `main`, check out `origin/main` (a new `loop.sh` there restarts the lane, as for the others)
   and build `jackioh` from it. The AI that plays is always the one on `main`.
2. Load the finished days' record files into Postgres with `stats-import`, once each, when the box has
   `DATABASE_URL` (the same step as the other lanes').
3. Play the next `TRAIN_GATHER_GAMES` (100) games with `jackioh stats --series gather --patch <patch>`,
   where `<patch>` is `cargo jackioh catalog-version`, `main`'s. Game `n`'s seed is `gather:<n>`, so
   the lane's games are neither the gate's nor a person's `dev` run's. The file `cursor-<patch>` in
   `~/training-out/gather/` holds the next `n`: no game of a patch is played twice, and a new patch
   starts at 1.
   A patch stops at `TRAIN_GATHER_PATCH_GAMES` (5000), which keeps its records near 9 MB in Postgres;
   the lane then idles, looking again every `TRAIN_IDLE_SECONDS`, until `main` ships another patch.
4. Append the batch to `~/training-out/gather/<date>.jsonl`, advancing the cursor first, so a batch
   the loop never finished is played again and none is appended twice (`stats-import` skips the ids it
   holds, too), and rewrite the patch's card figures, `cards-<patch>.txt` and `.json`, from every game
   the lane's files hold.
5. Sleep `TRAIN_PAUSE_SECONDS` (60) and repeat.

The records are `source: "dev"`, mode `random`, both pilots `ai`, with the id
`dev:<patch>:gather:<n>`. The games run under `nice -n 19`, so the other two lanes' gates keep the CPU.
A record is about 1.8 KB and a game takes about 4 seconds on one core. If the AI is promoted in the
middle of a patch, that patch's games come from more than one generation of it. Nothing feeds the
records back to the AI, the gate or the shadow ban (`gather.md`, What it never does).

### Setting it up

The training box sets up its lanes in `bot/machine/setup.sh`, which the bot never edits, so a person
does this once:

- Add `gather` to `TRAIN_LANES` and to the `systemctl enable --now` line in `bot/machine/setup.sh`,
  then run `setup.sh --training` (`bot/machine/README.md`, The training box). It creates the user
  `agent-train-gather` with its own checkout and the service `jackioh-train@gather`.
- To load its records into Postgres, run `sudo systemctl edit jackioh-train@gather` and add
  `Environment=DATABASE_URL=…`. Without it the lane keeps only the files.
- It needs no Devin login. A `gh` login is needed only if the repository is private, so the lane can
  read it.

### Reading the figures

```sh
tail -f ~agent-train-gather/logs/gather.log
cat ~agent-train-gather/training-out/gather/cards-<patch>.txt     # the patch's card figures, a row per card
cargo jackioh stats report --source=dev --patch <patch> ~agent-train-gather/training-out/gather/*.jsonl
jackioh-server stats-cards --source=dev --patch=<patch>           # the same, from Postgres, once loaded
jackioh-server stats-cards --source=dev --patch=<patch> --card=<id> --json
jackioh-server stats-cards --patch=<patch>                        # that patch's live games, human seats
jackioh-server stats-cards --patch=<patch> --pilot=ai             # the ladder bots' sides of them
```

Each row of the report is a card: the win rate, with the games behind it, of the decks that held it
(in deck), held it in the opening hand, went first or second with it, played it, or drew it and never
played it, and its played delta (the played win rate less the drawn-but-not-played one, in points). A
card's play rate is its played games over its in-deck games, both printed in the row. `--json` has the
same figures as numbers, and `stats-cards --source=all` reads both sources at once. Development and
live games are never added together unless asked (R378, R1451).

### Live games

The live games' records are what the gather lane's figures sit beside, and they are filed already:

- Every finished live match files a record, and no switch turns that off: `api::results::record_result`
  calls `api::game_records::record_live_game` once the result is in, and `LADDER_BOTS` only decides
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

## Where the numbers go

| What | Where |
|---|---|
| Every game the arena plays (a promotion run or an experiment): one `GameRecord` (R376) per line, `source: "dev"`, `mode: "random"` | `~/training-out/<lane>/<date>.jsonl` (`$JACKIOH_TRAINING_OUT`; `arena` without it prints to stdout) |
| The same records in Postgres, loaded once per finished day when the box has `DATABASE_URL` | `jackioh-server stats-import`, read back with `jackioh-server stats-cards --source=dev` |
| The gather lane's records: one `GameRecord` per line, a file a day | `~/training-out/gather/<date>.jsonl` |
| The gather lane's next game number for a patch, and that patch's card figures, rewritten each cycle | `~/training-out/gather/cursor-<patch>`, `cards-<patch>.txt` and `cards-<patch>.json` |
| The days already loaded into Postgres (each lane's `~/training-out/<lane>/`) | `imported.txt` |
| A promotion's gate numbers | `training/history/<lane>.jsonl` and `crates/ai/generation.json`, committed with it |
| What each Devin session tried, and why the loop set a promotion back | `~/training-out/<lane>/attempts.md`, quoted in each session's comment on the lane's pull request |
| The gate's report for each pull request | `~/training-out/<lane>/promote-<t>.md` |
| The loop's log, each Devin session's export, and each run of the web's tests | `~/logs/<lane>.log`, `~/logs/<lane>-<t>.json`, `~/logs/<lane>-web-<t>.log` |

## Stopping, starting and watching a lane

```sh
sudo systemctl stop jackioh-train@improve      # stop the lane now (its Devin session ends with it)
sudo systemctl disable jackioh-train@improve   # and keep it stopped across reboots
sudo systemctl enable --now jackioh-train@improve
systemctl status jackioh-train@unban
tail -f ~agent-train-unban/logs/unban.log
tail -f ~agent-train-gather/logs/gather.log
```

To refuse a promotion, close its pull request: the loop opens a new draft with its next session,
from `main`. A lane notices a new `loop.sh` on `main` at the start of a cycle, or while it waits on a
promotion, and restarts on it. A lane's thresholds change only by a pull request from a person to
`crates/engine/src/config.rs`.

## Reading the history

`crates/ai/generation.json` is the AI on `main`. `training/history/<lane>.jsonl` holds one line per
promotion of that lane, oldest first:

```json
{"generation":8,"lane":"improve","parent":"<main commit>","tree":"<crates/ai/src tree>","vsRandom":"93/100","vsParent":"87/100","shadowBan":11,"parentShadowBan":11,"date":"2026-11-02"}
```

- `generation`: the parent's plus one. Generations count across both lanes: they share `main`.
- `parent`: the `main` commit the parent was built from; `tree`: the AI's source tree, which names
  the seeds (`<lane>:<tree>:1` … `:100`), so anyone can re-run the gate:
  `git checkout <parent> && cargo build --release -p jackioh-tools && cp target/release/jackioh /tmp/parent`,
  then on the promotion's commit `cargo jackioh promote --lane <lane> --parent-bin /tmp/parent --verify`.
- `vsRandom`, `vsParent`: the candidate's wins out of `TRAINING_GAMES`. `vsRandom` is `null` from
  generation 10 on: the lanes play no random games (R732001).
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
