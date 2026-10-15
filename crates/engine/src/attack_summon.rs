//! ME-ATTACKSUMMON (Meditative #91 Windfast, #91.1 Windfurious Prime): two hooks on the
//! attack pipeline (§4.2).
//!
//! A replacement (R1202; MD-E14): on every attack Windfast makes, declared or forced, a random Unit
//! of its controller's hand (the match rng's; patch v0.3.9, R831006: their pick before) is summoned
//! into the leftmost open unit zone (R64, no Cry, R1) and makes that attack on the same target as a
//! forced attack (R53), with its own trap window and combat; Windfast neither strikes nor is struck. The
//! base face bounces the substitute after its combat if it is still on the field. With no Unit in
//! hand or no open zone Windfast attacks itself.
//!
//! A rider (R1203; MD-E15): on every attack the Prime makes, declared or forced, `joiners` Units
//! drawn uniformly from its controller's hand and deck together are summoned (no Cry) and attack
//! its target first as forced attacks, in lane order, before the Prime's own combat; joiners stay.
//! A declared attack whose target is gone is cancelled, the exertion spent.
//!
//! Everything parked is plain JSON (R113): the owed combat or bounce runs here. Every draw is the
//! match rng's.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::combat::AttackTarget;
use crate::script::EngineSink;
use crate::state::{CardInstance, DeclaredAttack, GameState, Resume, find_instance};
use crate::wire::{CardType, PlayerId, Row};

/// R113: the `resume.hook` of the engine sequence this module parks — the rest of a Prime's combat or
/// a substitute's bounce a question stopped.
pub const ATTACK_SUMMON_HOOK: &str = "@attackSummon";

/// Where the owed record sits inside the prompt's and the work item's `resume.data`.
const OWED_KEY: &str = "attackSummon";

/// The bounce a question stopped (`bounce_after`).
const BOUNCE_STEP: &str = "bounce";

/// The combat the joiners stopped (`prime_combat`).
const COMBAT_STEP: &str = "combat";

/// What a paused replacement or rider still owes, as plain JSON (R113): whose attack it was, its
/// target, whether it was forced, whether the substitute bounces afterwards, the declaration a
/// declared attack stands on, whose attack a resumed combat makes instead, and the stays it began
/// on.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OwedAttackSummon {
    pub attacker_id: String,
    pub target_id: String,
    pub forced: bool,
    pub bounce: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared: Option<DeclaredAttack>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instead_of: Option<String>,
    pub since: u32,
}

/// Whether `replace_attacker` replaced the attack: no substitute, or the summoned substitute making it.
pub enum Replaced {
    No,
    By(Box<CardInstance>),
}

/// Whether `joiners_first` fought at once or parked the Prime's combat behind the joiners' run.
pub enum JoinOutcome {
    Fight,
    Paused,
}

/// Whether a card in a hand or library pile is a Unit — tokens included, unlike `recruit`.
fn is_unit(state: &GameState, card: &CardInstance) -> bool {
    crate::faces::card_type_of(state, card) == CardType::Unit
}

/// The Unit cards in a player's hand, in hand order.
fn hand_units(state: &GameState, controller: PlayerId) -> Vec<CardInstance> {
    state.players[controller]
        .hand
        .iter()
        .filter(|card| is_unit(state, card))
        .cloned()
        .collect()
}

/// Summon a Unit from its controller's hand for the attack — into the leftmost open unit zone
/// (R64), no Cry (R1) — or `None` when the pick is no longer there to summon or no zone is open.
/// A summon, so traps that answer one fire on the events it emits.
fn summon_hand_unit(
    sink: &mut EngineSink<'_>,
    declarer: &CardInstance,
    pick: &CardInstance,
) -> Option<CardInstance> {
    let controller = declarer.controller;
    if !sink.state.players[controller]
        .hand
        .iter()
        .any(|card| card.id == pick.id)
    {
        return None;
    }
    let mut ctx = crate::resolve::make_context(
        sink,
        Some(declarer),
        crate::resolve::HookOptions {
            controller: Some(controller),
            ..crate::resolve::HookOptions::default()
        },
    );
    crate::effects::summon::summon_existing_card(&mut ctx, pick, controller)
}

/// The owed record a prompt or work item carries, re-read defensively: it came back through JSON
/// (§10.1), so nothing about the payload is assumed.
fn owed_of(data: &IndexMap<String, Value>) -> Option<OwedAttackSummon> {
    let raw = data.get(OWED_KEY)?;
    serde_json::from_value::<OwedAttackSummon>(raw.clone()).ok()
}

fn owe_record(sink: &mut EngineSink<'_>, step: &str, owed: &OwedAttackSummon) {
    let mut data = IndexMap::new();
    data.insert(
        OWED_KEY.to_string(),
        serde_json::to_value(owed).expect("an owed attack summon is plain JSON (§10.1)"),
    );
    // R113: owed as an engine sequence at the moment it pauses, never in advance (R117).
    crate::work::owe(
        sink,
        Resume {
            def_id: String::new(),
            hook: ATTACK_SUMMON_HOOK.to_string(),
            step: step.to_string(),
            radiant: false,
            instance_id: None,
            data,
        },
    );
}

// ---------------------------------------------------------------------------
// The replacement (R1202)
// ---------------------------------------------------------------------------

/// R1202, R831006: what attacks when `attacker` would — a random Unit of its controller's hand
/// instead, or itself. `No` unless the attacker carries `attack_from_hand`, its controller holds a
/// Unit, and a unit zone is open. The pick is the match rng's; one Unit draws no random number
/// (R129).
pub fn replace_attacker(sink: &mut EngineSink<'_>, attacker: &CardInstance) -> Replaced {
    if crate::scripts::flags_of(sink.state, attacker).attack_from_hand != Some(true) {
        return Replaced::No;
    }
    let controller = attacker.controller;
    let units = hand_units(sink.state, controller);
    if units.is_empty() {
        return Replaced::No;
    }
    if crate::zones::first_entry_zone(sink.state, controller, Row::Units).is_none() {
        return Replaced::No;
    }
    let Some(unit) = sink
        .rng
        .shuffle(&units)
        .first()
        .and_then(|pick| summon_hand_unit(sink, attacker, pick))
    else {
        return Replaced::No;
    };
    Replaced::By(Box::new(unit))
}

// ---------------------------------------------------------------------------
// The rider (R1203)
// ---------------------------------------------------------------------------

/// R1203: the joiners' attack first. `Fight` at once unless the attacker carries `attack_joiners`.
/// The pool is the Unit cards in the controller's hand, then library — unit tokens included,
/// unlike `recruit`. An empty pool or no open zone fights at once and draws nothing. Otherwise
/// `joiners` cards from the shuffled pool are summoned, each while a zone is open, and attack the
/// Prime's target first as forced attacks in lane order; then the Prime's own combat. `Paused`
/// when a joiner's Death asked something mid-run.
pub fn joiners_first(
    sink: &mut EngineSink<'_>,
    prime: &CardInstance,
    target: &AttackTarget,
    since: u32,
) -> JoinOutcome {
    if crate::scripts::flags_of(sink.state, prime).attack_joiners != Some(true) {
        return JoinOutcome::Fight;
    }
    let controller = prime.controller;
    let mut pool: Vec<CardInstance> = hand_units(sink.state, controller);
    pool.extend(
        sink.state.players[controller]
            .library
            .iter()
            .filter(|card| is_unit(sink.state, card))
            .cloned(),
    );
    if pool.is_empty() {
        return JoinOutcome::Fight;
    }
    if crate::zones::first_entry_zone(sink.state, controller, Row::Units).is_none() {
        return JoinOutcome::Fight;
    }
    let count = crate::params::param_value(
        sink.state,
        Some(prime),
        "joiners",
        crate::params::ParamValueOptions {
            def_id: None,
            radiant: None,
        },
    )
    .max(0) as usize;
    let mut joiners: Vec<CardInstance> = Vec::new();
    for pick in sink.rng.shuffle(&pool).into_iter().take(count) {
        let mut ctx = crate::resolve::make_context(
            sink,
            Some(prime),
            crate::resolve::HookOptions {
                controller: Some(controller),
                ..crate::resolve::HookOptions::default()
            },
        );
        if let Some(unit) = crate::effects::summon::summon_existing_card(&mut ctx, &pick, controller) {
            joiners.push(unit);
        }
    }
    if joiners.is_empty() {
        return JoinOutcome::Fight;
    }
    joiners.sort_by_key(|unit| {
        crate::zones::slot_of(sink.state, unit)
            .map(|slot| slot.lane)
            .unwrap_or(i32::MAX)
    });
    crate::combat::force_attacks_on(sink, &joiners, target, Some(since));
    if crate::work::paused(sink) {
        JoinOutcome::Paused
    } else {
        JoinOutcome::Fight
    }
}

// ---------------------------------------------------------------------------
// Afterwards (R1202)
// ---------------------------------------------------------------------------

/// R1202: bounce the substitute after its combat — even when its attack was cancelled — if it is
/// still on the field on the stay it attacked from. The game over ends everything; a prompt open
/// owes the bounce behind it.
pub fn bounce_after(sink: &mut EngineSink<'_>, id: &str, since: u32) {
    if sink.state.result.is_some() {
        return;
    }
    if sink.state.pending.is_some() {
        let Some(card) = find_instance(sink.state, id).cloned() else {
            return;
        };
        owe_record(
            sink,
            BOUNCE_STEP,
            &OwedAttackSummon {
                attacker_id: card.id.clone(),
                target_id: String::new(),
                forced: false,
                bounce: true,
                declared: None,
                instead_of: None,
                since,
            },
        );
        return;
    }
    let Some(card) = find_instance(sink.state, id).cloned() else {
        return;
    };
    if !crate::zones::acts_on_field(sink.state, &card) {
        return;
    }
    if crate::stays::left_field_after(sink.state, since, id) {
        return;
    }
    crate::effects::move_::bounce_card(sink, &card);
}

/// Owe the Prime's combat once the joiners' run has parked the rest of its run behind it (R113's
/// order: the run's own remainder first, then this). `work.rs` runs it.
pub fn owe_combat(sink: &mut EngineSink<'_>, owed: &OwedAttackSummon) {
    owe_record(sink, COMBAT_STEP, owed);
}

/// R113: the `"@attackSummon"` work item — the bounce or the combat a question stopped.
pub fn run_owed(sink: &mut EngineSink<'_>, item: &crate::state::WorkItem) {
    let Some(owed) = owed_of(&item.resume.data) else {
        return;
    };
    if item.resume.step == BOUNCE_STEP {
        bounce_after(sink, &owed.attacker_id, owed.since);
        return;
    }
    if item.resume.step == COMBAT_STEP {
        crate::combat::prime_combat(sink, &owed);
    }
}
