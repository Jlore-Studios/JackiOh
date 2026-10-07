//! decide's short-circuits: forced moves, not its turn, the mulligan and draw offers
//! (SPEC §9.9, R188; docs/polish/3-ai.md B16, B20, B21).
//!
//! B16: one candidate is played without searching (no node, no draw from the AI's rng), and a seat
//! that owes nothing gets null. B20: the mulligan returns exactly the cards costing more than
//! AI_MULLIGAN.keepMaxCost. B21 and R188: the AI answers an unanswered draw offer at once with a
//! decline, and never concedes, offers a draw or accepts one.
//!
//! Port of `packages/ai/test/decide.test.ts`. TS's per-test timeouts have no `cargo test` twin and
//! are dropped.

use jackioh_ai::{
    AI_GATE, AI_GATE_BUDGET, AI_MULLIGAN, AiOptions, Decision, DecisionReason, MatchHooks, Matchup,
    SeatController, ai_to_act, decide, game_config, mulligan_keep, play_match, unanswered_draw_offer,
};
use jackioh_engine::testkit::{
    Action, ActionBody, ActionType, GameEvent, GameState, PerPlayer, PlayerId, PromptKind, create_rng,
    def_of, json, legal_actions, mulligan_prompt_for, query_cost, reduce, seat_to_act, subsystems,
};

use super::support::{AI, HUMAN, act, act_with_nonce, clone, dealt_game, is_legal, register_cards, scenario};

fn must_decide(state: &GameState, seat: PlayerId, seed: &str) -> Decision {
    decide(state, seat, &mut AiOptions::new(create_rng(seed, 0)))
        .unwrap_or_else(|| panic!("decide returned null for {seat}"))
}

/// The opponent's turn at turn 10, both sides with something to do.
fn humans_turn(seed: &str) -> GameState {
    register_cards();
    scenario(json!({
        "seed": seed,
        "active": HUMAN,
        "turn": 10,
        "p1": { "hand": ["core-008"], "field": ["core-011"] },
        "p2": { "hand": ["core-011"], "field": ["core-008"] },
    }))
    .state()
    .clone()
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

fn hand_ids(state: &GameState, seat: PlayerId) -> Vec<String> {
    state.players[seat]
        .hand
        .iter()
        .map(|card| card.id.clone())
        .collect()
}

/// The ids of `player`'s cards `kind` (drawn or shuffled in) names in `events`.
fn ids_where(events: &[GameEvent], player: PlayerId, shuffled: bool) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Drawn {
                player: who,
                instance_id,
                ..
            } if !shuffled && *who == player => Some(instance_id.clone()),
            GameEvent::ShuffledIn {
                player: who,
                instance_id,
                ..
            } if shuffled && *who == player => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// B16
// ---------------------------------------------------------------------------------------------

mod forced_moves_and_nothing_to_do_b16 {
    use super::*;

    #[test]
    fn b16_a_prompt_with_one_option_is_answered_with_reason_forced_no_node_spent_and_the_rng_untouched() {
        register_cards();
        let mut s = scenario(json!({
            "seed": "decide-forced-prompt",
            "p1": { "hand": ["core-072"], "graveyard": ["core-008"], "library": ["core-011"] },
            "p2": { "hand": ["core-005"] },
        }));
        s.play("core-072", json!({}));
        let state = s.state().clone();
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(AI));
        // R211 offers concede beside the prompt's answers; the AI skips it (R84, R188).
        let legal: Vec<ActionBody> = legal_actions(&state, AI)
            .into_iter()
            .filter(|action| action.action_type() != ActionType::Concede)
            .collect();
        assert_eq!(legal.len(), 1);

        let mut options = AiOptions::new(create_rng("decide-forced-prompt", 0));
        let decision = decide(&state, AI, &mut options);
        assert_eq!(decision.as_ref().map(|d| d.reason), Some(DecisionReason::Forced));
        assert_eq!(
            decision.as_ref().map(|d| d.action.clone()),
            legal.first().cloned()
        );
        assert_eq!(decision.as_ref().map(|d| d.stats.nodes), Some(0));
        assert_eq!(options.rng.cursor(), 0);
        // The engine takes it.
        act(&state, AI, &decision.expect("a decision").action);
    }

    #[test]
    fn b16_a_main_phase_whose_only_candidate_is_end_turn_is_forced_with_no_node_and_the_rng_untouched() {
        register_cards();
        let s = scenario(json!({
            "seed": "decide-forced-end",
            "p1": {},
            "p2": { "field": ["core-008"], "hand": ["core-005"] },
        }));
        let mut options = AiOptions::new(create_rng("decide-forced-end", 0));
        let decision = decide(s.state(), AI, &mut options);
        assert_eq!(decision.as_ref().map(|d| d.reason), Some(DecisionReason::Forced));
        assert_eq!(
            decision.as_ref().map(|d| d.action.clone()),
            Some(ActionBody::EndTurn)
        );
        assert_eq!(decision.as_ref().map(|d| d.stats.nodes), Some(0));
        assert_eq!(options.rng.cursor(), 0);
    }

    #[test]
    fn b16_the_seats_own_prompt_with_several_options_is_not_forced_it_is_searched_and_answered_legally() {
        register_cards();
        let mut s = scenario(json!({
            "seed": "decide-not-forced",
            "p1": { "hand": ["core-072"], "graveyard": ["core-008", "core-044", "core-005"], "library": ["core-011"] },
            "p2": { "hand": ["core-005"], "field": ["core-008"] },
        }));
        s.play("core-072", json!({}));
        let state = s.state().clone();
        assert!(legal_actions(&state, AI).len() > 1);

        let decision = decide(
            &state,
            AI,
            &mut AiOptions::with_budget(create_rng("decide-not-forced", 0), AI_GATE_BUDGET),
        );
        assert_ne!(decision.as_ref().map(|d| d.reason), Some(DecisionReason::Forced));
        assert_eq!(
            decision.as_ref().map(|d| d.action.action_type()),
            Some(ActionType::Answer)
        );
        assert!(decision.as_ref().is_some_and(|d| d.stats.nodes > 0));
        assert!(is_legal(&state, AI, &decision.expect("a decision").action));
    }

    #[test]
    fn b16_ai_to_act_is_true_in_the_seats_main_phase_and_at_its_own_prompt() {
        register_cards();
        let main = scenario(json!({
            "seed": "decide-owes",
            "p1": { "hand": ["core-008"] },
            "p2": { "hand": ["core-005"] },
        }));
        assert!(ai_to_act(main.state(), AI));
        assert!(!ai_to_act(main.state(), HUMAN));

        let mut s = scenario(json!({
            "seed": "decide-owes-prompt",
            "p1": { "hand": ["core-072"], "graveyard": ["core-008", "core-044"], "library": ["core-011"] },
            "p2": { "hand": ["core-005"] },
        }));
        s.play("core-072", json!({}));
        assert!(ai_to_act(s.state(), AI));
    }

    #[test]
    fn b16_on_the_opponents_turn_with_nothing_to_answer_decide_returns_null() {
        let state = humans_turn("decide-null-turn");
        assert!(!ai_to_act(&state, AI));
        assert!(decide(&state, AI, &mut AiOptions::new(create_rng("decide-null-turn", 0))).is_none());
    }

    #[test]
    fn b16_while_the_opponents_prompt_is_open_decide_returns_null_for_the_seat_that_does_not_hold_it() {
        register_cards();
        let mut s = scenario(json!({
            "seed": "decide-null-prompt",
            "active": HUMAN,
            "turn": 10,
            "p1": { "hand": ["core-008"] },
            "p2": { "hand": ["core-072"], "graveyard": ["core-008", "core-044"], "library": ["core-011"] },
        }));
        s.play("core-072", json!({}));
        assert_eq!(
            s.state().pending.as_ref().map(|pending| pending.player_id),
            Some(HUMAN)
        );
        assert!(!ai_to_act(s.state(), AI));
        assert!(ai_to_act(s.state(), HUMAN));
        assert!(
            decide(
                s.state(),
                AI,
                &mut AiOptions::new(create_rng("decide-null-prompt", 0))
            )
            .is_none()
        );
    }

    #[test]
    fn r265_b16_both_seats_owe_their_mulligan_at_once_and_a_seat_that_has_answered_owes_nothing_while_the_others_is_open()
     {
        let dealt = dealt_game("decide-null-mulligan");
        assert!(dealt.pending.is_none());
        // Neither seat waits on the other: each is asked for its own mulligan straight away.
        for seat in [PlayerId::P1, PlayerId::P2] {
            assert!(ai_to_act(&dealt, seat), "{seat}");
            let decision = decide(
                &dealt,
                seat,
                &mut AiOptions::new(create_rng(&format!("decide-null-mulligan:{seat}"), 0)),
            );
            assert_eq!(decision.map(|d| d.reason), Some(DecisionReason::Mulligan));
        }
        // p2 answers first; its answer is sealed and p1's mulligan is still open (R266).
        let state = act(&dealt, PlayerId::P2, ActionBody::Mulligan { keep: vec![] });
        assert!(!ai_to_act(&state, PlayerId::P2));
        assert!(
            decide(
                &state,
                PlayerId::P2,
                &mut AiOptions::new(create_rng("decide-null-mulligan", 0))
            )
            .is_none()
        );
        assert!(ai_to_act(&state, PlayerId::P1));
    }

    #[test]
    fn b16_once_the_game_is_over_decide_returns_null_for_both_seats() {
        register_cards();
        let mut s = scenario(json!({
            "seed": "decide-null-over",
            "p1": { "field": ["core-011"], "hand": ["core-008"] },
            "p2": { "health": 3, "hand": ["core-005"] },
        }));
        s.attack("core-011", "hero");
        assert_eq!(
            s.state().result.and_then(|result| result.winner.player()),
            Some(AI)
        );
        for seat in [AI, HUMAN] {
            assert!(!ai_to_act(s.state(), seat));
            assert!(
                decide(
                    s.state(),
                    seat,
                    &mut AiOptions::new(create_rng("decide-null-over", 0))
                )
                .is_none()
            );
        }
    }
}

// ---------------------------------------------------------------------------------------------
// B20
// ---------------------------------------------------------------------------------------------

mod the_mulligan_b20 {
    use super::*;

    #[test]
    fn b20_the_ai_returns_exactly_the_cards_costing_more_than_keep_max_cost_with_reason_mulligan_and_reduce_takes_it()
     {
        let mut returned_total = 0;
        let mut kept_total = 0;
        for n in 1..=8 {
            let seed = format!("decide-mulligan-{n}");
            let mut state = dealt_game(&seed);
            let mut returned_by: PerPlayer<Vec<String>> = PerPlayer::new(vec![], vec![]);
            let mut events: Vec<GameEvent> = Vec::new();
            for seat in [PlayerId::P1, PlayerId::P2] {
                let prompt = mulligan_prompt_for(&state, seat).cloned();
                assert_eq!(
                    prompt.as_ref().map(|p| p.kind),
                    Some(PromptKind::Mulligan),
                    "{seed} {seat}"
                );
                assert_eq!(prompt.as_ref().map(|p| p.player_id), Some(seat), "{seed} {seat}");

                let hand = state.players[seat].hand.clone();
                let keep = sorted(
                    hand.iter()
                        .filter(|card| {
                            query_cost(def_of(Some(&state), &card.def_id)) <= AI_MULLIGAN.keep_max_cost
                        })
                        .map(|card| card.id.clone())
                        .collect(),
                );
                let returned: Vec<String> = hand
                    .iter()
                    .map(|card| card.id.clone())
                    .filter(|id| !keep.contains(id))
                    .collect();

                let mut options = AiOptions::new(create_rng(&format!("{seed}:{seat}"), 0));
                let decision = decide(&state, seat, &mut options);
                assert_eq!(
                    decision.as_ref().map(|d| d.reason),
                    Some(DecisionReason::Mulligan),
                    "{seed} {seat}"
                );
                let decision = decision.expect("a decision");
                let action = decision.action.clone();
                assert_eq!(action.action_type(), ActionType::Mulligan);
                let chosen = match &action {
                    ActionBody::Mulligan { keep } => Some(sorted(keep.clone())),
                    _ => None,
                };
                assert_eq!(chosen, Some(keep.clone()), "{seed} {seat}");
                assert_eq!(
                    sorted(mulligan_keep(&state, seat, AI_MULLIGAN.keep_max_cost)),
                    keep,
                    "{seed} {seat}"
                );
                assert_eq!(options.rng.cursor(), 0, "{seed} {seat}");
                assert_eq!(decision.stats.score, 0.0);

                let result = reduce(&state, &Action::new(action, seat, format!("{seed}-{seat}")));
                assert_eq!(result.error, None, "{seed} {seat}");
                state = result.state;
                events = result.events;
                returned_total += returned.len();
                kept_total += keep.len();
                returned_by[seat] = returned;
            }
            // Both answers are in, so both resolved in the second one (R265). R9: the replacements were
            // drawn before the returned cards went back, so none of setup's draws brought one back.
            // (TS's `slice(0, -1)` when no turn started is kept: it drops the last event.)
            let end = events
                .iter()
                .position(|event| matches!(event, GameEvent::TurnStarted { .. }))
                .unwrap_or(events.len().saturating_sub(1));
            let setup_events = &events[..end];
            for seat in [PlayerId::P1, PlayerId::P2] {
                let drawn = ids_where(setup_events, seat, false);
                let shuffled = ids_where(setup_events, seat, true);
                assert_eq!(drawn.len(), returned_by[seat].len(), "{seed} {seat}");
                assert_eq!(
                    sorted(shuffled),
                    sorted(returned_by[seat].clone()),
                    "{seed} {seat}"
                );
                for id in &returned_by[seat] {
                    assert!(!drawn.contains(id), "{seed} {seat} {id}");
                }
            }
        }
        // Not vacuous: the seeds dealt both kinds of card.
        assert!(returned_total > 0);
        assert!(kept_total > 0);
    }
}

/// p1's mulligan with its hand's defs replaced, so the costs are chosen by the test.
fn mulligan_with(seed: &str, def_ids: &[&str]) -> GameState {
    let mut state = clone(&dealt_game(seed));
    for (at, card) in state.players.p1.hand.iter_mut().enumerate() {
        card.def_id = def_ids[at % def_ids.len()].to_string();
    }
    state
}

mod the_mulligan_at_its_bounds_b20 {
    use super::*;

    #[test]
    fn b20_a_hand_of_cards_costing_exactly_keep_max_cost_is_kept_whole() {
        register_cards();
        let three = "core-019"; // Midrange Menace, 3
        assert_eq!(query_cost(def_of(None, three)), AI_MULLIGAN.keep_max_cost);
        let state = mulligan_with("decide-mulligan-keep-all", &[three]);
        let decision = must_decide(&state, PlayerId::P1, "decide-mulligan-keep-all");
        let kept = match &decision.action {
            ActionBody::Mulligan { keep } => Some(sorted(keep.clone())),
            _ => None,
        };
        assert_eq!(kept, Some(sorted(hand_ids(&state, PlayerId::P1))));
        act(&state, PlayerId::P1, &decision.action);
    }

    #[test]
    fn b20_a_hand_of_cards_costing_more_than_keep_max_cost_is_returned_whole() {
        register_cards();
        let four = "core-025"; // 4-mana 7/7
        assert!(query_cost(def_of(None, four)) > AI_MULLIGAN.keep_max_cost);
        let state = mulligan_with("decide-mulligan-return-all", &[four]);
        let decision = must_decide(&state, PlayerId::P1, "decide-mulligan-return-all");
        assert_eq!(decision.reason, DecisionReason::Mulligan);
        assert_eq!(decision.action, ActionBody::Mulligan { keep: vec![] });
        // Sealed until p2 answers (R265); p2's answer resolves both, and every card p1 held goes back.
        let sealed = act(&state, PlayerId::P1, &decision.action);
        let resolved = reduce(
            &sealed,
            &Action::new(
                ActionBody::Mulligan {
                    keep: hand_ids(&sealed, PlayerId::P2),
                },
                PlayerId::P2,
                "decide-mulligan-return-all-p2",
            ),
        );
        assert_eq!(resolved.error, None);
        let shuffled = ids_where(&resolved.events, PlayerId::P1, true);
        assert_eq!(sorted(shuffled), sorted(hand_ids(&state, PlayerId::P1)));
    }
}

// ---------------------------------------------------------------------------------------------
// B21, R188
// ---------------------------------------------------------------------------------------------

mod draw_offers_b21 {
    use super::*;

    #[test]
    fn r188_b21_with_the_opponents_offer_unanswered_the_ai_owes_an_action_and_declines_it_at_once() {
        let before = humans_turn("decide-offer");
        assert!(!unanswered_draw_offer(&before, AI));
        assert!(!ai_to_act(&before, AI));

        let offered = act(&before, HUMAN, ActionBody::OfferDraw);
        assert!(unanswered_draw_offer(&offered, AI));
        assert!(!unanswered_draw_offer(&offered, HUMAN));
        assert!(ai_to_act(&offered, AI));

        let mut options = AiOptions::new(create_rng("decide-offer", 0));
        let decision = decide(&offered, AI, &mut options);
        assert_eq!(
            decision.as_ref().map(|d| d.action.clone()),
            Some(ActionBody::AnswerDraw { accept: false })
        );
        assert_eq!(
            decision.as_ref().map(|d| d.reason),
            Some(DecisionReason::DrawOffer)
        );
        assert_eq!(decision.as_ref().map(|d| d.stats.score), Some(0.0));
        assert_eq!(options.rng.cursor(), 0);

        let answered = act(&offered, AI, &decision.expect("a decision").action);
        assert!(answered.result.is_none());
        assert_eq!(answered.active, HUMAN);
        // Answered once: the AI owes nothing more, so it cannot loop on the same offer.
        assert!(!unanswered_draw_offer(&answered, AI));
        assert!(!ai_to_act(&answered, AI));
        assert!(
            decide(
                &answered,
                AI,
                &mut AiOptions::new(create_rng("decide-offer-after", 0))
            )
            .is_none()
        );
    }

    #[test]
    fn r188_b21_once_declined_the_human_cannot_offer_again_at_once_r36_so_the_ai_is_not_asked_twice() {
        let offered = act(&humans_turn("decide-offer-block"), HUMAN, ActionBody::OfferDraw);
        let declined = act(
            &offered,
            AI,
            &must_decide(&offered, AI, "decide-offer-block").action,
        );
        assert!(
            !legal_actions(&declined, HUMAN)
                .iter()
                .any(|action| action.action_type() == ActionType::OfferDraw)
        );
        assert!(!unanswered_draw_offer(&declined, AI));
    }

    #[test]
    fn r188_b21_an_offer_from_an_earlier_turn_is_not_unanswered_on_this_one() {
        let offered = act(&humans_turn("decide-offer-stale"), HUMAN, ActionBody::OfferDraw);
        let next = act(&offered, HUMAN, ActionBody::EndTurn);
        assert_eq!(next.active, AI);
        assert!(!unanswered_draw_offer(&next, AI));
        let decision = must_decide(&next, AI, "decide-offer-stale");
        assert_ne!(decision.action.action_type(), ActionType::AnswerDraw);
        assert_ne!(decision.reason, DecisionReason::DrawOffer);
    }

    #[test]
    fn r188_b21_while_the_offering_players_own_prompt_is_open_the_offer_does_not_yet_ask_the_ai() {
        register_cards();
        let s = scenario(json!({
            "seed": "decide-offer-prompt",
            "active": HUMAN,
            "turn": 10,
            "p1": { "hand": ["core-008"], "field": ["core-011"] },
            "p2": { "hand": ["core-072", "core-011"], "graveyard": ["core-008", "core-044"], "library": ["core-005"] },
        }));
        let offered = act(s.state(), HUMAN, ActionBody::OfferDraw);
        let sixth_sense = s.card("core-072").id.clone();
        let play = legal_actions(&offered, HUMAN)
            .into_iter()
            .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == sixth_sense))
            .expect("the play is legal");
        let prompted = act(&offered, HUMAN, play);
        assert_eq!(
            prompted.pending.as_ref().map(|pending| pending.player_id),
            Some(HUMAN)
        );
        assert!(!unanswered_draw_offer(&prompted, AI));
        assert!(!ai_to_act(&prompted, AI));
        assert!(
            decide(
                &prompted,
                AI,
                &mut AiOptions::new(create_rng("decide-offer-prompt", 0))
            )
            .is_none()
        );
    }

    #[test]
    fn r188_b21_the_player_who_offered_never_answers_their_own_offer() {
        let offered = act(&humans_turn("decide-offer-self"), HUMAN, ActionBody::OfferDraw);
        assert!(!unanswered_draw_offer(&offered, HUMAN));
        let decision = must_decide(&offered, HUMAN, "decide-offer-self");
        assert_ne!(decision.action.action_type(), ActionType::AnswerDraw);
        assert_ne!(decision.reason, DecisionReason::DrawOffer);
    }

    #[test]
    fn r188_b21_the_decline_is_legal_for_the_ai_and_an_accept_is_never_chosen_whatever_the_ais_rng() {
        let offered = act(&humans_turn("decide-offer-many"), HUMAN, ActionBody::OfferDraw);
        for k in 0..5 {
            let decision = must_decide(&offered, AI, &format!("decide-offer-many:{k}"));
            assert_eq!(decision.action, ActionBody::AnswerDraw { accept: false });
            assert!(is_legal(&offered, AI, &decision.action));
        }
    }

    #[test]
    fn r188_b21_across_a_game_against_a_player_who_offers_a_draw_at_every_chance_the_ai_declines_each_once_and_never_concedes_or_offers()
     {
        let seed = "decide-offerer";
        let mut state = dealt_game(seed);
        let mut ai_options = AiOptions::with_budget(create_rng(&format!("{seed}:ai"), 0), AI_GATE_BUDGET);
        let mut policy = create_rng(&format!("{seed}:human"), 0);
        let mut ai_actions: Vec<ActionBody> = Vec::new();
        let mut offers = 0;

        let mut n = 0;
        while state.result.is_none() && n < 300 {
            let actor = if ai_to_act(&state, AI) {
                AI
            } else {
                seat_to_act(&state).expect("a live game has a seat to act")
            };
            let body: Option<ActionBody> = if actor == AI {
                let decision = decide(&state, AI, &mut ai_options);
                assert!(decision.is_some(), "action {n}");
                let body = decision.expect("checked above").action;
                ai_actions.push(body.clone());
                Some(body)
            } else {
                let offer = legal_actions(&state, HUMAN)
                    .into_iter()
                    .find(|action| action.action_type() == ActionType::OfferDraw);
                if offer.is_some() {
                    offers += 1;
                }
                offer.or_else(|| subsystems::choose_action(&state, HUMAN, &mut policy))
            };
            let Some(body) = body else {
                break;
            };
            state = act_with_nonce(&state, actor, body, &format!("offerer-{n}"));
            n += 1;
        }

        assert!(offers > 0);
        let still_open = if unanswered_draw_offer(&state, AI) { 1 } else { 0 };
        let answers: Vec<&ActionBody> = ai_actions
            .iter()
            .filter(|action| action.action_type() == ActionType::AnswerDraw)
            .collect();
        assert_eq!(answers.len(), offers - still_open);
        for answer in answers {
            assert_eq!(*answer, ActionBody::AnswerDraw { accept: false });
        }
        let forbidden: Vec<&ActionBody> = ai_actions
            .iter()
            .filter(|action| matches!(action.action_type(), ActionType::Concede | ActionType::OfferDraw))
            .collect();
        assert!(forbidden.is_empty(), "{forbidden:?}");
    }

    #[test]
    fn r188_b21_in_gate_games_the_ais_log_holds_no_concede_no_offer_draw_and_no_accepting_answer_draw() {
        register_cards();
        for n in [1, 2] {
            let config = game_config(Matchup::AiVsRandom, n, AI_GATE_BUDGET, AI_GATE.seed_series);
            let subject = [PlayerId::P1, PlayerId::P2]
                .into_iter()
                .find(|&seat| matches!(config.controllers[seat], SeatController::Ai { .. }))
                .expect("gameConfig seated no AI");
            let record = play_match(&config, &mut MatchHooks::default());
            let ai_actions: Vec<&Action> = record
                .log
                .iter()
                .filter(|action| action.player_id == subject)
                .collect();
            assert!(!ai_actions.is_empty(), "game {n}");
            for action in ai_actions {
                assert_ne!(action.action_type(), ActionType::Concede, "game {n}");
                assert_ne!(action.action_type(), ActionType::OfferDraw, "game {n}");
                if let ActionBody::AnswerDraw { accept } = action.body {
                    assert!(!accept, "game {n}");
                }
            }
        }
    }
}
