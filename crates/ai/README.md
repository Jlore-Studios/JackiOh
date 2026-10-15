# `jackioh-ai`

The computer opponent of practice games (spec §9.9). It plays one seat of an ordinary engine game:
it reads a state, returns one `ActionBody`, and the caller reduces it. It also plays the AI team of a
Duos game, both seats, on the server (§13.7, [Duos](#duos-137) below). The design is
`docs/polish/3-ai.md`; this file is the contract the WASM module, the tools and the training lanes
rely on.

## Purity

Pure and seeded like the engine (CLAUDE.md rule 4): its `Cargo.toml` depends on the engine, `serde`,
`serde_json` and `indexmap` only, and its `clippy.toml` is the engine's (no clock, no I/O, no
environment, no threads, no `HashMap`, no `RefCell`). Every random draw comes from an `Rng` the
caller passes in. The clock arrives only as a callback (`AiOptions::should_stop`, `SweepOptions`'s
`now`); a decision the sweep's clock found slow goes out the same way, to `SweepOptions`'s `retime`
(R977000), which `cargo jackioh sweep` answers against the perf gate's yardstick. A node counter keeps its tallies in `Cell`, which the lint allows.

`src/` never depends on `jackioh-cards`. Callers call `jackioh_cards::register_all()` first: the WASM
module, the `jackioh` tools, and this crate's tests (dev-dependency).

Every AI number is a named constant (CLAUDE.md rule 9): `AI_BUDGET`, `AI_GATE_BUDGET`, `AI_SEARCH`,
`AI_DETERMINIZE`, `AI_MULLIGAN` and `AI_MULLIGAN_CURVE` in `src/config.rs`, and the deck, gate, sweep
and development-run numbers in their modules (`AI_DECK` in `deck.rs`, `AI_GATE` in `gate.rs`,
`AI_SWEEP` in `sweep.rs`, `AI_DEV_RUN` in `dev_run.rs`), with the Duos gate's `DUOS_GATE_GAMES_SMOKE` and
`DUOS_GATE_WIN_FLOOR` in `src/config.rs`. The tiers are the engine's (`AI_DIFFICULTY`, `AI_TUTORIAL`
and Duos's `duos_ai_team` in `crates/engine/src/config.rs`).

## `decide` reads only `redact`

```rust
pub fn decide(state: &GameState, seat: PlayerId, options: &mut AiOptions) -> Option<Decision>;
pub fn decide_styled(state: &GameState, seat: PlayerId, options: &mut AiOptions, style: &AiStyle) -> Option<Decision>;
pub struct AiStyle { pub temperature: f64, pub missed_lethal_chance: f64 }
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
answers its mulligan as soon as the mulligans open (R265, R266): `mulligan_keep_curved` sends back
every card costing more than `AI_MULLIGAN.keep_max_cost` except the one proven four-drop the shaped
keep holds, and keeps the cheap ones to `AI_MULLIGAN_CURVE` (#993): at most two one-drops, and only
the cheapest card when none is an early play, counted in the seat's crystals (its mana bonus and,
going second, The Coin). Nothing here reads a difficulty, and a handicap only as the seat's mana (the
mulligan's bonus and cap): the tiers change the AI seat's resources only (R180), and the tutorial's
opponent is the same AI on `AI_TUTORIAL` with the lesson's fixed deck (R290, R291).

`decide` runs a bounded lethal solver (`lethal.rs`), then a turn-level beam search, scoring the best
lines one turn deeper after the opponent's reply (`reply.rs`) on every determinization, and plays the
first action of the best mean. It re-plans after every action.

`decide_styled` is `decide` under an `AiStyle` (R1452), the server's ladder bots' strength knob beside
their budget: a `missed_lethal_chance` to pass over the lethal line the solver found (the search then
keeps off every line that wins this turn, `lines_missing_lethal`, or it would find the same lethal),
and a softmax
`temperature`, in evaluation points, over the finalists' means (`pick_finalist`) instead of always the
best. Each takes one draw from `options.rng` when it is above zero and none at zero, so `decide`,
which is `decide_styled` at `AI_STYLE_EXACT`, draws only for its determinizations, as it always has.

## Budgets

A decision's cost is counted in nodes: one node is one `reduce` call the AI makes, the reply's
included. `AI_BUDGET` is what the browser plays with; `AI_GATE_BUDGET`, the gates' and the sweep's,
is the same budget, so both measure the AI that ships. The same state, seed and budget always give the
same `Decision`. The browser's wall-clock cap arrives as `should_stop` (the WASM binding's deadline),
which ends the search early with the best line found.

## Decks and the shadow ban (R184, R186, R390, R1370, R1390, R817000, R976000)

`build_ai_deck(rng, size, &AiDeckOptions)` deals `size` distinct non-token ids from every set by
weighted sampling without replacement (`AI_DECK`), leaving out `SHADOW_BAN_IDS` unless `banned` says
otherwise. `lean_set` (R1370) names a set at least `AI_DECK.lean_min_share` of the deck comes from, a
hard floor kept together with the units' and the theme's; absent, the draw is exactly the one it always
was. `build_ai_deck_traced` also counts the picks that floor forced, the measurement `lean_boost` was
chosen by. Every deck holds at most `AI_DECK.max_quickdraw` (2) Quickdraw cards, an `include`d one
counted and always dealt, and Quickdraw is never a theme (R817000). `gated_sets` (R1390) names the sets the draw never deals from: its default is the soft gate
`AI_DECK_GATED_SETS` in `src/config.rs`, so every AI deck holds none of them whatever ships or a
thread previews, and a lean on a gated set leans on the newest ungated set that ships (`ungated_lean`).
A player's random deck passes `Some(vec![])` beside `banned: Some(vec![])`; an `include` still forces a
gated card in by name. Emptying the list lifts the gate and deals every deck as before it, which issue
#732 did once it had measured the AI on the Meditative set, the list's one set until then: the list is
empty, and gating a set that ships before the AI is trained on it is listing it there. `shape`
(R976000) multiplies each card's weight by `shaped_weight`, the unban lane's dealt-quality table in
`src/shadow_ban.rs`; it is off unless a caller sets it, whatever `banned` holds, and no caller does, so
practice, the quality gates and both seats of an arena or a promotion deal unshaped. The mulligan still
reads `shaped_weight` (`mulligan_keep_shaped`).
`src/shadow_ban.rs` holds `SHADOW_BAN` (each card with the sweep flags that banned it) and
`SHADOW_WATCH` (cards found at risk and cleared). Both are decided by the sweep, never by hand: run
`cargo jackioh sweep` and copy the printed rows and header line into the file. The ban governs AI deck
building and nothing else: a banned card stays legal for every player. Since #732 the ban holds
Meditative cards only, at most half of the set (R732000): no sweep, lane or person adds a card of an
earlier set, and the unban lane (`training/unban.md`) only removes entries.

## Matches and the quality gates

`play_match(&MatchConfig, &mut MatchHooks) -> MatchRecord` plays one whole game between two
controllers (the AI, `greedy_action` or `random_action`, §10.7's random policy) and returns its log,
hash and bookkeeping; `play_ai_turn` plays one AI turn out. The gates (`run_gate_games`, `AI_GATE`) are
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

A seat the AI plays has a skill (R747000): `SeatController::Ai { budget, skill }`, the chance in
`[0, 1]` that each decision follows the AI, the rest being a random legal action (`random_action`,
§10.7's policy). `None` is `AI_SKILL_MAX`: full skill, which takes no draw, so a seat without one
plays as it did before the knob; `AI_SKILL_MIN` is the random policy exactly, with no search. In
between, `skill::plays_best(skill, rng)` rolls the seat's own seeded stream once per decision
(`skill::clamp_skill` pins the range, a NaN reading as full), so a seeded run reproduces. The knob
knows nothing of cards or sets: it keeps meaning the same when the AI improves or a set ships. The
arena (`crates/tools/README.md`) and the server's ladder-bot seat (`crates/server/README.md`) roll it
the same way, and the skill-to-Elo curve is measured with the arena.

## Development runs for the card statistics (R378)

`dev_game_config` and `dev_game_record` deal and play game n of a development run as All Random
deals a live game (R258), both seats AI on the spec's resources at `AI_BUDGET`, filed as a game
record of source `dev` under the patch the run tests, id `dev:<patch>:<seed>` (`dev_record_id`).
`cargo jackioh stats --out <file>` writes a run; `jackioh-server stats-import <file>` loads it.

## Duos (§13.7)

`src/duos.rs` plays a seat of a Duos game (`jackioh_engine::duos`): one AI plays both seats of team B,
and any human seat the server hands it after a disconnect (§13.8).

```rust
pub fn duos_decide(state: &DuosState, seat: DuosSeat, options: &mut AiOptions) -> Option<DuosActionBody>;
pub fn duos_passes(state: &DuosState, seat: DuosSeat) -> Vec<String>;
pub fn duos_gate(games: usize, seed: &str) -> DuosGateReport;   // { games, wins, draws }
```

- `duos_decide` is `decide` on the seat's own duel: its mulligan and its prompts as a 1v1 seat answers
  them, and its own turn by the same search. When `decide` would end the turn, it first sends the
  passes `duos_passes` names, one action at a time, then the `endTurn`. `None` when the seat owes
  nothing. The same state and options give the same action.
- `duos_passes` names the hand cards to pass now, best first: a card the seat cannot play this turn
  and its teammate can (a round seat that has not ended, can afford it and has a zone for it), when
  the seat can pay the pass; with `pass_cost` 0 also a card the teammate can afford next turn and the
  seat cannot; never more than the passes left or the teammate's free hand slots.
- It knows what its team knows and nothing more: its own duel through `decide`'s `redact` and
  `determinize` (R185), and its teammate's own state, which a team shares (§13.6). An opponent's
  hidden card never changes a decision.
- A seat's life is the engine's `effective_life` (own HP and the team's pool while unspent, §13.4)
  everywhere the AI reads one: the evaluation, `ready_gap`, the reply's `reply_face_value` and the My
  Pawn surcharge. The hero value and `closing` are anchored at `starting_life` (`DUOS_OWN_HEALTH` plus
  the pool's cap). A 1v1 seat reads its hero's health and `HERO_HEALTH`, as before (R964000).
- The tiers are `duos_ai_team(difficulty)` for the team (Easy the humans' rules, Medium free passing,
  Hard free passing and a `DUOS_POOL_HEALTH_HARD` pool) and `AI_DIFFICULTY` for each AI seat's
  handicap, which the caller sets in `DuosCreateOptions`; nothing here reads a difficulty. The AI
  never concedes (R188), so it neither offers nor confirms a team's concede.
- `duos_gate` plays the Easy AI team against a team on §10.7's random policy; it passes at
  `DUOS_GATE_WIN_FLOOR` (0.8) of `DUOS_GATE_GAMES_SMOKE` (20) games (`cargo jackioh gate --duos`).

## Commands

```
cargo test -p jackioh-ai                      # every AI test (the unit tests, tests/ai.rs, ai_slow_a.rs, ai_slow_b.rs)
cargo test -p jackioh-ai --test ai_slow_b     # one binary: CI gives each its own job (#873)
cargo jackioh gate                            # the three gates and the perf gate at smoke size
cargo jackioh gate --full                     # at full size (the daily super run plays it in shards)
cargo jackioh gate --duos                     # the Duos gate: the Easy AI team against a random team (--games N)
cargo jackioh gate --full --shard 1/4 --out ai-gate-shards && cargo jackioh gate merge ai-gate-shards
cargo jackioh trace ai-vs-greedy 3            # one gate game, turn by turn
cargo jackioh sweep core-011 classic-020      # the shadow-ban sweep over these ids (no ids: every card)
cargo jackioh stats --games 50 --from 51 --out dev.jsonl   # a slice of a development run
```
