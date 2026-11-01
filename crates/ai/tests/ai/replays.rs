//! `AI_SAME_CARD_PLAYS_PER_TURN` (#1885): the AI plays one card at most that many times in a turn. A
//! (0) Spell that comes back after every cast (Classic #90 In Too Deep's reward L lets its controller
//! play it from the graveyard again, with no price floor) was cast until the match's action cap. Here
//! the turn log is written by hand: The Coin in the AI's hand, already played some times this turn.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, ai_options, register_cards};

const COIN: &str = "core-t-coin";

/// The AI's turn holding The Coin, its turn log naming it `times` times.
fn played(times: usize) -> (GameState, ActionBody) {
    register_cards();
    let mut state = scenario(json!({
        "seed": "ai-replays",
        "p1": { "hand": [COIN], "library": ["core-008", "core-008", "core-008"] },
        "p2": { "library": ["core-008", "core-008", "core-008"] },
    }))
    .state()
    .clone();
    let coin = state.players[AI].hand[0].id.clone();
    state.players[AI].turn_log.played_ids = vec![coin.clone(); times];
    let play = legal_actions(&state, AI)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == coin))
        .expect("The Coin is legal to play");
    (state, play)
}

#[test]
fn a_card_played_fewer_times_than_the_cap_this_turn_is_a_candidate() {
    let (state, play) = played(AI_SAME_CARD_PLAYS_PER_TURN - 1);
    assert!(candidate_actions(&state, AI, Prices::NONE).contains(&play));
}

#[test]
fn a_card_played_the_cap_times_this_turn_is_no_candidate_and_the_ai_ends_its_turn() {
    let (state, play) = played(AI_SAME_CARD_PLAYS_PER_TURN);
    let candidates = candidate_actions(&state, AI, Prices::NONE);
    assert!(!candidates.contains(&play), "{candidates:?}");
    assert!(
        legal_actions(&state, AI).contains(&play),
        "still legal: the cap is the AI's own"
    );
    let decision = decide(&state, AI, &mut ai_options("ai-replays-k", AI_BUDGET)).expect("a decision");
    assert_ne!(decision.action, play);
}
