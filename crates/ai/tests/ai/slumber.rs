//! Dormant in the AI's reads (R1839000): the candidates come from `legal_actions`, so a Dormant card's
//! attack, switch and abilities, and an attack on one, are never among them, and neither `decide` nor the
//! random policy ever acts through it; it is no threat until it wakes (`active_units_of` leaves it out
//! of `face_threat`); the evaluation prices it by the unit it wakes as, less for each turn it waits
//! (R2380000); and the search signature tells a position holding one apart from one without it.
//!
//! Most boards put a unit on the field to sleep by writing its status, as `slumber::start_slumber_on_field`
//! would. The cards: Mr. Vanilla 4/4, Tempo Timmy 3/3 and Imprisoned Cosmality, (2) 10/6 Pierce, Dormant for
//! 2 turns.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards};

const LIBRARY: &[&str] = &["core-008", "core-011", "core-008", "core-011"];

fn board() -> GameState {
    register_cards();
    scenario(json!({
        "seed": "ai-dormant",
        "p1": { "hand": ["core-011"], "field": ["core-008"], "library": LIBRARY },
        "p2": { "hand": ["core-011"], "field": ["core-011"], "library": LIBRARY },
    }))
    .state()
    .clone()
}

fn sleep(state: &mut GameState, player: PlayerId) -> String {
    let id = active_units_of(state, player)[0].id.clone();
    find_instance_mut(state, &id).expect("the unit").slumber = Some(Slumber { turns: Some(2) });
    id
}

fn emptied(state: &GameState, player: PlayerId) -> GameState {
    let mut out = state.clone();
    out.players[player].units[0] = None;
    out
}

fn names(action: &ActionBody, id: &str) -> bool {
    serde_json::to_string(action)
        .expect("serializes")
        .contains(&format!("\"{id}\""))
}

#[test]
fn r1839000_a_dormant_card_is_never_in_a_candidate_as_actor_or_target() {
    let mut state = board();
    let own = active_units_of(&state, AI)[0].id.clone();
    let theirs = active_units_of(&state, HUMAN)[0].id.clone();
    assert!(
        candidate_actions(&state, AI, Prices::NONE)
            .iter()
            .any(|action| names(action, &own))
    );
    assert!(
        candidate_actions(&state, AI, Prices::NONE)
            .iter()
            .any(|action| names(action, &theirs))
    );
    sleep(&mut state, AI);
    sleep(&mut state, HUMAN);
    assert!(
        !candidate_actions(&state, AI, Prices::NONE)
            .iter()
            .any(|action| names(action, &own) || names(action, &theirs)),
        "neither the AI's Dormant card nor the human's is in any candidate"
    );
}

#[test]
fn r1839000_decide_and_the_random_policy_never_act_through_a_dormant_card() {
    let mut state = board();
    let own = sleep(&mut state, AI);
    let theirs = sleep(&mut state, HUMAN);
    let decision = decide(&state, AI, &mut AiOptions::new(create_rng("ai-dormant", 0))).expect("a decision");
    assert!(
        !names(&decision.action, &own) && !names(&decision.action, &theirs),
        "{:?}",
        decision.action
    );
    for cursor in 0..32 {
        let mut rng = create_rng("ai-dormant-policy", cursor);
        let action = subsystems::ai_policy::choose_action(&state, AI, &mut rng).expect("an action");
        assert!(!names(&action, &own) && !names(&action, &theirs), "{action:?}");
    }
}

#[test]
fn r1839000_a_dormant_card_is_no_threat_until_it_wakes() {
    let awake = board();
    let mut dormant = awake.clone();
    sleep(&mut dormant, HUMAN);
    assert!(face_threat(&awake, HUMAN) > 0);
    assert_eq!(face_threat(&dormant, HUMAN), 0);
    assert!(eval(&dormant, &AI_EVAL) > eval(&awake, &AI_EVAL));
}

#[test]
fn r1839000_two_positions_that_differ_only_in_a_dormant_card_sign_apart() {
    let awake = board();
    let mut dormant = awake.clone();
    let id = sleep(&mut dormant, HUMAN);
    let gone = emptied(&awake, HUMAN);
    assert_ne!(search_signature(&dormant, AI), search_signature(&gone, AI));
    assert_ne!(search_signature(&dormant, AI), search_signature(&awake, AI));
    // Its countdown is part of the position too.
    let mut later = dormant.clone();
    find_instance_mut(&mut later, &id).expect("the unit").slumber = Some(Slumber { turns: Some(1) });
    assert_ne!(search_signature(&dormant, AI), search_signature(&later, AI));
}

const COSMALITY: &str = "stinkyscape-019";

fn eval(state: &GameState, w: &EvalWeights) -> f64 {
    evaluate(state, AI, NextSwing::Enemy, w)
}

/// `state` with `player`'s first unit on the field Dormant with `turns` left on its countdown (`None`:
/// waiting for an effect), and what that unit is worth awake.
fn asleep(state: &GameState, player: PlayerId, turns: Option<i32>) -> (GameState, f64) {
    let mut out = state.clone();
    let worth = unit_worth(state, active_units_of(state, player)[0], &AI_EVAL, Prices::NONE);
    let id = sleep(&mut out, player);
    find_instance_mut(&mut out, &id).expect("the unit").slumber = Some(Slumber { turns });
    (out, worth)
}

/// The AI's turn 3, its second (2 crystals), Imprisoned Cosmality alone in its hand, both boards empty.
fn cosmality_on_curve(tier: Handicap, human_hand: &[&str]) -> GameState {
    register_cards();
    let mut state = scenario(json!({
        "seed": "ai-dormant-curve",
        "turn": 3,
        "p1": { "hand": [COSMALITY], "mana": 2, "library": LIBRARY },
        "p2": { "hand": human_hand, "library": LIBRARY },
    }))
    .state()
    .clone();
    state.players[AI].handicap = Some(tier);
    state
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} vs {b}");
}

#[test]
fn r2380000_an_own_dormant_card_scores_above_an_empty_zone_and_an_enemy_one_below_it() {
    let awake = board();
    let share = AI_EVAL.slumber_turn_share * AI_EVAL.slumber_turn_share;
    // The AI's sleeping Vanilla, two turns from waking, counts a quarter of the Vanilla it wakes as.
    let (own, worth) = asleep(&awake, AI, Some(2));
    let gone = emptied(&awake, AI);
    assert!(eval(&own, &AI_EVAL) > eval(&gone, &AI_EVAL));
    close(eval(&own, &AI_EVAL) - eval(&gone, &AI_EVAL), share * worth);
    // The human's sleeping Timmy counts against the AI alike.
    let (theirs, worth) = asleep(&awake, HUMAN, Some(2));
    let gone = emptied(&awake, HUMAN);
    assert!(eval(&theirs, &AI_EVAL) < eval(&gone, &AI_EVAL));
    close(eval(&gone, &AI_EVAL) - eval(&theirs, &AI_EVAL), share * worth);
}

#[test]
fn r2380000_fewer_turns_left_is_worth_more() {
    let awake = board();
    let own = |turns| eval(&asleep(&awake, AI, Some(turns)).0, &AI_EVAL);
    assert!(own(1) > own(2), "{} vs {}", own(1), own(2));
    assert!(own(2) > own(3), "{} vs {}", own(2), own(3));
    // The enemy's is the nearer danger the sooner it wakes.
    let theirs = |turns| eval(&asleep(&awake, HUMAN, Some(turns)).0, &AI_EVAL);
    assert!(theirs(1) < theirs(2), "{} vs {}", theirs(1), theirs(2));
    assert!(theirs(2) < theirs(3), "{} vs {}", theirs(2), theirs(3));
}

#[test]
fn r2380000_a_dormant_card_with_no_countdown_counts_at_the_waiting_share() {
    let awake = board();
    let (own, worth) = asleep(&awake, AI, None);
    close(
        eval(&own, &AI_EVAL) - eval(&emptied(&awake, AI), &AI_EVAL),
        AI_EVAL.slumber_waiting_share * worth,
    );
}

#[test]
fn r2380000_greedy_eval_still_counts_a_dormant_card_as_nothing() {
    assert_eq!(GREEDY_EVAL.slumber_turn_share, 0.0);
    assert_eq!(GREEDY_EVAL.slumber_waiting_share, 0.0);
    let awake = board();
    for player in [AI, HUMAN] {
        for turns in [Some(2), None] {
            assert_eq!(
                eval(&asleep(&awake, player, turns).0, &GREEDY_EVAL),
                eval(&emptied(&awake, player), &GREEDY_EVAL),
                "{player:?} {turns:?}"
            );
        }
    }
}

#[test]
fn r2380000_decide_plays_imprisoned_cosmality_on_curve_at_medium_and_hard() {
    for tier in [AI_DIFFICULTY.medium, AI_DIFFICULTY.hard] {
        let state = cosmality_on_curve(tier, &["core-011"]);
        let cosmality = state.players[AI].hand[0].id.clone();
        for cursor in 0..2 {
            let decision = decide(
                &state,
                AI,
                &mut AiOptions::new(create_rng("ai-dormant-curve", cursor)),
            )
            .expect("a decision");
            assert!(
                matches!(&decision.action, ActionBody::Play { instance_id, .. } if *instance_id == cosmality),
                "{tier:?} cursor {cursor}: {:?}",
                decision.action
            );
        }
    }
}

#[test]
fn r2380000_positions_that_differ_only_in_a_hidden_card_give_the_same_decision() {
    let decide_on = |state: &GameState| {
        decide(state, AI, &mut AiOptions::new(create_rng("ai-dormant-hidden", 0))).expect("a decision")
    };
    // A sleeping unit on each side, and the human's one hidden hand card told apart. The hands keep
    // their size, which is public.
    let first = asleep(&asleep(&board(), AI, Some(1)).0, HUMAN, Some(2)).0;
    let mut second = first.clone();
    second.players[HUMAN].hand[0].def_id = "core-008".to_string();
    let curve = (
        cosmality_on_curve(AI_DIFFICULTY.hard, &["core-011"]),
        cosmality_on_curve(AI_DIFFICULTY.hard, &["core-008"]),
    );
    for (one, other) in [(first, second), curve] {
        assert_ne!(hash_state(&one), hash_state(&other));
        assert_eq!(decide_on(&one), decide_on(&other));
    }
}
