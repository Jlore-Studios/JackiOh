# Slice: part 3 (engine 2), chunk 2 of 3 — modifiers, prompts, resolve, state check, targeting
BUILDS-RUN: 0

## FILES
Full ports, every TS function in TS order with its doc and rule comments:
- `crates/engine/src/modifiers.rs` ← `packages/engine/src/modifiers.ts`
- `crates/engine/src/prompts.rs` ← `packages/engine/src/prompts.ts`
- `crates/engine/src/resolve.rs` ← `packages/engine/src/resolve.ts`
- `crates/engine/src/state_check.rs` ← `packages/engine/src/stateCheck.ts`
- `crates/engine/src/targeting.rs` ← `packages/engine/src/targeting.ts`

Not ported, by SURFACE §6.6 (registration hooks go): `registerPromptAnswerer`, `registerTargetingHooks`
and its `TargetingHooks` type, `registerCastDriver` and its `CastDriver` type, the
`registerDefaultWorkHandler(...)` and `registerWorkHandler(DEATHS_WORK, ...)` module-scope calls. Their
replacements are below (Decisions). `STATE_CHECK_PASS_CAP` and `MAX_PROMPT_ANSWERS` are read from
`crate::config` (part 1 moved them), not redefined.

## SURFACE
Matched §4.1 paths, §4.2 names, §4.3 types, §6.5 sink. Part 1 wins where it differs from SURFACE:
`EngineSink`/`EffectContext` from `script.rs` (re-exported as `crate::resolve::EngineSink`), hooks take
`&mut EffectContext`, `ReplacementDef`/`TargetedReplacement` from `script.rs`, `EngineError` from `state.rs`.

## DEPENDS-ON (names called, expected from other parts, with the shapes assumed)
- part 2 `scripts`: `script_of(&GameState, def_id: &str) -> CardScripts` (part 2 brief step 3). TS's
  `scriptOf(instance)`/`flagsOf(instance)` (face + Vanilla guard) are private `face_script` copies here.
- part 2 `catalog::fused_id_parts(&str) -> Option<Vec<String>>`; `layers::unit_view(&GameState,
  &CardInstance)` returning a struct with `attack`, `max_health`, `health: i32`, `keywords: Vec<Keyword>`,
  `position: Position`; `faces::card_type_of(&GameState, &CardInstance) -> CardType`;
  `tuning::x_of(&CardInstance) -> i32`; `restrictions::spell_cannot_reach(&GameState,
  Option<&CardInstance>, &CardInstance) -> bool`; `game_over::end_game(&mut EngineSink, Winner,
  GameOverReason)`; `carriers::settle_carried(&mut EngineSink)`.
- `zones` (part 2): `ZoneSlot { player, row, lane: i32 }` (struct literal; an alias of `ZoneRef` also
  works), `OffFieldZone::Graveyard`, `MoveResult::Replaced`, `PlaceOnFieldOptions { stack: Option<bool> }`,
  `is_buried(&GameState, &CardInstance) -> bool`, `first_entry_zone(&GameState, PlayerId, Row) ->
  Option<ZoneSlot>`, `is_unit_token(&GameState, &CardInstance) -> bool`,
  `remove_from_field(&mut GameState, &CardInstance, opts) -> bool`,
  `move_to_zone(&mut GameState, &mut CardInstance, OffFieldZone, opts) -> MoveResult` (opts passed as
  `Default::default()`), `place_on_field(&mut GameState, &mut CardInstance, &ZoneSlot,
  PlaceOnFieldOptions) -> bool`, `report_graveyard_landing(&mut EngineSink, &CardInstance, MoveResult)`,
  `reset_instance(&mut CardInstance)`. The instance-moving pair takes `&mut CardInstance` because TS
  moved the object it was handed (a card already taken off the field is in no pile).
- `work` (part 3, another chunk): `PausedStep { from: usize, targets: Vec<Selection>, modes: Vec<String>,
  part: Option<Vec<usize>>, memo: Option<Vec<Value>>, exits_from: Option<u32>, chosen_from: Option<u32>,
  summoned: Option<Vec<String>>, resolving: Option<bool>, event_stay: Option<EventStay> }` (built by
  struct literal; `Serialize` with skip-None), `RunMarks { exits_from: Option<u32>, summoned:
  Option<Vec<String>>, resolving: Option<bool>, event_stay: Option<EventStay> }` (`Serialize` with
  skip-None: it is written into `data[RUN_MARKS_KEY]`), `WorkPlan { resume: Resume, owner: PlayerId }`,
  `PAUSE_KEY`, `RUN_MARKS_KEY: &str`, `begin_work_cascade(&mut EngineSink)`, `card_data(&IndexMap) ->
  IndexMap`, `drain_work(&mut EngineSink) -> bool`, `park_work(&mut EngineSink, &WorkPlan, PausedStep)`,
  `paused_of(&IndexMap) -> Option<PausedStep>`, `run_marks_of(&IndexMap) -> Option<RunMarks>`,
  `script_step_for(&Script, &Resume) -> Option<Hook>`, `push_work(&mut EngineSink, Resume,
  Option<PlayerId>)`.
- `work.rs`'s dispatcher must route `"@deaths"` to `crate::state_check::run_owed_deaths(sink, &item)` and
  its default arm to `crate::prompts::default_work_handler(sink, &item)` (or `run_resume` with
  `controller: Some(item.owner)`, the same thing).
- `targeting_point` (part 3, another chunk): `targetable_options(&GameState, &OpenPromptArgs) ->
  Vec<PromptOption>` (private in TS: must be `pub`), `why_target_answer_refused(&GameState,
  &PendingChoice, &[Selection]) -> Result<(), EngineError>`, `target_answer(&mut EngineSink,
  &PendingChoice, &[Selection]) -> Vec<Selection>`.
- answerers, each `fn(&mut EngineSink, &AnswerInput) -> Result<(), EngineError>` and `pub` (all but the
  first are private in TS): `play_steps::answer_play_prompt`, `effects::plague::answer_placement`,
  `effects::fuse::answer_fuse_onto`, `cry_trigger::answer_cry_prompt`.
- `play_steps::cast_through_pipeline(&mut EngineSink, &CardInstance, CastOptions)` (private in TS: must be `pub`).
- `random_cast::cast_mode_for_prompt(&GameState, PlayerId, Option<&str>)` answering an `Option` of a
  struct with `random: bool`, `target_enemies: bool`; `random_cast::prefer_enemies` /
  `prefer_friends(&GameState, PlayerId, &[T], impl Fn(&T) -> Selection, required: i32) -> Vec<T>`.
- `replacements::would_die_window(&mut EngineSink, &[CardInstance]) -> Vec<CardInstance>`;
  `subsystems::quests::notice_quests(&mut EngineSink)`.

## GAPS
- The private TS functions above (`targetableOptions`, `answerPlacement`, `answerFuseOnto`,
  `answerCryPrompt`, `castThroughPipeline`) are called here and must be `pub` in their Rust modules.
- `EngineSink::reborrow` (part 1) copies `converting`/`dry_running` into the nested sink; a change made
  through a context built by `make_context` does not flow back to the outer sink. TS's module `let`s
  were global, so a counter raised inside a nested context and read outside it would differ.
  Part 31: check `replacements.rs`'s use of `converting` across `make_context`.
- `ctx.self_` is a snapshot (part 1's design): TS's `resumeSelf` read the live `self.id`, which
  `zones.freshFaceDownId` can change mid-run (R227). The snapshot's id is used.
- `placeOnField`'s options struct name (`PlaceOnFieldOptions`) is a guess; TS's was anonymous.
- `owe(sink, resume)` (variadic over a union in TS) is called as its exact equivalent for one
  `Resume`, `push_work(sink, resume, None)` (owner = the active player).

## Decisions
- The resumable runners take the context alone: `apply_resumable(ctx, &plan, effects, paused)` and
  `run_resumable_list(ctx, &plan, effects, paused) -> ListStatus`. TS passed `(sink, ctx, …)`; the
  context is that sink (it derefs to it) and holds its borrow, so a second `&mut` cannot be passed.
- `make_context<'b>(&'b mut EngineSink, Option<&CardInstance>, HookOptions) -> EffectContext<'b>`
  (copies the instance in; `HookOptions` is all-`Option`, `Default`).
- Refusal-string functions return `Result<(), EngineError>` with TS's text (SURFACE §4.4.9):
  `why_answer_refused`, `answer_prompt(sink, &AnswerInput)`, `why_targeting_discards_unpayable`.
- The answerer registry is `has_answerer`/`answer_owned`: a `match` on the literal hooks `"play"`,
  `"plague:placement"`, `"fuse:onto"`, `"@triggerCry"`. The targeting point is three direct calls; the
  cast driver one direct call.
- `add_modifier(sink, player, expiry, kind)`: TS's `DistributiveOmit<PlayerModifier, "id">` is the
  modifier's `expiry` and `kind`. `due_delayed` and `due_start_of_turn_effects` answer clones;
  `turn_ends_of` answers `Option<&PlayerModifier>`; `cut_turn_short` writes the rider back by id.
  `due_delayed` takes `Phase` (`Start`/`End`).
- New names for TS's anonymous types: `ResumeAtArgs` (`resumeAt`'s argument), `HookInstance`
  (`runHookResumable`'s `{ id, defId, controller, radiant }`, with `From<&CardInstance>`; the function
  takes `impl Into<HookInstance>`), `HookResumableOptions`, `CastAfterward` (`afterward: "exile"`),
  `RebornEntry`, `CollectedEntry` (the pass's list items), `DeathCause` (private), `default_work_handler`
  (TS's anonymous default handler). `HookName`, `ListStatus` are plain enums (not `string_union!`, which
  exports TS types). `RebornFace` is `pub` because `DeathPass` is.
- `resume_of(&Resume)` takes the holder's `resume` (TS took the holder); `prompt_owner_of` and
  `continue_answer` take `&PendingChoice`; `interposes_from_hand(state, card)` takes the state (a
  script is looked up through it, SURFACE §6.6).
- TS identity comparisons: "a prompt this effect opened" is a pending id that differs from the one open
  before (ids are minted from `nextId`); `expireModifiers`' kept set is a keep flag per modifier.
- TS walked and wrote live instances. Here a card is read as a copy and written back through
  `find_instance_mut` by id; `collect` re-reads each dying card from the state before reading it, so a
  stale copy handed in reads as the live card. Reborn: the graveyard's own card gets TS's in-place
  changes (no Reborn, not Vanilla, the X/X face) even when no zone takes the body, as TS's shared object
  did; on a return the graveyard copy is removed before the body's flags are set, so `find_instance_mut`
  reaches the body on the field.
- The parked pass stores its zone as `ZoneRef` (`{ player, row, lane }`, the same JSON as TS's
  `ZoneSlot`), so the pass serialises whatever `zones::ZoneSlot` derives; `zones` calls get a `ZoneSlot`
  built from it.
- Private copies on purpose (rule 5): `scriptOf`/`flagsOf` (`face_script`), `zones.activeUnitsOf`
  (`units_of`), `plague.permanentsOnField` (`permanents_in_play`), `zones.slotOf`, `cardAt`, `isReserved`,
  `reserveZone`, `releaseZone`, `stays.exitMark`.
- Memo levels park as `Value` (`None` as `null`, as TS's JSON round trip wrote `undefined`), and are
  handed back as `Some(value)`; every TS `expand` reads its memo with `typeof`, so `null` and absent agree.
- `answer_at_random` keeps TS's probe without a budget, and draws once from `sink.rng` when any answer
  exists, as TS did.
