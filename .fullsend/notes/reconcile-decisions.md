# engine tests

## Cluster: fixture CardDef shape (static vs fn)
Winner: each `crates/engine/tests/rules/fixtures/<x>.rs` as written — score n/a (rule 2: the fixture file owns its names) — LazyLock statics in combat, field, instance_data, generation, damage_combat, datacenter, activate, call_to_chaos_plus, core_patches, fruit, kill_credit, board_history; `fn x() -> CardDef` in play_pipeline_b, prompts, quests, scripts, turn, twice_forward, ky_test, papaya
Losers: the callers' guesses (`plain().id` for a static, `titan.id` for a fn) — rewritten, no fixture changed
Callers updated: 1,354 sites (1,344 static calls → `x.field` / `x.field.clone()` where moved / `&*x` / `x.clone()`; 10 fn field reads → `x().id`) in 89 test files
Semantic conflicts: test.fixture_defs → the fixture file's own definition (rule 2)
Unresolved: none
Lines: before 99,006, after 98,968 (whole `crates/engine/tests`, all clusters)

## Cluster: fixture script tables (TS static vs brief `scripts()` vs hedge alias)
Winner: the TS-named statics (`TURN_SCRIPTS`, `TWICE_FORWARD_SCRIPTS`, `PB_SCRIPTS`, `PROMPT_SCRIPTS`, `FIXTURE_SCRIPTS`, `*_DEFS`; SURFACE §4.2: constants keep their TS names) — score 0 behaviours lost, 0 options — plus each fixture's brief-mandated `scripts()`/`catalog()` (part 24 brief step 2), kept
Losers: `turn_scripts()`, `turn_defs()`, `twice_forward_scripts()`, `pb_scripts()`, `pb_defs()`, `prompt_scripts()`, `prompt_defs()`, `fixture_scripts()`, `fixture_defs()` (hedge aliases that only cloned the static) — deleted
Callers updated: 5 files (turn_wiring, delayed_kinds, draw_limit, twice_forward, effects_fruit); plus names no fixture had, pointed at the owners': `activate_scripts()`→`ACTIVATE_SCRIPTS`, `combat_scripts()`→`COMBAT_SCRIPTS`, `fruit_scripts()`→`FRUIT_SCRIPTS`, `gen_scripts()`→`GEN_SCRIPTS`, `core_patch_scripts()`→`CORE_PATCH_SCRIPTS`, `lab_pool()`→`LAB_POOL`, `test_mark()`→`TEST_MARK`, `ct()`→`CT`, `rng_child::run`→`rng_child::rng_child` (8 files)
Semantic conflicts: turn::scripts() vs twice_forward_scripts()/activate_scripts() naming → TS constant names
Unresolved: `scripts()` and the static are still two names for one table in every fixture; kept because part 24's brief mandates `scripts()` and the TS name is the static (removing either touches ~25 files)
Lines: 49 deleted in fixtures

## Cluster: harness helper signatures (put, new_game, in_hand, set_library, flush, vanilla_deck, token_def, cast_now, recorder, replayable, action helpers)
Winner: the owners' definitions — `harness::put(state, id, slot, options: Value)` (4 args), `harness::new_game(&str, Option<(Vec, Vec)>)`, `instance_data::instance_game(&str, Option<..>)`, `harness::in_hand(.., count: i32)`, `harness::set_library(.., &[impl AsRef<str>])`, `field::flush(.., mana: i32)`, `catalog::vanilla_deck(i32, i32)`/`vanilla_catalog(i32, i32)`, `catalog::token_def(name, impl Into<Vec<Tag>>)`, `prompt_harness::cast_now(.., PlayerId, bool) -> SinkResult`, `damage_combat::recorder(&GameState)`, `prompt_harness::replayable(&str, &[String], &[String])`, and `act`/`act_result`/`pb_act`/`pb_reduce`/`play`/`refusal` taking `impl Serialize`
Losers: callers' 3-arg `put` (794), 1-arg `new_game`/`instance_game` (11), `None`/`Some(n)` for TS defaults (in_hand 41, flush 9, vanilla_deck/catalog 35 calls, token_def 5, cast_now 3, instance_game 2), `harness::PutOptions` (3), `&[]` without a type (14), owned `recorder` args (32), `json_as(json!)` into `impl Serialize` (81), `&[x.as_str()]` into replayable (6), `OffFieldZone` into `damage_combat::in_pile` (18 → `ZoneName`)
Callers updated: ~1,050 sites
Semantic conflicts: test.default_params → fixture owners take TS defaults as plain values (TS `= 1`, `= 10`, `= 20, 1`, `= ["Token"]`, `= "p1", false`); test.literals → `json!` into fixture helpers that take `impl Serialize`
Unresolved: none for fixtures; engine functions with the same Option-vs-plain question are the engine owner's (damage file, Seams)

## Cluster: LOG_LANE type
Winner: `i32` (fixtures/activate.rs, fixtures/damage_combat.rs) — settled by part 1's frozen `ZoneRef.lane: i32`, which `harness::slot` takes
Losers: `fixtures/turn.rs`'s `pub const LOG_LANE: usize` — changed to `i32`, its two indexing uses cast
Callers updated: 0 (all four callers already passed it to `slot`)
Semantic conflicts: lane number type → i32 (part 1, SURFACE §4.3 game quantity)
Unresolved: none
Lines: 0

## Decision: local mechanical fixes taken with the clusters
One syntax error (`fixtures/copied_text.rs:253`, a missing `)`, which stopped the whole `rules` binary at the parser); four parameters renamed that shadowed fixture statics (E0530: `body` in fuse_variants, instance_data ×2, params); `use std::sync::Arc;` in four files (the testkit exports no `Arc`); `preview.rs`'s helper reads `wire::UnitView` (part 1: the root `UnitView` is `layers`'). No assertion changed.
