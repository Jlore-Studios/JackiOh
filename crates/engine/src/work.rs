//! Unfinished engine work: the resumable-work queue (SPEC §9.3, §10.3, §10.6, R113).
//!
//! Several engine sequences do one thing after another where any step can open a prompt: the play
//! pipeline of §10.5, the attack window of §4.2 step 4, the Death hooks of §4.5 step 3, the Reborn
//! returns of §4.5 step 4, the end-of-turn hooks of §2.2, and any card's own effect list. A prompt
//! ends the action — "mid-action choices are state, not callbacks" (§9.3) — and the answer arrives
//! as a fresh action with a fresh event list, so a sequence that kept its position in a local
//! variable lost it and silently dropped the rest. That is the bug this module exists to prevent:
//! any engine sequence that spans a prompt must be resumable through `state.work`.
//!
//! So a scope that has to stop parks what it still owes on `state.work` and returns, and the
//! resolution loop of §10.3 drains it after the answer. Every owed item is plain data — a `Resume`
//! naming the continuation, the step to pick up at and the payload it captured — so a paused state
//! survives `JSON.parse(JSON.stringify(state))` and replays exactly (§9.3, §10.1). Holding the
//! remaining `Effect[]` instead would be holding closures, which §9.3 forbids; a hook is a pure
//! builder (CLAUDE.md rule 5), so re-entering it and skipping the effects that already ran
//! continues the same sequence.
//!
//! R113, the order things resume in, is neither a queue nor a stack, because both are wrong. Take a
//! Cry whose effect list is `[damage, chooseTarget, damage]`, paused inside play step 5:
//!
//!   - the Cry parks its tail (W1) and then the pipeline parks steps 6-8 (W2). W1 must run first:
//!     step 6 comes *after* step 5, not inside it. A stack would bury the Spell before its own
//!     effect finished.
//!   - W1 then resumes and opens a second prompt, parking W3. W3 is still inside step 5, so it must
//!     precede W2. A plain queue would run step 6 too early.
//!
//! So: `state.work_cursor` is where the pause cascade that is running now parks its next item, and it
//! advances with each park, which lands one cascade's items innermost-first; taking an item (or
//! entering a drain, which is the boundary between one action's cascade and the next) resets the
//! cursor to 0, so a pause that happens *during* a resumption is inserted ahead of everything still
//! owed.
//!
//! Against R68 and R59: work is drained *before* the trigger queue moves (`triggers::settle` does
//! this), because an interrupted sequence is the action still finishing rather than a fresh reaction
//! to it — a queued trigger only ever starts once nothing is owed. The state check is the caller's
//! (R59: it runs between whole effects and whole triggers, never between the hits of one), so this
//! module never calls it.
//!
//! Port of `packages/engine/src/work.ts` (part 3). TS's handler registry (`registerWorkHandler`,
//! `registerDefaultWorkHandler`), which existed only to break import cycles, is one `match` on
//! `resume.hook` here (SURFACE §6.6, research A §2.7): `run_work_item` calls the module that owns
//! each engine sequence directly, and every other hook is a card's own step, re-entered through
//! `prompts::run_resume` (TS's default handler, `prompts.ts:912`). TS's `DrainSink.owedBehind`, a
//! transient field TS added to the sink object, is `EngineSink::owed_behind`.

use indexmap::{IndexMap, IndexSet};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::MAX_WORK_STEPS;
use crate::script::{ACTIVATION_HOOK_PREFIX, EngineSink, Hook, Script, activation_decls, hook};
use crate::state::{EventStay, GameState, Resume, WorkItem};
use crate::wire::{GameEvent, PlayerId, Selection};

/// Which sequence an item belongs to: `resume.hook`. An engine sequence uses a name no card
/// `Script` has (`play_steps::PLAY_WORK_KIND`), a card continuation uses the hook whose step runs
/// (`"resume"`, `"cry"`, `"delayed"`), and `resume.step` says where in it to pick up.
pub type WorkKind = str;

/// A continuation plus whose sequence it is. `prompts::ResumePlan` is this type; the owner is the
/// controller the re-entered step runs as, which a `Resume` alone does not carry. (TS
/// `Resume & { owner }`, its fields written out.)
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
    /// `{ ...resume, owner }`.
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

    /// The plan's `Resume` half.
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
    /// The parts of a composed list the pause stood inside (`Effect.expand`), outermost first: the
    /// index of the part at each level. Absent for a pause in a card's own list. The continuation
    /// builds those parts again, finishes the innermost list from `from`, and then goes on with each
    /// enclosing list after the part it stood in (`prompts::apply_resumable`, R102, R113).
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

/// Where the continuation a prompt stores (`prompts::resume_self`) keeps what it carries of the run it
/// continues, apart from the card's own data: the answered step is the same run as the list that
/// asked (R113, §10.6), so it reads the stays that run began with (R174) and the units it summoned
/// (R136), whichever action it resumes in.
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

/// A JSON number read back as a count or a mark (`typeof x === "number"`).
fn number_u32(value: Option<&Value>) -> Option<u32> {
    value.and_then(Value::as_u64).map(|n| n as u32)
}

/// A JSON array's strings, the rest dropped (`filter((id): id is string => typeof id === "string")`).
fn strings_in(value: &[Value]) -> Vec<String> {
    value
        .iter()
        .filter_map(|id| id.as_str().map(str::to_string))
        .collect()
}

/// An `EventStay` read back out of stored JSON, or `None` when there is none.
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

// ---------------------------------------------------------------------------------------------
// The stored control block
// ---------------------------------------------------------------------------------------------

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

/// R102: which ingredient's text of a fused card is running, as the path of ingredient indices from
/// the outermost fusion in (a card fused from a fused card nests). `subsystems::fuse` writes it into
/// the context each part of a combined hook builds and applies with, so what that text leaves behind
/// is that ingredient's own: the continuation a prompt of its stores (`prompts::resume_self` copies the
/// card's data) comes back to its step alone, and what it remembers stays apart from what another
/// ingredient remembers under the same name (`effects::memory`).
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

/// The keys `REMEMBERED_KEY` notes, strings only.
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

/// Whether `stored` is `key` itself or one of its ingredient keys `key@…`.
fn is_key_or_part(stored: &str, key: &str) -> bool {
    stored == key || stored.strip_prefix(key).is_some_and(|rest| rest.starts_with('@'))
}

/// R102, R77: a card a Fuse keeps becomes ingredient `index` of the new fusion, and what its texts
/// remembered goes with them: its own `key` becomes `key@<index>`, and a key an earlier fusion's part
/// wrote, `key@<path>`, becomes `key@<index>.<path>` — the path that text now runs at. So the card
/// reads back what it remembered, and the ingredients fused onto it, which remember nothing yet, read
/// nothing of it: a Cube kept under a played Cube copies its one meal once, not once per Cube text.
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
            // `delete memory[stored]`, then `memory[key@path] = value`: a new key goes last, one that
            // is there already keeps its place (as a JS object's do). A value an earlier rename of
            // this walk already took is `undefined` in TS, which a state's JSON never holds.
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

/// R102, R384: the memory as ingredient `index`'s text reads it from a hook that carries no context
/// to name its place (an Activate ability's `canActivate` and `has`) — the inverse of
/// `reroot_remembered`: what that text remembered, `key@<index>`, under `key`, what the ingredients
/// fused into it remembered, `key@<index>.<path>`, under `key@<path>`, and what the other ingredients
/// remembered left out. The engine's own entries, which no text remembered, stay as they are.
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
        // `out[k] = value`: in place when `out` still holds `k`, else last, as a JS object's keys go.
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

// ---------------------------------------------------------------------------------------------
// Parking (R113)
// ---------------------------------------------------------------------------------------------

/// Start a fresh pause cascade: the next park goes ahead of everything still owed (R113). Taking an
/// item and entering `drain_work` both do this, so a reducer that settles after every action gets it
/// for free; call it explicitly when a cascade begins somewhere else.
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
/// `state.next_seq` (R68's creation order), which is in state, so the queue a replay builds is the
/// queue the live game had; `owner` defaults to the active player.
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

/// Park one owed continuation without taking a number: its id borrows `state.next_seq` without moving
/// it. R177: for an item whose existence hangs on a card someone may not read — the end-of-turn trap
/// window's remainder exists only when a trap still to be offered the event watches it, and a
/// face-down trap is read by its controller alone (R33) — so the ids the counter hands the modifiers
/// next (R169) say nothing about it. Nothing orders or finds a work item by its `seq`.
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

/// One thing a pause owes (TS `Resume | WorkItem`): a `Resume`, parked as a new item, or a whole
/// `WorkItem`, parked again as it stands.
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

/// What a pause still owes. A `Resume` is parked as a new item; a whole `WorkItem` is parked again
/// as it stands, keeping its id and `seq`, which is what a handler that has to wait behind another
/// one needs. (TS took any number of them; every caller parks one, and `owe_all` parks several.)
pub fn owe(sink: &mut EngineSink<'_>, item: impl Into<OweItem>) -> Vec<WorkItem> {
    owe_all(sink, vec![item.into()])
}

/// `owe(sink, ...items)`: each in order.
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

// ---------------------------------------------------------------------------------------------
// Reading and taking
// ---------------------------------------------------------------------------------------------

pub fn has_work(state: &GameState) -> bool {
    !state.work.is_empty()
}

/// The next item without taking it.
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

/// Take the next owed item, or `None` when nothing is owed. Taking one ends the cascade that
/// parked it, so the cursor resets: whatever the resumed item parks goes ahead of the rest (R113).
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

// ---------------------------------------------------------------------------------------------
// Running
// ---------------------------------------------------------------------------------------------

/// Where a continuation of an event trigger keeps the event it answers (`triggers::queue_trigger`).
pub const EVENT_KEY: &str = "event";

/// An event trigger's own list, re-entered by its id (R113). A `TriggerDef` lives in a list
/// (`triggers`, `hand_triggers`, `deck_triggers`, `graveyard_triggers`), not under a `Script` key, so
/// a trigger or a trap whose list asks mid-list parks its tail under the trigger's id, and the tail is
/// rebuilt here from the event the continuation carries in its data.
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

/// R384: an Activate ability's own list, re-entered by its id (`script::activation_hook`). The ability
/// lives in a list (`Script.activations`), like a trigger, so a tail its prompt parked comes back
/// here by the id the hook carries.
fn activation_step_for(script: &Script, resume: &Resume) -> Option<Hook> {
    let id = resume.hook.strip_prefix(ACTIVATION_HOOK_PREFIX)?;
    activation_decls(script)
        .into_iter()
        .find(|decl| decl.id == id)
        .map(|decl| decl.run)
}

/// The step a script registers for a continuation: a hook of its own (`cry`, `delayed`), an entry
/// in its step table (`resume: { picked: … }`), an event trigger named by its id, or an Activate
/// ability named by its id (R384). `prompts.rs` re-enters a continuation through this, and
/// `can_resume` asks it, so the two cannot disagree.
///
/// TS read `script[resume.hook]` by name: a function there is the step, an object's entry under
/// `resume.step` is the step. The effect-list hooks are `Script::hook_named`'s, and the one object of
/// steps is `resume`; no continuation names any other key.
pub fn script_step_for(script: &Script, resume: &Resume) -> Option<Hook> {
    // R1241: Meditative #95's rest-of-game effect is the engine's own step, whatever script it names: a
    // start-of-turn effect re-enters through here and never reaches `run_engine_delayed`.
    if resume.hook == crate::subsystems::CHAOS_ETERNAL_HOOK {
        return Some(crate::subsystems::eternal_step());
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

/// The engine sequences `run_work_item` runs itself, by `resume.hook` (research A §2.7; TS's
/// `registerWorkHandler` calls).
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
    "@afterAttack",
    "@startOfTurn",
    "@endOfTurn",
    "@activate",
    "@aiTurn",
    "@setRepeat",
];

/// Whether anything at all knows how to resume this item (R113).
pub fn can_resume(state: &GameState, resume: &Resume) -> bool {
    if ENGINE_WORK_HOOKS.contains(&resume.hook.as_str()) {
        return true;
    }
    // A sequence a test made (`testkit::seams::register_work_handler`, TS's `registerWorkHandler`).
    #[cfg(feature = "testkit")]
    if crate::testkit::seams::work_handler(&resume.hook).is_some() {
        return true;
    }
    card_step_for(state, resume).is_some()
}

/// Run one owed item: the module that owns its sequence, or the card-continuation runner when the
/// card's own script has that step (R113). When neither knows the hook this raises, because a work
/// item that cannot be resumed is a lost sequence — a Spell that never reaches the graveyard, an
/// Echo repeat that never happens — and must never be dropped in silence.
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
        "@afterAttack" => crate::combat::run_owed_after_attack(sink, item),
        "@combatCopies" => crate::combat::run_owed_combat_copies(sink, item),
        "@startOfTurn" => crate::turn::run_owed_start_of_turn(sink, item),
        "@endOfTurn" => crate::turn::run_owed_end_of_turn(sink, item),
        "@activate" => crate::subsystems::activate::run_owed_activation(sink, item),
        "@aiTurn" => crate::subsystems::ai_policy::run_owed_ai_turn(sink, item),
        "@setRepeat" => crate::cry_trigger::run_owed_repeat(sink, item),
        _ => {
            // A sequence a test made (`testkit::seams::register_work_handler`): TS's handler map, read
            // before its default handler.
            #[cfg(feature = "testkit")]
            if let Some(handler) = crate::testkit::seams::work_handler(&item.resume.hook) {
                handler(sink, item);
                return;
            }
            // TS's default handler (`prompts.ts:912`): a card's own continuation, re-entered at the
            // step it names with the data the pause captured.
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

/// Run the next owed item, if any, and report whether one ran, so a resolution loop can keep going
/// until nothing is left. Nothing runs while a prompt is open or the game is over: the answer is
/// another action, and what to do with the pause is the caller's decision.
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
    // R117: what is owed behind this item is the enclosing sequences', and a resolution loop running
    // inside it — a play's step-4 loop, a turn stage's settle — can neither take nor re-run it. The
    // items this one parks go ahead of them (R113), so they stay the queue's tail; only the drain that
    // took this item comes back for them, once the item is done.
    let outer = sink.owed_behind.take();
    let behind: IndexSet<String> = sink.state.work.iter().map(|item| item.id.clone()).collect();
    sink.owed_behind = Some(behind);
    run_work_item(sink, &next);
    sink.owed_behind = outer;
    true
}

/// Continue the sequences a prompt interrupted, in R113's order, until they are all done or one of
/// them opens a prompt of its own — which parks its own tail, so the rest keeps waiting in state.
/// Returns whether the queue is empty. The state check and the trigger queue are the caller's
/// (§10.3, R59); `triggers::settle` drains here first and only then pops a trigger.
pub fn drain_work(sink: &mut EngineSink<'_>) -> bool {
    // Entering a drain is the boundary between the cascade that parked this work and the next one.
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

/// `drain_work` for a caller that does not read the result.
pub fn run_work(sink: &mut EngineSink<'_>) {
    drain_work(sink);
}
