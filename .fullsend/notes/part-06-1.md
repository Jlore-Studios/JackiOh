# Slice: part 6, chunk 1 of 2 (engine 5: effect verbs, first half; #394, parent #306)
BUILDS-RUN: 0

## FILES
All fourteen hold the whole port of their TS file, every function in TS order with its doc and rule
comments (R-ids and §-ids kept):
`crates/engine/src/effects/{add_to_hand, after_check, animate, brittle, card_scope, choose,
choose_where, coins, combat, counters, cry, destroy, heal, kill_credit}.rs`.
No `todo!`, `unimplemented!` or `// TODO`. No function left out.

## SURFACE
- Each TS constructor is `pub fn <name>(args: <Name>Args) -> Effect` returning `Effect::new("<TS kind>",
  move |ctx| …)` (SURFACE §6.6). A constructor whose TS args defaulted to `{}` still takes its args by
  value (`animate(AnimateArgs::default())`). `destroy_all` takes `BoardScope` itself, as TS did.
- Argument types: `Serialize, Deserialize, Clone, Debug, PartialEq`, camelCase, `Default` when every
  field is optional, every `x?:` an `Option` with `default, skip_serializing_if`. Names: `<FnName>Args`
  for TS's inline objects (`AddToHandArgs`, `DestroyArgs`, `ChoosePickArgs`, …); TS's named types keep
  their names (`AnimateArgs`, `HealArgs`, `BrittleTarget`, `CardScope`, `CardZone`, `Readers`,
  `ScopedCard`, `TargetScope`, `CellScope`, `PileSpec`, `PickFilter`, `DiscoverOffer`, `LibraryFilter`,
  `ZoneSpec`, `ForcedSide`, `ForcedAttackerFilter`, `ForcedTarget`).
- Intersections: `BrittleTarget & { n }` is `GiveBrittleArgs { #[serde(flatten)] on: BrittleTarget, n }`
  (`GainBrittleArgs` is an alias); `{ target } & BoardScope` is `DestroyAdjacentToArgs { target,
  #[serde(flatten)] scope }`.
- Args holding a callback (`ChooseFromHandArgs.where_`, `ChooseTargetWhereArgs.where_`,
  `DiscoverFromCatalogArgs.query_fn`) carry it as `Option<Arc<dyn Fn … + Send + Sync>>` under
  `#[serde(skip)]`, so they derive `Serialize, Deserialize, Clone, Default` and a hand-written `Debug`,
  but not `PartialEq`. `WithKillCreditArgs` (two callbacks and an `Effect`) is plain Rust data:
  `Clone` and a hand-written `Debug`, no serde.
- Exported constants: `ANSWER_OPTION_IDS: &[&str]` (choose.rs). Private: `PART_MEMO`, `READER_ORDER`,
  `TRAP_TYPES`, `MAX_SAFE_INTEGER`.

## DEPENDS-ON
Called by their TS names snake_cased at their TS module's path (rule 6), with these assumed shapes:
- part 7, `effects::targets`: `TargetSpec` (tag `of`; variants `SelfCard` = "self", `SelfHero`,
  `EnemyHero`, `Instance { instance_id }`, `Chosen { index: Option<usize> }`), `PlayerSpec` (`Copy`;
  `SelfSide` = "self", `Enemy`), `BoardScope`; `player_of(&EffectContext, PlayerSpec) -> PlayerId`,
  `resolve_target(&EffectContext, &TargetSpec) -> Option<DamageTarget>`,
  `instance_of(&EffectContext, &TargetSpec) -> Option<CardInstance>`,
  `instance_on_its_stay(&EffectContext, &str) -> Option<CardInstance>`,
  `cards_in_scope(&EffectContext, Option<&BoardScope>) -> Vec<CardInstance>`,
  `adjacent_to(&EffectContext, &TargetSpec, Option<&BoardScope>) -> Vec<CardInstance>`.
- part 6.2, `effects::buff`: `BuffAmount { attack: Option<i32>, health: Option<i32> }` (`Default`),
  `buff(..) -> Effect`, `grant_keyword(..) -> Effect`. Their args are built with
  `json_as(json!({ "target": …, "attack": …, "health": … }))` / `({ "target", "keyword" })`, so only the
  serde shape (TS's) matters, not the struct's layout or name.
- part 2: `catalog::{CatalogQueryArgs (Default, Clone, serde), GlitchOdds { system_plays: Option<i32> },
  def_of(&GameState, &str) -> &CardDef, excluding_def_id(&CatalogQueryArgs, Option<&str>) ->
  CatalogQueryArgs, query(&CatalogQueryArgs) -> Vec<CardDef>, pick_generated(&mut Rng, &[CardDef],
  Option<&GlitchOdds>) -> Option<CardDef>}`; `zones::{ZoneSlot (struct literal { player, row, lane }),
  active_units_of(&GameState, PlayerId) -> Vec<&CardInstance>, card_at(&GameState, &ZoneSlot) ->
  Option<&CardInstance>, slots_of(PlayerId, Row) -> Vec<ZoneSlot>, slot_of(&GameState, &CardInstance) ->
  Option<ZoneSlot>, row_size(Row) -> i32, is_unit_token, is_locked(&GameState, &ZoneSlot),
  lock_zone/unlock_zone(&mut GameState, &ZoneSlot)}`; `faces::card_type_of(&GameState, &CardInstance)`;
  `preview::is_face_down(&GameState, &CardInstance)`; `layers::unit_view(&GameState, &CardInstance) ->
  UnitView` (`.health`); `plague::{place_plague_on(&mut EngineSink, &CardInstance, i32),
  remove_plague(same), plague_on(&CardInstance) -> i32}`; `brittle_count::{active_brittle_count(
  &CardInstance) -> Option<i32>, give_brittle_count/gain_brittle_count(&mut GameState, &CardInstance,
  i32)}`; `kill_credit::{KILL_CREDIT_KEY, KillCredit { victim_id, to_id }}`; `stays::exit_mark(&GameState)
  -> u32`; `animated::animate_card(&mut EngineSink, &CardInstance, Option<Position>)`.
- part 3: `prompts::{OpenPromptArgs { player, kind, aim, prompt, options: Vec<PromptOption>, min, max,
  budget, owner, resume } (every optional an Option), open_prompt(&mut EngineSink, OpenPromptArgs),
  resume_self(&EffectContext, &str, Option<IndexMap<String, Value>>) -> Resume, hero_option_label(
  PlayerId, PlayerId) -> String, cell_option_label(PlayerId, Row, i32, PlayerId) -> String,
  summoned_so_far(&EffectContext) -> Vec<String>, ANSWER_KEY, answer_key_of(&IndexMap<String, Value>) ->
  Option<String>}`; `resolve::{lazy_part(&'static str, impl Fn(&mut EffectContext, &Memo) -> EffectPart
  + Send + Sync + 'static) -> Effect, apply_effects(&[Effect], &mut EffectContext)}`;
  `state_check::{state_check(&mut EngineSink), sacrifice_now(&mut EngineSink, &CardInstance)}`;
  `damage::{DamageTarget::{Unit { instance: CardInstance }, Hero { player }}, already_killed(&GameState,
  &CardInstance), heal_unit/heal_to_full(&mut EngineSink, &CardInstance, …), heal_hero/heal_hero_up_to(
  &mut EngineSink, PlayerId, i32)}`; `combat::{AttackTarget, AttackAmong, force_attacks_on(&mut
  EngineSink, &[CardInstance], &AttackTarget, Option<u32>), force_attacks_random(&mut EngineSink,
  &CardInstance, AttackAmong, i32, Option<u32>), force_attack_own_hero(&mut EngineSink, &CardInstance)}`;
  `triggers::{SETTLE_PASS_CAP, SettleSink { sink: EngineSink, dispatched: Option<u32> },
  dispatch_pending(&mut SettleSink), run_queued_trigger(&mut EngineSink, &QueuedTrigger)}`;
  `work::paused(&EngineSink) -> bool`.
- part 4: `draw::add_to_hand(&mut EngineSink, &CardInstance)` (return value unused),
  `mana::effective_cost(&GameState, &CardInstance, Option<…options>) -> i32`,
  `cry_trigger::{cry_place_of(&GameState, &CardInstance) -> Option<CryPlace>, trigger_cry_of(&mut
  EngineSink, &CardInstance, PlayerId)}`.
- part 8: `subsystems::ai_policy::play_out_turn(&mut EngineSink, PlayerId, Option<…PolicyOptions>)`.

## GAPS
- Not ported: nothing. Every TS function of the fourteen files is here.
- Names and shapes I could not see, assumed as listed under DEPENDS-ON. The likeliest misses, for part 31:
  - `combat::AttackAmong`: TS `"enemies" | "enemyUnits"` is an inline union in `combat.ts`
    (`randomAttackTargets`, `forceAttacksRandom`), so part 3 names it; `effects::combat::
    ForcedAttackRandomArgs.among` and `effects::split` (part 7) need the same type.
  - `triggers::SettleSink`: TS `EngineSink & { dispatched?: number }`; assumed a struct with fields
    `sink: EngineSink` and `dispatched: Option<u32>` (`effects::combat::settle_before_playout`).
  - `TargetSpec::SelfCard` and `PlayerSpec::SelfSide` for TS's `"self"` (part 1's precedent:
    `ReplacementWhere::SelfCard`, `DrawLimitPlayer::SelfSide`). `ZoneSpec::SelfCard`,
    `CardScopeSide::SelfSide`, `CellSide::SelfSide`, `ForcedSide::SelfSide` are mine, the same way.
  - `catalog::GlitchOdds { system_plays }`: TS passed the state as `GlitchOdds` structurally.
  - `animated::animate_card`'s TS options `{ position? } = {}` passed as its one field,
    `Option<Position>`.
  - Trailing TS parameters with a default (`since = exitMark(…)`, `options = {}`, `scope = {}`,
    `data = {}`) are passed as `Option` (`None` where TS omitted them, `Some(&x)` for an object).
- Possible glob collision (part 6.2/7): `effects/radiant.ts` has a private `Readers` type and
  `readersOf`; if its port makes them `pub`, `effects::*` sees two `Readers`/`readers_of` (mine are
  `card_scope`'s, which TS's barrel exported).
- `crate::prelude::json_as` is called from engine code (coins.rs), not only from cards.

## Decisions
- Card instances crossing a module boundary: a TS function that returned the live `CardInstance`
  returns an owned clone when it reads through `ctx` (`effects::targets`), a reference when it reads a
  `&GameState` (`zones`, `state::find_instance`). Every callee that took a TS card gets `&CardInstance`
  (the card as found); a callee that mutates it finds it by id. Where TS wrote through the live object
  itself (cost riders, `markedDestroyed`, `memory[KILL_CREDIT_KEY]`, the radiant rider), I write through
  `state::find_instance_mut` by id; where TS read the live object after a write (`brittle`'s `report`,
  `destroy`'s `alreadyKilled`, `counters`' `ctx.self` zone), I read it again by id (`live_self()`).
- `add_to_hand`'s "did it reach the hand": computed before the move with `draw.ts`'s own test (the
  owner's hand below `HAND_CAP`), not from `draw::add_to_hand`'s return value, whose Rust type I cannot
  see. Equivalent: `addToHand` burns exactly when that hand is full.
- `{ ...ctx, exitsFrom }` (after_check.rs) is the same context with `exits_from` replaced for the call
  and restored after (`with_exits_from`).
- `after_state_check`'s rest memo: any JSON number reads back as the mark (`as_f64`), as TS's `typeof
  memo === "number"`.
- `TargetScope.side`/`.of` reuse the wire's `FilterSide`/`FilterOf` (same literals; `FilterOf` is wider).
- `choose_answer`'s `correct` is `usize` (SURFACE §4.3: an index), so TS's `correct < 0` guard is moot.
- `chosen_number`: TS `Number(option)` on the trimmed, non-blank option, kept when finite and integral.
- `creation_number` keeps TS's float (`Number.MAX_SAFE_INTEGER` for a non-`c<n>` id) and sorts stably
  with `total_cmp`.
- `slice(0, count)` keeps JS's negative-end meaning (`slice_end`).
- IndexMap removal of `KILL_CREDIT_KEY` is `shift_remove` (JS `delete` keeps the other keys' order).
- `settle_before_playout` runs on `ctx.sink.reborrow()` wrapped in a `SettleSink`, and panics with TS's
  message after `SETTLE_PASS_CAP` passes.
- TS's comments that only described a TS mechanism (the import cycle note in `aiPlaysOutTurn`, the
  "context without a mark falls back to 0" note) are reworded for Rust; every rule they stated is kept.
