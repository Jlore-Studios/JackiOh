//! Alternative win conditions (SPEC §2.5, ME-WIN, R848–R850) and the state check's game-end point; also
//! the vote tally that removes a player (#1843, R1843000, R1843001) and fleeing (R1843002).
//!
//! The check runs where `hero_check` does once the board has settled (Meditative #8's Ascent 10, #20's
//! chosen conditions): a hero at 0 loses first, a full vote tally then removes its player, a held win
//! then wins, and two winners draw. A fleer who also holds another win wins by it.

use crate::config::VOTE_TALLY_SIZE;
use crate::game_over::end_game;
use crate::layers::unit_view;
use crate::script::EngineSink;
use crate::state::{AltWinCondition, GameState, ModifierKind, VoteTally};
use crate::wire::{
    Ballot, FleeOpponent, GameEvent, GameOverReason, PLAYER_IDS, PlayerId, Winner, opponent_of,
};
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

/// R849: the board condition's two totals, attack and current health over acting Units (dormant cards
/// excluded, R13), read through the §10.4 layers.
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

/// What a held win is, in the order a player's own claims are read (R1843002): an effect's win, then
/// a met alternative condition, then a flight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Claim {
    WonByEffect,
    AltWin,
    Fled(FleeOpponent),
}

fn claim_of(state: &GameState, player: PlayerId) -> Option<Claim> {
    let claim = own_claim_of(state, player)?;
    // R2201000: against a seat that cannot lose, no claim ends the game as its loss. A win by effect
    // or a held win is not read; a flight that would leave it a loss leaves it a draw (R1843004).
    if crate::state::can_lose(state, opponent_of(player)) {
        return Some(claim);
    }
    match claim {
        Claim::WonByEffect | Claim::AltWin => None,
        Claim::Fled(_) => Some(Claim::Fled(FleeOpponent::Draw)),
    }
}

fn own_claim_of(state: &GameState, player: PlayerId) -> Option<Claim> {
    let side = &state.players[player];
    if side.won_by_effect == Some(true) {
        return Some(Claim::WonByEffect);
    }
    let met = side.mods.iter().any(|modifier| match &modifier.kind {
        ModifierKind::AltWin { condition, threshold } => condition_met(state, player, *condition, *threshold),
        _ => false,
    });
    if met {
        return Some(Claim::AltWin);
    }
    side.fled.map(Claim::Fled)
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

/// R850: the state check's game-end point, after the hero check. R1843001: the vote tallies first
/// (`vote_check`), since a removal is a loss and a loss is read before any held win. Two holders draw,
/// `Fled` when both fled and claim nothing else (R1843002), else `AltWin`; a flight is announced by
/// `fled` before the `gameOver`.
pub fn win_check(sink: &mut EngineSink<'_>) -> bool {
    if vote_check(sink) {
        return true;
    }
    let claims: Vec<(PlayerId, Claim)> = PLAYER_IDS
        .into_iter()
        .filter_map(|player| claim_of(sink.state, player).map(|claim| (player, claim)))
        .collect();
    match claims.as_slice() {
        [] => false,
        [(winner, claim)] => {
            let (winner, claim) = (*winner, *claim);
            let reason = match claim {
                Claim::WonByEffect => GameOverReason::WonByEffect,
                Claim::AltWin => GameOverReason::AltWin,
                Claim::Fled(opponent) => {
                    sink.events.push(GameEvent::Fled {
                        player: winner,
                        opponent,
                    });
                    match opponent {
                        FleeOpponent::Draw => GameOverReason::Fled,
                        FleeOpponent::Loss => GameOverReason::FledDecisive,
                    }
                }
            };
            end_game(sink, winner, reason);
            true
        }
        _ => {
            let fleers: Vec<(PlayerId, FleeOpponent)> = claims
                .iter()
                .filter_map(|(player, claim)| match claim {
                    Claim::Fled(opponent) => Some((*player, *opponent)),
                    _ => None,
                })
                .collect();
            let reason = if fleers.len() == claims.len() {
                for (player, opponent) in fleers {
                    sink.events.push(GameEvent::Fled { player, opponent });
                }
                GameOverReason::Fled
            } else {
                GameOverReason::AltWin
            };
            end_game(sink, Winner::Draw, reason);
            true
        }
    }
}

/// R1843000: whether `player`'s tally this turn is full, so the game-end point removes them.
pub fn voted_out(state: &GameState, player: PlayerId) -> bool {
    state.players[player]
        .votes
        .as_ref()
        .is_some_and(|tally| tally.turn == state.turn && tally.ballots.len() as i32 >= VOTE_TALLY_SIZE)
}

/// R1843000: one vote to remove `player`, by `ballot`'s card (`effects::vote_out`). A tally kept for
/// an earlier turn starts again, and a card whose definition already voted against them this turn adds
/// and announces nothing.
pub fn cast_vote(sink: &mut EngineSink<'_>, player: PlayerId, ballot: Ballot) {
    if sink.state.result.is_some() {
        return;
    }
    let turn = sink.state.turn;
    let tally = sink.state.players[player].votes.get_or_insert_with(|| VoteTally {
        turn,
        ballots: Vec::new(),
    });
    if tally.turn != turn {
        *tally = VoteTally {
            turn,
            ballots: Vec::new(),
        };
    }
    if tally.ballots.iter().any(|cast| cast.def_id == ballot.def_id) {
        return;
    }
    tally.ballots.push(ballot.clone());
    let votes = tally.ballots.len() as i32;
    sink.events.push(GameEvent::VoteCast {
        player,
        instance_id: ballot.instance_id,
        def_id: ballot.def_id,
        votes,
        needed: VOTE_TALLY_SIZE,
    });
}

/// R1843000, R1843001: the vote tallies at the game-end point: a player whose tally is full is removed,
/// the other player's win, or a draw when both tallies are full.
fn vote_check(sink: &mut EngineSink<'_>) -> bool {
    let out: Vec<PlayerId> = PLAYER_IDS
        .into_iter()
        .filter(|player| voted_out(sink.state, *player))
        // R2201000: a seat that cannot lose is never removed.
        .filter(|player| crate::state::can_lose(sink.state, *player))
        .collect();
    if out.is_empty() {
        return false;
    }
    for player in &out {
        let ballots = sink.state.players[*player]
            .votes
            .as_ref()
            .map(|tally| tally.ballots.clone())
            .unwrap_or_default();
        sink.events.push(GameEvent::VotedOut {
            player: *player,
            ballots,
        });
    }
    match out.as_slice() {
        [removed] => end_game(sink, opponent_of(*removed), GameOverReason::VotedOut),
        _ => end_game(sink, Winner::Draw, GameOverReason::BothVotedOut),
    }
    true
}
