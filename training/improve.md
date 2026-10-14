# Standing prompt: the improve lane

You are improving JackiOh's Rust AI in `crates/ai/` (SPEC §9.9 and the doc comments of
`crates/ai/src/`). Your goal is a stronger AI: one that beats its predecessor, the AI on `main`, in
at least 220 of 400 games on the gate's 200 hold-out deals, each deal played twice with the seats
swapped (R850000). The gate plays no games against random (R732001): generation 9 beat random in at
least 90 of 100, and the daily quality gates still check that every AI on `main` does. The lane is
described in `training/README.md`; read it first, and its section The improve stack above all.

## What you may change

1. You may change only `crates/ai/**`. The gate itself writes `crates/ai/generation.json` and
   `training/history/improve.jsonl`; commit those two as the promotion commit (How you work) and
   edit neither by hand.
2. Never change the engine (`crates/engine`), the cards (`crates/cards`), any test outside
   `crates/ai`, the gate (`crates/tools`), the seeds, the seed sets or the thresholds
   (`TRAINING_IMPROVE`, `TRAINING_UNBAN` in `crates/engine/src/config.rs`). CI refuses a pull request
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

## The stack

Your session starts from the lane's stack, the branch `ai/improve-stack`, which the loop rebased onto
`main` before it started you: the changes earlier sessions measured and kept, one commit each (`git
log --oneline origin/main..HEAD`; each message carries its screen's numbers). Build on it. When the
session ends, the loop saves your commits as the stack, so a gain too small for the gate is kept for
the next session instead of lost (`training/README.md`, The improve stack).

- Commit each change you keep, once its screen has kept it (How you measure), as one commit:
  `git add crates/ai && git commit -m "Stack (improve): <what changed>" -m "<games> games on <the
  set>, <wins>-<losses>, edge <edge> (<edgeLow> to <edgeHigh>) against <the control's commit>"`.
- Never commit an unmeasured change, a change its screen did not keep, the record
  (`crates/ai/generation.json`, `training/history/`) or anything outside `crates/ai`: the loop
  refuses to save such commits as the stack. Uncommitted work is not saved.
- To drop a kept change that a later measure shows does not hold, `git revert` its commit, with
  that measure in the message. Never rewrite the stack's history, and never start a stack commit's
  subject with `AI gen `, which is the promotion's.
- When `attempts.md` says `main`'s AI moved under the stack, the loop screened the rebased stack
  against the new parent before it started you, and its note holds the tally. Read it before you
  build on the stack, and fix or revert what no longer holds.
- When `attempts.md` says the stack does not rebase onto `main`, your session started from `main`
  and the stack's commits are the local branch the note names (`ai/improve-stack-conflict-<t>`).
  Rebuild the stack first: `git cherry-pick` each of `git log --reverse --format=%h
  origin/main..<that branch>`, in order, resolving each conflict in `crates/ai` the way the kept
  change meant it. Leave out a commit whose change `main` now holds or that no longer makes sense,
  and say so in `attempts.md`. Then screen the rebuilt stack against the parent (Before the gate)
  before you build on it. Your commits replace the stack when the session ends; until then it stays
  on GitHub as it was.
- Do not push, and never touch `ai/improve-stack` or any other branch yourself: the loop does.

## The first job: one structural change

Thirty-six sessions (14 to 49 in `attempts.md`, 2026-10-09 to 10-14) spent most of their hours
sweeping the eval's and the search's knobs, and the stacks they kept read about even with their
parent on fair measures. These are ruled out; do not try them again unless you know why they will go
differently:

- weight and knob sweeps, of the hand eval and of how the search spends its budget (s35, s41): the
  eval's edge and the search's were each about 8 points alone and did not stack, and a sweep's best
  arm, picked on the seeds that measured it, read about 10 points lower on the gate's;
- a linear evaluator fitted to self-play positions (s15): a logistic model on 19,918 positions
  matched the hand eval (AUC 0.749) and added nothing;
- a terminal rollout vote, a second-ply vote and more determinizations (s14): noise or worse.

The losses the notes trace again and again are midgame snowballs: an engine, a unit that grows or
makes value every turn, priced at its body today and left unanswered. So make the session's first
job one change of a kind those notes have not ruled out, screened as below, before any knob:

- a search that sees past one opponent turn within the same budget, for example information-set
  Monte Carlo tree search (ISMCTS) over the worlds the AI already determinizes, with today's eval at
  its leaves; or
- an evaluator that prices what a linear one cannot (engines, snowballing, terms that interact),
  once a pipeline that records positions as well as results is on `main`. Today's records hold only
  results, and the pipeline (`crates/tools`, outside this lane) is a follow-up of issue #850.

Put it behind a switch, a named constant in `crates/ai/src/config.rs`, so one line turns it off. It
must hold the browser's time limit: do not raise `AI_BUDGET` or `AI_GATE_BUDGET`, and every duel
line's `msP95` (the 95th percentile of the AI's decision time, in milliseconds) stays under 2000, the
sweep's `timeout` threshold. A change too large for one session goes on the stack in measured steps,
each screened and kept on its own.

## How you measure

Two fixed sets of deals measure a change, each deal played twice with the seats swapped: the
candidate on p1 in a deal's first game and on p2 in its second, with the same decks and shuffles.
That cancels most of the deal's luck, the first seat's edge and the god-hand alike, since a deal
that favours one seat gives each AI one game of it (R850000, R850001).

- **The dev set** is for screens: deals `improve-dev:1` to `:200`, the same in every session,
  played by `cargo jackioh arena … --set improve-dev`.
- **The hold-out set** is the gate's own deals, `improve:<tree>:1` to `:200`, named for the
  committed AI's source tree. Only the gate reads it, once per stack tip. The arena refuses a seed
  base that names it, and the gate's dry run refuses uncommitted changes in `crates/ai/src`.

### A screen

1. Before you change anything, build a control from the committed tip, the AI without your change:
   `cargo build --release -p jackioh-tools && cp target/release/jackioh ~/control-jackioh`. Rebuild
   it each time you commit a change to the stack.
2. Make one change and screen it against the control, which measures the change and nothing else:

   ```sh
   cargo jackioh arena --a self --b bin:~/control-jackioh --set improve-dev
   ```

   It plays 400 games, 25 to 45 minutes on the box, which the improve lane shares with the unban
   lane. Each game prints a duel line; the tally that closes them gives the wins and `aShare` with
   its interval (`aLow`, `aHigh`), and the paired result: `deals`, and `edge`, the change's wins
   less the control's as a share of the games, with its 95% interval `edgeLow` to `edgeHigh`.
3. Keep the change only when `edgeLow` is above 0, and then commit it to the stack (The stack).
   Otherwise revert it. You may stop a screen early when it is plainly losing, never keep one early.
4. Decide a screen's size before you start it. 400 games is the floor; a change you expect to gain
   only a point or two needs more to show it (`--games 800` plays deals 1 to 400). Never extend a
   screen because it came close, never screen the same change again to get a better read, and never
   keep the best of several arms of a knob because it read best: each arm you keep needs its own
   `edgeLow` above 0, and the stack's screen against the parent (below) says whether the picks hold.
5. Record every screen in `attempts.md` with its interval, kept or reverted, for example
   `400 games on improve-dev, 214-186, edge +0.070 (+0.012 to +0.128) vs control 1a2b3c4: kept`.

`cargo jackioh trace` prints one game turn by turn when you need to see why the AI lost it; trace
the dev set's games, never the gate's.

### Before the gate

Once the stack holds changes kept since the last promotion, screen the whole stack against the
parent on the dev set:

```sh
cargo jackioh arena --a self --b bin:~/parent-jackioh --set improve-dev
```

Go to the gate only when its `edgeLow` is above 0. Read that edge as optimistic: every change on the
stack was kept on these deals, so the stack reads better on them than it will on deals it has never
seen. The gate's hold-out deals are that fair read.

### How much a result says

The lane's history shows how much a series can mislead. Sessions kept changes on 60-game reads 1 to
2 standard deviations wide (`37-23` reads as anything from 49% to 73%), swept knobs and kept the best
arm on the seeds that measured it, and built stacks that read 40 to 68 of 100 on the gate's seeds and
about even (95-105, 101-99) when their pieces were re-screened at 200 games against a control.
Fresh-seed reads ran about 10 points above the gate's in every session from s40 to s48. A change
that wins by chance is reverted by the next fair measure. So:

- the gate needs 220 of 400 on its hold-out deals: an equal successor passes it at most about 2.6%
  of the time, a stack that truly wins 55% about half the time, one at 58% nine times in ten;
- 400 paired games put a change's edge within about ±0.1 of its true value when a deal's two games
  fall independently, and closer when the deal's luck cancels, so a screen keeps a gain of a few
  points only when pairing has narrowed the interval, or on a larger screen decided in advance;
- **never fish for seeds.** The gate's deals come from the committed tree, so a change that does
  nothing (a comment, a renamed variable, a reordered constant) draws new ones. Never commit one to
  play the gate again: every gate run must measure a tree that differs from the last in a way you
  measured.

Fewer, measured changes beat many lucky ones, and every kept change stays on the stack for the next
session to build on.

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
they were. Before you commit a kept change to the stack, run them, so the stack never holds a change
that breaks a lesson unnoticed:

```sh
sh scripts/build-wasm.sh && pnpm --dir apps/web exec vitest run src/tutorial src/practice src/routes/practice
```

The loop runs the same before it puts a promotion up. When a promotion of this lane was set back,
`attempts.md` says why (the failing checks, with links) and names the local branch that keeps its
commit; read those checks' logs (`gh pr checks <n>`, `gh run view <run id> --log-failed`) before you
try that change again.

## What was tried before

Before you change anything, read `training/history/improve.jsonl` (every promotion of this lane and
its numbers), `crates/ai/generation.json` (the AI on `main`), the stack (`git log origin/main..HEAD`,
each kept change with its screen) and `~/training-out/improve/`: its `attempts.md` (what earlier
sessions tried and what came of it, and the loop's notes on the stack) and the `promote-*.md`
reports. Do not repeat an attempt that is recorded as failed unless you know why it will go
differently.

## How you work

1. Keep each attempt small: one idea, one measured change. Before you start the next one, add an
   entry to `~/training-out/improve/attempts.md`: the date, what you changed and why, the numbers
   of every measure (games, wins, the edge and its interval, against the control or the parent), and
   whether you kept it (with its stack commit) or reverted it.
2. The gate reads the AI from git, so a promotion is made in this order, and only this order:
   1. When the stack's screen against the parent has its `edgeLow` above 0 (Before the gate), every
      kept change is committed and nothing in `crates/ai/src` is left uncommitted, run the gate once:
      `cargo jackioh promote --lane improve --parent-bin ~/parent-jackioh`. It plays the committed
      stack's hold-out deals, 400 games, as long as a screen. A dry run (`--dry-run`) plays the very
      same games and is no second chance, so run the gate for real.
   2. If it exits 0, it has written `crates/ai/generation.json` and
      `training/history/improve.jsonl`: commit those two alone, on top of the stack,
      `git add crates/ai/generation.json training/history/improve.jsonl && git commit -m "AI gen <N> (improve): <what changed>"`,
      where `<N>` is the `generation` it wrote and `<what changed>` sums up the stack in one line.
      That is the promotion commit; nothing else may be committed after it. The loop folds the stack
      into it and checks it against the parent again before it goes up.
   3. If it exits 1, it wrote nothing. Record its numbers in `attempts.md` (the wins and the 220
      needed, the paired edge and its interval) and go on improving the stack. Never run it again on
      the same tip: it would read the same games.
3. Commit nothing but kept changes to the stack and, after a passing gate, the promotion commit.
   Do not push, open a pull request or touch other branches: the loop (`training/loop.sh`) does that
   after the session, once it has checked the promotion against the parent again.
4. Stop the session after a promotion, or after 4 hours, whichever comes first. A screen and the
   gate each take 25 to 45 minutes, so start neither when it cannot finish inside the 4 hours: a run
   the session's time limit cuts off measures nothing. Before you stop without a promotion, make sure
   every kept change is committed to the stack and `attempts.md` says what you tried and what you
   would try next.
