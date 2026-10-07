//! `reduce` (M1-T3): refusals leave the state untouched, the non-active player's actions, the open
//! mulligans (R265), nonce dedupe, the game's end, bad plays, and every action `legalActions` lists
//! applying cleanly.
//!
//! Port of `packages/engine/test/reduce.test.ts`. TS's `toBe` on a state (object identity) is
//! equality here: a Rust value has no identity to compare.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::harness::new_game;

/// TS's module `let seq`.
static SEQ: AtomicU32 = AtomicU32::new(0);

fn nonce() -> String {
    let seq = SEQ.fetch_add(1, Ordering::SeqCst) + 1;
    format!("r{seq}")
}

/// An action from its TS object literal (`{ type, …, playerId, nonce }`).
fn action(body: Value) -> Action {
    json_as(body)
}

fn hand_ids(state: &GameState, player: PlayerId) -> Vec<String> {
    state.players[player].hand.iter().map(|c| c.id.clone()).collect()
}

fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&new_game(seed, None)).state;
    state = reduce(
        &state,
        &Action::new(ActionBody::Mulligan { keep: hand_ids(&state, P1) }, P1, nonce()),
    )
    .state;
    state = reduce(
        &state,
        &Action::new(ActionBody::Mulligan { keep: hand_ids(&state, P2) }, P2, nonce()),
    )
    .state;
    state
}

/// TS `expect(error).toMatch(/text/)`.
fn assert_error(error: &Option<String>, text: &str) {
    assert!(
        error.as_deref().is_some_and(|message| message.contains(text)),
        "expected an error matching {text:?}, got {error:?}"
    );
}

/// reduce (M1-T3)
mod reduce_m1_t3 {
    use super::*;

    #[test]
    fn rejects_an_action_from_the_non_active_player_and_leaves_the_state_untouched() {
        let state = playing("reduce");
        let before = clone_state(&state);

        let result = reduce(&state, &Action::new(ActionBody::EndTurn, P2, nonce()));
        assert_error(&result.error, "not your turn");
        assert_eq!(result.events, Vec::<GameEvent>::new());
        assert_eq!(result.state, before);
    }

    #[test]
    fn lets_the_non_active_player_concede_and_answer_a_draw_offer() {
        let state = playing("non-active");
        assert_eq!(reduce(&state, &Action::new(ActionBody::Concede, P2, nonce())).error, None);

        let offered = reduce(&state, &Action::new(ActionBody::OfferDraw, P1, nonce())).state;
        let answered = reduce(
            &offered,
            &Action::new(ActionBody::AnswerDraw { accept: false }, P2, nonce()),
        );
        assert_eq!(answered.error, None);
    }

    #[test]
    fn r265_refuses_anything_but_a_mulligan_while_the_mulligans_are_open_from_either_seat() {
        let state = begin_game(&new_game("pending", None)).state;
        assert!(state.pending.is_none());
        assert_eq!(mulligan_owed(&state), vec![P1, P2]);

        let first_card = state.players.p1.hand.first().map(|c| c.id.clone()).unwrap_or_default();
        let played = reduce(
            &state,
            &action(json!({ "type": "play", "instanceId": first_card, "playerId": "p1", "nonce": nonce() })),
        );
        assert_error(&played.error, "the mulligan is open");
        assert_error(
            &reduce(&state, &Action::new(ActionBody::EndTurn, P1, nonce())).error,
            "the mulligan is open",
        );
        assert_error(
            &reduce(&state, &Action::new(ActionBody::OfferDraw, P2, nonce())).error,
            "the mulligan is open",
        );

        // Either seat answers its own first; a second answer from the same seat is refused.
        let p2_first = reduce(&state, &Action::new(ActionBody::Mulligan { keep: vec![] }, P2, nonce()));
        assert_eq!(p2_first.error, None);
        assert_eq!(mulligan_owed(&p2_first.state), vec![P1]);
        let again = reduce(
            &p2_first.state,
            &Action::new(ActionBody::Mulligan { keep: vec![] }, P2, nonce()),
        );
        assert_error(&again.error, "already answered");
        let stranger = reduce(
            &state,
            &Action::new(
                ActionBody::Mulligan {
                    keep: vec!["nope".to_string()],
                },
                P1,
                nonce(),
            ),
        );
        assert_error(&stranger.error, "not in your hand");
    }

    #[test]
    fn dedupes_a_repeated_nonce_same_state_no_duplicate_events() {
        let state = playing("dedupe");
        let id = nonce();
        let first = reduce(&state, &Action::new(ActionBody::EndTurn, P1, id.clone()));
        let second = reduce(&first.state, &Action::new(ActionBody::EndTurn, P1, id));

        assert_eq!(second.state, first.state);
        assert_eq!(second.events, first.events);
        assert_eq!(second.error, None);
    }

    #[test]
    fn refuses_an_action_once_the_game_is_over() {
        let over = reduce(&playing("over"), &Action::new(ActionBody::Concede, P1, nonce())).state;
        assert_error(
            &reduce(&over, &Action::new(ActionBody::EndTurn, P2, nonce())).error,
            "game is over",
        );
    }

    #[test]
    fn errors_on_a_card_that_is_not_in_hand_and_on_a_zone_that_is_taken() {
        let mut state = playing("bad-play");
        let ghost = new_instance(&mut state, "fx-1", P1, Zone::Hand { player: P1 });
        assert_error(
            &reduce(
                &state,
                &action(json!({ "type": "play", "instanceId": ghost.id, "playerId": "p1", "nonce": nonce() })),
            )
            .error,
            "no card",
        );

        let card = state.players.p1.hand.first().map(|c| c.id.clone()).unwrap_or_default();
        let mut played = reduce(
            &state,
            &action(json!({
                "type": "play",
                "instanceId": card,
                "zone": { "row": "units", "lane": 1 },
                "playerId": "p1",
                "nonce": nonce(),
            })),
        );
        assert_eq!(played.error, None);

        let second = played.state.players.p1.hand.first().map(|c| c.id.clone()).unwrap_or_default();
        played.state.players.p1.mana.current = 4; // enough mana, so the zone is what refuses
        let blocked = reduce(
            &played.state,
            &action(json!({
                "type": "play",
                "instanceId": second,
                "zone": { "row": "units", "lane": 1 },
                "playerId": "p1",
                "nonce": nonce(),
            })),
        );
        assert_error(&blocked.error, "not open");
    }

    // An explicit timeout, because the 5 s this inherited from vitest's default was never chosen for
    // it and is the only thing here that measures the machine rather than the engine. Measured on the
    // dev box: the whole file runs in 704 ms, or 1.00 s under coverage instrumentation. It still timed
    // out at 5 s inside a Linux container running the coverage step while three other agents worked —
    // a shared or throttled CI runner is the same environment, and `pnpm test:coverage` instruments
    // every module this walk touches. 30 s keeps roughly a 30x margin over the measured cost while
    // still failing an engine that has genuinely stopped terminating. No assertion below changes:
    // the walk still probes every action legalActions offers at each of the 200 states.
    // (A Rust test has no default timeout, so there is none to raise.)
    #[test]
    fn every_action_legal_actions_lists_succeeds_over_200_random_states() {
        let mut rng = Rng::new("legal-actions-walk", 0);
        let mut states = 0;
        let mut probes = 0;

        let mut game = 0;
        while states < 200 {
            let mut state = playing(&format!("walk-{game}"));

            while state.result.is_none() && states < 200 {
                // TS's seatToAct always names a seat; its last fallback is the active player.
                let player = seat_to_act(&state).unwrap_or(state.active);
                let actions = legal_actions(&state, player);
                assert!(!actions.is_empty());
                states += 1;

                // Every listed action must apply cleanly from this state, concedes and offers included.
                for listed in &actions {
                    let probe = reduce(&state, &Action::new(listed.clone(), player, nonce()));
                    assert!(
                        probe.error.is_none(),
                        "{} should be legal: {}",
                        listed.action_type(),
                        probe.error.clone().unwrap_or_default()
                    );
                    probes += 1;
                }

                // Walk on with something that does not end the game.
                let walkable: Vec<&ActionBody> = actions
                    .iter()
                    .filter(|listed| {
                        listed.action_type() != ActionType::Concede && listed.action_type() != ActionType::AnswerDraw
                    })
                    .collect();
                let at = rng.int(walkable.len() as i32) as usize;
                let chosen = walkable.get(at).or_else(|| walkable.first()).map(|body| (*body).clone());
                let chosen = chosen.expect("a walkable action");
                state = reduce(&state, &Action::new(chosen, player, nonce())).state;
            }
            game += 1;
        }

        assert_eq!(states, 200);
        assert!(probes > 200);
    }

    #[test]
    fn r211_offers_only_concede_to_a_player_with_nothing_to_answer() {
        let begun = begin_game(&new_game("prompt-actions", None)).state;
        // R265: both seats owe a mulligan at once, so both are offered theirs and concede.
        for player in [P1, P2] {
            let legal = legal_actions(&begun, player);
            assert!(legal
                .iter()
                .all(|a| matches!(a.action_type(), ActionType::Mulligan | ActionType::Concede)));
            assert_eq!(
                legal.iter().filter(|a| a.action_type() == ActionType::Mulligan).count(),
                1usize << begun.players[player].hand.len()
            );
        }
        // Once p1 has answered, p1 has nothing left to answer and is offered concede alone.
        let state = reduce(&begun, &Action::new(ActionBody::Mulligan { keep: vec![] }, P1, "r211-m")).state;
        assert_eq!(legal_actions(&state, P1), vec![ActionBody::Concede]);
        assert!(legal_actions(&state, P2)
            .iter()
            .all(|a| matches!(a.action_type(), ActionType::Mulligan | ActionType::Concede)));
        // And `reduce` agrees: a concede is accepted from the seat that owes nothing.
        assert_eq!(
            reduce(&state, &Action::new(ActionBody::Concede, P1, "r211-concede")).error,
            None
        );
    }

    #[test]
    fn offers_one_play_per_open_zone_for_a_unit() {
        let state = playing("zones-listed");
        let unit_id = state.players.p1.hand.first().map(|c| c.id.clone()).unwrap_or_default();
        let plays = legal_actions(&state, P1)
            .into_iter()
            .filter(|a| matches!(a, ActionBody::Play { instance_id, .. } if *instance_id == unit_id))
            .count();
        assert_eq!(plays, 5);
    }

    #[test]
    fn keeps_the_deck_lists_out_of_the_state_it_returns() {
        let state = playing("serializable");
        let round: GameState =
            serde_json::from_str(&serde_json::to_string(&state).expect("serialises")).expect("parses");
        assert_eq!(round, state);
        assert!(state.players.p1.library.len() + state.players.p1.hand.len() <= DECK_SIZE as usize);
        assert_eq!(vanilla_deck(DECK_SIZE, 1).len(), DECK_SIZE as usize);
    }
}
