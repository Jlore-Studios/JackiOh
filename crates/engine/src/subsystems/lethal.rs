//! What a declared attack would actually do to the defending hero, for My Pawn's trap window
//! (SPEC §4.2 step 4, BUILD M3-T7, R44): "Lethal = projected damage to the hero after Armor and the
//! cap, Trample excess from an attack on a unit included, ≥ health".
//!
//! A projection: it walks §4.4's ordered steps through the pipeline's own readers, never the state.

use crate::combat::AttackTarget;
use crate::damage::DamageTarget;
use crate::layers::unit_view;
use crate::state::{CardInstance, GameState};
use crate::wire::{KeywordKind, PlayerId, armor_of, has_keyword};

/// The hero a hit on this target can reach: itself, or through Trample its controller's (§4.4 step 9).
pub fn defending_hero(target: &AttackTarget) -> PlayerId {
    match target {
        DamageTarget::Hero { player } => *player,
        DamageTarget::Unit { instance } => instance.controller,
    }
}

/// §4.4 on a hero: step 2 subtracts its Armor, backrow grants included (#84, R124: or My Pawn would call
/// lethal an attack Going Long blunts), unless `pierce` (R346); step 3 clamps to the Anti-oneshot cap,
/// and the zero rule stops anything left at 0 (R63).
pub fn projected_hero_damage(state: &GameState, player: PlayerId, amount: i32, pierce: bool) -> i32 {
    // B5 E6: the pipeline's own reading of steps 2 and 3, the divisors between them included.
    crate::damage::hero_hit_amount(state, player, amount, pierce)
}

/// R346: a striking unit with Pierce skips §4.4 step 2 on every hit it makes.
fn piercing(state: &GameState, unit: &CardInstance) -> bool {
    crate::damage::pierces(state, Some(unit), None)
}

/// §4.4 step 9: what a Trample source's hit on a unit passes on to that unit's hero; steps 1, 2, 4 and
/// the zero rule can end the hit on the unit first. R176: every hit of the attack counts, Cleave's
/// (step 10) included, as R63 makes Cleave the attack's and R44 counts "Trample excess from an attack".
fn trample_excess(state: &GameState, source: &CardInstance, unit: &CardInstance, amount: i32) -> i32 {
    if amount <= 0 {
        return 0;
    }
    let view = unit_view(state, unit);

    // Step 1: Divine Shield negates the whole hit.
    if has_keyword(&view.keywords, KeywordKind::DivineShield) && unit.divine_shield_spent != Some(true) {
        return 0;
    }
    // Step 4: an Indestructible unit takes nothing at all.
    if has_keyword(&view.keywords, KeywordKind::Indestructible) {
        return 0;
    }

    // Step 2, then the zero rule. A Pierce source skips step 2 (R346).
    let after_armor = if piercing(state, source) {
        amount
    } else {
        (amount - armor_of(&view.keywords)).max(0)
    };
    // Step 3: the unit's own cap (R2179005), before step 5 splits off the excess.
    let after_armor =
        crate::damage::unit_damage_cap(state, unit).map_or(after_armor, |cap| after_armor.min(cap));
    if after_armor <= 0 {
        return 0;
    }

    if !has_keyword(&unit_view(state, source).keywords, KeywordKind::Trample) {
        return 0;
    }
    if after_armor <= view.health {
        return 0;
    }
    // Step 5: only what lands beyond the unit's health becomes the step 9 instance (R63).
    after_armor - view.health.max(0)
}

/// The damage a declared attack would deal to the defending hero (R44), assumed legal: §4.2 steps 1
/// to 3 have passed when the trap window opens.
pub fn projected_damage(state: &GameState, attacker: &CardInstance, target: &AttackTarget) -> i32 {
    // MD-D4, R1120: a combat-only bonus on the attacker's strike at a Unit target is dealt there, so
    // the projection reads it through the same function the combat does.
    let attack = match target {
        DamageTarget::Hero { .. } => unit_view(state, attacker).attack,
        DamageTarget::Unit { instance } => {
            unit_view(state, attacker).attack
                + crate::combat::attack_mod_for(state, attacker, instance).attack
        }
    };
    let hero = defending_hero(target);
    let pierce = piercing(state, attacker);
    let defender = match target {
        DamageTarget::Hero { .. } => return projected_hero_damage(state, hero, attack, pierce),
        DamageTarget::Unit { instance } => instance,
    };
    // R176, §4.3 step 1: an attacker a First Strike defender destroys first deals nothing, Cleave
    // included. R783000: a Ranged attacker takes no strike back from a defender that is not.
    if crate::combat::takes_strike_back(state, attacker, defender)
        && falls_to_first_strike(state, attacker, defender)
    {
        return 0;
    }
    let hit = attack * crate::combat::lane_multiplier_for(state, attacker, defender);
    // Each hit is its own damage instance on the hero, so Armor and the cap apply to each (§4.4).
    struck_by(state, attacker, defender)
        .iter()
        .fold(0, |total, unit| {
            let amount = if unit.id == defender.id { hit } else { attack };
            total + projected_hero_damage(state, hero, trample_excess(state, attacker, unit, amount), pierce)
        })
}

/// The units one attack on `defender` strikes: the defender, then, with Cleave, each unit adjacent to
/// it on its own side (§4.4 step 10, §3.1), never one dormant under a Stack (R13).
fn struck_by(state: &GameState, attacker: &CardInstance, defender: &CardInstance) -> Vec<CardInstance> {
    if !has_keyword(&unit_view(state, attacker).keywords, KeywordKind::Cleave) {
        return vec![defender.clone()];
    }
    let Some(at) = crate::zones::slot_of(state, defender) else {
        return vec![defender.clone()];
    };
    let mut struck = vec![defender.clone()];
    for slot in crate::zones::adjacent(at) {
        if let Some(card) = crate::zones::card_at(state, slot)
            && crate::zones::is_adjacent_card(state, defender, card)
        {
            struck.push(card.clone());
        }
    }
    struck
}

/// §4.3 step 1 and R93, projected: a defender with First Strike, against an attacker without it,
/// strikes first, and an attacker that falls there never lands its own hit. §4.4's stops apply to the
/// strike back, and a Poisonous defender destroys with any damage at all (step 7).
fn falls_to_first_strike(state: &GameState, attacker: &CardInstance, defender: &CardInstance) -> bool {
    let mine = unit_view(state, attacker);
    let theirs = unit_view(state, defender);
    if !has_keyword(&theirs.keywords, KeywordKind::FirstStrike)
        || has_keyword(&mine.keywords, KeywordKind::FirstStrike)
    {
        return false;
    }
    let theirs_attack = theirs.attack * crate::combat::lane_multiplier_for(state, defender, attacker);
    if theirs_attack <= 0 {
        return false;
    }

    if has_keyword(&mine.keywords, KeywordKind::DivineShield) && attacker.divine_shield_spent != Some(true) {
        return false;
    }
    if has_keyword(&mine.keywords, KeywordKind::Indestructible) {
        return false;
    }
    let dealt = if piercing(state, defender) {
        theirs_attack
    } else {
        (theirs_attack - armor_of(&mine.keywords)).max(0)
    };
    if dealt <= 0 {
        return false;
    }
    if has_keyword(&theirs.keywords, KeywordKind::Poisonous) {
        return true;
    }
    dealt >= mine.health
}

/// §4.3, §4.4 step 8: what the defender's Lifesteal strike back heals the hero the attack's Trample
/// reaches: nothing for a hero target, a defender a First Strike attacker destroys first (§4.3 step 1),
/// one a Ranged attacker takes nothing back from (R783000), or a hit Divine Shield, Indestructible or
/// Armor stops. A strike back never tramples (R1750000 over R176), so all it deals heals.
fn strike_back_heal(state: &GameState, attacker: &CardInstance, target: &AttackTarget) -> i32 {
    let defender = match target {
        DamageTarget::Unit { instance } => instance,
        DamageTarget::Hero { .. } => return 0,
    };
    // R783000: no strike back at all, so nothing for its Lifesteal to heal.
    if !crate::combat::takes_strike_back(state, attacker, defender) {
        return 0;
    }
    let theirs = unit_view(state, defender);
    if !has_keyword(&theirs.keywords, KeywordKind::Lifesteal) || theirs.attack <= 0 {
        return 0;
    }
    if falls_to_first_strike(state, defender, attacker) {
        return 0;
    }
    let mine = unit_view(state, attacker);
    if has_keyword(&mine.keywords, KeywordKind::DivineShield) && attacker.divine_shield_spent != Some(true) {
        return 0;
    }
    if has_keyword(&mine.keywords, KeywordKind::Indestructible) {
        return 0;
    }
    let pierce = piercing(state, defender);
    if pierce {
        theirs.attack
    } else {
        (theirs.attack - armor_of(&mine.keywords)).max(0)
    }
}

/// R44: the projection against the defending hero's health at §4.5 step 2's check after the combat, net
/// of what a defender's Lifesteal strike back heals it in the same simultaneous step (R176). #936 B17: a
/// Duos hero stands until its HP and its team's pool are both gone. R1168003: a First Strike defender
/// heals before the hit lands, so a spent Duos hero's pool takes it only up to its cap (B14).
pub fn is_lethal(state: &GameState, attacker: &CardInstance, target: &AttackTarget) -> bool {
    let hero = defending_hero(target);
    let mut heal = strike_back_heal(state, attacker, target);
    if heals_first(state, attacker, target) {
        heal = crate::duos_life::hero_heal_room(state, hero, heal);
    }
    let net = projected_damage(state, attacker, target) - heal;
    // R2201000: no hit is lethal to a hero that cannot lose.
    crate::state::can_lose(state, hero) && net >= crate::duos_life::effective_life(state, hero)
}

/// §4.3 step 1: a defender with First Strike, against an attacker without it, strikes first, so its
/// Lifesteal heals before the attacker's own hit.
fn heals_first(state: &GameState, attacker: &CardInstance, target: &AttackTarget) -> bool {
    let DamageTarget::Unit { instance: defender } = target else {
        return false;
    };
    has_keyword(&unit_view(state, defender).keywords, KeywordKind::FirstStrike)
        && !has_keyword(&unit_view(state, attacker).keywords, KeywordKind::FirstStrike)
}
