# Slice: part 8 (engine 7: subsystems), chunk 3 of 3 (#396)
BUILDS-RUN: 0

## FILES
All seven are full ports of their TS files, every function in TS order (exported or not), the header
as `//!`, every comment that states a rule or cites a ruling (the R-ids of each Rust file equal its TS
file's, checked by script; quests.rs carries `QuestGoal`'s doc, R541 included, above its re-export):
- `crates/engine/src/subsystems/activate.rs` ← `activate.ts`
- `crates/engine/src/subsystems/board_history.rs` ← `boardHistory.ts`
- `crates/engine/src/subsystems/combo_index.rs` ← `comboIndex.ts`
- `crates/engine/src/subsystems/last_boards.rs` ← `lastBoards.ts`
- `crates/engine/src/subsystems/papaya.rs` ← `papaya.ts`
- `crates/engine/src/subsystems/quests.rs` ← `quests.ts`
- `crates/engine/src/subsystems/twice_forward.rs` ← `twiceForward.ts`
No `todo!`, `unimplemented!` or `// TODO`. Nothing else touched but these notes.

## SURFACE
Matched §4 (paths, names, types), §4.4 (stable order, insertion-ordered maps, presence, RNG order:
every `pick`/`shuffle` where TS drew it), §6.5 (`&mut EngineSink`), §6.6 (effects as `Effect::new`
closures, hooks via `hook`). Part 1 wins: hooks take `&mut EffectContext`; `QuestGoal`, `QuestDef`,
`QuestRewardDef`, `QuestBook` are script.rs's and only `pub use`d from quests.rs; `EngineError` and
`EngineSink` from state.rs/script.rs. Constants from `crate::config` (`FIRST_GRADE`, `GRADE_D_*`,
`GRADE_A_DAMAGE`, `FUSED_MIN_PARTS`, `PAPAYA_*`, `BOARD_HISTORY_DEPTH`, `ACTIVATE_UNLIMITED_CAP`); none
redefined. `LAST_GRADE` (= `GRADES.len()`) and `PAPAYA_LANES` (= `UNIT_ZONES`) are derived here, as TS.

## GAPS

### Needs a change to part 1's frozen files (part 31)
- **`script::TriggerWhen` must take `&mut EffectContext`** (and `TriggerDef::with_when` likewise; the
  callers in traps.rs/triggers.rs pass `&mut ctx`). C+ #74's predicate (twiceForward.ts `when`) writes
  its card's memory (`plays`, `fuseOn`), reveals the card and applies `gain_brittle` while it declines —
  the only part of a trap that runs without firing it (R99). `twice_forward_trigger` builds its
  `TriggerDef` by struct literal with `when: Some(Arc::new(|ctx: &mut EffectContext, event| …))`, which
  does not type-check against today's `&EffectContext` alias. Every other `with_when` closure keeps
  compiling once the alias takes `&mut` (a closure that only reads its `ctx` accepts either).

### Not ported
- `lastBoards.ts` `isEntry`: it guarded TS's `unknown[]` input. Part 1's `LastBoardInput` is typed
  `(Vec<LastBoardEntry>, Vec<LastBoardEntry>)`, so every entry already is one. Behaviour difference at
  the boundary only: a malformed stored last board is refused by serde where TS dropped the bad entries.

### Names called in other parts' modules (TS name snake_cased at its TS path; shape assumed)
Aligned with the notes of the parts that wrote them where those notes exist (2.2, 3.1, 3.2, 3.3, 4.2,
6.1, 6.2, 7.1, 7.2, 8.2); the rest are guesses.
- part 2.1 `zones`: `ZoneSlot { player, row, lane: i32 }` (struct literal), `slots_of(PlayerId, Row) ->
  Vec<ZoneSlot>`, `card_at`/`carried_at(&GameState, &ZoneSlot) -> Option<&CardInstance>`,
  `zone_contents(&GameState, &ZoneSlot)` (read only through `.iter()`/`.first()` and `.id`, so owned or
  borrowed elements both work), `active_units_of(&GameState, PlayerId)` (likewise), `slot_of(&GameState,
  &CardInstance) -> Option<ZoneSlot>`, `acts_on_field`/`is_buried(&GameState, &CardInstance) -> bool`,
  `place_on_field(&mut GameState, &mut CardInstance, &ZoneSlot, PlaceOnFieldOptions { stack:
  Option<bool> }) -> bool` (parts 3.2 and 7.1 assumed this name), `remove_from_field(&mut GameState,
  &CardInstance, RemoveFromFieldOptions { with_pile: Option<bool> }) -> bool` (the options name is my
  guess, by analogy), `remove_from_any_zone(&mut GameState, &CardInstance)`, `release_home(&mut
  GameState, &str)`, `fresh_face_down_id(&mut GameState, &mut CardInstance) -> String` (the card is in
  no pile when it is called).
- part 2.1 `stays`: `exit_mark(&GameState) -> u32`, `moves_in(&[GameEvent], Option<&GameState>) ->
  LaterMoves { moved: IndexSet<String>, controller_before: IndexMap<String, PlayerId> }`.
- part 2.1 `scripts::script_of(&GameState, &str) -> CardScripts` (SURFACE §6.6). TS's
  `scriptOf(instance)` is a private `script_of_card` in activate.rs and quests.rs.
- part 2.1 `brittle_count::start_brittle_on_field(&mut GameState, &CardInstance, bool)` — by id, as
  part 2.2's convention for "writes a live card, reads the state". twice_forward writes `face_up` on the
  card in the state first, so the count starts on the revealed card either way.
- part 2.1 `tuning::tuned_count(&CardInstance, &str, i32) -> i32` (3 arguments; TS's `min` default).
- part 2.1 `params::param(&EffectContext, &str) -> i32`; `faces::card_type_of(&GameState, &CardInstance)`.
- part 2.2 `catalog::{fused_id_specs(Option<&GameState>, &str), is_digest_id(&str), def_of(Option<&GameState>,
  &str)}` (last_boards passes `None`: `create_game` freezes before a state exists, and a digest id is
  refused before the lookup); `preview::is_face_down`; `layers::unit_view -> UnitView { attack, health,
  max_health, .. }`.
- part 3.1 `combat::enter_new_side(&mut EngineSink, &CardInstance, PlayerId)` (writes the card by id;
  board_history writes its own changes to the placed card first, then calls it);
  `work::{paused(&EngineSink) -> bool, push_work(&mut EngineSink, Resume, Option<PlayerId>)}`;
  `targeting_point::pay_targeting_discards(&mut EngineSink, PlayerId, i32, &[String])`. The work
  dispatcher's `"@activate"` arm calls `subsystems::activate::run_owed_activation(sink, &item)` (pub).
- part 3.2 `resolve::{make_context(&mut EngineSink, Option<&CardInstance>, HookOptions), HookOptions {
  controller, .. }: Default}`; `prompts::{run_hook_resumable(&mut EngineSink, impl Into<HookInstance>,
  &str, HookResumableOptions) -> bool, HookInstance { id, def_id, controller, radiant },
  HookResumableOptions { controller, targets, modes, data, exits_from }}` (all five fields set; part
  3.2 named the types; part 4.2 and 3.3 assumed `&CardInstance` / `RunHookOptions` — part 31 picks one);
  `state_check::{state_check(&mut EngineSink), sacrifice_together(&mut EngineSink, &[CardInstance])}`;
  `targeting::why_targeting_discards_unpayable(&GameState, PlayerId, i32, &[String]) -> Result<(), EngineError>`.
- part 4.1 `mana::{spend_mana(&mut PlayerState, i32), mana_event(PlayerId, &PlayerState) -> GameEvent}`.
- part 4.2 `play_choices::{DeclaredChoices { targets: Vec<TargetDecl>, modes: Vec<ModeDecl> },
  in_declared_order(.., &[Selection], &[String], Option<&[TargetDecl]>) -> Vec<Selection>,
  targeting_discards_required(.., Option<&[TargetDecl]>) -> i32, play_choice_combinations(..,
  Option<&DeclaredChoices>) -> Vec<PlayChoices { targets: Option<Vec<Selection>>, modes:
  Option<Vec<String>> }>, why_declared_choices_refused(..) -> Result<(), EngineError>}`.
- effects, each built with `json_as(json!({…}))` from TS's literal (arg type names free):
  `add_to_hand`, `cost::set_cost_mod`, `damage::damage`, `move_::{exile, discard_random}`,
  `radiant::set_radiant_random`, `choose::choose_cell`, `fuse::fuse_cards`, `brittle::gain_brittle`;
  plus `choose::chosen_cells(&EffectContext) -> Vec<ZoneRef>`, `move_::bounce_card(&mut EngineSink,
  &CardInstance)` (part 7.2's), `targets::self_on_its_stay(&EffectContext) -> Option<CardInstance>` (7.1's).

### Called into this slice (shapes others must use)
- `last_boards::freeze_last_boards(Option<&LastBoardInput>, &CardDefs) -> Option<PerPlayerOpt<Vec<LastBoardEntry>>>`
  (part 1's state.rs), `last_board_for(&GameState, PlayerId) -> Vec<LastBoardEntry>` (lib.rs root),
  `last_board_candidates(&GameState, PlayerId, &[String])` (TS default `[]` is `&[]`),
  `rebuildable_from_id(&str, &CardDefs) -> bool`.
- `quests::observe_quest_event(&mut EngineSink, &GameEvent, after: &dyn Fn(&EngineSink) -> Vec<GameEvent>)`
  — exactly part 3.3's assumption (the closure is handed the sink). `notice_quests(&mut EngineSink)`,
  `quest_view_of(&GameState, &CardInstance) -> Option<QuestView>`, `quest_book_of(&GameState,
  &CardInstance) -> Option<QuestBook>` (**takes the state**: the script lookup needs it), `quest_def_of`/
  `quest_reward_of(&QuestBook, &str) -> Option<&…>`, `open_quest`/`hold_quest_aura(impl Into<String>) ->
  Effect`, `held_quest_auras`/`open_quests_of`/`completed_quests_of(&CardInstance) -> Vec<String>`.
- `activate`: `ActivateAction { instance_id, ability, targets, modes, tributes }` (a struct of
  `ActionBody::Activate`'s fields; serde through `ActionBody`, `From`/`TryFrom`, `into_body`,
  `from_body`); `activate_ability(&mut EngineSink, PlayerId, &ActivateAction) -> Result<(), EngineError>`
  (part 5.2's reduce passes `&ActionBody::Activate {…}`: part 31, wrap it with
  `ActivateAction::from_body(&body)` there, or make this take the body);
  `activate_actions_for(&GameState, PlayerId, &CardInstance) -> Vec<ActivateAction>` (reduce/legal
  actions need `.into()` per item); `why_cannot_activate_ability(&GameState, PlayerId, &str,
  Option<&str>)`, `why_activate_refused(&GameState, PlayerId, &ActivateAction)` both `-> Result<(),
  EngineError>`; `activation_views_for(&GameState, PlayerId, &CardInstance) -> Option<Vec<ActivationView>>`;
  `abilities_of(&GameState, &CardInstance) -> Vec<ActivationDecl>`, `uses_allowed(&CardInstance,
  &ActivationDecl) -> i32`, `uses_this_turn(&GameState, &CardInstance) -> i32`, `is_acting_on_field`,
  `activation_paid(&EffectContext) -> ActivationPaid { ability, tributed: Vec<TributedUnit> }`.
- `combo_index`: `grade_name(i32) -> Grade` (an enum with `as_str()` and `Display`; part 5.2's view
  wants the letter as a `String`: `grade_name(g).to_string()`), `start_grade(StartGradeArgs)` /
  `raise_grade(RaiseGradeArgs)` (TS's `{}` defaults are `::default()`), `combo_index_end_of_turn(&EngineSink,
  &CardInstance) -> Vec<Effect>` (a card passes its `&mut ctx`, which coerces), `grade_rises`, `grade_of`.
- `board_history`: `record_board_snapshot(&mut GameState)` (turn.rs), `roll_back(RollBackArgs { turns_ago,
  sides: RollBackSides::{SelfSide, Enemy, Both} })` (serde "self"/"enemy"/"both"; effects/mod.rs
  re-exports it), `restore_board(&mut EngineSink, PlayerId, i32, &[PlayerId]) -> Option<i32>`,
  `snapshot_for(&GameState, i32) -> Option<&BoardSnapshot>`.
- `papaya`: `papaya_begin() -> Vec<Effect>`, `papaya_answered(&mut EffectContext) -> Vec<Effect>` (a fn a
  card wraps with `hook`), `PAPAYA_STEP`, `cards_on_curve(&GameState, PlayerId, &[PapayaPoint], bool)` (TS
  took `Pick<EffectContext, "state" | "controller">`: the two fields as arguments).
- `twice_forward`: `twice_forward_trigger(TwiceForwardArgs { radiant_copy })` (data, so `json_as` of TS's
  literal works), `twice_forward_plays(&CardInstance) -> i32`.

## Decisions
- Live objects: TS wrote through live `CardInstance`s (the quest line, the use count, the grade, the
  trap's memory, a restored card's turn state). Here a card is a copy, re-read by id before each read
  TS made after something could move it, and every write goes to the card in the state
  (`find_instance_mut`); `ctx.live_self()` stands in for TS's live `ctx.self`.
- `why_*` refusals and `activate_ability` return `Result<(), EngineError>` with TS's text (SURFACE
  §4.4.9, as parts 3 and 4 did); the view turns an `Err` into its `reason`.
- `combo_index::naming` swaps `ctx.targets` for the one selection around the library effect's `apply`
  and restores it (TS spread a fresh context), as part 8.2's `onInstance` does.
- `Grade`, `RollBackSides` are hand-written serde enums, not `string_union!` (which would export client
  types that are not wire). `GRID_ROWS` is an immutable `static` (SURFACE §3 bans mutable statics only).
- papaya's rationals are `i64`; TS's identity test `pj !== pi` is the index test (the lanes are distinct).
- twice_forward: JS `plays % 0` is `NaN` (never 0), so a zero `plays` param fuses nothing; Rust's `%`
  would panic, hence the guard.
- board_history: TS assigned `state.homes = [...]` (possibly empty) mid-restore and normalised at the
  end; kept (presence is hashed, and the end state is TS's). `copyOf` is `clone()`.
- quests: `observe_quest_event` reads `after` once, right after the `questProgressed` early return and
  before any write — exactly where TS's memo first read it (the first loop pass, before `openFirstIfNew`).
  `QuestMemory.progress` is `IndexMap<String, i32>`; TS's `Object.entries` of an array (indices as keys)
  is kept for a malformed bag. `quest_memory_of` treats a JSON array as TS's `typeof "object"` did.
- activate: the uses record compares `turn` as a number (`f64`) as TS did; `mark_use` writes
  `{ turn, count }` with `json!` (key order is not observable: the hash sorts keys).
