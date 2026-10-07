# Slice: part 4 (engine 3), chunk 1 of 2 — the play pipeline, Echo, mana, play counts, random casts
BUILDS-RUN: 0

## FILES
- `crates/engine/src/echo.rs` ← `packages/engine/src/echo.ts` (whole)
- `crates/engine/src/mana.rs` ← `packages/engine/src/mana.ts` (whole)
- `crates/engine/src/play_counts.rs` ← `packages/engine/src/playCounts.ts` (whole)
- `crates/engine/src/play_steps.rs` ← `packages/engine/src/playSteps.ts` (whole)
- `crates/engine/src/random_cast.rs` ← `packages/engine/src/randomCast.ts` (whole)

Every TS function, exported or not, is ported in TS order with its doc comment and every comment that
states a rule or cites a ruling. Not ported, by SURFACE §6.6: `registerWorkHandler(PLAY_WORK_KIND, …)`,
`registerCastDriver(…)` and `registerPromptAnswerer(PLAY_WORK_KIND, …)` in playSteps.ts; their functions
are `pub` for the dispatchers (below). No `todo!`, `unimplemented!` or `// TODO`.

## SURFACE
§4.1 paths, §4.2 names, §4.3 types, §4.4 semantics (stable order everywhere; `IndexSet`/`IndexMap` for
every Set/Map; RNG draws in TS's order: `random_picks` draws `int` then `shuffle`, `ask_repeat_modes`
draws `pick`, nothing hoisted), §6.5 (`&mut EngineSink` for sink mutators). Part 1's frozen types are
used, never redefined: `state::{CastMode, AnnounceRecord, PlayRecord, FaceUpRecord, GameLog, EchoItem,
PromptOption, ModifierKind, ModifierExpiry, EngineError}`, `script::{EngineSink, HookArgs, CostArgs,
StaticFlags, FlagOrCount}`, `wire::{PlagueSpend, Selection, Enchantment, …}`, `config::{MIN_CHOSEN_X,
QUICKSTRIKER_COMBO_MULTIPLE (ByFace::on), RANDOM_CAST_CHAIN_CAP, LAST_FACE_UP_SKIPPED_TAGS, GLITCH_DEF_ID,
TUNE_MIN_AMOUNT}`. `play_steps` does `pub use crate::play_choices::PlayAction;` (one type for both
modules, as part 4.2 asks; the root globs then agree).

## DEPENDS-ON (dispatchers that must call my functions; no registration, SURFACE §6.6)
- `work.rs` `"play"` arm → `play_steps::run_owed_play(&mut EngineSink, &WorkItem)`.
- `prompts.rs` `"play"` answerer → `play_steps::answer_play_prompt(&mut EngineSink, &AnswerInput) ->
  Result<(), EngineError>`.
- `resolve::cast_card` → `play_steps::cast_through_pipeline(&mut EngineSink, &CardInstance, CastOptions)`
  (options by value, as part 3.2 calls it).
- `reduce.rs` / `subsystems::scorer` → `play_steps::run_play_steps(&mut EngineSink, PlayerId,
  &PlayAction) -> Result<(), EngineError>`. Part 5.2 wrote `&ActionBody`: part 31 converts at that call
  (`PlayAction::from_body`, part 4.2's).
- What my modules offer others (their shapes): `mana::effective_cost(&GameState, &CardInstance,
  CostOptions) -> i32` (TS's three arguments; `CostOptions { as_play: Option<bool> }: Default + Copy` —
  part 5.2 called it with two, part 6.1 with an `Option`: part 31 aligns those call sites),
  `play_cost`, `cost_now`, `can_afford`, `printed_cost`, `is_x_cost(&GameState, &CardInstance)`,
  `modifier_is_live(&GameState, &PlayerModifier)`, `max_mana_for(&PlayerState)`, `refresh_mana`,
  `gain_mana`, `spend_mana`, `refresh_some_mana(&mut PlayerState, i32)`, `mana_event(PlayerId,
  &PlayerState) -> GameEvent`, `NEXT_REFRESH_MODIFIER_ID`, `cost_rules_spent_by -> Vec<String>`;
  `echo::{echo_grant_of(&GameState, PlayerId, &PlayerModifier) -> i32, exile_on_landing(&mut
  CardInstance), EXILE_ON_LANDING, printed_echo(&CardInstance, &GameState) (state required),
  queue_echo_repeats, granted_echo, add_echo_repeats, take_echo_repeat, drop_echo_repeats,
  echo_repeats_owed, ResolvedCard, land_after_resolution}`; `play_counts::{play_record_of(&GameState,
  &CardInstance) -> Option<PlayRecord>, record_play(&mut GameState, PlayerId, &CardInstance)}`;
  `random_cast::{cast_modes_of(&GameState) -> &[CastMode], with_cast_mode, random_cast_of(&GameState,
  PlayerId) -> Option<&CastMode>, may_cast_now, count_chain_cast(&mut GameState, PlayerId),
  CastPromptMode { random, target_enemies }, cast_mode_for_prompt(&GameState, PlayerId, Option<&str>) ->
  Option<CastPromptMode>, is_target_pick(&Selection), is_enemy_pick, is_friendly_pick,
  prefer_enemies/prefer_friends<T: Clone>(&GameState, PlayerId, &[T], impl Fn(&T) -> Selection, i32) ->
  Vec<T>, random_picks<T: Clone>(&mut Rng, &[T], i32, i32) -> Vec<T>}`; `play_steps::{PlayRun,
  RepeatRecord, PlayStepName, PLAY_STEPS, ResolvePart, RESOLVE_PARTS, Awaiting, PlaySource,
  PLAY_WORK_KIND, is_play_resume(&Resume), run_of(&Resume) -> Option<PlayRun>, validate_play ->
  Result<PlayRun, EngineError>, is_permanent, take_from_play_source(&mut GameState, &PlayRun, &mut
  CardInstance) -> bool}`.

## GAPS
Functions not ported: none.

Names I call that other parts provide (module path; the shape I assumed). Where an owner's notes were
already on `staging` I followed them (marked "owner"); the rest are guesses for part 31:
- `catalog` (part 2): `def_of(Option<&GameState>, &str)` (owner; read for `.name`, `.cost`, `.type_`,
  `.tags`, owned or borrowed); `fused_id_parts(Option<&GameState>, &str) -> Option<Vec<String>>` (owner).
- `scripts` (part 2): `script_of(&GameState, &str) -> CardScripts` (part 2's brief). TS's
  `scriptOf(instance)`/`flagsOf(instance)` (Vanilla → empty, else the face) are private copies here.
- `faces::card_type_of(&GameState, &CardInstance) -> CardType`; `animated::animate_on_entry(&mut
  EngineSink, &CardInstance) -> bool` (owner).
- `announce`: `begin_announce(&mut GameState, AnnounceRecord)`, `end_announce(&mut GameState, &str) ->
  Option<AnnounceRecord>`, `is_announce_live(&GameState, &str) -> bool`.
- `draw_complete`: `CAST_ON_DRAW_KEY: &str`, `release_draw(&mut GameState, &str)`.
- `stays`: `exit_mark(&GameState) -> u32`, `left_field_after(&GameState, u32, &str) -> bool`.
- `zones` (part 2; its owner's notes were not on `staging`): `ZoneSlot { player, row, lane: i32 }` that is
  `Copy` and serde (an alias of `wire::ZoneRef` is ideal: it rides `PlayRun.zone` in JSON);
  `OffFieldZone::{Graveyard, Exile}`, `MoveResult::Moved`, `MoveOptions: Default`,
  `move_to_zone(&mut GameState, &mut CardInstance, OffFieldZone, MoveOptions) -> MoveResult`,
  `place_on_field(&mut GameState, &mut CardInstance, &ZoneSlot, PlaceOnFieldOptions { stack:
  Option<bool> }) -> bool` (the options name is part 3.2's guess, adopted so the two agree),
  `report_graveyard_landing(&mut EngineSink, &CardInstance, MoveResult)`, `remove_from_any_zone(&mut
  GameState, &CardInstance)`, `cease_to_exist(&mut GameState, &CardInstance)`, `fresh_face_down_id(&mut
  GameState, &mut CardInstance) -> String`, `card_at(&GameState, &ZoneSlot) -> Option<&CardInstance>`,
  `first_free_zone(&GameState, PlayerId, Row) -> Option<ZoneSlot>`, `is_open`, `lands_face_down(&GameState,
  &CardInstance, Row)`, `release_zone`/`reserve_zone(&mut GameState, &ZoneSlot)`, `slots_of(PlayerId, Row)`.
  After every call that moves or places a card I re-read the card by id, so either an in-place or a
  copy-taking implementation works once the argument types match.
- `cost_rules` (part 4.2, owner): `why_play_banned(&GameState, PlayerId, &CardInstance, i32) ->
  Result<(), EngineError>`, `cost_floor_of(&CardInstance) -> i32`, `price_rules_for(&GameState, PlayerId,
  &CardInstance, live)` (I pass `&closure`, which is an `impl Fn` too) `-> PriceRules`,
  `climb_price_rules(i32, &PriceRules, &curvature) -> ClimbedPrice { price, used }`.
- `graveyard_play` (part 4.2): `in_own_graveyard(&GameState, PlayerId, &CardInstance) -> bool`,
  `playable_from_graveyard(&GameState, &CardInstance) -> bool`, `mana_due(i32, Option<&PlagueSpend>) ->
  i32`, `spend_plague_tokens(&mut EngineSink, &CardInstance, i32)` (owner),
  `why_graveyard_play_refused(&GameState, PlayerId, &CardInstance, i32, Option<&PlagueSpend>) ->
  Result<(), EngineError>`.
- `draw` (part 4.2, owner): `add_to_hand(&mut EngineSink, &mut CardInstance)`, `draw(&mut EngineSink,
  PlayerId, i32) -> Vec<DrawOutcome>`.
- `play_choices` (part 4.2, owner): `PlayAction` (struct, the eight `ActionBody::Play` fields),
  `DECLARATION_SLICES_KEY`, `declaration_slices(..) -> Vec<usize>`, `declared_targets`/`declared_modes(
  &GameState, &CardInstance)`, `default_zone_for(.., &[String])`, `in_declared_order`/`targeting_decls_of`/
  `targeting_discards_required(.., declared: Option<&[TargetDecl]>)` (I pass `None`), `why_choices_refused
  -> Result<(), EngineError>`, `chooses_x`, `resolving_face -> CardInstance`, `play_made_radiant`,
  `plays_on_stack`, `play_uses(&CardInstance, &[Selection]) -> Vec<String>`, `active_target_decls(&[TargetDecl],
  &[String]) -> Vec<TargetDecl>`, `targets_follow_modes(&[TargetDecl])`, `legal_selections_for(&GameState,
  PlayerId, &CardInstance, &TargetDecl) -> Vec<Selection>`, `interceptor_fits_decl(&GameState, PlayerId,
  &CardInstance, &TargetDecl, &CardInstance) -> bool`. `targeting_decls_of -> Vec<Option<TargetDecl>>`
  (guess).
- `modifiers` (part 3): `remove_modifier(&mut EngineSink, PlayerId, &str)`, `install_lasting_modifiers(&mut
  EngineSink)`.
- `damage` (part 3.1, owner): `deal_damage(&mut EngineSink, DamageArgs { source: Option<CardInstance>,
  target: DamageTarget::Hero { player }, amount, flags: None })`.
- `prompts` (part 3.2, owner): `AnswerInput { player_id, choice_id, selection }`, `OpenPromptArgs { player,
  kind, aim, prompt, options, min, max, budget, owner, resume }`, `open_prompt -> Option<PendingChoice>`,
  `close_prompt`, `hero_option_label(PlayerId, PlayerId)`, `cell_option_label(PlayerId, Row, i32, PlayerId)`,
  `in_offered_order(&PendingChoice, &[Selection]) -> Vec<Selection>`, `why_answer_refused(&PendingChoice,
  &AnswerInput) -> Result<(), EngineError>`, `run_hook_resumable(&mut EngineSink, impl Into<HookInstance>
  (I pass &CardInstance), &str, HookResumableOptions { controller, targets: Option<Vec<Selection>>, modes:
  Option<Vec<String>>, data: Option<IndexMap<String, Value>>, exits_from: Option<u32> })` (the field set is
  my guess). TS's `resumeOf(pending)` is not called: a typed `Resume` needs no defaults.
- `resolve` (part 3.2): `CastOptions { targets, modes, data, random, target_enemies, afterward:
  Option<CastAfterward>, .. }` (fields flat; `afterward` read with `is_some()`, its one literal),
  `HookName::OnPlayHook`, `MANA_BEFORE_PLAY_KEY`, `flag_return_to_hand_at_end_of_turn(&mut GameState, &str)`.
- `state_check` (part 3.2): `state_check(&mut EngineSink)`, `sacrifice_together(&mut EngineSink,
  &[CardInstance])`.
- `targeting::target_aim(&TargetDecl) -> TargetAim`.
- `targeting_point` (part 3.1, owner): `InterceptArgs { chooser, picks: Vec<Selection>, targeting:
  Option<&dyn Fn(usize) -> bool>, accepts: Option<&dyn Fn(&GameState, &CardInstance, usize) -> bool>, what:
  None, source: Option<CardType> }`, `intercept_targeting -> Vec<Selection>`, `pay_targeting_discards(&mut
  EngineSink, PlayerId, i32, &[String])`, `target_answer`, `why_target_answer_refused -> Result<(), EngineError>`.
- `triggers` (part 3.3, owner where noted): `OWED_TO_TRAPS`, `SETTLE_PASS_CAP` (owner, in triggers.rs),
  `SettleOptions { hold_check: Option<bool> }: Default`, `settle(&mut EngineSink, SettleOptions)` (owner),
  `dispatch_pending(&mut EngineSink)`, `event_of_queued(&QueuedTrigger) -> Option<GameEvent>` (or `&`),
  `run_queued_trigger(&mut EngineSink, &QueuedTrigger)` (owner), `TriggerHolder { card, controller, script,
  .. }` (owned), `trigger_holder_for(&GameState, &CardInstance) -> Option<TriggerHolder>`,
  `trigger_holders_with_hook(&GameState, HookName, Option<PlayerId>) -> Vec<TriggerHolder>`.
- `work` (part 3.1, owner): `begin_work_cascade`, `drain_work`, `drop_work(&mut GameState, impl
  Fn(&WorkItem) -> bool)`, `paused(&EngineSink)`, `paused_of(&IndexMap<String, Value>) -> Option<PausedStep>`
  (`.targets: Vec<Selection>`), `push_work(&mut EngineSink, Resume, Option<PlayerId>)`.
- `subsystems::copied_text` (part 8.2, owner): `copies_text(&CardInstance) -> bool`, `copied_text_of(&GameState,
  &CardInstance) -> Option<PlayRecord>`, `fix_copied_text(&mut GameState, &str, Option<Option<PlayRecord>>)`
  (by id, after the card is in the resolving zone, TS's order), `text_face_of(&GameState, &CardInstance) ->
  CardInstance`, `copied_echo(&GameState, &CardInstance) -> i32` (guess).
- `subsystems::glitch::count_system_play(&mut GameState, &CardInstance)`.
- Formatting: some lines run past rustfmt's 110 columns (rustfmt was not run, rule 1); `cargo fmt`
  reflows them in Wave 3.

## Decisions
- **Live objects.** TS passed live `CardInstance`s and wrote through them. Here a card is a copy
  (`snapshot`) handed to calls as `&CardInstance` (or `&mut` where the owner said the call moves it),
  written back through `find_instance_mut` by id, and re-read by id (`live`) after every call that could
  have moved or changed it, at each point TS read the object again. Where TS's `removeFromAnyZone` wrote
  onto the object it was handed (it deletes `returnToHandAtEndOfTurn` on a card leaving a graveyard, R155),
  the same write is made on my copy (`take_from_play_source` takes `&mut CardInstance`; `cast_through_pipeline`).
- **`PlayRun`** is a serde struct (camelCase) stored as JSON under `resume.data["__play"]`, presence exactly
  TS's: `x?:` → `Option` + skip; `x: T | null` (`zone`, `repeat`, `awaiting`) → `Option` serialised `null`;
  `copied?: PlayRecord | null` → `Option<Option<PlayRecord>>` with a `deserialize_with` that keeps `null`
  apart from absent. Required fields read with `#[serde(default)]` (lenient as TS's `runOf`, which checked
  only `instanceId` and `at`), always written. Cursors (`at`, `hookAt`, `resolveAt`, `declAt`, `modeAt`,
  `partAt`) are `usize`; marks (`exitsFrom`, …) `u32`; `targetSlices` `Vec<usize>` (part 4.2's type).
  `run.plague` reuses `wire::PlagueSpend` (same shape). `resume_for` writes a whole copy with `at` set,
  which is TS's `{ ...run, at }`.
- `PlayStepName`, `ResolvePart`, `Awaiting`, `PlaySource` are plain serde enums (not `string_union!`,
  which would export TS types to the web's generated directory). `STEP_TABLE` is a `const` of fn pointers
  (internal steps, not hooks).
- `with_cast_mode(&mut EngineSink, Option<CastMode>, impl FnOnce(&mut EngineSink) -> T) -> T`: TS took the
  state and a closure over the caller's sink; the sink is the one borrow both can share. Its only caller
  is `play_steps::drive`. No `finally`: a panic discards the state anyway.
- `printed_echo(card, state)`: TS's `state` was optional; every caller passed it and Rust needs it for the
  script lookup, so it is required.
- `effective_cost` keeps TS's three arguments (`CostOptions` by value, `Default`), which most callers'
  notes use; `play_cost`, `can_afford`, `cost_now` pass `CostOptions::default()` / `as_play`.
- Refusals (`validate_play`, `run_play_steps`, `answer_play_prompt`) are `Result<_, EngineError>` with
  TS's text verbatim; called refusals (`why_choices_refused`, `why_play_banned`, `why_graveyard_play_refused`,
  `why_answer_refused`, `why_target_answer_refused`) are read with `?`.
- Optional/default parameters: a trailing TS default computed from the other arguments → `Option`
  (`None` passed); a default `[]` → a slice; a default `{}` options object → a `Default` struct. TS's
  private defaults (`castChoicesMade`'s `step = "resolve"`, `askRepeatChoices`' `step = "echo"`) are passed
  explicitly at every call.
- Private copies (fullsend rule 5): `scriptOf`/`flagsOf` (`running_script`, `flags_of_card`) in four
  files; `tuning.tunedCount` for Echo (`tuned_echo`, floor `TUNE_MIN_AMOUNT` = TS's `TUNED_FLOOR`);
  `timesPlayed.countPlay`; `enchantments.hasEnchantment` for the two kinds read (so I do not guess the
  kind argument's type); `query.playedEarlier`'s string-id branch (`played_earlier_this_turn`).
- `fileSelection`'s `Number.parseInt(option, 10)` is a small `parse_int` (leading spaces, sign, digit
  prefix; `None` for TS's `NaN`).
- `selectionIn` deserialises a raw `selection`/`targets` array into `Vec<Selection>` (TS cast it unchecked;
  a malformed array reads as none).
- `runAnnounceWindow`'s throw is `panic!` with TS's message (SURFACE §4.4.9).
- `cast_through_pipeline` uses the instance it is handed (the caller's copy is TS's live object), and
  `fix_copied_text`, `exile_on_landing` and the random X are written to the card in the state by id.
- `land_after_resolution`'s `cardResolved.radiant` is always `Some` (TS always set it); `arrivedDuring`
  only when non-empty; `exitsFrom` always.
- `record_play` computes the play record after the log writes, as TS (a `recordsPlayAs` hook reads the
  state then); `playedByTag` counts each of the def's tags once in first-seen order (`new Set`).
