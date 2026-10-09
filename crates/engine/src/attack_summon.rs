//! ME-ATTACKSUMMON (Meditative #91 Windfast, #91.1 Windfurious Prime): two hooks on the
//! attack pipeline (§4.2).
//!
//! A replacement (R1202; MD-E14): on every attack Windfast makes, declared or forced, its
//! controller's Unit from their hand — their pick, no prompt with one — is summoned into the
//! leftmost open unit zone (R64, no Cry, R1) and makes that attack on the same target as a forced
//! attack (R53), with its own trap window and combat; Windfast neither strikes nor is struck. The
//! base face bounces the substitute after its combat if it is still on the field. With no Unit in
//! hand or no open zone Windfast attacks itself.
//!
//! A rider (R1203; MD-E15): on every attack the Prime makes, declared or forced, `joiners` Units
//! drawn uniformly from its controller's hand and deck together are summoned (no Cry) and attack
//! its target first as forced attacks, in lane order, before the Prime's own combat; joiners stay.
//! A declared attack whose target is gone is cancelled, the exertion spent.
//!
//! Everything parked is plain JSON (R113): the hand pick's prompt answers through this module, and
//! the owed combat or bounce runs here. Every draw is the match rng's.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::combat::AttackTarget;
use crate::script::EngineSink;
use crate::state::{CardInstance, DeclaredAttack, EngineError, GameState, Resume, find_instance};
use crate::wire::{CardType, PlayerId, PromptKind, Row, Selection};

/// R113: the `resume.hook` of the engine sequence this module parks — a replaced attack's hand
/// pick, and the rest of a Prime's combat or a substitute's bounce a question stopped.
pub const ATTACK_SUMMON_HOOK: &str = "@attackSummon";

/// Where the owed record sits inside the prompt's and the work item's `resume.data`.
const OWED_KEY: &str = "attackSummon";

/// The hand pick this module's prompt asks for.
const PICK_STEP: &str = "pick";

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

/// Whether `replace_attacker` replaced the attack: no substitute, a hand pick asked, or the
/// summoned substitute making it.
pub enum Replaced {
    No,
    Asked,
    By(CardInstance),
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

/// R1202: what attacks when `attacker` would — its controller's Unit from their hand instead, or
/// itself. `No` unless the attacker carries `attack_from_hand`, its controller holds a Unit, and a
/// unit zone is open. One Unit is summoned at once; several ask the controller's pick (a `hand`
/// prompt, held on the opponent's turn under its own clock, R79 — one Unit is no prompt, R129).
pub fn replace_attacker(
    sink: &mut EngineSink<'_>,
    attacker: &CardInstance,
    target: &AttackTarget,
    forced: bool,
) -> Replaced {
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
    if units.len() == 1 {
        let Some(unit) = units
            .first()
            .and_then(|pick| summon_hand_unit(sink, attacker, pick))
        else {
            return Replaced::No;
        };
        return Replaced::By(unit);
    }
    let owed = OwedAttackSummon {
        attacker_id: attacker.id.clone(),
        target_id: crate::reduce::attack_target_id(target),
        forced,
        bounce: crate::scripts::flags_of(sink.state, attacker).bounce_attacker == Some(true),
        declared: None,
        instead_of: None,
        since: crate::stays::exit_mark(sink.state),
    };
    let mut data = IndexMap::new();
    data.insert(
        OWED_KEY.to_string(),
        serde_json::to_value(&owed).expect("an owed attack summon is plain JSON (§10.1)"),
    );
    let options: Vec<crate::state::PromptOption> = units
        .iter()
        .map(|card| crate::state::PromptOption {
            key: format!("instance:{}", card.id),
            label: crate::catalog::def_of(Some(sink.state), &card.def_id)
                .name
                .clone(),
            selection: Selection::Instance {
                instance_id: card.id.clone(),
            },
            cost: None,
            radiant: None,
        })
        .collect();
    crate::prompts::open_prompt(
        sink,
        crate::prompts::OpenPromptArgs {
            player: controller,
            kind: PromptKind::Hand,
            aim: None,
            prompt: "Choose a Unit to make the attack".to_string(),
            options,
            min: Some(1),
            max: Some(1),
            budget: None,
            owner: None,
            resume: Resume {
                def_id: attacker.def_id.clone(),
                hook: ATTACK_SUMMON_HOOK.to_string(),
                step: PICK_STEP.to_string(),
                radiant: attacker.radiant,
                instance_id: Some(attacker.id.clone()),
                data,
            },
        },
    );
    Replaced::Asked
}

/// R1202: the attack the pick makes — summoned if that is still possible, else the declarer
/// itself (MD-E14: with nothing to summon Windfast attacks itself). A declared attack keeps its
/// window; a forced one stays forced.
fn continue_pick(
    sink: &mut EngineSink<'_>,
    owed: &OwedAttackSummon,
    pick: Option<Selection>,
) -> Result<(), EngineError> {
    let Some(declarer) = find_instance(sink.state, &owed.attacker_id).cloned() else {
        return Ok(());
    };
    let unit = match &pick {
        Some(Selection::Instance { instance_id }) => find_instance(sink.state, instance_id)
            .cloned()
            .and_then(|card| summon_hand_unit(sink, &declarer, &card)),
        _ => None,
    };
    let Some(target) = crate::combat::attack_target_of(sink.state, &owed.target_id) else {
        // The target is gone: nothing attacks, but a summoned substitute still bounces home.
        if owed.bounce
            && let Some(unit) = unit
        {
            bounce_after(sink, &unit.id, crate::stays::exit_mark(sink.state));
        }
        return Ok(());
    };
    if owed.forced {
        let (attacker, instead_of) = match unit {
            Some(unit) => (unit, Some(owed.attacker_id.clone())),
            None => (declarer, None),
        };
        let bounce = instead_of.is_some() && owed.bounce;
        crate::combat::force_attack_instead(sink, &attacker, &target, instead_of.as_deref());
        if bounce {
            bounce_after(sink, &attacker.id, crate::stays::exit_mark(sink.state));
        }
        return Ok(());
    }
    match unit {
        Some(unit) => {
            crate::combat::declare_instead(sink, &unit, &target, Some(&owed.attacker_id), owed.bounce)
        }
        None => crate::combat::declare_instead(sink, &declarer, &target, None, false),
    }
}

/// R122: the answer to the replacement's hand pick — validated as any prompt's, filed, and the
/// sequence goes on: the summon, then the attack it makes. Then the rest of what the question
/// interrupted drains in R113's order, as `prompts::answer_prompt` does for a card's own step.
pub fn answer_pick(
    sink: &mut EngineSink<'_>,
    answer: &crate::prompts::AnswerInput,
) -> Result<(), EngineError> {
    let Some(pending) = sink.state.pending.clone() else {
        return Err(EngineError::new("no prompt is open"));
    };
    crate::prompts::why_answer_refused(&pending, answer)?;
    let owed = owed_of(&pending.resume.data);
    crate::prompts::close_prompt(sink);
    crate::work::begin_work_cascade(sink);
    if let Some(owed) = owed {
        let pick = crate::prompts::in_offered_order(&pending, &answer.selection)
            .into_iter()
            .next();
        continue_pick(sink, &owed, pick)?;
    }
    crate::work::drain_work(sink);
    Ok(())
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
pub fn owe_combat(
    sink: &mut EngineSink<'_>,
    attacker_id: &str,
    target: &AttackTarget,
    forced: bool,
    bounce: bool,
    declared: Option<DeclaredAttack>,
    instead_of: Option<&str>,
    since: u32,
) {
    owe_record(
        sink,
        COMBAT_STEP,
        &OwedAttackSummon {
            attacker_id: attacker_id.to_string(),
            target_id: crate::reduce::attack_target_id(target),
            forced,
            bounce,
            declared,
            instead_of: instead_of.map(str::to_string),
            since,
        },
    );
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
