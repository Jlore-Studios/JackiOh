//! Trap matching and immediate resolution (SPEC §5.1, §10.3, §2.2's end-of-turn window; BUILD M3-T2).
//! Traps are checked before other triggers because they are responses, and a trap resolves to completion,
//! its owner's prompts and forced attacks included, before the action it answered continues (§10.3); the
//! end-of-turn window is the one scheduled exception (R62).
//!
//! The module matches (`traps_watching`, R68 order), fires (`fire_traps_for`, `run_trap_window`), consumes
//! (`consume_trap`; a Field Trap stays, face-up, R33) and owes: an interrupted window parks the traps not
//! yet offered the event (`TRAP_WINDOW_WORK`), a trap whose list asks parks the rest of its firing
//! (`TRAP_FIRING_WORK`), and the immediate path owes its rest as `triggers.rs`'s `OWED_TO_TRAPS`, so no
//! event is dropped half-delivered (§9.3, R113). The *when* is the caller's: `triggers.rs` offers each
//! fresh event before any ordinary trigger, `turn.rs` runs the window (R100), and R17's two moments (R61's
//! `permanent`) are §10.5's. `work.rs` calls `run_owed_firing` and `run_owed_window` (SURFACE §6.6).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::config::PERCENT;
use crate::script::{EngineSink, TriggerDef};
use crate::state::{
    CardInstance, GameState, Resume, WorkItem, active_first, find_instance, find_instance_mut,
};
use crate::stays::LaterMoves;
use crate::wire::{GameEvent, GameEventType, PlayerId, Row, ZoneName};

/// A trap trigger. R99: a trap fires when its trigger's `on` matches the event **and** its `when` admits
/// it. R61 makes `run` returning `[]` mean "fired, consumed, did nothing", so a condition that must leave
/// the trap armed (Bear Honeypot's "costing 1 or less") belongs in `when`, never in `run`. A trigger with
/// no predicate answers every event it names, whichever side caused it.
///
/// The card files annotate their trap triggers with this alias (#18, #41, #60, #71, #85, #96).
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

/// A dispatch as the module reads it: `owed` names the traps the pause stopped it from offering the event
/// to, which a resume needs and no caller may guess by subtracting `fired` (a trap whose `when` declined
/// has seen the event and is not owed it again).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct TrapRun {
    fired: Vec<String>,
    paused: bool,
    owed: Vec<String>,
}

/// R212: the player each trap a dispatch offers the event to answers for: its controller when the event
/// happened, read as the dispatch begins and kept across any pause. A trap stolen in between (a cast's
/// events wait for its list, R70; the window offers `turnEnded` trap by trap, R62) answers, and makes its
/// tokens, for the player who held it then (R52).
pub type TrapControllers = IndexMap<String, PlayerId>;

/// An immediate dispatch, as `triggers.rs` parks and resumes it: the traps still owed the event in the
/// dispatch's order (R68, fixed as it began), who each answers for, and the stays it began with (R174).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ImmediateDispatch {
    pub fired: Vec<String>,
    pub paused: bool,
    pub owed: Vec<String>,
    pub controllers: TrapControllers,
    pub mark: u32,
}

/// R113: the `resume.hook` a paused end-of-turn trap window parks under. No `Script` can hold the name;
/// `run_owed_window` is `work.rs`'s arm for it (`work::run_work_item` panics on a hook nothing knows).
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

/// R100: the events the scheduled end-of-turn window owns, withheld from the immediate check, so Bread and
/// Butter and Intern Stimmy fire once per turn end, in the window, on both sides. This list is the whole
/// of that withholding, and R62's end-of-turn order depends on it.
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

/// Every trap on the field, in R68 order: the active player's first; on each side the unit lanes 1–5,
/// then the backrow lanes 1–5. An animated Field Trap (R383, B3.1 rule 3) stands in a unit zone as a Unit
/// and fires from there (Classic #5 Tesla keeps zapping). Only the card acting in a zone is one (§3.2): a
/// trap dormant under a backrow pile never fires (R447).
pub fn traps_in_order(state: &GameState) -> Vec<CardInstance> {
    let sides = active_first(state);
    let mut out: Vec<CardInstance> = Vec::new();
    for player in sides {
        for slot in crate::zones::slots_of(player, Row::Units) {
            if let Some(card) = crate::zones::card_at(state, slot)
                && crate::animated::is_animated(state, card)
                && is_trap_type(state, card)
            {
                out.push(card.clone());
            }
        }
        for slot in crate::zones::slots_of(player, Row::Backrow) {
            if let Some(card) = crate::zones::card_at(state, slot)
                && is_trap_type(state, card)
            {
                out.push(card.clone());
            }
        }
    }
    out
}

fn trap_triggers_of(state: &GameState, trap: &CardInstance) -> Vec<TrapTrigger> {
    // R783006, R1040, R1044: a set card reveals by its Reveal alone, not on its printed triggers.
    if trap.set_as.is_some() {
        return crate::alt_play::reveal_triggers(trap);
    }
    crate::scripts::script_of(state, trap).triggers.clone()
}

/// R119: the events that say a card arrived. A card does not answer the play that put it onto the field,
/// though §10.5 step 4 places it before emitting `cardPlayed`: a trap is never offered the `cardPlayed`,
/// `summoned` or `cardResolved` that names itself. R17's two moments are both here, and a Trap is itself a
/// card someone plays, so without this a trap watching either moment would answer its own arrival.
const ARRIVAL_EVENTS: &[GameEventType] = &[
    GameEventType::CardPlayed,
    GameEventType::Summoned,
    GameEventType::CardResolved,
];

/// The arrival events' `instanceId`.
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
    // R119: nor the play the trap arrived on the field during, whatever put it there (#95 summoning a
    // Bear Honeypot face-down; a Sheepish a tributed Cube's Death copied at step 2).
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

/// The traps this event woke, matched by the event type each trigger names; a `when` predicate needs an
/// `EffectContext`, so it is evaluated when the trap fires. `cardResolved` (R17) is matched too, and only
/// on the immediate path: R100 keeps the two disjoint.
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

/// §5.1: a Trap fires, then goes to the graveyard; only a Field Trap fires again. A Trap is face-down
/// until it fires (R33), so a face-up one still in the backrow is mid-firing (#96 My Pawn's effects are a
/// whole AI turn) and answers nothing more; `fire_trap` turns it face-up before its effects for this.
pub fn is_spent(state: &GameState, trap: &CardInstance) -> bool {
    trap.face_up == Some(true) && !is_field_trap(state, trap)
}

/// §3.2: backrow cards go to the owner's graveyard when their effect ends, a Field Trap excepted, which is
/// face-up to both players from then on (R33). R61: a trap that fired is consumed whatever its effects did.
pub fn consume_trap(sink: &mut EngineSink<'_>, instance: &CardInstance) {
    let mut card = crate::zones::live_or(sink.state, instance);
    card.face_up = Some(true);
    if let Some(live) = find_instance_mut(sink.state, &instance.id) {
        live.face_up = Some(true);
    }
    // R783006: a Field Spell set by its Cloaked has revealed where it stands, and is its own card again.
    if card.set_as.is_some()
        && crate::faces::card_type_of_face(sink.state, &card.def_id, card.radiant)
            == crate::wire::CardType::FieldSpell
    {
        if let Some(live) = find_instance_mut(sink.state, &instance.id) {
            live.set_as = None;
        }
        return;
    }
    if is_field_trap(sink.state, &card) {
        return;
    }
    // ME-ALTPLAY, R1041: a revealed set Unit is a Unit now and stays on the field.
    if crate::animated::face_type_of(sink.state, &card) == crate::wire::CardType::Unit {
        return;
    }
    // R383 (B3.1 rules 2 and 4): an Animated Trap's firing ends by animating it, or leaves it face-up in
    // its zone when no unit zone is open; either way it stays, spent (`is_spent`).
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

/// R152, §3.2, §5.1: #96 My Pawn's effect is the rest of the turn it handed to the AI, so the Trap goes to
/// the graveyard at that turn's cleanup (`turn.rs` calls this), not when `fire_trap` returns from under the
/// AI's playout, inside the next turn (#37 Gravedigger must find it there). A trap its AI turn moved off
/// the field is not touched.
pub fn end_handed_over_turn(sink: &mut EngineSink<'_>) {
    for trap in traps_in_order(sink.state) {
        if is_spent(sink.state, &trap) {
            consume_trap(sink, &trap);
        }
    }
}

/// Fire one trap: emit `trapFired`, run every trigger this event woke, run the state check, then consume
/// it whatever its effects achieved (an Immutable Sheepish target or a Fuse with no legal target still
/// spends it, R17, R61). False when no trigger's `when` admitted the event, leaving the trap armed and
/// face-down (R99). `controller` defaults to the trap's own.
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
    // R1849009: a trap with a chance to reveal rolls it now, once for the firing its triggers admitted;
    // a miss leaves it set and emits nothing, as a `when` that declined does (R99).
    if !chance_admits(sink, trap, controller) {
        return false;
    }

    // R154: the zone is read before the trap resolves, because firing it can move the card — a Trap
    // reaches its owner's graveyard on consumption and would then have no slot to report.
    let at = crate::zones::slot_of(sink.state, trap);
    // `controller` is the seat the trap sits on (R154); its identity follows R97 on both seats
    // (R2734000). A trap stolen since the event is face-down on its new controller's side: the flip is
    // announced on the seat it sits on.
    // R2657001: a Field Trap already face-up reveals nothing as it fires again (`again`).
    sink.events.push(GameEvent::TrapFired {
        instance_id: trap.id.clone(),
        def_id: trap.def_id.clone(),
        controller: trap.controller,
        row: at.map_or(Row::Backrow, |slot| slot.row),
        lane: at.map_or(0, |slot| slot.lane),
        again: fires_again(sink.state, trap),
    });

    // §5.1: it has fired, and it is face-up from this moment (R33) — which also keeps it out of every
    // dispatch its own effects start, since a Trap fires once (`is_spent`).
    if let Some(live) = find_instance_mut(sink.state, &trap.id) {
        live.face_up = Some(true);
    }
    // R783011: a Crater Trap Locks the backrow zone it is revealed in, before its effects.
    crate::crater::lock_on_firing(sink, trap, at);
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

/// R2657001: `Some(true)` when the trap firing is a Field Trap that has fired before and stands face-up,
/// so it reveals nothing this time; `None` for a reveal. What `trapFired.again` carries.
pub fn fires_again(state: &GameState, trap: &CardInstance) -> Option<bool> {
    let live = find_instance(state, &trap.id).unwrap_or(trap);
    (live.face_up == Some(true) && is_field_trap(state, live)).then_some(true)
}

/// R1849009 (Stinkyscape #174 Aerelret, #22 Gatinsir): "30% chance to Reveal". A face-down trap whose face
/// prints a chance fires only when the match rng rolls under it (its declared `chance`, out of `PERCENT`),
/// the controller's Luck and the trap's Lucky rolling again, any hit enough (R987, R1440). Any other firing
/// (a face-up Field Trap, a set card revealing, R783006) draws nothing, so it never moves the rng.
fn chance_admits(sink: &mut EngineSink<'_>, trap: &CardInstance, controller: PlayerId) -> bool {
    if trap.face_up == Some(true) || trap.set_as.is_some() {
        return true;
    }
    let Some(printed) = crate::scripts::flags_of(sink.state, trap).reveal_chance else {
        return true;
    };
    let percent = crate::params::declared_or(sink.state, trap, "chance", printed).clamp(0, PERCENT);
    let chance = f64::from(percent) / f64::from(PERCENT);
    let lucky =
        (crate::query::lucky_on(sink.state, trap) + crate::query::luck_of(sink.state, controller)).max(0);
    if lucky == 0 {
        sink.rng.chance(chance)
    } else {
        sink.rng.lucky(lucky, |rng| rng.chance(chance), |a, b| a || b)
    }
}

/// What is left of one trap's firing: its armed triggers from `at` on, then its end (consumed, and the
/// state check, §10.3). All JSON, so it survives a prompt on `state.work` as `TRAP_FIRING_WORK` (R113).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct TrapFiring {
    trap_id: String,
    controller: PlayerId,
    triggers: Vec<String>,
    at: usize,
    event: GameEvent,
}

/// R113: the `resume.hook` of a trap's firing a prompt interrupted: the triggers it has not run, and its
/// end. §10.3 has a trap resolve to completion, its owner's prompts included, so its list runs resumably:
/// `prompts::apply_resumable` parks the effects after one that asks (re-entered by the trigger's id,
/// `work::script_step_for`), and this item behind them owes the rest.
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

/// The end of a firing: the trap consumed and the state check run (§3.2, §10.3). R216: once the trap's
/// effect has ended the game (#96's AI turn killed a hero) nothing follows, the consumption included.
///
/// The trap is found again by id, since #96 My Pawn's AI turn replaces every instance
/// (`ai_policy::adopt_state`), and consumed only while on the field: one its own effects moved (exiled by
/// #34 Collateral Damage, §6.3) stays put, and one whose turn-long effect ended went at cleanup (R152).
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

/// §4.2 step 4, §6.3 "Cancel an attack": a declaration's window answers it only while it stands. Once a
/// trap has canceled it (R44), the window has closed (#96's AI turn), or an earlier trap has destroyed,
/// stolen or moved the attacker or its target (R220, `combat::declared_attack_stands`), there is no attack
/// to answer, and a second My Pawn stays armed (R99). A forced attack opens no window (R121).
fn declaration_stands(state: &GameState, event: &GameEvent) -> bool {
    let GameEvent::AttackDeclared {
        attacker_id,
        target_id,
        forced,
        ..
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

/// The match as the board holds it now: an earlier trap can have replaced every instance (#96's AI turn,
/// `adopt_state`), moved this one off the field or turned it face-up, so it is found again by id, and one
/// off the field or spent never fires (R61, §5.1).
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

/// R174, R61: the event as a trap later in the same dispatch meets it. `cardResolved`'s `permanent` is
/// read as the card landed, but an earlier trap can take the card off the field first (#60 Bear Honeypot's
/// tokens kill it), so the next trap meets a play no longer in play: #85 fuses nothing out of a graveyard,
/// and a second #60's tokens do not attack a Reborn body (R83). `dispatch_mark` is the field's departures
/// as the dispatch began (`stays::exit_mark`), carried across a prompt (`triggers::run_owed_traps`); a
/// play's event carries its own mark too (`stays::event_mark`, R212).
pub fn standing_event(sink: &EngineSink<'_>, event: &GameEvent, dispatch_mark: u32) -> Option<GameEvent> {
    let state: &GameState = &*sink.state;
    // B5 E1, R448: an announce is answered only while its play can still be countered; once a Counter
    // canceled it, a later trap is not offered it and stays set (R99) rather than firing at nothing (R61).
    if let GameEvent::CardAnnounced { instance_id, .. } = event {
        return if crate::announce::is_announce_live(state, instance_id) {
            Some(event.clone())
        } else {
            None
        };
    }
    // R212: the stay is the one the event happened on; a late dispatch (a cast's `cardResolved` after the
    // list that cast it) must not move the event's own mark forward.
    let mark = dispatch_mark.min(crate::stays::event_mark(event).unwrap_or(dispatch_mark));
    match event {
        GameEvent::CardPlayed { instance_id, .. } | GameEvent::Summoned { instance_id, .. } => {
            // The same for step 4's pair (#41 Sheepish's moment, R17): a card an earlier trap took off the
            // field meets no later trap (R61); Sheepish rewrites neither a bounced unit nor a Reborn body,
            // which is nobody's play (R83).
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
        // §9.3: a prompt is state; the traps from here on have not seen the event, and are owed it (R113).
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
        let controller = controllers
            .get(&live.trap.id)
            .copied()
            .unwrap_or(live.trap.controller);
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
        out.insert(
            trap_match.trap.id.clone(),
            before.unwrap_or(trap_match.trap.controller),
        );
    }
    out
}

/// §10.3: offer one event to the traps as the resolution loop dispatches it, after it happened. R212
/// has them answer as the board stood then, which the events owed behind it say (`later`, read only when
/// a trap watches): a trap that reached the field since is not offered it, and one whose controller has
/// changed answers for the player who held it then. What a prompt stops is returned owed, in order (R68,
/// R113). `later` takes the sink so it holds no borrow across the dispatch.
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
            owed: matches
                .iter()
                .map(|trap_match| trap_match.trap.id.clone())
                .collect(),
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

/// Finish an immediate dispatch a prompt interrupted: the traps it still owed, in its order (R113; a fresh
/// scan would reorder a trap the answer moved sides), each for its player (R212), each meeting the event as
/// the board now stands (`standing_event`, R174). One that declined before the pause is not offered it
/// again (R99).
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

/// §10.3: offer a freshly emitted event to the traps before any ordinary trigger is queued. The whole trap
/// resolves inside this call, forced attacks included (R53); a prompt for its owner stops the dispatch for
/// the answer to resume (BUILD M3-T2). R62's window events are withheld for `run_trap_window`.
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
    // `triggers.rs` parks the rest of an immediate dispatch in front of the trigger queue (`OWED_TO_TRAPS`):
    // a trap is a response. With nothing owed behind the event, the board it meets is the one it happened on.
    let matches = traps_watching(sink.state, event);
    let mark = crate::stays::exit_mark(sink.state);
    let controllers = controllers_of(&matches, None);
    let run = dispatch(sink, event, &matches, mark, &controllers);
    TrapDispatch {
        fired: run.fired,
        paused: run.paused,
    }
}

// The end-of-turn window, and the remainder it owes (§2.2, R62, R100, R113)

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
        Some(owed) => owed
            .iter()
            .filter_map(|id| id.as_str().map(str::to_string))
            .collect(),
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

/// Park the rest of the window (R113). The item lands at `state.work_cursor`: behind anything a trap's own
/// effects parked inside it, and in front of everything else still owed when a resumption parks again.
///
/// R117: called at the moment the window pauses, never in advance, so a `settle` inside a trap's effects
/// cannot take the traps the window is standing in.
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
    data.insert(
        "controllers".to_string(),
        Value::Object(kept.into_iter().collect()),
    );
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

/// §2.2 and R62: the end-of-turn trap window, after the end-of-turn triggers and before the delayed
/// effects, on both sides in R68 order (the ending player's first); Bread and Butter's token is the trap's
/// controller's whoever ended the turn with unspent mana (R52). A trap's prompt stops the window and the
/// traps that have not seen the event are owed in state (not a Field Trap that fired, R100).
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
        let owed: Vec<String> = matches
            .iter()
            .map(|trap_match| trap_match.trap.id.clone())
            .collect();
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

/// Finish a window a prompt interrupted: offer the event to the traps owed it, in the window's order, and
/// park again if one prompts. The owed list keeps a Field Trap that already fired, still on the field, from
/// firing twice (§5.1, R33, R100); the board is re-read only to drop a trap that left the field (R61).
fn resume_window(sink: &mut EngineSink<'_>, parked: OwedWindow) {
    let OwedWindow {
        event,
        owed,
        controllers,
        mark,
    } = parked;
    let watching = traps_watching(sink.state, &event);
    // The owed list's order, not a fresh scan's: R68's sides are read off `state.active`, and the
    // window's order was fixed when it opened (R62).
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
