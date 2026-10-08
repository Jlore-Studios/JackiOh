# `jackioh-ai`

The computer opponent of practice games (spec §9.9). It plays one seat of an ordinary engine game:
it reads a state, returns one `ActionBody`, and the caller reduces it. The design is
`docs/polish/3-ai.md`; this file is the contract the WASM module, the tools and the training lanes
rely on.

## Purity

Pure and seeded like the engine (CLAUDE.md rule 4): its `Cargo.toml` depends on the engine, `serde`,
`serde_json` and `indexmap` only, and its `clippy.toml` is the engine's (no clock, no I/O, no
environment, no threads, no `HashMap`, no `RefCell`). Every random draw comes from an `Rng` the
caller passes in. The clock arrives only as a callback (`AiOptions::should_stop`, `SweepOptions`'s
`now`). A node counter keeps its tallies in `Cell`, which the lint allows.

`src/` never depends on `jackioh-cards`. Callers call `jackioh_cards::register_all()` first: the WASM
module, the `jackioh` tools, and this crate's tests (dev-dependency).

Every AI number is a named constant (CLAUDE.md rule 9): `AI_BUDGET`, `AI_GATE_BUDGET`, `AI_SEARCH`
and `AI_DETERMINIZE` in `src/config.rs`, and the deck, gate, sweep and development-run numbers in
their modules (`AI_DECK` in `deck.rs`, `AI_GATE` in `gate.rs`, `AI_SWEEP` in `sweep.rs`,
`AI_DEV_RUN` in `dev_run.rs`). The tiers are the engine's (`AI_DIFFICULTY`, `AI_TUTORIAL` in
`crates/engine/src/config.rs`).

## `decide` reads only `redact`

```rust
pub fn decide(state: &GameState, seat: PlayerId, options: &mut AiOptions) -> Option<Decision>;
pub struct AiOptions<'a> { pub rng: Rng, pub budget: SearchBudget, pub should_stop: Option<&'a dyn Fn() -> bool> }
```

`decide` is the one entry point. It reads the true `GameState` only through `ai_to_act` and `redact`
(R185): `redact` blanks what the seat may not know (the opponent's hand and library, the backrow
cards it cannot read, its own library's order and the cards in it that came from the opponent, the
seed, the event history). Every simulation runs on a `determinize`d copy whose hidden cards are
resampled from the non-token cards of every set the opponent has not shown (R185, R380), a hidden
backrow card from the traps alone, minus any whose live aura would change a shown stat (R602), and
of the shown cost while an unseen trap of that cost is left (R351, R762). So two states that differ
only in hidden cards give the same decision under the same rng. The caller reads `options.rng.cursor()`
back afterwards.

The AI never concedes and never offers a draw, and it declines every draw offer at once (R188). It
answers its mulligan as soon as the mulligans open (R265, R266). Nothing here reads a difficulty or a
handicap: the tiers change the AI seat's resources only (R180), and the tutorial's opponent is the
same AI on `AI_TUTORIAL` with the lesson's fixed deck (R290, R291).

`decide` runs a bounded lethal solver (`lethal.rs`), then a turn-level beam search, scoring the best
lines one turn deeper after the opponent's reply (`reply.rs`) on every determinization, and plays the
first action of the best mean. It re-plans after every action.

## Budgets

A decision's cost is counted in nodes: one node is one `reduce` call the AI makes, the reply's
included. `AI_BUDGET` is what the browser plays with; `AI_GATE_BUDGET`, the gates' and the sweep's,
is the same budget, so both measure the AI that ships. The same state, seed and budget always give the
same `Decision`. The browser's wall-clock cap arrives as `should_stop` (the WASM binding's deadline),
which ends the search early with the best line found.

## Decks and the shadow ban (R184, R186, R390, R1370)

`build_ai_deck(rng, size, &AiDeckOptions)` deals `size` distinct non-token ids from every set by
weighted sampling without replacement (`AI_DECK`), leaving out `SHADOW_BAN_IDS` unless `banned` says
otherwise. `lean_set` (R1370) names a set at least `AI_DECK.lean_min_share` of the deck comes from, a
hard floor kept together with the units' and the theme's; absent, the draw is exactly the one it always
was. `build_ai_deck_traced` also counts the picks that floor forced, the measurement `lean_boost` was
chosen by. `src/shadow_ban.rs` holds `SHADOW_BAN` (each card with the sweep flags that banned it) and
`SHADOW_WATCH` (cards found at risk and cleared). Both are decided by the sweep, never by hand: run
`cargo jackioh sweep` and copy the printed rows and header line into the file. The ban governs AI deck
building and nothing else: a banned card stays legal for every player.

## Matches and the quality gates

`play_match(&MatchConfig, &mut MatchHooks) -> MatchRecord` plays one whole game between two
controllers (the AI, `greedy_action` or `random_action`, §10.7's random policy) and returns its log,
hash and bookkeeping; `play_ai_turn` plays one AI turn out. The gates (`run_gate`, `AI_GATE`) are
three matchups whose subject alternates seats, every game folded back to its hash. A run of n games
needs `gate_needed(matchup, n)` wins (the rule spec §9.9 proposes); only wins count.

| Matchup | Subject | Opponent | Full run | Smoke run |
| --- | --- | --- | --- | --- |
| `ai-vs-random` | the AI on Easy | random policy on Easy | 91 of 100 | 17 of 20 |
| `ai-vs-greedy` | the AI on Easy | greedy baseline on Easy | 28 of 50 | 10 of 20 |
| `hard-vs-easy` | the AI on Hard | the AI on Easy | 40 of 50 | 16 of 20 |

The perf gate holds each decision of the first ai-vs-greedy game, and two hand-built wide boards,
within the node budget and under `AI_GATE.max_decision_ms`. Tuning changes the weights in
`src/config.rs`, never the floors, the seed counts or the brief's rates. The gates see only a broken
AI; whether a change made it stronger is the training lanes' question (`training/README.md`,
`cargo jackioh promote`).

## Development runs for the card statistics (R378)

`dev_game_config` and `dev_game_record` deal and play game n of a development run as All Random
deals a live game (R258), both seats AI on the spec's resources at `AI_BUDGET`, filed as a game
record of source `dev` under the patch the run tests, id `dev:<patch>:<seed>` (`dev_record_id`).
`cargo jackioh stats --out <file>` writes a run; `jackioh-server stats-import <file>` loads it.

## Commands

```
cargo test -p jackioh-ai                      # every AI test (the unit tests and tests/ai.rs)
cargo jackioh gate                            # the three gates and the perf gate at smoke size
cargo jackioh gate --full                     # at full size (the daily super run plays it in shards)
cargo jackioh gate --full --shard 1/4 --out ai-gate-shards && cargo jackioh gate merge ai-gate-shards
cargo jackioh trace ai-vs-greedy 3            # one gate game, turn by turn
cargo jackioh sweep core-011 classic-020      # the shadow-ban sweep over these ids (no ids: every card)
cargo jackioh stats --games 50 --from 51 --out dev.jsonl   # a slice of a development run
```
