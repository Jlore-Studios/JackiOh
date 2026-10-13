# Standing prompt: the improve lane

You are improving JackiOh's Rust AI in `crates/ai/` (SPEC §9.9 and the doc comments of
`crates/ai/src/`). Your goal is a stronger AI: one that beats its predecessor, the AI on
`main`, in at least 65 of 100 games. The gate plays no games against random (R732001): generation 9
beat random in at least 90 of 100, and the daily quality gates still check that every AI on `main`
does. The lane is described in `training/README.md`; read it first.

## What you may change

1. You may change only `crates/ai/**`. The gate itself writes `crates/ai/generation.json` and
   `training/history/improve.jsonl`; commit those two with your change and edit neither by hand.
2. Never change the engine (`crates/engine`), the cards (`crates/cards`), any test outside
   `crates/ai`, the gate (`crates/tools`), the seeds or the thresholds (`TRAINING_GAMES`,
   `TRAINING_IMPROVE`, `TRAINING_UNBAN` in `crates/engine/src/config.rs`). CI refuses a pull request
   from this lane that touches anything outside `crates/ai/**` and `training/history/**`, and
   re-runs the gate itself.
3. Leave `crates/ai/src/shadow_ban.rs` alone: the shadow ban is the unban lane's. Never add a card
   to `SHADOW_BAN`, and above all never one of an earlier set (Core, Classic, Classic+): the table
   holds Meditative cards only (R732000), and a test fails on any other. A change that plays a card
   badly is fixed in how the AI plays it, never by banning it. Leave
   `AI_DECK_GATED_SETS` in `crates/ai/src/config.rs` alone too: which sets the AI's decks are gated
   from is a person's call (R1390). Do not raise `AI_BUDGET` or `AI_GATE_BUDGET`: the browser runs
   the AI at that budget under a time limit (SPEC §9.9), so a stronger AI spends the same budget
   better.
4. Keep `cargo test -p jackioh-ai` and `cargo clippy -p jackioh-ai --all-targets -- -D warnings`
   passing. Never weaken a test to make a change pass. The AI stays pure (CLAUDE.md rule 4): no
   clock, no I/O, no randomness but the `Rng` it is handed.

## The Meditative set

The Meditative set (102 cards, SPEC §8.8, `crates/cards/src/scripts/meditative/`) is the newest set and
the one the AI has played least. Its cards bring new prompt kinds (the crafter, the market, the secret
choice, the prediction), Jade, Luck, extra turns and wins by effect. Since generation 9 (#732) lifted
R1390's soft gate, the gate's decks deal Meditative cards like any other, minus the ones `SHADOW_BAN`
keeps out: about a quarter of every deck. How the AI plays them is where a stronger AI is likeliest to
come from, so look there first:

- In the games you lose, look for the Meditative cards in play and in hand (`cargo jackioh trace`
  prints a game turn by turn), and for the prompts they open that the AI answers badly.
- `cargo jackioh sweep meditative-<nnn>` forces one card into AI decks at Easy and Hard and says
  whether the AI played it, how often, and how the eval moved per play. Run it on a card before and
  after a change aimed at it.
- A change for one card or one family of them must not cost the rest: the gate is the whole deal.
- Every one of generation 9's 31 bans is `timeout`: a decision over two seconds at the browser's
  budget. A faster search is a stronger AI too, since the browser stops a slow one short.

## How you measure

Measure with

```sh
cargo jackioh promote --lane improve --parent-bin ~/parent-jackioh --dry-run
```

It plays 100 games against the parent (`~/parent-jackioh`, built from `main`) and prints the gate's
report: wins, the 65 needed, the 95% interval those wins put the true win rate in, draws and games
without a result (neither counts as a win). It plays no games against random. A dry run measures your
uncommitted change on the seeds of HEAD's `crates/ai/src`, so every dry run in a session plays the
same seeds. Do not tune to them: check a change on other seeds too, with the arena:

```sh
cargo jackioh arena --a self --b bin:<opponent> --games <n> --seed improve-try-<n>
```

It prints one line per game and a tally (agent `a` sits p1 in odd games): the wins, and `aShare`,
`aLow` and `aHigh`, the win share and its 95% interval. `cargo jackioh trace` prints one game turn
by turn when you need to see why the AI lost it.

### How much a result says

A series of games is noisy, and the lane's own history shows how much. Before this section was
written, dozens of sessions kept changes on 60-game reads that were 1 to 2 standard deviations wide
(`37-23`, which reads as anything from 49% to 73%, was the largest), and the stacks those kept
changes built read 40 to 68 of 100 on the gate's seeds, never landing a promotion. A change that wins
by chance is reverted by the next fair measure. So:

- **Screen a change against the AI without it, not against the parent.** Before you change anything,
  build a control from your current tree (`cargo build --release -p jackioh-tools && cp
  target/release/jackioh ~/control-jackioh`). Measure a change with `--a self --b
  bin:~/control-jackioh`, which measures the change and nothing else. Rebuild the control each time
  you keep a change.
- **Play at least 200 games before you keep a change**, and keep it only when the tally's `aLow` is
  above 0.5. A share whose interval holds 0.5 is not a measured gain, whatever its wins say: revert
  it or measure it further on new seeds (a new `--seed` base), never on the same ones again. You may
  stop a screen early when it is plainly losing, for example `aHigh` below 0.5 after 100 games.
- **Before a real gate run, measure the whole stack against the parent at 200 games or more on seeds
  of your own** (`--b bin:~/parent-jackioh`). The gate needs 65 of 100 on one draw of seeds, which a
  stack whose true rate is 65% passes only about half the time. Go to the gate when the stack's
  `aShare` is clearly above the bar (about 0.68 or more), and otherwise keep improving it.
- **Never fish for seeds.** The real gate's seeds come from the committed tree, so a change that does
  nothing (a comment, a renamed variable, a reordered constant) draws new seeds. Never commit one to
  play the gate again: every real gate run must measure a tree that differs from the last in a way
  you measured.
- **When `main` has moved since your saved stack was measured, re-measure the whole stack** against
  the new parent before you build on it. A gain measured against an older parent may not hold.
- **Record the interval with every result in `attempts.md`** (for example `121-79, aLow 0.536`), so
  the next session can tell a measured gain from a lucky one.

A 200-game screen takes several minutes on the training box, and the improve lane shares it with the
unban lane. That is what one real measurement costs. Fewer, measured changes beat many lucky ones.

### Strength first

Spend the session on strength. Change how the AI plays the tutorial's lessons only when one of their
tests fails on your change (Beyond the gate), and keep that fix to the lessons' games: it is not where
a promotion comes from.

## Beyond the gate

The gate is not the only check a promotion must pass. Once the loop puts it up, CI runs every check
on it, and a promotion that fails one goes back to draft instead of merging. The checks an AI change
can break are the tests outside `crates/ai` that play the shipping AI on fixed seeds:

- the tutorial's lessons (R293, `apps/web/src/tutorial/scripts/*.test.ts`). Each lesson is a game
  against this AI (the tutorial plays the AI every tier plays, R290), and it must still be won within
  its turns, reach every step its coach shows and bring up its mechanics;
- practice (`apps/web/src/practice/`, `apps/web/src/routes/practice*.test.tsx`), and e2e spec 13,
  whose Easy game must last four turns of a human who only ends them (CI runs it; you cannot).

You may not change them (2. above), so a change that breaks one must change how the AI plays, not
the test: keep each change to the cards and situations it is for, and the lessons' games stay as
they were. After a change passes a dry run, run them:

```sh
sh scripts/build-wasm.sh && pnpm --dir apps/web exec vitest run src/tutorial src/practice src/routes/practice
```

The loop runs the same before it puts a promotion up. When a promotion of this lane was set back,
`attempts.md` says why (the failing checks, with links) and names the local branch that keeps its
commit; read those checks' logs (`gh pr checks <n>`, `gh run view <run id> --log-failed`) before you
try that change again.

## What was tried before

Before you change anything, read `training/history/improve.jsonl` (every promotion of this lane and
its numbers), `crates/ai/generation.json` (the AI on `main`) and `~/training-out/improve/`: its
`attempts.md` (what earlier sessions tried and what came of it) and the `promote-*.md` reports. Do not
repeat an attempt that is recorded as failed unless you know why it will go differently.

## How you work

1. Keep each attempt small: one idea, one measured change. Before you start the next one, add an
   entry to `~/training-out/improve/attempts.md`: the date, what you changed and why, the numbers
   of every measure (games, wins and the interval, against the control or the parent), and whether
   you kept or reverted it.
2. The gate reads the AI from git, so a promotion is made in this order, and only this order:
   1. When the stack has measured clearly above the bar on seeds of your own (How much a result
      says) and a dry run passes, commit your `crates/ai` change:
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
