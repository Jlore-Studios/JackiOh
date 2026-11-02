//! Positions, exertion, attack validation and combat resolution (SPEC §4.1, §4.2, §4.3; BUILD M2-T1,
//! M2-T2, M2-T4). The damage pipeline of §4.4 lives in `damage.rs` and the state check of §4.5 in
//! `state_check.rs`: this module orders them, it never re-implements them.
//!
//! Nothing here is random or asks the player directly; every refusal is a string the action layer hands
//! back untouched (CLAUDE.md rule 4). §4.2 step 4's trap window can prompt, and a prompt is state
//! (§9.3), so `declare_attack` is resumable: the declaration sits in `state.declared_attack` and, if the
//! window pauses, step 5's combat is owed to `state.work` under `ATTACK_WINDOW_WORK` (R113, R117). The
//! resume holds only ids, since the window can replace every instance (R44's AI turn drives `reduce`,
//! which clones), and a card argument is re-read by id as the state stands now (`find_instance`).
//! `work.rs` calls the `run_owed_*` handlers and `traps.rs` `declared_attack_stands` directly (SURFACE §6.6).

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::{LANE_RESTRICTED_ATTACKS, WINDFURY_ATTACKS};
use crate::damage::{DamageArgs, DamageFlags, DamageTarget, deal_damage};
use crate::layers::AuraTable;
use crate::script::{AttackMod, AttackModArgs, EngineSink, TargetedWhat};
use crate::state::{
    CardInstance, DeclaredAttack, EngineError, Exertion, GameState, Position, Resume, WorkItem,
    find_instance, find_instance_mut,
};
use crate::wire::{GameEvent, Keyword, KeywordKind, PLAYER_IDS, PlayerId, has_keyword, opponent_of};
use crate::work::{PausedStep, WorkPlan, owe, paused as is_paused, paused_of};
use crate::zones::live_or;

/// What an attack can be declared on (§4.2 step 2), the damage pipeline's own target type.
pub type AttackTarget = DamageTarget;

/// The two halves of a unit's turn (§4.1). A unit has one of them; Deft Duelist has both (R49).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum ExertionKind {
    Attack,
    Switch,
}

/// Every entry point returns the §4.2 refusal reason, or nothing when it went through (SURFACE §4.4.9).
pub type CombatResult = Result<(), EngineError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct SwitchPositionOptions {
    /// False for a switch that is an effect rather than the player's action (R20).
    pub spend_exertion: Option<bool>,
    /// Where to put the unit; by default it flips.
    pub to: Option<Position>,
}

/// B5 E35: what a forced attack on "a random enemy" draws from.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub enum AttackAmong {
    Enemies,
    EnemyUnits,
}

// Live reads and private copies of `scripts.rs`'s readers (fullsend rule 5).

/// The card as it stands in the state now, or `None` once it is nowhere (it ceased to exist).
fn live_card(state: &GameState, card: &CardInstance) -> Option<CardInstance> {
    find_instance(state, &card.id).cloned()
}

fn live_target_or_given(state: &GameState, target: &AttackTarget) -> AttackTarget {
    match target {
        DamageTarget::Unit { instance } => DamageTarget::Unit {
            instance: live_or(state, instance),
        },
        DamageTarget::Hero { player } => DamageTarget::Hero { player: *player },
    }
}

fn active_now(state: &GameState, id: &str) -> bool {
    find_instance(state, id).is_some_and(|card| is_active_on_field(state, card))
}

fn to_json<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).unwrap_or(Value::Null)
}

fn combat_flags() -> Option<DamageFlags> {
    Some(DamageFlags {
        combat: Some(true),
        ..DamageFlags::default()
    })
}

/// The flags of the attacker's own hit, and of its Cleave's: `{ combat: true }` and `attacking`, the
/// damage a Unit's Trample applies to and no strike back's (R1750000), plus `poisonous` when an
/// attack modifier grants it (MD-D4, R1120).
fn attack_flags(poisonous: bool) -> Option<DamageFlags> {
    Some(DamageFlags {
        combat: Some(true),
        poisonous: if poisonous { Some(true) } else { None },
        attacking: Some(true),
        ..DamageFlags::default()
    })
}

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

/// R49: a unit with Deft has two exertions, one attack and one switch, where every other unit has one,
/// read through §10.4's layers so a granted Deft counts the same as a printed one.
fn has_two_exertions(auras: &AuraTable<'_>, unit: &CardInstance) -> bool {
    auras.unit_has(unit, KeywordKind::Deft)
}

/// R636: how many attacks the unit may declare in a turn — two with Windfury, read through the layers
/// (§10.4) so a keyword granted or lost mid-turn counts from that moment, one otherwise.
pub fn attacks_per_turn(state: &GameState, unit: &CardInstance) -> i32 {
    attacks_per_turn_in(&AuraTable::per_read(state), unit)
}

/// `attacks_per_turn` read through a sweep's `AuraTable` (#892).
pub fn attacks_per_turn_in(auras: &AuraTable<'_>, unit: &CardInstance) -> i32 {
    if auras.unit_has(unit, KeywordKind::Windfury) {
        WINDFURY_ATTACKS
    } else {
        1
    }
}

/// §4.1: one exertion per turn, so a unit that attacked cannot switch and one that switched cannot
/// attack (R6); Deft spends the two independently (R49). R636: Windfury attacks again up to
/// `attacks_per_turn`, the first attack already spending the exertion a switch needs.
pub fn has_exertion(state: &GameState, unit: &CardInstance, kind: ExertionKind) -> bool {
    has_exertion_in(&AuraTable::per_read(state), unit, kind)
}

/// `has_exertion` read through a sweep's `AuraTable` (#892). R1174000: a Frozen Unit has neither
/// half, whatever it spent (`freeze.rs`); R1839000: nor has a Dormant card, until it awakens.
pub fn has_exertion_in(auras: &AuraTable<'_>, unit: &CardInstance, kind: ExertionKind) -> bool {
    unit.frozen.is_none() && unit.slumber.is_none() && exertion_unspent_in(auras, unit, kind)
}

/// R1174000: the half of the unit's exertion it has not spent this turn, Frozen or not — what a
/// freeze landing now takes away (`freeze::thaw_turn`). Every other reader asks `has_exertion`.
pub fn exertion_unspent(state: &GameState, unit: &CardInstance, kind: ExertionKind) -> bool {
    exertion_unspent_in(&AuraTable::per_read(state), unit, kind)
}

pub fn exertion_unspent_in(auras: &AuraTable<'_>, unit: &CardInstance, kind: ExertionKind) -> bool {
    let spent = &unit.exertion;
    if kind == ExertionKind::Attack {
        let attacks = spent.attacks.unwrap_or(if spent.attacked { 1 } else { 0 });
        if attacks >= attacks_per_turn_in(auras, unit) {
            return false;
        }
        return has_two_exertions(auras, unit) || !spent.switched;
    }
    if has_two_exertions(auras, unit) {
        return !spent.switched;
    }
    !spent.attacked && !spent.switched
}

/// §4.1: a unit that entered the field this turn is summoning sick: a Reborn body (R83), a unit whose
/// controller changed (R171, `enter_new_side`) and a card a Stack pile uncovers (R1829000,
/// `zones::remove_from_field`) alike, so this is the one comparison.
pub fn is_sick(state: &GameState, unit: &CardInstance) -> bool {
    unit.summoned_turn == Some(state.turn)
}

/// §4.1, §6.1: summoning sick with neither Rush nor Charge — §4.2 step 1's sickness refusal, and
/// `UnitView.sick` (R849000). `keywords` are the unit's viewed keywords (`layers::unit_view`).
pub fn is_asleep(state: &GameState, unit: &CardInstance, keywords: &[Keyword]) -> bool {
    is_sick(state, unit)
        && !has_keyword(keywords, KeywordKind::Rush)
        && !has_keyword(keywords, KeywordKind::Charge)
}

/// R171: a card whose controller changes has entered its new controller's side on this turn (§4.1, for
/// `is_sick`), with a fresh exertion. Every path that changes control on the field calls this, for every
/// card that changes sides (dormant and backrow included) and for nothing else. A player modifier a
/// permanent installed and still owns (#79 Twinspell, `sourceId`) is its lasting effect (§5.1) whose
/// "you" is the controller (§8), so it follows the card from `from` to its new side.
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
    // R1174000: a Frozen card stays Frozen, its thaw stamped again for its new controller.
    crate::freeze::restamp(sink.state, &card.id);
    crate::modifiers::move_sourced_modifiers(sink, &card.id, from, controller);
}

/// §3.2: the card that acts in a zone is the top of its Stack pile. A card dormant underneath one is
/// not on the field for anything: it neither acts nor can be targeted (R13).
pub fn is_active_on_field(state: &GameState, unit: &CardInstance) -> bool {
    // R446: a Unit a carrier holds acts from its backrow zone, with the carrier acting beneath it.
    crate::zones::acts_on_field(state, unit)
}

/// R446: a Unit standing on a carrier (Classic+ #33 Ivory Tower) can neither attack nor be attacked,
/// declared or forced (R53 never waives this), and so binds no attacker with its Taunt (§4.2 step 3).
fn carried_out_of_combat(state: &GameState, unit: &CardInstance) -> bool {
    crate::zones::is_carried(state, unit)
}

/// §4.1: the position the player's own switch puts a unit in (a unit with none is in Attack Position).
pub fn position_after_switch(unit: &CardInstance) -> Position {
    if unit.position.unwrap_or(Position::Atk) == Position::Atk {
        Position::Def
    } else {
        Position::Atk
    }
}

/// §4.1, R49, R1174000: why the unit could not switch to `to`, or `Ok` when it can. `switch_position`
/// refuses with it and `legal_actions` lists a `switchPosition` by it (`reduce::can_switch`), so the two
/// never disagree. `spend_exertion` is false for an effect's switch (R20), which spends nothing.
pub fn why_cannot_switch(
    state: &GameState,
    unit: &CardInstance,
    to: Position,
    spend_exertion: bool,
) -> CombatResult {
    why_cannot_switch_in(&AuraTable::per_read(state), unit, to, spend_exertion)
}

pub fn why_cannot_switch_in(
    auras: &AuraTable<'_>,
    unit: &CardInstance,
    to: Position,
    spend_exertion: bool,
) -> CombatResult {
    // R1839000: a Dormant card switches by nothing, its own action or an effect.
    if crate::slumber::is_slumbering(unit) {
        return Err(EngineError::new(crate::restrictions::SLUMBER_REFUSAL));
    }
    // R1174000: a Frozen Unit's own switch is refused; an effect's (R20) is not.
    if spend_exertion && unit.frozen.is_some() {
        return Err(EngineError::new(FROZEN_REFUSAL));
    }
    if spend_exertion && !has_exertion_in(auras, unit, ExertionKind::Switch) {
        return Err(EngineError::new("that unit has already acted this turn"));
    }
    // §4.1: a Unit with Can't Defend cannot be switched to Defense Position, by an action or by an
    // effect (R2657000; `never_defense` is the flag it took over from, as #65.1 Spikey Pillow printed it),
    // nor can any Unit while an aura says Units cannot be (R1849006, Stinkyscape #23 Warp Disruptor), nor
    // an Indestructible one, whatever made it so (R2179000).
    if to == Position::Def
        && (crate::scripts::flags_of(auras.state(), unit).never_defense == Some(true)
            || auras.unit_has(unit, KeywordKind::CantDefend)
            || crate::restrictions::defense_barred(auras.state())
            || auras.unit_has(unit, KeywordKind::Indestructible))
    {
        return Err(EngineError::new("that unit cannot be in Defense Position"));
    }
    Ok(())
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
    // R1839000: told it is Dormant ahead of "not on the field", which it is (it fills its zone).
    if crate::slumber::is_slumbering(&unit) {
        return Err(EngineError::new(crate::restrictions::SLUMBER_REFUSAL));
    }
    if !is_active_on_field(sink.state, &unit) {
        return Err(EngineError::new("that unit is not on the field"));
    }

    let from = unit.position.unwrap_or(Position::Atk);
    let to = options.to.unwrap_or_else(|| position_after_switch(&unit));

    // R91: asking for the position the unit already holds "does nothing: no error, no event and no
    // exertion spent", read before the exertion because there is no switch to refuse.
    if to == from {
        return Ok(());
    }

    why_cannot_switch(sink.state, &unit, to, spend_exertion)?;

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

/// R1174000: why a Frozen Unit may neither declare an attack nor switch its own position, nor (R2246000)
/// use an Activate.
pub const FROZEN_REFUSAL: &str = "that Unit is Frozen";

/// §4.2 step 1 in the step's order (exertion unspent, Attack Position, not sick unless Rush or Charge,
/// attack above 0, no "Can't attack"), after §3.2's "on the field": the order decides which reason a
/// unit failing several ways reports, so R6's spent exertion is read before its position.
pub fn why_cannot_declare(state: &GameState, attacker: &CardInstance) -> Result<(), EngineError> {
    why_cannot_declare_in(&AuraTable::per_read(state), attacker)
}

pub fn why_cannot_declare_in(auras: &AuraTable<'_>, attacker: &CardInstance) -> Result<(), EngineError> {
    let state = auras.state();
    // R1839000: a Dormant card is on the field and attacks nothing until it awakens.
    if crate::slumber::is_slumbering(attacker) {
        return Err(EngineError::new(crate::restrictions::SLUMBER_REFUSAL));
    }
    if !is_active_on_field(state, attacker) {
        return Err(EngineError::new("that unit is not on the field"));
    }
    if carried_out_of_combat(state, attacker) {
        return Err(EngineError::new("a Unit on a carrier cannot attack"));
    }
    // R1174000: a Frozen Unit has lost its attack, which it is told ahead of what it spent.
    if attacker.frozen.is_some() {
        return Err(EngineError::new(FROZEN_REFUSAL));
    }
    if !has_exertion_in(auras, attacker, ExertionKind::Attack) {
        return Err(EngineError::new("that unit has already acted this turn"));
    }

    let view = auras.unit_view(attacker);
    if view.position != Position::Atk {
        return Err(EngineError::new("only Attack-Position units may attack"));
    }
    if is_asleep(state, attacker, &view.keywords) {
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

/// Every enemy unit whose Taunt forces the target, printed, granted or from Defense (§4.2 step 3),
/// among the units this attacker may attack at all (B5 E35): a Taunt only its own lane may attack
/// (Classic+ #19.1) leaves an attacker in another lane free, and one a carrier holds (R446) binds nobody.
fn taunt_wall(auras: &AuraTable<'_>, attacker: &CardInstance, enemy: PlayerId) -> Vec<CardInstance> {
    let state = auras.state();
    crate::zones::active_units_of(state, enemy)
        .into_iter()
        .filter(|unit| {
            !carried_out_of_combat(state, unit)
                && has_keyword(&auras.unit_view(unit).keywords, KeywordKind::Taunt)
                && crate::restrictions::attack_restriction_in(
                    auras,
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

/// Steps 1 to 3 of §4.2, as the reason the attack is refused, or `Ok` when it is legal: the `attack`
/// action's error and `attack_targets`'s filter, so the two can never disagree.
pub fn why_cannot_attack(
    state: &GameState,
    attacker: &CardInstance,
    target: &AttackTarget,
) -> Result<(), EngineError> {
    why_cannot_attack_in(&AuraTable::per_read(state), attacker, target)
}

pub fn why_cannot_attack_in(
    auras: &AuraTable<'_>,
    attacker: &CardInstance,
    target: &AttackTarget,
) -> Result<(), EngineError> {
    let state = auras.state();
    why_cannot_declare_in(auras, attacker)?;

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
            // R1839000: nor is a Dormant card attacked, though it fills its zone.
            if crate::slumber::is_slumbering(instance) {
                return Err(EngineError::new(crate::restrictions::SLUMBER_REFUSAL));
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
    // B5 E35: the unit restrictions, which a forced attack obeys as well (`force_attack`).
    crate::restrictions::attack_restriction_in(auras, attacker, target)?;

    // §6.1: Rush lifts sickness for unit targets only, Charge for units and the hero. Step 1 has
    // already established that a sick attacker has one of the two.
    if matches!(target, DamageTarget::Hero { .. })
        && is_sick(state, attacker)
        && !auras.unit_has(attacker, KeywordKind::Charge)
    {
        return Err(EngineError::new("Rush cannot hit the hero on its summon turn"));
    }

    // Step 3: while any enemy unit this attacker may attack has Taunt, the target must be one of them.
    let wall = taunt_wall(auras, attacker, enemy);
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
    can_attack_in(&AuraTable::per_read(state), attacker, target)
}

pub fn can_attack_in(auras: &AuraTable<'_>, attacker: &CardInstance, target: &AttackTarget) -> bool {
    why_cannot_attack_in(auras, attacker, target).is_ok()
}

/// Every target this attacker may legally declare on, enemy units in lane order and then the enemy
/// hero (§4.2), which the action layer and the AI policy read rather than enumerate attacks themselves.
pub fn attack_targets(state: &GameState, attacker: &CardInstance) -> Vec<AttackTarget> {
    attack_targets_in(&AuraTable::per_read(state), attacker)
}

pub fn attack_targets_in(auras: &AuraTable<'_>, attacker: &CardInstance) -> Vec<AttackTarget> {
    let enemy = opponent_of(attacker.controller);
    let mut candidates: Vec<AttackTarget> = crate::zones::active_units_of(auras.state(), enemy)
        .into_iter()
        .cloned()
        .map(|instance| DamageTarget::Unit { instance })
        .collect();
    candidates.push(DamageTarget::Hero { player: enemy });
    candidates
        .into_iter()
        .filter(|target| can_attack_in(auras, attacker, target))
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
/// names (#96 My Pawn reads the event, §4.2 step 5 the state). The `hero-<player>` spelling is this
/// module's, so nobody else should be parsing it.
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
    // MD-D31, R1124: marked for exile falls with the destroyed, though no death collects it.
    view.health <= 0 || unit.marked_destroyed == Some(true) || unit.marked_exiled == Some(true)
}

fn has_fallen_now(state: &GameState, id: &str) -> bool {
    match find_instance(state, id) {
        Some(card) => has_fallen(state, card),
        None => true,
    }
}

/// MD-D4, R1120: what the board's combat-only attack modifiers add to this attacker's strike on
/// this defender. Every acting card's `attack_mods` hook is summed — attacks added, Poisonous or-ed
/// — over §10.4's aura sources, read with the source's own face, as `aura_mods` does.
pub fn attack_mod_for(state: &GameState, attacker: &CardInstance, defender: &CardInstance) -> AttackMod {
    let mut out = AttackMod::default();
    for source in crate::layers::aura_sources(state) {
        if source.vanilla {
            continue;
        }
        let Some(hook) = crate::scripts::script_of(state, source).attack_mods.clone() else {
            continue;
        };
        let got = hook(AttackModArgs {
            state,
            self_: source,
            radiant: source.radiant,
            attacker,
            defender,
        });
        out.attack += got.attack;
        out.poisonous = out.poisonous || got.poisonous;
    }
    out
}

/// MD-F14, R1283: multiplier for attacks across its lane in combat.
pub fn lane_multiplier_for(state: &GameState, striker: &CardInstance, struck: &CardInstance) -> i32 {
    let Some(printed) = crate::scripts::flags_of(state, striker).lane_multiplier else {
        return 1;
    };
    let Some(striker_slot) = crate::zones::slot_of(state, striker) else {
        return 1;
    };
    let Some(struck_slot) = crate::zones::slot_of(state, struck) else {
        return 1;
    };
    // R1081000: a Unit standing in a backrow zone is in that zone's lane, as one in its unit zone is.
    if crate::zones::in_units_row_or_standing(state, striker)
        && crate::zones::in_units_row_or_standing(state, struck)
        && striker_slot.player != struck_slot.player
        && striker_slot.lane == struck_slot.lane
    {
        crate::params::declared_or(state, striker, "multiplier", printed)
    } else {
        1
    }
}

/// §4.4 step 10: Cleave deals the attacker's attack to each unit adjacent to the target as separate
/// instances. It belongs to the attack, not the hit, so it lands even when Divine Shield,
/// Indestructible or the zero rule stopped the hit (R63); adjacency never crosses sides (§3.1).
fn cleave(
    sink: &mut EngineSink<'_>,
    attacker: &CardInstance,
    target: &AttackTarget,
    amount: i32,
    poisonous: bool,
) {
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
        // R1081000: beside a Unit standing in the backrow, only a Unit standing there is adjacent.
        if !crate::zones::is_adjacent_card(sink.state, &defender, &neighbour) {
            continue;
        }
        deal_damage(
            sink,
            DamageArgs {
                source: Some(attacker.clone()),
                target: DamageTarget::Unit { instance: neighbour },
                amount,
                flags: attack_flags(poisonous),
            },
        );
    }
}

/// The attacker's hit and its Cleave, with the attacker read as it stands when it strikes.
fn strike(
    sink: &mut EngineSink<'_>,
    attacker: &CardInstance,
    target: &AttackTarget,
    hit: i32,
    cleave_amount: i32,
    poisonous: bool,
) {
    let source = live_or(sink.state, attacker);
    deal_damage(
        sink,
        DamageArgs {
            source: Some(source),
            target: target.clone(),
            amount: hit,
            flags: attack_flags(poisonous),
        },
    );
    let source = live_or(sink.state, attacker);
    cleave(sink, &source, target, cleave_amount, poisonous);
}

fn strike_back(sink: &mut EngineSink<'_>, defender: &CardInstance, attacker: &CardInstance, attack: i32) {
    let source = live_or(sink.state, defender);
    let target = live_or(sink.state, attacker);
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

/// R783000: whether `attacker` takes the defender's strike back — always, unless it is Ranged and the
/// defender is not. Read once per combat, beside R94's reads, by the combat and by the lethal
/// projection alike (`subsystems::lethal`).
pub fn takes_strike_back(state: &GameState, attacker: &CardInstance, defender: &CardInstance) -> bool {
    !crate::layers::unit_has(state, attacker, KeywordKind::Ranged)
        || crate::layers::unit_has(state, defender, KeywordKind::Ranged)
}

/// The exchange of §4.3, with no validation, no exertion and no state check: the First Strike step,
/// then the simultaneous step, both attacks read before either lands. A hero never strikes back, and a
/// Defense-Position defender strikes back with its full attack (its Armor +1 only on what it takes).
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
        strike(sink, &attacker, target, attack, attack, false);
        return;
    };

    // R94: both attacks are read once, here, before either hit lands, so a First Strike survivor is
    // struck back with the attack the defender had before the hit.
    let strike_back_attack = crate::layers::unit_view(sink.state, &defender).attack;
    // MD-D4, R1120: the combat-only modifiers on this attacker's strike at this defender, read once
    // beside R94's reads. Only the attacker's strikes get it; the strike back never does.
    let bonus = attack_mod_for(sink.state, &attacker, &defender);
    let attack = attack + bonus.attack;

    let hit = attack * lane_multiplier_for(sink.state, &attacker, &defender);
    let strike_back_attack = strike_back_attack * lane_multiplier_for(sink.state, &defender, &attacker);

    // R783000: a Ranged attacker's strike lands alone — no strike back, a First Strike one included —
    // unless the defender is Ranged too. Its Cleave splashes as ever (R783001).
    if !takes_strike_back(sink.state, &attacker, &defender) {
        strike(sink, &attacker, target, hit, attack, bonus.poisonous);
        return;
    }

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
        strike(sink, &attacker, target, hit, attack, bonus.poisonous);
        if !has_fallen_now(sink.state, &defender.id) {
            strike_back(sink, &defender, &attacker, strike_back_attack);
        }
        return;
    }
    if defender_first && !attacker_first {
        strike_back(sink, &defender, &attacker, strike_back_attack);
        if !has_fallen_now(sink.state, &attacker.id) {
            strike(sink, &attacker, target, hit, attack, bonus.poisonous);
        }
        return;
    }

    // Two First Strikers strike simultaneously in step 1, two ordinary units in step 2, and either
    // way the first death does not cancel the exchange (R59).
    strike(sink, &attacker, target, hit, attack, bonus.poisonous);
    strike_back(sink, &defender, &attacker, strike_back_attack);
}

// §4.2 step 4: the trap window between the declaration and the damage (R44, R100, R113, R117)

/// R113: the `resume.hook` of the one work item this module parks, the rest of an attack whose step 4
/// window a prompt interrupted: a name no `Script` can hold, which `work.rs` runs by `run_owed_attack`.
/// `work::run_work_item` raises on a hook nothing knows, since an attack that cannot be resumed is a
/// declaration that spent an exertion and never became damage.
pub const ATTACK_WINDOW_WORK: &str = "@attackWindow";

/// The window owes one step: §4.2 step 5, the combat the declaration has not resolved yet.
const ATTACK_COMBAT_STEP: &str = "combat";

/// R100: "an event the window is scheduled to deliver is not also offered to the immediate trap
/// check". The window delivers `attackDeclared` itself, so §10.3's frontier must not (a Field Trap
/// would answer twice): moving its cursor (`triggers::dispatched`) past the event takes it. That is per
/// declaration, not per *type* (`traps::TRAP_WINDOW_EVENTS`): a forced attack's opens no window (R121).
/// A test's sink may have its cursor behind the event, and moving it would drop the events between.
fn withhold_from_frontier(sink: &mut EngineSink<'_>, at: usize) {
    // Every sink over the action's list shares the frontier (`EngineSink::frontier`), so a loop on
    // another sink (a cast's step-4 window inside the window's traps, R70) sees the cursor move too.
    if crate::triggers::dispatched(sink) == at {
        crate::triggers::set_dispatched(sink, at + 1);
    }
}

/// The ordinary (non-trap) triggers on the declaration, queued in R68's order: the second half of
/// `triggers::dispatch_event`, whose first half step 4's window replaces; they pop after the combat.
/// R212: the board is read after the window, so a card that moved zones since is on a stay that did not
/// see the declaration (R174), and one whose controller changed answers for its controller then (R171).
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

/// Park the rest of the attack (R113) at `state.work_cursor`, which `traps::run_trap_window` has just
/// moved past the traps it still owes, so the window finishes first; only ids are held. R117: only when
/// the window pauses, never in advance, so a resolution loop inside the window (My Pawn's AI playout)
/// can neither take nor re-run the combat; pre-parking made a Cry fire twice (R1).
fn owe_declared_attack(sink: &mut EngineSink<'_>, id: &str) {
    let mut data = IndexMap::new();
    data.insert("declaredAttack".to_string(), json!(id));
    owe(sink, engine_resume(ATTACK_WINDOW_WORK, ATTACK_COMBAT_STEP, data));
}

fn close_window(state: &mut GameState, id: &str) -> Option<DeclaredAttack> {
    if state.declared_attack.as_ref().is_some_and(|open| open.id == id) {
        state.declared_attack.take()
    } else {
        None
    }
}

/// R220: whether a declaration still stands, asked by §4.2 step 5 and by the window's later traps
/// (`traps.rs`). It is aimed at the declared stays (R174), so an attacker or target that left the field
/// is gone even if a Reborn body (R83) stands there; the attacker must still be the declarer's and the
/// target its enemy (R173). Otherwise, as in Hearthstone, it is over: no combat, exertion spent (R44).
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
    // MD-D19, R1122: a redirected attack fights the ally it was re-aimed at, so a Unit target its
    // own controller holds still stands. Anything else must still be the attacker's enemy (R173).
    if open.redirected == Some(true)
        && matches!(&target, DamageTarget::Unit { instance } if instance.controller == attacker.controller)
    {
        return true;
    }
    is_enemy_of(attacker, &target)
}

/// §4.2 step 5 once step 4's window has closed: the combat and state check, when the declaration still
/// stands (R220). Read back by id, since the window can have replaced every instance (R44's AI turn,
/// `subsystems::ai_policy::adopt_state`); R94's attack read is `resolve_combat`'s, after the window.
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
    // R2285000: the attacker's own "When this attacks" first, before any damage.
    let open = if declared_attack_stands(sink.state, &open)
        && find_instance(sink.state, &open.attacker_id).is_some_and(|card| has_when_attacks(sink.state, card))
    {
        let owed = OwedWhenAttacks {
            attacker_id: open.attacker_id.clone(),
            target_id: open.target_id.clone(),
            declared: Some(open),
            since,
            stage: None,
        };
        match answer_when_attacks(sink, &owed, None).and_then(|owed| owed.declared) {
            Some(open) => open,
            None => return,
        }
    } else {
        open
    };
    defend_and_fight(sink, open, since, bounce);
}

/// §4.2 step 5 for a declaration whose attacker has answered it (R2285000): the defender's side's
/// "one of your Units is attacked" (R1849008), then the combat.
fn defend_and_fight(sink: &mut EngineSink<'_>, open: DeclaredAttack, since: u32, bounce: bool) {
    // R1849008: the defender's side answers "one of your Units is attacked" before the combat.
    if declared_attack_stands(sink.state, &open) {
        let holders = ally_attacked_holders(sink.state, &open);
        if !holders.is_empty() {
            let owed = OwedAllyAttacked {
                declared: open.clone(),
                holders,
                check_only: None,
            };
            if !answer_ally_attacked(sink, owed, None) {
                return;
            }
        }
    }
    fight_declared(sink, open, since, bounce);
}

/// §4.2 step 5 for a declaration that has come through its window (and its R1849008 hooks): the
/// combat, when it still stands (R220), else nothing but the substitute's bounce.
fn fight_declared(sink: &mut EngineSink<'_>, open: DeclaredAttack, since: u32, bounce: bool) {
    if sink.state.result.is_some() {
        return;
    }
    if !declared_attack_stands(sink.state, &open) {
        // R1203, MD-E15: a Prime whose target left in the window is canceled before any joiner musters.
        if let Some(prime) = find_instance(sink.state, &open.attacker_id)
            && is_active_on_field(sink.state, prime)
            && crate::scripts::flags_of(sink.state, prime).attack_joiners == Some(true)
        {
            sink.events.push(GameEvent::AttackCancelled {
                attacker_id: open.attacker_id.clone(),
                target_id: open.target_id.clone(),
                by_instance_id: open.attacker_id.clone(),
            });
        }
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

    // R1203: the Prime's joiners attack first; anything else fights at once, a substitute's bounce after.
    if crate::scripts::flags_of(sink.state, &attacker).attack_joiners != Some(true) {
        resolve_combat(sink, &attacker, &target);
        close_combat(sink, &attacker, &target, false, open.copies_for);
        if bounce {
            crate::attack_summon::bounce_after(sink, &attacker.id, since);
        }
        return;
    }
    match crate::attack_summon::joiners_first(sink, &attacker, &target, since) {
        crate::attack_summon::JoinOutcome::Paused => {
            crate::attack_summon::owe_combat(
                sink,
                &crate::attack_summon::OwedAttackSummon {
                    attacker_id: attacker.id.clone(),
                    target_id: crate::reduce::attack_target_id(&target),
                    forced: false,
                    bounce,
                    declared: Some(open),
                    instead_of: None,
                    since,
                },
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

/// R1202, R1203: the combat a replacement or rider parked (`attack_summon::run_owed`). A declared
/// attack that still stands fights; one whose target left the field first is canceled
/// (`attackCancelled`, MD-E15, exertion spent). A forced attack fights when attacker and target still
/// act since `since`. Then the bounce home a substitute owes, fought or not.
pub(crate) fn prime_combat(sink: &mut EngineSink<'_>, owed: &crate::attack_summon::OwedAttackSummon) {
    if let Some(open) = &owed.declared {
        if declared_attack_stands(sink.state, open) {
            let attacker = find_instance(sink.state, &open.attacker_id).cloned();
            let target = attack_target_of(sink.state, &open.target_id);
            if let (Some(attacker), Some(target)) = (attacker, target) {
                resolve_combat(sink, &attacker, &target);
                close_combat(sink, &attacker, &target, false, open.copies_for);
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
                forced_combat(sink, &attacker, &target);
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

/// §10.3, §4.2 step 4: the traps answer what the window did before step 5. A trap's events reach the
/// traps at once and resolve inside step 4, before any damage (one that destroys the attacker leaves
/// none for step 5, R220); the ordinary triggers wait for the loop after the combat. A trap that asks
/// pauses step 4 again, the combat owed behind it (R113). False when the combat must not resolve now.
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

/// `work.rs`'s handler for a parked attack (R113), the window's own events answered first.
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

/// The player's attack: §4.2 steps 1 to 5, validation, then step 4 (the exertion, the trap window),
/// then step 5's combat and state check; an attack Windfast would make, its substitute makes (R1202).
/// The declaration goes into `state.declared_attack` (what `effects::combat::cancel_attack` marks, §6.3,
/// R44) before `traps::run_trap_window` (§2.2) offers it to the traps, sharing R68's side order and R113's
/// parking. When the window pauses (R117) the combat is owed behind the traps it still owes: queued by
/// `triggers::dispatch_event` it would fire after the damage they exist to pre-empt.
pub fn declare_attack(
    sink: &mut EngineSink<'_>,
    attacker: &CardInstance,
    chosen: &AttackTarget,
) -> CombatResult {
    let attacker = live_or(sink.state, attacker);
    let chosen = live_target_or_given(sink.state, chosen);
    why_cannot_attack(sink.state, &attacker, &chosen)?;

    // B5 E5, E9: before the trap window, "a friendly unit is targeted": a defender's card that
    // interposes from the hand is summoned and the attack moves to it (`replacements::answer_targeting`).
    // An attack has no source card, so a `by: "spell"` card (Classic #33 Joro, R651) never answers one.
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

    // Step 4's first sentence. R44 never gives this back, so a canceled attack is gone either way.
    // R636: only a repeat attack writes `attacks`, so an ordinary exertion keeps its two-flag shape.
    // R1202: this is the declarer's exertion, Windfast's, whose substitute emits no declaration.
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

    // R1202, R831006: a random Unit summoned from its controller's hand makes Windfast's attack instead.
    match crate::attack_summon::replace_attacker(sink, &attacker) {
        crate::attack_summon::Replaced::By(unit) => {
            let bounce = crate::scripts::flags_of(sink.state, &attacker).bounce_attacker == Some(true);
            return declare_instead(sink, &unit, &target, Some(&attacker.id), bounce);
        }
        crate::attack_summon::Replaced::No => {}
    }

    declare_instead(sink, &attacker, &target, None, false)
}

/// R1202: `declare_attack` after its exertion (the declaration, the trap window, the combat) for
/// `attacker`, which a substitute may be. A substitute attacks only when the unit restrictions let it
/// (`attack_restriction`); its declaration carries `bounce_after` and a post-summon `exits_from`, its
/// event `instead_of` and `forced: false`, so it opens its trap window like any declared attack.
pub(crate) fn declare_instead(
    sink: &mut EngineSink<'_>,
    attacker: &CardInstance,
    target: &AttackTarget,
    instead_of: Option<&str>,
    bounce: bool,
) -> CombatResult {
    let attacker = live_or(sink.state, attacker);
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
        redirected: None,
        copies_for: None,
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
    // The interposer's `summoned` and `redirected` precede the declaration on the frontier (R100), the
    // declaration the window's alone. Marked unconditionally: where a replacement parked behind a prompt
    // (`attack_summon`) the cursor no longer stands at it, and elsewhere the mark changes nothing.
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

/// A forced attack (Moths to the Flame, Bear Honeypot; §4.2's last paragraph, R53): it skips steps 1
/// to 3 and spends no exertion; the target still strikes back, and its own state check follows.
/// R121: it opens no trap window ("declared by the effect, not the player"), so `state.declared_attack`
/// stays null and nothing cancels it (§6.3; #96's `when` refuses `forced`), and no prompt splits a card's
/// effect list, §10.3's one unit of work. Its `attackDeclared` still reaches the immediate check.
pub fn force_attack(sink: &mut EngineSink<'_>, attacker: &CardInstance, target: &AttackTarget) {
    force_attack_with(sink, attacker, target, false);
}

/// `force_attack`, with `allies` set for R1848005's melee, whose compulsion reaches the attacker's own
/// side too ("allies can attack each other"): the one change to R173 a melee makes.
fn force_attack_with(
    sink: &mut EngineSink<'_>,
    attacker: &CardInstance,
    target: &AttackTarget,
    allies: bool,
) {
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
    // on: against a target not its enemy the attacker is passed over in silence, as R96 passes one gone.
    if !allies && !is_enemy_of(&attacker, &target) {
        return;
    }
    // B5 E35: nor the unit restrictions; such an attack is passed over in silence the same way.
    if crate::restrictions::attack_restriction(sink.state, &attacker, &target).is_err() {
        return;
    }

    // R1202: a forced attack Windfast would make, its substitute makes instead.
    let since = crate::stays::exit_mark(sink.state);
    let (attacker, instead_of, bounce) = match crate::attack_summon::replace_attacker(sink, &attacker) {
        crate::attack_summon::Replaced::By(unit) => {
            let bounce = crate::scripts::flags_of(sink.state, &attacker).bounce_attacker == Some(true);
            (*unit, Some(attacker.id.clone()), bounce)
        }
        crate::attack_summon::Replaced::No => (attacker, None, false),
    };

    // R1203: the Prime's joiners attack first, then its own combat.
    if crate::scripts::flags_of(sink.state, &attacker).attack_joiners == Some(true) {
        match crate::attack_summon::joiners_first(sink, &attacker, &target, since) {
            crate::attack_summon::JoinOutcome::Paused => {
                crate::attack_summon::owe_combat(
                    sink,
                    &crate::attack_summon::OwedAttackSummon {
                        attacker_id: attacker.id.clone(),
                        target_id: crate::reduce::attack_target_id(&target),
                        forced: true,
                        bounce,
                        declared: None,
                        instead_of: instead_of.clone(),
                        since,
                    },
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

/// R1202: a forced attack that skips the replacement (the substitute's own, or one continued after its
/// pick or its joiners), declared with `instead_of`, then its combat and state check.
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

    forced_combat(sink, attacker, target);
}

/// A forced attack's combat and state check (R53), its attacker's "When this attacks" first
/// (R2285000); after one, the combat is fought only while attacker and target still act on the stays
/// the attack began on (R174). A question in the hook owes the rest, the combat included (R113), and
/// a run that made this attack owes its own rest behind it.
fn forced_combat(sink: &mut EngineSink<'_>, attacker: &CardInstance, target: &AttackTarget) {
    if !has_when_attacks(sink.state, attacker) {
        resolve_combat(sink, attacker, target);
        close_combat(sink, attacker, target, true, None);
        return;
    }
    let owed = OwedWhenAttacks {
        attacker_id: attacker.id.clone(),
        target_id: target_id_of(target),
        declared: None,
        since: crate::stays::exit_mark(sink.state),
        stage: None,
    };
    if let Some(owed) = answer_when_attacks(sink, &owed, None) {
        fight_forced_after(sink, &owed);
    }
}

/// R2285000: a forced attack's combat once its attacker's hook has run.
fn fight_forced_after(sink: &mut EngineSink<'_>, owed: &OwedWhenAttacks) {
    let attacker = find_instance(sink.state, &owed.attacker_id).cloned();
    let target = attack_target_of(sink.state, &owed.target_id);
    let (Some(attacker), Some(target)) = (attacker, target) else {
        return;
    };
    if !is_active_on_field(sink.state, &attacker)
        || crate::stays::left_field_after(sink.state, owed.since, &attacker.id)
        || !target_acts_since(sink.state, &target, owed.since)
    {
        return;
    }
    resolve_combat(sink, &attacker, &target);
    close_combat(sink, &attacker, &target, true, None);
}

/// §4.2 step 2: an attack is made on an enemy unit or the enemy hero, forced or not (R173).
pub(crate) fn is_enemy_of(attacker: &CardInstance, target: &AttackTarget) -> bool {
    let enemy = opponent_of(attacker.controller);
    match target {
        DamageTarget::Hero { player } => *player == enemy,
        DamageTarget::Unit { instance } => instance.controller == enemy,
    }
}

/// R53: the named units attack the named target one at a time, in the order given (lane order), each
/// its own combat and state check, stopping once the target is off the field; R174 counts a target
/// that left and came back (a Reborn body, R83) as gone. So with an attacker: one that died and came
/// back through Reborn is passed over in silence (R96). `since: None` is the mark now.
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
        // R53, R113: the last combat's check (R59) collected a Death that asks; the attackers still to
        // come are owed rather than walked on over the prompt, and the answer's drain brings them.
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

/// R113: the `resume.hook` of a forced run a Death's prompt stopped between two combats: the rest, by
/// id, since R44's AI turn can replace every instance before the answer.
pub const FORCED_RUN_WORK: &str = "@forcedRun";

/// Park the rest of a run at the pause, with the mark it began at, so the answer's own departures
/// count against the stays the run named too (R174).
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

// B5 E35: forced attacks on "a random enemy" and on the unit's own hero

/// B5 E35: what a forced attack on "a random enemy" draws from: enemy units in lane order, then the
/// enemy hero for "an enemy". R53 waives position, sickness and Taunt; the unit restrictions and a
/// carrier (R446) still narrow the list, so no roll is spent on a carried Unit.
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
/// among the ones it may attack as that attack begins (Classic #78, Classic+ #19.2), each its own combat
/// and state check, stopping once the attacker has left the field on the stay it began on or has
/// nothing to attack. A Death between two that asks owes the rest (`FORCED_RANDOM_WORK`, R113).
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

/// B5 E35: a forced attack on the unit's OWN hero (Classic+ #19.5 Bot Loser while Berserk): R53's
/// compulsion with the one change R173 never allows elsewhere. A hero never strikes back (§4.3), the
/// combat has its own state check, and the unit restrictions still hold.
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
    forced_combat(sink, &attacker, &target);
}

// R1848005: the melee, every Unit on the field attacking another until one is left

/// R1848005 (Stinkyscape #120 Wild System, "allies can attack each other, max 20 attacks"): up to
/// `attacks` forced attacks (R53), each between a pair drawn from the match rng among the Units acting
/// on the field (`active_units_of`, active player's first; none beneath a Stack, R13, nor carried,
/// R446), never a hero. Each is its own combat and state check; a question owes the rest (R113).
pub fn run_melee(sink: &mut EngineSink<'_>, attacks: i32) {
    let mut left = attacks;
    while left > 0 {
        if sink.state.result.is_some() {
            return;
        }
        if is_paused(sink) {
            owe_melee(sink, left);
            return;
        }
        let Some((attacker, target)) = melee_pair(sink) else {
            return;
        };
        force_attack_with(sink, &attacker, &target, true);
        left -= 1;
    }
}

/// R1848005: the next pair of a melee — the attacker drawn among the fighters that have a target, then
/// its target among the Units it may attack — or `None` when one Unit or none is left or no fighter may
/// attack another. Two draws per attack, none when there is no pair.
fn melee_pair(sink: &mut EngineSink<'_>) -> Option<(CardInstance, AttackTarget)> {
    let pairs: Vec<(CardInstance, Vec<AttackTarget>)> = {
        let state = &*sink.state;
        let first = state.active;
        let fighters: Vec<CardInstance> = crate::zones::active_units_of(state, first)
            .into_iter()
            .chain(crate::zones::active_units_of(state, opponent_of(first)))
            .filter(|unit| !carried_out_of_combat(state, unit))
            .cloned()
            .collect();
        if fighters.len() <= 1 {
            return None;
        }
        fighters
            .iter()
            .filter_map(|attacker| {
                let targets: Vec<AttackTarget> = fighters
                    .iter()
                    .filter(|other| other.id != attacker.id)
                    .map(|other| DamageTarget::Unit {
                        instance: other.clone(),
                    })
                    .filter(|target| crate::restrictions::attack_restriction(state, attacker, target).is_ok())
                    .collect();
                (!targets.is_empty()).then(|| (attacker.clone(), targets))
            })
            .collect()
    };
    let (attacker, targets) = sink.rng.pick(&pairs)?.clone();
    let target = sink.rng.pick(&targets)?.clone();
    Some((attacker, target))
}

/// R113: the `resume.hook` of a melee a combat's question stopped between two attacks.
pub const MELEE_WORK: &str = "@melee";

/// (`{ left }`: the attacks the melee may still make.)
fn owe_melee(sink: &mut EngineSink<'_>, left: i32) {
    if left <= 0 || sink.state.result.is_some() {
        return;
    }
    let mut data = IndexMap::new();
    data.insert("left".to_string(), json!(left));
    owe(sink, engine_resume(MELEE_WORK, "run", data));
}

/// `work.rs`'s handler for a melee a prompt stopped: the same melee, with the attacks it has left.
pub fn run_owed_melee(sink: &mut EngineSink<'_>, item: &WorkItem) {
    let Some(left) = item.resume.data.get("left").and_then(Value::as_i64) else {
        return;
    };
    run_melee(sink, left as i32);
}

// "When this attacks": the attacker's `whenAttacks` hook, before any damage of its attack (R2285000)

/// R113: the `resume.hook` of an attacker's `whenAttacks` owed before its combat (its rest after a
/// question of its own, or its state check), then the rest of the attack.
pub const WHEN_ATTACKS_WORK: &str = "@whenAttacks";

/// The facts an attack hands its attacker's `whenAttacks` hook in its `ctx.data` (`when_attacks_of`
/// reads them back): the target (a unit's id or `hero-<player>`) and whether it was forced (R53).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WhenAttacksFacts {
    pub target_id: String,
    pub forced: bool,
}

/// The facts a `whenAttacks` hook was handed, or `None` outside one.
pub fn when_attacks_of(ctx: &crate::script::EffectContext<'_>) -> Option<WhenAttacksFacts> {
    let data = &ctx.data;
    let target_id = data.get("targetId")?.as_str()?.to_string();
    let forced = data.get("forced")?.as_bool()?;
    Some(WhenAttacksFacts { target_id, forced })
}

/// What an owed `@whenAttacks` carries, all JSON: the attack the hook comes before — the declaration
/// a declared attack stands on, or for a forced one (`declared: None`) its attacker, its target and
/// the stays it began on, by id (R44's AI turn can replace every instance before an answer).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct OwedWhenAttacks {
    attacker_id: String,
    target_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    declared: Option<DeclaredAttack>,
    since: u32,
    /// How far the sequence got: `None` before the hook is done, `"check"` once only the state check
    /// after it is owed, `"attack"` once only the rest of the attack is.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stage: Option<String>,
}

const WHEN_ATTACKS_KEY: &str = "whenAttacks";
const CHECK_STAGE: &str = "check";
const ATTACK_STAGE: &str = "attack";

fn has_when_attacks(state: &GameState, card: &CardInstance) -> bool {
    crate::scripts::script_of(state, card).when_attacks.is_some()
}

fn owe_when_attacks(sink: &mut EngineSink<'_>, owed: &OwedWhenAttacks) {
    let mut data = IndexMap::new();
    data.insert(WHEN_ATTACKS_KEY.to_string(), to_json(owed));
    owe(sink, engine_resume(WHEN_ATTACKS_WORK, "hook", data));
}

/// R2285000: the attacker's hook, from `paused` when a question split it, then its state check (R59).
/// It runs for the attacker's controller, a new one only while the attacker still acts on the field.
/// The attack to go on with, retargeted when the hook transformed its target (R2285002); `None` when
/// the game ended or a question stopped the sequence, its rest owed on `state.work` (R113).
fn answer_when_attacks(
    sink: &mut EngineSink<'_>,
    owed: &OwedWhenAttacks,
    paused: Option<PausedStep>,
) -> Option<OwedWhenAttacks> {
    if sink.state.result.is_some() {
        return None;
    }
    match owed.stage.as_deref() {
        Some(ATTACK_STAGE) => return Some(owed.clone()),
        Some(CHECK_STAGE) => return check_after_when_attacks(sink, owed),
        _ => {}
    }
    let card = find_instance(sink.state, &owed.attacker_id)
        .filter(|card| paused.is_some() || is_active_on_field(sink.state, card))
        .cloned();
    let hook = card
        .as_ref()
        .and_then(|card| crate::scripts::script_of(sink.state, card).when_attacks.clone());
    let (Some(card), Some(hook)) = (card, hook) else {
        return Some(owed.clone());
    };
    let from = sink.events.len();
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert("targetId".to_string(), json!(owed.target_id));
    data.insert("forced".to_string(), json!(owed.declared.is_none()));
    let mut plan_data = IndexMap::new();
    plan_data.insert(WHEN_ATTACKS_KEY.to_string(), to_json(owed));
    let plan = WorkPlan {
        def_id: String::new(),
        hook: WHEN_ATTACKS_WORK.to_string(),
        step: "hook".to_string(),
        radiant: false,
        instance_id: None,
        data: plan_data,
        owner: card.controller,
    };
    let status = {
        let mut ctx = crate::resolve::make_context(
            sink,
            Some(&card),
            crate::resolve::HookOptions {
                controller: Some(card.controller),
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
    let followed = retargeted(owed, &followed_target(sink, &owed.target_id, from));
    match status {
        crate::prompts::ListStatus::Done => check_after_when_attacks(sink, &followed),
        // The hook's last effect asked: the hook is done, and its check waits for the answer.
        crate::prompts::ListStatus::Asked => {
            owe_when_attacks(
                sink,
                &OwedWhenAttacks {
                    stage: Some(CHECK_STAGE.to_string()),
                    ..followed
                },
            );
            None
        }
        // Parked: the hook's rest is on `state.work`, and takes the target the hook already transformed.
        crate::prompts::ListStatus::Parked => {
            if followed.target_id != owed.target_id {
                repark_retargeted(sink, owed, &followed);
            }
            None
        }
        crate::prompts::ListStatus::Over => None,
    }
}

/// The state check after the hook (R59); a Death's question there owes the rest of the attack.
fn check_after_when_attacks(sink: &mut EngineSink<'_>, owed: &OwedWhenAttacks) -> Option<OwedWhenAttacks> {
    crate::state_check::state_check(sink);
    if is_paused(sink) {
        if sink.state.result.is_none() {
            owe_when_attacks(
                sink,
                &OwedWhenAttacks {
                    stage: Some(ATTACK_STAGE.to_string()),
                    ..owed.clone()
                },
            );
        }
        return None;
    }
    Some(owed.clone())
}

/// R2285002: the Unit an attack's target became, following each `transformed` since `from`: §6.3's
/// Transform puts a new instance in the same zone and position, and the attack goes on against it.
fn followed_target(sink: &EngineSink<'_>, target_id: &str, from: usize) -> String {
    let mut id = target_id.to_string();
    for event in sink.events.get(from..).unwrap_or(&[]) {
        if let GameEvent::Transformed {
            instance_id,
            new_instance_id,
            ..
        } = event
            && *instance_id == id
        {
            id = new_instance_id.clone();
        }
    }
    id
}

fn retargeted(owed: &OwedWhenAttacks, target_id: &str) -> OwedWhenAttacks {
    let mut owed = owed.clone();
    if let Some(open) = owed.declared.as_mut() {
        open.target_id = target_id.to_string();
    }
    owed.target_id = target_id.to_string();
    owed
}

/// R2285002: the item a question parked mid-hook carries the attack as the hook began; it goes on
/// against the target's new instance instead.
fn repark_retargeted(sink: &mut EngineSink<'_>, was: &OwedWhenAttacks, now: &OwedWhenAttacks) {
    let parked = sink.state.work.iter_mut().find(|item| {
        item.resume.hook == WHEN_ATTACKS_WORK
            && owed_when_attacks_of(&item.resume.data).is_some_and(|owed| owed == *was)
    });
    if let Some(item) = parked {
        item.resume
            .data
            .insert(WHEN_ATTACKS_KEY.to_string(), to_json(now));
    }
}

fn owed_when_attacks_of(data: &IndexMap<String, Value>) -> Option<OwedWhenAttacks> {
    let raw = data.get(WHEN_ATTACKS_KEY)?;
    let owed = raw.as_object()?;
    owed.get("attackerId")?.as_str()?;
    owed.get("targetId")?.as_str()?;
    owed.get("since")?.as_u64()?;
    serde_json::from_value::<OwedWhenAttacks>(raw.clone()).ok()
}

/// `work.rs`'s handler: the hook (or its rest, or its check) where the pause left it, then the rest
/// of the attack — a declared one's defenders and combat, a forced one's combat (R113).
pub fn run_owed_when_attacks(sink: &mut EngineSink<'_>, item: &WorkItem) {
    let Some(owed) = owed_when_attacks_of(&item.resume.data) else {
        return;
    };
    let Some(owed) = answer_when_attacks(sink, &owed, paused_of(&item.resume.data)) else {
        return;
    };
    match owed.declared {
        Some(open) => {
            let since = open
                .exits_from
                .unwrap_or_else(|| crate::stays::exit_mark(sink.state));
            let bounce = open.bounce_after == Some(true);
            defend_and_fight(sink, open, since, bounce);
        }
        None => fight_forced_after(sink, &owed),
    }
}

// "After this attacks": the attacker's `afterAttack` hook, once the combat's state check has closed

/// R113: the `resume.hook` of an attacker's `afterAttack` owed behind its combat's closing check (a
/// Death there asked), or of its rest after a question of its own, with the check that follows it.
pub const AFTER_ATTACK_WORK: &str = "@afterAttack";

/// The facts a combat hands its attacker's `afterAttack` hook, in the hook's `ctx.data`
/// (`after_attack_of` reads them back): the attack's target (a unit's id or `hero-<player>`), the units
/// the attacker's lethal hits destroyed (R42's killer: the attacked unit, and Cleave's kills), whether
/// the attacker is still on the field on the stay it attacked from once the check has closed (false only
/// for a "Whenever this attacks" whose attacker died or left, R2396000; R2285001), and whether the attack
/// was forced (R53).
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
    /// The attacker just before the check that closed its combat (R78, R89): whose it was in that combat.
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

/// R113: the `resume.hook` of a combat's owed Radiant copies (MD-D20, R1123) — the fresh copies a
/// Death hook's question stopped before they were summoned.
pub const COMBAT_COPIES_WORK: &str = "@combatCopies";

/// One fresh copy MD-D20 owes: by definition and face (R409's reading), for `player`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct CombatCopy {
    def_id: String,
    radiant: bool,
}

/// What an owed `@combatCopies` item carries, all JSON.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct OwedCombatCopies {
    player: PlayerId,
    copies: Vec<CombatCopy>,
}

const COMBAT_COPIES_KEY: &str = "copies";

/// MD-D20, R1123: the fresh copies this combat owes `player` — one per `Destroyed` event since `from`
/// whose killer was one of the two combatants (the attacker or the target unit), by definition and
/// face, in event order. Anything else that died meanwhile — a Death hook's victim — is no copy.
fn combat_copies(
    sink: &EngineSink<'_>,
    attacker_id: &str,
    target: &AttackTarget,
    from: usize,
) -> Vec<CombatCopy> {
    let target_id = match target {
        DamageTarget::Unit { instance } => Some(instance.id.as_str()),
        DamageTarget::Hero { .. } => None,
    };
    sink.events
        .get(from.min(sink.events.len())..)
        .unwrap_or(&[])
        .iter()
        .filter_map(|event| match event {
            GameEvent::Destroyed {
                def_id,
                killer_id: Some(killer),
                radiant,
                ..
            } if killer == attacker_id || target_id == Some(killer.as_str()) => Some(CombatCopy {
                def_id: def_id.clone(),
                radiant: radiant == &Some(true),
            }),
            _ => None,
        })
        .collect()
}

/// MD-D20, R1123: summon the owed copies for `player`, left to right (R64), a full board taking
/// fewer. Fresh cards by definition and face — no Cry, summoning sick — as R409's reading copies.
fn apply_combat_copies(sink: &mut EngineSink<'_>, player: PlayerId, copies: &[CombatCopy]) {
    use crate::effects::summon::{SummonArgs, summon};
    use crate::effects::targets::PlayerSpec;
    for copy in copies {
        let mut ctx = crate::resolve::make_context(
            sink,
            None,
            crate::resolve::HookOptions {
                controller: Some(player),
                ..crate::resolve::HookOptions::default()
            },
        );
        let effect = summon(SummonArgs {
            player: Some(PlayerSpec::SelfSide),
            def_id: Some(copy.def_id.clone()),
            radiant: if copy.radiant { Some(true) } else { None },
            ..SummonArgs::default()
        });
        (effect.apply)(&mut ctx);
    }
}

fn owe_combat_copies(sink: &mut EngineSink<'_>, player: PlayerId, copies: &[CombatCopy]) {
    let owed = OwedCombatCopies {
        player,
        copies: copies.to_vec(),
    };
    let mut data = IndexMap::new();
    data.insert(COMBAT_COPIES_KEY.to_string(), to_json(&owed));
    owe(sink, engine_resume(COMBAT_COPIES_WORK, "copies", data));
}

/// `work.rs`'s handler for owed Radiant copies: the same copies, where the pause left them (R113).
pub fn run_owed_combat_copies(sink: &mut EngineSink<'_>, item: &WorkItem) {
    let Some(raw) = item.resume.data.get(COMBAT_COPIES_KEY) else {
        return;
    };
    let Ok(owed) = serde_json::from_value::<OwedCombatCopies>(raw.clone()) else {
        return;
    };
    if owed.copies.is_empty() || sink.state.result.is_some() {
        return;
    }
    apply_combat_copies(sink, owed.player, &owed.copies);
}

/// The state check that closes a combat (§4.2 step 5, §4.3 step 3, R53), then the attacker's
/// `afterAttack` hook (Classic+ #73.1, Stinkyscape #91): only for an attacker still on the field on the
/// stay it attacked from (R2285001), or, for a "Whenever this attacks" (Core #32), whatever became of it
/// (R2396000). A called-off attack (R44, R220) never reaches here. The hook is a whole effect, so a check
/// follows it (R59); a question inside it, or in the check before it, owes the rest to `state.work` (R113).
///
/// MD-D20, R1123: when `copies_for` names a player, that player's fresh copies of what the two
/// combatants destroyed come first, right after the check and before `afterAttack`.
fn close_combat(
    sink: &mut EngineSink<'_>,
    attacker: &CardInstance,
    target: &AttackTarget,
    forced: bool,
    copies_for: Option<PlayerId>,
) {
    let snapshot = live_or(sink.state, attacker);
    // MD-C26: the defender as it fought, before the closing check; a pure read (no rng), so replays hold.
    let defender_snapshot = match target {
        DamageTarget::Unit { instance } => Some(live_or(sink.state, instance)),
        DamageTarget::Hero { .. } => None,
    };
    let since = crate::stays::exit_mark(sink.state);
    let from = sink.events.len();
    crate::state_check::state_check(sink);
    if let Some(player) = copies_for {
        let copies = combat_copies(sink, &snapshot.id, target, from);
        if !copies.is_empty() {
            if is_paused(sink) {
                owe_combat_copies(sink, player, &copies);
            } else {
                apply_combat_copies(sink, player, &copies);
            }
        }
    }
    if sink.state.result.is_some() {
        return;
    }
    // R2199000: the cards of the attacker's side that answer "after one of your Units attacks",
    // read as the check closes; they run after the combatants' own hooks. None, nothing runs.
    let allies = OwedAllyAttack {
        attacker_id: snapshot.id.clone(),
        target_id: target_id_of(target),
        forced,
        since,
        holders: ally_attack_holders(sink.state, snapshot.controller),
        survived: None,
        check_only: None,
    };
    combatant_hooks(sink, snapshot, defender_snapshot, target, forced, since, from);
    if !allies.holders.is_empty() && sink.state.result.is_none() {
        if is_paused(sink) {
            owe_ally_attack(sink, &allies);
        } else {
            answer_ally_attack(sink, allies, None);
        }
    }
}

/// MD-C26: the attacker's `afterAttack`, then the defender's `afterAttacked`, each when it has one.
fn combatant_hooks(
    sink: &mut EngineSink<'_>,
    snapshot: CardInstance,
    defender_snapshot: Option<CardInstance>,
    target: &AttackTarget,
    forced: bool,
    since: u32,
    from: usize,
) {
    let attacker_id = snapshot.id.clone();
    if crate::scripts::script_of(sink.state, &snapshot)
        .after_attack
        .is_some()
    {
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
    // MD-C26: then the defender's `afterAttacked`, once per combat it was attacked in (never a hero).
    if let Some(defender) = defender_snapshot
        && crate::scripts::script_of(sink.state, &defender)
            .after_attacked
            .is_some()
    {
        let owed = OwedAfterAttacked {
            defender_id: defender.id.clone(),
            attacker_id,
            snapshot: defender,
            forced,
            since,
            defender_survived: None,
            attacker_survived: None,
            check_only: None,
        };
        if is_paused(sink) {
            owe_after_attacked(sink, &owed);
            return;
        }
        run_after_attacked(sink, &owed, None);
    }
}

fn owe_after_attack(sink: &mut EngineSink<'_>, owed: &OwedAfterAttack) {
    let mut data = IndexMap::new();
    data.insert(AFTER_ATTACK_KEY.to_string(), to_json(owed));
    owe(sink, engine_resume(AFTER_ATTACK_WORK, "hook", data));
}

/// The attacker's hook, from `paused` when a question split it, then the check that follows it.
fn run_after_attack(sink: &mut EngineSink<'_>, owed: &OwedAfterAttack, paused: Option<PausedStep>) {
    let script = crate::scripts::script_of(sink.state, &owed.snapshot);
    let Some(hook) = script.after_attack.clone() else {
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
    // R2285001, R2396000 (above): an "After this attacks" for a gone attacker does nothing and owes no
    // check; a "Whenever this attacks" runs as the snapshot it fought with, as `run_after_attacked` runs
    // a dead defender; a Reborn body is not it, so it runs once.
    let self_card = match (live, survived) {
        (Some(live), true) => live,
        _ if script.after_attack_even_if_gone => owed.snapshot.clone(),
        _ => return,
    };
    let judged = OwedAfterAttack {
        survived: Some(survived),
        ..owed.clone()
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
                // R426: its controller in that combat, though a Death in the check took it since.
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

// "After this is attacked": the defender's `afterAttacked` hook (Meditative #49.3, MD-C26, R1026)

/// R113: the `resume.hook` of a defender's `afterAttacked` owed behind its combat's closing check, or
/// of its rest after a question of its own, with the check that follows it.
pub const AFTER_ATTACKED_WORK: &str = "@afterAttacked";

/// The facts a combat hands its defender's `afterAttacked` hook, in its `ctx.data`
/// (`after_attacked_of` reads them back): the attacker's id, whether the attack was forced (R53), and
/// whether the attacker survived the closing check. The attacker's hook runs first (MD-C26).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AfterAttackedFacts {
    pub attacker_id: String,
    pub forced: bool,
    pub attacker_survived: bool,
}

/// The combat facts an `afterAttacked` hook was handed, or `None` outside one.
pub fn after_attacked_of(ctx: &crate::script::EffectContext<'_>) -> Option<AfterAttackedFacts> {
    let data = &ctx.data;
    let attacker_id = data.get("attackerId")?.as_str()?.to_string();
    let forced = data.get("forced")?.as_bool()?;
    let attacker_survived = data.get("attackerSurvived")?.as_bool()?;
    Some(AfterAttackedFacts {
        attacker_id,
        forced,
        attacker_survived,
    })
}

/// What an owed `afterAttacked` carries, all JSON: the combat's facts and the defender as it fought.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct OwedAfterAttacked {
    defender_id: String,
    /// The defender just before the check that closed its combat (R78, R89): its self if it died.
    snapshot: CardInstance,
    attacker_id: String,
    forced: bool,
    /// The field's departures before that check, to tell a survivor from a Reborn body (R174).
    since: u32,
    /// Judged once, as the hook first runs, when the check has closed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    defender_survived: Option<bool>,
    /// Judged once, beside it: whether the attacker is still on the field it attacked from.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    attacker_survived: Option<bool>,
    /// The hook is done and only the state check that follows it is owed. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    check_only: Option<bool>,
}

const AFTER_ATTACKED_KEY: &str = "attacked";

/// Whether this card is still on the field on the stay it fought from: live, acting, and not left
/// since the check's mark — the same judgement `run_after_attack` makes for its attacker.
fn fought_on(state: &GameState, since: u32, id: &str) -> bool {
    find_instance(state, id).is_some_and(|live| {
        is_active_on_field(state, live) && !crate::stays::left_field_after(state, since, &live.id)
    })
}

/// The defender's hook, from `paused` when a question split it, then the check that follows it.
fn run_after_attacked(sink: &mut EngineSink<'_>, owed: &OwedAfterAttacked, paused: Option<PausedStep>) {
    let Some(hook) = crate::scripts::script_of(sink.state, &owed.snapshot)
        .after_attacked
        .clone()
    else {
        return;
    };
    if sink.state.result.is_some() {
        return;
    }
    let live = find_instance(sink.state, &owed.defender_id).cloned();
    let defender_survived = owed
        .defender_survived
        .unwrap_or_else(|| fought_on(sink.state, owed.since, &owed.defender_id));
    let attacker_survived = owed
        .attacker_survived
        .unwrap_or_else(|| fought_on(sink.state, owed.since, &owed.attacker_id));
    let judged = OwedAfterAttacked {
        defender_survived: Some(defender_survived),
        attacker_survived: Some(attacker_survived),
        ..owed.clone()
    };
    // A survivor is itself, on the field; one that died is the snapshot it fought with, which a
    // continuation reads back too (`prompts::SELF_KEY`, R89).
    let self_card = match (&live, defender_survived) {
        (Some(live), true) => live.clone(),
        _ => owed.snapshot.clone(),
    };
    // The facts, then `SELF_KEY` for a defender that died, in that key order.
    let mut data: IndexMap<String, Value> = IndexMap::new();
    data.insert("attackerId".to_string(), json!(owed.attacker_id));
    data.insert("forced".to_string(), json!(owed.forced));
    data.insert("attackerSurvived".to_string(), json!(attacker_survived));
    if !defender_survived {
        data.insert(crate::prompts::SELF_KEY.to_string(), to_json(&owed.snapshot));
    }
    let mut plan_data = IndexMap::new();
    plan_data.insert(AFTER_ATTACKED_KEY.to_string(), to_json(&judged));
    let plan = WorkPlan {
        def_id: String::new(),
        hook: AFTER_ATTACKED_WORK.to_string(),
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
                // MD-C26: its controller in that combat, though a Death in the check took it since.
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
        crate::prompts::ListStatus::Asked => owe_after_attacked(
            sink,
            &OwedAfterAttacked {
                check_only: Some(true),
                ..judged
            },
        ),
        _ => {}
    }
}

fn owe_after_attacked(sink: &mut EngineSink<'_>, owed: &OwedAfterAttacked) {
    let mut data = IndexMap::new();
    data.insert(AFTER_ATTACKED_KEY.to_string(), to_json(owed));
    owe(sink, engine_resume(AFTER_ATTACKED_WORK, "hook", data));
}

fn owed_after_attacked_of(data: &IndexMap<String, Value>) -> Option<OwedAfterAttacked> {
    let raw = data.get(AFTER_ATTACKED_KEY)?;
    let owed = raw.as_object()?;
    owed.get("defenderId")?.as_str()?;
    owed.get("attackerId")?.as_str()?;
    owed.get("since")?.as_u64()?;
    owed.get("snapshot")?;
    owed.get("forced")?.as_bool()?;
    serde_json::from_value::<OwedAfterAttacked>(raw.clone()).ok()
}

/// `work.rs`'s handler: the hook (or its rest), then the check, where the pause left them (R113).
pub fn run_owed_after_attacked(sink: &mut EngineSink<'_>, item: &WorkItem) {
    let Some(owed) = owed_after_attacked_of(&item.resume.data) else {
        return;
    };
    if owed.check_only == Some(true) {
        crate::state_check::state_check(sink);
        return;
    }
    run_after_attacked(sink, &owed, paused_of(&item.resume.data));
}

// "When one of your Units is attacked": `allyAttacked` hooks (Stinkyscape #14 Arm of the Jempire, R1849008)

/// R113: the `resume.hook` of the `allyAttacked` hooks a declared attack still owes before its combat,
/// and the combat after them, from wherever a question (a hook's own, or a Death's) stopped them.
pub const ALLY_ATTACKED_WORK: &str = "@allyAttacked";

/// The facts an `allyAttacked` hook is handed in its `ctx.data` (`ally_attacked_of` reads them back):
/// the attacker's id and the id of the attacked Unit, as the attack stands after its trap window.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AllyAttackedFacts {
    pub attacker_id: String,
    pub defender_id: String,
}

/// The facts an `allyAttacked` hook was handed, or `None` outside one.
pub fn ally_attacked_of(ctx: &crate::script::EffectContext<'_>) -> Option<AllyAttackedFacts> {
    let data = &ctx.data;
    let attacker_id = data.get("attackerId")?.as_str()?.to_string();
    let defender_id = data.get("defenderId")?.as_str()?.to_string();
    Some(AllyAttackedFacts {
        attacker_id,
        defender_id,
    })
}

/// What an owed `@allyAttacked` carries, all JSON: the declaration its combat resolves, and the
/// holders still to answer it in R68's order — the first is the one a pause stood in.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct OwedAllyAttacked {
    declared: DeclaredAttack,
    holders: Vec<String>,
    /// The first holder's hook is done; only its state check is owed. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    check_only: Option<bool>,
}

const ALLY_ATTACKED_KEY: &str = "allyAttacked";

/// R1849008: the cards that answer this declaration, every card acting on the attacked Unit's side
/// with an `allyAttacked` hook (the Unit included) in R68's order; none for an attack on a hero.
fn ally_attacked_holders(state: &GameState, open: &DeclaredAttack) -> Vec<String> {
    let Some(DamageTarget::Unit { instance }) = attack_target_of(state, &open.target_id) else {
        return Vec::new();
    };
    crate::damage::acting_texts_of(state, instance.controller)
        .into_iter()
        .filter(|card| crate::scripts::script_of(state, card).ally_attacked.is_some())
        .map(|card| card.id)
        .collect()
}

fn owe_ally_attacked(sink: &mut EngineSink<'_>, owed: &OwedAllyAttacked) {
    let mut data = IndexMap::new();
    data.insert(ALLY_ATTACKED_KEY.to_string(), to_json(owed));
    owe(sink, engine_resume(ALLY_ATTACKED_WORK, "hook", data));
}

/// R1849008: the holders' hooks in turn, each a whole effect followed by its state check (R59), from
/// `paused` when a question split the first. A holder no longer acting is passed over, and once the
/// declaration no longer stands (R220) no further hook starts. True when the combat may go on now;
/// false when the game ended or a question stopped the sequence, its rest owed on `state.work` (R113).
fn answer_ally_attacked(
    sink: &mut EngineSink<'_>,
    mut owed: OwedAllyAttacked,
    mut paused: Option<PausedStep>,
) -> bool {
    if owed.check_only.take() == Some(true) && !owed.holders.is_empty() {
        owed.holders.remove(0);
        crate::state_check::state_check(sink);
        if is_paused(sink) {
            if sink.state.result.is_none() {
                owe_ally_attacked(sink, &owed);
            }
            return false;
        }
    }
    while let Some(id) = owed.holders.first().cloned() {
        if sink.state.result.is_some() {
            return false;
        }
        // A hook a question split finishes whatever became of the attack; a new one starts only on
        // an attack that still stands.
        let resumed = paused.take();
        if resumed.is_none() && !declared_attack_stands(sink.state, &owed.declared) {
            return true;
        }
        let card = find_instance(sink.state, &id)
            .filter(|card| resumed.is_some() || is_active_on_field(sink.state, card))
            .cloned();
        let hook = card
            .as_ref()
            .and_then(|card| crate::scripts::script_of(sink.state, card).ally_attacked.clone());
        let (Some(card), Some(hook)) = (card, hook) else {
            owed.holders.remove(0);
            continue;
        };
        let mut data: IndexMap<String, Value> = IndexMap::new();
        data.insert("attackerId".to_string(), json!(owed.declared.attacker_id));
        data.insert("defenderId".to_string(), json!(owed.declared.target_id));
        let mut plan_data = IndexMap::new();
        plan_data.insert(ALLY_ATTACKED_KEY.to_string(), to_json(&owed));
        let plan = WorkPlan {
            def_id: String::new(),
            hook: ALLY_ATTACKED_WORK.to_string(),
            step: "hook".to_string(),
            radiant: false,
            instance_id: None,
            data: plan_data,
            owner: card.controller,
        };
        let status = {
            let mut ctx = crate::resolve::make_context(
                sink,
                Some(&card),
                crate::resolve::HookOptions {
                    controller: Some(card.controller),
                    data: Some(data),
                    ..crate::resolve::HookOptions::default()
                },
            );
            if let Some(resumed) = &resumed {
                if let Some(exits_from) = resumed.exits_from {
                    ctx.exits_from = Some(exits_from);
                }
                if let Some(summoned) = &resumed.summoned {
                    ctx.summoned = Some(summoned.clone());
                }
            }
            let effects = hook(&mut ctx);
            crate::prompts::run_resumable_list(&mut ctx, &plan, effects, resumed)
        };
        match status {
            crate::prompts::ListStatus::Done => {
                owed.holders.remove(0);
                crate::state_check::state_check(sink);
                if is_paused(sink) {
                    if sink.state.result.is_none() {
                        owe_ally_attacked(sink, &owed);
                    }
                    return false;
                }
            }
            // The hook's last effect asked: the hook is done, and its check waits for the answer.
            crate::prompts::ListStatus::Asked => {
                owe_ally_attacked(
                    sink,
                    &OwedAllyAttacked {
                        check_only: Some(true),
                        ..owed
                    },
                );
                return false;
            }
            // Parked: the rest of the hook went to `state.work` as this sequence's own item.
            crate::prompts::ListStatus::Parked | crate::prompts::ListStatus::Over => return false,
        }
    }
    true
}

fn owed_ally_attacked_of(data: &IndexMap<String, Value>) -> Option<OwedAllyAttacked> {
    let raw = data.get(ALLY_ATTACKED_KEY)?;
    let owed = raw.as_object()?;
    owed.get("declared")?.as_object()?;
    owed.get("holders")?.as_array()?;
    serde_json::from_value::<OwedAllyAttacked>(raw.clone()).ok()
}

/// `work.rs`'s handler: the hooks still owed, where the pause left them, then the combat (R113).
pub fn run_owed_ally_attacked(sink: &mut EngineSink<'_>, item: &WorkItem) {
    let Some(owed) = owed_ally_attacked_of(&item.resume.data) else {
        return;
    };
    let declared = owed.declared.clone();
    if !answer_ally_attacked(sink, owed, paused_of(&item.resume.data)) {
        return;
    }
    let since = declared
        .exits_from
        .unwrap_or_else(|| crate::stays::exit_mark(sink.state));
    let bounce = declared.bounce_after == Some(true);
    fight_declared(sink, declared, since, bounce);
}

// "After one of your Units attacks": `allyAttack` hooks (Training Grounds #31 Level Up!, R2199000)

/// R113: the `resume.hook` of the `allyAttack` hooks a combat still owes after its check and its
/// combatants' own hooks, from wherever a question stopped them.
pub const ALLY_ATTACK_WORK: &str = "@allyAttack";

/// The facts an `allyAttack` hook is handed in its `ctx.data` (`ally_attack_of` reads them back): the
/// attacker's id, the target, `forced` (R53) and whether the attacker still stands as the hook starts.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AllyAttackFacts {
    pub attacker_id: String,
    pub target_id: String,
    pub forced: bool,
    pub survived: bool,
}

/// The facts an `allyAttack` hook was handed, or `None` outside one.
pub fn ally_attack_of(ctx: &crate::script::EffectContext<'_>) -> Option<AllyAttackFacts> {
    let data = &ctx.data;
    Some(AllyAttackFacts {
        attacker_id: data.get("attackerId")?.as_str()?.to_string(),
        target_id: data.get("targetId")?.as_str()?.to_string(),
        forced: data.get("forced")?.as_bool()?,
        survived: data.get("survived")?.as_bool()?,
    })
}

/// What an owed `@allyAttack` carries, all JSON: the combat's facts and the holders still to answer
/// it in R68's order — the first is the one a pause stood in.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct OwedAllyAttack {
    attacker_id: String,
    target_id: String,
    forced: bool,
    /// The field's departures before the closing check, to tell a survivor from a Reborn body (R174).
    since: u32,
    holders: Vec<String>,
    /// The first holder's `survived`, judged once as its hook starts, so a pause re-reads the same.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    survived: Option<bool>,
    /// The first holder's hook is done; only its state check is owed. Only ever `Some(true)`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    check_only: Option<bool>,
}

const ALLY_ATTACK_KEY: &str = "allyAttack";

/// R2199000: the cards that answer a combat of `player`'s attacker — every card acting on that
/// player's side with an `allyAttack` hook, the attacker itself included, in R68's order.
fn ally_attack_holders(state: &GameState, player: PlayerId) -> Vec<String> {
    crate::damage::acting_texts_of(state, player)
        .into_iter()
        .filter(|card| crate::scripts::script_of(state, card).ally_attack.is_some())
        .map(|card| card.id)
        .collect()
}

fn owe_ally_attack(sink: &mut EngineSink<'_>, owed: &OwedAllyAttack) {
    let mut data = IndexMap::new();
    data.insert(ALLY_ATTACK_KEY.to_string(), to_json(owed));
    owe(sink, engine_resume(ALLY_ATTACK_WORK, "hook", data));
}

/// R2199000: the holders' hooks in turn, each a whole effect followed by its state check (R59), from
/// `paused` when a question split the first; one no longer acting is passed over (R113 for a pause).
fn answer_ally_attack(sink: &mut EngineSink<'_>, mut owed: OwedAllyAttack, mut paused: Option<PausedStep>) {
    if owed.check_only.take() == Some(true) && !owed.holders.is_empty() {
        owed.holders.remove(0);
        owed.survived = None;
        crate::state_check::state_check(sink);
        if is_paused(sink) {
            if sink.state.result.is_none() {
                owe_ally_attack(sink, &owed);
            }
            return;
        }
    }
    while let Some(id) = owed.holders.first().cloned() {
        if sink.state.result.is_some() {
            return;
        }
        let resumed = paused.take();
        let card = find_instance(sink.state, &id)
            .filter(|card| resumed.is_some() || is_active_on_field(sink.state, card))
            .cloned();
        let hook = card
            .as_ref()
            .and_then(|card| crate::scripts::script_of(sink.state, card).ally_attack.clone());
        let (Some(card), Some(hook)) = (card, hook) else {
            owed.holders.remove(0);
            owed.survived = None;
            continue;
        };
        let survived = *owed.survived.get_or_insert_with(|| {
            find_instance(sink.state, &owed.attacker_id).is_some_and(|live| {
                is_active_on_field(sink.state, live)
                    && !crate::stays::left_field_after(sink.state, owed.since, &live.id)
            })
        });
        let mut data: IndexMap<String, Value> = IndexMap::new();
        data.insert("attackerId".to_string(), json!(owed.attacker_id));
        data.insert("targetId".to_string(), json!(owed.target_id));
        data.insert("forced".to_string(), json!(owed.forced));
        data.insert("survived".to_string(), json!(survived));
        let mut plan_data = IndexMap::new();
        plan_data.insert(ALLY_ATTACK_KEY.to_string(), to_json(&owed));
        let plan = WorkPlan {
            def_id: String::new(),
            hook: ALLY_ATTACK_WORK.to_string(),
            step: "hook".to_string(),
            radiant: false,
            instance_id: None,
            data: plan_data,
            owner: card.controller,
        };
        let status = {
            let mut ctx = crate::resolve::make_context(
                sink,
                Some(&card),
                crate::resolve::HookOptions {
                    controller: Some(card.controller),
                    data: Some(data),
                    ..crate::resolve::HookOptions::default()
                },
            );
            if let Some(resumed) = &resumed {
                if let Some(exits_from) = resumed.exits_from {
                    ctx.exits_from = Some(exits_from);
                }
                if let Some(summoned) = &resumed.summoned {
                    ctx.summoned = Some(summoned.clone());
                }
            }
            let effects = hook(&mut ctx);
            crate::prompts::run_resumable_list(&mut ctx, &plan, effects, resumed)
        };
        match status {
            crate::prompts::ListStatus::Done => {
                owed.holders.remove(0);
                owed.survived = None;
                crate::state_check::state_check(sink);
                if is_paused(sink) {
                    if sink.state.result.is_none() {
                        owe_ally_attack(sink, &owed);
                    }
                    return;
                }
            }
            // The hook's last effect asked: the hook is done, and its check waits for the answer.
            crate::prompts::ListStatus::Asked => {
                owe_ally_attack(
                    sink,
                    &OwedAllyAttack {
                        check_only: Some(true),
                        ..owed
                    },
                );
                return;
            }
            // Parked: the rest of the hook went to `state.work` as this sequence's own item.
            crate::prompts::ListStatus::Parked | crate::prompts::ListStatus::Over => return,
        }
    }
}

fn owed_ally_attack_of(data: &IndexMap<String, Value>) -> Option<OwedAllyAttack> {
    let raw = data.get(ALLY_ATTACK_KEY)?;
    let owed = raw.as_object()?;
    owed.get("attackerId")?.as_str()?;
    owed.get("holders")?.as_array()?;
    serde_json::from_value::<OwedAllyAttack>(raw.clone()).ok()
}

/// `work.rs`'s handler: the hooks still owed, where the pause left them (R113).
pub fn run_owed_ally_attack(sink: &mut EngineSink<'_>, item: &WorkItem) {
    let Some(owed) = owed_ally_attack_of(&item.resume.data) else {
        return;
    };
    answer_ally_attack(sink, owed, paused_of(&item.resume.data));
}
