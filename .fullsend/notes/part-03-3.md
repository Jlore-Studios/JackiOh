# Slice: part 3 (engine 2), chunk 3 of 3: replacements, traps, triggers
BUILDS-RUN: 0

## FILES
- `crates/engine/src/replacements.rs` — full port of `packages/engine/src/replacements.ts`.
- `crates/engine/src/traps.rs` — full port of `packages/engine/src/traps.ts`.
- `crates/engine/src/triggers.rs` — full port of `packages/engine/src/triggers.ts`.
Every TS function is ported, exported or not, in TS order, with its doc comments and every R-/§-citing
comment. Not ported, by SURFACE §6.6 (registration hooks go): `registerGraveyardRedirect(...)` (zones
calls `replacements::graveyard_redirect_for` directly), both `registerWorkHandler(...)` calls in traps
(work.rs's dispatcher calls `traps::run_owed_firing` for `"@trapFiring"` and `traps::run_owed_window`
for `"@trapWindow"`), and `registerDeclarationCheck`, its `DeclarationCheck` type and the
`attackStands` module `let` (traps calls `combat::declared_attack_stands` directly). The
`import "./cryTrigger"` side effect needs nothing (prompts matches `"@triggerCry"`).

## GAPS
### Needs a change to part 1's frozen files (part 31)
- **`EngineSink` needs `pub frontier: crate::triggers::FrontierSlot<'a>`.** `EngineSink::new` sets
  `FrontierSlot::default()` (an owned, empty `Frontier`); `reborrow()` sets
  `FrontierSlot::Shared(self.frontier.get_mut())`. Why: TS's `triggers.ts` told the action's events
  apart by object identity with two module-level `WeakSet`s (`collected`, `dispatchedElsewhere`) plus
  `SettleSink.dispatched` and `list.indexOf(event)`. Rust events are values, so these are positions in
  the action's event list, which every sink over that list must share (a loop on an `EffectContext`
  and the action's own loop see one frontier) — a by-value flag copied by `reborrow` cannot carry
  them back up. `Frontier` and `FrontierSlot` are defined in `triggers.rs` (no interior mutability:
  the reborrowed sink holds `&mut` to its parent's). Consequence: `resolve::make_context` must build
  its context's sink with `sink.reborrow()` (never `EngineSink::new` over the same lists), or a
  context's loop would collect the action's events a second time.
- `SettleSink` is `EngineSink` (type alias); TS's `sink.dispatched` is `triggers::dispatched(sink)` /
  `triggers::set_dispatched(sink, n)`. combat's `withhold_from_frontier` (part 3 chunk with combat)
  should read/write those, then call `mark_dispatched(sink, &[event])`. Behaviour difference, test-only:
  the cursor is shared, so the TS shape "a sink a test has already run a combat on" now withholds too.

### Names I call that other parts provide (signatures assumed)
- `crate::scripts::script_of(state, def_id)` → `CardScripts` (or `&`/`Arc`; read via `.base`/`.radiant`).
  The instance-face rule (Vanilla → empty, radiant → radiant face) is a private `with_face` in each of
  my files.
- `crate::resolve::{make_context(sink: &mut EngineSink, self_: Option<&CardInstance>, HookOptions) ->
  EffectContext, HookOptions { controller: Option<PlayerId>, data: Option<IndexMap<String, Value>>, .. }
  : Default, HookName}` — `HookName` an enum with variants `Cry, Death, StartOfGame, StartOfTurn,
  StartOfOpponentTurn, EndOfTurn, Activate, OnPlayHook`, `Copy + PartialEq`, with `as_str()`.
- `crate::prompts::apply_resumable(ctx: &mut EffectContext, plan: &WorkPlan, effects: &[Effect],
  paused: Option<PausedStep>) -> bool` — **TS's `sink` argument dropped** (the context is the sink; Rust
  cannot hand both). `crate::prompts::run_hook_resumable(sink, &CardInstance, hook_name: &str,
  RunHookOptions { controller: Option<PlayerId>, .. }: Default) -> bool` (TS's anonymous options type
  named `RunHookOptions`).
- `crate::work::{WorkPlan { resume: Resume, owner: PlayerId }, owe(sink, Resume), owe_unnumbered(sink,
  Resume, Option<PlayerId>), push_work(sink, Resume, Option<PlayerId>), drain_work(sink) -> bool,
  card_data(&IndexMap) -> IndexMap, run_marks_of(&IndexMap) -> Option<RunMarks> (field event_stay:
  Option<EventStay>), EVENT_KEY, RUN_MARKS_KEY}`. TS `owe` was variadic; every caller passes one Resume.
- `crate::damage::{deal_damage(sink, DamageArgs) -> i32, DamageArgs { source: Option<CardInstance>,
  target: DamageTarget, amount: i32, flags: Option<DamageFlags> }, DamageFlags { ignore_armor:
  Option<bool>, .. }: Default, DamageTarget::{Unit { instance: CardInstance }, Hero { player }}: Clone}`.
- `crate::zones::{active_units_of(state, player), card_at(state, &ZoneSlot) -> Option<&CardInstance>,
  slots_of(player, Row) -> Vec<ZoneSlot>, first_entry_zone(state, player, Row) -> Option<ZoneSlot>,
  is_unit_token, acts_on_field, is_buried, is_carried (state, &CardInstance), move_to_zone(&mut state,
  &CardInstance, OffFieldZone, options: Default) -> MoveResult, report_graveyard_landing(sink,
  &CardInstance, MoveResult), remove_from_any_zone(&mut state, &CardInstance), place_on_field(&mut
  state, CardInstance /* owned */, ZoneSlot, options: Default) -> bool, OffFieldZone::Graveyard,
  GraveyardRedirect: Deserialize from TS's JSON shape}`. ZoneSlot has `row`, `lane` (I build `ZoneRef`).
- `crate::stays::{event_stay_of(state, &GameEvent) -> EventStay, moves_in(&[GameEvent],
  Option<&GameState>) -> LaterMoves, uncovered_by(state, &GameEvent) -> Vec<String>,
  note_reported(&mut state, &GameEvent), LaterMoves { moved: <set of String>.contains, controller_before:
  <map String→PlayerId>.get }}`. `exit_mark`, `event_mark`, `left_field_after` are private copies.
- `crate::subsystems::quests::observe_quest_event(sink, &GameEvent, after: &dyn Fn(&EngineSink) ->
  Vec<GameEvent>)` — **TS's `() => …` closure takes the sink as its argument** (a closure capturing
  the sink cannot coexist with the `&mut` argument). Same shape for my own `traps::offer_event_to_traps`.
- `crate::animated::{face_type_of, is_animated, animated_kind_of -> Option<_>}`,
  `crate::announce::is_announce_live(state, &str)`, `crate::book_swap::granted_hand_triggers(&CardInstance,
  &[TriggerDef])` (Vec or slice), `crate::draw_complete::held_back(state, &GameEvent)`,
  `crate::effects::delay::refresh_scope_marks(sink)`, `crate::marks::sweep_marks(sink)`,
  `crate::preview::is_face_down(state, &CardInstance)`, `crate::faces::card_type_of(state, &CardInstance)`,
  `crate::mana::modifier_is_live(state, &PlayerModifier)`, `crate::modifiers::add_modifier(sink, player,
  ModifierExpiry, ModifierKind)` (TS `DistributiveOmit<PlayerModifier, "id">` as its two halves),
  `crate::effects::flicker::flicker_card(sink, &CardInstance) -> bool`,
  `crate::state_check::state_check(sink)`, `crate::combat::declared_attack_stands(state, &DeclaredAttack)`.

### Names others call in my files (their shapes)
- `replacements::lethal_hit_window(sink, player, amount, source_id: Option<String>) -> Option<PlayerId>`
  and `answer_targeting(sink, &CardInstance, by, TargetedWhat) -> Option<CardInstance>`: TS's
  anonymous object arguments are separate arguments. `healing_replaced(sink, &DamageTarget, i32)`,
  `would_die_window(sink, &[CardInstance]) -> Vec<CardInstance>`, `graveyard_redirect_for(&GameState,
  &CardInstance) -> Option<GraveyardRedirect>`, `replacement_of(&EffectContext) -> Option<ReplacementRecord>`.
- `traps::fire_trap(sink, &TrapMatch, &GameEvent, controller: Option<PlayerId>)` (TS default param);
  `run_owed_firing`/`run_owed_window(sink, &WorkItem)`; `trap_controllers_of(Option<&Value>)`;
  `arrived_during_play(&GameEvent) -> &[String]`; `standing_event(&EngineSink, …)`.
- `triggers::settle(sink, SettleOptions)` — TS's optional second argument is always passed
  (`SettleOptions::default()`); `run_hooks_in_trigger_order`/`queue_hooks_in_trigger_order(sink,
  HookName, Option<PlayerId>)`; `run_queued_trigger(sink, &QueuedTrigger)`; `triggers_on_event(&holder,
  GameEventType) -> Vec<TriggerDef>`; `mark_dispatched(sink, &[GameEvent])` (TS took no sink).

## Decisions
- Instances: TS passed live objects; I hold snapshots (`CardInstance` clones) and re-read the card by
  id (`find_instance`) wherever TS read the object after something could have moved it, and write
  `face_up`/`summoned_turn` to the live card with `find_instance_mut`.
- `ReplacementDef`, `ReplacedEvent`, `ReplacementContext` and the other declaration types are part 1's
  (script.rs) and `pub use`d from replacements.rs (same items, so the root glob is not ambiguous).
  `FlickeredCard`, `ReplacementRecord`, `REPLACED_KEY` are defined here (serde camelCase).
- `GraveyardRedirect` is built by deserialising TS's JSON shape so this file names none of zones'
  variants (zones' type must derive `Deserialize`).
- `TriggerZone` is a plain enum with `as_str()`, not `string_union!` (which would export a TS type to
  the web's generated dir). `TrapFiring` is private with serde; `TrapRun` private.
- `SETTLE_PASS_CAP` stays in triggers.rs, derived from config's numbers as TS derived it (24,000).
- `creation_number`: `c<digits>` → i64, else `i64::MAX`; ties broken by id, stable sort.
- `events_after_dispatched`: an owed dispatch item is "this action's own" when its seq is in
  `Frontier.positions`, or it equals an event at an `elsewhere` position (the AI turn's events, whose
  seqs the outer frontier never saw). The event's own position is passed down from the loop (`at`);
  the public `dispatch_event` finds it by value.
- `mark_dispatched` marks the tail of the list when it equals the given events (both TS call sites
  push then mark), else the latest unmarked equal entry of each.
- Comparisons TS made by identity (`state.pending !== before`) are by value (`PartialEq`); a new
  prompt has a fresh id, so they agree.
- TS `throw` on a stuck loop is `panic!` with the same message.
- Performance note for Wave 3: `TriggerHolder.script` clones the running face's `Script` for every
  holder on every `cards_in_trigger_order` call (TS shared one object); an `Arc<Script>` from
  `scripts::script_of` would remove it.
