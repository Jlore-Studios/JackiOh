# Slice: part 3, chunk 1 of 3 (#391): combat, damage, the targeting point, the work queue
BUILDS-RUN: 0

## FILES
- `crates/engine/src/combat.rs` ← `packages/engine/src/combat.ts` (every function, TS order)
- `crates/engine/src/damage.rs` ← `packages/engine/src/damage.ts`
- `crates/engine/src/targeting_point.rs` ← `packages/engine/src/targetingPoint.ts`
- `crates/engine/src/work.rs` ← `packages/engine/src/work.ts`

## SURFACE
Matched §4–§6 as written, with part 1's frozen types (hooks take `&mut EffectContext`, `EngineSink`
and `EngineError` where part 1 put them). Not ported, per §6.6: `registerWorkHandler`,
`registerDefaultWorkHandler`, `WorkHandler`, `registerDeclarationCheck` and `registerTargetingHooks`
calls (direct calls instead). `MAX_WORK_STEPS` is used from `config` (not re-exported from `work`).

## DEPENDS-ON
See GAPS: the names below, at their TS module paths.

## GAPS

### Fields wished on `EngineSink` (script.rs, part 1's; part 31 adds them)
- `owed_behind: Option<indexmap::IndexSet<String>>` — TS `DrainSink.owedBehind` (work.ts, R117),
  read and written by `work::run_next_work`. `new` sets `None`; **`reborrow` must clone it**, or a
  drain nested inside a resumed item (a play's step-4 loop, `prompts::run_resume`'s context) takes
  the items owed behind it. Every write is paired with a restore, so clone-on-reborrow is exact.
- `dispatched: Option<usize>` — TS `SettleSink.dispatched` (triggers.ts), read and written by
  `combat::withhold_from_frontier`. Unlike `owed_behind`, TS shares it across nested calls on the
  same sink object; whoever ports triggers.rs decides how it survives `reborrow`.

### Names called in other parts' modules (signatures as called here)
- `crate::scripts::script_of(state, def_id) -> CardScripts` (SURFACE §6.6; `.base`/`.radiant`
  cloned, so `&CardScripts` also works) and `crate::scripts::INGREDIENTS_KEY`. combat.rs and
  damage.rs keep private copies of TS's `scriptOf(instance)`, `flagsOf`, `textsOf` (vanilla → empty
  script) on top of it; work.rs's `card_step_for` uses it for TS's `scriptsFor(defId)`.
- `crate::kill_credit::KILL_CREDIT_KEY` (damage.rs keeps a private `credited_killer_id`).
- zones: `slots_of(player, Row) -> Vec<ZoneSlot>`, `card_at(state, &slot) -> Option<&CardInstance>`,
  `active_units_of(state, player) -> Vec<&CardInstance>` (references assumed: `.cloned()` used),
  `acts_on_field(state, &card)`, `is_carried(state, &card)`, `slot_of(state, &card) -> Option<ZoneSlot>`
  (`.lane`), `adjacent(&slot) -> Vec<ZoneSlot>`.
- layers: `unit_view(state, &card) -> UnitView { attack, max_health, health, keywords, armor,
  position: Position }`, `unit_has(state, &card, KeywordKind) -> bool`.
- `crate::faces::card_type_of(state, &card) -> CardType`.
- restrictions: `is_spell_source(state, Option<&CardInstance>) -> bool`,
  `attack_restriction(state, &attacker, &AttackTarget) -> Result<(), EngineError>` (a `why`-style
  refusal, SURFACE §4.4.9 — see Decisions).
- replacements (part 3, another chunk): `lethal_hit_window(sink, player, amount, source_id:
  Option<String>) -> Option<PlayerId>`, `healing_replaced(sink, &DamageTarget, amount) -> bool`,
  `answer_targeting(sink, &target: &CardInstance, by: PlayerId, what: TargetedWhat) ->
  Option<CardInstance>` — TS's anonymous `{ … }` argument objects as positional arguments in field order.
- targeting (another chunk): `interceptor_for(state, chooser, &targeted, Option<CardType>) ->
  Option<&CardInstance>`, `targeting_discards_for(state, &[Selection], None) -> i32` (third argument
  `Option<&dyn Fn(usize) -> bool>`), `may_target(state, chooser, Option<&CardInstance>, &candidate,
  leaving: Option<&str>) -> bool`.
- resolve (another chunk): `make_context(sink: &mut EngineSink, self_: Option<&CardInstance>,
  HookOptions) -> EffectContext<'_>` (a reborrow of the sink), `HookOptions { controller, targets,
  modes, data }` all `Option`, deriving `Default`.
- prompts (another chunk): `SELF_KEY`, `OpenPromptArgs { player, kind, options: Vec<PromptOption>,
  resume, … }`, `run_resumable_list(&mut ctx, &WorkPlan, &[Effect], Option<&PausedStep>) ->
  ListStatus` (TS's `sink` argument dropped: the context is the sink), `ListStatus::{Done, Parked,
  Asked, Over}`, `run_resume(sink, &Resume, ResumeOptions) -> bool`, `ResumeOptions { controller,
  targets, modes, chosen_from }` deriving `Default`. `ResumePlan` should be `= work::WorkPlan`.
- triggers (another chunk): `cards_in_trigger_order(state) -> Vec<TriggerHolder>` with **owned**,
  `Clone` holders (`card: CardInstance`, `controller`, `is_trap`, …: the loop calls `queue_trigger`
  with the sink while it walks them), `triggers_on_event(&holder, GameEventType)`,
  `queue_trigger(sink, &holder, &def, &event)`, `dispatch_pending(sink)`,
  `mark_dispatched(sink, &[usize])` — **indices into `sink.events`**, since TS's `WeakSet` of event
  objects has no Rust equivalent but a position in the action's one event list.
- stays: `exit_mark(state) -> u32`, `left_field_after(state, mark: u32, id) -> bool`,
  `moves_in(&[GameEvent], Option<&GameState>) -> LaterMoves { moved: IndexSet<String>,
  controller_before: IndexMap<String, PlayerId> }`.
- `crate::modifiers::move_sourced_modifiers(sink, source_id: &str, from, to)` (another chunk).
- `crate::traps::run_trap_window(sink, &GameEvent) -> TrapDispatch` (another chunk).
- `crate::state_check::state_check(sink)` (another chunk).
- `crate::effects::summon::summon(SummonArgs) -> Effect` (built with `json_as` from TS's literal),
  `crate::effects::move_::discard_from_hand(sink, &card)`.
- The work dispatcher (work.rs `run_work_item`) calls, each `(sink: &mut EngineSink, item: &WorkItem)`
  and each **must be `pub`** (TS kept them module-private): `setup::run_owed_setup`,
  `state_check::run_owed_deaths`, `traps::run_owed_window`, `traps::run_owed_firing`,
  `play_steps::run_owed_play`, `draw::run_owed_draw_chain`, `draw::run_owed_draw_count`,
  `turn::run_owed_start_of_turn`, `turn::run_owed_end_of_turn`,
  `subsystems::activate::run_owed_activation`, `subsystems::ai_policy::run_owed_ai_turn`.
- Called *into* this slice by other parts: `combat::declared_attack_stands(state, &DeclaredAttack)`
  (traps.rs, TS `registerDeclarationCheck`); `targeting_point::{targetable_options(state,
  &OpenPromptArgs), why_target_answer_refused, target_answer}` (prompts.rs, TS
  `registerTargetingHooks`); `work::script_step_for(&Script, &Resume) -> Option<Hook>` (prompts.rs).

### Engine tests (parts 24–27)
TS tests that swap work handlers with `registerWorkHandler` (a fixture kind) have no registry to
swap; the dispatcher knows only the engine's sequences and card steps.

## Decisions
- **Live objects.** TS passed and wrote through live `CardInstance`s. Every function here takes
  `&CardInstance` (the caller's copy) and reads/writes the card under that id in the state now
  (`find_instance`, `find_instance_mut`); a card that is nowhere has ceased to exist and is on no
  field. `DamageTarget::Unit { instance }` carries a copy; `land_hit` re-reads the target by id at
  every step. A damage **source** is read as handed (TS read the object it was given: a Death hook's
  source is the snapshot, R89); combat hands the attacker/defender as they stand at each hit.
- `DamageArgs { source: Option<CardInstance>, target, amount, flags: Option<DamageFlags> }`,
  `DamageFlags` all `Option<bool>`; `DamageSink<'a> = EngineSink<'a>`; `deal_damage(sink, DamageArgs)`
  by value. `hero_hit_amount(state, player, amount, pierce: bool)` (TS default `false` explicit).
- `why*` refusals (`why_cannot_attack`, `why_target_answer_refused`, private `why_cannot_declare`)
  return `Result<(), EngineError>` (SURFACE §4.4.9); `CombatResult = Result<(), EngineError>`.
- `SwitchPositionOptions { spend_exertion: Option<bool>, to: Option<Position> }` (Default).
- TS default `since = exitMark(...)` parameters are `Option<u32>` (`force_attacks_on`,
  `force_attacks_random`).
- `"enemies" | "enemyUnits"` is `combat::AttackAmong { Enemies, EnemyUnits }` (camelCase serde: it
  rides `@forcedRandom`'s data). `ExertionKind { Attack, Switch }`. Neither uses `string_union!`,
  which would export them to the web's generated types.
- `targeting_point::RedirectKind = wire::RedirectWhat` (the two literals a targeting moves).
- `InterceptArgs<'a> { chooser, picks: Vec<Selection>, targeting: Option<&'a dyn Fn(usize) -> bool>,
  accepts: Option<&'a dyn Fn(&GameState, &CardInstance, usize) -> bool>, what, source }` — `accepts`
  is handed the state because the caller's own state is borrowed by the sink (playSteps' filter closed
  over it in TS).
- `work::owe(sink, impl Into<OweItem>) -> Vec<WorkItem>` (every TS caller owes one item;
  `OweItem::{Resume, Item}` with `From` impls) plus `owe_all(sink, Vec<OweItem>)` for TS's varargs.
  `push_work`/`owe_unnumbered(sink, Resume, Option<PlayerId>)`; `park_work(sink, &WorkPlan,
  &PausedStep)`; `drop_work(state, impl Fn(&WorkItem) -> bool)`.
- `WorkPlan` is TS's `Resume & { owner }` with the fields written out (`WorkPlan::new(resume, owner)`,
  `.resume()`). `PausedStep.from` and `.part` are `usize` (list indices), marks `u32`, `memo:
  Option<Vec<Value>>`. `can_resume(state, &Resume)` takes the state (fused scripts come from it).
- The work dispatcher has exactly TS's registered hooks (research A §2.7). `"@delayedDestroy"` and
  `"@delayedDiscardHand"` (named in part 3's brief) are `DelayedEffect` hooks that `turn.rs` runs; TS
  never registered them as work handlers, so they are not arms here (a work item carrying one would
  raise in TS too).
- `script_step_for` reads `Script::hook_named(hook)`, then `script.resume[step]` for hook
  `"resume"`, then triggers, then activations. TS's `script[hook]` would also have returned a
  non-hook function field (`aura`, `cost`, …) or a `targetChecks` entry; no continuation names those.
- `trigger_step_for`'s rebuilt hook deserialises the stored event; one that does not parse as a
  `GameEvent` runs nothing (TS cast it unchecked).
- JSON key order of objects this slice writes follows TS's literals where an `IndexMap` holds it
  (`afterAttack`'s data, `remember_on`, `memory_of_part`, `reroot_remembered` keep JS object
  insertion semantics: assignment to an existing key keeps its place); nested `json!` objects are
  `serde_json::Map` (sorted), which only the hash reads, and it sorts anyway.
- Integer `Math.trunc` calls on amounts are dropped (game quantities are already `i32`);
  `Math.ceil(a / d)` is `(a + d - 1) / d`.
