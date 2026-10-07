//! The turn cap doubled (docs/classic-sets.md B4.3, R389, rewriting R2): 60 player-turns, 30 each, then
//! the game is a draw. Its knock-on is proved too: two 20-card decks that do nothing now fatigue out
//! before the cap, so fatigue is the usual end of a long game and the cap a backstop for games that
//! never fatigue (§2.4, R3).
//!
//! Port of `packages/engine/test/turn-cap.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{new_game, put, slot};
use crate::rules::fixtures::scripts::infinite_reserves;

static NONCE: AtomicU32 = AtomicU32::new(0);

fn act(state: &GameState, body: Value) -> GameState {
    let nonce = NONCE.fetch_add(1, Ordering::SeqCst) + 1;
    let input: ActionInput = json_as(body);
    let result = reduce(state, &input.with_nonce(format!("cap{nonce}")));
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

/// Past both mulligans, in p1's main phase on turn 1, with nothing ending a turn but End turn.
fn playing(seed: &str) -> GameState {
    let decks = (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21));
    let mut state = begin_game(&new_game(&format!("turn-cap-{seed}"), Some(decks))).state;
    for player in [PlayerId::P1, PlayerId::P2] {
        let keep: Vec<String> = state.players[player].hand.iter().map(|card| card.id.clone()).collect();
        state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": player }));
    }
    state
}

/// Both players press End turn until the game is over, and nothing else.
fn pass_until_over(state: GameState) -> GameState {
    let mut next = state;
    let mut step = 0;
    while step <= 2 * TURN_CAP_PLAYER_TURNS && next.result.is_none() {
        next = act(&next, json!({ "type": "endTurn", "playerId": next.active }));
        step += 1;
    }
    next
}

mod r389_b4_3_the_turn_cap {
    use super::*;

    #[test]
    fn r389_the_cap_is_60_player_turns_30_each_a_game_that_never_fatigues_is_a_draw_at_the_end_of_the_60th() {
        assert_eq!(TURN_CAP_PLAYER_TURNS, 60);
        let mut state = playing("reserves");
        // #75 Infinite Reserves turns every empty-library draw into a card, so no hero ever fatigues.
        put(&mut state, &infinite_reserves().id, slot(PlayerId::P1, Row::Backrow, 1), Default::default());
        put(&mut state, &infinite_reserves().id, slot(PlayerId::P2, Row::Backrow, 1), Default::default());
        let over = pass_until_over(state);
        assert_eq!(
            over.result,
            Some(GameResult {
                winner: Winner::Draw,
                reason: GameOverReason::TurnCap
            })
        );
        assert_eq!(over.turn, TURN_CAP_PLAYER_TURNS);
        assert_eq!(over.players.p1.turns_started, 30);
        assert_eq!(over.players.p2.turns_started, 30);
    }

    #[test]
    fn r389_r3_two_do_nothing_20_card_decks_fatigue_out_before_the_cap_so_fatigue_ends_such_a_game_2_4() {
        let over = pass_until_over(playing("fatigue"));
        assert_eq!(over.result.map(|result| result.reason), Some(GameOverReason::HeroDeath));
        assert!(over.turn < TURN_CAP_PLAYER_TURNS);
        // B4.3's arithmetic: the second player's library empties first (four opening cards to three),
        // and their eighth fatigue draw kills them on their 24th turn, player-turn 48.
        assert_eq!(over.turn, 48);
        assert_eq!(over.result.map(|result| result.winner), Some(Winner::P1));
    }
}
