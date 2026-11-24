# Measuring a change to the AI

> This was the standing prompt of the improve lane, which #4701 removed with the other training lanes
> (`training/README.md`; the prompt is in git history). What is left is how a change to the AI in
> `crates/ai/` is measured and what it must leave alone: AI issues follow it, and
> `crates/ai/src/config.rs` records each knob's screen by it (A screen).

A change to the AI makes it stronger when it wins more than the AI without it on the same deals. SPEC
§9.9 and the doc comments of `crates/ai/src/` say how the AI plays.

## What a change to the AI leaves alone

- `crates/ai/src/shadow_ban.rs`: the sweep of record keeps the shadow ban (R1678000). Never add a card
  to `SHADOW_BAN`, and above all never one of an earlier set: the table holds only the sets that may
  hold bans (R732000, R1186000), and a test fails on any other. A change that plays a card badly is
  fixed in how the AI plays it, never by banning it.
- `AI_DECK_GATED_SETS` in `crates/ai/src/config.rs`: which sets the AI's decks are gated from is a
  person's call (R1390).
- `AI_BUDGET` and `AI_GATE_BUDGET`: the browser runs the AI at that budget under a time limit (SPEC
  §9.9), so a stronger AI spends the same budget better.
- The tests: keep `cargo test -p jackioh-ai` and `cargo clippy -p jackioh-ai --all-targets -- -D
  warnings` passing, and never weaken a test to make a change pass. The AI stays pure (CLAUDE.md rule
  4): no clock, no I/O, no randomness but the `Rng` it is handed.

## How you measure

Two fixed sets of deals measure a change, each deal played twice with the seats swapped: the
candidate on p1 in a deal's first game and on p2 in its second, with the same decks and shuffles.
That cancels most of the deal's luck, the first seat's edge and the god-hand alike, since a deal
that favors one seat gives each AI one game of it (R850000, R850001).

- **The dev set** is for screens: deals `improve-dev:1` to `:200`, the same in every screen,
  played by `cargo jackioh arena … --set improve-dev`.
- **The hold-out set** is the gate's own deals, `improve:<tree>:1` to `:200`, named for the
  committed AI's source tree. Only the gate reads it, once per tree. The arena refuses a seed
  base that names it, and the gate's dry run refuses uncommitted changes in `crates/ai/src`.

A change gets one kind of screen, the deal screen or the fork screen, chosen before it runs and
recorded with it: a change still gets one screen under R850001's rule against second reads. The
deal screen plays each deal twice, the candidate against the control, and reads whole games. The
fork screen (`--forks`) measures the change at the moves it changes: the trunk plays each dev-set
deal once with the control on both seats, and wherever the candidate would have made a different
move that position is played out several times both ways, both arms on with the control (R4397000).
A change that acts in one game in ten is buried under the other nine on the deal screen and reads
cleanly on the fork screen; a change that acts only in positions the control never reaches is
invisible to the fork screen and wants the deal screen.

### A screen

1. Before you change anything, build a control from the committed tip, the AI without your change:
   `cargo build --release -p jackioh-tools && cp target/release/jackioh ~/control-jackioh`. Rebuild
   it each time you commit a change.
2. Make one change and screen it against the control, which measures the change and nothing else:

   ```sh
   cargo jackioh arena --a self --b bin:~/control-jackioh --set improve-dev
   ```

   It plays 400 games, 25 to 45 minutes on two cores. Each game prints a duel line; the tally that closes them gives the wins and `aShare` with
   its interval (`aLow`, `aHigh`), and the paired result: `deals`, and `edge`, the change's wins
   less the control's as a share of the games, with its 95% interval `edgeLow` to `edgeHigh`.
   It also gives `decided`, the deals the change actually decided split into `wonBoth` and
   `lostBoth`, and `diverged`, the deals the two agents played differently; a screen with fewer
   than `SCREEN_MIN_DECIDED` (20) decided deals prints `inconclusive: true` and says nothing
   (R4319000).
3. Keep the change only when `edgeLow` is above 0, and then commit it.
   Otherwise revert it. You may stop a screen early when it is plainly losing, never keep one early.
   A screen reading `inconclusive: true` neither keeps nor drops the change: the knob is settled
   by its scene tests, and its config comment records the screen as inconclusive with its decided
   count. On the fork screen (`--forks`, 200 trunk games) keep the change only when `forkEdgeLow`
   is above 0; it reads `inconclusive: true` below `SCREEN_MIN_DECIDED` (20) live forks, and a change
   whose gain compounds across moves reads low on it, never high (R4397000). Size one by #4448's:
   the screen of `AI_EVAL.turn_hook` played 200 trunk games and 155 forks in 3 h 7 min, and that of
   `AI_EVAL.activation` 200 and 205 in 4 h 0 min, each on a 4-core Intel Xeon at 2.30 GHz, about 12
   and 16 core-hours. Neither fits a bot's model call (150 minutes), and a fork screen prints nothing
   until its tally, so pass `--save` at a path inside your worktree: each finished deal is appended
   to the file as soon as it is done, the harness carries the file into a continued run, and after
   a cut the same command plays only the deals still missing (R4607000). The first run of #4448's
   `turn_hook` screen died 3 hours in, when the idle cloud session it ran in was reclaimed. Record
   the tally in the knob's config comment as today, and delete the save file before finishing.
4. Decide a screen's size before you start it. 400 games is the floor; a change you expect to gain
   only a point or two needs more to show it (`--games 800` plays deals 1 to 400). Never extend a
   screen because it came close, never screen the same change again to get a better read, and never
   keep the best of several arms of a knob because it read best: each arm you keep needs its own
   `edgeLow` above 0.
   `--extend` is the one exception: decided in advance by the flag, it plays on in 100-deal batches
   to `SCREEN_MIN_DECIDED` decided deals or `SCREEN_MAX_GAMES`, reading the edge only at the end;
   the stop looks at how many deals were decided, never which way, so under a neutral change each
   decided deal is a fair coin however many there are. On the fork screen `--extend` plays on in
   100-deal batches to `SCREEN_MIN_DECIDED` live forks or `SCREEN_MAX_GAMES` games, reading the
   edge only at the end.
5. Record every screen (in the knob's config comment, and in the pull request) with its interval and its decided (`wonBoth`–`lostBoth`)
   and diverged counts, kept or reverted, for example `400 games on improve-dev, 214-186, edge
   +0.070 (+0.012 to +0.128), 12 decided (8–4), 40 diverged vs control 1a2b3c4: kept`. Record a fork
   screen with its kind and its forks, live forks and edge, for example `fork screen on
   improve-dev, 34 forks, 11 live, edge +0.062 (-0.104 to +0.229) vs control 1a2b3c4: reverted`.

`cargo jackioh trace` prints one game turn by turn when you need to see why the AI lost it; trace
the dev set's games, never the gate's.

### How much a result says

The improve lane's history shows how much a series can mislead. Its sessions kept changes on 60-game
reads 1 to 2 standard deviations wide (`37-23` reads as anything from 49% to 73%), swept knobs and kept the best
arm on the seeds that measured it, and built stacks that read 40 to 68 of 100 on the gate's seeds and
about even (95-105, 101-99) when their pieces were re-screened at 200 games against a control.
Fresh-seed reads ran about 10 points above the gate's in every session from s40 to s48. A change
that wins by chance is reverted by the next fair measure. So:

- the gate's sequential test (R1679000) passes an equal successor about 5% of the time, a stack that
  truly wins 52% about 60% of the time and one at 53% about nine times in ten, in about 800 games;
- 400 paired games put a change's edge within about ±0.1 of its true value when a deal's two games
  fall independently, and closer when the deal's luck cancels, so a screen keeps a gain of a few
  points only when pairing has narrowed the interval, or on a larger screen decided in advance;
- **never fish for seeds.** The gate's deals come from the committed tree, so a change that does
  nothing (a comment, a renamed variable, a reordered constant) draws new ones. Never commit one to
  play the gate again: every gate run must measure a tree that differs from the last in a way you
  measured.

Fewer, measured changes beat many lucky ones.

### Strength first

Change how the AI plays the tutorial's lessons only when one of their tests fails on your change
(Beyond the screen), and keep that fix to the lessons' games.

## Beyond the screen

A screen is not the only check a change to the AI must pass: CI runs every check on its pull request.
The checks an AI change can break are the tests outside `crates/ai` that play the shipping AI on fixed seeds:

- the tutorial's lessons (R293, `apps/web/src/tutorial/scripts/*.test.ts`). Each lesson is a game
  against this AI (the tutorial plays the AI every tier plays, R290), and it must still be won within
  its turns, reach every step its coach shows and bring up its mechanics;
- practice (`apps/web/src/practice/`, `apps/web/src/routes/practice*.test.tsx`), and e2e spec 13,
  whose Easy game must last four turns of a human who only ends them (CI runs it; you cannot).

A change that breaks one changes how the AI plays, not the test: keep each change to the cards and
situations it is for, and the lessons' games stay as they were. Run them before you commit a kept
change:

```sh
sh scripts/build-wasm.sh && pnpm --dir apps/web exec vitest run src/tutorial src/practice src/routes/practice
```
