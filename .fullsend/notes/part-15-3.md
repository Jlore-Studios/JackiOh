# Slice: part 15 (cards lane 7: Classic+ #19–#49), chunk 3 of 4 (#403, parent #306)
BUILDS-RUN: 0

## FILES
All twelve were absent on `staging` (no bot half-port, no `part-15-bot.md`); each is a full port of
its TS script (every function in TS order, header and rule comments kept) and of its TS test file
(one `#[test]` per `it`, one `mod` per inner `describe`, counts checked against the TS: 10, 9, 22, 13,
21, 11, 20, 14, 13, 27, 11, 13):
`crates/cards/src/scripts/classic_plus/{c036_1_bone_storm, c036_conjure_bones, c037_wardrum,
c038_1_solarius_prime, c038_solarius, c039_book_worm, c040_appropriations, c041_kys_constant,
c042_1_kys_gift, c042_kys_test, c043_ai_slop, c044_simplicity_audit}.rs`. No `todo!`,
`unimplemented!` or `// TODO`. Nothing left.

## SURFACE
§4.1 paths, §4.2 names, §7.1 file shape (`pub const ID`, `pub fn script() -> CardScripts`, tests at
the bottom under the TS test file's header comment), §8 testkit verbs. Hooks take `&mut EffectContext`
(part 1). Effect arguments that are data are built from TS's literal with `json_as(json!(…))`; the
effect verbs are imported explicitly from `jackioh_engine::effects` (an explicit import beats the
prelude's globs, so no name a glob shares can be ambiguous).

## GAPS (names called in other parts' modules, with the shape assumed; part 31 reconciles)
- `params::param(&impl ParamContext, &str) -> i32` (part 2.1): `param(&*ctx, …)` in hooks,
  `param(&c, …)` on a `ConditionContext`, `param(ctx, …)` on a `&EffectContext`.
- `faces::card_type_of_face(&GameState, &str, bool) -> CardType` (TS `cardTypeOf(state, { defId, radiant })`).
- `query::{played_ids_this_turn(&GameState, PlayerId) -> &[String] or Vec<String>,
  played_this_turn_of_type(&GameState, PlayerId, &[CardType]) -> i32, played_this_game_with_tag(&GameState,
  PlayerId, Tag) -> i32}`; `zones::first_free_zone(&GameState, PlayerId, Row) -> Option<_>`;
  `plague::plague_on(&CardInstance) -> i32`; `numbers::numbers_on(&GameState, &CardInstance) ->
  Vec<NumberOnCard { id: String, value: i32, .. }>`; `catalog::{def_of(Option<&GameState>, &str) ->
  &CardDef, query(&CatalogQueryArgs), query_cost(&CardDef) -> i32}`; `mana::effective_cost(&GameState,
  &CardInstance, Default::default())`; `prompts::answer_key_of(&IndexMap<String, Value>) -> Option<String>`;
  `params::step_param(&mut CardInstance, &str, i32)`; `enchantments::{has_enchantment(&CardInstance,
  EnchantmentKind), EnchantmentKind::{CastOnDraw, TargetEnemies}}` (C+ #40's test).
- Effects (parts 6–7), by `json_as`: `damage_all`, `shuffle_into`, `place_plague`,
  `add_random_from_catalog`, `buff_cards`, `grant_keyword_cards`, `radiant_chance`,
  `shuffle_random_from_catalog`, `set_number`, `discover_number`, `discard_random`, `gain_mana`, `heal`,
  `fuse_generated`, `exile`; `summon_this()`; readers `chosen_options(&EffectContext) -> Vec<String>`,
  `chosen_tuning_number(&EffectContext) -> Option<{ instance_id: String, which: NumberRef (Serialize) }>`,
  `unreadable_by(&GameState, &CardInstance) -> Vec<PlayerId>`.
- Function-holding effect args, by struct literal (part 6.2's names), each behind one private helper:
  `CastNewArgs { def: CastNewDef::Read(Arc<dyn Fn(&mut EffectContext) -> Option<CastDef> + Send + Sync>),
  radiant: None, how: CastHow::default() }` with `CastDef { def_id: String, radiant: Option<bool> }`
  (C+ #37 `cast_new_read`); `CastRandomArgs { query: CastRandomQuery::Fixed(<json_as>), count:
  Some(CastRandomCount::Fixed(i32)), radiant: Some(bool), how: <json_as of { targetEnemies: true }> }`
  (C+ #38.1 `cast_random_spells`); `ForEachCardArgs { cards: Arc<dyn Fn(&mut EffectContext) ->
  Vec<String>>, each: Arc<dyn Fn(&str) -> Effect> }` (C+ #37, #44), fed fn items or a returned
  `impl Fn`, so a `&EffectContext` alias needs only the `&mut` dropped there.
- Subsystems (part 8): `subsystems::ky_test_script(&[KyTestProblem]) -> KyTestScript { cry: Hook,
  resume: IndexMap<&'static str, Hook> }`; `subsystems::{lines_of_code(&GameState, &str) -> i32,
  audit_targets(&GameState, &AuditArgs { controller, active, loc, more, enemy_only }) -> Vec<_>}`
  (answers owned or borrowed, both taken through `Borrow`); `subsystems::{fuse(&mut EngineSink,
  FuseArgs { ingredients, target: Option<CardInstance>, .. }: Default) -> Option<CardInstance>,
  fused_ingredient_specs(&GameState, &str) -> Option<Vec<FusedIngredient>>, fused_ingredients(&GameState,
  &str) -> Option<Vec<String>>, POWER_KEY}`. TS's `fusedIngredientSpecs(defId)` took no state; I pass it
  (part 8.1/17: the digest needs the state).
- Cards crate: `crate::ky_test_bank::KY_TEST_BANK` (chunk with `ky_test_bank.rs`), iterable with
  `.iter()`/`.len()` and borrowable as `&[KyTestProblem]` (a `&'static [_]` const or a
  `LazyLock<Vec<_>>` both work); `KyTestProblem { id, difficulty, statement, options, answer }`, each
  read with `to_string()` so `String` or `&str`/`KyTestDifficulty` all compile.
- Testkit (part 5): `scenario(Value)`, `Scenario::{play, attack, answer, end_turn, activate,
  expect_*, expect_refused_with, state, state_mut, events, view, unit, backrow, hand, pile, card,
  stats}` as part 5.1's notes give them; `FoldArgs` must `Deserialize` (built with `json_as`), and
  `fold(&FoldArgs) -> { state, errors }`.

## Decisions
- Card tests run inside the cards crate, so each test module wraps the testkit's `scenario` in a local
  `scenario` that calls `crate::register_all()` first (part 5.1: the TS harness registered at import).
- Events, views and preview values are compared as JSON (`serde_json::to_value`), keeping TS's literals
  and field names; states and cards are read through their frozen Rust fields.
- TS `stepParam(s.card(x), …)` and writes to `s.card(x).…` (memory, tuning, enchantments, damage,
  granted keywords) go through `find_instance_mut(s.state_mut(), id)` (TS wrote the live object).
- TS `expect(radiant).toBe(base)` (one script object) is `Arc::ptr_eq` on the hook both faces share
  (`radiant = base.clone()` shares the `Arc`s).
- TS `{ ...side, ...opts.p1 }` is a shallow JSON merge (`merged`); TS `indexOf`/`findIndex` comparisons
  keep their -1 (`i64`); the Easy problem's regex is parsed by hand (no regex crate).
- C+ #40's R656 test reads the eight Books' scripts through `crate::scripts_of()[id]` rather than their
  module paths.
- C+ #43 keeps TS's literal `count < 2`; C+ #44's `audit` takes a `fn(&EffectContext) -> bool`.
- Test names: TS titles snake_cased with ruling tokens leading; a title that starts with `§n.m` gets an
  `s` prefix (`s2_4_…`), apostrophes dropped (`opponents`).
