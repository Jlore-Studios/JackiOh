# Standing prompt: the unban lane

You are improving JackiOh's Rust AI in `crates/ai/` (SPEC §9.9 and the doc comments of
`crates/ai/src/`) so that it plays well with fewer shadow-banned cards. The shadow ban (R186,
`crates/ai/src/shadow_ban.rs`) lists the cards the AI never deals into its own decks, because it
played them badly. Your goal is to remove entries from it, teaching the AI to play those cards,
while the AI still beats its predecessor, the AI on `main`, in at least 75 of 100 games and SPEC
§10.7's random policy in at least 90 of 100. A promotion needs strictly fewer shadow bans than the
parent's. The lane is described in `training/README.md`; read it first.

## What you may change

1. You may change only `crates/ai/**`. The gate itself writes `crates/ai/generation.json` and
   `training/history/unban.jsonl`; commit those two with your change and edit neither by hand.
2. Never change the engine (`crates/engine`), the cards (`crates/cards`), any test outside
   `crates/ai`, the gate (`crates/tools`), the seeds or the thresholds (`TRAINING_GAMES`,
   `TRAINING_IMPROVE`, `TRAINING_UNBAN` in `crates/engine/src/config.rs`). CI refuses a pull request
   from this lane that touches anything outside `crates/ai/**` and `training/history/**`, and
   re-runs the gate itself.
3. Only remove entries from `SHADOW_BAN`; never add one, and keep the table sorted by id. When you
   remove one, say in the comment above the table which generation removed it and why the AI now
   plays it. `SHADOW_WATCH` is the sweep's (R390): leave it. Do not raise `AI_BUDGET` or
   `AI_GATE_BUDGET`: the browser runs the AI at that budget under a time limit (SPEC §9.9).
4. Keep `cargo test -p jackioh-ai` and `cargo clippy -p jackioh-ai --all-targets -- -D warnings`
   passing. Never weaken a test to make a change pass; a test that pins the ban list itself is
   updated to the new list. The AI stays pure (CLAUDE.md rule 4): no clock, no I/O, no randomness but
   the `Rng` it is handed.

## How you measure

Measure with

```sh
cargo jackioh promote --lane unban --parent-bin ~/parent-jackioh --dry-run
```

It plays 100 games against random and 100 against the parent (`~/parent-jackioh`, built from
`main`; its seats keep the parent's own shadow ban, yours keep yours) and prints the gate's report:
wins against each, the 90 and 75 needed, both ban counts, and draws and games without a result
(neither counts as a win). A dry run measures your uncommitted change on the seeds of HEAD's
`crates/ai/src`, so every dry run in a session plays the same seeds. Do not tune to them: check a
change on other seeds too, for example

```sh
cargo jackioh arena --a self --b bin:~/parent-jackioh --games 40 --seed unban-try-<n>
```

which prints one line per game and a tally (agent `a` sits p1 in odd games). To see how the AI plays
one card, `cargo jackioh sweep <card id>` runs R186's sweep on it, and `cargo jackioh trace` prints one
game turn by turn.

## What was tried before

Before you change anything, read `training/history/unban.jsonl` (every promotion of this lane and its
numbers), `crates/ai/generation.json` (the AI on `main`) and `~/training-out/unban/`: its
`attempts.md` (which cards earlier sessions tried to unban, what they changed, and what came of it)
and the `promote-*.md` reports. Do not repeat an attempt that is recorded as failed unless you know
why it will go differently.

## How you work

1. Keep each attempt small: one card (or one family of cards) and the change that teaches the AI to
   play it. Before you start the next one, add an entry to `~/training-out/unban/attempts.md`: the
   date, the card ids, what you changed and why, the dry run's numbers (vs random, vs parent, bans),
   and whether you kept or reverted it.
2. The gate reads the AI from git, so a promotion is made in this order, and only this order:
   1. When a dry run passes, commit your `crates/ai` change:
      `git add crates/ai && git commit -m "AI gen <N> (unban): <what changed>"`, where `<N>` is
      `crates/ai/generation.json`'s `generation` plus one and `<what changed>` is one line naming the
      cards unbanned.
   2. Run the gate for real: `cargo jackioh promote --lane unban --parent-bin ~/parent-jackioh`.
      Your committed AI has a new source tree, so this plays new seeds.
   3. If it exits 0, it has written `crates/ai/generation.json` and `training/history/unban.jsonl`:
      amend them into the commit with
      `git add crates/ai/generation.json training/history/unban.jsonl && git commit --amend --no-edit`.
      That is the promotion commit; nothing else may be committed after it.
   4. If it exits 1, undo the commit and keep the change (`git reset --soft HEAD~1`), record the
      result in `attempts.md`, and go on.
3. Commit nothing else: no commit that a non-dry-run `promote` has not passed survives the session.
   Do not push, open a pull request or touch other branches: the loop (`training/loop.sh`) does that
   after the session, once it has checked the promotion against the parent again.
4. Stop the session after a promotion, or after 4 hours, whichever comes first. Before you stop
   without a promotion, make sure `attempts.md` says what you tried and what you would try next.
