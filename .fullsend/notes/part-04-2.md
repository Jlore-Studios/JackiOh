# Slice: part 4 (engine 3), chunk 2 of 2 — cost rules, Trigger a Cry, draw, graveyard play, play choices
BUILDS-RUN: 0

## FILES
- `crates/engine/src/cost_rules.rs` ← `packages/engine/src/costRules.ts` (whole)
- `crates/engine/src/cry_trigger.rs` ← `packages/engine/src/cryTrigger.ts` (whole)
- `crates/engine/src/draw.rs` ← `packages/engine/src/draw.ts` (whole)
- `crates/engine/src/graveyard_play.rs` ← `packages/engine/src/graveyardPlay.ts` (whole)
- `crates/engine/src/play_choices.rs` ← `packages/engine/src/playChoices.ts` (whole)

Every TS function, exported or not, is ported in TS order with its doc comment and every comment that
cites a rule or ruling. Nothing left out. Not ported, by design: the two `registerWorkHandler` calls
(draw.ts) and the `registerPromptAnswerer` call (cryTrigger.ts) — SURFACE §6.6; the handlers are `pub`
for the dispatchers (below). TS's local alias `TributeFlags` is a comment (StaticFlags already has
`tribute_enemies`).

## SURFACE
Matched §4.1 paths, §4.2 names (snake_cased TS names, TS type names kept), §4.3 types, §4.4 semantics
(stable sorts only; IndexSet/IndexMap for every Set/Map; RNG draw order unchanged — `draw` draws only
in `shuffle_into_library`'s `rng.int(len + 1)`, as TS), §6.5 (`&mut EngineSink` for sink mutators).
Uses part 1's frozen types and never redefines them: `state::CostRule`, `script::{CostAura,
CostAuraWhose, CostAuraArgs, GraveyardPlayPermission, DrawLimit, DrawLimitPlayer, HookArgs,
TargetCheckArgs, EngineSink}`, `wire::PlagueSpend` (`graveyard_play` does `pub use crate::wire::PlagueSpend`
so TS's path resolves; same item, no glob ambiguity), config's `RADIANT_SHEEP_TRIBUTE_VALUE`,
`SHEEP_TRIBUTE_VALUE` (re-exported from `play_choices`), `MAX_CHOICE_COMBINATIONS: usize`,
`MIN_CHOSEN_X`, `CAST_ON_DRAW_CHAIN_CAP`, `FATIGUE_DAMAGE`, `HAND_CAP`, `LIBRARY_CAP`, `SETUP_TURN`,
`MIN_PLAGUE_PAYMENT`, `PLAGUE_TOKEN_MANA`, `GLITCH_DEF_ID`.

## DEPENDS-ON
Dispatchers that must name my functions (SURFACE §6.6, no registration):
- `work.rs` (part 3): `"@drawChain"` → `draw::run_owed_draw_chain(&mut EngineSink, &WorkItem)`,
  `"@drawCount"` → `draw::run_owed_draw_count(&mut EngineSink, &WorkItem)`.
- `prompts.rs` (part 3): `"@triggerCry"` (`cry_trigger::TRIGGER_CRY_HOOK`) →
  `cry_trigger::answer_cry_prompt(&mut EngineSink, &AnswerInput) -> Result<(), EngineError>`.

## GAPS
Names I call that other parts provide, with the shapes I assumed (part 31: these are guesses wherever TS
had an anonymous options object or a string union):
- `catalog` (part 2): `def_of(Option<&GameState>, &str)` → something with `.name`, `.tags`, `.cost`
  (read through a reference and cloned at once); `def_by_index(SetName, &str) -> Option<&CardDef>`.
- `scripts` (part 2): `script_of(&GameState, &str) -> CardScripts` (part 2's brief). I read `.base` /
  `.radiant` by reference and clone, so a `&CardScripts` answer works too.
- `faces` (part 2): `card_type_of(&GameState, &CardInstance) -> CardType`.
- `zones` (part 2): `ZoneSlot { player, row, lane: i32 }`; `slots_of(PlayerId, Row) -> Vec<ZoneSlot>`;
  `card_at(&GameState, &ZoneSlot) -> Option<&CardInstance>`; `active_units_of(&GameState, PlayerId) ->
  Vec<&CardInstance>`; `pile_at(&GameState, &ZoneSlot) -> Option<&Pile>`; `is_open`, `is_locked`,
  `is_reserved(&GameState, &ZoneSlot) -> bool`; `row_size(Row) -> i32`; `carrier_zones_for(&GameState,
  PlayerId) -> Vec<ZoneSlot>`; `why_cannot_carry(&GameState, &ZoneSlot) -> Result<(), EngineError>`;
  `accepts_stack_card(&GameState, &ZoneSlot, <options: Default>)` (passed `Default::default()`);
  `first_free_zone`, `first_entry_zone(&GameState, PlayerId, Row) -> Option<ZoneSlot>`;
  `is_buried`, `is_unit_token(&GameState, &CardInstance) -> bool`;
  `move_to_zone(&mut GameState, &mut CardInstance, OffFieldZone, MoveOptions) -> MoveResult` with
  `OffFieldZone::{Hand, Library, Graveyard, Exile}`, `MoveOptions { position:
  Option<LibraryPosition>, keep_state: Option<bool> }: Default`, `LibraryPosition::At(i32)` (TS
  `"top" | "bottom" | number`); `report_graveyard_landing(&mut EngineSink, &CardInstance, MoveResult)`.
- `layers` (part 2): `unit_has(&GameState, &CardInstance, KeywordKind) -> bool`.
- `params` (part 2): `param_decl_of(&GameState, &str, &str) -> Option<_>`; `param_value(&GameState,
  Option<&CardInstance>, &str, <options: Default>) -> i32`.
- `restrictions` (part 2): `spell_cannot_reach(&GameState, Option<&CardInstance>, &CardInstance) -> bool`.
- `stays` (part 2): `exit_mark(&GameState) -> u32`; `left_field_after(&GameState, u32, &str) -> bool`.
- `own_library` (part 2): `show_to_owner(&mut CardInstance)` (part 1's notes).
- `targeting`: `can_pay_to_target(&GameState, PlayerId, &CardInstance, Option<&str>) -> bool`;
  `targeting_discards_of(&GameState, &CardInstance) -> i32`;
  `why_targeting_discards_unpayable(&GameState, PlayerId, i32, &[String]) -> Result<(), EngineError>`.
- `mana` (part 4 chunk 1): `play_cost(&GameState, &CardInstance) -> i32`; `effective_cost(&GameState,
  &CardInstance, CostOptions)` (passed `Default::default()`, so `CostOptions` by value); `is_x_cost`.
  `mana` calls me as `cost_rules::price_rules_for(state, player, card, |m| modifier_is_live(state, m))
  -> PriceRules { mods: Vec<CostRuleModifier>, auras: Vec<CostAura> }`, `climb_price_rules(price,
  &PriceRules, impl Fn(i32) -> i32) -> ClimbedPrice { price, used: Vec<String> }`, `cost_floor_of(&CardInstance)`.
- `prompts` (part 3): `open_prompt(&mut EngineSink, OpenPromptArgs) -> Option<PendingChoice>` with
  `OpenPromptArgs { player, kind, aim: Option<TargetAim>, prompt, options: Vec<PromptOption>, min:
  Option<i32>, max: Option<i32>, budget: Option<i32>, owner: Option<PlayerId>, resume }` (all ten
  fields written out); `close_prompt(&mut EngineSink) -> Option<PendingChoice>`;
  `hero_option_label(PlayerId, PlayerId) -> String`; `cell_option_label(PlayerId, Row, i32, PlayerId)
  -> String`; `in_offered_order(&PendingChoice, &[Selection]) -> Vec<Selection>`;
  `why_answer_refused(&PendingChoice, &AnswerInput) -> Result<(), EngineError>`; `AnswerInput {
  player_id, choice_id, selection: Vec<Selection> }`; `resume_at(ResumeAtArgs { def_id: String, step:
  String, hook: Option<String>, radiant: Option<bool>, instance_id: Option<String>, data:
  Option<IndexMap<String, Value>> }) -> Resume` (TS anonymous args; name guessed);
  `run_hook_resumable(&mut EngineSink, &CardInstance, &str, RunHookOptions { controller, targets,
  modes, data, exits_from: Option<u32> }) -> bool` (name guessed); `run_resume(&mut EngineSink, Resume,
  ResumeOptions) -> bool` with `ResumeOptions: Default` (`..Default::default()`);
  `run_start_of_game(&mut EngineSink, &CardInstance, PlayerId) -> bool`.
- `work` (part 3): `RUN_MARKS_KEY`; `begin_work_cascade(&mut EngineSink)`; `drain_work(&mut EngineSink)
  -> bool`; `paused(&EngineSink) -> bool`; `owe(&mut EngineSink, Vec<T>)` where `T: From<Resume>` (TS's
  variadic `...items: (Resume | WorkItem)[]`; I call `owe(sink, vec![resume.into()])`).
- `resolve` (part 3): `cast_card(&mut EngineSink, &mut CardInstance, CastOptions)` with `CastOptions {
  data: Option<IndexMap<String, Value>>, .. }: Default`.
- `draw_complete`: `hold_draw(&mut GameState, &str)`, `CAST_ON_DRAW_KEY`.
- `state_check`: `state_check(&mut EngineSink)`.
- `damage`: `deal_damage(&mut EngineSink, DamageArgs { source: None, target: DamageTarget::Hero {
  player }, amount, flags: None }) -> i32` (`DamageTarget` variant shape guessed).
- `subsystems::copied_text` (part 8): `copies_text(&CardInstance) -> bool`; `copied_chooses_x`,
  `copied_casts_on_draw(&GameState, &CardInstance) -> bool`; `text_face_of(&GameState, &CardInstance)
  -> CardInstance`.
- Callers of mine whose TS signatures I changed (part 31 aligns them): `play_choices::declared_targets`,
  `declared_modes`, `tribute_cost_of`, `may_tribute_enemy_units` take `(state, card)` (TS took the card
  alone; the script lookup needs the state). `legal_zones_for`/`default_zone_for` take `tributes:
  &[String]` (TS defaulted it). `why_x_refused(.., most: Option<i32>)`; `targeting_decls_of`,
  `targeting_discards_required`, `in_declared_order(.., declared: Option<&[TargetDecl]>)`;
  `play_choice_combinations(.., declared: Option<&DeclaredChoices>)`.
- `PlayAction` is defined in both `play_choices` (mine) and `play_steps` (TS duplicate). `lib.rs`
  resolves the root to `play_choices::PlayAction`; part 31 should make `play_steps` use this one.

## Decisions
- Refusals: every TS `string | null` refusal (`why_play_banned`, `why_graveyard_play_refused`,
  `why_x_refused`, `why_choices_refused`, `why_declared_choices_refused`, the private `refuse_*`, the
  Cry answerer) is `Result<(), EngineError>` with TS's text verbatim (SURFACE §4.4.9, §16
  `error.style`). The private `refuse_*` chain with `?`, which is TS's `??` chain.
- `PlayAction` is a struct with the eight fields of `ActionBody::Play`, serialised through
  `#[serde(into/try_from = "ActionBody")]` so its JSON is TS's (`"type": "play"` included);
  `From<PlayAction> for ActionBody`, `TryFrom<ActionBody>`, `into_body()`, `from_body(&ActionBody)`.
  `legal_actions` needs `.into()` on each. `PlayChoices { targets, modes }` and `DeclaredChoices {
  targets: Vec<TargetDecl>, modes: Vec<ModeDecl> }` are owned structs.
- `CostRuleModifier { id, expiry, rule }` with `CostRuleModifier::of(&PlayerModifier)`; `PriceRules`,
  `ClimbedPrice` named for the two anonymous TS answers. `cost_rule_modifier_label(&PlayerModifier)`
  (viewFor passes the modifier, as in TS).
- `GraveyardGrant { source: CardInstance (cloned), permission }`; `PlayPayment { plague:
  Option<PlagueSpend> }` (serde as TS's `{ plague? }`). `spend_plague_tokens(sink, &CardInstance,
  tokens)` writes the live card it finds by id.
- draw: `add_to_hand(sink, &mut CardInstance) -> AddToHandOutcome { Hand, Burned }`,
  `shuffle_into_library(sink, &mut CardInstance, existing, copy_of: Option<&str>) -> ShuffleInOutcome
  { Library, Dropped }`, `DrawOutcome` enum, `complete_draw(sink, player, CardInstance (by value),
  Option<ChainLinkOrCount>)`, `draw_one(sink, player, Option<ChainLinkOrCount>)`, `draw(sink, player,
  i32) -> Vec<DrawOutcome>`, `cast_dealt_card(sink, &CardInstance) -> bool` (removes the hand's copy
  by id and casts that). `ChainLinkOrCount { Link(ChainLink), Count(i32) }` is TS `ChainLink | number`
  (`From` both). `shuffle_into_library` calls `show_to_owner` on the card now in the library (by id)
  and on the caller's copy.
- `stopped`'s TS `pending !== before` (object identity) compares the open prompt's id: ids `q<n>` are
  never reused.
- cry_trigger: `CryPlace` is a plain serde enum (not `string_union!`, which would export a TS type to
  the web's generated files); `CryRun` is a private serde struct in TS's field order, `awaiting`
  serialised `null` (not skipped) as TS writes it; `run_of` re-reads it defensively (§4.4.10).
- Private copies (fullsend rule 5): `script_of_card` (TS `scriptOf(instance)`: Vanilla → empty
  script, else the face) in all five files; the `returnAfterResolve` / `castOnDraw` enchantment reads in
  `cost_rules`/`draw` (so I do not guess `enchantments`' kind-argument type).
- `declaration_slices` → `Vec<usize>`; `stored_declaration_slices` → `Option<Vec<usize>>` (any
  non-number entry → `None`, as TS).
- `cross_product` keeps TS's index-keyed dedupe; the non-`forModes` cross of target and mode answers
  is crossed over their indices (identical selection, since the cross picks by index alone).
  `Number.MAX_SAFE_INTEGER` ranks are `usize::MAX`; `in_declared_order` sorts stably by (rank, at).
- `SHEEP_TOKEN_INDEX` stays in `play_choices` (a string key, not a rules number).
- TS's `Number.isInteger` checks on X, a lane and Plague tokens are dropped (the values are `i32`); each
  site says so.
