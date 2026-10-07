# Standing prompt: the improve lane

You are improving JackiOh's Rust AI in `crates/ai/` (SPEC §9.9 and the doc comments of
`crates/ai/src/`). Your goal is a stronger AI: one that beats its predecessor, the AI on
`main`, in at least 85 of 100 games, and SPEC §10.7's random policy in at least 90 of 100. The lane
is described in `training/README.md`; read it first.

## What you may change

1. You may change only `crates/ai/**`. The gate itself writes `crates/ai/generation.json` and
   `training/history/improve.jsonl`; commit those two with your change and edit neither by hand.
2. Never change the engine (`crates/engine`), the cards (`crates/cards`), any test outside
   `crates/ai`, the gate (`crates/tools`), the seeds or the thresholds (`TRAINING_GAMES`,
   `TRAINING_IMPROVE`, `TRAINING_UNBAN` in `crates/engine/src/config.rs`). CI refuses a pull request
   from this lane that touches anything outside `crates/ai/**` and `training/history/**`, and
   re-runs the gate itself.
3. Leave `crates/ai/src/shadow_ban.rs` alone: the shadow ban is the unban lane's. Do not raise
   `AI_BUDGET` or `AI_GATE_BUDGET`: the browser runs the AI at that budget under a time limit
   (SPEC §9.9), so a stronger AI spends the same budget better.
4. Keep `cargo test -p jackioh-ai` and `cargo clippy -p jackioh-ai --all-targets -- -D warnings`
   passing. Never weaken a test to make a change pass. The AI stays pure (CLAUDE.md rule 4): no
   clock, no I/O, no randomness but the `Rng` it is handed.

## How you measure

Measure with

```sh
cargo jackioh promote --lane improve --parent-bin ~/parent-jackioh --dry-run
```

It plays 100 games against random and 100 against the parent (`~/parent-jackioh`, built from
`main`) and prints the gate's report: wins against each, the 90 and 85 needed, draws and games
without a result (neither counts as a win). A dry run measures your uncommitted change on the seeds
of HEAD's `crates/ai/src`, so every dry run in a session plays the same seeds. Do not tune to them:
check a change on other seeds too, for example

```sh
cargo jackioh arena --a self --b bin:~/parent-jackioh --games 40 --seed improve-try-<n>
```

which prints one line per game and a tally (agent `a` sits p1 in odd games). `cargo jackioh trace`
prints one game turn by turn when you need to see why the AI lost it.

## What was tried before

Before you change anything, read `training/history/improve.jsonl` (every promotion of this lane and
its numbers), `crates/ai/generation.json` (the AI on `main`) and `~/training-out/improve/`: its
`attempts.md` (what earlier sessions tried and what came of it) and the `promote-*.md` reports. Do not
repeat an attempt that is recorded as failed unless you know why it will go differently.

## How you work

1. Keep each attempt small: one idea, one measured change. Before you start the next one, add an
   entry to `~/training-out/improve/attempts.md`: the date, what you changed and why, the dry run's
   numbers (vs random, vs parent), and whether you kept or reverted it.
2. The gate reads the AI from git, so a promotion is made in this order, and only this order:
   1. When a dry run passes, commit your `crates/ai` change:
      `git add crates/ai && git commit -m "AI gen <N> (improve): <what changed>"`, where `<N>` is
      `crates/ai/generation.json`'s `generation` plus one and `<what changed>` is one line.
   2. Run the gate for real: `cargo jackioh promote --lane improve --parent-bin ~/parent-jackioh`.
      Your committed AI has a new source tree, so this plays new seeds.
   3. If it exits 0, it has written `crates/ai/generation.json` and
      `training/history/improve.jsonl`: amend them into the commit with
      `git add crates/ai/generation.json training/history/improve.jsonl && git commit --amend --no-edit`.
      That is the promotion commit; nothing else may be committed after it.
   4. If it exits 1, undo the commit and keep the change (`git reset --soft HEAD~1`), record the
      result in `attempts.md`, and go on improving.
3. Commit nothing else: no commit that a non-dry-run `promote` has not passed survives the session.
   Do not push, open a pull request or touch other branches: the loop (`training/loop.sh`) does that
   after the session, once it has checked the promotion against the parent again.
4. Stop the session after a promotion, or after 4 hours, whichever comes first. Before you stop
   without a promotion, make sure `attempts.md` says what you tried and what you would try next.
