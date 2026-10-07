# Damage: `crates/cards/**` (part 31, cards reconciler)

## How it was measured

**Before** (`9684288`, staging's head once the orchestrator had restored Core #37/#38): the real build
could not reach `jackioh-cards` (`jackioh-engine` lib: 44 errors). So the first count is a **shadow
build**: a scratch copy of the workspace (outside the repository, never committed) in which every
non-`const` function body of `jackioh-engine` (testkit included) is `loop {}` and its `#[cfg(test)]`
modules are emptied, signatures, types and consts kept. That checks the cards crate against the
engine's signatures as they stood on `staging`. The lib and its unit tests (`--lib --tests`) were
checked against that engine; `tests/cards.rs` (the cross tests) against the same engine plus a
cards lib whose bodies were likewise stubbed (the lib's own errors stop the test binary otherwise).
Type errors stop borrow checking body by body, so these are floors.

**After** (`a31f95d` on `0366add`): the real build. Part 32 had made the engine lib type-check, so
`cargo check -p jackioh-cards --all-targets --message-format short` ran for real.

## Before (first build, shadow)

183 errors: lib 28; lib unit tests 40 more (68 with the lib's); `tests/cards.rs` 115 (one more,
`radiant_standard.rs`'s `include_str!("../../../../docs/radiant-audit.md")`, was the shadow copy's,
which had no `docs/`; not counted).

| File | Errors |
|---|---|
| `crates/cards/tests/cross/paused_sequences.rs` | 36 |
| `crates/cards/tests/cross/query.rs` | 20 |
| `crates/cards/src/scripts/core/c085_unlicensed_experimentation.rs` | 18 |
| `crates/cards/src/scripts/core/c086_miss_mrow.rs` | 11 |
| `crates/cards/tests/cross/tributes.rs` | 11 |
| `crates/cards/tests/cross/turn_clock_and_legality.rs` | 9 |
| `crates/cards/tests/cross/fused_hooks.rs` | 6 |
| `crates/cards/tests/cross/trigger_stays.rs` | 6 |
| `crates/cards/tests/cross/turn_stages.rs` | 6 |
| `crates/cards/src/scripts/core/c083_transmogulate.rs` | 5 |
| `crates/cards/src/scripts/core/c084_going_long.rs` | 4 |
| `crates/cards/tests/cross/play_choices.rs` | 4 |
| `crates/cards/tests/cross/pools_and_randomness.rs` | 4 |
| `crates/cards/src/scripts/core/c089_corpse_eater.rs` | 3 |
| `crates/cards/tests/cross/deaths_and_reborn.rs` | 3 |
| `crates/cards/src/scripts/core/c096_my_pawn.rs`, `classic/c079_risky_die.rs`, `tests/cross/{hand_returns,re_entry,registry}.rs` | 2 each |
| 27 more files (Classic #7, #9, #33, #40, #42, #44, #46, #53, #56, #63, #78; C+ #1, #2, #8, #19.2, #38.1, #44, #45, #47, #73, T-AI-6; Core #87, #91; cross `invariants`, `plays_and_casts`, `resolving_face`, `fused_nested_resume`) | 1 each |

By code: E0599 87 (`.cloned()` on the testkit's owned `unit()`/`backrow()` answer), E0308 26, E0631 24
(`&EffectContext` where `TriggerWhen` and the effect closures take `&mut`), E0433 20 (`catalog::` as a
module path where query.rs's `catalog` is a value), E0061 8, E0515 8, E0277 4, E0560 2, E0596 2,
E0425 1, E0063 1.

## After (real build)

`cargo check -p jackioh-cards --all-targets`: **0 errors**, 1 warning
(`tests/cross/condition_active.rs:1096`, an unused `use super::*`).

What Wave 3 still has here is clippy (`cargo clippy -p jackioh-cards --all-targets`, CI's `-D
warnings`): 273 warnings, 0 errors. Top files: `tests/cross/preview.rs` 10, `classic/c046_divine_favor.rs`
10, `classic/c037_last_hurrah.rs` 10, `classic/c040_mc_tech.rs` 9, `classic_plus/c002_groom_shroom.rs` 7,
`core/c083_transmogulate.rs` 6, `tests/cross/control_change.rs` 5, `core/c085_unlicensed_experimentation.rs`
5, `core/c050_k_pop_fanatic.rs` 5, `classic_plus/c053_book_of_tokens.rs` 5, `classic/c043_plague_nuke.rs` 5,
`core/c087_pocket_chaos.rs` 4, `core/c051_kys_private_tutor.rs` 4, `core/c037_gravedigger.rs` 4,
`core/c023_reoccurring_dream.rs` 4. By lint: needless borrow (`&*x` immediately dereferenced) 93,
needless borrows for generic args 52, redundant closure 35, doc list item without indentation 29,
collapsible if/match 11, the rest under 8 each. All mechanical.

## Collisions (one concept, several places) — every one has one winner now

| Concept | Places | Winner |
|---|---|---|
| TS `stepParam(s.card(x), key, n)` on the live card | 102 `step` + 2 `step_id` + 4 `step_param_of` + `step_param_on` + 3 `step_card_param` + `step_live_param` + `step_box`/`step_flag`/`step_fungus` (Classic, Classic+, cross `preview.rs`) | engine `params::step_param` + testkit `Scenario::card_mut`, inline as TS wrote it |
| the live card in a test (`card_mut`) | 17 private `card_mut(s, id)` (13 card tests, 4 cross files) | testkit `Scenario::card_mut` |
| TS `s.card(x).radiant = true` | 9 `make_radiant`/`flag_radiant`/`set_radiant` (the `()`-returning ones; TS's own `makeRadiant` in #16/#17 stays) | `s.card_mut(x).radiant = true` |
| `_glow.ts` | Core #38 `glows`+`hand_glows`, #41 `glows`+`backrow_glows` | `testkit::glow` (cross `condition_active.rs` keeps its own: TS's condition-active.test.ts had its own too) |
| "register the shipped cards, then `scenario()`" (TS's vitest globalSetup) | 72 card-test wrappers, 9 cross wrappers, 7 cross `game` wrappers | `crate::scenario` (lib.rs, cfg(test)), `cross::scenario` (tests/cross/mod.rs) |
| an engine value as JSON | 141 `js` | `crate::js` |
| TS `toMatchObject` | 36 `matches_object` in card tests | `crate::matches_object` |
| TS `{ ...a, ...b }` | 30 `merged` | `crate::merged` |
| TS `s.unit(p, lane)?.id ?? ""` | 10 `unit_or_blank` | `crate::unit_or_blank` |
| "owned whatever the reader returns" hedge | `owned` in Core #51, C+ #44 | deleted (`zone_cards`, `audit_targets` answer owned cards) |
| naming.ts | `tools/src/patches.rs` (owner, port-map) and `tests/cross/registry.rs`'s `mod naming` | **unresolved**: the cards tests cannot reach the tools binary (below) |

Not collisions, kept: the per-file `must` (TS's 32 test files each had a `must`), cross tests' per-file
helpers that mirror TS's own (`unit_at`, `at`, `setup`, `entries`, `strings`, …), `tests/cross/params.rs`'s
shallow `matches_object` (another test binary, one copy), `crate::query`'s wrapper over
`jackioh_engine::catalog::query` (TS had both modules), Classic #28's `recalled_on` (below).

## Seams (a call to a name that does not exist, or exists with another shape) — all settled at the owner

`catalog::def_of(Option<&GameState>, …)` 7; `fused_id_parts`/`scripts_for` take the state 3;
`TriggerWhen` takes `&mut EffectContext` 9 fns; `ForEachCardArgs.cards`/`DrawWhileArgs.more`/
`CastNewDef::Read`/`KillCreditPairs` take `&mut` 9 closures + Spell Tyrant's factory; query.rs's
`pool(id, &CardQuery) -> Vec<&'static CardDef>` (Transmogulate); cross `query.rs`'s `catalog::x(` →
`catalog.x(` and `catalog.trap_types` 19; testkit `unit()`/`backrow()` answer owned copies 95;
`expect_refused*` closures return `&mut Scenario` 7; `CastRandomArgs` has `target_enemies`/`afterward`,
no `how` 2; `AuditArgs`, `SweepReader`, `CostOptions` by value 4; `roll_chaos_effects`'s `Option` table 5;
`score_def`'s TS defaults 3; `answer_prompt(&AnswerInput)` 1; `EngineSink::new` (Joro's test) 1;
movers take `&mut CardInstance` 2; `IndexSet` is not in the prelude 1; `CardRef: From<&&str>` 1;
`let mut` 2.

## Drift

- Dependencies: none. `crates/cards/Cargo.toml` is as part 1 wrote it (serde, serde_json, indexmap, the
  engine; the engine with `testkit` as a dev-dependency).
- Registry: build.rs's card list and `catalog.json` agree, 318 files ↔ 318 ids (268 cards, 50 tokens),
  no duplicate `ID`, no file without one, no catalog id without a file, every file under
  `src/scripts/{core,classic,classic_plus}`. Core #37/#38 had been deleted by part 14.2's rebase
  (`00a3357`) and were restored by the orchestrator (`9684288`). `cargo jackioh catalog check` (part 22)
  validates the same `crates/cards/catalog.json` (census per set and rarity), so the two agree.
- `tests/cross/radiant_standard.rs` compiles `docs/radiant-audit.md` in with `include_str!` from outside
  the crate: part 37 must keep that file where it is (or move it under `crates/cards/`).

## Semantic conflicts (`.assumptions` keys with more than one value in parts 9–16, 22.3, 27)

- `cards.card_def`: `&'static CardDef` vs owned `CardDef` → owned (part 1's lib.rs, the owner).
- `test.live_objects`: write through `find_instance_mut(s.state_mut(), id)` vs a live card → the
  testkit's `card_mut` (the engine reconciler's decision); 98 inline long-hand writes that are no helper
  stay as they are (equal, not a second concept).
- `test.default_params`: TS default passed explicitly vs a trailing `Option` → the owner's signature,
  TS's default written out (`None`, `Default::default()`), as the ai and engine reconcilers decided.
- `test.scenario`: `register_all()` then `scenario` — one registering `scenario()` per test binary.
- `matches_object` arrays (no assumption key, found in the code): `==` (5 copies) vs item by item
  (31) → item by item, Jest's `toMatchObject` rule the TS tests relied on.
- `hook.style`, `registry.style`, `error.style`, `rng.source`, `test.literals`, `test.name`: wording
  differs, values agree (part 1's `&mut EffectContext` hooks, the testkit override, `Result<_,
  EngineError>`, `Rng::new(seed, cursor)`).
