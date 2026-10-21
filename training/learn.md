# The learn lane

The learned evaluation's lane (#466, #1012; `docs/learned-eval.md`). It keeps the AI's learned
evaluation (`crates/ai/src/learned.rs`) trained on the AI's own games and promotes a better one to
`main` through the same kind of gate every generation passes. Its decisions are a script's, not an
LLM's (D2), so this file is its contract rather than a prompt, as `gather.md` is the gather lane's.

It is the other half of the AI. The `improve` and `unban` lanes are **supervised**: a Muse session
reads the AI, changes the hand-written evaluation or the search, and measures each change
(`improve.md`, `unban.md`). The `learn` lane is **learned**: a network fitted to the outcomes of
games corrects the hand eval where the search finally scores a line. The hand eval stays the
network's input (D4's skip connection) and the beam's pruning, so the supervised lanes keep improving
the AI the network corrects, and the network is retrained on their work.

## How it runs

`.github/workflows/learn.yml`, weekly and by dispatch, on the owner's PC alone: every job of it runs
on the local runner (`jackioh-selfplay-local`, `bot/machine/README.md`, The local runner), none on
GitHub's runners. That runner takes one job at a time and only while the PC is otherwise idle, so the
export's shards play one after another, at about 2,500 games an hour on its six CPUs: a run's 100,000
games take about 40 hours of an idle PC, and the retrain, the screen and the gate about two more. A
week's run may take several days; the next one waits for it (one run at a time). Each run:

1. **Waits** while the lane's pull request (`ai/learn`) is open with its checks green or running: a
   promotion is on its way to `main`. One with a failed check, or open more than 48 hours
   (`LEARN_PR_STALE_HOURS`), it closes, saying why, and goes on.
2. **Exports** `games` games (100,000 by default) through `selfplay.yml` on a fresh base,
   `learned-<catalog>-<date>-<run>`, never a gate's deals or the dev set's (R1006002), in `shards` jobs
   (20 by default, about two hours each) played one after another, in five rounds: a round starts
   once the one before it has ended, so no shard waits for the runner longer than its round takes
   (about 8 hours of an idle PC), well under the 24 hours GitHub lets a job wait.
3. **Retrains** with `training/ml/retrain.sh` on `main`, over the shards that finished (a shard the PC
   lost costs the export its games, not the run; with none finished the run stops): the positions
   (R1006003, R1006004), folded by the export's own build while its features are `main`'s (`--positions-bin`), so a rules change merged
   since the export drops none of its games (run 5 lost 10,707 of 100,000 to #1199 and #1200), the fit
   and its report (`training/ml/README.md`), the integer network that ships (D7, R1011000) into
   `crates/ai/src/learned/weights.json` and the switch, `AI_EVAL_KIND`, on. It stops when the network
   does not beat the calibrated hand eval on held-out turns 4 to 10. The fit trains six seeds; the best by validation whose integer network
   passes the AI's tests with the switch on ships (`cargo test -p jackioh-ai`, as the promotion's CI
   will run them), and it stops when none does.
4. **Screens** the candidate against the AI on `main` on the improve lane's dev set: 800 paired games
   (`arena --set improve-dev --games 800`), a count fixed before it is played, never extended or played
   again to read better (`improve.md`, How you measure). It goes on only when `edgeLow` is above zero.
5. **Gates** on `main` as it is now, with `training/ml/learn-promote.sh`: the network goes onto
   `origin/main` as fetched (with that `main`'s AI built again as the parent, and the AI's tests run
   again with the network when `main` moved since the screen), then `promote --lane learn`, the improve
   lane's bar (`TRAINING_LEARN`, 220 of 400 paired games) on the lane's own hold-out deals,
   `learn:<tree>:<d>` (R1012000, R850001), and the practice-claim fixture recorded again by the new AI.
   A gate that does not pass ends the run.
6. **Delivers** a pass as `AI gen <N> (learn): the learned evaluation, retrained on <games> games`: the
   weights, the switch, `crates/ai/generation.json`, a line of `training/history/learn.jsonl` and the
   fixture, pushed to `ai/learn` with auto-merge on. CI's training gate plays the gate again
   (`promote --verify`) and every required check runs before it merges.

Each run keeps its reports as the artifact `learn-<run>` (the fit's report, the screen's tally, each
gate's report, `deliver.json`, the weights) for 90 days, and its summary says how far it got. The
night bot's status issue ("Night bot status", pinned) has a section for the lane: the generation on
`main`, the lane's pull request, the next scheduled run and how each of the last runs ended
(`bot/harness/dashboard.py`, `learn_lane`).

## Running unattended

Nobody needs to watch the lane, and no model does: every decision above is a script's, and the lane
mends what would otherwise stop it. What can go wrong, and what it does:

| What happens | What the lane does |
|---|---|
| `main` moves during the run. It does, about once an hour, in the AI, the engine and the cards: CI's training gate replays the gate against `main` as it is when the pull request opens, and the fixture's game is played on the merge | The gate plays on `main` as fetched just before it (step 5), not on the `main` the run began on. When `main` moved in what the gate depends on during the gate (`learn-promote.sh`'s `play_paths`), the network goes onto the newer `main` and is gated again, up to three times (`LEARN_GATE_TRIES`). Only a pass is ever gated again, so no pass is picked out of several. When `main` moved through all three, the run starts another on the same export (`export_run`), at most two in a row (`LEARN_CHAIN_MAX`) |
| CI fails on the lane's pull request | `watch` (the same workflow, on CI's completion for `ai/learn`) runs the failed jobs once more, for a flaky test. Red again, it closes the pull request, naming the failed jobs, and starts a run on the same export on `main` as it is by then, at most two in a row |
| A promotion stays open: its checks failed and nothing closed it, auto-merge could not merge it, or a check never reported | The next run closes it, saying why, and goes on (step 1). After 48 hours it closes one whatever its checks say |
| Something merged to `main` that pins the AI's own play outside `crates/ai` (as R1303002's fixture did, #1480) | The lane can't fix that: it may change nothing outside its files. CI fails, `watch` closes the promotion and tries the export again, and the lane stays stuck until a person fixes the test. The status issue shows it as a run whose promotion closed |
| The screen or the gate does not pass | Nothing: that is the lane working. A run's network passes the screen about 70% of the time and the gate about 30%, so expect a promotion every few weeks. Each run's export is new self-play by the AI on `main`, so each is a new candidate |
| `LEARN_LANE_TOKEN` is missing or has expired | A passing run fails in red at its deliver step and keeps the promotion as its artifact (`promotion.patch`). A person renews the token (Setting it up) |
| GitHub skips or delays the weekly schedule | The next week's run, or `gh workflow run learn.yml`, picks it up. Nothing is lost |
| The PC is in use | The runner works only while the PC is idle: a job waiting is not started, and one in the middle of its work is slowed to half a CPU until the PC is idle again. Nothing is lost; the run takes longer (its timeouts allow days) |
| The PC is off, asleep or busy for 24 hours | GitHub fails a job no runner takes in 24 hours. A lost shard costs the export its games and the retrain goes on with the rest; a lost plan or retrain ends the run, and the next week's run comes anyway. A shard in the middle of its work when the PC sleeps or restarts is lost the same way. The status issue shows a run the PC never took as "⏸️ the PC's runner took none of it in 24 hours" |
| The PC's supervisor is not running (nobody logged in since a restart) | The same as the PC off: its scheduled task starts it at logon (`bot/machine/README.md`, The local runner) |

What needs a person, then: a test outside `crates/ai` that pins the AI's play, an expired token, a PC
that stays off, or a change to the exporter, the features, the trainer or the gate.

## What it may change

Only `crates/ai/src/learned/weights.json`, the switch in `crates/ai/src/config.rs`,
`crates/ai/generation.json` and `training/history/learn.jsonl`, as every lane is held to `crates/ai/**`
and `training/history/**` (CI's training gate), and the practice-claim fixture
(`crates/server/tests/fixtures/practice-claim.json`, R1303002), which the run records again with the
new AI after its gate passes, since every AI change plays that game differently. It never changes the
exporter, the features, the trainer or the gate. A change to those is a person's pull request, measured with the same script:
a feature change bumps `FEATURES_VERSION` and refits from stored logs with no new games (D3).

## Setting it up

The runner: the local runner set up on the PC as `bot/machine/README.md` (The local runner) says,
from an image built with `gh` (`docker build -t jackioh-pc-runner:latest bot/machine/local` again
after a change to its Dockerfile), and its supervisor serving `learn.yml` (`config.json`). The PC
should not sleep while it is plugged in, or a shard in the middle of its work is lost each time it
does. Without the runner no run starts at all: its jobs wait 24 hours and fail.

The token: a person does this once: create a fine-grained token that can write contents and pull requests on
this repository alone, and save it as the repository secret `LEARN_LANE_TOKEN` (Settings → Secrets
and variables → Actions). A pull request opened with a run's own token would start no CI. Without
the secret, or once it has expired, a passing run fails in red at its deliver step and stops at its
artifact (`promotion.patch`), which a person can push. A fine-grained token expires on the date it was
given: renew it before then.

To run it now: `gh workflow run learn.yml` (`-f games=… -f shards=… -f screen=800`). `-f
export_run=<run id>` plays no export and retrains on that earlier run's, kept 30 days: a change to
the fit or the search is measured on the same games without 40 hours of self-play. The gate's deals are the candidate tree's (R850001): a changed tree plays new ones. To
stop it, disable the workflow (`gh workflow disable learn.yml`); to refuse a promotion, close its
pull request.
