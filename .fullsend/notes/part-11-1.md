# Slice: part 11, chunk 1 of 4 (cards lane 3: Core #82–#90.1)
BUILDS-RUN: 0

## FILES
All complete (script, and every TS `it` as a `#[test]`). None was on `staging` before this chunk (the
night bot's copy at 0180138 had none of them), so every file is new.
- `crates/cards/src/scripts/core/c082_kys_trial.rs` (9 tests)
- `crates/cards/src/scripts/core/c083_transmogulate.rs` (15 tests)
- `crates/cards/src/scripts/core/c084_going_long.rs` (13 tests)
- `crates/cards/src/scripts/core/c085_unlicensed_experimentation.rs` (23 tests; the R662 `for` loop is 6)
- `crates/cards/src/scripts/core/c086_miss_mrow.rs` (10 tests)
- `crates/cards/src/scripts/core/c087_pocket_chaos.rs` (17 tests)
- `crates/cards/src/scripts/core/c088_twisting_nether.rs` (9 tests)
- `crates/cards/src/scripts/core/c089_corpse_eater.rs` (14 tests)
- `crates/cards/src/scripts/core/c090_1_cn_virus.rs` (script only: the TS card has no test file; #90's
  tests cover it)

## SURFACE
Each file: `pub const ID: &str` and `pub fn script() -> CardScripts`; nothing else public.

## DEPENDS-ON
Engine (via `jackioh_engine::prelude::*`, by TS name snake_cased):
- effects: `discover_from_catalog`, `chosen_options`, `add_to_hand`, `transform`, `fuse_cards`, `steal`,
  `swap`, `exile`, `destroy_all`, `buff`, `damage`, `delay`, `shuffle_into`, `THIS_TURN`; every one
  built with `json_as(json!({ …the TS literal… }))`.
- reads: `def_by_index(SetName, &str)`, `def_of(&GameState, &str)` (TS `defOf(state, defId)`),
  `slots_of(PlayerId, Row) -> Vec<ZoneSlot>`, `card_at(&GameState, &ZoneSlot) -> Option<&CardInstance>`,
  `unit_has(&GameState, &CardInstance, KeywordKind)`, `zone_cards(&GameState, PlayerId, OffFieldZone)`
  (used as `.to_vec()` into `Vec<CardInstance>`), `numbering_order` (part 1's), `find_instance`
  (part 1's), `fusable_permanents_of(&GameState, PlayerId, Option<&str>)`, `killer_of(&GameState,
  Option<&CardInstance>)`, `printed_cost(&GameState, &CardInstance) -> i32`, `RESUME_HOOK`,
  `TrapTrigger`.
- cards: `crate::query::{pool, TRAP_TYPES}` (part 9), `crate::CATALOG`, `crate::card_def` (part 1).
- tests: `scenario`, `Scenario` and its verbs (part 5, SURFACE §8), `backrow_glows`/`opponent_sees_glow`,
  `flags_of`, `keywords_of`, `legal_actions`, `view_for`, `lock_zone`, `is_locked`, `find_instance_mut`.

## GAPS
- `flags_of` (part 2, `scripts.rs`): called as `flags_of(&GameState, &CardInstance) -> StaticFlags`. TS
  `flagsOf(instance)` takes no state, but SURFACE §6.6 builds fused scripts on lookup from the state,
  so I passed it (c084 tests). Drop the first argument if part 2 kept TS's shape.
- `discover_from_catalog`'s args (part 7, `effects/choose.rs`): TS `query` is an object or a function;
  c082 passes the object as JSON (`"query": { "set": "Core" }`), so the args struct must deserialise
  `query` from an object.
- `crate::query::pool(own_id: &str, args: CatalogQueryArgs) -> Vec<CardDef>` (part 9): called with the
  args by value, built with `json_as`; `TRAP_TYPES` serialised with `json!` (any slice/array/Vec of
  `CardType` works).
- Scenario signatures assumed (part 5): `s.unit/backrow(PlayerId, i32) -> Option<&CardInstance>`,
  `s.hand(Option<PlayerId>)`, `s.pile(PlayerId, &str)` → `Vec<CardInstance>`, `s.view(Option<PlayerId>)`,
  a card ref accepted as `&str` (catalog id or instance id) or `&CardInstance`, `s.attack(card, card |
  "hero")` with the two arguments of independent ref types, `s.state_mut()`, `s.expect_refused(_with)`
  taking `FnOnce(&mut Scenario)`; glow helpers taking `&Scenario`.
- `zone_cards` return type: I wrote `.to_vec()` into a `Vec<CardInstance>`, which fits a
  `Vec<CardInstance>` or `&[CardInstance]` answer; a `Vec<&CardInstance>` answer needs `.cloned()`.

## Decisions
- No bot half-ports existed for these nine files (`.fullsend/notes/part-11-bot.md` lists none of
  mine), so all were written fresh; the bot's CONVENTIONS were followed where part 1 agrees. Where they
  disagree, part 1 won: hooks take `&mut EffectContext`; triggers are `TriggerDef::new(id, &[types],
  |ctx, event| …).with_when(|ctx, event| …)` (two arguments, no `TriggerContext`); typed hooks via
  `condition_hook`; `crate::card_def(id)` returns an owned `CardDef`; the catalog is `crate::CATALOG`.
- TS module constants that hold a `Vec`/`Arc` (`SWAP_MODE`, `GIFT_MODE`, Twisting Nether's `modes`,
  Unlicensed Experimentation's `conditionMet`, Mrow's `death`) became private `fn`s returning the value;
  a hook shared by both faces is built once in `script()` and cloned.
- TS test constants computed from the catalog (`POOL`, `LEGENDARY_UNITS`, `LEGENDARY_FIELD_SPELLS` in
  #83) became private functions called after `register_all()`.
- `toEqual` against an object literal compares `serde_json::to_value` with `json!` (private `js`);
  `toMatchObject` is a private `matches_object` (c089 only).
- TS loops over faces that generate `it`s (#85 R662) are one `#[test]` per face calling a shared private
  body; loops inside one `it` (#86, #87) stay loops inside one `#[test]`.
- `s.card(X).radiant = true` (the TS "harness request" workaround) is `find_instance_mut(s.state_mut(),
  &id).radiant = true`.
- c090_1 has no `mod tests` (no TS test file).
