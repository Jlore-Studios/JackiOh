//! Alternative win conditions (SPEC §2.5, ME-WIN, R848–R850): what a held win reads, whether one
//! holds, the badge caption with its progress, and the game-end point of the state check.
//!
//! Meditative #8's Ascent 10 and #20's chosen conditions are the first held wins. The check runs
//! where `hero_check` does once the board has settled: a hero at 0 loses first, a held win then
//! wins, and two winners draw.

use crate::game_over::end_game;
use crate::layers::unit_view;
use crate::script::EngineSink;
use crate::state::{AltWinCondition, GameState, ModifierKind};
use crate::wire::{GameOverReason, PLAYER_IDS, PlayerId, Winner};
use crate::zones::active_units_of;

/// R848: whether the threshold is met now.
pub fn condition_met(
    state: &GameState,
    player: PlayerId,
    condition: AltWinCondition,
    threshold: i32,
) -> bool {
    match condition {
        AltWinCondition::Health => state.players[player].hero.health >= threshold,
        AltWinCondition::Graveyard => state.players[player].graveyard.len() as i32 >= threshold,
        AltWinCondition::Board => {
            let (attack, health) = board_totals(state, player);
            attack >= threshold && health >= threshold
        }
    }
}

/// R849: the board condition's two totals — attack and current health over acting Units
/// (`active_units_of`: tops of piles and animated cards, dormant cards excluded, R13), read
/// through the §10.4 layers.
pub fn board_totals(state: &GameState, player: PlayerId) -> (i32, i32) {
    let mut attack = 0;
    let mut health = 0;
    for unit in active_units_of(state, player) {
        let view = unit_view(state, unit);
        attack += view.attack;
        health += view.health;
    }
    (attack, health)
}

/// R848, R850: whether this player holds a win — their `win_game` flag, or any kept `AltWin`
/// modifier that is met.
pub fn holds_win(state: &GameState, player: PlayerId) -> bool {
    if state.players[player].won_by_effect == Some(true) {
        return true;
    }
    state.players[player]
        .mods
        .iter()
        .any(|modifier| match &modifier.kind {
            ModifierKind::AltWin { condition, threshold } => {
                condition_met(state, player, *condition, *threshold)
            }
            _ => false,
        })
}

/// R848: a held condition's badge caption, with its progress toward the threshold.
pub fn alt_win_label(
    state: &GameState,
    player: PlayerId,
    condition: AltWinCondition,
    threshold: i32,
) -> String {
    match condition {
        AltWinCondition::Health => {
            let now = state.players[player].hero.health;
            format!("You win at {threshold} hero Health ({now}/{threshold})")
        }
        AltWinCondition::Graveyard => {
            let now = state.players[player].graveyard.len() as i32;
            format!("You win at {threshold} cards in your graveyard ({now}/{threshold})")
        }
        AltWinCondition::Board => {
            let (attack, health) = board_totals(state, player);
            format!(
                "You win at {threshold} Attack and {threshold} Health on your Units ({attack}/{threshold}, {health}/{threshold})"
            )
        }
    }
}

/// R850: the state check's game-end point. No holder: nothing. Two holders: a draw (`AltWin`).
/// One holder: `WonByEffect` when their `win_game` flag is set, else `AltWin`.
pub fn win_check(sink: &mut EngineSink<'_>) -> bool {
    let winners: Vec<PlayerId> = PLAYER_IDS
        .into_iter()
        .filter(|player| holds_win(sink.state, *player))
        .collect();
    if winners.is_empty() {
        return false;
    }
    if winners.len() == 2 {
        end_game(sink, Winner::Draw, GameOverReason::AltWin);
        return true;
    }
    let winner = winners[0];
    let reason = if sink.state.players[winner].won_by_effect == Some(true) {
        GameOverReason::WonByEffect
    } else {
        GameOverReason::AltWin
    };
    end_game(sink, winner, reason);
    true
}
