//! What a declared attack would actually do to the defending hero, for My Pawn's trap window
//! (SPEC §4.2 step 4, BUILD M3-T7, R44): "Lethal = projected damage to the hero after Armor and the
//! cap, Trample excess from an attack on a unit included, ≥ health".
//!
//! This is a projection, so it never touches the state: it walks the ordered steps of §4.4 reading
//! the same sources the pipeline reads (`unitView` for the layers, `heroDamageCap` for the
//! Anti-oneshot clamp) instead of dealing the damage and looking at the result.
//!
//! Port of `packages/engine/src/subsystems/lethal.ts`.

use crate::combat::AttackTarget;
use crate::damage::DamageTarget;
use crate::layers::unit_view;
use crate::state::{CardInstance, GameState};
use crate::wire::{KeywordKind, PlayerId, armor_of, has_keyword};

/// The hero a hit on this target can reach: the hero itself, or, through Trample, the target unit's
/// controller (§4.4 step 9 sends the excess to "the target's controller's hero").
pub fn defending_hero(target: &AttackTarget) -> PlayerId {
    match target {
        DamageTarget::Hero { player } => *player,
        DamageTarget::Unit { instance } => instance.controller,
    }
}

/// §4.4 on a hero: step 2 subtracts the hero's Armor, step 3 clamps to the Anti-oneshot cap, and
/// the zero rule stops anything left at 0 (R63). A hit from a source with Pierce skips step 2
/// (R346), so `pierce` is the striking unit's keyword, which `piercing` below reads; every other
/// hit a declared attack makes pays step 2.
///
/// The Armor is `heroArmorOf`, the pipeline's own reader: the hero's stored Armor plus every backrow
/// grant (#84), summed per R124. Reading the bare field instead would make My Pawn (R44) call an
/// attack lethal that Going Long is about to blunt.
pub fn projected_hero_damage(state: &GameState, player: PlayerId, amount: i32, pierce: bool) -> i32 {
    // B5 E6: the pipeline's own reading of steps 2 and 3, the divisors between them included.
    crate::damage::hero_hit_amount(state, player, amount, pierce)
}

/// R346: a striking unit with Pierce skips §4.4 step 2 on every hit it makes, the pipeline's own reading.
fn piercing(state: &GameState, unit: &CardInstance) -> bool {
    crate::damage::pierces(state, Some(unit), None)
}

/// §4.4 step 9: what a Trample source's hit on a unit passes on to that unit's hero. Steps 1, 2, 4
/// and the zero rule can all end the hit on the unit, and then nothing tramples through.
///
/// R176: every hit of the attack counts, Cleave's (step 10) included — R63 makes Cleave belong to
/// the attack rather than to the hit on the defender, and R44 counts "Trample excess from an attack
/// on a unit". `projectedDamage` below asks this once per unit the attack strikes.
fn trample_excess(state: &GameState, source: &CardInstance, unit: &CardInstance, amount: i32) -> i32 {
    if amount <= 0 {
        return 0;
    }
    let view = unit_view(state, unit);

    // Step 1: Divine Shield negates the whole hit, so there is no excess.
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

/// The damage a declared attack by `attacker` on `target` would deal to the defending hero, after
/// Armor and the Anti-oneshot cap, Trample excess from an attack on a unit included (R44).
/// The attack is assumed legal: §4.2 steps 1 to 3 have already passed when the trap window opens.
pub fn projected_damage(state: &GameState, attacker: &CardInstance, target: &AttackTarget) -> i32 {
    let attack = unit_view(state, attacker).attack;
    let hero = defending_hero(target);
    let pierce = piercing(state, attacker);
    let defender = match target {
        DamageTarget::Hero { .. } => return projected_hero_damage(state, hero, attack, pierce),
        DamageTarget::Unit { instance } => instance,
    };
    // R176, §4.3 step 1: an attacker a First Strike defender destroys first deals nothing at all —
    // no hit on the defender and so no Cleave either — so nothing of it can reach the hero.
    if falls_to_first_strike(state, attacker, defender) {
        return 0;
    }
    // Each hit is its own damage instance on the hero, so Armor and the cap apply to each (§4.4).
    struck_by(state, attacker, defender).iter().fold(0, |total, unit| {
        total + projected_hero_damage(state, hero, trample_excess(state, attacker, unit, attack), pierce)
    })
}

/// The units one attack on `defender` strikes: the defender, then — §4.4 step 10 — each unit
/// adjacent to it on its own side when the attacker has Cleave. Adjacency never crosses sides (§3.1)
/// and a card dormant under a Stack is not struck (R13).
fn struck_by(state: &GameState, attacker: &CardInstance, defender: &CardInstance) -> Vec<CardInstance> {
    if !has_keyword(&unit_view(state, attacker).keywords, KeywordKind::Cleave) {
        return vec![defender.clone()];
    }
    let Some(at) = crate::zones::slot_of(state, defender) else {
        return vec![defender.clone()];
    };
    let mut struck = vec![defender.clone()];
    for slot in crate::zones::adjacent(at) {
        if let Some(card) = crate::zones::card_at(state, slot) {
            struck.push(card.clone());
        }
    }
    struck
}

/// §4.3 step 1 and R93, projected: a defender with First Strike, against an attacker without it,
/// strikes first, and an attacker that falls there never lands its own hit. The strike back runs the
/// §4.4 steps that can stop it — Divine Shield, Armor, Indestructible and the zero rule — and a
/// Poisonous defender destroys with any damage at all (step 7).
fn falls_to_first_strike(state: &GameState, attacker: &CardInstance, defender: &CardInstance) -> bool {
    let mine = unit_view(state, attacker);
    let theirs = unit_view(state, defender);
    if !has_keyword(&theirs.keywords, KeywordKind::FirstStrike)
        || has_keyword(&mine.keywords, KeywordKind::FirstStrike)
    {
        return false;
    }
    if theirs.attack <= 0 {
        return false;
    }

    if has_keyword(&mine.keywords, KeywordKind::DivineShield) && attacker.divine_shield_spent != Some(true) {
        return false;
    }
    if has_keyword(&mine.keywords, KeywordKind::Indestructible) {
        return false;
    }
    let dealt = if piercing(state, defender) {
        theirs.attack
    } else {
        (theirs.attack - armor_of(&mine.keywords)).max(0)
    };
    if dealt <= 0 {
        return false;
    }
    if has_keyword(&theirs.keywords, KeywordKind::Poisonous) {
        return true;
    }
    dealt >= mine.health
}

/// §4.3, §4.4 step 8: what the defender's strike back heals its own hero in the same combat — its
/// Lifesteal heals its controller by the damage it deals the attacker, and the hero it heals is the
/// one the attack's Trample reaches. Nothing strikes back for a hero, a defender an attacker with First
/// Strike destroys first (§4.3 step 1), or a hit the attacker's Divine Shield, Indestructible or Armor
/// stops.
///
/// Lifesteal heals the damage actually dealt, which is the hit as §4.4 splits it. A strike back with
/// Trample counts on the attacker only up to its health (step 5, R63), and the rest is its own
/// instance on the attacking hero, through that hero's Armor and cap (step 9) — so Going Long's Armor
/// can stop it at 0, and a 0 heals nothing. Counting the whole strike as healed called a swing
/// survivable that leaves the defending hero at 0.
fn strike_back_heal(state: &GameState, attacker: &CardInstance, target: &AttackTarget) -> i32 {
    let defender = match target {
        DamageTarget::Unit { instance } => instance,
        DamageTarget::Hero { .. } => return 0,
    };
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
    let strike = if pierce {
        theirs.attack
    } else {
        (theirs.attack - armor_of(&mine.keywords)).max(0)
    };
    if strike <= 0 || !has_keyword(&theirs.keywords, KeywordKind::Trample) || strike <= mine.health {
        return strike;
    }
    let on_unit = mine.health.max(0);
    on_unit + projected_hero_damage(state, attacker.controller, strike - on_unit, pierce)
}

/// R44: the projection against the defending hero's health. §4.5 step 2 ends the game at 0 or less
/// at the check after the combat, so "≥ health" is exactly the hit that would end it — net of what
/// the same combat gives back: R176, a defender's Lifesteal strike back heals the hero in the same
/// simultaneous step, so a Trample swing it outheals leaves the hero standing at that check.
pub fn is_lethal(state: &GameState, attacker: &CardInstance, target: &AttackTarget) -> bool {
    let hero = defending_hero(target);
    let net = projected_damage(state, attacker, target) - strike_back_heal(state, attacker, target);
    net >= state.players[hero].hero.health
}
