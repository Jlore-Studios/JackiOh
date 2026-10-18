# The learn lane

The learned evaluation's lane (#466, #1012; `docs/learned-eval.md`). It keeps the AI's learned
evaluation (`crates/ai/src/learned.rs`) trained on the AI's own games and promotes a better one to
`main` through the same kind of gate every generation passes. Its decisions are a script's, not an
LLM's (D2), so this file is its contract rather than a prompt, as `gather.md` is the gather lane's.

It is the other half of the AI. The `improve` and `unban` lanes are **supervised**: a Devin session
reads the AI, changes the hand-written evaluation or the search, and measures each change
(`improve.md`, `unban.md`). The `learn` lane is **learned**: a network fitted to the outcomes of
games corrects the hand eval where the search finally scores a line. The hand eval stays the
network's input (D4's skip connection) and the beam's pruning, so the supervised lanes keep improving
the AI the network corrects, and the network is retrained on their work.

## How it runs

`.github/workflows/learn.yml`, weekly and by dispatch, on GitHub's runners (free for this public
repository) and, for the export, the local runner if asked (`bot/machine/README.md`, The local
runner). Each run:

1. **Waits** while the lane's pull request (`ai/learn`) is open: a promotion is on its way to `main`.
2. **Exports** `games` games (100,000 by default) through `selfplay.yml` on a fresh base,
   `learned-<catalog>-<date>-<run>`, never a gate's deals or the dev set's (R1006002).
3. **Retrains** with `training/ml/retrain.sh` on `main`: the positions (R1006003, R1006004), the fit
   and its report (`training/ml/README.md`), the integer network that ships (D7, R1011000) into
   `crates/ai/src/learned/weights.json` and the switch, `AI_EVAL_KIND`, on. It stops when the network does not beat the calibrated hand eval
   on held-out turns 4 to 10.
4. **Screens** the candidate against the AI on `main` on the improve lane's dev set: 800 paired games
   (`arena --set improve-dev --games 800`), a count fixed before it is played, never extended or played
   again to read better (`improve.md`, How you measure). It goes on only when `edgeLow` is above zero.
5. **Gates** once: `promote --lane learn`, the improve lane's bar (`TRAINING_LEARN`, 220 of 400 paired
   games) on the lane's own hold-out deals, `learn:<tree>:<d>` (R1012000, R850001).
6. **Delivers** a pass as `AI gen <N> (learn): the learned evaluation, retrained on <games> games`: the
   weights, the switch, `crates/ai/generation.json` and a line of `training/history/learn.jsonl`, pushed
   to `ai/learn` with auto-merge on. CI's training gate plays the gate again (`promote --verify`) and
   every required check runs before it merges. When `main`'s AI moved during the run, it delivers
   nothing and the next run retrains on the new `main`.

Each run keeps its reports as the artifact `learn-<run>` (the fit's report, the screen's tally, the
gate's report, the weights) for 90 days, and its summary says how far it got.

## What it may change

Only `crates/ai/src/learned/weights.json`, the switch in `crates/ai/src/config.rs`,
`crates/ai/generation.json` and `training/history/learn.jsonl`, as every lane is held to `crates/ai/**`
and `training/history/**` (CI's training gate). It never changes the exporter, the features, the
trainer or the gate. A change to those is a person's pull request, measured with the same script:
a feature change bumps `FEATURES_VERSION` and refits from stored logs with no new games (D3).

## Setting it up

A person does this once: create a fine-grained token that can write contents and pull requests on
this repository alone, and save it as the repository secret `LEARN_LANE_TOKEN` (Settings → Secrets
and variables → Actions). A pull request opened with a run's own token would start no CI. Without
the secret a passing run stops at its artifact (`promotion.patch`), which a person can push.

To run it now: `gh workflow run learn.yml` (`-f games=… -f runner=jackioh-selfplay-local -f
screen=800`). To stop it, disable the workflow (`gh workflow disable learn.yml`); to refuse a
promotion, close its pull request.
