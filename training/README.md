# The AI training lanes

Two lanes that keep improving JackiOh's AI (`crates/ai/`, SPEC §9.9), each a Devin session after
another on the training box, each gated by a fixed number of games against the AI on `main`, and
nothing else: since generation 9 they play no games against random (R732001). They
succeed #55's ladder (the Python `ladder/` that v0.3.0 replaced and deleted). The design is docs/v0.3.0/README.md §8; the interfaces are
docs/v0.3.0/SURFACE.md §14.

| Lane | Goal | Standing prompt |
|---|---|---|
| `improve` | a stronger AI: beat its predecessor in 220 of 400 games, 200 deals each played twice with the seats swapped (R850000) | [`improve.md`](improve.md) |
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
measure a candidate against its parent alone: `unban` at 60 of 100, and `improve` at 65 of 100 until
issue #850 lowered its bar to 220 of 400 paired games (R850000), so that the few points a change
gains can bank and compound. The daily quality gates (`cargo jackioh gate --full`) still hold the AI
on `main` to their floor against random, so an AI that forgets how to beat random fails the daily
run.

## How a lane runs

The training box (an always-on EC2 `m7i-flex.large`, set up by #306's part 39) runs three systemd services,
`jackioh-train@improve`, `jackioh-train@unban` and `jackioh-train@gather`, each as its own Linux user
(`agent-train-improve`, `agent-train-unban`, `agent-train-gather`) with its own checkout (the two
training lanes also hold a Devin login and GitHub token, which the gather lane needs neither of). Each runs
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
   - The `improve` lane then checks out its stack, `ai/improve-stack`, rebased onto `main` (The
     improve stack): the session starts from its kept changes, not from bare `main`.
   - The `unban` lane stops here while the parent shadow-bans no card. A promotion needs strictly
     fewer bans, so a session could do nothing. The lane retitles its draft
     `AI gen <N> (unban): idle, the AI on main bans no card`, says so once in a comment, and looks
     again every `TRAIN_IDLE_SECONDS` (15 minutes). The first cycle after `main`'s AI bans a card
     again starts a session. A sweep of record adds such bans, as #732's did for the Meditative set,
     and both standing prompts send the lanes to the Meditative cards first.
   - Otherwise push `ai/<lane>` as that start (`main`, or the improve lane's rebased stack) plus an
     empty `[skip ci]` commit naming the session, and make the lane's pull request this session's
     draft, titled
     `AI gen <N> (<lane>): in training, nothing promoted yet` (opening one if none is open). Its
     description opens with the gate's latest pass (Watching a lane on GitHub).
3. Run one Devin session on the lane's standing prompt
   (`devin -p --prompt-file training/<lane>.md --model "$DEVIN_MODEL" --permission-mode dangerous
   --respect-workspace-trust false --export ~/logs/<lane>-<t>.json`). Devin changes the AI and
   measures it: the `unban` lane with dry runs of the gate, committing only once a real run has
   passed; the `improve` lane with paired screens on its dev set, committing each change it keeps to
   its stack and going to the gate only with a stack that screened above its parent
   ([`improve.md`](improve.md)). A session ends after a promotion or after four hours. Meanwhile,
   every 15 minutes (`TRAIN_PROGRESS_SECONDS`), the loop pushes the checkout as it is to `ai/<lane>`
   as one `[skip ci]` commit when it changed, rewrites the session's comment on the draft, and
   rewrites the draft's description when a new pass of the gate changed it. When the session ends,
   the loop stops any `jackioh` process it left running as the lane's user (an arena or a gate cut
   off by the time limit, and the parent agents it spawned), so the next measurement has the box.
   Without a promotion, the `improve` lane saves the session's commits as its stack.
4. When the session ends with a promotion commit, fold every commit of the session into it (the
   improve lane's stack included) and land it on `main`. If `main`'s crates did not move
   since the session's start, the parent and the candidate are the builds the gate measured: rebase
   the commit and re-check it against the parent (`cargo jackioh promote --verify`). If they moved,
   the parent changed: build it from `main` again, apply the promotion's change (less the record the
   gate writes) to `main`'s AI and play the gate again for real, which renumbers the generation.
   Then run the web's tests that play the AI (the tutorial's lessons and practice, as the standing
   prompts' "Beyond the gate" lists them) when the box has `pnpm`, push `ai/<lane>`, retitle the pull
   request with the promotion's subject (`AI gen <N> (<lane>): <what changed>`), give it the gate's
   report as its body, mark it ready and turn on auto-merge (squash). A promotion that fails any of
   that is set back as in 1. Until it merges, the improve lane's promotion, less its record, is its
   stack, so a set-back promotion is where the next session starts.
5. Sleep 60 seconds and repeat.

A lane never merges anything itself: CI does, through branch protection, once its checks pass.

### The improve stack

The `improve` lane banks its work between sessions on a branch of its own, `ai/improve-stack`, so
a measured gain too small for the gate is not lost when a session ends or `main` moves (issue
#850). It holds the lane's kept changes as commits on `main`, one per change, each subject
`Stack (improve): <what changed>` and each body the screen that kept it (its games, wins, edge and
interval, and the control it was measured against; [`improve.md`](improve.md), The stack). No stack
commit holds anything outside `crates/ai/`, nor `crates/ai/generation.json` or a history line, and
none is a promotion (its subject never starts `AI gen `).

- **When it is saved.** A session without a promotion ends with its commits pushed to
  `ai/improve-stack` (`save_stack`), uncommitted work left out. A session whose commits change
  anything else is refused as a stack: the stack stays as it was, the commits are kept on the box as
  the local branch `ai/improve-stack-unsaved-<t>`, and `attempts.md` says so.
- **A promotion.** The loop folds the session's commits into the promotion commit, as it always
  has, and pushes it less its record as the stack: one commit, its message the promotion's subject
  and every folded commit's message (`save_promoted_stack`). A promotion set back, or refused by
  closing its pull request, is where the next session starts; once it merges, `main` holds its
  change, and the next cycle empties the stack to `main`.
- **Each session.** Step 2 fetches the stack. When `main` already holds its change, it is emptied.
  Otherwise it is rebased onto `main` and checked out as `ai/improve`, and the session starts from
  it. Its comment on the draft says so.
- **When `main`'s AI moved under it** (`crates/ai` differs between the `main` the stack was on and
  today's, say after an unban promotion), the loop builds the rebased stack and screens it against
  the new parent on the dev set (`arena --a self --b bin:~/parent-jackioh --set improve-dev`, 400
  games, about 40 minutes on the box) before the session, and puts the tally in `attempts.md`. The
  session reads it first; a stack that lost its edge is the session's to fix or drop. That screen
  reads higher than the gate would, as every dev-set read can: the gate is the hold-out read.
- **A rebase conflict.** When the stack does not rebase onto `main`, the loop aborts the rebase,
  keeps the stack's commits as the local branch `ai/improve-stack-conflict-<t>`, notes the conflict
  and its files in `attempts.md`, and starts the session from `main`. The stack stays on GitHub as
  it was. The session rebuilds it first (`improve.md`, The stack): it cherry-picks the kept commits
  onto `main`, resolving the conflicts in `crates/ai`, and screens the result against the parent.
  Its commits replace the stack when it ends; a session that commits nothing leaves it as it was. A
  stack is never dropped without a note.
- **A stack the loop cannot read** (GitHub unreachable at the session's start) is left alone: the
  session starts from `main`, `attempts.md` says so, and its commits are kept as a local branch
  rather than pushed over the stack.

The gate still measures only what is committed: `promote` refuses uncommitted changes in
`crates/ai/src`, so the promotion's seeds are the committed stack's (R850001).

### Watching a lane on GitHub

The lane's pull request is the place to follow it. Its **Files changed** is the session's work in
progress against `main` (CI does not run on a `[skip ci]` commit), the top of its description is
the latest pass, and its comments say what happened:

- the latest pass of the gate, at the top of the draft's description: every run of `promote` (a dry
  run, a promotion or a `--verify`) rewrites `~/training-out/<lane>/last-pass.md` with it, failed
  runs included, and the loop puts that file above the lane's own text within 15 minutes. It says
  how many of the games the candidate beat the AI on `main` in, with what the lane needs, a table of
  the games won, lost, drawn and without a result as p1, as p2 and in all, the interval those wins
  give and the shadow bans, and a grid with one square per game (🟩 beat the bot, 🟥 lost, ⬜ drawn,
  ⬛ no result). Arena runs are not passes and do not change it;

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

`cargo jackioh promote --lane <lane> --parent-bin <parent> [--dry-run] [--verify]` plays the lane's
games against the parent (`TRAINING_IMPROVE.games`, 400; `TRAINING_UNBAN.games`, 100):

- game `k`'s seed is `<lane>:<tree>:<k>`, where `<tree>` is `git rev-parse HEAD:crates/ai/src`, so
  the seeds name the AI being measured and committing `generation.json` does not move them;
- the candidate sits p1 on odd games and p2 on even ones;
- the improve lane's gate is paired (R850000): its `k` is a deal, and each deal is played twice
  with the seats swapped, game `2k − 1` with the candidate on p1 and game `2k` with it on p2, the
  same decks, shuffles and first seat in both. Its 200 deals are the lane's hold-out set (R850001):
  named for the committed AI and read only by it, never by a screen;
- each seat's deck is `build_ai_deck` over every set that ships (Meditative included since
  generation 9) minus its own AI's shadow ban, with no handicap (Easy both). Both seats are dealt
  by that one rule, and neither is shaped by the lane's dealt-quality table (`DEALT_Q`; the deck
  builder's `shape` option, off), so the gate's decks are drawn as practice's and the daily gates'
  are (R976000). Until issue #976 the builder shaped every seat whose ban list was this build's
  own, so in the unban lane the candidate's seat was always shaped and the parent's never. Both AIs
  still mulligan by `mulligan_keep_shaped`, which is part of how the AI plays;
- the AI plays at `AI_GATE_BUDGET`, the browser's own budget, so the gate measures the AI that ships.

| Lane | vs parent | Shadow bans |
|---|---|---|
| `improve` | ≥ 220 of 400 paired games (`TRAINING_IMPROVE.vs_parent`) | — |
| `unban` | ≥ 60 of 100 (`TRAINING_UNBAN.vs_parent`) | strictly fewer than the parent's |

220 of 400 is about two standard deviations above an equal successor's 200: one passes at most about
2.6% of the time, less with the luck pairing cancels, while a successor that truly wins 55% passes
about half the time and one at 58% nine times in ten (R850000). It was 65 of 100 until issue #850.

It plays no games against random (R732001), and a promotion records `"vsRandom": null`. A draw is not
a win, and neither is a game that ended without a result. Exit 0 on a pass, 1 on a failed gate. The
report (Markdown) goes to stdout; a paired gate's report and latest pass add the paired edge, the
candidate's wins less the parent's over each deal's two games, in points of the games, with its 95%
interval.

Against a parent whose shadow ban is empty, the unban lane's gate cannot pass (strictly fewer than
none), so `promote --lane unban` says the lane has nothing to do and exits 1 before it plays a game,
whatever the flag, writing nothing and printing no report.

- `--dry-run` measures and writes nothing. The unban lane's runs on a working copy with uncommitted
  changes in `crates/ai/src` (on HEAD's seeds, and says so). The improve lane's refuses one: its
  deals are read only by the committed AI they are named for (R850001), so a dry run on a clean
  checkout plays exactly the games the promotion would.
- Without a flag, a pass writes `crates/ai/generation.json` and appends the same object to
  `training/history/<lane>.jsonl`. It refuses to run while `crates/ai/src` has uncommitted changes,
  since the seeds come from the committed tree: commit the change first, then run it, then commit
  the two files (the unban lane amends them into the change's commit; the improve lane commits them
  alone on top of its stack, as the promotion commit).
- `--verify` (CI, and `loop.sh` before it pushes) recomputes and compares every field of the
  branch's `generation.json` except `date` and `parent`.

The games run on rayon's threads (`RAYON_NUM_THREADS` is respected); no result depends on the
thread count.

### The arena and the agent

- `cargo jackioh arena --a self|random|bin:<path> --b self|random|bin:<path> --games N --seed <base>
  [--paired] [--out <dir>]` plays games `1..=N` with seeds `<base>:<n>`, agent `a` on p1 in odd
  games. It is the free-form version of the gate, for experiments on other seeds than the gate's.
  With `--paired` (R850000) it plays deals instead: deal `d` is `<base>:<d>`, played twice with the
  seats swapped, so `N` is even. Per game it prints duel.ts's line (who won, why, the hash, whether
  the log replays, decision times) and a tally at the end: the wins, draws and games without a
  result, and agent `a`'s win share with its 95% Wilson interval (`aShare`, `aLow`, `aHigh`); a
  paired series adds `deals` and `edge`, the mean over deals of `a`'s wins less `b`'s over the deal's
  two games (−1 to 1), with its 95% interval (`edgeLow`, `edgeHigh`). The gate's report gives the
  same intervals for its wins. A base that is a lane's name, or one followed by `:`, is refused:
  those are the gates' own seeds (R850001).
- `cargo jackioh arena --a … --b … --set improve-dev [--games N]` plays the improve lane's fixed dev
  set, deals `improve-dev:1` to `:200`, paired, 400 games unless `--games` says more, never fewer
  (R850001). The improve lane screens every change on it (`improve.md`, How you measure); the gate's
  hold-out deals are never a screen's.
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
   A patch has no cap: the lane plays it until `main` ships another, about 4,000 games a day. In
   Postgres each game is folded into the card statistics' totals as it loads, which grow with the
   cards and not the games, and its detail is kept `GAME_RECORD_RAW_RETENTION_DAYS` (7), so a week of
   the lane's games, some 50 MB, is all the detail held (R1044000, R1044001). The files here keep
   every game's detail.
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

- Run `setup.sh --training` on the training box (`bot/machine/README.md`, The training box),
  whose `TRAIN_LANES` already holds `gather`. It creates the user `agent-train-gather` with its own
  checkout and the service `jackioh-train@gather`.
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

## The ladder bots after a promotion

The ladder bots' strengths are measured against each other on the AI on `main` (R748000,
`docs/bot-calibration.md`), beside the skill curve their seed ratings are read from (R747000), so a
merged promotion is followed by that document's re-calibration: `cargo run --release -p
jackioh-server -- bot-calibrate`, run by a person or the night bot, whose report is compared with the
curve and, where it should win, pasted into `crates/server/src/config.rs` in a pull request. A lane
never edits `crates/server`, and a promotion does not wait for it.

## Where the numbers go

| What | Where |
|---|---|
| Every game the arena plays (a promotion run or an experiment): one `GameRecord` (R376) per line, `source: "dev"`, `mode: "random"` | `~/training-out/<lane>/<date>.jsonl` (`$JACKIOH_TRAINING_OUT`; `arena` without it prints to stdout) |
| The same records in Postgres, loaded once per finished day when the box has `DATABASE_URL`: folded into the card statistics' totals as they load, each record's detail kept a week (R1044000, R1044001) | `jackioh-server stats-import`, read back with `jackioh-server stats-cards --source=dev` (the totals) or `stats-export --source=dev` (the last week's records) |
| The gather lane's records: one `GameRecord` per line, a file a day | `~/training-out/gather/<date>.jsonl` |
| The gather lane's next game number for a patch, and that patch's card figures, rewritten each cycle | `~/training-out/gather/cursor-<patch>`, `cards-<patch>.txt` and `cards-<patch>.json` |
| The days already loaded into Postgres (each lane's `~/training-out/<lane>/`) | `imported.txt` |
| A promotion's gate numbers | `training/history/<lane>.jsonl` and `crates/ai/generation.json`, committed with it |
| The latest pass of the gate (any mode, failed ones included), in Markdown: the top of the lane's draft pull request | `~/training-out/<lane>/last-pass.md`, rewritten by every run of `promote` |
| What each Devin session tried, with each screen's numbers and interval, why the loop set a promotion back, and what it did with the improve stack (a rebase conflict, a re-measure) | `~/training-out/<lane>/attempts.md`, quoted in each session's comment on the lane's pull request |
| The improve lane's kept changes between sessions, one commit each | the branch `ai/improve-stack` on GitHub (The improve stack) |
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
from `main`, or for the improve lane from its stack, which still holds the refused promotion's change
(The improve stack). To refuse the improve stack's changes as well, stop the lane, delete the branch
`ai/improve-stack` on GitHub, then start the lane: its next session starts from `main` and saves a new
stack. Delete it only while the lane is stopped, or the session running then saves it again. A lane
notices a new `loop.sh` on `main` at the start of a cycle, or while it waits on a promotion, and
restarts on it. A lane's thresholds change only by a pull request from a person to
`crates/engine/src/config.rs`.

## Reading the history

`crates/ai/generation.json` is the AI on `main`. `training/history/<lane>.jsonl` holds one line per
promotion of that lane, oldest first:

```json
{"generation":8,"lane":"improve","parent":"<main commit>","tree":"<crates/ai/src tree>","vsRandom":"93/100","vsParent":"87/100","shadowBan":11,"parentShadowBan":11,"date":"2026-11-02"}
```

- `generation`: the parent's plus one. Generations count across both lanes: they share `main`.
- `parent`: the `main` commit the parent was built from; `tree`: the AI's source tree, which names
  the seeds (`<lane>:<tree>:1` … `:100`, or the improve lane's deals `:1` … `:200`), so anyone can
  re-run the gate:
  `git checkout <parent> && cargo build --release -p jackioh-tools && cp target/release/jackioh /tmp/parent`,
  then on the promotion's commit `cargo jackioh promote --lane <lane> --parent-bin /tmp/parent --verify`.
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
