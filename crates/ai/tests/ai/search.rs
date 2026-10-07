//! Candidate moves and the search's contract (docs/polish/3-ai.md B14, B15).
//!
//! B14: `candidateActions` is `legalActions` minus the four action types the AI never searches
//! (R84's concede, offerDraw and answerDraw, and the mulligan), with plays that differ only in their
//! lane collapsed to the lowest and the highest, and endTurn last whenever it is legal.
//!
//! B15: `decide` is a pure function of (state, seat, rng seed, budget): the same inputs give a
//! deep-equal Decision, it never spends more nodes than the budget, its action is always legal, and
//! a wall clock that says stop at the first poll still yields a legal action instead of a throw.
//!
//! Port of `packages/ai/test/search.test.ts`. TS's per-test `{ timeout }` has no `cargo test` twin.

use std::cell::Cell;

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, act, dealt_game, is_legal};

const SKIPPED: &[ActionType] = &[ActionType::Concede, ActionType::OfferDraw, ActionType::AnswerDraw, ActionType::Mulligan];

/// TS's `{ rng: createRng(seed), budget }`, with no clock.
fn ai_options(seed: &str, budget: SearchBudget) -> AiOptions<'static> {
    AiOptions { rng: create_rng(seed, 0), budget, should_stop: None }
}

/// A value as its JSON: `SearchStats.stoppedBy` is compared by its literal, whatever its Rust name.
fn js<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

/// An action body from TS's object literal.
fn body(literal: Value) -> ActionBody {
    json_as(literal)
}

/// A mid-game p1 turn with plays in several lanes, a targeted spell, attacks and hidden p2 cards.
fn mid_game() -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "search-mid",
        "p1": { "hand": ["core-008", "core-035"], "field": ["core-011"], "library": ["core-020", "core-053"] },
        "p2": {
            "hand": ["core-005", "core-016"],
            "field": ["core-008"],
            "backrow": [{ "def": "core-041", "faceUp": false }],
            "library": ["core-019", "core-047"],
            "health": 20,
        },
    }))
    .state()
    .clone()
}

/// A play with its zone removed: the "otherwise identical" key B14 groups by.
fn without_zone(action: &ActionBody) -> String {
    match action {
        ActionBody::Play { .. } => {
            let mut rest = action.clone();
            if let ActionBody::Play { zone, .. } = &mut rest {
                *zone = None;
            }
            action_key(&rest)
        }
        _ => action_key(action),
    }
}

fn lane_of(action: &ActionBody) -> Option<i32> {
    match action {
        ActionBody::Play { zone: Some(zone), .. } => Some(zone.lane),
        _ => None,
    }
}

fn is_play(action: &ActionBody) -> bool {
    matches!(action, ActionBody::Play { .. })
}

// ---------------------------------------------------------------------------------------------
// B14
// ---------------------------------------------------------------------------------------------

mod candidate_actions_b14 {
    use super::*;

    /// B14: actionKey is canonical: key order does not matter, and different actions differ
    #[test]
    fn b14_action_key_is_canonical_key_order_does_not_matter_and_different_actions_differ() {
        assert_eq!(
            action_key(&body(json!({ "type": "attack", "attackerId": "c1", "targetId": "hero-p2" }))),
            action_key(&body(json!({ "targetId": "hero-p2", "attackerId": "c1", "type": "attack" }))),
        );
        assert_ne!(
            action_key(&body(json!({ "type": "attack", "attackerId": "c1", "targetId": "hero-p2" }))),
            action_key(&body(json!({ "type": "attack", "attackerId": "c1", "targetId": "c9" }))),
        );
        assert_ne!(
            action_key(&body(json!({ "type": "play", "instanceId": "c1", "zone": { "row": "units", "lane": 2 } }))),
            action_key(&body(json!({ "type": "play", "instanceId": "c1", "zone": { "row": "units", "lane": 3 } }))),
        );
    }

    /// B14: is legalActions minus concede, offerDraw, answerDraw and mulligan, zones collapsed, endTurn last
    #[test]
    fn b14_is_legal_actions_minus_concede_offer_draw_answer_draw_and_mulligan_zones_collapsed_end_turn_last() {
        let state = mid_game();
        let legal = legal_actions(&state, AI);
        let candidates = candidate_actions(&state, AI);
        let legal_keys: IndexSet<String> = legal.iter().map(|action| action_key(action)).collect();

        // Nothing that is not legal, and none of the four skipped types.
        for action in &candidates {
            assert!(legal_keys.contains(&action_key(action)), "{}", action_key(action));
            assert!(!SKIPPED.contains(&action.action_type()));
        }
        // No duplicates.
        let candidate_keys: IndexSet<String> = candidates.iter().map(|action| action_key(action)).collect();
        assert_eq!(candidate_keys.len(), candidates.len());

        // Every legal non-play action of a kept type is a candidate.
        for action in &legal {
            if is_play(action) || SKIPPED.contains(&action.action_type()) {
                continue;
            }
            assert!(candidate_keys.contains(&action_key(action)), "{}", action_key(action));
        }

        // Plays: per otherwise-identical play, exactly the lowest and the highest lane legal offers.
        let mut groups: IndexMap<String, Vec<ActionBody>> = IndexMap::new();
        for action in &legal {
            if !is_play(action) {
                continue;
            }
            groups.entry(without_zone(action)).or_default().push(action.clone());
        }
        assert!(groups.values().any(|group| group.len() > 2), "some play is offered in 3+ lanes");
        for (key, group) in &groups {
            let offered: Vec<&ActionBody> =
                candidates.iter().filter(|action| is_play(action) && without_zone(action) == *key).collect();
            let lanes: Vec<i32> = group.iter().filter_map(lane_of).collect();
            if lanes.is_empty() {
                let offered_keys: Vec<String> = offered.iter().map(|action| action_key(action)).collect();
                let group_keys: Vec<String> = group.iter().map(|action| action_key(action)).collect();
                assert_eq!(offered_keys, group_keys, "{key}");
                continue;
            }
            let low = lanes.iter().copied().min().unwrap_or(0);
            let high = lanes.iter().copied().max().unwrap_or(0);
            let mut expected: Vec<Option<i32>> =
                [low, high].into_iter().collect::<IndexSet<i32>>().into_iter().map(Some).collect();
            expected.sort_by_key(|lane| lane.unwrap_or(0));
            let mut offered_lanes: Vec<Option<i32>> = offered.iter().map(|action| lane_of(action)).collect();
            offered_lanes.sort_by_key(|lane| lane.unwrap_or(0));
            assert_eq!(offered_lanes, expected, "{key}");
        }

        // endTurn is last, once.
        assert_eq!(candidates.iter().filter(|action| **action == ActionBody::EndTurn).count(), 1);
        assert_eq!(candidates.last(), Some(&ActionBody::EndTurn));
    }

    /// B14: a play with one free lane keeps that lane, and one with two free lanes keeps both
    #[test]
    fn b14_a_play_with_one_free_lane_keeps_that_lane_and_one_with_two_free_lanes_keeps_both() {
        jackioh_cards::register_all();
        let one = scenario(json!({
            "seed": "search-one-lane",
            "p1": { "hand": ["core-008"], "field": ["core-011", "core-020", "core-019", "core-025"] },
            "p2": { "hand": ["core-005"] },
        }))
        .state()
        .clone();
        let one_lanes: Vec<Option<i32>> =
            candidate_actions(&one, AI).iter().filter(|action| is_play(action)).map(lane_of).collect();
        assert_eq!(one_lanes, vec![Some(5)]);

        let two = scenario(json!({
            "seed": "search-two-lanes",
            "p1": { "hand": ["core-008"], "field": ["core-011", "core-020", "core-019"] },
            "p2": { "hand": ["core-005"] },
        }))
        .state()
        .clone();
        let mut two_lanes: Vec<Option<i32>> =
            candidate_actions(&two, AI).iter().filter(|action| is_play(action)).map(lane_of).collect();
        two_lanes.sort_by_key(|lane| lane.unwrap_or(0));
        assert_eq!(two_lanes, vec![Some(4), Some(5)]);
    }

    /// B14: an attack on the enemy hero leads, plays come before position switches, and endTurn trails
    #[test]
    fn b14_an_attack_on_the_enemy_hero_leads_plays_come_before_position_switches_and_end_turn_trails() {
        let state = mid_game();
        let candidates = candidate_actions(&state, AI);
        let first = candidates.first();
        assert_eq!(first.map(|action| action.action_type()), Some(ActionType::Attack));
        let target = match first {
            Some(ActionBody::Attack { target_id, .. }) => Some(target_id.as_str()),
            _ => None,
        };
        assert_eq!(target, Some("hero-p2"));

        let last_play = candidates.iter().enumerate().filter(|(_, action)| is_play(action)).map(|(at, _)| at).max();
        let first_switch =
            candidates.iter().position(|action| matches!(action, ActionBody::SwitchPosition { .. }));
        if let Some(first_switch) = first_switch {
            // TS's `Math.max(...[])` is -Infinity, so with no play any switch comes after.
            assert!(last_play.is_none_or(|last_play| first_switch > last_play));
        }
        assert_eq!(
            candidates.iter().position(|action| *action == ActionBody::EndTurn),
            Some(candidates.len() - 1)
        );
    }

    /// B14: a seat answering its own prompt gets exactly the prompt's answers and no endTurn
    #[test]
    fn b14_a_seat_answering_its_own_prompt_gets_exactly_the_prompts_answers_and_no_end_turn() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "search-prompt",
            "p1": { "hand": ["core-072"], "graveyard": ["core-044", "core-008", "core-005"], "library": ["core-011"] },
            "p2": { "hand": ["core-005"] },
        }));
        s.play("core-072", json!({}));
        let state = s.state().clone();
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(AI));

        let candidates = candidate_actions(&state, AI);
        // R211 offers concede beside the prompt's answers, and the AI never takes it (R84, R188).
        let answers: Vec<ActionBody> =
            legal_actions(&state, AI).into_iter().filter(|action| *action != ActionBody::Concede).collect();
        let mut candidate_keys: Vec<String> = candidates.iter().map(|action| action_key(action)).collect();
        candidate_keys.sort();
        let mut answer_keys: Vec<String> = answers.iter().map(|action| action_key(action)).collect();
        answer_keys.sort();
        assert_eq!(candidate_keys, answer_keys);
        assert!(candidates.iter().all(|action| matches!(action, ActionBody::Answer { .. })));
        assert_eq!(candidates.len(), 3);
    }

    /// B14: during the mulligan there is no candidate, since the mulligan is not searched
    #[test]
    fn b14_during_the_mulligan_there_is_no_candidate_since_the_mulligan_is_not_searched() {
        let state = dealt_game("search-mulligan");
        assert_eq!(mulligan_prompt_for(&state, AI).map(|prompt| prompt.kind), Some(PromptKind::Mulligan));
        assert!(!legal_actions(&state, AI).is_empty());
        assert_eq!(candidate_actions(&state, AI), Vec::<ActionBody>::new());
    }

    /// B14: a seat facing an opponent's draw offer on the opponent's turn has no candidate
    #[test]
    fn b14_a_seat_facing_an_opponents_draw_offer_on_the_opponents_turn_has_no_candidate() {
        jackioh_cards::register_all();
        let s = scenario(json!({
            "seed": "search-offer",
            "active": "p2",
            "turn": 10,
            "p1": { "hand": ["core-008"] },
            "p2": { "hand": ["core-011"] },
        }));
        let offered = act(s.state(), PlayerId::P2, &ActionBody::OfferDraw);
        assert!(legal_actions(&offered, AI).iter().any(|action| matches!(action, ActionBody::AnswerDraw { .. })));
        assert_eq!(candidate_actions(&offered, AI), Vec::<ActionBody>::new());
    }

    /// B14: a finished game has no candidate for either seat
    #[test]
    fn b14_a_finished_game_has_no_candidate_for_either_seat() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "search-over",
            "p1": { "field": ["core-011"], "hand": ["core-008"] },
            "p2": { "health": 3, "hand": ["core-005"] },
        }));
        s.attack("core-011", "hero");
        assert!(s.state().result.is_some());
        assert_eq!(candidate_actions(s.state(), AI), Vec::<ActionBody>::new());
        assert_eq!(candidate_actions(s.state(), PlayerId::P2), Vec::<ActionBody>::new());
    }

    /// B14: on the opponent's turn with nothing to answer there is no candidate
    #[test]
    fn b14_on_the_opponents_turn_with_nothing_to_answer_there_is_no_candidate() {
        jackioh_cards::register_all();
        let state = scenario(json!({ "seed": "search-idle", "active": "p2", "turn": 10, "p1": { "hand": ["core-008"] } }))
            .state()
            .clone();
        assert_eq!(candidate_actions(&state, AI), Vec::<ActionBody>::new());
    }
}

// ---------------------------------------------------------------------------------------------
// B15
// ---------------------------------------------------------------------------------------------

mod decides_determinism_and_budget_b15 {
    use super::*;

    /// TS's loop over `[["AI_GATE_BUDGET", AI_GATE_BUDGET], ["AI_BUDGET", AI_BUDGET]]`: the one body both
    /// generated tests run.
    fn same_inputs_same_decision(budget: SearchBudget) {
        let state = mid_game();
        let before = hash_state(&state);
        let first = decide(&state, AI, &mut ai_options("search-b15", budget));
        let second = decide(&state, AI, &mut ai_options("search-b15", budget));

        assert!(first.is_some());
        assert_eq!(second, first);
        let decision = first.expect("a decision");
        assert!(decision.stats.nodes <= budget.nodes);
        assert!(decision.stats.nodes > 0);
        assert!(is_legal(&state, AI, &decision.action));
        assert!(!decision.line.is_empty());
        assert_eq!(action_key(&decision.line[0]), action_key(&decision.action));
        // decide read the state and changed nothing.
        assert_eq!(hash_state(&state), before);
    }

    /// B15: at AI_GATE_BUDGET the same state, seat, rng seed and budget give a deep-equal, legal, in-budget decision
    #[test]
    fn b15_at_ai_gate_budget_the_same_state_seat_rng_seed_and_budget_give_a_deep_equal_legal_in_budget_decision() {
        same_inputs_same_decision(AI_GATE_BUDGET);
    }

    /// B15: at AI_BUDGET the same state, seat, rng seed and budget give a deep-equal, legal, in-budget decision
    #[test]
    fn b15_at_ai_budget_the_same_state_seat_rng_seed_and_budget_give_a_deep_equal_legal_in_budget_decision() {
        same_inputs_same_decision(AI_BUDGET);
    }

    /// B15: omitting the budget is AI_BUDGET
    ///
    /// Rust has no omitted field: `AiOptions.budget` defaults to AI_BUDGET where the caller builds it
    /// (SURFACE §9), so the "implicit" decision is the one TS's `{ rng }` alone gave, built the way every
    /// caller without a budget builds it.
    #[test]
    fn b15_omitting_the_budget_is_ai_budget() {
        let state = mid_game();
        let implicit = decide(&state, AI, &mut AiOptions { rng: create_rng("search-default", 0), budget: AI_BUDGET, should_stop: None });
        let explicit = decide(&state, AI, &mut ai_options("search-default", AI_BUDGET));
        assert_eq!(implicit, explicit);
    }

    /// B15: a clock that says stop at the first poll still gives a legal action, stopped by the clock, with no node spent
    #[test]
    fn b15_a_clock_that_says_stop_at_the_first_poll_still_gives_a_legal_action_stopped_by_the_clock_with_no_node_spent() {
        let state = mid_game();
        let polls = Cell::new(0);
        let stop = || {
            polls.set(polls.get() + 1);
            true
        };
        let decision = decide(
            &state,
            AI,
            &mut AiOptions { rng: create_rng("search-clock", 0), budget: AI_GATE_BUDGET, should_stop: Some(&stop) },
        );
        assert!(decision.is_some());
        assert!(polls.get() > 0);
        let decision = decision.expect("a decision");
        assert_eq!(js(decision.stats.stopped_by), json!("clock"));
        assert_eq!(decision.stats.nodes, 0);
        assert!(is_legal(&state, AI, &decision.action));
    }

    /// B15: a clock that stops after N polls caps the nodes at N
    #[test]
    fn b15_a_clock_that_stops_after_n_polls_caps_the_nodes_at_n() {
        let state = mid_game();
        let limit = 25;
        let polls = Cell::new(0);
        let stop = || {
            polls.set(polls.get() + 1);
            polls.get() > limit
        };
        let decision = decide(
            &state,
            AI,
            &mut AiOptions { rng: create_rng("search-clock-n", 0), budget: AI_BUDGET, should_stop: Some(&stop) },
        )
        .expect("a decision");
        assert_eq!(js(decision.stats.stopped_by), json!("clock"));
        assert!(decision.stats.nodes <= limit);
        assert!(is_legal(&state, AI, &decision.action));
    }

    /// B15: a budget of zero nodes falls back to endTurn, stopped by the budget
    #[test]
    fn b15_a_budget_of_zero_nodes_falls_back_to_end_turn_stopped_by_the_budget() {
        let state = mid_game();
        let empty = SearchBudget { nodes: 0, lethal_nodes: 0, ..AI_GATE_BUDGET };
        let decision = decide(&state, AI, &mut ai_options("search-empty", empty)).expect("a decision");
        assert_eq!(decision.reason, DecisionReason::Fallback);
        assert_eq!(decision.action, ActionBody::EndTurn);
        assert_eq!(decision.stats.nodes, 0);
        assert_eq!(js(decision.stats.stopped_by), json!("budget"));
    }

    /// B15: the smallest search (one world, a beam of one, one step deep) still gives a legal, in-budget decision
    #[test]
    fn b15_the_smallest_search_one_world_a_beam_of_one_one_step_deep_still_gives_a_legal_in_budget_decision() {
        let state = mid_game();
        let tiny = SearchBudget {
            nodes: 12,
            lethal_nodes: 2,
            determinizations: 1,
            beam_width: 1,
            root_branching: 1,
            branching: 1,
            max_depth: 1,
            finalists: 1,
        };
        let decision = decide(&state, AI, &mut ai_options("search-tiny", tiny));
        assert!(decision.is_some());
        let decision = decision.expect("a decision");
        assert!(decision.stats.nodes <= tiny.nodes);
        assert!(is_legal(&state, AI, &decision.action));
    }

    /// B15: with no lethal budget the solver finds nothing, yet a lethal board still gets a legal decision
    #[test]
    fn b15_with_no_lethal_budget_the_solver_finds_nothing_yet_a_lethal_board_still_gets_a_legal_decision() {
        jackioh_cards::register_all();
        let state = scenario(json!({
            "seed": "search-no-lethal",
            "p1": { "field": ["core-011", "core-008"] },
            "p2": { "health": 6 },
        }))
        .state()
        .clone();
        let budget = SearchBudget { lethal_nodes: 0, ..AI_GATE_BUDGET };
        let decision = decide(&state, AI, &mut ai_options("search-no-lethal", budget)).expect("a decision");
        assert_ne!(decision.reason, DecisionReason::Lethal);
        assert!(decision.stats.nodes <= budget.nodes);
        assert!(is_legal(&state, AI, &decision.action));
    }

    /// B15: decide never throws on a state whose hidden cards it cannot know, across many rng seeds
    #[test]
    fn b15_decide_never_throws_on_a_state_whose_hidden_cards_it_cannot_know_across_many_rng_seeds() {
        let state = mid_game();
        for k in 0..6 {
            let decision = decide(&state, AI, &mut ai_options(&format!("search-many:{k}"), AI_GATE_BUDGET));
            assert!(decision.is_some(), "seed {k}");
            let decision = decision.expect("a decision");
            assert!(is_legal(&state, AI, &decision.action), "seed {k}");
            assert!(decision.stats.nodes <= AI_GATE_BUDGET.nodes, "seed {k}");
        }
    }

    /// B15: the node counter grants exactly its limit, then reports the budget
    #[test]
    fn b15_the_node_counter_grants_exactly_its_limit_then_reports_the_budget() {
        let counter = create_node_counter(3, None);
        assert_eq!([counter.take(), counter.take(), counter.take()], [true, true, true]);
        assert!(!counter.take());
        assert_eq!(counter.used(), 3);
        assert_eq!(counter.limit(), 3);
        assert_eq!(js(counter.stopped_by()), json!("budget"));

        let none = create_node_counter(0, None);
        assert!(!none.take());
        assert_eq!(none.used(), 0);
    }

    /// B15: the node counter polls the clock before each node and reports it
    #[test]
    fn b15_the_node_counter_polls_the_clock_before_each_node_and_reports_it() {
        let polls = Cell::new(0);
        let stop = || {
            polls.set(polls.get() + 1);
            polls.get() > 2
        };
        let counter = create_node_counter(10, Some(&stop));
        assert!(counter.take());
        assert!(counter.take());
        assert!(!counter.take());
        assert_eq!(counter.used(), 2);
        assert_eq!(js(counter.stopped_by()), json!("clock"));
    }
}
