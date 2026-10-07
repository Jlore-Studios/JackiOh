//! Trap matching and immediate resolution (SPEC §5.1, §10.3, §2.2's end-of-turn window; BUILD M3-T2).
//!
//! §10.3: "Traps are checked before other triggers because they are responses (the end-of-turn trap
//! window of §2.2 is the one scheduled exception, R62); a trap that fires during the opponent's turn
//! resolves to completion (including forced attacks and prompts for the trap's owner) before the
//! opponent's action continues."
//!
//! This module owns four things and nothing else:
//!   1. matching  — which backrow traps watch a given event (`traps_watching`), in R68 order;
//!   2. firing    — emit `trapFired`, run the trigger to completion, run the state check
//!                  (`fire_traps_for`, `run_trap_window`);
//!   3. consuming — a Trap goes to its owner's graveyard, a Field Trap stays and is face-up (R33)
//!                  (`consume_trap`);
//!   4. owing     — a window a prompt interrupted parks the traps that have not seen the event yet
//!                  on `state.work`, so the answer finishes the window (`TRAP_WINDOW_WORK`, R113),
//!                  and a trap whose own list asks parks the rest of its firing — its other
//!                  triggers, its consumption and its check — behind that list (`TRAP_FIRING_WORK`).
//!
//! The *when* is the caller's: `triggers.rs` offers every freshly emitted event to `fire_traps_for`
//! before it queues any ordinary trigger, and `turn.rs` calls `run_trap_window` at R62's scheduled
//! point — after the end-of-turn triggers and before the end-of-turn delayed effects. R17's two
//! moments are likewise the play pipeline's (§10.5): Sheepish fires on the `summoned`/`cardPlayed`
//! pair emitted at step 4, before the Cry of step 5; Bear Honeypot, Unstable Clone Machine and
//! Unlicensed Experimentation fire on `cardResolved`, which step 7 emits once per play or cast after
//! the Cry and every Echo repeat, and whose `permanent` flag is R61's "played permanents only".
//! `cardResolved` needs no list here — a trigger that names an event type is matched on it — but it
//! does need R119: a trap is a played card too, so `traps_watching` never offers a trap the arrival
//! event that names the trap itself. And it travels the immediate path alone: R100 keeps the
//! scheduled window's events to the window, and `cardResolved` is not one of them.
//!
//! A window is a sequence that can span a prompt, so it is resumable through `state.work` and
//! through nothing else (§9.3, R113). Neither of the two dispatch paths may ever *drop* an event it
//! has not finished delivering: the immediate one owes the rest of its traps as `triggers.rs`'s
//! front-of-queue `OWED_TO_TRAPS` entry, and the scheduled window owes its remainder here as a
//! `TRAP_WINDOW_WORK` item — the event plus the ids of the traps still to be offered it, all plain
//! JSON. A bare `if state.pending.is_some() { return; }` would be the bug and not the fix: it strands
//! the rest of the window with no way back, which is worse than firing late.
//!
//! Port of `packages/engine/src/traps.ts` (part 3). TS's two `registerWorkHandler` calls are not
//! ported (SURFACE §6.6): `work.rs`'s dispatcher calls `run_owed_firing` (`"@trapFiring"`) and
//! `run_owed_window` (`"@trapWindow"`) directly. Nor is `registerDeclarationCheck`: the declaration
//! check is a direct call to `combat::declared_attack_stands`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::script::{EngineSink, Script, TriggerDef, empty_script};
use crate::state::{CardInstance, GameState, Resume, WorkItem, find_instance, find_instance_mut};
use crate::stays::LaterMoves;
use crate::wire::{GameEvent, GameEventType, PlayerId, Row, Zone, ZoneName, ZoneRef};

/// A trap trigger, and the two rows that settle what firing means.
///
/// R99: a trap fires when its trigger's `on` matches the event **and** its `when` predicate admits
/// it. The predicate is kept out of the effect list on purpose, because R61 makes `run` returning
/// `[]` mean "fired, consumed, did nothing" — so a trap whose condition simply was not met cannot
/// say so through `run` and would be spent by an event it should ignore. A condition that must leave
/// the trap armed (Bear Honeypot's "costing 1 or less", My Pawn's "would be lethal", Sheepish's
/// own-side summon, Unlicensed Experimentation's "whose type matches one you control") therefore
/// belongs in `when`, never in `run`. A trigger that declares no predicate answers every event it
/// names, whichever side caused it (R99), and with the predicate satisfied an empty effect list
/// still spends the trap (R61).
///
/// `when` is declared on `script.rs`'s `TriggerDef` now, so this alias adds nothing to it: reading
/// the predicate below is the declared contract, not a structural workaround. The name stays because
/// the card files annotate their trap triggers with it (#18, #41, #60, #71, #85, #96).
pub type TrapTrigger = TriggerDef;

/// One trap and the triggers of its own script that this event woke.
#[derive(Clone, Debug)]
pub struct TrapMatch {
    pub trap: CardInstance,
    pub triggers: Vec<TrapTrigger>,
}

/// What a dispatch did: the traps that fired, and whether a prompt paused the rest (§10.3).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TrapDispatch {
    /// Instance ids, in the order they resolved.
    pub fired: Vec<String>,
    /// A prompt (or the end of the game) stopped the dispatch before it finished.
    pub paused: bool,
}

/// A dispatch as the module reads it internally: `owed` names the traps the pause stopped it from
/// offering the event to at all, which is what a resume needs and what neither caller may guess by
/// subtracting `fired` — a trap whose `when` declined has seen the event and is not owed it again.
/// (TS `TrapDispatch & { owed }`.)
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct TrapRun {
    fired: Vec<String>,
    paused: bool,
    owed: Vec<String>,
}

/// R212: the player each trap a dispatch offers the event to answers it for — the one who controlled
/// it when the event happened, read as the dispatch begins and kept, like the owed list, across any
/// pause. A trap an earlier effect stole between the event and its dispatch — a cast's events wait
/// for the list that cast it (R70), and the end-of-turn window offers `turnEnded` to one trap after
/// another (R62) — still answers for the player who held it then, and its tokens are theirs (R52).
pub type TrapControllers = IndexMap<String, PlayerId>;

/// An immediate dispatch, as `triggers.rs` parks and resumes it: the traps still owed the event, in
/// the order the dispatch offered it (R68, fixed as it began), who each answers for, and the stays
/// it began with (R174). (TS `TrapDispatch & { owed; controllers; mark }`.)
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImmediateDispatch {
    pub fired: Vec<String>,
    pub paused: bool,
    pub owed: Vec<String>,
    pub controllers: TrapControllers,
    pub mark: u32,
}

/// R113: the `resume.hook` of the one work item this module parks — the rest of an end-of-turn trap
/// window. It is an engine sequence, not a card's, so the name is one no `Script` can hold, and
/// `run_owed_window` below is the dispatcher's arm for it (`work.rs`): `work::run_work_item` panics
/// on a hook nothing knows, and a window that cannot be resumed is exactly the lost sequence that
/// rule exists to prevent.
pub const TRAP_WINDOW_WORK: &str = "@trapWindow";

/// The window has one step, named so a reader of `state.work` can see what is owed.
const TRAP_WINDOW_STEP: &str = "window";

/// What a parked window carries: the event it is delivering, and who has not seen it yet.
#[derive(Clone, Debug, PartialEq)]
pub struct OwedWindow {
    pub event: GameEvent,
    /// Instance ids of the traps the window still has to offer the event to, in R68 order.
    pub owed: Vec<String>,
    /// R212: who each of them answers for — its controller when the window opened.
    pub controllers: Option<TrapControllers>,
    /// R174: the field's departures when the window opened (`stays::exit_mark`).
    pub mark: Option<u32>,
}

/// R100: the events the *scheduled* end-of-turn window owns. An event the window is scheduled to
/// deliver is not also offered to the immediate check, so Bread and Butter and Intern Stimmy fire
/// once per turn end, in the window, on both sides — and not a second time as ordinary responses.
/// This list is the whole of that withholding, and R62's end-of-turn order depends on it.
pub const TRAP_WINDOW_EVENTS: &[GameEventType] = &[GameEventType::TurnEnded];

pub fn is_trap_window_event(event: &GameEvent) -> bool {
    TRAP_WINDOW_EVENTS.contains(&event.event_type())
}

/// §5.1: a Field Trap is a Trap that stays after firing, so both are "a Trap" (R61). Read off the
/// running face's own type (B2.7, `animated::face_type_of`) and never off `faces::card_type_of`: an
/// animated Field Trap is a Unit where it stands (R383) and still fires as the trap its face is.
pub fn is_trap_type(state: &GameState, instance: &CardInstance) -> bool {
    let face = crate::animated::face_type_of(state, instance);
    face == crate::wire::CardType::Trap || face == crate::wire::CardType::FieldTrap
}

pub fn is_field_trap(state: &GameState, instance: &CardInstance) -> bool {
    crate::animated::face_type_of(state, instance) == crate::wire::CardType::FieldTrap
}

fn is_on_field(instance: &CardInstance) -> bool {
    instance.zone.z() == ZoneName::Field
}

/// §3.2, R447: a trap acts only on top of its zone — one a Stack card buried since it was matched never fires.
fn acts_as_trap(state: &GameState, instance: &CardInstance) -> bool {
    is_on_field(instance) && crate::zones::acts_on_field(state, instance)
}

/// Every trap on the field, in R68 order: the active player's cards first, then the opponent's; on each
/// side the unit lanes 1–5, then the backrow lanes 1–5. A trap sits in a backrow zone (§5.1) unless it
/// has animated (R383, B3.1 rule 3): an animated Field Trap stands in a unit zone as a Unit and keeps its
/// text, so it fires from there, in the unit lanes of R68's order (Classic #5 Tesla keeps zapping). Only
/// the card acting in a zone is one (§3.2): a trap dormant under a backrow pile never fires (R447).
pub fn traps_in_order(state: &GameState) -> Vec<CardInstance> {
    let sides: [PlayerId; 2] = if state.active == PlayerId::P1 {
        [PlayerId::P1, PlayerId::P2]
    } else {
        [PlayerId::P2, PlayerId::P1]
    };
    let mut out: Vec<CardInstance> = Vec::new();
    for player in sides {
        for slot in crate::zones::slots_of(player, Row::Units) {
            if let Some(card) = crate::zones::card_at(state, &slot)
                && crate::animated::is_animated(state, &card)
                && is_trap_type(state, &card)
            {
                out.push(card.clone());
            }
        }
        for slot in crate::zones::slots_of(player, Row::Backrow) {
            if let Some(card) = crate::zones::card_at(state, &slot)
                && is_trap_type(state, &card)
            {
                out.push(card.clone());
            }
        }
    }
    out
}

fn trap_triggers_of(state: &GameState, trap: &CardInstance) -> Vec<TrapTrigger> {
    crate::scripts::script_of(state, trap).triggers
}

/// R119: the events that say a card arrived. A card does not answer the play that put it onto the
/// field. Not because it was absent when the play began — §10.5 step 4 places it and only then emits
/// `cardPlayed`, so it is already a registered watcher by the time its own arrival is dispatched.
/// The exclusion is deliberate, and for traps it lives here: a trap is never offered the
/// `cardPlayed`, `summoned` or `cardResolved` event that names the trap itself. R17's two moments are
/// both in this list, which is why the rule belongs here and not in each card's predicate: Sheepish
/// answers step 4's `summoned`/`cardPlayed` pair, and Bear Honeypot, Unstable Clone Machine and
/// Unlicensed Experimentation answer step 7's `cardResolved` — and a Trap or Field Trap is itself a
/// card someone plays, so without this a trap watching either moment answers its own arrival.
const ARRIVAL_EVENTS: &[GameEventType] = &[
    GameEventType::CardPlayed,
    GameEventType::Summoned,
    GameEventType::CardResolved,
];

/// TS `(event as { instanceId?: unknown }).instanceId`, for the arrival events.
fn arrival_id(event: &GameEvent) -> Option<&str> {
    match event {
        GameEvent::CardPlayed { instance_id, .. }
        | GameEvent::Summoned { instance_id, .. }
        | GameEvent::CardResolved { instance_id, .. } => Some(instance_id.as_str()),
        _ => None,
    }
}

fn is_own_arrival(trap: &CardInstance, event: &GameEvent) -> bool {
    if !ARRIVAL_EVENTS.contains(&event.event_type()) {
        return false;
    }
    if arrival_id(event) == Some(trap.id.as_str()) {
        return true;
    }
    // R119: nor the play the trap arrived on the field during, whatever put it there — #95 summoning a
    // Bear Honeypot face-down does not have that Honeypot answer #95's own `cardResolved`, and a
    // Sheepish a tributed Cube's Death copied at step 2 does not answer the `cardPlayed` of the play
    // that paid the Tribute.
    arrived_during_play(event).contains(&trap.id)
}

/// R119: the arrivals a play's `cardPlayed`, `summoned` or `cardResolved` names, which it does not answer.
pub fn arrived_during_play(event: &GameEvent) -> &[String] {
    match event {
        GameEvent::CardPlayed { arrived_during, .. }
        | GameEvent::Summoned { arrived_during, .. }
        | GameEvent::CardResolved { arrived_during, .. } => arrived_during.as_deref().unwrap_or(&[]),
        _ => &[],
    }
}

/// The traps this event woke, matched by the event type each trigger names. This is matching only:
/// a `when` predicate needs an `EffectContext`, so it is evaluated when the trap fires.
///
/// Any event type a trigger names is matched, `cardResolved` included — R17's "after the card
/// resolves", which §10.5 step 7 emits once per play or cast. It reaches the traps through the
/// immediate path (`fire_traps_for`), never the scheduled one: R100 keeps the two disjoint, and only
/// `TRAP_WINDOW_EVENTS` belongs to the window.
pub fn traps_watching(state: &GameState, event: &GameEvent) -> Vec<TrapMatch> {
    let wakes = event.event_type();
    traps_in_order(state)
        .into_iter()
        .filter_map(|trap| {
            if is_own_arrival(&trap, event) {
                return None;
            }
            if is_spent(state, &trap) {
                return None;
            }
            let triggers: Vec<TrapTrigger> = trap_triggers_of(state, &trap)
                .into_iter()
                .filter(|trigger| trigger.on.contains(&wakes))
                .collect();
            if triggers.is_empty() {
                None
            } else {
                Some(TrapMatch { trap, triggers })
            }
        })
        .collect()
}

/// §5.1: a Trap "fires …, then goes to the graveyard"; only a Field Trap "stays after firing and can
/// fire again". A Trap is face-down until it fires (R33), so a face-up one that is still in the
/// backrow is one whose firing has not finished — #96 My Pawn's effects are a whole AI turn — and it
/// answers nothing more: it has fired, and firing is once. `fire_trap` turns it face-up before its
/// effects run for exactly this reason.
pub fn is_spent(state: &GameState, trap: &CardInstance) -> bool {
    trap.face_up == Some(true) && !is_field_trap(state, trap)
}

/// §3.2: "backrow cards go to the owner's graveyard when their effect ends (traps after firing
/// unless Field Trap)". R33: a Field Trap that has fired is face-up to both players from then on.
/// R61 makes this unconditional: a trap that fired is consumed whether or not its effects achieved
/// anything, so the caller never asks what the effect list managed to do.
pub fn consume_trap(sink: &mut EngineSink<'_>, instance: &CardInstance) {
    let mut card = find_instance(sink.state, &instance.id)
        .cloned()
        .unwrap_or_else(|| instance.clone());
    card.face_up = Some(true);
    if let Some(live) = find_instance_mut(sink.state, &instance.id) {
        live.face_up = Some(true);
    }
    if is_field_trap(sink.state, &card) {
        return;
    }
    // R383 (B3.1 rules 2 and 4): an Animated Trap's firing ends by animating it, so it is a Unit in a unit
    // zone now, or it could not for want of an open one and stays in its zone face-up, as a Field Trap
    // does — either way it does not go to the graveyard. Spent all the same (`is_spent`): a Trap fires once.
    if crate::animated::is_animated(sink.state, &card)
        || crate::animated::animated_kind_of(sink.state, &card).is_some()
    {
        return;
    }

    // B5 E5: its graveyard, or wherever a replacement sends it (Classic #50 Voidwalker's exile).
    let result = crate::zones::move_to_zone(
        sink.state,
        &mut card,
        crate::zones::OffFieldZone::Graveyard,
        Default::default(),
    );
    let landed = find_instance(sink.state, &card.id).cloned().unwrap_or(card);
    crate::zones::report_graveyard_landing(sink, &landed, result);
}

/// R152, §3.2, §5.1: #96 My Pawn's effect is the rest of the turn it handed to the AI, so that
/// effect ends at the cleanup of that turn (`turn.rs` calls this there) — and a Trap "goes to the
/// graveyard" when its effect ends. The trap is still face-up in its backrow (`is_spent`), because
/// `fire_trap` is still on the stack underneath the AI's playout, and the next turn begins inside that
/// same playout: consuming it only once the playout returns left it in the backrow through the
/// opponent's start of turn, where #37 Gravedigger could not find it in the graveyard. A trap its own
/// AI turn moved off the field (exiled, destroyed) is no longer in a backrow and is not touched.
pub fn end_handed_over_turn(sink: &mut EngineSink<'_>) {
    for trap in traps_in_order(sink.state) {
        if is_spent(sink.state, &trap) {
            consume_trap(sink, &trap);
        }
    }
}

/// Fire one trap: emit `trapFired`, run every trigger of its that this event woke, run the state
/// check so the trap has resolved to completion, then consume it. The trap is consumed whatever its
/// effects achieved — an Immutable Sheepish target or a Fuse with no legal target still spends it
/// (R17, R61). Returns false when no trigger's `when` admitted the event, which leaves the trap
/// armed and face-down (R99) — the one outcome that is not a firing.
///
/// `controller` defaults to the trap's own (TS's default parameter).
pub fn fire_trap(
    sink: &mut EngineSink<'_>,
    trap_match: &TrapMatch,
    event: &GameEvent,
    controller: Option<PlayerId>,
) -> bool {
    let trap = &trap_match.trap;
    let controller = controller.unwrap_or(trap.controller);
    let armed: Vec<String> = {
        let mut ctx = crate::resolve::make_context(
            sink,
            Some(trap),
            crate::resolve::HookOptions {
                controller: Some(controller),
                ..Default::default()
            },
        );
        trap_match
            .triggers
            .iter()
            .filter(|trigger| trigger.when.as_ref().is_none_or(|when| when(&mut ctx, event)))
            .map(|trigger| trigger.id.clone())
            .collect()
    };
    if armed.is_empty() {
        return false;
    }

    // R154: the zone is read before the trap resolves, because firing it can move the card — a Trap
    // reaches its owner's graveyard on consumption and would then have no slot to report.
    let at = crate::zones::slot_of(sink.state, trap);
    // `controller` names the seat that reads the trap's identity (R154), and a trap stolen since the
    // event is face-down on its new controller's side: the flip is announced to the seat it sits on.
    sink.events.push(GameEvent::TrapFired {
        instance_id: trap.id.clone(),
        def_id: trap.def_id.clone(),
        controller: trap.controller,
        row: at.map_or(Row::Backrow, |slot| slot.row),
        lane: at.map_or(0, |slot| slot.lane),
    });

    // §5.1: it has fired, and it is face-up from this moment (R33) — which also keeps it out of every
    // dispatch its own effects start, since a Trap fires once (`is_spent`).
    if let Some(live) = find_instance_mut(sink.state, &trap.id) {
        live.face_up = Some(true);
    }
    run_armed_triggers(
        sink,
        TrapFiring {
            trap_id: trap.id.clone(),
            // R212, R52: everything the trap does is the player's it answered for.
            controller,
            triggers: armed,
            at: 0,
            event: event.clone(),
        },
    );
    true
}

/// What is left of one trap's firing: its armed triggers from `at` on, then its end — consumed, and
/// the state check that makes it "resolve to completion" (§10.3). All JSON, so a firing a prompt
/// interrupted is owed on `state.work` as `TRAP_FIRING_WORK` and survives the answer (R113).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct TrapFiring {
    trap_id: String,
    controller: PlayerId,
    triggers: Vec<String>,
    at: usize,
    event: GameEvent,
}

/// R113: the `resume.hook` of a trap's firing that a prompt interrupted — the triggers it has not
/// run and its end. §10.3 has a trap "resolve to completion (including … prompts for the trap's
/// owner) before the opponent's action continues", and a trap's list is an effect list like any
/// other, so it runs resumably: the effects after one that asks are parked by
/// `prompts::apply_resumable` (re-entered by the trigger's id, `work::script_step_for`), and this item
/// behind them owes the rest. Running the list straight through walked on over the open prompt,
/// dropped a second one (`open_prompt` never overwrites) and consumed the trap and ran the check in
/// the middle of its effect.
pub const TRAP_FIRING_WORK: &str = "@trapFiring";

/// The armed triggers of one firing, from `firing.at`, each with the trap's controller as "you"
/// (R52), then the trap's end. Stops at a pause and owes the rest (R117: at the pause, never before).
fn run_armed_triggers(sink: &mut EngineSink<'_>, firing: TrapFiring) {
    for at in firing.at..firing.triggers.len() {
        if sink.state.result.is_some() {
            return;
        }
        let Some(trap) = find_instance(sink.state, &firing.trap_id).cloned() else {
            continue;
        };
        let Some(trigger) = trap_triggers_of(sink.state, &trap)
            .into_iter()
            .find(|def| def.id == firing.triggers[at])
        else {
            continue;
        };
        let mut data: IndexMap<String, Value> = IndexMap::new();
        data.insert(
            crate::work::EVENT_KEY.to_string(),
            serde_json::to_value(&firing.event).expect("an event is plain JSON (§10.1)"),
        );
        let before = sink.state.pending.clone();
        let plan = crate::work::WorkPlan::new(
            Resume {
                def_id: trap.def_id.clone(),
                hook: trigger.id.clone(),
                step: String::new(),
                radiant: trap.radiant,
                instance_id: Some(trap.id.clone()),
                data: data.clone(),
            },
            firing.controller,
        );
        {
            let mut ctx = crate::resolve::make_context(
                sink,
                Some(&trap),
                crate::resolve::HookOptions {
                    controller: Some(firing.controller),
                    data: Some(data),
                    ..Default::default()
                },
            );
            let effects = (trigger.run)(&mut ctx, &firing.event);
            crate::prompts::apply_resumable(&mut ctx, &plan, effects, None);
        }
        if sink.state.result.is_some() {
            return;
        }
        if sink.state.pending.is_some() && sink.state.pending != before {
            owe_firing(
                sink,
                TrapFiring {
                    at: at + 1,
                    ..firing.clone()
                },
            );
            return;
        }
    }
    end_firing(sink, &firing.trap_id);
}

/// The end of a firing: the trap consumed and the state check run (§3.2, §10.3).
///
/// R216: once the trap's effect has ended the game (#96's AI turn ran to an end-of-turn hit that
/// killed a hero) nothing happens after it — the trap is not consumed afterwards.
///
/// R52: everything the trap did belongs to the trap's controller, whoever caused the event. §3.2
/// sends a backrow card to its owner's graveyard when its effect ends, so the trap is consumed where
/// it is NOW — found again by id, because #96 My Pawn's AI turn replaces every instance in the state
/// (`ai_policy::adopt_state`). Only a trap still on the field is consumed: one its own effects moved off
/// it — exiled by the AI's #34 Collateral Damage, destroyed — is where that move put it, and exile is
/// a pile nothing takes a card back out of (§6.3). One whose turn-long effect has already ended was
/// consumed by `turn::cleanup` at the end of that turn (R152) and is in the graveyard by now.
fn end_firing(sink: &mut EngineSink<'_>, trap_id: &str) {
    if sink.state.result.is_some() {
        return;
    }
    if let Some(live) = find_instance(sink.state, trap_id).cloned()
        && is_on_field(&live)
    {
        consume_trap(sink, &live);
    }
    crate::state_check::state_check(sink);
}

fn owe_firing(sink: &mut EngineSink<'_>, firing: TrapFiring) {
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(
        "firing".to_string(),
        serde_json::to_value(&firing).expect("a trap firing is plain JSON (§10.1)"),
    );
    let resume = Resume {
        def_id: String::new(),
        hook: TRAP_FIRING_WORK.to_string(),
        step: "firing".to_string(),
        radiant: false,
        instance_id: None,
        data,
    };
    crate::work::owe(sink, resume);
}

/// `work.rs`'s arm for a firing a prompt interrupted (`"@trapFiring"`): the same trap, where it
/// stopped (R113). The payload came through JSON, so it is read back defensively.
pub fn run_owed_firing(sink: &mut EngineSink<'_>, item: &WorkItem) {
    let Some(raw) = item.resume.data.get("firing") else {
        return;
    };
    if !raw.is_object() {
        return;
    }
    if !raw.get("trapId").is_some_and(Value::is_string) || !raw.get("at").is_some_and(Value::is_number) {
        return;
    }
    if !raw.get("triggers").is_some_and(Value::is_array) || raw.get("event").is_none() {
        return;
    }
    if !matches!(raw.get("controller").and_then(Value::as_str), Some("p1" | "p2")) {
        return;
    }
    let Ok(firing) = serde_json::from_value::<TrapFiring>(raw.clone()) else {
        return;
    };
    run_armed_triggers(sink, firing);
}

/// §4.2 step 4, §6.3 "Cancel an attack": the window a declaration opens answers that declaration,
/// and only while it stands. Once a trap has cancelled it (R44), or the window has closed — #96's AI
/// turn can end the turn, and every declaration the AI makes opens and closes its own — there is no
/// attack left to answer: a cancelled attack resolves no combat, so it "would be lethal" to nobody,
/// and a second My Pawn stays armed and face-down (R99). The same holds once a trap earlier in the
/// window has destroyed, stolen or moved the attacker or its target (R220): that attack is over, and
/// there is nothing for My Pawn to call off. A forced attack opens no window at all (R121) and is not
/// held back here.
///
/// R220's "still stands as it was declared" is `combat.rs`'s (`declared_attack_stands`), which TS
/// registered here (`registerDeclarationCheck`) only to break an import cycle.
fn declaration_stands(state: &GameState, event: &GameEvent) -> bool {
    let GameEvent::AttackDeclared {
        attacker_id,
        target_id,
        forced,
    } = event
    else {
        return true;
    };
    if *forced {
        return true;
    }
    let Some(open) = &state.declared_attack else {
        return false;
    };
    !open.cancelled
        && open.attacker_id == *attacker_id
        && open.target_id == *target_id
        && crate::combat::declared_attack_stands(state, open)
}

/// The match as the board holds it now. A dispatch reads its matches when the window opens, and a
/// trap that fired before this one can have replaced every instance (#96's AI turn, `adopt_state`),
/// moved this trap off the field or turned it face-up, so each is found again by id before it is
/// offered the event: a trap no longer on the field, or already spent, never fires (R61, §5.1).
fn live_match(state: &GameState, trap_match: &TrapMatch, event: &GameEvent) -> Option<TrapMatch> {
    let trap = find_instance(state, &trap_match.trap.id)?.clone();
    if !acts_as_trap(state, &trap) || !is_trap_type(state, &trap) {
        return None;
    }
    if is_spent(state, &trap) || is_own_arrival(&trap, event) {
        return None;
    }
    let wakes = event.event_type();
    let triggers: Vec<TrapTrigger> = trap_triggers_of(state, &trap)
        .into_iter()
        .filter(|trigger| trigger.on.contains(&wakes))
        .collect();
    if triggers.is_empty() {
        None
    } else {
        Some(TrapMatch { trap, triggers })
    }
}

/// R174, R61: the event as a trap later in the same dispatch meets it. `cardResolved`'s `permanent`
/// is step 7's answer to "is the played card still in play", read as the card landed — but the traps
/// that answer the play fire one after another, and an earlier one can take the card off the field
/// before a later one reads the flag: #60 Bear Honeypot's tokens kill it, and it is in its graveyard
/// or back through Reborn by the time the next trap fires. That stay has ended, so the next trap
/// meets a play that is no longer in play: #85 fuses nothing out of a graveyard, and a second #60's
/// tokens do not attack a Reborn body, which is a new arrival (R83). `dispatch_mark` is the field's
/// departures when the dispatch began (`stays::exit_mark`), which a dispatch a prompt split carries to
/// the traps it still owes (`triggers::run_owed_traps`), so the answer that killed the card counts too —
/// and a play's event carries the mark it happened at (`stays::event_mark`), so what took the card off
/// the field between the event and its dispatch counts as well (R212).
pub fn standing_event(sink: &EngineSink<'_>, event: &GameEvent, dispatch_mark: u32) -> Option<GameEvent> {
    let state: &GameState = &*sink.state;
    // B5 E1, R448: an announce is answered only while its play can still be countered. Once a Counter
    // earlier in the window has cancelled it (or step 4 has moved the card), a later trap finds no card
    // and is not offered it at all, so it stays set (R99) rather than firing at nothing (R61).
    if let GameEvent::CardAnnounced { instance_id, .. } = event {
        return if crate::announce::is_announce_live(state, instance_id) {
            Some(event.clone())
        } else {
            None
        };
    }
    // R212: the stay is the one the event happened on. A play's events carry the mark they were
    // emitted at (`stays::event_mark`), which a late dispatch — a cast's `cardResolved` after the rest of
    // the list that cast it, its sweep and the Reborn that put a body back — must not move forward.
    let mark = dispatch_mark.min(crate::stays::event_mark(event).unwrap_or(dispatch_mark));
    match event {
        GameEvent::CardPlayed { instance_id, .. } | GameEvent::Summoned { instance_id, .. } => {
            // The same for step 4's pair (#41 Sheepish's moment, R17): a trap answering the arrival of a card
            // an earlier trap answering it has already taken off the field meets no card in play, and is not
            // offered the event at all, as #85 is not offered a play that is no longer a permanent in play
            // (R61). Sheepish does not reach into a hand to rewrite the unit the first trap bounced there, or
            // turn the Reborn body of the one it killed into a Sheep: that body is nobody's play (R83).
            if crate::stays::left_field_after(state, mark, instance_id) {
                None
            } else {
                Some(event.clone())
            }
        }
        GameEvent::CardResolved {
            instance_id,
            permanent: true,
            ..
        } => {
            let stays = find_instance(state, instance_id).is_some_and(is_on_field)
                && !crate::stays::left_field_after(state, mark, instance_id);
            if stays {
                return Some(event.clone());
            }
            let mut gone = event.clone();
            if let GameEvent::CardResolved { permanent, .. } = &mut gone {
                *permanent = false;
            }
            Some(gone)
        }
        _ => Some(event.clone()),
    }
}

fn dispatch(
    sink: &mut EngineSink<'_>,
    event: &GameEvent,
    matches: &[TrapMatch],
    mark: u32,
    controllers: &TrapControllers,
) -> TrapRun {
    let mut fired: Vec<String> = Vec::new();
    for (index, trap_match) in matches.iter().enumerate() {
        // §9.3: a prompt is state, so the rest of the dispatch waits for the answer action — and the
        // traps from here on have not seen the event, which is precisely what is owed (R113).
        if crate::work::paused(sink) {
            return TrapRun {
                fired,
                paused: true,
                owed: matches[index..].iter().map(|rest| rest.trap.id.clone()).collect(),
            };
        }
        // A trap the previous one destroyed, bounced or fused away never fires (R61), and nothing is
        // offered an attack a trap before it has already called off (§6.3).
        if !declaration_stands(sink.state, event) {
            continue;
        }
        let Some(live) = live_match(sink.state, trap_match, event) else {
            continue;
        };
        // R174, R212: a trap that has left the field since the dispatch began and stands there again is
        // a new arrival, on a stay that did not see the event.
        if crate::stays::left_field_after(sink.state, mark, &live.trap.id) {
            continue;
        }
        let Some(met) = standing_event(sink, event, mark) else {
            continue;
        };
        let controller = controllers.get(&live.trap.id).copied().unwrap_or(live.trap.controller);
        if fire_trap(sink, &live, &met, Some(controller)) {
            fired.push(live.trap.id.clone());
        }
    }
    TrapRun {
        fired,
        paused: crate::work::paused(sink),
        owed: Vec::new(),
    }
}

/// R212: who each trap answers for, read as a dispatch begins — before any of them has fired.
fn controllers_of(matches: &[TrapMatch], later: Option<&LaterMoves>) -> TrapControllers {
    let mut out: TrapControllers = IndexMap::new();
    for trap_match in matches {
        let before = later.and_then(|moves| moves.controller_before.get(&trap_match.trap.id).copied());
        out.insert(trap_match.trap.id.clone(), before.unwrap_or(trap_match.trap.controller));
    }
    out
}

/// §10.3: offer one event to the traps, as the resolution loop dispatches it — some time after it
/// happened, since the loop dispatches an event after whatever ran first. R212 has the traps answer it
/// as the board stood when it happened, which the events owed behind it say (`later`, read only when
/// some trap watches the event): a trap that reached the field since — summoned by the rest of the
/// list whose draw cast the card — is on a stay that did not see it and is not offered it, and one
/// whose controller has changed since answers for the player who held it then. What a prompt stops
/// the dispatch from offering is returned owed, in the order the dispatch had (R68, R113).
///
/// `later` is handed the sink (TS's closure captured it), so it reads the events owed behind this one
/// without holding a borrow across the dispatch.
pub fn offer_event_to_traps(
    sink: &mut EngineSink<'_>,
    event: &GameEvent,
    later: &dyn Fn(&EngineSink<'_>) -> LaterMoves,
) -> ImmediateDispatch {
    let mark = crate::stays::exit_mark(sink.state);
    if is_trap_window_event(event) {
        return ImmediateDispatch {
            mark,
            ..ImmediateDispatch::default()
        };
    }
    let watching = traps_watching(sink.state, event);
    if watching.is_empty() {
        return ImmediateDispatch {
            paused: crate::work::paused(sink),
            mark,
            ..ImmediateDispatch::default()
        };
    }
    let moves = later(sink);
    let matches: Vec<TrapMatch> = watching
        .into_iter()
        .filter(|trap_match| !moves.moved.contains(&trap_match.trap.id))
        .collect();
    let controllers = controllers_of(&matches, Some(&moves));
    if crate::work::paused(sink) {
        return ImmediateDispatch {
            fired: Vec::new(),
            paused: true,
            owed: matches.iter().map(|trap_match| trap_match.trap.id.clone()).collect(),
            controllers,
            mark,
        };
    }
    let run = dispatch(sink, event, &matches, mark, &controllers);
    ImmediateDispatch {
        fired: run.fired,
        paused: run.paused,
        owed: run.owed,
        controllers,
        mark,
    }
}

/// Finish an immediate dispatch a prompt interrupted: the traps it still owed, in the order it had
/// (R113 — a fresh scan would put a trap the answer moved onto the active side ahead of one the
/// dispatch had ordered before it), each for the player it answered for (R212), and each meeting the
/// event as the board now stands (`standing_event`, R174). A trap that met the event before the pause
/// and declined it is not in the list, and is not offered it again (R99).
pub fn resume_event_to_traps(
    sink: &mut EngineSink<'_>,
    event: &GameEvent,
    owed: &[String],
    mark: u32,
    controllers: &TrapControllers,
) -> ImmediateDispatch {
    let matches: Vec<TrapMatch> = owed
        .iter()
        .filter_map(|id| {
            let trap = find_instance(sink.state, id)?.clone();
            live_match(
                sink.state,
                &TrapMatch {
                    trap,
                    triggers: Vec::new(),
                },
                event,
            )
        })
        .collect();
    let run = dispatch(sink, event, &matches, mark, controllers);
    ImmediateDispatch {
        fired: run.fired,
        paused: run.paused,
        owed: run.owed,
        controllers: controllers.clone(),
        mark,
    }
}

/// §10.3: offer one freshly emitted event to the traps, before any ordinary trigger is queued. The
/// whole trap resolves inside this call — forced attacks included (R53) — so the action that emitted
/// the event continues only afterwards. A prompt for the trap's owner stops the dispatch instead:
/// the answer action resumes it, which is what "pauses the opponent's action" means (BUILD M3-T2).
///
/// R62's window events are withheld: `run_trap_window` fires those at their scheduled point.
pub fn fire_traps_for(sink: &mut EngineSink<'_>, event: &GameEvent) -> TrapDispatch {
    if is_trap_window_event(event) {
        return TrapDispatch {
            fired: Vec::new(),
            paused: false,
        };
    }
    if crate::work::paused(sink) {
        return TrapDispatch {
            fired: Vec::new(),
            paused: true,
        };
    }
    // The remainder of an immediate dispatch is `triggers.rs`'s to park: a trap is a response, so it
    // is owed in front of the trigger queue (`OWED_TO_TRAPS`), not behind the interrupted sequence.
    // Handed an event with nothing owed behind it, the board it meets is the board it happened on.
    let matches = traps_watching(sink.state, event);
    let mark = crate::stays::exit_mark(sink.state);
    let controllers = controllers_of(&matches, None);
    let run = dispatch(sink, event, &matches, mark, &controllers);
    TrapDispatch {
        fired: run.fired,
        paused: run.paused,
    }
}

// ---------------------------------------------------------------------------
// The end-of-turn window, and the remainder it owes (§2.2, R62, R100, R113)
// ---------------------------------------------------------------------------

/// The event a parked window captured, read back defensively: it came through JSON (§10.1).
fn window_event_of(data: &IndexMap<String, Value>) -> Option<GameEvent> {
    let captured = data.get("event")?;
    if !captured.is_object() {
        return None;
    }
    if !captured.get("type").is_some_and(Value::is_string) {
        return None;
    }
    serde_json::from_value::<GameEvent>(captured.clone()).ok()
}

fn owed_traps_of(data: &IndexMap<String, Value>) -> Vec<String> {
    match data.get("owed").and_then(Value::as_array) {
        Some(owed) => owed.iter().filter_map(|id| id.as_str().map(str::to_string)).collect(),
        None => Vec::new(),
    }
}

/// R212: a parked dispatch's `controllers`, read back defensively (it came through JSON).
pub fn trap_controllers_of(raw: Option<&Value>) -> TrapControllers {
    let mut out: TrapControllers = IndexMap::new();
    let Some(Value::Object(entries)) = raw else {
        return out;
    };
    for (id, player) in entries {
        match player.as_str() {
            Some("p1") => {
                out.insert(id.clone(), PlayerId::P1);
            }
            Some("p2") => {
                out.insert(id.clone(), PlayerId::P2);
            }
            _ => {}
        }
    }
    out
}

/// What a `TRAP_WINDOW_WORK` item owes, or `None` when it is not one: the reader for its payload.
pub fn owed_window_of(resume: &Resume) -> Option<OwedWindow> {
    if resume.hook != TRAP_WINDOW_WORK {
        return None;
    }
    let event = window_event_of(&resume.data)?;
    let mark = resume
        .data
        .get("mark")
        .and_then(Value::as_u64)
        .and_then(|mark| u32::try_from(mark).ok());
    Some(OwedWindow {
        event,
        owed: owed_traps_of(&resume.data),
        controllers: Some(trap_controllers_of(resume.data.get("controllers"))),
        mark,
    })
}

/// Park the rest of the window (R113). `work.rs` owns `state.work`, so this only ever calls
/// `owe_unnumbered`: the item lands at `state.work_cursor`, which puts it behind anything a trap's own
/// effects parked inside it (innermost first) and — when a resumption parks again — in front of
/// everything else still owed. Nothing is held but plain JSON, so the paused window survives a round trip.
///
/// R117: this is called at the moment the window pauses and never in advance. While `dispatch` is on
/// the stack the traps it has not reached are the dispatch's alone, so a `settle` running inside one
/// of them — a trap's own effects can start one — cannot take the traps the window is standing in.
fn owe_window(
    sink: &mut EngineSink<'_>,
    event: &GameEvent,
    owed: &[String],
    controllers: &TrapControllers,
    mark: u32,
) {
    if owed.is_empty() {
        return;
    }
    let mut kept: IndexMap<String, Value> = IndexMap::new();
    for id in owed {
        if let Some(player) = controllers.get(id) {
            kept.insert(id.clone(), Value::String(player.as_str().to_string()));
        }
    }
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert(
        "event".to_string(),
        serde_json::to_value(event).expect("an event is plain JSON (§10.1)"),
    );
    data.insert(
        "owed".to_string(),
        Value::Array(owed.iter().map(|id| Value::String(id.clone())).collect()),
    );
    data.insert("controllers".to_string(), Value::Object(kept.into_iter().collect()));
    data.insert("mark".to_string(), Value::from(mark));
    let resume = Resume {
        def_id: String::new(),
        hook: TRAP_WINDOW_WORK.to_string(),
        step: TRAP_WINDOW_STEP.to_string(),
        radiant: false,
        instance_id: None,
        data,
    };
    // R177: the remainder exists only when a trap still owed the event watches it, and whether a
    // face-down one does is its controller's to know (R33), so it takes no number (`owe_unnumbered`).
    crate::work::owe_unnumbered(sink, resume, None);
}

/// §2.2 and R62: the end-of-turn trap window. It runs after the end-of-turn triggers and before the
/// end-of-turn delayed effects, and it fires on both sides in R68 order — the ending player's traps
/// first — which is what makes Bread and Butter's token go to the trap's controller whichever player
/// ended the turn with unspent mana (R52).
///
/// A trap that prompts its controller stops the window where it stands and the rest of it is owed in
/// state, so the answer action delivers the event to the traps that have not seen it — a Field Trap
/// that already fired is not among them, since it fired rather than being passed over (R62, R100).
/// The game ending stops the window for good: there is nothing left to resume into.
pub fn run_trap_window(sink: &mut EngineSink<'_>, event: &GameEvent) -> TrapDispatch {
    if sink.state.result.is_some() {
        return TrapDispatch {
            fired: Vec::new(),
            paused: true,
        };
    }
    let matches = traps_watching(sink.state, event);
    // R212: the window's event has just happened, so each trap answers for its controller now — and
    // keeps answering for that player when an earlier trap of the window steals it before its turn.
    let controllers = controllers_of(&matches, None);
    let mark = crate::stays::exit_mark(sink.state);

    // A prompt already open when the window opens means the window has delivered nothing at all:
    // every matched trap is owed the event. Returning without parking would lose the whole window.
    if sink.state.pending.is_some() {
        let owed: Vec<String> = matches.iter().map(|trap_match| trap_match.trap.id.clone()).collect();
        owe_window(sink, event, &owed, &controllers, mark);
        return TrapDispatch {
            fired: Vec::new(),
            paused: true,
        };
    }

    let run = dispatch(sink, event, &matches, mark, &controllers);
    if sink.state.result.is_none() {
        owe_window(sink, event, &run.owed, &controllers, mark);
    }
    TrapDispatch {
        fired: run.fired,
        paused: run.paused,
    }
}

/// Finish a window a prompt interrupted: offer the event to the traps that were owed it, in the
/// order the window had, and park again if one of those prompts too. The board is re-read only to
/// drop a trap that has left the field since — destroyed, bounced or fused away — which never fires
/// (R61); the owed list is what keeps a trap that already fired from firing twice, which a Field
/// Trap would otherwise do, since firing leaves it on the field (§5.1, R33, R100).
fn resume_window(sink: &mut EngineSink<'_>, parked: OwedWindow) {
    let OwedWindow {
        event,
        owed,
        controllers,
        mark,
    } = parked;
    let watching = traps_watching(sink.state, &event);
    // The order is the owed list's, not a fresh scan's: R68's sides are read off `state.active`, and
    // the window's order was fixed when it opened (R62 — the ending player's traps, then the
    // opponent's). The board is re-read only to drop what has left the field since (R61).
    let matches: Vec<TrapMatch> = owed
        .iter()
        .filter_map(|id| {
            watching
                .iter()
                .find(|candidate| candidate.trap.id == *id)
                .cloned()
        })
        .collect();
    let controllers = controllers.unwrap_or_default();
    let mark = mark.unwrap_or_else(|| crate::stays::exit_mark(sink.state));
    let run = dispatch(sink, &event, &matches, mark, &controllers);
    if sink.state.result.is_none() {
        owe_window(sink, &event, &run.owed, &controllers, mark);
    }
}

/// `work.rs`'s arm for a parked window (`"@trapWindow"`): the same window, continued where it
/// stopped (R113).
pub fn run_owed_window(sink: &mut EngineSink<'_>, item: &WorkItem) {
    let Some(parked) = owed_window_of(&item.resume) else {
        return;
    };
    resume_window(sink, parked);
}
