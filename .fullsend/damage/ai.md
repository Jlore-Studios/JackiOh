# Damage: `crates/ai/**` (part 31, ai reconciler)

## How it was measured

The real build cannot reach `jackioh-ai`: `cargo check -p jackioh-ai` stops in `jackioh-engine`
(185 errors at `9edbee2`, the first build; 44 at `83a4255`, after the engine reconciler's commits).
So every AI count below is from a **shadow build**: a scratch copy of the workspace (outside the
repository, never committed) in which every function body of `jackioh-engine` and `jackioh-cards`
is replaced by `loop {}` (signatures, types, consts and the testkit kept), the card scripts are left
out, and seven names the engine has not settled yet (`RunHookOptions`, `MoveOptions`,
`PlaceOptions`, `MovePosition`, `MoveToZonePosition`, `AddToHandResult`,
`CardsInCardScopeOptions`, none of which the AI names) get placeholder types. That checks the AI
crate against the engine's **signatures as they stand on `staging`**, which is all an AI error can
depend on. The tests were first checked against a likewise stubbed AI library (the library's own
errors stop the test binary otherwise).

## Before (first build, `9edbee2`)

Real build: 0 AI errors reachable; `jackioh-engine (lib)` 185 errors.

Shadow build, `jackioh-ai` lib: **21** errors.

| File | Errors |
|---|---|
| `crates/ai/src/decide.rs` | 13 |
| `crates/ai/src/lethal.rs` | 4 |
| `crates/ai/src/match_.rs` | 2 |
| `crates/ai/src/search.rs` | 1 |
| `crates/ai/src/baselines.rs` | 1 |

By code: E0308 14, E0277 6, E0599 1. Every one is the `NodeCounter`/`SearchBudget` conflict below
(`i32` node counts against `types.rs`'s `usize`, and `sim_errors()` on `&dyn NodeCounter`). Type
checking stops borrow checking, so this is a floor; after the fix borrowck found nothing more.

Shadow build, `tests/ai.rs` (test binary): **41** errors.

| File | Errors |
|---|---|
| `crates/ai/tests/ai/surface.rs` | 8 |
| `crates/ai/tests/ai/observe.rs` | 8 |
| `crates/ai/tests/ai/shadow_ban.rs` | 5 |
| `crates/ai/tests/ai/reply.rs` | 4 |
| `crates/ai/tests/ai/lethal.rs` | 4 |
| `crates/ai/tests/ai/match_refusal.rs` | 3 |
| `crates/ai/tests/ai/dev_run.rs` | 3 |
| `crates/ai/tests/ai/puzzles.rs` | 2 |
| `crates/ai/tests/ai/determinize_shown_cost.rs` | 2 |
| `crates/ai/tests/ai/search.rs` | 1 |
| `crates/ai/tests/ai/prompts_v020.rs` | 1 |

By code: E0061 19, E0308 18, E0609 3, E0432 1.

## After (`83a4255`, staging pulled)

Real build: 0 AI errors reachable; `jackioh-engine (lib)` 44 errors (the engine reconciler's).

Shadow build: `jackioh-ai` lib **0** errors, 0 warnings; `tests/ai.rs` **0** errors, 1 warning
(`support.rs:29`: the `Scenario`/`ScenarioOptions` re-export nobody imports; a `-D warnings` item
for Wave 3). Outside the scope, against the reconciled AI crate: `jackioh-wasm` 0 errors;
`jackioh-tools` 1 (`crates/tools/src/gate.rs:590`, `GREEDY_PROBE_ACTIONS: i32` into
`MatchConfig.max_actions: Option<usize>`, which this part did not change).

What is left for the AI crate is whatever the engine's own reconciliation changes in signatures
the AI calls (the shadow build passes against today's), plus the engine's bodies at test time.

## Collisions

- **`NodeCounter`** (the counter shape, one concept written twice): chunk 1's `&mut dyn
  NodeCounter` with a trait `sim_errors()` and `i32` counts, in `decide.rs`, `lethal.rs`,
  `reply.rs`, `search.rs`, `baselines.rs`; chunk 2's trait with `&self` methods, `usize` counts and
  `sim_error_tally()`, in `types.rs` and `simulate.rs`. Winner `types.rs`/`simulate.rs`.
- **`SearchBudget` field types**: chunk 1 assumed `i32` (`decide.rs`, `search.rs`, `lethal.rs`'s
  `limit`), `types.rs` has `usize`. Winner `types.rs`.
- **Test helper shapes**: `tests/ai/support.rs` (owner) against 17.3's guesses (`act(.., Option<&str>)`,
  `run_puzzle(.., Option<SearchBudget>)`, `random_decks(seed, None)`,
  `random_policy_states(.., Option<max>)`). Winner `support.rs`.
- **`dev_game_config`**: `dev_run.rs` (owner) `(n, series, budget)` against 17.2's test
  `(n, &DevRunOptions)` with a `series_only` helper. Winner `dev_run.rs`; the helper deleted.
- Not collisions, left standing: `match_::message_of` and `simulate::panic_message` (TS also had
  the expression twice: `match.ts`'s `messageOf` and `simulate.ts` inline); per-file test helpers
  (`ai_options` ×6, `det` ×5, `js` ×6, `spread` ×4, `eval` ×4, `expect_folds`, `worlds`, `asked` ×2,
  …) that port per-file TS idioms; tests are not edited beyond their calls.

## Seams

- `jackioh_ai::find_lethal_with_quick_nodes` (tests/ai/lethal.rs): did not exist; added to
  `lethal.rs` (17.2 GAPS).
- `MatchHooks.override_choice` (tests/ai/match_refusal.rs): did not exist; added to `match_.rs`
  (17.2 GAPS).
- `determinize(.., &DeterminizeOptions)` in tests against the owner's by-value parameter.
- `beam_search(.., &SearchBudget)` in `surface.rs` against the owner's by-value parameter.
- `sweep_card`/`sweep_at_risk(.., SweepOptions)` in tests against the owner's `&SweepOptions`.
- `CountingNodeCounter::sim_errors` called on `&dyn NodeCounter` in `decide.rs`.
- Outside the scope (for their reconcilers): `crates/server/src/actor/engine.rs:187`
  `banned: vec![]` where `AiDeckOptions.banned` is `Option<Vec<String>>` (SURFACE §4.3,
  part 19.2's guess); `crates/tools/src/gate.rs:590` above.

## Drift

None. `crates/ai/Cargo.toml` names only serde, serde_json, indexmap and the engine (SURFACE §3);
no persona code in the crate (R645).

## Semantic conflicts

`cat .fullsend/notes/part-17-{1,2,3}.assumptions | sort -u | cut -d: -f1 | uniq -c | awk '$1>1'`:

- `ai.counter`: 17.1 "trait (used, limit, take, stopped_by, sim_errors); every search fn takes
  `&mut dyn NodeCounter`"; 17.3 "trait …; `&mut dyn NodeCounter` (tests pass `&mut counter`)";
  17.2's notes (owner): `&self` methods, `&dyn NodeCounter`. Settled: `&dyn`, `types.rs`.
- `ai.weights` (17.1 only) "every AI count i32" against 17.2's notes "budgets and node counts
  `usize`". Settled: node budgets and counts `usize` (`types.rs`), the AI's config counts
  (`AI_SEARCH`, `AI_REPLY`, `AI_MULLIGAN`) stay `i32` and are cast where they meet a node count.
- `error.style`, `state.mut`, `rng.source`, `hook.style`, `registry.style`: the same value with a
  parenthetical gloss; no conflict.
