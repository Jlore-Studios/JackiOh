//! Dormant in the AI's reads (R1839000): the candidates come from `legal_actions`, so a Dormant card's
//! attack, switch and abilities, and an attack on one, are never among them, and neither `decide` nor the
//! random policy ever acts through it; it is no threat until it wakes (`active_units_of` leaves it out
//! of `face_threat`); the evaluation prices it by the unit it wakes as, less for each turn it waits
//! (R2380000), plus what its "When this awakens" gives, against its hookless twin's wake (R3064000); and
//! the search signature tells a position holding one apart from one without it.
//!
//! Most boards put a unit on the field to sleep by writing its status, as `slumber::start_slumber_on_field`
//! would. The cards: Mr. Vanilla 4/4, Tempo Timmy 3/3 and Imprisoned Cosmality, (2) 10/6 Pierce, Dormant for
//! 2 turns.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, close, register_cards};

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

// R3064000: what a Dormant card's "When this awakens" gives its controller. The cards: Imprisoned
// Cosmality, whose wake deals 10 damage randomly split among all enemies; Imprisoned Ship Smith, (1) 2/2,
// Dormant for 2 turns, whose wake summons a Fiery Waraxe; the Felinor Token, a (1) 1/1 with no text; and
// Mr. Vanilla and Tempo Timmy, whose sleep has no wake trigger.

const SHIP_SMITH: &str = "stinkyscape-017";
const FELINOR_TOKEN: &str = "core-t-felinor";
const VANILLA: &str = "core-008";

/// The AI's awakening prices on `state` and the nodes their probes spent.
fn priced(state: &GameState) -> (Vec<AwakenPrice>, usize) {
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = awaken_prices(state, AI, &counter);
    (prices, counter.used())
}

/// `awakenings` as a decision passes them, with no other price.
fn prices_of(awakenings: Vec<AwakenPrice>) -> Prices {
    Prices {
        awakenings,
        ..Prices::default()
    }
}

/// The price of `controller`'s `def_id`, which must be priced.
fn price_of(prices: &[AwakenPrice], def_id: &str, controller: PlayerId) -> f64 {
    prices
        .iter()
        .find(|entry| entry.def_id == def_id && entry.controller == controller)
        .map(|entry| entry.price)
        .unwrap_or_else(|| panic!("{controller}'s {def_id}'s awakening is priced: {prices:?}"))
}

fn eval_with(state: &GameState, prices: &Prices) -> f64 {
    evaluate_priced(state, AI, NextSwing::Enemy, &AI_EVAL, prices)
}

/// The AI's turn with `def` on `player`'s field, two turns from waking, and Mr. Vanilla on the other
/// side's, each side holding Tempo Timmy; and the sleeper's id.
fn sleeper_board(def: &str, player: PlayerId) -> (GameState, String) {
    register_cards();
    let side = |field: &str| json!({ "hand": ["core-011"], "field": [field], "library": LIBRARY });
    let (p1, p2) = if player == AI {
        (side(def), side(VANILLA))
    } else {
        (side(VANILLA), side(def))
    };
    let mut state = scenario(json!({ "seed": "ai-awaken-board", "p1": p1, "p2": p2 }))
        .state()
        .clone();
    // `active_units_of` leaves a Dormant card out, so the sleeper is read off its row.
    let id = state.players[player]
        .units
        .iter()
        .flatten()
        .next()
        .expect("a pile")[0]
        .id
        .clone();
    find_instance_mut(&mut state, &id).expect("the sleeper").slumber = Some(Slumber { turns: Some(2) });
    (state, id)
}

/// `state` with the sleeper `id` as its hookless twin: Vanilla, so its wake trigger is off, and granted
/// the keywords it prints, so its body reads the same.
fn hookless(state: &GameState, id: &str) -> GameState {
    let printed = face_of(state, find_instance(state, id).expect("the sleeper")).keywords;
    let mut twin = state.clone();
    let card = find_instance_mut(&mut twin, id).expect("the sleeper");
    card.granted_keywords.extend(printed);
    card.vanilla = true;
    twin
}

/// The AI's turn 3, its second, with `mana` crystals, `hand` in its hand and both boards empty.
fn hand_on_turn_3(tier: Handicap, hand: &[&str], mana: i32, human_hand: &[&str]) -> GameState {
    register_cards();
    let mut state = scenario(json!({
        "seed": "ai-awaken-hand-play",
        "turn": 3,
        "p1": { "hand": hand, "mana": mana, "library": LIBRARY },
        "p2": { "hand": human_hand, "library": LIBRARY },
    }))
    .state()
    .clone();
    state.players[AI].handicap = Some(tier);
    state
}

/// `state` after the AI plays its first hand card of `def_id`.
fn after_playing(state: &GameState, def_id: &str) -> GameState {
    let play = candidate_actions(state, AI, Prices::NONE)
        .into_iter()
        .find(|action| plays_card(action, state, def_id))
        .expect("a play of the card");
    simulate(
        state,
        AI,
        &play,
        &create_node_counter(AI_BUDGET.nodes, None),
        Prices::NONE,
    )
    .expect("a node")
    .expect("a state")
}

/// Whether `action` plays an AI hand card of `def_id`.
fn plays_card(action: &ActionBody, state: &GameState, def_id: &str) -> bool {
    matches!(action, ActionBody::Play { instance_id, .. }
        if state.players[AI].hand.iter().any(|card| card.id == *instance_id && card.def_id == def_id))
}

#[test]
fn r3064000_an_own_sleeper_scores_above_the_same_body_without_its_wake_trigger_and_an_enemy_one_below() {
    let share = AI_EVAL.slumber_turn_share.powi(2);
    for def in [COSMALITY, SHIP_SMITH] {
        for (player, sign) in [(AI, 1.0), (HUMAN, -1.0)] {
            let (state, id) = sleeper_board(def, player);
            let (awakenings, used) = priced(&state);
            assert!(
                (2..=AI_AWAKEN_PROBE_NODES).contains(&used),
                "{def} {player:?}: {used} nodes"
            );
            let price = price_of(&awakenings, def, player);
            assert!(price > 0.0, "{def} {player:?}: {price}");
            let prices = prices_of(awakenings);
            close(
                eval_with(&state, &prices) - eval_with(&state, Prices::NONE),
                sign * share * AI_EVAL.awaken * price,
            );
            // The twin without the trigger reads the same body, priced or not.
            let twin = hookless(&state, &id);
            close(eval_with(&twin, Prices::NONE), eval_with(&state, Prices::NONE));
            close(eval_with(&twin, &prices), eval_with(&state, Prices::NONE));
            if player == AI {
                assert!(eval_with(&state, &prices) > eval_with(&twin, &prices), "{def}");
            } else {
                assert!(eval_with(&state, &prices) < eval_with(&twin, &prices), "{def}");
            }
        }
    }
}

#[test]
fn r3064000_a_sleeper_with_no_wake_trigger_is_priced_nothing_and_a_board_without_one_spends_no_node() {
    let mut timmy = board();
    sleep(&mut timmy, HUMAN);
    assert_eq!(priced(&timmy), (Vec::new(), 0));
    assert_eq!(priced(&board()), (Vec::new(), 0));
}

#[test]
fn r3064000_a_hand_unit_that_enters_dormant_is_priced_placed_first_and_an_enemy_hand_card_never_is() {
    register_cards();
    let state = scenario(json!({
        "seed": "ai-awaken-hand",
        "p1": { "hand": [COSMALITY, SHIP_SMITH, VANILLA], "library": LIBRARY },
        "p2": { "hand": [COSMALITY, SHIP_SMITH], "field": [VANILLA], "library": LIBRARY },
    }))
    .state()
    .clone();
    let (awakenings, used) = priced(&state);
    let keys: Vec<(&str, PlayerId)> = awakenings
        .iter()
        .map(|entry| (entry.def_id.as_str(), entry.controller))
        .collect();
    assert_eq!(keys, [(COSMALITY, AI), (SHIP_SMITH, AI)]);
    assert!(awakenings.iter().all(|entry| entry.price > 0.0), "{awakenings:?}");
    assert!((4..=AI_AWAKEN_PROBE_NODES).contains(&used), "{used} nodes");
}

#[test]
fn r3064000_decide_plays_ship_smith_alone_at_one_crystal_at_medium_and_hard() {
    for tier in [AI_DIFFICULTY.medium, AI_DIFFICULTY.hard] {
        let state = hand_on_turn_3(tier, &[SHIP_SMITH], 1, &["core-011"]);
        // Its sleeping body alone is worth less than the card in hand; its Waraxe tips it.
        let played = after_playing(&state, SHIP_SMITH);
        assert!(eval_with(&played, Prices::NONE) < eval_with(&state, Prices::NONE));
        let prices = prices_of(priced(&state).0);
        assert!(eval_with(&played, &prices) > eval_with(&state, &prices));
        for cursor in 0..2 {
            let decision = decide(
                &state,
                AI,
                &mut AiOptions::new(create_rng("ai-awaken-smith", cursor)),
            )
            .expect("a decision");
            assert!(
                plays_card(&decision.action, &state, SHIP_SMITH),
                "{tier:?} cursor {cursor}: {:?}",
                decision.action
            );
        }
    }
}

#[test]
fn r3064000_decide_plays_imprisoned_cosmality_over_a_felinor_token_at_medium_and_hard() {
    for tier in [AI_DIFFICULTY.medium, AI_DIFFICULTY.hard] {
        let state = hand_on_turn_3(tier, &[COSMALITY, FELINOR_TOKEN], 2, &["core-011"]);
        for cursor in 0..2 {
            let decision = decide(
                &state,
                AI,
                &mut AiOptions::new(create_rng("ai-awaken-cosmality", cursor)),
            )
            .expect("a decision");
            assert!(
                plays_card(&decision.action, &state, COSMALITY),
                "{tier:?} cursor {cursor}: {:?}",
                decision.action
            );
        }
    }
}

#[test]
fn r3064000_positions_that_differ_only_in_a_hidden_card_give_the_same_decision() {
    let decide_on = |state: &GameState| {
        decide(state, AI, &mut AiOptions::new(create_rng("ai-awaken-hidden", 0))).expect("a decision")
    };
    // The human's sleeping Ship Smith, and the AI holding Cosmality with the crystals for it; the
    // human's one hidden hand card told apart. The hands keep their size, which is public.
    let (mut first, _) = sleeper_board(SHIP_SMITH, HUMAN);
    first.players[AI].hand[0].def_id = COSMALITY.to_string();
    first.players[AI].mana.current = 2;
    first.players[AI].handicap = Some(AI_DIFFICULTY.hard);
    let mut second = first.clone();
    second.players[HUMAN].hand[0].def_id = VANILLA.to_string();
    let hand = (
        hand_on_turn_3(AI_DIFFICULTY.hard, &[COSMALITY, SHIP_SMITH], 2, &["core-011"]),
        hand_on_turn_3(AI_DIFFICULTY.hard, &[COSMALITY, SHIP_SMITH], 2, &[VANILLA]),
    );
    for (one, other) in [(first, second), hand] {
        assert_ne!(hash_state(&one), hash_state(&other));
        assert_eq!(decide_on(&one), decide_on(&other));
    }
}

#[test]
fn r3064000_a_zero_weight_and_greedy_eval_read_no_price() {
    const ZERO: EvalWeights = EvalWeights {
        awaken: 0.0,
        ..AI_EVAL
    };
    assert_eq!(GREEDY_EVAL.awaken, 0.0);
    let (state, _) = sleeper_board(COSMALITY, AI);
    let (awakenings, _) = priced(&state);
    assert!(!awakenings.is_empty());
    let prices = prices_of(awakenings);
    for w in [&ZERO, &GREEDY_EVAL] {
        assert_eq!(
            evaluate_priced(&state, AI, NextSwing::Enemy, w, &prices),
            evaluate_priced(&state, AI, NextSwing::Enemy, w, Prices::NONE)
        );
    }
}
