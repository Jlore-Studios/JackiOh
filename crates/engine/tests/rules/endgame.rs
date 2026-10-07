//! Port of `packages/engine/test/endgame.test.ts`: how a game ends (M1-T8, §2.5).

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;
use jackioh_engine::wire::PlayerId::{P1, P2};

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::{events_of_type, new_game, put, slot};
use crate::rules::fixtures::scripts::{double_edge, infinite_reserves};

/// TS's module `let seq = 0`.
static SEQ: AtomicU32 = AtomicU32::new(0);

/// TS's `act` answers `{ state, events: [] }`; the empty list is never read, so this answers the state.
fn act(state: &GameState, body: Value) -> GameState {
    let seq = SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    let input: ActionInput = json_as(body);
    let result = reduce(state, &input.with_nonce(format!("e{seq}")));
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

fn playing(seed: &str, decks: Option<(Vec<String>, Vec<String>)>) -> GameState {
    let mut state = begin_game(&new_game(seed, decks)).state;
    let keep: Vec<String> = state.players[P1].hand.iter().map(|c| c.id.clone()).collect();
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p1" }));
    let keep: Vec<String> = state.players[P2].hand.iter().map(|c| c.id.clone()).collect();
    state = act(&state, json!({ "type": "mulligan", "keep": keep, "playerId": "p2" }));
    state
}

fn action(literal: Value) -> Action {
    json_as(literal)
}

fn offers_draw(state: &GameState, player: PlayerId) -> bool {
    legal_actions(state, player)
        .iter()
        .any(|a| a.action_type() == ActionType::OfferDraw)
}

/// `describe("ending the game (M1-T8)")`.
mod ending_the_game_m1_t8 {
    use super::*;

    #[test]
    fn a_hero_at_0_loses() {
        let mut state = playing("hero-death", None);
        let bolt = new_instance(&mut state, &double_edge().id, P1, Zone::Hand { player: P1 });
        state.players[P1].hand.push(bolt.clone());

        let result = reduce(
            &state,
            &action(json!({ "type": "play", "instanceId": bolt.id, "playerId": "p1", "nonce": "hd" })),
        );
        assert_eq!(result.error, None);
        assert_eq!(
            result.state.result,
            Some(GameResult {
                winner: Winner::P1,
                reason: GameOverReason::HeroDeath
            })
        );
        assert_eq!(result.state.phase, Phase::Over);
        let winner = events_of_type(&result.events, GameEventType::GameOver)
            .iter()
            .find_map(|event| match event {
                GameEvent::GameOver { winner, .. } => Some(*winner),
                _ => None,
            });
        assert_eq!(winner, Some(Winner::P1));
    }

    #[test]
    fn r59_one_effect_that_leaves_both_heroes_at_0_is_a_draw() {
        let mut state = playing("simultaneous", None);
        let card = new_instance(&mut state, &double_edge().id, P1, Zone::Hand { player: P1 });
        state.players[P1].hand.push(card.clone());
        // p1 will fatigue for 5 on the draw the same effect takes, from 5 health.
        state.players[P1].library = vec![];
        state.players[P1].fatigue_count = 4;
        state.players[P1].hero.health = 5;

        let result = reduce(
            &state,
            &action(json!({ "type": "play", "instanceId": card.id, "playerId": "p1", "nonce": "sim" })),
        );
        assert_eq!(result.error, None);
        assert!(result.state.players[P1].hero.health <= 0);
        assert!(result.state.players[P2].hero.health <= 0);
        assert_eq!(
            result.state.result,
            Some(GameResult {
                winner: Winner::Draw,
                reason: GameOverReason::BothHeroesDead
            })
        );
    }

    #[test]
    fn the_same_two_hits_as_separate_actions_end_the_game_at_the_first_one() {
        let mut state = playing("sequential", None);
        let card = new_instance(&mut state, &double_edge().id, P1, Zone::Hand { player: P1 });
        state.players[P1].hand.push(card.clone());
        state.players[P1].hero.health = 5;
        state.players[P1].fatigue_count = 4;
        let top = new_instance(&mut state, "fx-2", P1, Zone::Library { player: P1 });
        state.players[P1].library = vec![top];

        let result = reduce(
            &state,
            &action(json!({ "type": "play", "instanceId": card.id, "playerId": "p1", "nonce": "seq" })),
        );
        assert_eq!(
            result.state.result,
            Some(GameResult {
                winner: Winner::P1,
                reason: GameOverReason::HeroDeath
            })
        );
    }

    #[test]
    fn a_conceding_player_loses() {
        let result = reduce(
            &playing("concede", None),
            &action(json!({ "type": "concede", "playerId": "p2", "nonce": "c1" })),
        );
        assert_eq!(
            result.state.result,
            Some(GameResult {
                winner: Winner::P1,
                reason: GameOverReason::Concede
            })
        );
    }

    #[test]
    fn r2_r389_ends_in_a_draw_after_the_60th_player_turn_not_the_59th() {
        let mut state = playing("cap", None);
        // R389: do-nothing decks fatigue out at player-turn 48 (§2.4, R3); #75 Infinite Reserves on both
        // sides turns every empty-library draw into a card, so nothing but the cap ends this game.
        put(&mut state, &infinite_reserves().id, slot(P1, Row::Backrow, 1), json!({}));
        put(&mut state, &infinite_reserves().id, slot(P2, Row::Backrow, 1), json!({}));
        for _ in 0..TURN_CAP_PLAYER_TURNS - 1 {
            let active = state.active;
            state = act(&state, json!({ "type": "endTurn", "playerId": active }));
        }
        assert_eq!(state.turn, TURN_CAP_PLAYER_TURNS);
        assert_eq!(state.result, None);

        let active = state.active;
        state = act(&state, json!({ "type": "endTurn", "playerId": active }));
        assert_eq!(
            state.result,
            Some(GameResult {
                winner: Winner::Draw,
                reason: GameOverReason::TurnCap
            })
        );
    }

    #[test]
    fn an_accepted_draw_offer_ends_the_game_as_a_draw() {
        let mut state = playing("offer-accept", None);
        state = act(&state, json!({ "type": "offerDraw", "playerId": "p1" }));
        state = act(&state, json!({ "type": "answerDraw", "accept": true, "playerId": "p2" }));
        assert_eq!(
            state.result,
            Some(GameResult {
                winner: Winner::Draw,
                reason: GameOverReason::DrawAccepted
            })
        );
    }

    #[test]
    fn r36_a_declined_offer_blocks_the_offering_player_for_three_of_their_turns_not_the_opponent() {
        let mut state = playing("offer-decline", None);
        state = act(&state, json!({ "type": "offerDraw", "playerId": "p1" }));
        state = act(&state, json!({ "type": "answerDraw", "accept": false, "playerId": "p2" }));
        assert_eq!(state.players[P1].draw_offer.blocked_until, Some(1 + DRAW_OFFER_BLOCK_TURNS + 1));

        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        // The opponent may still offer on their own turn.
        assert!(offers_draw(&state, P2));

        let advance_to_p1_turn = |from: GameState, target: i32| -> GameState {
            let mut next = from;
            while !(next.active == P1 && next.players[P1].turns_started == target) {
                let active = next.active;
                next = act(&next, json!({ "type": "endTurn", "playerId": active }));
            }
            next
        };

        // Their next three turns are blocked, and the fourth is free again.
        for turn in [2, 3, 4] {
            state = advance_to_p1_turn(state, turn);
            assert!(!offers_draw(&state, P1));
        }
        state = advance_to_p1_turn(state, 5);
        assert!(offers_draw(&state, P1));
    }

    #[test]
    fn auto_ends_a_turn_when_nothing_but_ending_it_is_left() {
        let empty: Vec<String> = vanilla_deck(DECK_SIZE, 1);
        let mut state = playing("auto-end", Some((empty, vanilla_deck(DECK_SIZE, 21))));
        state.players[P1].hand = vec![];
        state.players[P1].library = vec![];
        state.players[P1].hero.health = 40; // survive the fatigue draws

        // p1 ends; p2 plays their turn; p1's next turn has nothing to do, so it ends itself.
        state = act(&state, json!({ "type": "endTurn", "playerId": "p1" }));
        let result = reduce(&state, &action(json!({ "type": "endTurn", "playerId": "p2", "nonce": "auto" })));
        assert_eq!(result.error, None);
        let auto = events_of_type(&result.events, GameEventType::TurnAutoEnded);
        assert!(!auto.is_empty()); // toBeGreaterThanOrEqual(1)
        let first = auto.iter().find_map(|event| match event {
            GameEvent::TurnAutoEnded { player, .. } => Some(*player),
            _ => None,
        });
        assert_eq!(first, Some(P1));
    }

    #[test]
    fn r79_the_match_ceiling_ends_the_game_in_a_draw() {
        let result = reduce(
            &playing("ceiling", None),
            &action(json!({ "type": "ceilingReached", "playerId": "p1", "nonce": "ceil" })),
        );
        assert_eq!(
            result.state.result,
            Some(GameResult {
                winner: Winner::Draw,
                reason: GameOverReason::MatchCeiling
            })
        );
    }

    #[test]
    fn r79_a_timeout_answers_the_open_prompt_and_only_ends_the_turn_of_the_player_who_ran_out() {
        let with_prompt = begin_game(&new_game("timeout", None)).state;
        assert_eq!(mulligan_owed(&with_prompt), vec![P1, P2]);

        let answered = reduce(&with_prompt, &action(json!({ "type": "timeout", "playerId": "p1", "nonce": "to1" })));
        assert_eq!(answered.error, None);
        // p1's mulligan is answered (R268), so p2's is all that is left rather than the turn ending.
        assert_eq!(mulligan_owed(&answered.state), vec![P2]);
        assert_eq!(answered.state.turn, 0);

        let main_phase = playing("timeout-main", None);
        let ended = reduce(&main_phase, &action(json!({ "type": "timeout", "playerId": "p1", "nonce": "to2" })));
        assert_eq!(ended.error, None);
        assert_eq!(ended.state.active, P2);
    }

    #[test]
    fn lets_a_prompt_blocked_game_still_be_conceded_or_timed_out_build_m1_t3() {
        let with_prompt = begin_game(&new_game("blocked", None)).state;
        assert_eq!(
            reduce(&with_prompt, &action(json!({ "type": "concede", "playerId": "p2", "nonce": "b1" }))).error,
            None
        );
        assert_eq!(
            reduce(&with_prompt, &action(json!({ "type": "timeout", "playerId": "p2", "nonce": "b3" }))).error,
            None
        );
        let refused = reduce(
            &with_prompt,
            &action(json!({ "type": "play", "instanceId": "c1", "playerId": "p1", "nonce": "b2" })),
        )
        .error;
        assert!(
            refused.as_deref().is_some_and(|error| error.contains("the mulligan is open")),
            "expected a refusal matching /the mulligan is open/, got {refused:?}"
        );
    }

    #[test]
    fn r79_disconnect_expiry_is_a_loss() {
        let result = reduce(
            &playing("disconnect", None),
            &action(json!({ "type": "disconnectExpired", "player": "p2", "playerId": "p2", "nonce": "dc" })),
        );
        assert_eq!(
            result.state.result,
            Some(GameResult {
                winner: Winner::P1,
                reason: GameOverReason::Disconnect
            })
        );
    }
}
