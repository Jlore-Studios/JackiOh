# Improve lane cycle of record, 2026-10-16 (#1058)

One cycle of the `improve` lane, run by the night bot in Devin's place (`training/README.md`, The
improve stack; `training/improve.md`). It screened one structural change, the opponent's second reply,
against the AI on `main`.

**The verdict: reverted.** 400 games on `improve-dev`, 196-204, edge −0.020 (−0.092 to +0.052) against
the parent. `edgeLow` is not above 0, and one duel line's `msP95` was 2133, over the 2000 limit, so
the change fails both of the screen's rules. Nothing was kept, the gate was not read, and the stack
stays empty. `main`'s AI does not change: this record is the cycle's only file.

## How it ran

- **Who**: the night bot (`jgoetzmann-bot`, issue #1058). The issue asks a person to stop the box's
  lane first, and the bot cannot see the box to check that. Two build runs went into it. The first one's screen started at 15:19 UTC, but the runner was lost when the run hit
  its usage limit, and its tally was never read. The second run started over at 15:56 UTC, on a new
  runner, and played the same screen: the same code, the same seeds and the same parent. A game depends
  only on its seed and its two agents (`arena`'s header), so that run's games are the same games, and
  this is the screen's one read. Only its timings could differ.
- **Machine**: a GitHub Actions runner, 4 vCPUs (`nproc` 4, Intel Xeon Platinum 8370C at 2.80 GHz),
  with nothing else running during the screen. This is not the training box, which the improve lane
  shares with the unban lane, so the times here are not the box's.
- **Base**: `main` at `f04c284`, generation 12 (`jq .generation crates/ai/generation.json`). The plan
  named `443c14e`. Between the two, `crates/ai`, `crates/engine` and `crates/cards` are the same, and
  `crates/tools` differs only in `src/fuzz.rs`, which the arena does not run. The stack
  (`ai/improve-stack`) is empty, so the parent is the control.
- **Parent** (step 1): `main`'s `crates/ai/src`, built with `cargo build --release -p jackioh-tools`
  and copied to `/tmp/parent-jackioh`. It reports itself to the arena as `bin-gen12`.
- **Candidate**: the change described next, switched on, built the same way and copied to
  `/tmp/cand-1058`.
- **The screen** (step 5), 400 games: deals `improve-dev:1` to `:200`, each played twice with the
  seats swapped, from 16:02 to 16:17 UTC. No other screen, arm or extension was run:

  ```sh
  /tmp/cand-1058 arena --a self --b bin:/tmp/parent-jackioh --set improve-dev --out /tmp/screen-1058
  ```

## The change screened

The change is `AI_SECOND_REPLY`, a switch in `crates/ai/src/config.rs`. With it on, `reply_score`
(`crates/ai/src/reply.rs`) lets the opponent reply twice. After `simulate_reply`, the seat ends its
next turn without acting, at a cost of one node, and the opponent replies again by the same rule. The
line is then scored at the start of the seat's turn after that. So a threat left on the board is priced
over two of the opponent's turns.

The second reply is skipped in four cases: the game is over, the seat is not to act, a prompt is
pending, or the board is slow. A board is slow when either side's field or backrow holds a fused
card (R77) or one of `slow_def_ids`: Pareto Optimality (`meditative-071`) or The Cane
(`meditative-071-1`). It used no extra budget, since `AI_BUDGET` and `AI_GATE_BUDGET` were not changed.
It came with two tests in `crates/ai/tests/ai/reply.rs`. The first build run's notes say the 26
reply tests passed with the switch on; the second run did not run them again before the screen.

The code is the pull request's first commit, `51f34ae1a`. It is not on `main`.

## Screens

```
400 games on improve-dev, 196-204, edge -0.020 (-0.092 to +0.052) vs parent f04c284: reverted
```

- **The tally**: `aShare` 0.490 (0.441 to 0.539). The candidate won 101 of its 200 games as p1 and 95 of
  its 200 as p2. It won both games of 25 deals, split 146 and lost both of 29. There were no draws,
  aborted games, refusals, throws or fallbacks, and every game replayed.
- **Time**: 12,901 candidate decisions, 138.5 ms mean. The median line's `msP95` was 265; the 95th
  percentile over lines, 599. Three lines had an `msP95` of 1000 or more.
- **The one line at 2000 or more**: game 307, `improve-dev:154`, with the candidate on p1: `msP95`
  2133, `msMax` 2484, a 27-turn game the candidate won. The parent's side played Pareto Optimality
  (`meditative-071`), and the candidate played Knowledge Breaker (`meditative-045`, and its Prime
  `-045-1`). It also played nine All Purpose Apples (`classicplus-059`) and the Felinors (`meditative-093`,
  `-094`).
- **The slowest decision**: game 399, `improve-dev:200`, with the candidate on p1: 16,780 ms, though
  the line's `msP95` was 491 and the candidate lost. The candidate's side played five fused cards
  (`t-3` to `t-9`), over which `slow_board` skips the second reply. The record does not say whether
  that decision ran one. The duel lines time only the candidate, so this slow decision was not
  measured against the parent on the same position.
- **Step 7** (the stack-ready check) was not run, since the change was not kept.

## The gate

Not read. improve.md (Before the gate) sends a stack to the gate only once its screen against the
parent reads `edgeLow` above 0. Nothing was kept, so there is no new tree to gate. No hold-out deals
(`improve:<tree>:…`) were played or traced, for any tree.

## The planner's pilots

These are the plan's runs, made before the build on a 4-vCPU runner. They used private seed bases,
never `improve-dev` or `improve:`, and the builder did not re-run them. Each one was 200 paired games
against a build of the base:

| Idea | Wins–losses | Edge (95% interval) | Note |
|---|---|---|---|
| Two-turn reply (the seat's own turn by the reply's rule, then a second reply) | 92–108 | −0.080 (−0.194 to +0.034) | msP95 3033 |
| Reply-score every beam line while nodes allow | 97–103 | −0.030 (−0.082 to +0.022) | |
| +2.0 per turn hook in `unit_worth` | 97–103 | −0.030 (−0.089 to +0.029) | |
| The second reply, no guard | 106–94 | +0.060 (−0.047 to +0.167) | one decision took 34.6 s over a fused board |
| The second reply, fused guard only, same deals | 110–90 | +0.100 (−0.010 to +0.210) | msP95 4711 and 4245 in two games with Pareto Optimality or The Cane in play, hence `slow_def_ids` |

The planner also found that a search decision spends on average 172 of its 600 nodes (p50 121, p90
351). Switched off, 48 games against the base read an edge of exactly 0.0.

The best pilot, +0.100 on 200 games, was the second of two arms on the same deals. With the slow
list added, the same idea read −0.020 on the dev set's 400. That is what improve.md (How much a result
says) warns of: the best of several arms, read on the deals that measured it, reads high.

## Next step

The screen reverted the change, so the next session starts from an empty stack. These are the next
ideas, best first:

1. **Drop this form of the second reply.** Both of its forms read about even or worse. Passing the
   seat's next turn prices the line too low, because in a game the seat plays that turn. Playing the
   turn by the reply's fixed rule (the first pilot) read −0.080. A deeper search needs a real policy
   for the seat's own turn, not the reply's rule.
2. **Try the search that improve.md's first job names**: an information-set Monte Carlo tree search
   (ISMCTS) over the worlds the AI already determinizes, with today's eval at its leaves. Write it
   from scratch: Devin's one ISMCTS read of −0.167 is more likely a bug than proof that the idea fails.
   Before its first screen, give it a unit test that finds a two-turn lethal the beam misses. The node
   budget has room for it: decisions use about 172 of their 600 nodes.
3. **Fix the slow tail first.** It costs strength in the browser too, where a decision is cut short.
   It also blocks any deeper search, which would make it worse. The tail is one 16.8 s decision over
   fused cards (`improve-dev:200`) and a 2.1 s p95 over Pareto Optimality and Knowledge Breaker
   (`improve-dev:154`). A trace of those two dev games (`cargo jackioh trace`) shows where the time
   goes.

## For attempts.md

```
2026-10-16 (night bot, #1058, in Devin's place; a 4-vCPU GitHub Actions runner, not the box, nothing else running; parent main f04c284, generation 12, empty stack):
- The opponent's second reply (AI_SECOND_REPLY in config.rs, on for the screen): after simulate_reply the seat ends its next turn without acting and the opponent replies again; the line is scored at the seat's turn after that; skipped over a fused card, Pareto Optimality or The Cane (slow_def_ids). No budget change.
  400 games on improve-dev, 196-204, edge -0.020 (-0.092 to +0.052) vs parent f04c284: reverted.
  Slowest line msP95 2133 (improve-dev:154, candidate p1), over the 2000 limit. One decision of 16,780 ms (improve-dev:200, candidate p1, fused cards on its side).
  Code: PR commit 51f34ae1a, not on main.
- The planner's pilots (200 paired games each against the base, private seed bases, not improve-dev): two-turn reply 92-108, edge -0.080 (-0.194 to +0.034); reply-score every beam line 97-103, -0.030 (-0.082 to +0.022); +2.0 per turn hook in unit_worth 97-103, -0.030 (-0.089 to +0.029); second reply, no guard 106-94, +0.060 (-0.047 to +0.167); second reply, fused guard 110-90, +0.100 (-0.010 to +0.210). None kept.
- No gate: nothing kept. Stack unchanged (empty).
- Next: not this second reply again (it scores a passed turn); ISMCTS over the determinized worlds within AI_BUDGET, written fresh with a two-turn-lethal unit test first (decisions spend about 172 of 600 nodes); the slow tail on improve-dev:154 and :200 first.
```

## What a person does

1. Comment this record on #597 and #1058.
2. Paste the For attempts.md block above into `~/training-out/improve/attempts.md` on the box.
3. Nothing goes to `ai/improve-stack` or `ai/improve`: the stack stays empty.
4. Restart the lane: `sudo systemctl start jackioh-train@improve`.
