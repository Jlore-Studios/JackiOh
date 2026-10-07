# Slice: part 15 (cards lane 7: Classic+ #19–#49), chunk 2 of 4 (#403, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
None of the twelve existed on `staging` (no bot half-port; no `.fullsend/notes/part-15-bot.md`). All are
full ports: the TS header as `//!`, every TS function in order with its doc and rule comments, `pub const
ID`, `pub fn script() -> CardScripts`, and the TS test file as `#[cfg(test)] mod tests` (header above it,
one `mod` per `describe`, one `#[test]` per `it`; counts checked by grep). No `todo!`, `unimplemented!`
or `// TODO`.
- `c027_zephrys_zealotism.rs` (14 tests), `c028_nuestro_hogar_nuestras_tumbas.rs` (11),
  `c029_portal_to_the_past.rs` (21), `c030_felinor_fuser.rs` (9), `c031_fusion_lab.rs` (13),
  `c032_1_execute.rs` (8), `c032_2_brawl.rs` (11), `c032_3_blade_storm.rs` (15),
  `c032_otherworldly_removal.rs` (6), `c033_ivory_tower.rs` (13), `c034_memory_leak.rs` (13),
  `c035_rollback.rs` (34), all under `crates/cards/src/scripts/classic_plus/`.

## SURFACE
§7.1 shape throughout. Effect arguments are built from TS's object literal with `json_as(json!({…}))`,
so only each args type's serde shape (TS's keys) matters, not its Rust name. Testkit calls follow part
5.1's notes (the testkit's author): `unit`/`backrow` → `Option<CardInstance>`, `hand`/`pile` →
`Vec<CardInstance>`, `card` → `&CardInstance`, steps return `&mut Scenario`, `expect_refused(|s| s.step(..))`.

## GAPS (names called in other parts' modules; the shape assumed)
- Effects (parts 6/7): `refresh_mana`, `replace_hand_with_perfect` (effects re-export of
  `subsystems::perfect_hand`), `heal`, `discover_from_last_board`, `add_from_last_board`,
  `add_random_from_last_board`, `discover_from_catalog`, `fuse_cards`, `fuse_random_into`, `destroy`,
  `remember`, `remember_random`, `add_to_hand`, `cast_rounds_until_death`, `damage_rounds_until_death`,
  `set_radiant`, `lock_random_zone`, `lock_played_zone` (`{ event }`, a serialised `GameEvent`),
  `roll_back` (effects re-export of `subsystems::board_history`), each `fn(Args) -> Effect` whose Args
  deserialises from TS's literal.
- `effects::for_each_card(ForEachCardArgs { cards: Arc<dyn Fn(&mut EffectContext) -> Vec<String> + Send
  + Sync>, each: Arc<dyn Fn(&str) -> Effect + Send + Sync> })` (parts 6.2/24 notes; closures annotated
  with those parameter types).
- `effects::{chosen_options(&EffectContext) -> Vec<String>, chosen_number(&EffectContext) -> Option<i32>,
  cards_in_scope(&EffectContext, &BoardScope) -> Vec<CardInstance>, instance_of(&EffectContext,
  &TargetSpec) -> Option<CardInstance>}` (refs also compile: ids are `.clone()`d).
- `query::recalled(&EffectContext, &str) -> Option<Value>` (or `Option<&Value>`). **C+ #32.2 Brawl's base
  face needs it to read the live self's memory** (TS `ctx.self` was live): `remember_random` writes the
  survivor after the hook built its context, and the destroy reads it back. Same for #34's `memory.mode`.
- `params::{param(&impl ParamContext, &str) -> i32 (called as param(&*ctx, ..)), step_param(&mut
  CardInstance, &str, i32), param_decl_of(&GameState, &str, &str) -> Option<Param>}`.
- `catalog::{def_of(Option<&GameState>, &str), fused_id_parts(Option<&GameState>, &str) ->
  Option<Vec<String>>, fused_id_specs(Option<&GameState>, &str) -> Option<Vec<FusedIngredient>>}` (part 2.2).
- `zones::{slot_of(&GameState, &CardInstance) -> Option<ZoneSlot>, carried_at(&GameState, impl
  Into<ZoneSlot>) -> Option<&CardInstance>, stacked_onto(&CardInstance) -> Option<&str>}`;
  `layers::unit_has(&GameState, &CardInstance, KeywordKind)`; `mana::effective_cost(&GameState,
  &CardInstance, Default::default())`.
- `subsystems::{rank_perfect_hand(&GameState, PlayerId, RankPerfectHandOptions) -> Vec<Scored>,
  Scored { def, priority: ScorePriority }, ScorePriority::Lethal, snapshot_for(&GameState, i32) ->
  Option<&BoardSnapshot>, choose_action(&GameState, PlayerId, &mut Rng) -> Option<ActionBody>}`.
- Tests: `create_invariant_monitor(&GameState)` with `before(&mut, &GameState, PlayerId, &ActionBody)` and
  `after(&mut, &[GameEvent], &GameState)` → `Vec<String>` (part 5.4); `fold(&FoldArgs)` and
  `CreateGameArgs` built with `json_as` (both `Deserialize`); `seat_to_act -> Option<PlayerId>`;
  `crate::card_def(&str)`, `crate::register_all()`.
- `mana::refresh_some_mana` must not overflow on C+ #27's `ALL_MANA = i32::MAX` (TS `Infinity`):
  `current.saturating_add(amount)` or compare before adding.

## Decisions
- TS factories (`zealotism(radiant)`, `felinorFuser`, `fusionLab`, `otherworldlyRemoval`, `memoryLeak(has)`)
  are private fns returning `Script`; `memoryLeak`'s predicate is a generic `F: Fn(&EffectContext, &str)
  -> bool + Clone`, cloned into the end-of-turn hook and the trigger. `radiant: Script = base` is
  `base.clone()` (#28); `{ ...base, triggers }` is `Script { triggers, ..base.clone() }` (#33).
- TS constant objects that hold literals Rust cannot make `const` (`FELINOR_UNITS`, `HAND_PICK`,
  `DAMAGED_UNIT`, `SURVIVOR_PICK`, `TURNS`) are private fns; `SIDES` is a `const` table of pairs.
- #32's `TOKENS = cardDef(id).id` are literal ids (no static); the tests add all three by those ids.
- `TriggerContext = EffectContext & { event }` is the trigger's two arguments `(ctx, event)`; triggers are
  built with `TriggerDef::new(id, &[type], run)`. #33 reads the live Tower (`ctx.live_self()`) for
  `stacked_onto`/`slot_of`; #30 fuses onto `ctx.self_`'s id.
- Tests: `toMatchObject` is a private `matches_object` over JSON (`js` = `serde_json::to_value`); event and
  view literals are compared as JSON; `indexOf` keeps TS's -1; `s.unit(..) ?? ref` is a private `unit_or`;
  `stepParam(s.card(ref), ..)` writes through `find_instance_mut(s.state_mut(), id)`; `expect(base).not.toBe
  (radiant)` / `.toBe` are `Arc::ptr_eq` on a hook; every test starts with `crate::register_all()`.
- Names: titles snake_cased, `§` → `s`, `#` → `n`, a leading `C+` → `c`; R-id tokens kept in place.
- #35's random-policy test drops TS's `{ timeout: 120_000 }` (no per-test timeout in cargo test).
