//! Unfinished engine work: the resumable-work queue (SPEC §9.3, §10.3, §10.6, R113).
//!
//! Any step of an engine sequence can open a prompt (the play pipeline of §10.5, the attack window of
//! §4.2 step 4, the Death hooks and Reborn returns of §4.5 steps 3-4, the end-of-turn hooks of §2.2, a
//! card's effect list), and a prompt ends the action (§9.3). A sequence that spans one parks what it owes
//! in `state.work`, never in a local variable, and the §10.3 loop drains it after the answer. Owed items
//! are plain data (a `Resume`), never closures, so a paused state survives a JSON round trip and replays
//! exactly (§10.1); a hook is a pure builder (CLAUDE.md rule 5), so re-entering it and skipping the effects
//! that already ran continues the same sequence.
//!
//! R113's order is neither a queue nor a stack: `state.work_cursor`, where the running cascade parks next,
//! advances with each park and resets to 0 when an item is taken or a drain starts, so a pause during a
//! resumption goes ahead of everything still owed (a Cry's tail W1 runs before play steps 6-8, W2, and W3,
//! parked by W1 still inside step 5, precedes W2).
//!
//! R68, R59: work drains before the trigger queue moves (`triggers::settle`), as an interrupted sequence is
//! the action still finishing. The state check is the caller's (R59: between whole effects and triggers,
//! never between the hits of one), so this module never calls it. `run_work_item` dispatches on
//! `resume.hook` (SURFACE §6.6, research A §2.7).

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::MAX_WORK_STEPS;
use crate::script::{ACTIVATION_HOOK_PREFIX, EngineSink, Hook, Script, activation_decls, hook};
use crate::state::{EventStay, GameState, Resume, WorkItem};
use crate::wire::{GameEvent, PlayerId, Selection};

/// Which sequence an item belongs to: `resume.hook`. An engine sequence uses a name no card
/// `Script` has (`play_steps::PLAY_WORK_KIND`), a card continuation the hook whose step runs.
pub type WorkKind = str;

/// A `Resume` plus its owner, the controller the re-entered step runs as; `prompts::ResumePlan` is this.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkPlan {
    pub def_id: String,
    pub hook: String,
    pub step: String,
    pub radiant: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
    pub data: IndexMap<String, Value>,
    pub owner: PlayerId,
}

impl WorkPlan {
    pub fn new(resume: Resume, owner: PlayerId) -> WorkPlan {
        WorkPlan {
            def_id: resume.def_id,
            hook: resume.hook,
            step: resume.step,
            radiant: resume.radiant,
            instance_id: resume.instance_id,
            data: resume.data,
            owner,
        }
    }

    pub fn resume(&self) -> Resume {
        Resume {
            def_id: self.def_id.clone(),
            hook: self.hook.clone(),
            step: self.step.clone(),
            radiant: self.radiant,
            instance_id: self.instance_id.clone(),
            data: self.data.clone(),
        }
    }
}

/// Where a parked effect list keeps its control block, so the rest of `data` stays the card's.
pub const PAUSE_KEY: &str = "__paused";

/// The control block of a parked effect list: all JSON, so the tail survives the answer.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PausedStep {
    /// Index into the innermost list the pause stood in to continue from.
    pub from: usize,
    pub targets: Vec<Selection>,
    pub modes: Vec<String>,
    /// The parts of a composed list the pause stood inside (`Effect.expand`), outermost first, by
    /// index. The continuation rebuilds them, finishes the innermost list from `from`, then each
    /// enclosing list after its part (`prompts::apply_resumable`, R102, R113).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub part: Option<Vec<usize>>,
    /// What each part in `part` handed its rebuild (`EffectPart.memo`), level by level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memo: Option<Vec<Value>>,
    /// R174: the field's departures when the list began (`EffectContext.exits_from`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exits_from: Option<u32>,
    /// R174, §10.6: when the list's `targets` were picked, if later (`EffectContext.chosen_from`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chosen_from: Option<u32>,
    /// R136: the units the list summoned before the pause (`EffectContext.summoned`), since the events
    /// that say so belong to the action that paused and the tail resumes in a later one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summoned: Option<Vec<String>>,
    /// R98: the list's card was in the resolving zone as it began (`EffectContext.self_resolving`).
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolving: Option<bool>,
    /// R174, R212: a queued trigger's event's cards and its stays (`EffectContext.event_stay`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_stay: Option<EventStay>,
}

/// Where a continuation a prompt stores (`prompts::resume_self`) keeps its run's marks, apart from the
/// card's data: the answered step is the same run as the list that asked (R113, §10.6), so it reads
/// that run's stays (R174) and summoned units (R136), whichever action it resumes in.
pub const RUN_MARKS_KEY: &str = "__run";

/// What a continuation carries of the run it continues. All JSON.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct RunMarks {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exits_from: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summoned: Option<Vec<String>>,
    /// Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolving: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_stay: Option<EventStay>,
}

fn number_u32(value: Option<&Value>) -> Option<u32> {
    value.and_then(Value::as_u64).map(|n| n as u32)
}

fn strings_in(value: &[Value]) -> Vec<String> {
    value
        .iter()
        .filter_map(|id| id.as_str().map(str::to_string))
        .collect()
}

fn event_stay_in(block: Option<&Value>) -> Option<EventStay> {
    let stay = block?.as_object()?;
    let from = number_u32(stay.get("from"))?;
    let ids = stay.get("ids")?.as_array()?;
    Some(EventStay {
        from,
        ids: strings_in(ids),
    })
}

/// The marks a continuation's data carries, or `None` when it carries none.
pub fn run_marks_of(data: &IndexMap<String, Value>) -> Option<RunMarks> {
    let marks = data.get(RUN_MARKS_KEY)?.as_object()?;
    Some(RunMarks {
        exits_from: number_u32(marks.get("exitsFrom")),
        summoned: marks
            .get("summoned")
            .and_then(Value::as_array)
            .map(|ids| strings_in(ids)),
        resolving: if marks.get("resolving").and_then(Value::as_bool) == Some(true) {
            Some(true)
        } else {
            None
        },
        event_stay: event_stay_in(marks.get("eventStay")),
    })
}

/// Whether a scope must stop and park the rest: a prompt is open, or the game is over.
pub fn paused(sink: &EngineSink<'_>) -> bool {
    sink.state.pending.is_some() || sink.state.result.is_some()
}

// The stored control block

pub fn paused_of(data: &IndexMap<String, Value>) -> Option<PausedStep> {
    let step = data.get(PAUSE_KEY)?.as_object()?;
    Some(PausedStep {
        from: step
            .get("from")
            .and_then(Value::as_u64)
            .map_or(0, |from| from as usize),
        targets: step
            .get("targets")
            .and_then(Value::as_array)
            .map(|targets| {
                targets
                    .iter()
                    .filter_map(|target| serde_json::from_value::<Selection>(target.clone()).ok())
                    .collect()
            })
            .unwrap_or_default(),
        modes: step
            .get("modes")
            .and_then(Value::as_array)
            .map(|modes| strings_in(modes))
            .unwrap_or_default(),
        part: step.get("part").and_then(Value::as_array).map(|part| {
            part.iter()
                .filter_map(|at| at.as_u64().map(|at| at as usize))
                .collect()
        }),
        memo: step.get("memo").and_then(Value::as_array).cloned(),
        exits_from: number_u32(step.get("exitsFrom")),
        chosen_from: number_u32(step.get("chosenFrom")),
        summoned: step
            .get("summoned")
            .and_then(Value::as_array)
            .map(|ids| strings_in(ids)),
        resolving: if step.get("resolving").and_then(Value::as_bool) == Some(true) {
            Some(true)
        } else {
            None
        },
        event_stay: event_stay_in(step.get("eventStay")),
    })
}

/// R102: which ingredient's text of a fused card is running, as ingredient indices from the outermost
/// fusion in. `subsystems::fuse` writes it into each part's context, so a continuation that text stores
/// comes back to its step alone and its memory stays its ingredient's own (`effects::memory`).
pub const PART_KEY: &str = "__part";

/// How much of `PART_KEY`'s path the combined hooks above the running one have used. A build-time
/// mark only: a stored continuation drops it (`card_data`), so a re-entry reads the path from the top.
pub const PART_DEPTH_KEY: &str = "__partDepth";

/// The ingredient path a context's data names (`PART_KEY`), or `None` outside a fused card's part.
pub fn part_path_of(data: &IndexMap<String, Value>) -> Option<Vec<usize>> {
    let raw = data.get(PART_KEY)?.as_array()?;
    let path: Vec<usize> = raw
        .iter()
        .filter_map(|at| at.as_u64().map(|at| at as usize))
        .collect();
    if path.is_empty() { None } else { Some(path) }
}

/// R102: the key a fused card's ingredient keeps a memory under — its own, so two Carnivorous Cubes'
/// meals stay two. Outside a fused card's part it is the key itself.
pub fn part_memory_key(data: &IndexMap<String, Value>, key: &str) -> String {
    match part_path_of(data) {
        None => key.to_string(),
        Some(path) => {
            let joined: Vec<String> = path.iter().map(usize::to_string).collect();
            format!("{key}@{}", joined.join("."))
        }
    }
}

/// R102, R77: the keys `effects::memory::remember` has written on a card, by the key the card named
/// (`eaten` for #22), so a Fuse that keeps the card can tell what its texts remembered from the
/// engine's own entries (#98's rolled power, the ingredients' prices) and move it with them.
pub const REMEMBERED_KEY: &str = "__remembered";

fn noted_keys(memory: &IndexMap<String, Value>) -> Vec<String> {
    memory
        .get(REMEMBERED_KEY)
        .and_then(Value::as_array)
        .map(|noted| strings_in(noted))
        .unwrap_or_default()
}

/// Write what a card's text remembers, under its ingredient's own key (R102), and note the key.
pub fn remember_on(
    memory: &mut IndexMap<String, Value>,
    data: &IndexMap<String, Value>,
    key: &str,
    value: Value,
) {
    memory.insert(part_memory_key(data, key), value);
    let noted: Vec<Value> = memory
        .get(REMEMBERED_KEY)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if !noted.iter().any(|entry| entry.as_str() == Some(key)) {
        let mut next = noted;
        next.push(Value::String(key.to_string()));
        memory.insert(REMEMBERED_KEY.to_string(), Value::Array(next));
    }
}

fn is_key_or_part(stored: &str, key: &str) -> bool {
    stored == key || stored.strip_prefix(key).is_some_and(|rest| rest.starts_with('@'))
}

/// R102, R77: a card a Fuse keeps becomes ingredient `index` of the new fusion and its texts' memory
/// moves with it: `key` to `key@<index>`, an earlier fusion's `key@<path>` to `key@<index>.<path>`,
/// where that text now runs; the ingredients fused onto it read none of it.
pub fn reroot_remembered(memory: &mut IndexMap<String, Value>, index: usize) {
    let noted = noted_keys(memory);
    for key in &noted {
        let stored_keys: Vec<String> = memory.keys().cloned().collect();
        for stored in stored_keys {
            if !is_key_or_part(&stored, key) {
                continue;
            }
            let path = if stored == *key {
                format!("{index}")
            } else {
                format!("{index}.{}", &stored[key.len() + 1..])
            };
            // A new key goes last, one that is there already keeps its place (as a JS object's do). A
            // value an earlier rename of this walk took is gone, and so is its renamed key.
            let value = memory.shift_remove(&stored);
            let renamed = format!("{key}@{path}");
            match value {
                Some(value) => {
                    memory.insert(renamed, value);
                }
                None => {
                    memory.shift_remove(&renamed);
                }
            }
        }
    }
}

/// R102, R384: the memory as ingredient `index`'s text reads it from a hook with no context to name
/// its place (an Activate ability's `canActivate` and `has`): `reroot_remembered` inverted, the other
/// ingredients' memory left out and the engine's own entries as they are.
pub fn memory_of_part(memory: &IndexMap<String, Value>, index: usize) -> IndexMap<String, Value> {
    let noted = noted_keys(memory);
    let mut out = memory.clone();
    for key in &noted {
        let own = format!("{key}@{index}");
        for stored in memory.keys() {
            if !is_key_or_part(stored, key) {
                continue;
            }
            out.shift_remove(stored);
        }
        // In place when `out` still holds the key, else last, as a JS object's keys go.
        let own_dot = format!("{own}.");
        for (stored, value) in memory {
            if *stored == own {
                out.insert(key.clone(), value.clone());
            } else if let Some(rest) = stored.strip_prefix(own_dot.as_str()) {
                out.insert(format!("{key}@{rest}"), value.clone());
            }
        }
    }
    out
}

/// The card's own captured data, with the control blocks taken back out.
pub fn card_data(data: &IndexMap<String, Value>) -> IndexMap<String, Value> {
    data.iter()
        .filter(|(key, _)| {
            key.as_str() != PAUSE_KEY && key.as_str() != RUN_MARKS_KEY && key.as_str() != PART_DEPTH_KEY
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

// Parking (R113)

/// Start a fresh pause cascade: the next park goes ahead of everything still owed (R113). Taking an
/// item and entering `drain_work` do this; call it when a cascade begins somewhere else.
pub fn begin_work_cascade(sink: &mut EngineSink<'_>) {
    sink.state.work_cursor = 0;
}

fn insert_at(state: &GameState) -> usize {
    state.work_cursor.min(state.work.len())
}

fn place(state: &mut GameState, item: WorkItem) -> WorkItem {
    let at = insert_at(state);
    state.work.insert(at, item.clone());
    state.work_cursor = at + 1;
    item
}

/// Park one owed continuation at the cursor and return the item. The id and `seq` come from
/// `state.next_seq` (R68's creation order), so the queue a replay builds is the queue the live game
/// had; `owner` defaults to the active player.
pub fn push_work(sink: &mut EngineSink<'_>, resume: Resume, owner: Option<PlayerId>) -> WorkItem {
    let state = &mut *sink.state;
    let item = WorkItem {
        id: format!("w{}", state.next_seq),
        seq: state.next_seq,
        owner: owner.unwrap_or(state.active),
        resume,
    };
    state.next_seq += 1;
    place(state, item)
}

/// Park one owed continuation without taking a number: its id borrows `state.next_seq`. R177: for an
/// item whose existence hangs on a card someone may not read, like the end-of-turn trap window's rest,
/// owed only while a face-down trap (read by its controller alone, R33) still watches the event; the
/// ids handed the modifiers next (R169) say nothing of it. Nothing orders or finds work by its `seq`.
pub fn owe_unnumbered(sink: &mut EngineSink<'_>, resume: Resume, owner: Option<PlayerId>) -> WorkItem {
    let state = &mut *sink.state;
    let item = WorkItem {
        id: format!("u{}.{}", state.next_seq, state.work.len()),
        seq: state.next_seq,
        owner: owner.unwrap_or(state.active),
        resume,
    };
    place(state, item)
}

/// One thing a pause owes: a `Resume` to park as a new item, or a `WorkItem` to park as it stands.
#[derive(Clone, Debug, PartialEq)]
pub enum OweItem {
    Resume(Resume),
    Item(WorkItem),
}

impl From<Resume> for OweItem {
    fn from(resume: Resume) -> OweItem {
        OweItem::Resume(resume)
    }
}

impl From<WorkItem> for OweItem {
    fn from(item: WorkItem) -> OweItem {
        OweItem::Item(item)
    }
}

/// What a pause still owes. A whole `WorkItem` parked again keeps its id and `seq`, which is what a
/// handler that has to wait behind another one needs.
pub fn owe(sink: &mut EngineSink<'_>, item: impl Into<OweItem>) -> Vec<WorkItem> {
    owe_all(sink, vec![item.into()])
}

/// `owe` for several, each in order.
pub fn owe_all(sink: &mut EngineSink<'_>, items: Vec<OweItem>) -> Vec<WorkItem> {
    items
        .into_iter()
        .map(|item| match item {
            OweItem::Item(item) => place(sink.state, item),
            OweItem::Resume(resume) => push_work(sink, resume, None),
        })
        .collect()
}

/// Park the effects after the one that opened a prompt: the same continuation, plus the index to
/// continue from and the selections the step was running with (`prompts::apply_resumable`).
pub fn park_work(sink: &mut EngineSink<'_>, plan: &WorkPlan, step: &PausedStep) -> WorkItem {
    let mut data = card_data(&plan.data);
    data.insert(
        PAUSE_KEY.to_string(),
        serde_json::to_value(step).unwrap_or(Value::Null),
    );
    let resume = Resume {
        def_id: plan.def_id.clone(),
        hook: plan.hook.clone(),
        step: plan.step.clone(),
        radiant: plan.radiant,
        instance_id: plan.instance_id.clone(),
        data,
    };
    push_work(sink, resume, Some(plan.owner))
}

// Reading and taking

pub fn has_work(state: &GameState) -> bool {
    !state.work.is_empty()
}

pub fn peek_work(state: &GameState) -> Option<&WorkItem> {
    state.work.first()
}

/// Every owed item, or only the ones belonging to one sequence.
pub fn owed_work(state: &GameState, kind: Option<&WorkKind>) -> Vec<WorkItem> {
    match kind {
        None => state.work.clone(),
        Some(kind) => state
            .work
            .iter()
            .filter(|item| item.resume.hook == kind)
            .cloned()
            .collect(),
    }
}

pub fn is_owed(state: &GameState, kind: &WorkKind) -> bool {
    state.work.iter().any(|item| item.resume.hook == kind)
}

/// Take the next owed item. Taking one ends the cascade that parked it, so the cursor resets (R113).
pub fn take_work(state: &mut GameState) -> Option<WorkItem> {
    let item = if state.work.is_empty() {
        None
    } else {
        Some(state.work.remove(0))
    };
    state.work_cursor = 0;
    item
}

/// Drop the owed items a predicate names and return them — for a sequence that has caught up with
/// a tail it parked, which would otherwise run twice. The cursor follows the items it loses.
pub fn drop_work(state: &mut GameState, matches: impl Fn(&WorkItem) -> bool) -> Vec<WorkItem> {
    let mut dropped: Vec<WorkItem> = Vec::new();
    let mut kept: Vec<WorkItem> = Vec::new();
    let mut cursor = state.work_cursor;
    for (index, item) in state.work.iter().enumerate() {
        if !matches(item) {
            kept.push(item.clone());
            continue;
        }
        dropped.push(item.clone());
        if index < state.work_cursor {
            cursor = cursor.saturating_sub(1);
        }
    }
    if dropped.is_empty() {
        return dropped;
    }
    state.work_cursor = cursor.min(kept.len());
    state.work = kept;
    dropped
}

/// `drop_work` by id: the tail a step parked before it turned out not to need it.
pub fn unpark_work(state: &mut GameState, id: &str) -> Option<WorkItem> {
    drop_work(state, |item| item.id == id).into_iter().next()
}

// Running

/// Where a continuation of an event trigger keeps the event it answers (`triggers::queue_trigger`).
pub const EVENT_KEY: &str = "event";

/// An event trigger's own list, re-entered by its id (R113): a `TriggerDef` lives in a list, not under a
/// `Script` key, so its tail parks under its id and is rebuilt here from the event in the data.
fn trigger_step_for(script: &Script, resume: &Resume) -> Option<Hook> {
    let def = script
        .triggers
        .iter()
        .chain(script.hand_triggers.iter())
        // B5 E26: a deck or graveyard trigger's list parks under its id like any other trigger's.
        .chain(script.deck_triggers.iter())
        .chain(script.graveyard_triggers.iter())
        .find(|candidate| candidate.id == resume.hook)?
        .clone();
    Some(hook(move |ctx| {
        let Some(raw) = ctx.data.get(EVENT_KEY) else {
            return Vec::new();
        };
        if !raw.is_object() || !raw.get("type").is_some_and(Value::is_string) {
            return Vec::new();
        }
        let Ok(event) = serde_json::from_value::<GameEvent>(raw.clone()) else {
            return Vec::new();
        };
        (def.run)(ctx, &event)
    }))
}

/// R384: an Activate ability's own list (`Script.activations`), re-entered by its id like a trigger's.
fn activation_step_for(script: &Script, resume: &Resume) -> Option<Hook> {
    let id = resume.hook.strip_prefix(ACTIVATION_HOOK_PREFIX)?;
    activation_decls(script)
        .into_iter()
        .find(|decl| decl.id == id)
        .map(|decl| decl.run)
}

/// The step a script registers for a continuation: a hook of its own (`cry`, `delayed`), an entry in
/// its step table (`resume: { picked: … }`), or an event trigger or Activate ability (R384) by its id.
/// `prompts.rs` re-enters through this and `can_resume` asks it, so the two cannot disagree.
pub fn script_step_for(script: &Script, resume: &Resume) -> Option<Hook> {
    // R1241: Meditative #95's rest-of-game effect is the engine's own step, whatever script it names: a
    // start-of-turn effect re-enters through here and never reaches `run_engine_delayed`.
    if resume.hook == crate::subsystems::CHAOS_ETERNAL_HOOK {
        return Some(crate::subsystems::eternal_step());
    }
    // R2200002: Training Grounds #39's every-second-turn add, the same.
    if resume.hook == crate::subsystems::CHAOS_TG_ALTERNATE_HOOK {
        return Some(crate::subsystems::alternate_step());
    }
    // R1882004: Stinkyscape #150's death watcher, the same.
    if resume.hook == crate::subsystems::CHAOS_DEATH_WATCH_HOOK {
        return Some(crate::subsystems::death_watch_step());
    }
    // R1845002: a trade offer's steps, the same, as it comes due and as each of its prompts is answered.
    if resume.hook == crate::effects::trade::TRADE_OFFER_HOOK {
        return crate::effects::trade::trade_offer_step(&resume.step);
    }
    // R1841002: an Equip pick's answer is the engine's own step, whatever card asked.
    if resume.hook == "resume" && resume.step == crate::effects::fuse::EQUIP_PICK_STEP {
        return Some(crate::effects::fuse::equip_pick_step());
    }
    if let Some(step) = script.hook_named(&resume.hook) {
        return Some(step.clone());
    }
    if resume.hook == "resume"
        && let Some(step) = script.resume.get(resume.step.as_str())
    {
        return Some(step.clone());
    }
    trigger_step_for(script, resume).or_else(|| activation_step_for(script, resume))
}

/// The step a card's script registers for this continuation, on the face the pause recorded.
fn card_step_for(state: &GameState, resume: &Resume) -> Option<Hook> {
    let scripts = crate::scripts::scripts_ref(state, &resume.def_id);
    let script = if resume.radiant {
        &scripts.radiant
    } else {
        &scripts.base
    };
    script_step_for(script, resume)
}

/// The engine sequences `run_work_item` runs itself, by `resume.hook`.
const ENGINE_WORK_HOOKS: &[&str] = &[
    "@setup",
    "@deaths",
    "@trapWindow",
    "@trapFiring",
    "play",
    "@drawChain",
    "@drawCount",
    "@attackWindow",
    "@attackSummon",
    "@combatCopies",
    "@forcedRun",
    "@forcedRandom",
    "@whenAttacks",
    "@afterAttack",
    "@combatCopies",
    "@afterAttacked",
    "@allyAttacked",
    "@startOfTurn",
    "@endOfTurn",
    "@activate",
    "@aiTurn",
    "@setRepeat",
    "@extraCry",
    "@melee",
];

/// Whether anything at all knows how to resume this item (R113).
pub fn can_resume(state: &GameState, resume: &Resume) -> bool {
    if ENGINE_WORK_HOOKS.contains(&resume.hook.as_str()) {
        return true;
    }
    // A sequence a test made (`testkit::seams::register_work_handler`).
    #[cfg(feature = "testkit")]
    if crate::testkit::seams::work_handler(&resume.hook).is_some() {
        return true;
    }
    card_step_for(state, resume).is_some()
}

/// Run one owed item: the module that owns its sequence, or the card-continuation runner when the
/// card's own script has that step (R113). When neither knows the hook this panics: an item that
/// cannot be resumed is a lost sequence and must never be dropped in silence.
pub fn run_work_item(sink: &mut EngineSink<'_>, item: &WorkItem) {
    match item.resume.hook.as_str() {
        "@setup" => crate::setup::run_owed_setup(sink, item),
        "@deaths" => crate::state_check::run_owed_deaths(sink, item),
        "@trapWindow" => crate::traps::run_owed_window(sink, item),
        "@trapFiring" => crate::traps::run_owed_firing(sink, item),
        "play" => crate::play_steps::run_owed_play(sink, item),
        "@drawChain" => crate::draw::run_owed_draw_chain(sink, item),
        "@drawCount" => crate::draw::run_owed_draw_count(sink, item),
        "@attackWindow" => crate::combat::run_owed_attack(sink, item),
        "@attackSummon" => crate::attack_summon::run_owed(sink, item),
        "@forcedRun" => crate::combat::run_owed_forced_run(sink, item),
        "@forcedRandom" => crate::combat::run_owed_forced_random(sink, item),
        "@whenAttacks" => crate::combat::run_owed_when_attacks(sink, item),
        "@afterAttack" => crate::combat::run_owed_after_attack(sink, item),
        "@combatCopies" => crate::combat::run_owed_combat_copies(sink, item),
        "@afterAttacked" => crate::combat::run_owed_after_attacked(sink, item),
        "@allyAttacked" => crate::combat::run_owed_ally_attacked(sink, item),
        "@allyAttack" => crate::combat::run_owed_ally_attack(sink, item),
        "@startOfTurn" => crate::turn::run_owed_start_of_turn(sink, item),
        "@endOfTurn" => crate::turn::run_owed_end_of_turn(sink, item),
        "@activate" => crate::subsystems::activate::run_owed_activation(sink, item),
        "@aiTurn" => crate::subsystems::ai_policy::run_owed_ai_turn(sink, item),
        "@setRepeat" => crate::cry_trigger::run_owed_repeat(sink, item),
        "@extraCry" => crate::cry_trigger::run_owed_extra_cries(sink, item),
        "@melee" => crate::combat::run_owed_melee(sink, item),
        _ => {
            // A sequence a test made (`testkit::seams::register_work_handler`), before a card's step.
            #[cfg(feature = "testkit")]
            if let Some(handler) = crate::testkit::seams::work_handler(&item.resume.hook) {
                handler(sink, item);
                return;
            }
            // A card's own continuation, re-entered at the step it names with the data it captured.
            if card_step_for(sink.state, &item.resume).is_some() {
                crate::prompts::run_resume(
                    sink,
                    &item.resume,
                    crate::prompts::ResumeOptions {
                        controller: Some(item.owner),
                        ..crate::prompts::ResumeOptions::default()
                    },
                );
                return;
            }
            let of = if item.resume.def_id.is_empty() {
                String::new()
            } else {
                format!(" of {}", item.resume.def_id)
            };
            panic!(
                "no handler for owed work \"{}\" step \"{}\"{of} (R113)",
                item.resume.hook, item.resume.step
            );
        }
    }
}

/// Run the next owed item, if any, and report whether one ran. Nothing runs while a prompt is open or
/// the game is over: the answer is another action, and the pause is the caller's to handle.
pub fn run_next_work(sink: &mut EngineSink<'_>) -> bool {
    if paused(sink) {
        return false;
    }
    let Some(head) = sink.state.work.first() else {
        return false;
    };
    if sink
        .owed_behind
        .as_ref()
        .is_some_and(|behind| behind.contains(&head.id))
    {
        return false;
    }
    let Some(next) = take_work(sink.state) else {
        return false;
    };
    // R117: what is owed behind this item is the enclosing sequences', which no resolution loop inside
    // it (a play's step-4 loop, a turn stage's settle) may take or re-run. Items this one parks go ahead
    // of them (R113); only the drain that took this item comes back for them, once it is done.
    let outer = sink.owed_behind.take();
    let behind: IndexSet<String> = sink.state.work.iter().map(|item| item.id.clone()).collect();
    sink.owed_behind = Some(behind);
    run_work_item(sink, &next);
    sink.owed_behind = outer;
    true
}

/// Continue the sequences a prompt interrupted, in R113's order, until all are done or one opens a
/// prompt of its own (parking its own tail, so the rest keeps waiting in state). Returns whether the
/// queue is empty; `triggers::settle` drains here before it pops a trigger (§10.3, R59).
pub fn drain_work(sink: &mut EngineSink<'_>) -> bool {
    begin_work_cascade(sink);
    for _ in 0..MAX_WORK_STEPS {
        if paused(sink) {
            return !has_work(sink.state);
        }
        if !run_next_work(sink) {
            return true;
        }
    }
    panic!("owed work did not drain in {MAX_WORK_STEPS} steps (§9.3)");
}
