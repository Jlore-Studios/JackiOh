//! Positions, exertion, attack validation and combat resolution (SPEC §4.1, §4.2, §4.3; BUILD M2-T1,
//! M2-T2, M2-T4). The damage pipeline of §4.4 lives in `damage.rs` and the state check of §4.5 in
//! `state_check.rs`: this module orders them, it never re-implements them.
//!
//! Nothing here is random and nothing here asks the player *directly*: the client's `attack` and
//! `switchPosition` actions come in through `reduce`, and every refusal is a string the action layer
//! hands back untouched (CLAUDE.md rule 4).
//!
//! The one thing that is not a plain function of the state is §4.2 step 4, the trap window between a
//! declaration and its damage. A trap that fires there can prompt its controller, and a prompt is
//! state rather than a callback (§9.3), so `declare_attack` is a resumable sequence: it puts the
//! declaration in `state.declared_attack`, hands the `attackDeclared` event to `traps::run_trap_window`,
//! and — only if that pauses — owes step 5's combat to `state.work` under `ATTACK_WINDOW_WORK`
//! (R113, R117). Everything the resume needs is an id, because the window can replace every instance
//! in the state before step 5 runs (R44's AI turn drives `reduce`, which clones).
//!
//! Port of `packages/engine/src/combat.ts` (part 3). TS passed live `CardInstance` objects and wrote
//! through them; here a card argument is the instance as the caller had it, and this module reads and
//! writes the card under that id in the state as it stands now (`find_instance`), which is what the
//! TS object was. A card no longer anywhere in the state has ceased to exist, so it is on no field.
//! TS's `registerWorkHandler` and `registerDeclarationCheck` calls go (SURFACE §6.6): `work.rs`'s
//! dispatcher calls `run_owed_attack`, `run_owed_forced_run`, `run_owed_forced_random` and
//! `run_owed_after_attack` directly, and `traps.rs` calls `declared_attack_stands`.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::{LANE_RESTRICTED_ATTACKS, WINDFURY_ATTACKS};
use crate::damage::{DamageArgs, DamageFlags, DamageTarget, deal_damage};
use crate::script::{EngineSink, TargetedWhat};
use crate::state::{
    CardInstance, DeclaredAttack, EngineError, Exertion, GameState, Position, Resume, WorkItem,
    find_instance, find_instance_mut,
};
use crate::wire::{GameEvent, KeywordKind, PLAYER_IDS, PlayerId, has_keyword, opponent_of};
use crate::work::{PausedStep, WorkPlan, owe, paused as is_paused, paused_of};

/// What an attack can be declared on (§4.2 step 2): an enemy unit or the enemy hero. It is the
/// damage pipeline's own target type, so a declared attack hands its target straight to `deal_damage`.
pub type AttackTarget = DamageTarget;

/// The two halves of a unit's turn (§4.1). A unit has one of them; Deft Duelist has both (R49).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ExertionKind {
    Attack,
    Switch,
}

/// Every entry point returns the §4.2 refusal reason, or nothing when it went through (TS
/// `{ error?: string }`, SURFACE §4.4.9).
pub type CombatResult = Result<(), EngineError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct SwitchPositionOptions {
    /// False for a switch that is an effect rather than the player's action (R20).
    pub spend_exertion: Option<bool>,
    /// Where to put the unit; by default it flips.
    pub to: Option<Position>,
}

/// B5 E35: what a forced attack on "a random enemy" draws from (TS `"enemies" | "enemyUnits"`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum AttackAmong {
    Enemies,
    EnemyUnits,
}

// ---------------------------------------------------------------------------------------------
// Live reads (TS held the live object) and private copies of `scripts.rs`'s readers (fullsend
// rule 5).
// ---------------------------------------------------------------------------------------------

/// The card as it stands in the state now, or `None` once it is nowhere (it ceased to exist).
fn live_card(state: &GameState, card: &CardInstance) -> Option<CardInstance> {
    find_instance(state, &card.id).cloned()
}

/// The card as it stands now, or the one given when it is nowhere: for reads that TS made off the
/// object it was handed whatever became of it.
fn live_or_given(state: &GameState, card: &CardInstance) -> CardInstance {
    live_card(state, card).unwrap_or_else(|| card.clone())
}

/// The target with its unit read as it stands now (the hero as it is).
fn live_target_or_given(state: &GameState, target: &AttackTarget) -> AttackTarget {
    match target {
        DamageTarget::Unit { instance } => DamageTarget::Unit {
            instance: live_or_given(state, instance),
        },
        DamageTarget::Hero { player } => DamageTarget::Hero { player: *player },
    }
}

/// Whether the card under this id acts on the field now; a card that is nowhere does not.
fn active_now(state: &GameState, id: &str) -> bool {
    find_instance(state, id).is_some_and(|card| is_active_on_field(state, card))
}

fn to_json<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

/// A combat hit's flags: `{ combat: true }`.
fn combat_flags() -> Option<DamageFlags> {
    Some(DamageFlags {
        combat: Some(true),
        ..DamageFlags::default()
    })
}

/// An engine sequence's own continuation: no card, the hook and step named, `data` as given.
fn engine_resume(hook: &str, step: &str, data: IndexMap<String, Value>) -> Resume {
    Resume {
        def_id: String::new(),
        hook: hook.to_string(),
        step: step.to_string(),
        radiant: false,
        instance_id: None,
        data,
    }
}

/// R49: a unit with Deft has two exertions, one attack and one switch, where every other unit has
/// one. The keyword is read through §10.4's layers, so a granted Deft counts the same as a printed
/// one (patch v0.2.4 made R49's flag a keyword).
fn has_two_exertions(state: &GameState, unit: &CardInstance) -> bool {
    crate::layers::unit_has(state, unit, KeywordKind::Deft)
}

/// R636: how many attacks the unit may declare in a turn — two with Windfury, read through the layers
/// (§10.4) so a keyword granted or lost mid-turn counts from that moment, one otherwise.
pub fn attacks_per_turn(state: &GameState, unit: &CardInstance) -> i32 {
    if crate::layers::unit_has(state, unit, KeywordKind::Windfury) {
        WINDFURY_ATTACKS
    } else {
        1
    }
}

/// §4.1: one exertion per turn, so a unit that attacked cannot switch and a unit that switched
/// cannot attack (R6). A unit with Deft spends the two independently (R49). R636: a unit with
/// Windfury attacks again until it has declared `attacks_per_turn`, and the first of them already
/// spends the exertion a switch needs.
pub fn has_exertion(state: &GameState, unit: &CardInstance, kind: ExertionKind) -> bool {
    let spent = &unit.exertion;
    if kind == ExertionKind::Attack {
        let attacks = spent.attacks.unwrap_or(if spent.attacked { 1 } else { 0 });
        if attacks >= attacks_per_turn(state, unit) {
            return false;
        }
        return has_two_exertions(state, unit) || !spent.switched;
    }
    if has_two_exertions(state, unit) {
        return !spent.switched;
    }
    !spent.attacked && !spent.switched
}

/// §4.1: a unit that entered the field this turn is summoning sick. A unit that enters again, a
/// Reborn body included, entered it on that turn like any other (R83), and so does a unit whose
/// controller changed this turn (R171, through `enter_new_side` below), so this is the one comparison.
pub fn is_sick(state: &GameState, unit: &CardInstance) -> bool {
    unit.summoned_turn == Some(state.turn)
}

/// R171: a card whose controller changes has entered its new controller's side on this turn — §4.1's
/// "entered the field" for `is_sick`, and a fresh exertion for its new controller. Every path that
/// changes control on the field calls this at the moment it emits `controlChanged`, for every card
/// that changes sides (a card dormant under a Stack and a backrow card included), and for nothing
/// else: a card moving along its own side has not entered anything.
///
/// `from` is the controller the card had before. A player modifier a permanent installed and still
/// owns (#79 Twinspell's "the next Spell you play gains Echo +1", `sourceId`) is that permanent's
/// lasting effect (§5.1), and §8's conventions read its "you" as the controller, so it follows the
/// card to its new side rather than staying with the player it left.
pub fn enter_new_side(sink: &mut EngineSink<'_>, card: &CardInstance, from: PlayerId) {
    let turn = sink.state.turn;
    let mut controller = card.controller;
    if let Some(live) = find_instance_mut(sink.state, &card.id) {
        live.summoned_turn = Some(turn);
        live.exertion = Exertion {
            attacked: false,
            switched: false,
            attacks: None,
        };
        controller = live.controller;
    }
    crate::modifiers::move_sourced_modifiers(sink, &card.id, from, controller);
}

/// §3.2: the card that acts in a zone is the top of its Stack pile. A card dormant underneath one is
/// not on the field for anything: it neither acts nor can be targeted (R13).
pub fn is_active_on_field(state: &GameState, unit: &CardInstance) -> bool {
    // R446: a Unit a carrier holds acts from its backrow zone, with the carrier acting beneath it.
    crate::zones::acts_on_field(state, unit)
}

/// R446: a Unit standing on a carrier (Classic+ #33 Ivory Tower) can neither attack nor be attacked —
/// declared or forced (R53 waives position, sickness and Taunt, never this) — and so binds no attacker
/// with its Taunt (§4.2 step 3).
fn carried_out_of_combat(state: &GameState, unit: &CardInstance) -> bool {
    crate::zones::is_carried(state, unit)
}

/// Flip a unit between Attack and Defense Position (§4.1). As the player's action it spends the
/// unit's exertion; as an effect (5pek Controller, #48) it spends nothing (R20). Defense Position's
/// Taunt and Armor +1 are layers, not state, so they follow from `position` alone (§10.4).
pub fn switch_position(
    sink: &mut EngineSink<'_>,
    unit: &CardInstance,
    options: SwitchPositionOptions,
) -> CombatResult {
    let spend_exertion = options.spend_exertion != Some(false);
    let Some(unit) = live_card(sink.state, unit) else {
        return Err(EngineError::new("that unit is not on the field"));
    };
    if !is_active_on_field(sink.state, &unit) {
        return Err(EngineError::new("that unit is not on the field"));
    }

    let from = unit.position.unwrap_or(Position::Atk);
    let to = options.to.unwrap_or(if from == Position::Atk {
        Position::Def
    } else {
        Position::Atk
    });

    // R91: asking for the position the unit already holds "does nothing: no error, no event and no
    // exertion spent". It is read before the exertion because there is no switch to refuse — a unit
    // that has already acted is not told it has, since nothing was going to be spent. #48's "switch
    // every unit" reaches units already in the position it would set.
    if to == from {
        return Ok(());
    }

    if spend_exertion && !has_exertion(sink.state, &unit, ExertionKind::Switch) {
        return Err(EngineError::new("that unit has already acted this turn"));
    }
    // §4.1: Spikey Pillow cannot be switched to Defense Position, by an action or by an effect.
    if to == Position::Def && crate::scripts::flags_of(sink.state, &unit).never_defense == Some(true) {
        return Err(EngineError::new("that unit cannot be in Defense Position"));
    }

    if let Some(live) = find_instance_mut(sink.state, &unit.id) {
        live.position = Some(to);
        if spend_exertion {
            live.exertion.switched = true;
        }
    }
    sink.events.push(GameEvent::PositionSwitched {
        instance_id: unit.id.clone(),
        position: to,
    });
    Ok(())
}

/// §4.2 step 1: "choose an attacker that can attack", in the order the step lists — exertion
/// unspent, Attack Position, not sick unless Rush or Charge, attack above 0, no "Can't attack" —
/// with §3.2's "is it even on the field" first. The order is what decides which reason a unit that
/// fails several ways reports, so R6's spent exertion is read before its position.
fn why_cannot_declare(state: &GameState, attacker: &CardInstance) -> Result<(), EngineError> {
    if !is_active_on_field(state, attacker) {
        return Err(EngineError::new("that unit is not on the field"));
    }
    if carried_out_of_combat(state, attacker) {
        return Err(EngineError::new("a Unit on a carrier cannot attack"));
    }
    if !has_exertion(state, attacker, ExertionKind::Attack) {
        return Err(EngineError::new("that unit has already acted this turn"));
    }

    let view = crate::layers::unit_view(state, attacker);
    if view.position != Position::Atk {
        return Err(EngineError::new("only Attack-Position units may attack"));
    }
    if is_sick(state, attacker)
        && !has_keyword(&view.keywords, KeywordKind::Rush)
        && !has_keyword(&view.keywords, KeywordKind::Charge)
    {
        return Err(EngineError::new("that unit is summoning sick"));
    }
    // R7: a 0-attack unit cannot declare an attack, printed (Big D-fender) or dragged there (§10.4).
    if view.attack <= 0 {
        return Err(EngineError::new("a unit with 0 attack cannot attack"));
    }
    if has_keyword(&view.keywords, KeywordKind::CantAttack) {
        return Err(EngineError::new("that unit cannot attack"));
    }
    Ok(())
}

/// Every enemy unit whose Taunt forces the target, printed, granted or from Defense (§4.2 step 3) —
/// among the units this attacker may attack at all: B5 E35 has a Taunt bind only the attackers that
/// could legally attack it, so a Taunt only its own lane may attack (Classic+ #19.1) leaves an attacker
/// in another lane free, and one that cannot be attacked — a Unit a carrier holds (R446) among them —
/// binds nobody.
fn taunt_wall(state: &GameState, attacker: &CardInstance, enemy: PlayerId) -> Vec<CardInstance> {
    crate::zones::active_units_of(state, enemy)
        .into_iter()
        .filter(|unit| {
            !carried_out_of_combat(state, unit)
                && has_keyword(
                    &crate::layers::unit_view(state, unit).keywords,
                    KeywordKind::Taunt,
                )
                && crate::restrictions::attack_restriction(
                    state,
                    attacker,
                    &DamageTarget::Unit {
                        instance: (*unit).clone(),
                    },
                )
                .is_ok()
        })
        .cloned()
        .collect()
}

/// Steps 1 to 3 of §4.2, as the reason the attack is refused, or `Ok` when it is legal. The action
/// layer surfaces this string as the error of an `attack` action, and `attack_targets` filters with
/// it, so the two can never disagree.
pub fn why_cannot_attack(
    state: &GameState,
    attacker: &CardInstance,
    target: &AttackTarget,
) -> Result<(), EngineError> {
    why_cannot_declare(state, attacker)?;

    // Step 2: an enemy unit, or the enemy hero.
    let enemy = opponent_of(attacker.controller);
    match target {
        DamageTarget::Hero { player } => {
            if *player != enemy {
                return Err(EngineError::new("that target is not an enemy"));
            }
        }
        DamageTarget::Unit { instance } => {
            if instance.controller != enemy {
                return Err(EngineError::new("that target is not an enemy"));
            }
            if !is_active_on_field(state, instance) {
                return Err(EngineError::new("that unit is not on the field"));
            }
            if carried_out_of_combat(state, instance) {
                return Err(EngineError::new("a Unit on a carrier cannot be attacked"));
            }
            // R5 decided attacks are not lane-restricted; flipping the constant restricts a unit to the
            // lane it stands in, which is the only reading §3.1's lanes give an attack.
            if LANE_RESTRICTED_ATTACKS {
                let from = crate::zones::slot_of(state, attacker);
                let to = crate::zones::slot_of(state, instance);
                if let (Some(from), Some(to)) = (from, to)
                    && from.lane != to.lane
                {
                    return Err(EngineError::new("that target is not in this unit's lane"));
                }
            }
        }
    }
    // B5 E35: the unit restrictions — can't be attacked, attacked only from its own lane, can't attack
    // or be attacked — which a forced attack obeys as well (`force_attack`).
    crate::restrictions::attack_restriction(state, attacker, target)?;

    // §6.1: Rush lifts sickness for unit targets only, Charge for units and the hero. Step 1 has
    // already established that a sick attacker has one of the two.
    if matches!(target, DamageTarget::Hero { .. })
        && is_sick(state, attacker)
        && !has_keyword(
            &crate::layers::unit_view(state, attacker).keywords,
            KeywordKind::Charge,
        )
    {
        return Err(EngineError::new("Rush cannot hit the hero on its summon turn"));
    }

    // Step 3: while any enemy unit this attacker may attack has Taunt, the target must be one of them.
    let wall = taunt_wall(state, attacker, enemy);
    let in_wall = match target {
        DamageTarget::Unit { instance } => wall.iter().any(|unit| unit.id == instance.id),
        DamageTarget::Hero { .. } => false,
    };
    if !wall.is_empty() && !in_wall {
        return Err(EngineError::new("a Taunt unit must be attacked first"));
    }

    Ok(())
}

pub fn can_attack(state: &GameState, attacker: &CardInstance, target: &AttackTarget) -> bool {
    why_cannot_attack(state, attacker, target).is_ok()
}

/// Every target this attacker may legally declare on, enemy units in lane order and then the enemy
/// hero (§4.2). Empty when the attacker cannot attack at all, which is what the action layer and the
/// AI policy read instead of enumerating attacks themselves.
pub fn attack_targets(state: &GameState, attacker: &CardInstance) -> Vec<AttackTarget> {
    let enemy = opponent_of(attacker.controller);
    let mut candidates: Vec<AttackTarget> = crate::zones::active_units_of(state, enemy)
        .into_iter()
        .cloned()
        .map(|instance| DamageTarget::Unit { instance })
        .collect();
    candidates.push(DamageTarget::Hero { player: enemy });
    candidates
        .into_iter()
        .filter(|target| can_attack(state, attacker, target))
        .collect()
}

/// §4.2 step 2's two possibilities as one id: an enemy unit's instance id, or `hero-<player>`.
const HERO_TARGET_PREFIX: &str = "hero-";

fn target_id_of(target: &AttackTarget) -> String {
    match target {
        DamageTarget::Unit { instance } => instance.id.clone(),
        DamageTarget::Hero { player } => format!("{HERO_TARGET_PREFIX}{player}"),
    }
}

/// The inverse of `target_id_of`: the target an `attackDeclared` event or an open `declaredAttack`
/// names. Exported because the ids are all a paused attack and a trap trigger have to go on — #96
/// My Pawn reads the event, and §4.2 step 5 reads the state back after the window — and because the
/// `hero-<player>` spelling is this module's, so nobody else should be parsing it.
pub fn attack_target_of(state: &GameState, target_id: &str) -> Option<AttackTarget> {
    if let Some(named) = target_id.strip_prefix(HERO_TARGET_PREFIX) {
        let player = PLAYER_IDS.into_iter().find(|id| id.as_str() == named)?;
        return Some(DamageTarget::Hero { player });
    }
    let unit = find_instance(state, target_id)?;
    Some(DamageTarget::Unit {
        instance: unit.clone(),
    })
}

/// §4.3's "if D is destroyed here it deals nothing". The state check has not run yet, so this asks
/// the question §4.5 step 1 will: 0 or less health or a destroy mark, while an Indestructible unit
/// falls only to max health 0 or less (R69).
fn has_fallen(state: &GameState, unit: &CardInstance) -> bool {
    if !is_active_on_field(state, unit) {
        return true;
    }
    let view = crate::layers::unit_view(state, unit);
    if has_keyword(&view.keywords, KeywordKind::Indestructible) {
        return view.max_health <= 0;
    }
    view.health <= 0 || unit.marked_destroyed == Some(true)
}

/// `has_fallen` on the card under this id now; a card that is nowhere has fallen.
fn has_fallen_now(state: &GameState, id: &str) -> bool {
    match find_instance(state, id) {
        Some(card) => has_fallen(state, card),
        None => true,
    }
}

/// §4.4 step 10: Cleave deals the attacker's attack to each unit adjacent to the target as separate
/// instances. It belongs to the attack rather than to the hit, so it lands even when Divine Shield,
/// Indestructible or the zero rule stopped the hit on the defender (R63); adjacency never crosses
/// sides (§3.1), so the attacker's own neighbours are never cleaved.
fn cleave(sink: &mut EngineSink<'_>, attacker: &CardInstance, target: &AttackTarget, amount: i32) {
    let DamageTarget::Unit { instance } = target else {
        return;
    };
    if !has_keyword(
        &crate::layers::unit_view(sink.state, attacker).keywords,
        KeywordKind::Cleave,
    ) {
        return;
    }

    let Some(defender) = live_card(sink.state, instance) else {
        return;
    };
    let Some(at) = crate::zones::slot_of(sink.state, &defender) else {
        return;
    };
    for slot in crate::zones::adjacent(at) {
        let Some(neighbour) = crate::zones::card_at(sink.state, slot).cloned() else {
            continue;
        };
        deal_damage(
            sink,
            DamageArgs {
                source: Some(attacker.clone()),
                target: DamageTarget::Unit { instance: neighbour },
                amount,
                flags: combat_flags(),
            },
        );
    }
}

/// The attacker's hit and its Cleave, with the attacker read as it stands when it strikes.
fn strike(sink: &mut EngineSink<'_>, attacker: &CardInstance, target: &AttackTarget, attack: i32) {
    let source = live_or_given(sink.state, attacker);
    deal_damage(
        sink,
        DamageArgs {
            source: Some(source),
            target: target.clone(),
            amount: attack,
            flags: combat_flags(),
        },
    );
    let source = live_or_given(sink.state, attacker);
    cleave(sink, &source, target, attack);
}

/// The defender's hit back on the attacker.
fn strike_back(sink: &mut EngineSink<'_>, defender: &CardInstance, attacker: &CardInstance, attack: i32) {
    let source = live_or_given(sink.state, defender);
    let target = live_or_given(sink.state, attacker);
    deal_damage(
        sink,
        DamageArgs {
            source: Some(source),
            target: DamageTarget::Unit { instance: target },
            amount: attack,
            flags: combat_flags(),
        },
    );
}

/// The exchange of §4.3, with no validation, no exertion and no state check: the First Strike step,
/// then the simultaneous step. Both attacks are read before either lands, which is what makes the
/// simultaneous step simultaneous; a hero never strikes back, and a defender in Defense Position
/// strikes back with its full attack, the position's Armor +1 applying only to what it takes.
///
/// The caller runs the state check, so both hits of one combat land before anything dies (R59).
pub fn resolve_combat(sink: &mut EngineSink<'_>, attacker: &CardInstance, target: &AttackTarget) {
    let Some(attacker) = live_card(sink.state, attacker) else {
        return;
    };
    if !is_active_on_field(sink.state, &attacker) {
        return;
    }
    let defender = match target {
        DamageTarget::Unit { instance } => {
            let Some(defender) = live_card(sink.state, instance) else {
                return;
            };
            if !is_active_on_field(sink.state, &defender) {
                return;
            }
            Some(defender)
        }
        DamageTarget::Hero { .. } => None,
    };

    let attack = crate::layers::unit_view(sink.state, &attacker).attack;

    // §4.3: when the defender is a hero, only the attacker deals damage.
    let Some(defender) = defender else {
        strike(sink, &attacker, target, attack);
        return;
    };

    // R94: both attacks are read once, here, before either hit lands. That is what makes the
    // simultaneous step simultaneous, and it is why a First Strike survivor is struck back with the
    // attack the defender had before the hit landed rather than with whatever it reads afterwards.
    let strike_back_attack = crate::layers::unit_view(sink.state, &defender).attack;

    let attacker_first = has_keyword(
        &crate::layers::unit_view(sink.state, &attacker).keywords,
        KeywordKind::FirstStrike,
    );
    let defender_first = has_keyword(
        &crate::layers::unit_view(sink.state, &defender).keywords,
        KeywordKind::FirstStrike,
    );

    // Step 1: one First Strike hits alone, and the other side answers in step 2 only if it survives.
    if attacker_first && !defender_first {
        strike(sink, &attacker, target, attack);
        if !has_fallen_now(sink.state, &defender.id) {
            strike_back(sink, &defender, &attacker, strike_back_attack);
        }
        return;
    }
    if defender_first && !attacker_first {
        strike_back(sink, &defender, &attacker, strike_back_attack);
        if !has_fallen_now(sink.state, &attacker.id) {
            strike(sink, &attacker, target, attack);
        }
        return;
    }

    // Two First Strikers strike simultaneously in step 1, two ordinary units in step 2, and either
    // way the first death does not cancel the exchange (R59).
    strike(sink, &attacker, target, attack);
    strike_back(sink, &defender, &attacker, strike_back_attack);
}

// ---------------------------------------------------------------------------------------------
// §4.2 step 4: the trap window between the declaration and the damage (R44, R100, R113, R117)
// ---------------------------------------------------------------------------------------------

/// R113: the `resume.hook` of the one work item this module parks — the rest of an attack whose
/// step 4 window a prompt interrupted. It is an engine sequence and not a card's, so the name is one
/// no `Script` can hold, and `run_owed_attack` below is what `work.rs` runs for it, the way `traps.rs`
/// owns its window and `turn.rs` its two boundaries. `work::run_work_item` raises on a hook nothing
/// knows, and an attack that cannot be resumed is exactly the lost sequence that rule exists to
/// prevent: a declaration that spent an exertion and never became damage.
pub const ATTACK_WINDOW_WORK: &str = "@attackWindow";

/// The window owes one step: §4.2 step 5, the combat the declaration has not resolved yet.
const ATTACK_COMBAT_STEP: &str = "combat";

/// R100: "an event the window is scheduled to deliver is not also offered to the immediate trap
/// check". The window below delivers `attackDeclared` itself, so §10.3's frontier must not deliver
/// it a second time — a trap that declined a non-lethal swing would otherwise be offered the same
/// declaration again *after* the damage, against a hero the attack has already hit, and a Field Trap
/// would answer one declaration twice.
///
/// `triggers.rs` copies events out of `sink.events` into `state.dispatch` starting at the frontier's
/// cursor (`triggers::dispatched`), so advancing it past this event is precisely "the frontier has
/// taken it" — it has, into this window. R100's own mechanism (`traps::TRAP_WINDOW_EVENTS`) cannot serve
/// here: it withholds an event *type*, and a forced attack's `attackDeclared` opens no window (R121)
/// and must keep reaching the immediate check like any other event a card's effect list emits.
///
/// The cursor is at the event on every path that opens a window: `reduce` hands an `attack` action a
/// sink whose event list it has not touched, and `reduce` is the only production caller. A sink a
/// test has already run a combat on is the one other shape; there the cursor is behind, and moving
/// it would silently drop the events in between, so the event is left on the frontier instead.
fn withhold_from_frontier(sink: &mut EngineSink<'_>, at: usize) {
    // Every sink over the action's list shares the frontier (`EngineSink::frontier`), so a loop on
    // another sink — a cast's step-4 window inside an effect the window's traps run (R70) — sees the
    // cursor move too.
    if crate::triggers::dispatched(sink) == at {
        crate::triggers::set_dispatched(sink, at + 1);
    }
}

/// The ordinary (non-trap) triggers on the declaration, queued in R68's order. This is the second
/// half of `triggers::dispatch_event`; its first half is the immediate trap check, which §4.2 step 4
/// replaces with the window, so calling `dispatch_event` itself would offer the event to the traps a
/// second time. Queueing only reads the board, so it cannot pause, and the entries pop in the
/// caller's own resolution loop — after the combat, exactly where they popped before step 4 existed.
///
/// R212, as `dispatch_event` reads it: the board is read after the window, which can have moved it, so
/// the events the window emitted since the declaration say how. A card that moved zones since — a
/// unit a trap in the window summoned, a Reborn body, anything My Pawn's AI turn put on the field — is
/// on a stay that did not see the declaration and does not answer it (R174), and a card whose
/// controller changed since — a unit a trap in the window stole — answers for the player who
/// controlled it when the attack was declared (R171).
fn queue_declaration_triggers(sink: &mut EngineSink<'_>, event: &GameEvent, since: &[GameEvent]) {
    let mut later: Option<crate::stays::LaterMoves> = None;
    for holder in crate::triggers::holders_answering(sink.state, event.event_type()) {
        if holder.is_trap {
            continue;
        }
        let defs = crate::triggers::triggers_on_event(&holder, event.event_type());
        if defs.is_empty() {
            continue;
        }
        let moves = later.get_or_insert_with(|| crate::stays::moves_in(since, Some(&*sink.state)));
        if moves.moved.contains(&holder.card.id) {
            continue;
        }
        let controller = moves
            .controller_before
            .get(&holder.card.id)
            .copied()
            .unwrap_or(holder.controller);
        let mut answering = holder.clone();
        answering.controller = controller;
        for def in &defs {
            crate::triggers::queue_trigger(sink, &answering, def, event);
        }
    }
}

/// Park the rest of the attack (R113). `work.rs` owns `state.work`, so this only ever calls `owe`:
/// the item lands at `state.work_cursor`, which `traps::run_trap_window` has just advanced past the
/// traps *it* still owes, so the window finishes before the combat it precedes. Nothing is held but
/// the declaration's id, which is plain JSON, so the paused attack survives a round trip.
///
/// R117: this is called at the moment the window pauses and never in advance. While `declare_attack`
/// is on the stack the combat is `declare_attack`'s alone, so a resolution loop running *inside* the
/// window — My Pawn's AI playout drives `reduce`, which settles — can neither take nor re-run it.
/// Pre-parking a sequence's continuation is what made a played card's Cry fire twice (R1, R117).
fn owe_declared_attack(sink: &mut EngineSink<'_>, id: &str) {
    let mut data = IndexMap::new();
    data.insert("declaredAttack".to_string(), json!(id));
    owe(sink, engine_resume(ATTACK_WINDOW_WORK, ATTACK_COMBAT_STEP, data));
}

/// Close the window, if this declaration is still the one that holds it open.
fn close_window(state: &mut GameState, id: &str) -> Option<DeclaredAttack> {
    if state.declared_attack.as_ref().is_some_and(|open| open.id == id) {
        state.declared_attack.take()
    } else {
        None
    }
}

/// R220: whether a declaration still stands as it was declared, which §4.2 step 5 asks before it
/// resolves anything and the traps later in the window ask before they answer it (`traps.rs`).
///
/// §4.2 step 2 makes an attack "an enemy unit, or the enemy hero", and a trap in step 4's window can
/// change that before step 5: destroy the attacker or its target, steal either, swap the boards. The
/// attack is aimed at the stays that were declared (R174), so an attacker or a target that has left
/// the field is gone even when a Reborn body stands in its zone — a new arrival, summoning sick
/// (R83), which declared nothing. The attacker must still be the declarer's: p1 declared the attack,
/// and a unit p2 now controls is not p1's to attack with. And the target must still be the
/// attacker's enemy (R173 says the same of a forced attack). Otherwise the attack is over, like one
/// My Pawn cancelled: no combat, and the exertion stays spent (R44). Hearthstone cancels an attack
/// whose attacker or defender has left play.
pub fn declared_attack_stands(state: &GameState, open: &DeclaredAttack) -> bool {
    if open.cancelled {
        return false;
    }
    let mark = open.exits_from.unwrap_or_else(|| crate::stays::exit_mark(state));
    let Some(attacker) = find_instance(state, &open.attacker_id) else {
        return false;
    };
    if !is_active_on_field(state, attacker) {
        return false;
    }
    if crate::stays::left_field_after(state, mark, &attacker.id) {
        return false;
    }
    if let Some(by) = open.by
        && attacker.controller != by
    {
        return false;
    }
    let Some(target) = attack_target_of(state, &open.target_id) else {
        return false;
    };
    if let DamageTarget::Unit { instance } = &target {
        if !is_active_on_field(state, instance) {
            return false;
        }
        if crate::stays::left_field_after(state, mark, &instance.id) {
            return false;
        }
    }
    is_enemy_of(attacker, &target)
}

/// §4.2 step 5, once step 4's window has closed: resolve the combat the declaration still owes, then
/// run the state check — when the declaration still stands (R220).
///
/// The attacker and the target are read back out of `state.declared_attack` by id rather than from
/// the caller's own variables, because the window can have replaced every instance in the state:
/// R44's AI turn drives `reduce`, which clones, and `subsystems::ai_policy::adopt_state` copies the
/// clone back field by field. Ids survive that; object references do not.
///
/// R94 is untouched. "Both units' attack is read at the start of the combat" — that read is
/// `resolve_combat`'s, and the combat starts here, after the window. Nothing reads either attack
/// before it, so a trap that buffed or shrank a unit inside the window is read once, in the right
/// order, and a First Strike survivor is still struck back with the attack the defender had when the
/// combat began.
fn resolve_declared_attack(sink: &mut EngineSink<'_>, id: &str) {
    // A window opened inside this one owns the field now, and this declaration is over either way.
    // The only Core shape is My Pawn handing the turn to the AI policy, which cancels first (R44).
    let Some(open) = close_window(sink.state, id) else {
        return;
    };
    // R1202: the substitute goes home even when its attack never fought.
    let bounce = open.bounce_after == Some(true);
    let since = open
        .exits_from
        .unwrap_or_else(|| crate::stays::exit_mark(sink.state));
    if open.cancelled {
        if bounce {
            crate::attack_summon::bounce_after(sink, &open.attacker_id, since);
        }
        return;
    }
    if sink.state.result.is_some() {
        return;
    }
    if !declared_attack_stands(sink.state, &open) {
        if bounce {
            crate::attack_summon::bounce_after(sink, &open.attacker_id, since);
        }
        return;
    }

    let Some(attacker) = find_instance(sink.state, &open.attacker_id).cloned() else {
        return;
    };
    let Some(target) = attack_target_of(sink.state, &open.target_id) else {
        return;
    };

    // R1203: the Prime's joiners attack first. Anything else fights at once, as before, with the
    // substitute's bounce after its combat.
    if crate::scripts::flags_of(sink.state, &attacker).attack_joiners != Some(true) {
        resolve_combat(sink, &attacker, &target);
        close_combat(sink, &attacker, &target, false);
        if bounce {
            crate::attack_summon::bounce_after(sink, &attacker.id, since);
        }
        return;
    }
    match crate::attack_summon::joiners_first(sink, &attacker, &target, since) {
        crate::attack_summon::JoinOutcome::Paused => {
            crate::attack_summon::owe_combat(
                sink,
                &attacker.id,
                &target,
                false,
                bounce,
                Some(open),
                None,
                since,
            );
        }
        crate::attack_summon::JoinOutcome::Fight => {
            prime_combat(
                sink,
                &crate::attack_summon::OwedAttackSummon {
                    attacker_id: attacker.id.clone(),
                    target_id: target_id_of(&target),
                    forced: false,
                    bounce,
                    declared: Some(open),
                    instead_of: None,
                    since,
                },
            );
        }
    }
}

/// R1202, R1203: the combat a replacement or rider parked (`attack_summon::run_owed`).
///
/// A declared attack that still stands fights now; one whose target has left the field before its
/// combat is cancelled (`attackCancelled`, MD-E15's shape — the exertion stays spent). A forced
/// attack fights when its attacker and its target still act since `since`. Then, when the
/// substitute owes it, the bounce home — whether or not it fought.
pub(crate) fn prime_combat(sink: &mut EngineSink<'_>, owed: &crate::attack_summon::OwedAttackSummon) {
    if let Some(open) = &owed.declared {
        if declared_attack_stands(sink.state, open) {
            let attacker = find_instance(sink.state, &open.attacker_id).cloned();
            let target = attack_target_of(sink.state, &open.target_id);
            if let (Some(attacker), Some(target)) = (attacker, target) {
                resolve_combat(sink, &attacker, &target);
                close_combat(sink, &attacker, &target, false);
            }
        } else {
            sink.events.push(GameEvent::AttackCancelled {
                attacker_id: open.attacker_id.clone(),
                target_id: open.target_id.clone(),
                by_instance_id: open.attacker_id.clone(),
            });
        }
    } else {
        let attacker = find_instance(sink.state, &owed.attacker_id).cloned();
        let target = attack_target_of(sink.state, &owed.target_id);
        match (attacker, target) {
            (Some(attacker), Some(target))
                if is_active_on_field(sink.state, &attacker)
                    && !crate::stays::left_field_after(sink.state, owed.since, &attacker.id)
                    && target_acts_since(sink.state, &target, owed.since) =>
            {
                resolve_combat(sink, &attacker, &target);
                close_combat(sink, &attacker, &target, true);
            }
            _ => {}
        }
    }
    if owed.bounce {
        crate::attack_summon::bounce_after(sink, &owed.attacker_id, owed.since);
    }
}

/// Whether a forced attack's target still acts since `since`: a unit on the field on its stay, a
/// hero always.
fn target_acts_since(state: &GameState, target: &AttackTarget, since: u32) -> bool {
    match target {
        DamageTarget::Hero { .. } => true,
        DamageTarget::Unit { instance } => find_instance(state, &instance.id).is_some_and(|live| {
            is_active_on_field(state, live) && !crate::stays::left_field_after(state, since, &live.id)
        }),
    }
}

/// §10.3, §4.2 step 4: the traps answer what the window did before step 5. A trap in the window is an
/// effect like any other, so its events — the hit it dealt the attacker, the unit it summoned — reach
/// the traps at once, and a trap answering them fires and resolves inside step 4, before any damage of
/// the attack: one that destroys the hit attacker leaves no attacker for step 5 (R220). The ordinary
/// triggers they wake are queued behind the declaration's and wait for the loop after the combat, as
/// they did before. A trap that asks here pauses step 4 again, and the combat stays owed behind its
/// answer (R113). False when the combat must not resolve now.
fn window_answered(sink: &mut EngineSink<'_>, id: &str) -> bool {
    crate::triggers::dispatch_pending(sink);
    if sink.state.result.is_some() {
        close_window(sink.state, id);
        return false;
    }
    if sink.state.pending.is_some() {
        owe_declared_attack(sink, id);
        return false;
    }
    true
}

/// `work.rs`'s handler for a parked attack: the same attack, continued where it stopped (R113) — the
/// window's own events answered first, the answer's included (`window_answered`).
pub fn run_owed_attack(sink: &mut EngineSink<'_>, item: &WorkItem) {
    let Some(id) = item
        .resume
        .data
        .get("declaredAttack")
        .and_then(Value::as_str)
        .map(str::to_string)
    else {
        return;
    };
    if !window_answered(sink, &id) {
        return;
    }
    resolve_declared_attack(sink, &id);
}

/// The player's attack: §4.2 steps 1 to 5. Validation first, then step 4 — the exertion, then the
/// trap window — and only then step 5's combat and state check.
///
/// R1202: an attack Windfast would make, its substitute makes instead (`attack_summon`): the
/// exertion below is Windfast's, spent before the pick, and no `attackDeclared` of its own is ever
/// emitted for it.
///
/// Step 4 in full: "Declaring the attack has now spent the attacker's exertion, before any damage.
/// Trap window: My Pawn checks whether the hit would be lethal and, if so, cancels the attack; the
/// exertion is not given back, so the attack is gone either way (R44)." So the declaration goes into
/// `state.declared_attack` before any trap sees it — that record is what `effects::combat::cancel_attack`
/// marks (§6.3 "Cancel an attack") — and the traps are offered the event through
/// `traps::run_trap_window`, the same scheduled-window entry point §2.2's end-of-turn window uses. That
/// is not an analogy: it is the one implementation, so the ordering, the R68 side order and, above
/// all, R113's parking of the traps a prompt stopped the window from reaching are shared rather than
/// re-derived here.
///
/// A trap in the window can prompt its controller, so the rest of the attack is a resumable sequence
/// like the two turn boundaries: at the moment the window pauses, and never before it (R117), the
/// combat is owed to `state.work` and `run_owed_attack` above picks it up when the answer drains the
/// queue. `state.work` is drained ahead of the trigger queue, and the traps the window still owes
/// are parked on `state.work` too, in front of the combat — which is why the window uses
/// `run_trap_window` rather than `triggers::dispatch_event`, whose remainder would wait *behind* the
/// combat in the trigger queue and so fire after the damage it exists to pre-empt.
pub fn declare_attack(
    sink: &mut EngineSink<'_>,
    attacker: &CardInstance,
    chosen: &AttackTarget,
) -> CombatResult {
    let attacker = live_or_given(sink.state, attacker);
    let chosen = live_target_or_given(sink.state, chosen);
    why_cannot_attack(sink.state, &attacker, &chosen)?;

    // B5 E5, E9: step 2's target chosen, and before the trap window, "a friendly unit is targeted" —
    // a card of the defender's that interposes (from the hand) is summoned and the attack moves to it
    // (`replacements::answer_targeting`: `summoned`, then `redirected`). An attack carries no source
    // card, so a `by: "spell"` card (Classic #33 Joro, R651) never answers one.
    let interposer = match &chosen {
        DamageTarget::Unit { instance } => {
            crate::replacements::answer_targeting(sink, instance, attacker.controller, TargetedWhat::Attack)
        }
        DamageTarget::Hero { .. } => None,
    };
    let target: AttackTarget = match interposer {
        None => chosen,
        Some(instance) => DamageTarget::Unit { instance },
    };

    // Step 4's first sentence. R44 never gives this back, so a cancelled attack is gone either way.
    // R636: only a repeat attack writes `attacks`, so an ordinary exertion keeps its two-flag shape.
    // R1202: this is the declarer's exertion — Windfast's, whose substitute spends it without
    // emitting its own declaration.
    if let Some(live) = find_instance_mut(sink.state, &attacker.id) {
        let attacks = live
            .exertion
            .attacks
            .unwrap_or(if live.exertion.attacked { 1 } else { 0 })
            + 1;
        live.exertion.attacked = true;
        if attacks > 1 {
            live.exertion.attacks = Some(attacks);
        }
    }

    // R1202: the attack Windfast would make, a Unit summoned from its controller's hand makes
    // instead. Asked parks the attack behind the hand pick; the pick's answer declares it.
    match crate::attack_summon::replace_attacker(sink, &attacker, &target, false) {
        crate::attack_summon::Replaced::Asked => return Ok(()),
        crate::attack_summon::Replaced::By(unit) => {
            let bounce = crate::scripts::flags_of(sink.state, &attacker).bounce_attacker == Some(true);
            return declare_instead(sink, &unit, &target, Some(&attacker.id), bounce);
        }
        crate::attack_summon::Replaced::No => {}
    }

    declare_instead(sink, &attacker, &target, None, false)
}

/// R1202: everything in `declare_attack` after its exertion block — the declaration, the trap
/// window, the combat — for `attacker`, which a substitute may be. A substitute attacks only when
/// the unit restrictions let it (`attack_restriction`); otherwise nothing happens, the declarer's
/// exertion spent either way. Its declaration carries `bounce_after` and an `exits_from` taken
/// after the summon, and its event carries `instead_of` — the declarer's id — and `forced: false`,
/// so a declared substitute's attack opens its trap window like any declared attack.
pub(crate) fn declare_instead(
    sink: &mut EngineSink<'_>,
    attacker: &CardInstance,
    target: &AttackTarget,
    instead_of: Option<&str>,
    bounce: bool,
) -> CombatResult {
    let attacker = live_or_given(sink.state, attacker);
    let target = live_target_or_given(sink.state, target);
    if instead_of.is_some()
        && crate::restrictions::attack_restriction(sink.state, &attacker, &target).is_err()
    {
        return Ok(());
    }

    let target_id = target_id_of(&target);
    let declared = DeclaredAttack {
        id: format!("d{}", sink.state.next_seq),
        attacker_id: attacker.id.clone(),
        target_id: target_id.clone(),
        cancelled: false,
        // R220: whose attack it is, and the stays it was declared on.
        by: Some(attacker.controller),
        exits_from: Some(crate::stays::exit_mark(sink.state)),
        bounce_after: if bounce { Some(true) } else { None },
    };
    sink.state.next_seq += 1;
    let declared_id = declared.id.clone();
    sink.state.declared_attack = Some(declared);

    let event = GameEvent::AttackDeclared {
        attacker_id: attacker.id.clone(),
        target_id,
        forced: false,
        instead_of: instead_of.map(str::to_string),
    };
    let at = sink.events.len();
    sink.events.push(event.clone());
    withhold_from_frontier(sink, at);
    // The interposer's `summoned` and `redirected` stand before the declaration on the frontier, which
    // the window's own dispatch delivers (R100) — the declaration is the window's alone all the same.
    // Marked unconditionally: on the paths a replacement parks behind a prompt (`attack_summon`)
    // the cursor no longer stands at the declaration when the answer makes it, so the mark is what
    // keeps the frontier from delivering it a second time. Where the cursor did stand there the mark
    // changes nothing — the declaration was already past it.
    let declaration = sink.events[at].clone();
    crate::triggers::mark_dispatched(sink, &[declaration]);

    // Step 4's second sentence: the traps answer the declaration, before any damage.
    crate::traps::run_trap_window(sink, &event);
    let since: Vec<GameEvent> = sink
        .events
        .get(at + 1..)
        .map(<[GameEvent]>::to_vec)
        .unwrap_or_default();
    queue_declaration_triggers(sink, &event, &since);

    if sink.state.result.is_some() {
        // The window ended the game; there is no step 5 and nothing to resume into.
        close_window(sink.state, &declared_id);
        return Ok(());
    }
    if sink.state.pending.is_some() {
        owe_declared_attack(sink, &declared_id);
        return Ok(());
    }

    if !window_answered(sink, &declared_id) {
        return Ok(());
    }
    resolve_declared_attack(sink, &declared_id);
    Ok(())
}

/// A forced attack (Moths to the Flame, Bear Honeypot): §4.2's last paragraph and R53. It skips
/// steps 1 to 3, so position, sickness and the Taunt rule do not apply, and it spends no exertion, so
/// the unit may still take its own attack on its own turn. The target still strikes back, and the
/// combat is followed by its own state check.
///
/// R121 — AND IT OPENS NO TRAP WINDOW. Step 4 is one sentence about spending the attacker's exertion
/// and one about the window that answers the declaration, and a forced attack has neither half: R121
/// says in as many words that it "is declared by the effect, not the player … so it is not 'the
/// opponent declaring an attack'", and §6.3's Cancel-an-attack row is "call off an attack already
/// declared", which is the player's declaration and not a compulsion. So `state.declared_attack` stays
/// null here and nothing can cancel a forced attack — which is also what #96 already says, since its
/// `when` refuses every event carrying `forced`.
///
/// The engineering reading agrees with the rules one. A window here would open inside a card's
/// effect list, where a trap's prompt would split the list §10.3 calls one unit of work, and where
/// `force_attacks_on`'s run would have to become a second resumable sequence — for a window no Core
/// card can fire in. Nothing is lost by leaving it out: the `attackDeclared` event a forced attack
/// emits still reaches the traps through §10.3's immediate check, exactly as it does today, because
/// the withholding above is per declaration rather than per event type.
pub fn force_attack(sink: &mut EngineSink<'_>, attacker: &CardInstance, target: &AttackTarget) {
    if sink.state.result.is_some() {
        return;
    }
    let Some(attacker) = live_card(sink.state, attacker) else {
        return;
    };
    if !is_active_on_field(sink.state, &attacker) {
        return;
    }
    let target = match target {
        DamageTarget::Unit { instance } => {
            let Some(defender) = live_card(sink.state, instance) else {
                return;
            };
            if !is_active_on_field(sink.state, &defender) {
                return;
            }
            DamageTarget::Unit { instance: defender }
        }
        DamageTarget::Hero { player } => DamageTarget::Hero { player: *player },
    };
    // R446: nor a carried Unit's, which neither attacks nor is attacked.
    if carried_out_of_combat(sink.state, &attacker) {
        return;
    }
    if let DamageTarget::Unit { instance } = &target
        && carried_out_of_combat(sink.state, instance)
    {
        return;
    }
    // R173: the compulsion waives position, sickness and Taunt (R53), never whose side the target is
    // on. A target that is not this attacker's enemy — it changed sides mid-run, or had crossed to
    // the attacker's side before the run began — is not attacked, and the attacker is passed over in
    // silence, as R96 passes over one that is gone.
    if !is_enemy_of(&attacker, &target) {
        return;
    }
    // B5 E35: nor the unit restrictions — a forced attack on a target its attacker may not attack does
    // not happen, and is passed over in silence the same way.
    if crate::restrictions::attack_restriction(sink.state, &attacker, &target).is_err() {
        return;
    }

    // R1202: a forced attack Windfast would make, its substitute makes instead.
    let since = crate::stays::exit_mark(sink.state);
    let (attacker, instead_of, bounce) =
        match crate::attack_summon::replace_attacker(sink, &attacker, &target, true) {
            crate::attack_summon::Replaced::Asked => return,
            crate::attack_summon::Replaced::By(unit) => {
                let bounce = crate::scripts::flags_of(sink.state, &attacker).bounce_attacker == Some(true);
                (unit, Some(attacker.id.clone()), bounce)
            }
            crate::attack_summon::Replaced::No => (attacker, None, false),
        };

    // R1203: the Prime's joiners attack first, then its own combat.
    if crate::scripts::flags_of(sink.state, &attacker).attack_joiners == Some(true) {
        match crate::attack_summon::joiners_first(sink, &attacker, &target, since) {
            crate::attack_summon::JoinOutcome::Paused => {
                crate::attack_summon::owe_combat(
                    sink,
                    &attacker.id,
                    &target,
                    true,
                    bounce,
                    None,
                    instead_of.as_deref(),
                    since,
                );
                return;
            }
            crate::attack_summon::JoinOutcome::Fight => {}
        }
    }

    force_attack_instead(sink, &attacker, &target, instead_of.as_deref());
    if bounce {
        crate::attack_summon::bounce_after(sink, &attacker.id, since);
    }
}

/// R1202: a forced attack that skips the replacement — the substitute's own, or any forced attack
/// continued after its pick or its joiners — made as a forced attack with `instead_of` set, then
/// its combat and state check.
pub(crate) fn force_attack_instead(
    sink: &mut EngineSink<'_>,
    attacker: &CardInstance,
    target: &AttackTarget,
    instead_of: Option<&str>,
) {
    sink.events.push(GameEvent::AttackDeclared {
        attacker_id: attacker.id.clone(),
        target_id: target_id_of(target),
        forced: true,
        instead_of: instead_of.map(str::to_string),
    });

    resolve_combat(sink, attacker, target);
    close_combat(sink, attacker, target, true);
}

/// §4.2 step 2: an attack is made on an enemy unit or the enemy hero, forced or not (R173).
fn is_enemy_of(attacker: &CardInstance, target: &AttackTarget) -> bool {
    let enemy = opponent_of(attacker.controller);
    match target {
        DamageTarget::Hero { player } => *player == enemy,
        DamageTarget::Unit { instance } => instance.controller == enemy,
    }
}

/// R53: the named units attack the named target one at a time, in the order given (lane order, as
/// `active_units_of` reports it), each its own combat with its own state check, and the sequence stops
/// as soon as the target is no longer on the field — and R174 has a target that left and came back
/// (a Reborn body) count as gone, since what is standing there now is a new arrival (R83). The same
/// holds for each attacker the run named: one that died in an earlier combat of the run and came back
/// through Reborn before its turn is not the unit the run named, so it is passed over in silence like
/// any attacker that is gone (R96). (TS `since = exitMark(sink.state)`: `None` is the mark now.)
pub fn force_attacks_on(
    sink: &mut EngineSink<'_>,
    attackers: &[CardInstance],
    target: &AttackTarget,
    since: Option<u32>,
) {
    let since = since.unwrap_or_else(|| crate::stays::exit_mark(sink.state));
    for (at, attacker) in attackers.iter().enumerate() {
        if sink.state.result.is_some() {
            return;
        }
        if let DamageTarget::Unit { instance } = target {
            if !active_now(sink.state, &instance.id) {
                return;
            }
            if crate::stays::left_field_after(sink.state, since, &instance.id) {
                return;
            }
        }
        // R53, R113: the check after the last combat collected a unit whose Death asks something, and
        // that check belongs to that combat (R59). The run waits for the answer rather than walking on
        // over the prompt: the attackers still to come are owed, and the answer's drain brings them.
        if is_paused(sink) {
            owe_forced_run(sink, &attackers[at..], target, since);
            return;
        }
        if crate::stays::left_field_after(sink.state, since, &attacker.id) {
            continue;
        }
        force_attack(sink, attacker, target);
    }
}

/// R113: the `resume.hook` of a forced run a Death hook's prompt stopped between two combats: the
/// attackers it has not reached and the target, by id, since R44's AI turn can replace every
/// instance before the answer (the same reason `ATTACK_WINDOW_WORK` carries ids).
pub const FORCED_RUN_WORK: &str = "@forcedRun";

/// Park the rest of a run at the pause, with the mark it began at, so the answer's own departures
/// count against the stays the run named too (R174). (TS `OwedForcedRun = { attackers; target:
/// { unit } | { hero }; since? }`, written as that JSON.)
fn owe_forced_run(sink: &mut EngineSink<'_>, rest: &[CardInstance], target: &AttackTarget, since: u32) {
    if sink.state.result.is_some() {
        return;
    }
    if let DamageTarget::Unit { instance } = target
        && crate::stays::left_field_after(sink.state, since, &instance.id)
    {
        return;
    }
    let attackers: Vec<String> = rest
        .iter()
        .filter(|unit| !crate::stays::left_field_after(sink.state, since, &unit.id))
        .map(|unit| unit.id.clone())
        .collect();
    if attackers.is_empty() {
        return;
    }
    let owed_target = match target {
        DamageTarget::Unit { instance } => json!({ "unit": instance.id }),
        DamageTarget::Hero { player } => json!({ "hero": player }),
    };
    let owed = json!({ "attackers": attackers, "target": owed_target, "since": since });
    let mut data = IndexMap::new();
    data.insert("run".to_string(), owed);
    owe(sink, engine_resume(FORCED_RUN_WORK, "run", data));
}

/// `work.rs`'s handler for a forced run a prompt stopped: the same run, where it stopped (R53).
pub fn run_owed_forced_run(sink: &mut EngineSink<'_>, item: &WorkItem) {
    let Some(owed) = item.resume.data.get("run").and_then(Value::as_object) else {
        return;
    };
    let Some(ids) = owed.get("attackers").and_then(Value::as_array) else {
        return;
    };
    let Some(owed_target) = owed.get("target").and_then(Value::as_object) else {
        return;
    };
    let target = if let Some(unit) = owed_target.get("unit") {
        let Some(instance) = unit.as_str().and_then(|id| find_instance(sink.state, id)) else {
            return;
        };
        DamageTarget::Unit {
            instance: instance.clone(),
        }
    } else {
        let Some(player) = owed_target
            .get("hero")
            .and_then(|hero| serde_json::from_value::<PlayerId>(hero.clone()).ok())
        else {
            return;
        };
        DamageTarget::Hero { player }
    };
    let attackers: Vec<CardInstance> = ids
        .iter()
        .filter_map(|id| id.as_str().and_then(|id| find_instance(sink.state, id)).cloned())
        .collect();
    let since = owed
        .get("since")
        .and_then(Value::as_u64)
        .map(|since| since as u32);
    force_attacks_on(sink, &attackers, &target, since);
}

// ---------------------------------------------------------------------------------------------
// B5 E35: forced attacks on "a random enemy" and on the unit's own hero
// ---------------------------------------------------------------------------------------------

/// B5 E35: what a forced attack on "a random enemy" draws from — the targets its attacker may attack,
/// enemy units in lane order and then (for "an enemy", not "an enemy Unit") the enemy hero. R53 waives
/// position, sickness and Taunt, so none of those narrows the list; the unit restrictions do, and so
/// does a carrier (R446): a carried Unit neither attacks nor is drawn, so no roll is spent on it.
pub fn random_attack_targets(
    state: &GameState,
    attacker: &CardInstance,
    among: AttackAmong,
) -> Vec<AttackTarget> {
    if carried_out_of_combat(state, attacker) {
        return Vec::new();
    }
    let enemy = opponent_of(attacker.controller);
    let mut candidates: Vec<AttackTarget> = crate::zones::active_units_of(state, enemy)
        .into_iter()
        .filter(|instance| !carried_out_of_combat(state, instance))
        .cloned()
        .map(|instance| DamageTarget::Unit { instance })
        .collect();
    if among == AttackAmong::Enemies {
        candidates.push(DamageTarget::Hero { player: enemy });
    }
    candidates
        .into_iter()
        .filter(|target| crate::restrictions::attack_restriction(state, attacker, target).is_ok())
        .collect()
}

/// B5 E35: the attacker makes `times` forced attacks (R53), each on a target drawn from the match rng
/// among the ones it may attack as that attack begins (Classic #78's "attacks a random enemy", twice on
/// its Radiant face; Classic+ #19.2's "a random enemy Unit"). Each is its own combat with its own state
/// check; the run stops once the attacker has left the field on the stay it began on, or has nothing
/// to attack. A Death between two of them that asks owes the rest (`FORCED_RANDOM_WORK`, R113).
/// (TS `since = exitMark(sink.state)`: `None` is the mark now.)
pub fn force_attacks_random(
    sink: &mut EngineSink<'_>,
    attacker: &CardInstance,
    among: AttackAmong,
    times: i32,
    since: Option<u32>,
) {
    let since = since.unwrap_or_else(|| crate::stays::exit_mark(sink.state));
    let mut made = 0;
    while made < times {
        if sink.state.result.is_some() {
            return;
        }
        let Some(live) = live_card(sink.state, attacker) else {
            return;
        };
        if !is_active_on_field(sink.state, &live)
            || crate::stays::left_field_after(sink.state, since, &live.id)
        {
            return;
        }
        if is_paused(sink) {
            owe_forced_random(sink, &live.id, among, times - made, since);
            return;
        }
        let targets = random_attack_targets(sink.state, &live, among);
        let Some(target) = sink.rng.pick(&targets).cloned() else {
            return;
        };
        force_attack(sink, &live, &target);
        made += 1;
    }
}

/// R113: the `resume.hook` of a random forced run a Death's question stopped between two attacks.
pub const FORCED_RANDOM_WORK: &str = "@forcedRandom";

/// (TS `OwedForcedRandom = { attacker; among; left; since }`, written as that JSON.)
fn owe_forced_random(sink: &mut EngineSink<'_>, attacker: &str, among: AttackAmong, left: i32, since: u32) {
    if left <= 0 || sink.state.result.is_some() {
        return;
    }
    let owed = json!({ "attacker": attacker, "among": among, "left": left, "since": since });
    let mut data = IndexMap::new();
    data.insert("run".to_string(), owed);
    owe(sink, engine_resume(FORCED_RANDOM_WORK, "run", data));
}

/// `work.rs`'s handler for a random forced run a prompt stopped: the same run, where it stopped.
pub fn run_owed_forced_random(sink: &mut EngineSink<'_>, item: &WorkItem) {
    let Some(owed) = item.resume.data.get("run").and_then(Value::as_object) else {
        return;
    };
    let Some(attacker) = owed.get("attacker").and_then(Value::as_str) else {
        return;
    };
    let Some(left) = owed.get("left").and_then(Value::as_i64) else {
        return;
    };
    let Some(since) = owed.get("since").and_then(Value::as_u64) else {
        return;
    };
    let among = match owed.get("among").and_then(Value::as_str) {
        Some("enemies") => AttackAmong::Enemies,
        Some("enemyUnits") => AttackAmong::EnemyUnits,
        _ => return,
    };
    let Some(attacker) = find_instance(sink.state, attacker).cloned() else {
        return;
    };
    force_attacks_random(sink, &attacker, among, left as i32, Some(since as u32));
}

/// B5 E35: a forced attack on the unit's OWN hero (Classic+ #19.5 Bot Loser while Berserk). R53's
/// compulsion, with the one change R173 never allows elsewhere: the target is its controller's hero.
/// A hero never strikes back (§4.3), so the unit deals its attack to its own hero, and the combat has
/// its own state check. The unit restrictions still hold: one that cannot attack does not.
pub fn force_attack_own_hero(sink: &mut EngineSink<'_>, attacker: &CardInstance) {
    if sink.state.result.is_some() {
        return;
    }
    let Some(attacker) = live_card(sink.state, attacker) else {
        return;
    };
    if !is_active_on_field(sink.state, &attacker) {
        return;
    }
    let target: AttackTarget = DamageTarget::Hero {
        player: attacker.controller,
    };
    if crate::restrictions::attack_restriction(sink.state, &attacker, &target).is_err() {
        return;
    }
    sink.events.push(GameEvent::AttackDeclared {
        attacker_id: attacker.id.clone(),
        target_id: target_id_of(&target),
        forced: true,
        instead_of: None,
    });
    resolve_combat(sink, &attacker, &target);
    close_combat(sink, &attacker, &target, true);
}

// ---------------------------------------------------------------------------------------------
// "After this attacks": the attacker's `afterAttack` hook, once the combat's state check has closed
// ---------------------------------------------------------------------------------------------

/// R113: the `resume.hook` of the engine sequence this section parks — an attacker's `afterAttack`
/// hook owed behind the state check that closes its combat (a Death there asked something), or the
/// rest of that hook after a question of its own, with the check that follows it.
pub const AFTER_ATTACK_WORK: &str = "@afterAttack";

/// The facts a combat hands its attacker's `afterAttack` hook, in the hook's `ctx.data`
/// (`after_attack_of` reads them back): the attack's target (a unit's id or `hero-<player>`), the units
/// that combat destroyed — the ones whose lethal hit was the attacker's (R42's killer: the attacked
/// unit, and Cleave's kills) — whether the attacker is still on the field on the stay it attacked from
/// once the check has closed, and whether the attack was forced (R53).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AfterAttackFacts {
    pub target_id: String,
    pub destroyed_ids: Vec<String>,
    pub survived: bool,
    pub forced: bool,
}

/// The combat facts an `afterAttack` hook was handed, or `None` outside one.
pub fn after_attack_of(ctx: &crate::script::EffectContext<'_>) -> Option<AfterAttackFacts> {
    let data = &ctx.data;
    let target_id = data.get("targetId")?.as_str()?.to_string();
    let destroyed_ids = data.get("destroyedIds")?.as_array()?;
    let survived = data.get("survived")?.as_bool()?;
    let forced = data.get("forced")?.as_bool()?;
    Some(AfterAttackFacts {
        target_id,
        destroyed_ids: destroyed_ids
            .iter()
            .filter_map(|id| id.as_str().map(str::to_string))
            .collect(),
        survived,
        forced,
    })
}

/// What an owed `afterAttack` carries, all JSON: the combat's facts and the attacker as it fought.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct OwedAfterAttack {
    attacker_id: String,
    /// The attacker just before the check that closed its combat (R78, R89): its self if it died.
    snapshot: CardInstance,
    target_id: String,
    destroyed_ids: Vec<String>,
    forced: bool,
    /// The field's departures before that check, to tell a survivor from a Reborn body (R174).
    since: u32,
    /// Judged once, as the hook first runs, when the check has closed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    survived: Option<bool>,
    /// The hook is done and only the state check that follows it is owed. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    check_only: Option<bool>,
}

const AFTER_ATTACK_KEY: &str = "attack";

/// The state check that closes a combat (§4.2 step 5, §4.3 step 3, R53), then the attacker's
/// `afterAttack` hook (Classic #13, Classic+ #73.1, Core #32) — also when the attacker died in it, on
/// the snapshot it fought with, as a Death hook reads its card (R78, R89). An attack that was called
/// off (R44, R220) never fought and never reaches here. The hook is a whole effect, so a check follows
/// it (R59); a question inside it, or a Death's question in the check before it, owes the rest to
/// `state.work` (R113).
fn close_combat(sink: &mut EngineSink<'_>, attacker: &CardInstance, target: &AttackTarget, forced: bool) {
    let snapshot = live_or_given(sink.state, attacker);
    let since = crate::stays::exit_mark(sink.state);
    let from = sink.events.len();
    crate::state_check::state_check(sink);
    if crate::scripts::script_of(sink.state, &snapshot)
        .after_attack
        .is_none()
        || sink.state.result.is_some()
    {
        return;
    }
    let destroyed_ids: Vec<String> = sink.events[from.min(sink.events.len())..]
        .iter()
        .filter_map(|event| match event {
            GameEvent::Destroyed {
                instance_id,
                killer_id: Some(killer),
                ..
            } if *killer == snapshot.id => Some(instance_id.clone()),
            _ => None,
        })
        .collect();
    let owed = OwedAfterAttack {
        attacker_id: snapshot.id.clone(),
        target_id: target_id_of(target),
        snapshot,
        destroyed_ids,
        forced,
        since,
        survived: None,
        check_only: None,
    };
    if is_paused(sink) {
        owe_after_attack(sink, &owed);
        return;
    }
    run_after_attack(sink, &owed, None);
}

fn owe_after_attack(sink: &mut EngineSink<'_>, owed: &OwedAfterAttack) {
    let mut data = IndexMap::new();
    data.insert(AFTER_ATTACK_KEY.to_string(), to_json(owed));
    owe(sink, engine_resume(AFTER_ATTACK_WORK, "hook", data));
}

/// The attacker's hook, from `paused` when a question split it, then the check that follows it.
fn run_after_attack(sink: &mut EngineSink<'_>, owed: &OwedAfterAttack, paused: Option<PausedStep>) {
    let Some(hook) = crate::scripts::script_of(sink.state, &owed.snapshot)
        .after_attack
        .clone()
    else {
        return;
    };
    if sink.state.result.is_some() {
        return;
    }
    let live = find_instance(sink.state, &owed.attacker_id).cloned();
    let survived = owed.survived.unwrap_or_else(|| {
        live.as_ref().is_some_and(|live| {
            is_active_on_field(sink.state, live)
                && !crate::stays::left_field_after(sink.state, owed.since, &live.id)
        })
    });
    let judged = OwedAfterAttack {
        survived: Some(survived),
        ..owed.clone()
    };
    // A survivor is itself, on the field (Classic+ #73.1 transforms it); one that died is the snapshot
    // it fought with, which a continuation reads back too (`prompts::SELF_KEY`, R89).
    let self_card = match (&live, survived) {
        (Some(live), true) => live.clone(),
        _ => owed.snapshot.clone(),
    };
    // `{ ...facts, ...(survived ? {} : { [SELF_KEY]: snapshot }) }`, in that key order.
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert("targetId".to_string(), json!(owed.target_id));
    data.insert("destroyedIds".to_string(), json!(owed.destroyed_ids));
    data.insert("survived".to_string(), json!(survived));
    data.insert("forced".to_string(), json!(owed.forced));
    if !survived {
        data.insert(crate::prompts::SELF_KEY.to_string(), to_json(&owed.snapshot));
    }
    let mut plan_data = IndexMap::new();
    plan_data.insert(AFTER_ATTACK_KEY.to_string(), to_json(&judged));
    let plan = WorkPlan {
        def_id: String::new(),
        hook: AFTER_ATTACK_WORK.to_string(),
        step: "hook".to_string(),
        radiant: false,
        instance_id: None,
        data: plan_data,
        owner: self_card.controller,
    };
    let status = {
        let mut ctx = crate::resolve::make_context(
            sink,
            Some(&self_card),
            crate::resolve::HookOptions {
                // R426: the player who controlled it in that combat, though a Death in the check took it since.
                controller: Some(owed.snapshot.controller),
                data: Some(data),
                ..crate::resolve::HookOptions::default()
            },
        );
        if let Some(paused) = &paused {
            if let Some(exits_from) = paused.exits_from {
                ctx.exits_from = Some(exits_from);
            }
            if let Some(summoned) = &paused.summoned {
                ctx.summoned = Some(summoned.clone());
            }
        }
        let effects = hook(&mut ctx);
        crate::prompts::run_resumable_list(&mut ctx, &plan, effects, paused)
    };
    match status {
        crate::prompts::ListStatus::Done => crate::state_check::state_check(sink),
        // The hook's last effect asked: the hook is done, and the check after it waits for the answer.
        crate::prompts::ListStatus::Asked => owe_after_attack(
            sink,
            &OwedAfterAttack {
                check_only: Some(true),
                ..judged
            },
        ),
        _ => {}
    }
}

fn owed_after_attack_of(data: &IndexMap<String, Value>) -> Option<OwedAfterAttack> {
    let raw = data.get(AFTER_ATTACK_KEY)?;
    let owed = raw.as_object()?;
    owed.get("attackerId")?.as_str()?;
    owed.get("targetId")?.as_str()?;
    owed.get("since")?.as_u64()?;
    owed.get("snapshot")?;
    owed.get("destroyedIds")?.as_array()?;
    owed.get("forced")?.as_bool()?;
    serde_json::from_value::<OwedAfterAttack>(raw.clone()).ok()
}

/// `work.rs`'s handler: the hook (or its rest), then the check, where the pause left them (R113).
pub fn run_owed_after_attack(sink: &mut EngineSink<'_>, item: &WorkItem) {
    let Some(owed) = owed_after_attack_of(&item.resume.data) else {
        return;
    };
    if owed.check_only == Some(true) {
        crate::state_check::state_check(sink);
        return;
    }
    run_after_attack(sink, &owed, paused_of(&item.resume.data));
}
