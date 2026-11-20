//! What a declared attack would actually do to the defending hero, for My Pawn's trap window
//! (SPEC §4.2 step 4, BUILD M3-T7, R44): "Lethal = projected damage to the hero after Armor and the
//! cap, Trample excess from an attack on a unit included, ≥ health".
//!
//! A projection: it walks §4.4's ordered steps through the pipeline's own readers, never the state.
//! Every hit's steps 1 to 4 are `damage::project_hit`'s, a conversion and the damage pool included
//! (R4032000), so one attack's hits spend a pool once between them.

use crate::combat::AttackTarget;
use crate::damage::{DamageTarget, ProjectedPools, project_hit};
use crate::layers::unit_view;
use crate::state::{CardInstance, GameState};
use crate::wire::{KeywordKind, PlayerId, has_keyword, opponent_of};

/// The hero a hit on this target can reach: itself, or through Trample its controller's (§4.4 step 9).
pub fn defending_hero(target: &AttackTarget) -> PlayerId {
    match target {
        DamageTarget::Hero { player } => *player,
        DamageTarget::Unit { instance } => instance.controller,
    }
}

/// §4.4 on a hero: step 2 subtracts its Armor, backrow grants included (#84, R124: or My Pawn would call
/// lethal an attack Going Long blunts), unless `pierce` (R346); step 3 clamps to the Anti-oneshot cap,
/// and the zero rule stops anything left at 0 (R63). Sourceless and pool-blind, as the AI reads a Burn
/// or Toxic tick (R3822000); an attack's projection is `damage::project_hit` (R4032000).
pub fn projected_hero_damage(state: &GameState, player: PlayerId, amount: i32, pierce: bool) -> i32 {
    // B5 E6: the pipeline's own reading of steps 2 and 3, the divisors between them included.
    crate::damage::hero_hit_amount(state, player, amount, pierce)
}

/// §4.4 steps 5 and 9: what a hit of `amount` from `source` deals `unit`, and what it passes on to that
/// unit's hero; steps 1 to 4 and the zero rule, as `damage::project_hit` reads them (a conversion and the
/// unit's side's damage pool included, R4032000), can end the hit on the unit first. Only a Trample
/// source passes anything on. R176: every hit of the attack counts, Cleave's (step 10) included, as R63
/// makes Cleave the attack's and R44 counts "Trample excess from an attack".
fn hit_and_excess(
    state: &GameState,
    source: &CardInstance,
    unit: &CardInstance,
    amount: i32,
    pools: &mut ProjectedPools,
) -> (i32, i32) {
    let target = DamageTarget::Unit {
        instance: unit.clone(),
    };
    let after = project_hit(state, Some(source), &target, amount, pools);
    if after <= 0 || !has_keyword(&unit_view(state, source).keywords, KeywordKind::Trample) {
        return (after, 0);
    }
    let health = unit_view(state, unit).health.max(0);
    if after <= health {
        return (after, 0);
    }
    // Step 5: only what lands beyond the unit's health becomes the step 9 instance (R63).
    (health, after - health)
}

/// One unit an attack strikes (`struck_units`): what its hit deals it, and what that hit's Trample
/// excess deals its hero as its own instance (§4.4 step 9).
pub(crate) struct StruckUnit {
    pub unit: CardInstance,
    pub dealt: i32,
    pub onto_hero: i32,
}

/// R4290000: the units an attack of `attack` (its combat-only bonus included) on `defender` strikes, in
/// `combat::strike`'s order, the blow first, then each Cleave hit (§4.4 step 10), each with what it
/// deals and what its Trample excess deals the hero. The blow takes the lane's multiplier, a Cleave hit
/// the bare attack. The hits spend the damage pools in that order (R4032000), as the combat does.
pub(crate) fn struck_units(
    state: &GameState,
    attacker: &CardInstance,
    defender: &CardInstance,
    attack: i32,
    pools: &mut ProjectedPools,
) -> Vec<StruckUnit> {
    let hit = attack * crate::combat::lane_multiplier_for(state, attacker, defender);
    let on_hero = DamageTarget::Hero {
        player: defender.controller,
    };
    crate::query::cleave_struck(state, attacker, defender)
        .into_iter()
        .map(|unit| {
            let amount = if unit.id == defender.id { hit } else { attack };
            let (dealt, excess) = hit_and_excess(state, attacker, &unit, amount, pools);
            let onto_hero = project_hit(state, Some(attacker), &on_hero, excess, pools);
            StruckUnit {
                unit,
                dealt,
                onto_hero,
            }
        })
        .collect()
}

/// The damage a declared attack would deal to the defending hero (R44), assumed legal: §4.2 steps 1
/// to 3 have passed when the trap window opens.
pub fn projected_damage(state: &GameState, attacker: &CardInstance, target: &AttackTarget) -> i32 {
    projected_damage_with(state, attacker, target, &mut ProjectedPools::default())
}

/// `projected_damage`, its hits spending the damage pools past what `pools` says they have soaked
/// already (R4032000), as the combat forecast's lethal sum adds one attacker's after another's.
pub(crate) fn projected_damage_with(
    state: &GameState,
    attacker: &CardInstance,
    target: &AttackTarget,
    pools: &mut ProjectedPools,
) -> i32 {
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
    let on_hero = DamageTarget::Hero { player: hero };
    let defender = match target {
        // R3554008, R4221003: the blow, then the sweep's hits on the enemy Units, each with its excess.
        DamageTarget::Hero { .. } => {
            let blow = project_hit(state, Some(attacker), target, attack, pools);
            return blow + swept_excess(state, attacker, None, attack, &on_hero, pools);
        }
        DamageTarget::Unit { instance } => instance,
    };
    // R176, §4.3 step 1: an attacker a First Strike defender destroys first deals nothing, Cleave
    // included. R783000: a Ranged attacker takes no strike back from a defender that is not.
    if crate::combat::takes_strike_back(state, attacker, defender)
        && falls_to_first_strike(
            state,
            attacker,
            defender,
            crate::combat::strike_back_multiplier_for(state, defender, attacker),
        )
    {
        return 0;
    }
    // Each hit is its own damage instance on the hero, so Armor and the cap apply to each (§4.4), in
    // `combat::strike`'s order: the blow and its excess, then each Cleave hit and its excess, then each
    // of the sweep's (R3554008, R4221003), so the pools soak the first of them (R4032000).
    let struck: i32 = struck_units(state, attacker, defender, attack, pools)
        .iter()
        .map(|unit| unit.onto_hero)
        .sum();
    struck + swept_excess(state, attacker, Some(defender), attack, &on_hero, pools)
}

/// R3554008, R4221003: what a sweep's other hits (`combat::sweep`) pass on to the hero: every enemy
/// Unit the sweep weighs but `struck`, the declared target, takes `attack` times its weight as one
/// instance, and a Trample attacker's excess over each goes on (§4.4 step 9), as Cleave's does (R176).
/// Each hit is read on the board as it stands, so a Unit Cleave struck too is read at its health before.
fn swept_excess(
    state: &GameState,
    attacker: &CardInstance,
    struck: Option<&CardInstance>,
    attack: i32,
    on_hero: &DamageTarget,
    pools: &mut ProjectedPools,
) -> i32 {
    if crate::scripts::flags_of(state, attacker).sweep_lane.is_none() {
        return 0;
    }
    let mut total = 0;
    for unit in crate::zones::active_units_of(state, opponent_of(attacker.controller)) {
        if struck.is_some_and(|struck| struck.id == unit.id) {
            continue;
        }
        let Some(weight) = crate::combat::sweep_weight(state, attacker, unit) else {
            continue;
        };
        let (_, excess) = hit_and_excess(state, attacker, unit, attack * weight, pools);
        total += project_hit(state, Some(attacker), on_hero, excess, pools);
    }
    total
}

/// §4.3 step 1 and R93, projected: a defender with First Strike, against an attacker without it,
/// strikes first, and an attacker that falls there never lands its own hit. §4.4's stops apply to the
/// strike back as `damage::project_hit` reads them, the attacker's own cap (R2179005), a conversion and
/// the attacker's side's damage pool included (R4032000), and a Poisonous defender destroys with any
/// damage at all (step 7). `multiplier` is the one on `defender`'s hit: `combat::strike_back_multiplier_for`
/// when it strikes back (R4156000), `combat::lane_multiplier_for` when it is the attacker
/// (`strike_back_heal`, sides swapped).
pub(crate) fn falls_to_first_strike(
    state: &GameState,
    attacker: &CardInstance,
    defender: &CardInstance,
    multiplier: i32,
) -> bool {
    let mine = unit_view(state, attacker);
    let theirs = unit_view(state, defender);
    if !has_keyword(&theirs.keywords, KeywordKind::FirstStrike)
        || has_keyword(&mine.keywords, KeywordKind::FirstStrike)
    {
        return false;
    }
    let theirs_attack = theirs.attack * multiplier;
    if theirs_attack <= 0 {
        return false;
    }
    let on_attacker = DamageTarget::Unit {
        instance: attacker.clone(),
    };
    // Step 3 reads the attacker's own cap on the hit (R2179005), as the strike back's damage does.
    let dealt = project_hit(
        state,
        Some(defender),
        &on_attacker,
        theirs_attack,
        &mut ProjectedPools::default(),
    );
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
/// one a Ranged attacker takes nothing back from (R783000), or a hit `damage::project_hit` stops
/// (Divine Shield, Armor, a cap, a conversion, the attacker's side's damage pool or Indestructible,
/// R4032000). A strike back never tramples (R1750000 over R176), so all it deals heals.
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
    if falls_to_first_strike(
        state,
        defender,
        attacker,
        crate::combat::lane_multiplier_for(state, attacker, defender),
    ) {
        return 0;
    }
    let on_attacker = DamageTarget::Unit {
        instance: attacker.clone(),
    };
    project_hit(
        state,
        Some(defender),
        &on_attacker,
        theirs.attack,
        &mut ProjectedPools::default(),
    )
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
