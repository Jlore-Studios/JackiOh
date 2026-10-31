# The learn lane

The learned evaluation's lane (#466, #1012; `docs/learned-eval.md`). It keeps the AI's learned
evaluation (`crates/ai/src/learned.rs`) trained on the AI's own games and promotes a better one to
`main` through the same kind of gate every generation passes. Its decisions are a script's, not an
LLM's (D2), so this file is its contract rather than a prompt, as `gather.md` is the gather lane's.

Since 2026-10-25 it is how the AI keeps getting stronger. The `improve` and `unban` lanes were
**supervised**: a Muse session read the AI, changed the hand-written evaluation or the search, and
measured each change (`improve.md`, `unban.md`). #682's audit found that they made no fair promotion
in about 427 hours, so both are stopped (#1677). The `learn` lane is **learned**: a network fitted to
the outcomes of games corrects the hand eval where the search finally scores a line, and no LLM
takes part in any step. The hand eval stays the network's input (D4's skip connection) and the
beam's pruning, so a person's or the night bot's change to it, through an ordinary issue, still
improves the AI the network corrects, and the next run retrains on it.

## How it runs

`.github/workflows/learn.yml`, one run after another and by dispatch, on the owner's PC alone: every job of it runs
on the local runner (`jackioh-selfplay-local`, `bot/machine/README.md`, The local runner), none on
GitHub's runners, whose hosted jobs CI needs (an export there held 12 of the organization's 20 for
hours, #1455). That runner takes one job at a time and only while the PC is otherwise idle, so the
export's shards play one after another, at about 2,500 games an hour on its six CPUs: a run's 100,000
games take about 40 hours of an idle PC, and the retrain and the screen about one more (run 21's
took 59 minutes there), the gate another. A run may take several days, and each starts the next as it
ends (#1976), so the PC is always exporting, retraining or gating while it is idle; a schedule twice a
day starts the lane again should that chain stop (one run at a time). Each run:

1. **Clears the way.** The lane's pull request (`ai/learn`) with a failed check, or open more than
   48 hours (`LEARN_PR_STALE_HOURS`), it closes, saying why. One whose checks are green or running is a
   promotion on its way to `main`: it merges while this run exports, about 40 hours, and should it
   still be open when this run's gate passes, the pass waits behind it (step 6).
2. **Exports** `games` games (100,000 by default) through `selfplay.yml` on a fresh base,
   `learned-<catalog>-<date>-<run>`, never a gate's deals or the dev set's (R1006002), in `shards` jobs
   (20 by default, about two hours each) played one after another, in five rounds: a round starts
   once the one before it has ended, so no shard waits for the runner longer than its round takes
   (about 8 hours of an idle PC), well under the 24 hours GitHub lets a job wait. The plan and the
   export's build read `main` as it is when the run starts, not as it was when the run was queued,
   so the export plays the newest patch however long the run waited behind the one before it (A new
   patch ships, below).
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
   again with the network when `main` moved since the screen), then `promote --lane learn`, every
   lane's gate (`TRAINING_GATE`, R1679000) on the lane's own hold-out deals, `learn:<tree>:<d>`
   (R1012000, R850001), and the practice-claim fixture recorded again by the new AI. The gate is a
   sequential test: it plays deals 100 at a time, each twice with the seats swapped, and stops as soon
   as the evidence is clear, a pass when the candidate looks a 53% AI (an edge of +0.06, about +21 Elo)
   and a fail when it looks no stronger than its parent, each wrong about 5% of the time; a test still
   undecided at 1,000 deals fails. It plays about 800 games, 2,000 at most: under an hour on the PC.
   A gate that does not pass ends the run.
6. **Delivers** a pass as `AI gen <N> (learn): the learned evaluation, retrained on <games> games`: the
   weights, the switch, `crates/ai/generation.json`, a line of `training/history/learn.jsonl` and the
   fixture, pushed to `ai/learn` with auto-merge on. CI's training gate plays the gate again
   (`promote --verify`) and every required check runs before it merges. While the lane's last
   promotion is still open, a pass waits behind it instead ("a promotion still open"): pushing
   `ai/learn` again would close that one, so this one stays the run's artifact (`promotion.patch`) and
   the next run gates on `main` once the open one has merged.
7. **Files the export's games** into production's card statistics (`import`, once the retrain has ended,
   however it ended): `cargo jackioh stats from-selfplay` over the shards that finished, with the build
   that played them (R1676000), then `jackioh-server stats-import --totals-only` (R1976000), which adds
   their counts to the totals and keeps no row per game, so the database grows with the patches, not the
   games. It needs `STATS_IMPORT_DATABASE_URL` (Setting it up); without it nothing is filed. A run on an
   earlier run's export (`export_run`) files nothing new.
8. **Starts the next run** (`next`), unless one is already on its way (a run on the same export, or the
   schedule's) or this one took under an hour (`LEARN_NEXT_MIN_MINUTES`): one that breaks that early
   would break again, so it waits for the schedule, at most 12 hours.

Each run keeps its reports as the artifact `learn-<run>` (the fit's report, the screen's tally, each
gate's report, `deliver.json`, the weights) for 90 days, and its summary says how far it got. The
night bot's status issue ("Night bot status", pinned) has a section for the lane: the generation on
`main`, the lane's pull request, the next scheduled run and how each of the last runs ended
(`bot/go/internal/dashboard`, `LearnLane`).

## Running unattended

Nobody needs to watch the lane, and no model does: every decision above is a script's, and the lane
mends what would otherwise stop it. What can go wrong, and what it does:

| What happens | What the lane does |
|---|---|
| `main` moves during the run. It does, about once an hour, in the AI, the engine and the cards: CI's training gate replays the gate against `main` as it is when the pull request opens, and the fixture's game is played on the merge | The gate plays on `main` as fetched just before it (step 5), not on the `main` the run began on. When `main` moved in what the gate depends on during the gate (`learn-promote.sh`'s `play_paths`), the network goes onto the newer `main` and is gated again, up to three times (`LEARN_GATE_TRIES`). Only a pass is ever gated again, so no pass is picked out of several. When `main` moved through all three, the run starts another on the same export (`export_run`), at most two in a row (`LEARN_CHAIN_MAX`) |
| A patch or a micro ships (A new patch ships, below) | Nothing stops: the next run plays it, since each run plans on `main` as it is when it starts |
| A set ships, a `major` patch (A new patch ships, below) | `learn-set.yml` cancels a run still in its export with fewer than 10 of its 20 shards played, and starts one on the new set when no other run is on its way. A run further on finishes, and the run after it exports the new set. The status issue shows a stopped run as canceled |
| CI fails on the lane's pull request | `watch` (the same workflow, on CI's completion for `ai/learn`) runs the failed jobs once more, for a flaky test. Red again, it closes the pull request, naming the failed jobs, and starts a run on the same export on `main` as it is by then, at most two in a row |
| A promotion stays open: its checks failed and nothing closed it, auto-merge could not merge it, or a check never reported | The next run closes it, saying why, and goes on (step 1). After 48 hours it closes one whatever its checks say |
| Something merged to `main` that pins the AI's own play outside `crates/ai` (as R1303002's fixture did, #1480) | The lane can't fix that: it may change nothing outside its files. CI fails, `watch` closes the promotion and tries the export again, and the lane stays stuck until a person fixes the test. The status issue shows it as a run whose promotion closed |
| The screen or the gate does not pass | Nothing: that is the lane working. A run's network passes the screen about 70% of the time and the gate about 30%, so with runs one after another (about three a week) expect a promotion about once a week. Each run's export is new self-play by the AI on `main`, so each is a new candidate |
| `LEARN_LANE_TOKEN` is missing or has expired | A passing run fails in red at its deliver step and keeps the promotion as its artifact (`promotion.patch`). A person renews the token (Setting it up) |
| A run ends | It starts the next (step 8). A run that broke within the hour starts none, and the schedule (06:23 and 18:23 UTC) starts the lane again |
| GitHub skips or delays the schedule | It matters only once the chain has stopped: the next one, 12 hours on, or `gh workflow run learn.yml`, picks it up. Nothing is lost. A scheduled run queued behind a running one waits; the next schedule's replaces it (the `learn` concurrency group holds one waiting), and the status issue leaves out a run canceled that way |
| `STATS_IMPORT_DATABASE_URL` is missing, or the import fails | The export's games are not filed; the retrain, the gate and the next run go on as before. The shards stay the run's artifacts for 30 days, so a person can file them by hand (`training/README.md`, The gather lane) |
| The PC is in use | The runner works only while the PC is idle: a job waiting is not started, and one in the middle of its work is slowed to half a CPU until the PC is idle again. Nothing is lost; the run takes longer (its timeouts allow days) |
| The PC is off, asleep or busy for 24 hours | GitHub fails a job no runner takes in 24 hours. A lost shard costs the export its games and the retrain goes on with the rest; a lost plan or retrain ends the run, and the next run comes anyway. A shard in the middle of its work when the PC sleeps or restarts is lost the same way. The status issue shows a run the PC never took as "⏸️ the PC's runner took none of it in 24 hours" |
| The PC's supervisor is not running (nobody logged in since a restart) | The same as the PC off: its scheduled task starts it at logon (`bot/machine/README.md`, The local runner) |

What needs a person, then: a test outside `crates/ai` that pins the AI's play, an expired token, a PC
that stays off, or a change to the exporter, the features, the trainer or the gate.

### A new patch ships

The network learns a worth for each card it has seen played, on top of the card's stats, which it
reads live (`docs/learned-eval.md`, D5 and D10). A patch leaves the learned worth of the cards it
changed stale until a run exports on it, and a set's new cards have none until then: they are valued
by their stats alone.

Patches and micros (`docs/issues-and-patches.md`, Version numbers) ship two or three times a day,
and a run takes about two days, about 40 hours of them its export. So a patch or a micro stops
nothing: a lane that started again on each would never finish an export. The next run plays it,
since each run plans on `main` as it is when it starts (step 2), so it waits at most one run.

A set (a `major` patch, about once a week; v0.5.0 brought 231 card changes) is what an export on the
set before cannot learn at all. `.github/workflows/learn-set.yml`, on the push that ships it, cancels
a run still in its export with fewer than `LEARN_SET_KEEP_SHARDS` (10, half of the 20) of its shards
played, and starts a run when no other is on its way (one waiting behind the stopped run starts by
itself, on the new set). From half on, finishing the export costs the lane less than playing it
again, so that run goes on, as does a run past its export, one on an earlier run's export and one
that planned after the set shipped; the run after it exports the new set. A stopped run's finished
shards stay its artifacts for 30 days, and their games are not filed (step 7): a person can file
them by hand (`training/README.md`, The gather lane).

## What it may change

Only `crates/ai/src/learned/weights.json`, the switch in `crates/ai/src/config.rs`,
`crates/ai/generation.json` and `training/history/learn.jsonl`, as every lane is held to `crates/ai/**`
and `training/history/**` (CI's training gate), and the practice-claim fixture
(`crates/server/tests/fixtures/practice-claim.json`, R2560000), which the run records again with the
new AI after its gate passes, since every AI change plays that game differently. It never changes the
exporter, the features, the trainer or the gate. A change to those is a person's pull request, measured with the same script:
a feature change bumps `FEATURES_VERSION` and refits from stored logs with no new games (D3).

## Setting it up

The runner: the local runner set up on the PC as `bot/machine/README.md` (The local runner) says,
from an image built with `gh` (`docker build -t jackioh-pc-runner:latest bot/machine/local` again
after a change to its Dockerfile), and its supervisor serving `learn.yml` (`config.json`). The PC
should not sleep while it is plugged in, or a shard in the middle of its work is lost each time it
does. Without the runner no run starts at all: its jobs wait 24 hours and fail.

The stats database: a person does this once, for step 7: create a login on production's database that
may act as `service_role` (Supabase's SQL editor: `create role stats_importer login noinherit password
'…'; grant service_role to stats_importer;`), and save its session-pooler connection string
(`postgresql://stats_importer.<project-ref>:<password>@aws-0-<region>.pooler.supabase.com:5432/postgres`,
the pooler because GitHub's runners and many home connections reach no IPv6-only direct host) as the
repository secret `STATS_IMPORT_DATABASE_URL`. Set on 2026-10-26 (#1976). `gather.yml` imports with
the same secret.

The token: a person does this once: create a fine-grained token that can write contents and pull requests on
this repository alone, and save it as the repository secret `LEARN_LANE_TOKEN` (Settings → Secrets
and variables → Actions). A pull request opened with a run's own token would start no CI. Without
the secret, or once it has expired, a passing run fails in red at its deliver step and stops at its
artifact (`promotion.patch`), which a person can push. A fine-grained token expires on the date it was
given: renew it before then. The shadow-ban sweep that follows each promotion
(`.github/workflows/sweep.yml`, R1678000) opens its pull request with the same token.

To run it now: `gh workflow run learn.yml` (`-f games=… -f shards=… -f screen=800`). `-f
export_run=<run id>` plays no export and retrains on that earlier run's, kept 30 days: a change to
the fit or the search is measured on the same games without 40 hours of self-play. The gate's deals are the candidate tree's (R850001): a changed tree plays new ones. To
stop it, disable the workflow (`gh workflow disable learn.yml`); to refuse a promotion, close its
pull request.
