//! A Duos seat's life (#936 S3, B12–B18): its own HP, then its team's shared pool. Every write to a
//! hero's health goes through here — `damage.rs`'s deal, heal, lose and set, and `effects/swap.rs`'s
//! health swap — and so does §4.4 step 4a's lethal judge (B17, `effective_life`).
//!
//! - B13: damage and life loss take Armor first, as ever (the pipeline's step 2), then own HP, then
//!   the pool. Own HP that reaches 0 or less is spent for the rest of the game, what the hit had left
//!   over comes off the pool, and the hero's `health` reads the pool from then on — so §4.5's hero
//!   death (`state_check`) is the pool running out, with no rule of its own.
//! - B14: while spent, damage and life loss come off the pool and healing goes to the pool, never
//!   above its cap; while not spent, healing works as today, without a cap.
//! - B15 is the wrapper's: after every step `duos::reduce` copies each team's pool to the team's
//!   seat in the other duel.
//!
//! Without `PlayerState.duos` — every 1v1 game — each helper is exactly the arithmetic it replaced,
//! so no 1v1 state, hash, golden trace or fuzz seed moves (B3).

use crate::state::{GameState, PlayerState};
use crate::wire::PlayerId;

/// Damage and life loss on `player`'s hero, after Armor (B13, B14).
pub fn hero_loses(state: &mut GameState, player: PlayerId, amount: i32) {
    side_loses(&mut state.players[player], amount);
}

/// A heal on `player`'s hero: what it healed (B14). Uncapped unless the hero is spent.
pub fn hero_heals(state: &mut GameState, player: PlayerId, amount: i32) -> i32 {
    side_heals(&mut state.players[player], amount)
}

/// E7's "set a hero's health to N" on `player`'s hero (B13, B14).
pub fn hero_set(state: &mut GameState, player: PlayerId, value: i32) {
    side_set(&mut state.players[player], value);
}

/// What it would take to bring `player` to 0: health, plus the pool while own HP is not spent (once
/// spent, `health` reads the pool). §4.4 step 4a and R44's projection judge a hit against this (B17).
pub fn effective_life(state: &GameState, player: PlayerId) -> i32 {
    let side = &state.players[player];
    match side.duos {
        Some(life) if !life.spent => side.hero.health + life.pool,
        _ => side.hero.health,
    }
}

/// R73's health swap (B18): the two heroes' health values change places — in a Duos duel their own
/// HP, and only while neither hero is spent. Answers whether anything moved.
pub fn heroes_swap_health(state: &mut GameState, one: PlayerId, other: PlayerId) -> bool {
    let spent = |side: &PlayerState| side.duos.is_some_and(|life| life.spent);
    if spent(&state.players[one]) || spent(&state.players[other]) {
        return false;
    }
    let keep = state.players[one].hero.health;
    state.players[one].hero.health = state.players[other].hero.health;
    state.players[other].hero.health = keep;
    true
}

fn side_loses(side: &mut PlayerState, amount: i32) {
    let Some(life) = side.duos.as_mut() else {
        side.hero.health -= amount;
        return;
    };
    if amount <= 0 {
        return;
    }
    if life.spent {
        life.pool -= amount;
        side.hero.health = life.pool;
        return;
    }
    side.hero.health -= amount;
    if side.hero.health > 0 {
        return;
    }
    // B13: own HP at 0 or less is spent for the game, and the rest of the hit comes off the pool.
    let overflow = -side.hero.health;
    life.spent = true;
    life.pool -= overflow;
    side.hero.health = life.pool;
}

fn side_heals(side: &mut PlayerState, amount: i32) -> i32 {
    match side.duos.as_mut() {
        Some(life) if life.spent => {
            // B14: a spent hero heals its team's pool, never past the pool's cap.
            let healed = amount.min(life.pool_cap - life.pool).max(0);
            life.pool += healed;
            side.hero.health = life.pool;
            healed
        }
        _ => {
            side.hero.health += amount;
            amount
        }
    }
}

fn side_set(side: &mut PlayerState, value: i32) {
    let Some(life) = side.duos.as_mut() else {
        side.hero.health = value;
        return;
    };
    if life.spent {
        // While spent the hero's health is the pool, so setting it sets the pool, within its cap; a
        // value of 0 or less empties it (B16 then ends the game).
        life.pool = value.min(life.pool_cap);
        side.hero.health = life.pool;
        return;
    }
    side.hero.health = value;
    if value <= 0 {
        // B13: own HP set to 0 or less is spent, and what it went below 0 by comes off the pool.
        life.spent = true;
        life.pool += value;
        side.hero.health = life.pool;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{DuosLife, create_player_state};

    fn duos_side(own: i32, pool: i32) -> PlayerState {
        let mut side = create_player_state();
        side.hero.health = own;
        side.duos = Some(DuosLife {
            pool,
            pool_cap: pool,
            spent: false,
        });
        side
    }

    #[test]
    fn without_duos_every_helper_is_todays_arithmetic() {
        let mut side = create_player_state();
        let start = side.hero.health;
        side_loses(&mut side, 7);
        assert_eq!(side.hero.health, start - 7);
        assert_eq!(side_heals(&mut side, 50), 50);
        assert_eq!(side.hero.health, start + 43);
        side_set(&mut side, -4);
        assert_eq!(side.hero.health, -4);
        assert_eq!(side.duos, None);
    }

    #[test]
    fn a_hit_past_own_hp_spends_it_and_the_rest_comes_off_the_pool() {
        let mut side = duos_side(5, 20);
        side_loses(&mut side, 3);
        assert_eq!((side.hero.health, side.duos.map(|life| life.pool)), (2, Some(20)));
        side_loses(&mut side, 6);
        let life = side.duos.unwrap();
        assert!(life.spent);
        assert_eq!(life.pool, 16);
        assert_eq!(side.hero.health, 16);
        // Spent: the pool takes every later hit, and the hero's health reads it.
        side_loses(&mut side, 4);
        assert_eq!(side.duos.unwrap().pool, 12);
        assert_eq!(side.hero.health, 12);
    }

    #[test]
    fn own_hp_reaching_exactly_zero_is_spent_with_nothing_taken_from_the_pool() {
        let mut side = duos_side(4, 20);
        side_loses(&mut side, 4);
        assert!(side.duos.unwrap().spent);
        assert_eq!(side.hero.health, 20);
    }

    #[test]
    fn healing_is_uncapped_until_spent_then_fills_the_pool_to_its_cap() {
        let mut side = duos_side(10, 20);
        assert_eq!(side_heals(&mut side, 15), 15);
        assert_eq!(side.hero.health, 25);
        side_loses(&mut side, 30);
        assert_eq!(side.duos.unwrap().pool, 15);
        assert_eq!(side_heals(&mut side, 8), 5);
        assert_eq!(side.hero.health, 20);
        assert_eq!(side_heals(&mut side, 3), 0);
    }

    #[test]
    fn setting_own_hp_below_zero_spends_it() {
        let mut side = duos_side(10, 20);
        side_set(&mut side, -3);
        assert!(side.duos.unwrap().spent);
        assert_eq!(side.hero.health, 17);
        side_set(&mut side, 40);
        assert_eq!(side.hero.health, 20);
    }

    #[test]
    fn setting_a_spent_heros_health_sets_the_pool_capped_and_zero_or_less_empties_it() {
        let mut side = duos_side(5, 20);
        side_loses(&mut side, 9);
        assert_eq!(side.duos.map(|life| (life.pool, life.spent)), Some((16, true)));
        side_set(&mut side, 18);
        assert_eq!(
            (side.hero.health, side.duos.map(|life| life.pool)),
            (18, Some(18))
        );
        side_set(&mut side, 99);
        assert_eq!(
            (side.hero.health, side.duos.map(|life| life.pool)),
            (20, Some(20))
        );
        side_set(&mut side, 0);
        assert_eq!((side.hero.health, side.duos.map(|life| life.pool)), (0, Some(0)));
        assert!(side.duos.unwrap().spent, "spent for the game");
    }
}
