//! The search's building blocks, each against its contract in docs/polish/3-ai.md §Surface: `lineStatus`
//! and `terminalScore` (simulate.ts), `scoreLine` and `beamSearch` (search.ts), `findLethal`
//! (lethal.ts), decide's "search" and "prompt" reasons and its planned line, `playAiTurn`'s loop and
//! nonces, and sweepCard's clock. The numbered behaviours reach these only through `decide` and the
//! puzzles; here each one is held to its own sentence in the design, so a change that keeps the
//! puzzles green but breaks a contract still fails.
//!
//! Port of `packages/ai/test/surface.test.ts`. TS's per-test `{ timeout }` has no `cargo test` twin.

use std::cell::Cell;

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, is_legal};

const TURN: i32 = 9;

/// `determinize` with TS's default options (`{}`).
fn det(public: &GameState, seat: PlayerId, rng: &mut Rng) -> GameState {
    determinize(public, seat, rng, DeterminizeOptions::default())
}

/// `evaluate(state, seat)`: TS's two defaulted arguments, passed explicitly.
fn eval(state: &GameState, seat: PlayerId) -> f64 {
    evaluate(state, seat, NextSwing::Enemy, &AI_EVAL)
}

/// TS's `{ rng: createRng(seed), budget }`, with no clock.
fn ai_options(seed: &str, budget: SearchBudget) -> AiOptions<'static> {
    AiOptions { rng: create_rng(seed, 0), budget, should_stop: None }
}

/// A value as its JSON, for comparisons that pin the wire shape rather than a Rust type's name.
fn js<T: serde::Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialisable")
}

/// p1's main phase on turn 9: one attacker, a 1-drop in hand (so an attack alone does not leave the
/// turn with nothing to do, which R82 would end on the spot), 3 unspent crystals, vanilla libraries,
/// and p2 far from dead.
fn quiet_turn() -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "surface-quiet",
        "active": "p1",
        "turn": TURN,
        "p1": { "hand": ["core-008"], "field": ["core-011"], "mana": 3, "library": ["core-008", "core-011"] },
        "p2": { "field": ["core-008"], "library": ["core-008", "core-011"], "health": 20 },
    }))
    .state()
    .clone()
}

/// P1's board: two attackers, an empty enemy board and 6 enemy health.
fn lethal_board() -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "surface-lethal",
        "active": "p1",
        "turn": TURN,
        "p1": { "field": ["core-011", "core-008"] },
        "p2": { "health": 6 },
    }))
    .state()
    .clone()
}

fn worlds(state: &GameState, seed: &str, count: usize) -> Vec<GameState> {
    let mut rng = create_rng(seed, 0);
    let public = redact(state, AI);
    (0..count).map(|_| det(&public, AI, &mut rng)).collect()
}

fn hero_attack(state: &GameState) -> ActionBody {
    let hero = format!("hero-{HUMAN}");
    candidate_actions(state, AI)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Attack { target_id, .. } if *target_id == hero))
        .unwrap_or_else(|| panic!("no attack on the enemy hero"))
}

/// The p1 turn ended through the reducer, as terminalScore's own endTurn step plays it (p2's cards
/// are vanilla, so there is no prompt of p2's for simulate to auto-answer). `act` stamps a nonce of
/// its own: two `simulate` calls on fresh counters would both use `sim:1`, and reduce would treat the
/// second as a repeat of the first.
fn ended_turn(state: &GameState) -> GameState {
    act(state, AI, &ActionBody::EndTurn)
}

fn passed_value(ended: &GameState) -> f64 {
    eval(ended, AI) - AI_EVAL.unspent_mana * f64::from(ended.players[AI].turn_log.unspent_at_end.unwrap_or(0))
}

/// TS's module constant `BOGUS`: an attack by a unit that does not exist.
fn bogus() -> ActionBody {
    ActionBody::Attack { attacker_id: "no-such-unit".to_string(), target_id: format!("hero-{HUMAN}") }
}

// ---------------------------------------------------------------------------------------------
// simulate.ts
// ---------------------------------------------------------------------------------------------

mod surface_line_status {
    use super::*;

    /// is open in the seat's own turn, passed once the turn moved on, yielded on the other seat's turn and over at a result
    #[test]
    fn is_open_in_the_seats_own_turn_passed_once_the_turn_moved_on_yielded_on_the_other_seats_turn_and_over_at_a_result() {
        let state = quiet_turn();
        assert!(state.pending.is_none());
        assert_eq!(line_status(&state, AI, TURN), LineStatus::Open);
        assert_eq!(line_status(&state, AI, TURN - 2), LineStatus::Passed);
        assert_eq!(line_status(&state, HUMAN, TURN), LineStatus::Yielded);
        let over = act(&state, AI, &ActionBody::Concede);
        assert_eq!(line_status(&over, AI, TURN), LineStatus::Over);
        assert_eq!(line_status(&over, HUMAN, TURN), LineStatus::Over);
    }
}

mod surface_terminal_score {
    use super::*;

    /// an open line in the seat's main phase ends its turn and pays for the crystals it left
    #[test]
    fn an_open_line_in_the_seats_main_phase_ends_its_turn_and_pays_for_the_crystals_it_left() {
        let state = quiet_turn();
        let ended = ended_turn(&state);
        assert_eq!(ended.players[AI].turn_log.unspent_at_end, Some(3));

        let counter = create_node_counter(10, None);
        assert_eq!(terminal_score(&state, AI, TURN, &counter), passed_value(&ended));
        assert!(counter.used() > 0);
    }

    /// a passed line is the evaluation less the unspent crystals, with no node spent
    #[test]
    fn a_passed_line_is_the_evaluation_less_the_unspent_crystals_with_no_node_spent() {
        let ended = ended_turn(&quiet_turn());
        assert_eq!(line_status(&ended, AI, TURN), LineStatus::Passed);
        let counter = create_node_counter(10, None);
        assert_eq!(terminal_score(&ended, AI, TURN, &counter), passed_value(&ended));
        assert!(passed_value(&ended) < eval(&ended, AI));
        assert_eq!(counter.used(), 0);
    }

    /// a yielded or finished line is the plain evaluation, with no node spent
    #[test]
    fn a_yielded_or_finished_line_is_the_plain_evaluation_with_no_node_spent() {
        let state = quiet_turn();
        let counter = create_node_counter(10, None);
        assert_eq!(terminal_score(&state, HUMAN, TURN, &counter), eval(&state, HUMAN));
        let over = act(&state, AI, &ActionBody::Concede);
        assert_eq!(terminal_score(&over, AI, TURN, &counter), eval(&over, AI));
        assert_eq!(eval(&over, AI), -AI_EVAL.win + f64::from(over.turn));
        assert_eq!(counter.used(), 0);
    }

    /// an open line with no node left for its endTurn is scored where it stands
    #[test]
    fn an_open_line_with_no_node_left_for_its_end_turn_is_scored_where_it_stands() {
        let state = quiet_turn();
        assert_eq!(terminal_score(&state, AI, TURN, &create_node_counter(0, None)), eval(&state, AI));
    }
}

// ---------------------------------------------------------------------------------------------
// search.ts
// ---------------------------------------------------------------------------------------------

mod surface_score_line {
    use super::*;

    /// a line whose first action is not a candidate is cut there and scored by terminalScore
    #[test]
    fn a_line_whose_first_action_is_not_a_candidate_is_cut_there_and_scored_by_terminal_score() {
        let state = quiet_turn();
        assert!(!is_legal(&state, AI, bogus()));
        assert_eq!(
            score_line(&state, AI, &[bogus()], &create_node_counter(20, None), false, None),
            Some(terminal_score(&state, AI, TURN, &create_node_counter(20, None))),
        );
    }

    /// a legal line is played, its turn ended, and the end scored; anything after a non-candidate is dropped
    #[test]
    fn a_legal_line_is_played_its_turn_ended_and_the_end_scored_anything_after_a_non_candidate_is_dropped() {
        let state = quiet_turn();
        let attack = hero_attack(&state);
        let stepped = match simulate(&state, AI, &attack, &create_node_counter(20, None)) {
            Some(Ok(next)) => next,
            _ => panic!("the attack was refused"),
        };
        let expected = passed_value(&ended_turn(&stepped));

        assert_eq!(
            score_line(&state, AI, std::slice::from_ref(&attack), &create_node_counter(20, None), false, None),
            Some(expected)
        );
        assert_eq!(
            score_line(
                &state,
                AI,
                &[attack.clone(), bogus(), ActionBody::EndTurn],
                &create_node_counter(20, None),
                false,
                None
            ),
            Some(expected)
        );
    }

    /// is null when the counter runs out before the line is scored
    #[test]
    fn is_null_when_the_counter_runs_out_before_the_line_is_scored() {
        let state = quiet_turn();
        let attack = hero_attack(&state);
        assert!(score_line(&state, AI, std::slice::from_ref(&attack), &create_node_counter(0, None), false, None).is_none());
        assert!(score_line(&state, AI, std::slice::from_ref(&attack), &create_node_counter(1, None), false, None).is_none());
    }
}

mod surface_beam_search {
    use super::*;

    /// returns complete lines best first, each starting with a candidate, within the counter and maxDepth
    #[test]
    fn returns_complete_lines_best_first_each_starting_with_a_candidate_within_the_counter_and_max_depth() {
        let state = quiet_turn();
        let det = worlds(&state, "surface-beam", 1).into_iter().next().unwrap_or_else(|| panic!("no determinization"));
        let counter = create_node_counter(AI_GATE_BUDGET.nodes, None);
        let lines = beam_search(&det, AI, &counter, AI_GATE_BUDGET);

        assert!(!lines.is_empty());
        assert!(counter.used() <= AI_GATE_BUDGET.nodes);
        let firsts: IndexSet<String> = candidate_actions(&det, AI).iter().map(action_key).collect();
        for (index, line) in lines.iter().enumerate() {
            assert!(!line.actions.is_empty());
            assert!(line.actions.len() <= AI_GATE_BUDGET.max_depth);
            assert!(firsts.contains(&action_key(&line.actions[0])));
            if index > 0 {
                assert!(line.score <= lines[index - 1].score);
            }
        }
        // endTurn is expanded at the root on top of the branching, so ending at once is always a line.
        assert!(lines.iter().any(|line| line.actions.len() == 1 && line.actions[0] == ActionBody::EndTurn));
    }

    /// with no node to spend there is no line
    #[test]
    fn with_no_node_to_spend_there_is_no_line() {
        let det = worlds(&quiet_turn(), "surface-beam-empty", 1)
            .into_iter()
            .next()
            .unwrap_or_else(|| panic!("no determinization"));
        assert!(beam_search(&det, AI, &create_node_counter(0, None), AI_GATE_BUDGET).is_empty());
    }
}

// ---------------------------------------------------------------------------------------------
// lethal.ts
// ---------------------------------------------------------------------------------------------

mod surface_find_lethal {
    use super::*;

    /// finds a line that kills the enemy hero on every determinization, within its node limit
    #[test]
    fn finds_a_line_that_kills_the_enemy_hero_on_every_determinization_within_its_node_limit() {
        let dets = worlds(&lethal_board(), "surface-lethal", 2);
        let counter = create_node_counter(AI_GATE_BUDGET.nodes, None);
        let line = find_lethal(&dets, AI, &counter, AI_GATE_BUDGET.lethal_nodes);
        assert!(line.is_some());
        assert!(counter.used() <= AI_GATE_BUDGET.lethal_nodes);

        for det in &dets {
            let mut state = det.clone();
            // One counter for the whole replay, so every step gets a nonce of its own.
            let replay = create_node_counter(20, None);
            for action in line.iter().flatten() {
                state = match simulate(&state, AI, action, &replay) {
                    Some(Ok(next)) => next,
                    _ => panic!("the lethal line was refused at {}", action.action_type()),
                };
            }
            assert_eq!(state.result.map(|result| result.winner), Some(Winner::from(AI)));
        }
    }

    /// with a limit of 0 it spends nothing and finds nothing
    #[test]
    fn with_a_limit_of_0_it_spends_nothing_and_finds_nothing() {
        let dets = worlds(&lethal_board(), "surface-lethal-zero", 2);
        let counter = create_node_counter(AI_GATE_BUDGET.nodes, None);
        assert!(find_lethal(&dets, AI, &counter, 0).is_none());
        assert_eq!(counter.used(), 0);
    }

    /// finds nothing on a board with no lethal, and stays within its limit
    #[test]
    fn finds_nothing_on_a_board_with_no_lethal_and_stays_within_its_limit() {
        let dets = worlds(&quiet_turn(), "surface-no-lethal", 2);
        let counter = create_node_counter(AI_GATE_BUDGET.nodes, None);
        assert!(find_lethal(&dets, AI, &counter, AI_GATE_BUDGET.lethal_nodes).is_none());
        assert!(counter.used() <= AI_GATE_BUDGET.lethal_nodes);
    }
}

// ---------------------------------------------------------------------------------------------
// decide.ts: the searched reasons
// ---------------------------------------------------------------------------------------------

mod surface_decides_searched_decisions {
    use super::*;

    /// a searched main-phase decision has reason "search" and plays the first action of its planned line
    #[test]
    fn a_searched_main_phase_decision_has_reason_search_and_plays_the_first_action_of_its_planned_line() {
        let state = quiet_turn();
        let decision = decide(&state, AI, &mut ai_options("surface-search", AI_GATE_BUDGET))
            .unwrap_or_else(|| panic!("decide returned null in the AI's main phase"));
        assert_eq!(decision.reason, DecisionReason::Search);
        assert_eq!(decision.line.first(), Some(&decision.action));
        assert!(decision.stats.lines > 0);
        assert_eq!(decision.stats.determinizations, AI_GATE_BUDGET.determinizations);
        assert!(is_legal(&state, AI, &decision.action));
    }

    /// a searched answer to the seat's own prompt has reason "prompt"
    #[test]
    fn a_searched_answer_to_the_seats_own_prompt_has_reason_prompt() {
        jackioh_cards::register_all();
        let mut s = scenario(json!({
            "seed": "surface-prompt",
            "p1": { "hand": ["core-072"], "graveyard": ["core-008", "core-044", "core-005"], "library": ["core-011"] },
            "p2": { "hand": ["core-005"], "field": ["core-008"] },
        }));
        s.play("core-072", json!({}));
        let decision = decide(s.state(), AI, &mut ai_options("surface-prompt", AI_GATE_BUDGET)).expect("a decision");
        assert_eq!(decision.reason, DecisionReason::Prompt);
        assert!(matches!(decision.action, ActionBody::Answer { .. }));
        assert_eq!(decision.line.first(), Some(&decision.action));
    }
}

// ---------------------------------------------------------------------------------------------
// match.ts: one AI turn
// ---------------------------------------------------------------------------------------------

mod surface_play_ai_turn {
    use super::*;

    /// plays decide's actions with nonces t0, t1, … until the AI owes nothing
    #[test]
    fn plays_decides_actions_with_nonces_t0_t1_until_the_ai_owes_nothing() {
        jackioh_cards::register_all();
        let start = scenario(json!({
            "seed": "surface-turn",
            "active": "p1",
            "turn": TURN,
            "p1": { "hand": ["core-025", "core-011"], "mana": 4, "library": ["core-008", "core-011"] },
            "p2": { "library": ["core-008", "core-011"] },
        }))
        .state()
        .clone();
        let turn = play_ai_turn(&start, AI, &mut ai_options("surface-turn", AI_GATE_BUDGET));

        assert!(turn.actions.len() > 1);
        assert_eq!(turn.actions.len(), turn.decisions.len());
        for (n, action) in turn.actions.iter().enumerate() {
            assert_eq!(action.nonce, format!("t{n}"));
            assert_eq!(action.player_id, AI);
            assert_eq!(Some(&action.body), turn.decisions.get(n).map(|decision| &decision.action));
        }
        assert!(!ai_to_act(&turn.state, AI));
    }
}

// ---------------------------------------------------------------------------------------------
// sweep.ts: the clock
// ---------------------------------------------------------------------------------------------

mod surface_sweep_cards_clock {
    use super::*;

    /// a decision the clock times at more than decisionMs raises the timeout flag
    #[test]
    fn a_decision_the_clock_times_at_more_than_decision_ms_raises_the_timeout_flag() {
        jackioh_cards::register_all();
        let t = Cell::new(0.0_f64);
        let slow = || {
            t.set(t.get() + AI_SWEEP.decision_ms as f64 + 1.0);
            t.get()
        };
        let result = sweep_card("core-011", &SweepOptions { seeds: Some(1), now: Some(&slow), tier: None });
        let result = js(&result);
        assert_eq!(result["games"], json!(1));
        assert!(result["timeouts"].as_i64().unwrap_or(0) > 0);
        assert!(result["flags"].as_array().is_some_and(|flags| flags.contains(&json!("timeout"))));
    }
}
