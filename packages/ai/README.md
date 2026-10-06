# @jackioh/ai

The computer opponent of practice games (SPEC §9.9). It plays one seat of an ordinary engine game:
it reads a state, returns one `ActionBody`, and the caller reduces it. The design and every
signature below are in `docs/polish/3-ai.md`; this file is the contract other packages rely on.

## Purity

`src/` is pure and seeded, exactly like `packages/engine` and `packages/cards` (CLAUDE.md rule 4),
and ESLint enforces it: no `Math.random`, no `Date`, no timers, `performance`, `process`, `fetch`,
`crypto` or `node:*` imports, and no async code. Every random draw comes from an `Rng` the caller
passes in. The clock only ever arrives as a callback (`AiOptions.shouldStop`, `sweepCard`'s `now`).
Nothing in `src/` recurses.

`src/` never imports `@jackioh/cards`. Callers register the catalog and the card scripts first:
`scripts/` and the web worker call `registerAll()` from `@jackioh/cards`, and so does the `ai` test
project, once, in its setup file (`test/setup.ts`).

Every AI number lives in `src/config.ts` (CLAUDE.md rule 9). The deck-building, match, gate and
sweep numbers live in their own modules' constants (`AI_DECK`, `AI_MATCH`, `AI_GATE`, `AI_SWEEP`).

## `decide` reads only `redact`

`decide(state, seat, options)` is the one entry point. It reads the true `GameState` only through
`aiToAct` and `redact` (R185). `redact` blanks everything the seat may not know: the opponent's hand
and library, the backrow cards it cannot read, the cards in its own library that came from the
opponent's deck, the order of its own library, the seed and the event history. Every simulation runs
on a `determinize`d copy, whose hidden cards are resampled from the non-token cards of every set
(Core, Classic and Classic+, R185, R380) that the opponent has not shown, never from
`AI_DETERMINIZE.excludeDefIds` (#98 Heroic Power, whose rolled power lives in its memory, R43), and
whose seed is the AI's own. A hidden backrow card is sampled from the Traps and Field Traps alone, minus
any whose live face-down aura (R403) would change a unit's shown stats (R602), so a world never holds a
Siphon Squad the board rules out. A face-down card on top of a backrow zone shows its cost (R351), so it
is sampled from the traps of that cost while the opponent has an unseen one left, and from the trap pool
as before once it has none (R762). `greedyAction` passes `{ matchShownCost: false }` and keeps the sampler
the quality gates were fixed on. So two states that differ only in hidden cards give the same decision
under the same rng, and no simulation can foresee a real draw or a real coin flip.

The AI never concedes and never offers a draw, and it declines every draw offer at once (R188).

The mulligan is its own step (R265): both seats owe one at once, so `aiToAct(state, seat)` is true as
soon as the mulligans open, whether or not the human has answered, and `decide` answers it at once
with `mulliganKeep`. The human's answer is sealed until both are in, and `redact` leaves the AI the
same state whatever it kept: that it has answered, and nothing else (R266). A harness that plays
both seats asks the engine's `seatToAct(state)` which one the game waits on first.

The difficulty tiers change the AI seat's resources only (`AI_DIFFICULTY` in the engine's
`config.ts`, R180). Nothing in this package reads a difficulty or a handicap to decide what to do.
The tutorial's opponent (SPEC §9.10) is the same AI on one more handicap, `AI_TUTORIAL` (R290): a
12-card deck, a mana cap of 3 and a hero that starts at 20 health, the tier below Easy. Its deck is
the lesson's fixed list rather than `buildAiDeck`'s draw (R291), and its budget is `AI_BUDGET`, as
at every tier. `test/tutorial-tier.test.ts` plays the greedy baseline against it.

## Search and the opponent's reply

`decide` runs a lethal solver, then a turn-level beam search on one determinization. The solver is
a bounded search for a line that wins this turn on every determinization. It walks depth-first in
move order for `AI_SEARCH.lethalQuickNodes` nodes, which finds the usual lethal (a few swings at
the face) at once; if that walk neither found one nor searched the whole tree, it spends the rest
of `lethalNodes` best-first, always expanding the position closest to lethal (`readyGap`: the enemy
hero's health less what the attacks still to come deal past its Taunts) and trying the first
`AI_SEARCH.lethalWidth` moves of each position it expands. So a lethal that starts with a card late
in move order, such as a Lava Golem tributing both enemy Taunts, is found as long as that card is
among the first `lethalWidth` moves of its position. A move past them is never tried, and an X-cost
spell's values, targets and modes can put hundreds of moves ahead of it. The beam's best lines are
scored one turn deeper, after the opponent's reply (`src/reply.ts`): on the
determinization the opponent plays any card the line itself put in its hand (a Pocket Chaos handed
over, units a Flood bounced), otherwise trades or swings at the face by static trade value, and
ends its turn; the line is scored at the start of the AI's next turn. The opponent never plays a
card it held unseen, because those are samples. The best first actions are scored the same way on
the other determinizations, and the best mean is played. Only that first action is played; the AI
re-plans after it.

The turn cap comes from the engine's `TURN_CAP_PLAYER_TURNS` (60 player-turns since patch v0.2.0,
R389), which is what the AI reads when it weighs face damage more as the cap nears, so the longer
cap needs no change here.

## Budgets

A decision's cost is counted in nodes: one node is one `reduce` call the AI makes, in any
determinization, the reply's included. `AI_BUDGET` is what the browser plays with, and
`AI_GATE_BUDGET`, the budget of the quality gates and the sweep, is the same budget, so that both
measure the AI that ships. Because budgets count nodes rather than time, the same state, seed and
budget always give the same `Decision`. The browser adds a wall-clock safety cap through
`shouldStop`, which ends the search early and answers with the best line found. The timing gate
(`test/gate-perf.test.ts`) holds one decision at `AI_BUDGET` under `AI_GATE.maxDecisionMs`.

## Decks and the shadow ban

`buildAiDeck(rng, size, options)` deals `size` distinct non-token ids from every set, Core, Classic
and Classic+ alike (R184, R380), by weighted sampling without replacement (`AI_DECK`): a mana curve
that shifts toward expensive cards as the seat's `manaCap` rises, a floor on units, an optional tag
theme, and a penalty for cards the seat could never cast. It leaves out `SHADOW_BAN_IDS` unless
`banned` says otherwise (`banned: []` for a human's random deck). A tutorial lesson's fixed decks
(R291) are not drawn and did not change.

`src/shadowBan.ts` (R186) lists the cards the AI never deals to itself, each with a reason that
starts with the sweep flags that put it there, and `SHADOW_WATCH` beside it, the cards the last sweep
found at risk and cleared (below). Both are decided by the sweep, never by hand:

```
pnpm ai:sweep                          # every non-token card of every set (268); prints rows, writes nothing
pnpm ai:sweep core-011 classic-020     # only these ids
```

**Pass 1.** For each card, `sweepCard` plays `AI_SWEEP.seedsPerCard` games at each tier in
`AI_SWEEP.tiers` (Easy and Hard) of an AI whose deck includes the card against the greedy baseline,
on seeds `sweep:<tier>:<id>:<n>`, and measures:

- `error`: a throw, a refused AI action or a fallback decision.
- `timeout`: a decision slower than `AI_SWEEP.decisionMs`, or a game still running at
  `AI_SWEEP.maxActions`.
- `neverPlayed`: the card sat affordable in hand on at least `AI_SWEEP.minAffordableTurns` turns and
  was never played.
- `selfHarm`: over at least `AI_SWEEP.minHarmPlays` plays, its plays lowered the AI's own
  evaluation by more than `AI_SWEEP.selfHarmDelta` on average.

**Pass 2** (R390, patch v0.2.0). The AI is slow to ban a card and deals the cards on track to be
banned far more often before it does:

- **At risk.** A card is at risk when pass 1's numbers meet a flag's condition at half strength —
  affordable in hand on `minAffordableTurns` turns and played at most once, or an average evaluation
  change below half of `selfHarmDelta` — or when it is banned already or listed in `SHADOW_WATCH`.
  The at-risk list is a pure function of pass 1's results.
- **More games, more deals.** Pass 2 sweeps only the at-risk cards, `AI_SWEEP.seedsPerCardAtRisk`
  (24) games each per tier, on named seeds `sweep2:<tier>:<id>:<n>`, so slices still run in parallel
  and a sweep of record replays. In every pass-2 game the AI's filler draw multiplies each at-risk
  card's weight by `AI_SWEEP.atRiskBoost` (4), the mechanism `buildAiDeck`'s `themeBoost` uses, so an
  at-risk card is dealt as the forced card of its own games and as filler in everyone else's. A
  card's numbers add up over every pass-2 game it was dealt in, forced or not: its filler games are
  its own evidence.
- **A ban needs pass-2 evidence.** `neverPlayed` needs 6 affordable turns (pass 1 reads 3) and no
  play in any pass-2 game at that tier; `selfHarm` needs 8 plays (pass 1 reads 4). Pass 1 alone never
  bans for either. `error` and `timeout` ban as they always have: they are bugs, not judgement.
- **Filler and blame.** Pass 2's filler draw lifts the ban for at-risk cards banned for
  `neverPlayed` or `selfHarm`, the judgements pass 2 exists to revisit, and keeps out cards banned for
  `error` or `timeout`, so a known bug is never filler. An `error` or `timeout` still bans only the
  game's forced card; one in a game that also dealt at-risk filler is listed against that filler too,
  as a `suspect` line in the sweep's output, and bans the filler only if its own forced games repeat
  it.
- **Memory between sweeps.** `SHADOW_WATCH` holds the cards that were at risk and cleared, with their
  numbers, and the next sweep counts them at risk from the start, so a card on track to be banned
  stays watched from one sweep to the next.

The ban's scope is unchanged (R186): it governs AI deck building and nothing else. The sweep never
rewards the AI's search for playing an at-risk card, because that would measure a different AI from
the one that plays.

Copy the printed entries into `SHADOW_BAN` and `SHADOW_WATCH` and the printed header line into the
file's header. A later engine, card or AI change can make the ban stale, so rerun the sweep after
one; a patch that adds cards or changes pools (patch v0.2.0 did both) needs a sweep of record over
every card. `timeout` is the one wall-clock flag: sweep a card it flags again, alone, before banning
it, because parallel sweeps on a busy machine slow every decision down. The unbanned pool must keep
at least `AI_DECK.minPool` cards, so that a 30-card Hard deck can always be built.

## Matches and the quality gates

`playMatch(config, hooks)` plays one whole game between two controllers (`ai`, `greedy` or `random`)
and returns its log, its hash and its bookkeeping: refused actions, throws, fallbacks, decisions and
nodes. It is deterministic: controllers draw from `${seed}:ctl:<seat>` and nonces are `m<n>`, so
`fold({ seed, decks, handicaps, log })` reproduces `record.hash`. `playAiTurn` plays one AI turn out
(nonces `t<n>`), which is what the puzzles use.

The baselines are `randomAction`, SPEC §10.7's random policy, and `greedyAction`, which looks one
action ahead on one determinization.

The gates (`runGate(matchup, seeds)`, `AI_GATE`) are three matchups whose subject alternates seats,
with every game folded back to its hash. Only wins count, in every gate; turn-cap draws are
reported beside them (`GateReport.turnCapDraws`). A run of n games needs `gateNeeded(matchup, n)`
wins: the brief's share of n (`AI_GATE.briefRate`), or fewer where an AI exactly as strong as the
one measured on fresh deals (`AI_GATE.measuredRate`) would miss that more often than
`AI_GATE.falseAlarm` allows. The rule is proposed in SPEC §9.9, pending the user's acceptance.

| Matchup | Subject | Opponent | Brief | Measured on fresh deals | Full run | Smoke run |
|---|---|---|---|---|---|---|
| `ai-vs-random` | the AI on Easy | random policy on Easy | 95% | 94.5% ± 0.7 | 91 of 100 | 17 of 20 |
| `ai-vs-greedy` | the AI on Easy | greedy baseline on Easy | 70% | 68.0% ± 1.5 | 28 of 50 | 10 of 20 |
| `hard-vs-easy` | the AI on Hard | the AI on Easy | 80% | 91.3% ± 1.6 | 40 of 50 | 16 of 20 |

A change to what the decks can hold changes what the gates' frozen seeds deal. Patch v0.2.0 opened
the pools to every set (R184, R185) and doubled the turn cap (R389), so its gates are re-run over the
three sets (BUILD M9) and the counts they record replace this table's here and in SPEC §9.9, as the
sweep of record replaces `shadowBan.ts`; `measuredRate` is measured again on fresh deals from the new
pools, under the rule below.

An AI at the measured rates fails any one of these runs by chance at most once in twenty. The price is
that a gate this size sees only a broken AI: the full greedy run fails with 90% probability only
once the win rate is down to 46%, and a 5-point loss fails it one time in eight (SPEC §9.9 has the
rest). So the gates are not how to tell whether a change made the AI stronger or weaker: play the
change and the AI as it stands on the same fresh deals with `duel.ts`, hundreds of them, and
compare them game by game.

`gate-perf.test.ts` is the fourth gate file: every decision of the first ai-vs-greedy gate game (six
under `pnpm ai:gate`), decided again at `AI_BUDGET`, must stay within the node budget and take under
`AI_GATE.maxDecisionMs` (the fastest of up to `AI_GATE.perfRepeats` runs counts, and the runs stop at
the first one under it). So must one decision on each of two hand-built wide boards (five units a
side, a hand of X-cost and targeted spells, at Hard's and at Easy's mana), because ordinary games
seldom reach the worst case.

```
pnpm vitest run --project ai    # every AI test, gates at AI_GATE.smokeSeeds per matchup
pnpm ai:gate                    # the three gate files at their full seed counts (CI job ai-gate)
```

Every gate run writes its wins and turn-cap draws to stdout, and a failing one also names the seeds
the subject did not win. `gameConfig(matchup, n)` rebuilds game `n`
exactly, so a tuner can replay it with `playMatch`. Tuning changes the weights in `src/config.ts`,
never the floors, the seed counts, `briefRate` or `falseAlarm`. `measuredRate` may be raised by a
new measurement on fresh deals; lowering it lowers every gate and needs the user's sign-off, recorded
in SPEC §9.9. If the full gate outgrows its CI job, shrink `AI_GATE_BUDGET` (it is
the browser's `AI_BUDGET` today) and say so, since the gates then measure a smaller search than the
one that ships.

Four tuning and diagnostic aids live in `scripts/` and write no file:
`bench.ts <matchup> <from> <to> [gate|full]` plays a range of gate games and prints one JSON line each (win, result, turns, nodes, the slowest
decision, whether the log replays), and `trace.ts <matchup> <n> [gate|full]` prints one game turn by
turn. Both take `OVERRIDE_<CONFIG>='{…}'` to try weights without editing `src/config.ts`, and
`BAN=id,id` to try a different shadow ban. `duel.ts <matchup> <from> <to>` plays the same games
with the weights set per seat (`SUBJECT_AI_SEARCH='{…}'`, `OPPONENT_AI_SEARCH='{…}'`, and so on),
so that a change can play the AI as it stands (`OPPONENT=ai`, and `SWAP_DECKS=1` for the same deals
with the decks swapped), and it prints each game's final hash and why it ended, so that two runs over
the same deals can be compared game by game. A game with no result (the action ceiling, or a
controller that threw) is `aborted`, never a draw. `oracle.ts <matchup> <from> <to> [fromTurn]`
replays the games the AI did not win and runs the lethal solver on the true state, which `decide`
may never read, at each of the AI's late turn starts, to tell a missed kill from a board with no
kill on it; its header says what that search cannot see.

## Development runs for the card statistics (R378)

`pnpm ai:stats` (`scripts/stats.ts`) is the pre-release half of SPEC §9.11: AI-against-AI games played
on the build in the checkout, each filed as a game record (R376) of source `dev`, under the patch the
run tests. `devGameConfig(n, { series })` deals game n as All Random deals a live game (R258,
`${seed}:p1-deck` and `${seed}:p2-deck`, nothing banned) to two AI seats on SPEC's resources (no
handicap) at `AI_BUDGET`, and `devGameRecord(n, options)` plays it and reads its record off the log
with the engine's `summarizeGame`; a game with no result is no record. Seeds are
`${series}:${n}`, the series `AI_DEV_RUN.series` unless the run names another, and no gate or tuning
run plays them. A record's id is `dev:<patch>:<seed>` (`devRecordId`), so the next patch's run on the
same seeds files new records, which `stats:import` adds rather than skipping. The shadow ban does not apply, since All Random bans nothing, so a banned card's
figures are the figures of a card the AI is known to misplay.

```
pnpm ai:stats                                      AI_DEV_RUN.games games, tagged with the newest patch
pnpm ai:stats --games=50 --from=51                 games 51–100, so slices of a run go in parallel
pnpm ai:stats --patch=v0.2.5 --out=v0.2.5-dev.jsonl   a pre-release run of v0.2.5, kept in a file
```

It prints the run's card win rates when it ends (progress goes to stderr). A game takes seconds at
the browser's budget, so a run of a few hundred takes the better part of an hour on one core. The
file, written a line per game as the run goes, is what `pnpm --filter @jackioh/server stats:import`
loads, so that `stats:cards --source=dev --patch=<version>` reads the run and
`stats:cards --patch=<version>` that patch's live games, two queries to compare
(apps/server/README.md, "Card statistics"). This is the one script here that writes a file, and only
the one `--out` names.
