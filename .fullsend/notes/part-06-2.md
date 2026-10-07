# Slice: part 6 (engine 5: effect verbs, first half), chunk 2 of 2
BUILDS-RUN: 0

## FILES
All new, each the whole TS file (every function in TS order, exported or not, its doc comments and
every comment that states a rule or cites a ruling): `crates/engine/src/effects/{buff, cast, cost,
damage, datacenter, delay, draw, draw_while, each, enchant, flicker, fruit, fuse, give, hand_exile,
health}.rs`. Nothing left.

## SURFACE
Matched §4.1 (paths), §4.2 (names: every TS function snake_cased, every exported constant kept:
`DELAYED_HOOK`, `THIS_TURN`, `DELAYED_DESTROY_HOOK`, `DELAYED_DISCARD_HAND_HOOK`, `FUSE_ONTO_HOOK`),
§6.6 (`Effect::new`, `lazy_part`; argument types are data with camelCase serde). Hooks take
`&mut EffectContext` (part 1).

## DEPENDS-ON / GAPS

### Called in other modules (TS name snake_cased at its TS module's path; the signature I assumed)
Part 6 chunk 1 (effects):
- `effects::targets` (part 7): `resolve_target(&EffectContext, &TargetSpec) -> Option<DamageTarget>`
  (the unit's instance owned); `instance_of(&EffectContext, &TargetSpec) -> Option<CardInstance>`;
  `instance_on_its_stay(&EffectContext, &str) -> Option<CardInstance>`; `stands_since_script_began(
  &EffectContext, &str) -> bool`; `player_of(&EffectContext, PlayerSpec) -> PlayerId` (by value);
  `cards_in_scope(&EffectContext, &BoardScope) -> Vec<CardInstance>` (OWNED: every caller here mutates
  the context while it walks the list); `sides_of(&EffectContext, Option<ScopeSide>) -> Vec<PlayerId>`.
  Types: `TargetSpec::{SelfCard, SelfHero, EnemyHero, Instance { instance_id }, Chosen { index:
  Option<usize> }}` (tag `of`; `SelfCard` after part 1's `ReplacementWhere::SelfCard`);
  `PlayerSpec::{SelfSide, Enemy}` (after part 1's `DrawLimitPlayer::SelfSide`), `Copy`;
  `BoardScope { side: Option<ScopeSide>, rows: Option<Vec<Row>>, types: Option<Vec<CardType>>, tags,
  not_tags, exclude_self }: Default + Serialize + Deserialize`; `ScopeSide::{Any, SelfSide, Enemy}`
  (the name of TS's inline `"any" | "self" | "enemy"`), `Copy`.
- `effects::card_scope`: `cards_in_card_scope(&EffectContext, &CardScope, options) -> Vec<ScopedCard>`
  (options passed as `Default::default()`); `ScopedCard { card: CardInstance, readers: Readers, .. }`;
  `Readers::Everyone`; `CardScope: Clone + Serialize + Deserialize + PartialEq + Debug`.
- `effects::choose`: `LibraryFilter: Default + Clone + serde`, `matches_library_filter(&GameState,
  &CardInstance, &LibraryFilter) -> bool`.
- `effects::add_to_hand`: `AddToHandArgs { def_id: Option<String>, player: Option<PlayerSpec>, radiant:
  Option<bool>, cost_override: Option<i32>, .. }: Default`, `add_to_hand(AddToHandArgs) -> Effect`.
- `effects::heal`: `HealArgs { target: TargetSpec, amount: Option<i32>, to_full: Option<bool>, up_to:
  Option<i32> }` (TS's three-way union as one struct of options), `heal(HealArgs) -> Effect`.
- `effects::destroy`: `DestroyArgs { target: TargetSpec }`, `destroy(DestroyArgs)`, `destroy_all(BoardScope)`.
- `effects::move_` (part 7): `ExileArgs { target: TargetSpec }`, `exile(ExileArgs)`;
  `DiscardHandArgs { player: Option<PlayerSpec> }`, `discard_hand(DiscardHandArgs)`.
Engine (parts 2–5):
- `damage::{DamageTarget::{Unit { instance }, Hero { player }}, DamageArgs { source: Option<CardInstance>,
  target, amount: i32, flags: Option<DamageFlags> }, DamageFlags { ignore_armor, combat, lifesteal,
  trample: Option<bool> } (the name of TS's inline `DamageArgs["flags"]`), deal_damage(&mut EngineSink,
  DamageArgs) -> i32, set_hero_health(&mut EngineSink, PlayerId, i32, Option<String>)}`.
- `resolve::{lazy_part(&'static str, impl Fn(&mut EffectContext, &Memo) -> EffectPart), make_context(
  EngineSink, None, HookOptions), HookOptions { controller: Option<PlayerId>, .. }: Default,
  cast_card(&mut EngineSink, CardInstance /* owned */, CastOptions), CastOptions { random, target_enemies:
  Option<bool>, afterward: Option<CastAfterward>, .. }: Default, CastAfterward::Exile}`
  (`CastAfterward` is the name of TS's inline `"exile"`; my `CastHow.afterward` uses it).
- `draw::{draw(&mut EngineSink, PlayerId, i32) -> Vec<DrawOutcome>, draw_one(&mut EngineSink, PlayerId,
  None) -> DrawOutcome, complete_draw(.., CardInstance, None), draw_blocked, DrawOutcome::{Drawn, Token}}`.
- `modifiers::{add_modifier(&mut EngineSink, PlayerId, ModifierExpiry, ModifierKind) -> PlayerModifier
  (TS's `Omit<PlayerModifier, "id">` as its two halves, in the struct's field order),
  schedule_delayed(&mut EngineSink, PlayerId, DelayedAt, Resume, Option<String>, Option<i32>) ->
  DelayedEffect, add_start_of_turn_effect(&mut EngineSink, PlayerId, Resume, &str)}`.
- `marks::{mark_delayed(&mut EngineSink, &DelayedEffect, &CardMark), sync_marks(&mut EngineSink, &str,
  &CardMark, &[String])}`.
- `prompts::{SELF_KEY: &str, resume_self(&EffectContext, &str, IndexMap<String, Value>) -> Resume,
  open_prompt(&mut EngineSink, OpenPromptArgs) -> Option<PendingChoice>, OpenPromptArgs { player, kind,
  aim, prompt, options, min, max, budget, owner, resume } (every TS field, the optional ones `Option`),
  close_prompt, why_answer_refused(&PendingChoice, &AnswerInput) -> Result<(), EngineError> (SURFACE
  §4.4.9), in_offered_order(&PendingChoice, &[Selection]) -> Vec<Selection>, AnswerInput { selection:
  Vec<Selection>, .. }}`. prompts.rs dispatches `FUSE_ONTO_HOOK` to
  `effects::fuse::answer_fuse_onto(&mut EngineSink, &AnswerInput) -> Result<(), EngineError>`.
- `work::{RUN_MARKS_KEY, begin_work_cascade(&mut EngineSink), drain_work(&mut EngineSink) -> bool}`.
- `zones::{flicker_in_place(&mut GameState, &CardInstance) -> bool (finds the card by id), is_carried,
  is_buried, lands_face_down(&GameState, &CardInstance, Row), card_at(&GameState, &ZoneSlot), slots_of,
  active_units_of, move_to_zone(&mut GameState, &CardInstance, OffFieldZone, Default::default()) ->
  MoveResult, MoveResult::Moved, report_graveyard_landing(&mut EngineSink, &CardInstance, MoveResult)}`.
- `subsystems::fuse::{fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>, FuseArgs { ingredients:
  Vec<CardInstance>, target, into: Option<CardInstance>, to_hand: Option<PlayerId>, hand_price:
  Option<HandPrice>, radiant: Option<bool>, radiant_ingredients: Option<Vec<String>>, keep_cost:
  Option<bool> }: Default, HandPrice::Free: Copy + serde}` (part 8).
- `subsystems::call_to_chaos::{CHAOS_TAG: Tag, CHAOS_CHAIN_KEY: &str, chaos_chain_of(Option<&CardInstance>) -> i32}`.
- `random_cast::may_cast_now(&GameState, PlayerId) -> bool`; `mana::effective_cost(&GameState,
  &CardInstance, Default::default())`; `layers::{unit_view, unit_has(&GameState, &CardInstance,
  KeywordKind)}`; `faces::card_type_of(&GameState, &CardInstance)`; `restrictions::{immune_to_spells,
  is_spell_source(&GameState, Option<&CardInstance>)}`; `animated::{is_animated, animate_on_entry(&mut
  EngineSink, &CardInstance)}`; `enchantments::add_enchantment`; `numbers::{numbered_keywords_on,
  NumberedKey::Lucky}`; `ownership::{take_into_hand -> Option<TakenTo>, TakenTo::Hand,
  draw_from_library_of, LibraryEnd::Bottom}`; `catalog::{def_of(Option<&GameState>, ..),
  excluding_def_id(Option<&GameState>, &CatalogQueryArgs, Option<&str>), query -> Vec<&'static CardDef>,
  pick_generated(&mut Rng, &[&CardDef], Option<&GameState>), roll_grape}` — these last as part 2 chunk 2's
  notes say it wrote them.

### Not ported
Nothing. TS's module-scope `registerPromptAnswerer(FUSE_ONTO_HOOK, answerFuseOnto)` is the plain
`answer_fuse_onto` (SURFACE §6.6: prompts.rs calls it by its hook).

## Decisions
- Live objects. TS wrote through the instances its resolvers handed back. Here every resolver hands
  back copies, and a write goes through `state::find_instance_mut(state, &card.id)`; where a verb reads
  the card again after its own write (`grant_random_keywords`' pool, `cost.rs`'s event price), it reads
  it back from the state, or keeps writing its copy when the card is in no pile (TS's detached object).
  Functions of other modules that wrote through a card take `&CardInstance` (found by id).
- `ctx.self` is `ctx.self_` (the card as the context was built): the damage source, the converter id,
  the def id a pool excludes. `card_this_draw_put_in_hand` takes `&EngineSink` (TS's `Pick<EffectContext,
  "state" | "events">`), which a context derefs to.
- Prompt identity (`ctx.state.pending !== before`, datacenter and drawWhile) is compared by prompt id.
- Arguments that TS typed as a value or a function of the context are enums with one variant per form:
  `CastNewDef::{Id, Def, Read}` (with `From<&str>`, `From<String>`, `From<CastDef>`),
  `CastRandomQuery::{Fixed, Read}`, `CastRandomCount::{Fixed, Read}`. An argument struct holding a
  function (`CastEachArgs`, `CastNewArgs`, `CastRandomArgs`, `DrawWhileArgs`, `ForEachCardArgs`) derives
  `Clone` only: a function is not data. Function types are named per module (`CastEachCards`,
  `ForEachCardCards`, `ForEachCardEach`, `DrawWhileMore`) so the effects barrel's globs never collide.
- `forEachCard`'s and `castEach`'s `cards` answer ids (`Vec<String>`), not TS's `CardInstance | string`;
  a card file maps its instances to `card.id`.
- TS intersections (`{ target } & BuffAmount`, `& CastHow`, `& DamageFlagArgs`, `& BoardScope`,
  `& TakenRiders`) are inline fields where they are two numbers (buff), and `#[serde(flatten)]` fields
  where the part is a named type (`CastArgs.how`, `DamageEffectArgs.flags`, `DamageAllArgs.flags` and
  `.scope`, `GiveFromHandArgs.riders`, `TakeFromLibraryArgs.riders`). `DamageFlagArgs` is pub (TS kept it
  private) so the flattened field can be named.
- Names for TS's anonymous unions: `BuffAllSide::{One(PlayerSpec), Both(BuffAllBoth)}` (untagged),
  `DelayPlayer::{SelfSide, Enemy, Turn}` ("self" | "enemy" | "turn"), `DiscardHandTurn`, `FuseCardsPick`,
  `FuseIntoPile`, `FuseOntoOtherwise`, `GiveFromHandCards`, `TakeFromLibraryPick`, `FieldSpellSide`,
  `CostRuleSpan`. `DestroyAtNextTurnStartArgs` and `FuseInto` are untagged enums (TS told them apart by
  which key is present).
- `fieldSpellsDoomed`'s reader is `SweepReader { state, self_, def_id, radiant, controller }` with
  `of_context(&EffectContext)` and `of_condition(ConditionContext)` (a card's `preview` reads it too);
  `restrictions.unaffectedBy`/`effectIsFromSpell` are copied privately over it (rule 5).
- `zones.freshFaceDownId` is copied privately in `flicker.rs` for a card standing in the state (TS's
  takes a card in no pile and mutates it), with `state::rename_in_board_history`.
- `resumeAt` (fuseOntoYourCard) and `resumeOf` (answerFuseOnto) are inlined: on these values they only
  fill defaults a typed `Resume` already has and run `work.cardData`, which keeps `{ ingredient }` as is.
- `compare_text` compares UTF-16 code units (`encode_utf16().cmp(..)`), JS's `<`, because card names may
  leave ASCII; the library sort is `sort_by` (stable, SURFACE §4.4.1).
- `delete`/rest-spread of a data key is `IndexMap::shift_remove` (keeps the other keys' order).
- Constants stay where TS put them except what part 1 moved: `HAND_CAP`, `RANDOM_KEYWORD_POOL`,
  `FUSE_MIN_INGREDIENTS` come from `crate::config`.
