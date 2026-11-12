//! R3695000 (issue #3695): a passed line's Float is credited, at `AI_EVAL.float_discount` a crystal, for
//! each crystal it took toward a "Float reduces cost" card in the seat's own hand that still cost more
//! than the seat's crystals (`float_credit`, read through the engine's `float_cuts_in_hand`). Before it,
//! each crystal so Floated scored `hand_cost_delta − unspent_mana`, −0.3, and the AI never saved mana to
//! bring Drone Core IX or X (S #35, #36) within its four crystals (§2.3, §6.2, R1563000). R3695001: once X
//! is within them, each Drone Battleship layer beneath the pile's top counts `AI_EVAL.dormant_share` of
//! its worth, so Hard casts it over a vanilla body (§3.2, R1842000).
//!
//! Every board is a `scenario()` with the AI as p1 and libraries of uncastable 9-cost stand-ins. p2
//! holds a 1-cost Unit (or a Unit on the field), so its turn has a play and the line's turn log lasts
//! until the AI's next turn.

use jackioh_ai::*;
use jackioh_engine::config::AI_DIFFICULTY;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, close};

/// A card nobody can cast: the libraries' draws change nothing.
const INERT: &str = "stand-in (9) 1/1";
/// A 1-cost 1/1, the issue's one-drop.
const ONE: &str = "stand-in (1) 1/1";
/// A 6-cost card with no text, Drone Core IX's price and no Float discount.
const SIX: &str = "stand-in (6) Spell";
/// S #35 Drone Core IX, 6 mana, "Float reduces cost" in hand.
const DC_IX: &str = "stinkyscape-035";
/// S #36 Drone Core X, 8 mana, "Float reduces cost" in hand.
const DC_X: &str = "stinkyscape-036";
/// The actions a test lets one AI turn take.
const TURN_ACTIONS: usize = 12;

/// p1 to act on `turn` (7, 9 and 11 give it MAX_MANA crystals) with `hand` and `mana` of them left.
fn on_turn(turn: i32, hand: &[&str], mana: i32) -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "ai-float-discount",
        "active": "p1",
        "turn": turn,
        "p1": { "hand": hand, "mana": mana, "library": [INERT, INERT, INERT, INERT] },
        "p2": { "hand": [ONE], "library": [INERT, INERT, INERT, INERT] },
    }))
    .state()
    .clone()
}

/// The line that ends the turn from `state`, scored as the search scores a passed line.
fn passed(state: &GameState) -> f64 {
    terminal_score(
        state,
        AI,
        state.turn,
        &create_node_counter(10, None),
        Prices::NONE,
    )
}

/// `state` with the AI's turn ended through the reducer.
fn ended(state: &GameState) -> GameState {
    act(state, AI, &ActionBody::EndTurn)
}

/// The AI's turn from `state`: `decide` and the reducer until the turn is no longer its own. The
/// support's `act` mints fresh nonces, so two turns of one game never replay one.
fn ai_turn(state: &GameState, options: &mut AiOptions) -> (GameState, Vec<ActionBody>) {
    let mut current = state.clone();
    let mut actions = Vec::new();
    while current.active == AI && current.result.is_none() && actions.len() < TURN_ACTIONS {
        let decision = decide(&current, AI, options).expect("the AI decides");
        current = act(&current, AI, &decision.action);
        actions.push(decision.action);
    }
    (current, actions)
}

fn plays(actions: &[ActionBody], instance: &str) -> bool {
    actions
        .iter()
        .any(|action| matches!(action, ActionBody::Play { instance_id, .. } if instance_id == instance))
}

#[test]
fn r3695000_a_floating_line_with_a_float_card_out_of_reach_scores_no_lower_than_without_one() {
    // What Floating `float` adds to the passed line over spending every crystal.
    let gain = |hand: &[&str], float: i32| passed(&on_turn(9, hand, float)) - passed(&on_turn(9, hand, 0));
    let crystals = on_turn(9, &[SIX], 0).players[AI].mana.max;
    // Drone Core IX's 6 against the four crystals.
    let distance = 6 - crystals;
    assert_eq!(distance, 2);
    for float in 1..=4 {
        let with = gain(&[DC_IX], float);
        let without = gain(&[SIX], float);
        assert!(
            with >= without,
            "Floating {float} scores {with} with Drone Core IX in hand, {without} without"
        );
        // The card's cheaper price (B3.4) and the credit for the crystals that brought it nearer.
        close(
            with - without,
            AI_EVAL.hand_cost_delta * f64::from(float)
                + AI_EVAL.float_discount * f64::from(float.min(distance)),
        );
    }
    for float in 1..=distance {
        assert!(
            gain(&[DC_IX], float) >= 0.0,
            "a Float that only brings the card nearer costs the line nothing ({float})"
        );
    }
}

#[test]
fn r3695000_with_no_float_card_in_hand_every_score_is_as_before() {
    for float in 0..=4 {
        let s = on_turn(9, &[SIX], float);
        let end = ended(&s);
        close(
            passed(&s),
            line_score(&end, AI, NextSwing::Enemy, Prices::NONE) - AI_EVAL.unspent_mana * f64::from(float),
        );
        assert_eq!(float_credit(&end, AI, float, &AI_EVAL), 0.0);
    }
}

#[test]
fn r3695000_float_credit_counts_the_distance_still_owed_and_nothing_else() {
    let fd = AI_EVAL.float_discount;
    let credit = |state: &GameState, float: i32| float_credit(&ended(state), AI, float, &AI_EVAL);

    // Drone Core IX at 6 against 4 crystals: a Float of 3 brings it 2 nearer, the third crystal is past it.
    let ix = on_turn(9, &[DC_IX], 3);
    close(credit(&ix, 3), fd * 2.0);
    assert_eq!(credit(&ix, 0), 0.0, "no Float, no credit");
    assert_eq!(float_credit(&ended(&ix), AI, 3, &GREEDY_EVAL), 0.0);

    // An earlier discount leaves less to bring nearer, and a card already in reach none.
    for (cost_mod, owed) in [(-1, 1.0), (-2, 0.0), (-5, 0.0)] {
        let mut s = ix.clone();
        s.players[AI].hand[0].cost_mod = cost_mod;
        close(credit(&s, 3), fd * owed);
    }

    // Each card counts its own distance: IX's 2 and X's 2 of the Float's 2.
    close(credit(&on_turn(9, &[DC_IX, DC_X], 2), 2), fd * 4.0);

    // The crystals are the state's `mana.max`, never the handicap's (R180).
    let mut seven = on_turn(11, &[DC_X], 3);
    seven.players[AI].mana.max = 7;
    let end = ended(&seven);
    close(float_credit(&end, AI, 3, &AI_EVAL), fd);
    let mut hard = end.clone();
    hard.players[AI].handicap = Some(AI_DIFFICULTY.hard);
    close(float_credit(&hard, AI, 3, &AI_EVAL), fd);

    assert_eq!(
        GREEDY_EVAL.float_discount, 0.0,
        "the greedy baseline counts no Float"
    );
    const { assert!(AI_EVAL.float_discount >= AI_EVAL.unspent_mana) };
}

#[test]
fn r3695000_a_reply_scored_line_carries_the_credit() {
    jackioh_cards::register_all();
    let s = scenario(json!({
        "seed": "ai-float-reply",
        "active": "p1",
        "turn": 9,
        "p1": { "hand": [DC_IX], "mana": 2, "library": [INERT, INERT] },
        "p2": { "field": ["core-008"], "library": [INERT, INERT] },
    }));
    let end = ended(s.state());
    assert_eq!(line_status(&end, AI, 9), LineStatus::Passed);
    assert_eq!(end.players[AI].turn_log.unspent_at_end, Some(2));
    let credit = float_credit(&end, AI, 2, &AI_EVAL);
    close(credit, AI_EVAL.float_discount * 2.0);

    let hidden = hidden_card_ids(&end, AI);
    let after =
        simulate_reply(&end, AI, &create_node_counter(20, None), &hidden, Prices::NONE).expect("a reply");
    assert_eq!(after.active, AI, "the reply hands the seat its turn");
    let score = reply_score(&end, AI, 9, &create_node_counter(20, None), &hidden, Prices::NONE);
    close(
        score.expect("a score"),
        finalist_score(&after, AI, NextSwing::Seat, Prices::NONE) - AI_EVAL.unspent_mana * 2.0 + credit,
    );
}

#[test]
fn r3695000_an_easy_seat_with_drone_core_ix_and_a_one_drop_ends_with_crystals_left_and_casts_it_next_turn() {
    let s = on_turn(7, &[DC_IX, ONE], 4);
    assert_eq!(s.players[AI].mana.max, 4);
    assert_eq!(
        s.players[AI].handicap, None,
        "Easy's handicap is a human's, none stored"
    );
    let ix = s.players[AI].hand[0].id.clone();
    let mut options = AiOptions::new(create_rng("ai-float-ix", 0));

    let (first, _) = ai_turn(&s, &mut options);
    assert_eq!(first.active, HUMAN);
    let floated = first.players[AI].turn_log.unspent_at_end;
    assert!(
        floated >= Some(2),
        "the AI ends its turn with crystals left: {floated:?}"
    );
    let card = first.players[AI]
        .hand
        .iter()
        .find(|card| card.id == ix)
        .expect("Drone Core IX is still in hand");
    assert!(effective_cost(&first, card, CostOptions::default()) <= 4);

    let theirs = act(&first, HUMAN, &ActionBody::EndTurn);
    assert_eq!(theirs.active, AI);
    let (_, actions) = ai_turn(&theirs, &mut options);
    assert!(plays(&actions, &ix), "the AI casts Drone Core IX: {actions:?}");
}

/// The Float the credit buys: without it (each crystal Floated −0.3) the AI spends its four crystals on
/// a 4-mana 1/1 and Drone Core X stays at 8, out of reach for good; with it, it Floats all four, X costs
/// 4, and the next turn casts it.
#[test]
fn r3695000_an_easy_seat_floats_over_a_weak_four_drop_and_casts_drone_core_x_next_turn() {
    let s = on_turn(7, &[DC_X, "stand-in (4) 1/1"], 4);
    let x = s.players[AI].hand[0].id.clone();
    let mut options = AiOptions::new(create_rng("ai-float-x-next", 0));

    let (first, actions) = ai_turn(&s, &mut options);
    assert_eq!(first.active, HUMAN);
    assert_eq!(
        first.players[AI].turn_log.unspent_at_end,
        Some(4),
        "the AI keeps its crystals: {actions:?}"
    );
    let card = first.players[AI]
        .hand
        .iter()
        .find(|card| card.id == x)
        .expect("Drone Core X is still in hand");
    assert_eq!(effective_cost(&first, card, CostOptions::default()), 4);

    let theirs = act(&first, HUMAN, &ActionBody::EndTurn);
    let (_, actions) = ai_turn(&theirs, &mut options);
    assert!(plays(&actions, &x), "the AI casts Drone Core X: {actions:?}");
}

#[test]
fn r3695000_decide_casts_drone_core_x_in_reach_with_a_free_zone_at_easy_and_hard() {
    // (hard, turn, crystals, costMod): Easy's MAX_MANA and Hard's 7, X brought down to exactly them.
    for (hard, turn, crystals, cost_mod) in [(false, 7, 4, -4), (true, 11, 7, -1)] {
        let mut s = on_turn(turn, &[DC_X], crystals);
        if hard {
            s.players[AI].handicap = Some(AI_DIFFICULTY.hard);
            s.players[AI].mana.max = crystals;
        }
        s.players[AI].hand[0].cost_mod = cost_mod;
        let x = s.players[AI].hand[0].id.clone();
        assert_eq!(
            effective_cost(&s, &s.players[AI].hand[0], CostOptions::default()),
            crystals
        );
        for cursor in 0..3 {
            let decision =
                decide(&s, AI, &mut AiOptions::new(create_rng("ai-float-x", cursor))).expect("a decision");
            assert!(
                plays(std::slice::from_ref(&decision.action), &x),
                "hard {hard}, cursor {cursor}: {:?}",
                decision.action
            );
        }
    }
}

// R3695001: the sweep's second cause. The Drone Battleship enters as one Stack pile (R1842000), so
// before it only its top, a Shield Layer that cannot attack, counted; the two layers beneath it count
// `dormant_share` of their worth.

/// S #36's three layers, bottom first, as `summon_pile` stacks them (§8.10).
const BATTLESHIP: [&str; 3] = [
    "stinkyscape-t-meltdown-layer",
    "stinkyscape-t-propagation-layer",
    "stinkyscape-t-shield-layer",
];
/// Classic #86 Genn, a (4) 14/14 with no text.
const GENN: &str = "classic-086";
/// Meditative #100 Greaser, a (2) 7/7 with no text.
const GREASER: &str = "meditative-100";
/// Core #8 Mr. Vanilla, a (1) 4/4 with no text.
const VANILLA: &str = "core-008";

#[test]
fn r3695001_each_battleship_layer_beneath_its_top_counts_dormant_share_of_its_worth() {
    jackioh_cards::register_all();
    let field = json!([
        BATTLESHIP[0],
        { "def": BATTLESHIP[1], "stack": true },
        { "def": BATTLESHIP[2], "stack": true },
    ]);
    let pile = scenario(json!({ "p1": { "field": field }, "p2": { "field": [VANILLA] } }))
        .state()
        .clone();
    let cards = pile.players[AI].units[0].clone().unwrap_or_default();
    assert_eq!(
        cards.len(),
        3,
        "setup: the Battleship stands as one pile in lane 1"
    );
    let mut top = pile.clone();
    if let Some(cards) = top.players[AI].units[0].as_mut() {
        cards.truncate(1);
    }
    let beneath: f64 = cards[1..]
        .iter()
        .map(|card| unit_worth(&pile, card, &AI_EVAL, Prices::NONE))
        .sum();
    close(
        evaluate(&pile, AI, NextSwing::Enemy, &AI_EVAL) - evaluate(&top, AI, NextSwing::Enemy, &AI_EVAL),
        AI_EVAL.dormant_share * beneath,
    );
    // The layers beneath count.
    const { assert!(AI_EVAL.dormant_share > 0.0) };
    assert_eq!(
        GREEDY_EVAL.dormant_share, 0.0,
        "the greedy baseline keeps them at 0"
    );
}

/// Hard (7 crystals) with Drone Core X brought down to them, beside `others`, against `enemy`.
fn hard_on_seven(others: &[&str], enemy: Value) -> GameState {
    jackioh_cards::register_all();
    let mut hand = vec![DC_X];
    hand.extend_from_slice(others);
    let mut s = scenario(json!({
        "seed": "ai-float-discount",
        "active": "p1",
        "turn": 11,
        "p1": { "hand": hand, "mana": 7, "library": [INERT, INERT, INERT, INERT] },
        "p2": { "hand": [ONE], "field": enemy, "library": [INERT, INERT, INERT, INERT] },
    }))
    .state()
    .clone();
    s.players[AI].handicap = Some(AI_DIFFICULTY.hard);
    s.players[AI].mana.max = 7;
    s.players[AI].hand[0].cost_mod = -1;
    s
}

/// The sweep's 3 affordable turns at Hard and no cast: on 7 crystals the AI took a vanilla body over
/// Drone Core X while the layers beneath the Shield Layer counted nothing. X costs all 7, so it is cast
/// first or not that turn.
#[test]
fn r3695001_hard_on_seven_crystals_casts_drone_core_x_over_genn_or_greaser() {
    for (others, enemy) in [
        (&[GENN][..], json!([])),
        (&[GENN][..], json!([VANILLA])),
        (&[GREASER][..], json!([])),
    ] {
        let s = hard_on_seven(others, enemy.clone());
        let x = s.players[AI].hand[0].id.clone();
        assert_eq!(
            effective_cost(&s, &s.players[AI].hand[0], CostOptions::default()),
            7
        );
        for cursor in 0..3 {
            let decision =
                decide(&s, AI, &mut AiOptions::new(create_rng("ai-float-x", cursor))).expect("a decision");
            assert!(
                plays(std::slice::from_ref(&decision.action), &x),
                "{others:?} against {enemy}, cursor {cursor}: {:?}",
                decision.action
            );
        }
    }
}
