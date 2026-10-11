//! ME-RANDOMTARGETS (Meditative #86 Mayor Medinamogger): while a Mayor acts on the field,
//! both players' declared targets, `target` prompts and attack targets are drawn at random from
//! the legal ones (R1200; MD-E9). The Radiant face's Lucky 1 rolls its controller's draws again
//! and keeps the better (R1201; MD-E10).
//!
//! Every draw goes through the match rng, so a replay draws the same targets, and luck 0 draws
//! exactly once, so a game with no Lucky Mayor in it replays byte for byte as before (D14).

use crate::combat::AttackTarget;
use crate::damage::DamageTarget;
use crate::rng::Rng;
use crate::state::GameState;
use crate::wire::{KeywordKind, PLAYER_IDS, PlayerId, Selection, TargetAim};

/// R1200: whether a Mayor acts on the field — a card in either player's active units whose running
/// face carries `random_targets`.
pub fn targets_random(state: &GameState) -> bool {
    PLAYER_IDS.into_iter().any(|player| {
        crate::zones::active_units_of(state, player)
            .into_iter()
            .any(|card| crate::scripts::flags_of(state, card).random_targets == Some(true))
    })
}

/// R1201: the Lucky of `chooser`'s acting Mayors, summed — the extra rolls their controller's
/// random targets take. The opponent's rolls take none.
pub fn mayor_luck(state: &GameState, chooser: PlayerId) -> i32 {
    crate::zones::active_units_of(state, chooser)
        .into_iter()
        .filter(|card| crate::scripts::flags_of(state, card).random_targets == Some(true))
        .filter_map(|card| {
            crate::tuning::numbered_sum(
                &crate::layers::unit_view(state, card).keywords,
                KeywordKind::Lucky,
            )
        })
        .sum::<i32>()
        .max(0)
}

/// How many of these picks are on the aim's preferred side (R656's aim: an enemy for harm, a
/// friend for help), as `rng.lucky`'s ranking reads them.
fn aimed_count(state: &GameState, chooser: PlayerId, aim: TargetAim, picks: &[Selection]) -> usize {
    picks
        .iter()
        .filter(|pick| match aim {
            TargetAim::Harm => crate::random_cast::is_enemy_pick(state, chooser, pick),
            TargetAim::Help => crate::random_cast::is_friendly_pick(state, chooser, pick),
        })
        .count()
}

/// R1200, R1201: draw a declaration's picks at random — `random_picks`' uniform set — rolling
/// again for the chooser's Lucky and keeping the later roll only when it has strictly more picks
/// on the aim's side. Luck 0 rolls once.
pub fn draw_picks(
    state: &GameState,
    rng: &mut Rng,
    chooser: PlayerId,
    options: &[Selection],
    low: i32,
    high: i32,
    aim: TargetAim,
) -> Vec<Selection> {
    let luck = mayor_luck(state, chooser);
    rng.lucky(
        luck,
        |rng| crate::random_cast::random_picks(rng, options, low, high),
        |first, second| {
            if aimed_count(state, chooser, aim, &second) > aimed_count(state, chooser, aim, &first) {
                second
            } else {
                first
            }
        },
    )
}

/// R1201: how an attack target ranks for `attacker` — first one whose strike back the attacker
/// survives, then one the attacker destroys (MD-E10's order after the aim's side, which every
/// legal attack target already is: an enemy).
fn attack_score(
    state: &GameState,
    attacker: &crate::state::CardInstance,
    target: &AttackTarget,
) -> (bool, bool) {
    let view = crate::layers::unit_view(state, attacker);
    match target {
        DamageTarget::Hero { player } => {
            let health = state.players[*player].hero.health;
            (true, view.attack >= health)
        }
        DamageTarget::Unit { instance } => {
            let defender = crate::layers::unit_view(state, instance);
            let survives = defender.attack < view.health;
            let destroys = view.attack >= defender.health;
            (survives, destroys)
        }
    }
}

/// R1200, R1201: draw an attack's target from the ones the attacker may legally attack (§4.2
/// steps 2 and 3: Taunt, lane restrictions, Rush on its arrival turn — `attack_targets` already
/// holds them). An empty list returns `None` and draws nothing (R129: a fizzle takes no
/// randomness). The chooser's Lucky rolls again, ranking by stats alone: surviving the strike
/// back first, then destroying.
pub fn draw_attack_target(
    state: &GameState,
    rng: &mut Rng,
    attacker: &crate::state::CardInstance,
) -> Option<AttackTarget> {
    let options = crate::combat::attack_targets(state, attacker);
    if options.is_empty() {
        return None;
    }
    let luck = mayor_luck(state, attacker.controller);
    let at = rng.lucky(
        luck,
        |rng| rng.int(options.len() as i32),
        |first, second| {
            let rank = |at: i32| {
                usize::try_from(at)
                    .ok()
                    .and_then(|at| options.get(at))
                    .map(|target| attack_score(state, attacker, target))
                    .unwrap_or((false, false))
            };
            if rank(second) > rank(first) { second } else { first }
        },
    );
    usize::try_from(at).ok().and_then(|at| options.get(at)).cloned()
}
