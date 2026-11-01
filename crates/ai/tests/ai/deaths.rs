//! A unit's Death in the AI's evaluation (R2424000): at a decision's root the AI prices each Death it
//! can read, on either field and among its own hand's Units (placed first), as its controller's
//! evaluation after the sandbox's destroy less after its remove (R923003); `unit_worth` adds
//! `AI_EVAL.death` of the price and `kill_worth` takes the whole of it off a kill, so a unit that comes
//! back bigger is worth more alive and less to kill, and one whose Death hurts its owner is worth less.
//! The frozen baseline reads no price, and a call not passed a decision's prices reads none.
//!
//! The cards: Trigor (traininggrounds-025), (3) 3/3 Tribute 3, "Death: Summon a Trigor with this one's
//! base stats +3/+3"; Floodgates (traininggrounds-030), (3) 0/12 Armor 3, "Death: Destroy all your
//! Units"; Training Dummy (traininggrounds-036), (1) 0/3, "Death: Add 1 random Radiant Training Grounds
//! card to your hand"; and Mr. Vanilla (core-008), a (1) 4/4 with no text.
//!
//! R2566000: a Starship Piece's death fuses a copy of it into its owner's Dormant Starship, summoning
//! one first (R1852000), which counts as the unit it wakes as at `slumber_waiting_share` (R2380000), so
//! a piece with no Death of its own is priced too, after every Death. The cards: After Burner
//! (stinkyscape-090), (2) 4/8 Starship Piece, whose one hook is a "When launched" that only a Starship
//! fires; and Hit Job (core-016), (3) "Destroy a Unit".
//!
//! R2687000: a backrow card's Death is priced the same way, after the units: each backrow card the AI
//! can read on either side (never a face-down enemy card, R185) and each Field Spell in its own hand
//! (set first), within the same budget; `material` adds `AI_EVAL.death` of the price outside the
//! Brittle share, a crumble being a destroy. The cards: Bauble Bubble (classicplus-061), (1) Field
//! Spell, "Brittle 1. Death: Add 2 Stockpiles to your hand. Each costs (0)"; Comet Crash
//! (stinkyscape-155), (2) Field Spell, "Crater, Brittle 1. Death: Gain 3 random Stinkyscape
//! Resources"; Job Application (stinkyscape-112), (1) Field Spell, a Quest whose Death resummons it at
//! Stage 1; The Narcor Project (stinkyscape-039), (3) Field Spell, whose Death pays only on a Brittle
//! destroy (R1861001); and Sheepish (core-041) and My Pawn (core-096), two (1) Traps.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards};

const TRIGOR: &str = "traininggrounds-025";
const FLOODGATES: &str = "traininggrounds-030";
const TRAINING_DUMMY: &str = "traininggrounds-036";
const VANILLA: &str = "core-008";
const AFTER_BURNER: &str = "stinkyscape-090";
const HIT_JOB: &str = "core-016";
const BAUBLE_BUBBLE: &str = "classicplus-061";
const COMET_CRASH: &str = "stinkyscape-155";
const JOB_APPLICATION: &str = "stinkyscape-112";
const NARCOR_PROJECT: &str = "stinkyscape-039";
const SHEEPISH: &str = "core-041";
const MY_PAWN: &str = "core-096";
/// The Death Permanent Underclassman (meditative-058) grants: "Add a random Book to your hand".
const BOOK_DEATH: &str = "meditative-058#bookDeath";
const LIBRARY: &[&str] = &[VANILLA, VANILLA, VANILLA, VANILLA];

/// The AI's turn with `hand` and `field`, the human's `enemy` and a Mr. Vanilla in the human's hand.
fn board(seed: &str, hand: Value, field: Value, enemy: Value) -> GameState {
    register_cards();
    let library = json!([VANILLA, VANILLA, VANILLA, VANILLA]);
    scenario(json!({
        "seed": seed,
        "p1": { "hand": hand, "field": field, "library": library },
        "p2": { "hand": [VANILLA], "field": enemy, "library": library },
    }))
    .state()
    .clone()
}

/// The AI's Death prices on `state` and the nodes their probes spent.
fn priced(state: &GameState) -> (Vec<DeathPrice>, usize) {
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = death_prices(state, AI, &counter);
    (prices, counter.used())
}

/// `deaths` as a decision passes them, with no turn-hook price.
fn prices_of(deaths: Vec<DeathPrice>) -> Prices {
    Prices {
        deaths,
        ..Prices::default()
    }
}

/// `player`'s unit of `def_id` on the field.
fn unit<'a>(state: &'a GameState, player: PlayerId, def_id: &str) -> &'a CardInstance {
    active_units_of(state, player)
        .into_iter()
        .find(|unit| unit.def_id == def_id)
        .expect("the unit is on the field")
}

/// The price of `def_id`'s Death, which must be priced.
fn price_of(prices: &[DeathPrice], def_id: &str) -> f64 {
    prices
        .iter()
        .find(|entry| entry.def_id == def_id)
        .map(|entry| entry.price)
        .unwrap_or_else(|| panic!("{def_id}'s Death is priced: {prices:?}"))
}

/// A 3/3 with no text in Attack Position, as `unit_worth` counts it.
fn vanilla_three_three() -> f64 {
    3.0 * AI_EVAL.attack + 3.0 * AI_EVAL.health
}

#[test]
fn r2424000_a_trigor_on_the_ai_board_is_worth_more_than_a_vanilla_3_3() {
    let state = board("ai-death-trigor", json!([]), json!([TRIGOR]), json!([]));
    let trigor = unit(&state, AI, TRIGOR);
    let body = unit_worth(&state, trigor, &AI_EVAL, Prices::NONE);
    assert!(
        (body - vanilla_three_three()).abs() < 1e-9,
        "unpriced, a Trigor is its 3/3 body: {body}"
    );
    let (prices, _) = priced(&state);
    let price = price_of(&prices, TRIGOR);
    assert!(
        price > vanilla_three_three(),
        "its Death leaves a 6/6, more than a 3/3: {price}"
    );
    let worth = unit_worth(&state, trigor, &AI_EVAL, &prices_of(prices));
    assert!((worth - (body + AI_EVAL.death * price)).abs() < 1e-9, "{worth}");
    assert!(worth > vanilla_three_three(), "{worth}");
}

#[test]
fn r2424000_killing_an_enemy_trigor_is_worth_less_than_killing_a_vanilla_3_3() {
    let state = board("ai-death-kill", json!([]), json!([VANILLA]), json!([TRIGOR]));
    let trigor = unit(&state, HUMAN, TRIGOR);
    let unpriced_kill = kill_worth(&state, trigor, &AI_EVAL, Prices::NONE);
    assert!(
        (unpriced_kill - vanilla_three_three()).abs() < 1e-9,
        "unpriced, killing a Trigor takes its 3/3 body: {unpriced_kill}"
    );
    let unpriced_eval = evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL);
    let (prices, _) = priced(&state);
    assert!(price_of(&prices, TRIGOR) > 0.0);
    let prices = prices_of(prices);
    let kill = kill_worth(&state, trigor, &AI_EVAL, &prices);
    let eval = evaluate_priced(&state, AI, NextSwing::Enemy, &AI_EVAL, &prices);
    assert!(
        kill < vanilla_three_three(),
        "the 6/6 it leaves its owner comes off the kill: {kill}"
    );
    assert!(
        eval < unpriced_eval,
        "an enemy Trigor counts for more against the AI: {eval} against {unpriced_eval}"
    );
}

#[test]
fn r2424000_a_death_that_harms_its_owner_prices_below_a_vanilla_body() {
    let state = board(
        "ai-death-floodgates",
        json!([]),
        json!([FLOODGATES, VANILLA, VANILLA]),
        json!([]),
    );
    let floodgates = unit(&state, AI, FLOODGATES);
    let body = unit_worth(&state, floodgates, &AI_EVAL, Prices::NONE);
    let (prices, _) = priced(&state);
    let price = price_of(&prices, FLOODGATES);
    assert!(price < 0.0, "its Death destroys its owner's two 4/4s: {price}");
    let worth = unit_worth(&state, floodgates, &AI_EVAL, &prices_of(prices));
    assert!(worth < body, "{worth} against its body's {body}");
}

#[test]
fn r2424000_a_training_dummy_is_priced_by_the_card_its_death_adds() {
    let state = board("ai-death-dummy", json!([]), json!([TRAINING_DUMMY]), json!([]));
    let (prices, _) = priced(&state);
    let price = price_of(&prices, TRAINING_DUMMY);
    assert!(price > 0.0, "a card in hand is worth something: {price}");
}

#[test]
fn r2424000_a_unit_in_the_ai_hand_is_priced_by_placing_it_first() {
    let state = board("ai-death-hand", json!([TRIGOR]), json!([VANILLA]), json!([]));
    let (prices, used) = priced(&state);
    assert!(price_of(&prices, TRIGOR) > vanilla_three_three(), "{prices:?}");
    assert!(
        (3..=AI_DEATH_PROBE_NODES).contains(&used),
        "a spawn, a destroy and a remove at least: {used}"
    );
}

#[test]
fn r2424000_a_full_unit_row_still_prices_a_hand_unit() {
    let state = board(
        "ai-death-full",
        json!([TRIGOR]),
        json!([VANILLA, VANILLA, VANILLA, VANILLA, VANILLA]),
        json!([]),
    );
    assert_eq!(active_units_of(&state, AI).len(), 5);
    let (prices, _) = priced(&state);
    assert!(price_of(&prices, TRIGOR) > vanilla_three_three(), "{prices:?}");
}

#[test]
fn r2424000_a_board_without_a_death_prices_nothing_and_spends_no_node() {
    let state = board(
        "ai-death-none",
        json!([VANILLA]),
        json!([VANILLA]),
        json!([VANILLA]),
    );
    let (prices, used) = priced(&state);
    assert!(prices.is_empty(), "{prices:?}");
    assert_eq!(used, 0);
}

#[test]
fn r2424000_the_frozen_baseline_and_a_finished_decision_read_no_price() {
    assert_eq!(GREEDY_EVAL.death, 0.0);
    let state = board(
        "ai-death-decide",
        json!([VANILLA]),
        json!([TRIGOR]),
        json!([VANILLA]),
    );
    let trigor = unit(&state, AI, TRIGOR);
    let (prices, _) = priced(&state);
    let greedy = unit_worth(&state, trigor, &GREEDY_EVAL, Prices::NONE);
    let held = unit_worth(&state, trigor, &GREEDY_EVAL, &prices_of(prices));
    assert_eq!(held, greedy, "the frozen baseline reads no price");

    let before = unit_worth(&state, trigor, &AI_EVAL, Prices::NONE);
    assert!(
        (before - vanilla_three_three()).abs() < 1e-9,
        "unpriced, a Trigor is its 3/3 body: {before}"
    );
    decide(&state, AI, &mut AiOptions::new(create_rng("ai-death-decide", 0))).expect("the AI acts");
    assert_eq!(
        unit_worth(&state, trigor, &AI_EVAL, Prices::NONE),
        before,
        "a decision leaves no price behind"
    );
}

/// A Mr. Vanilla at After Burner's 4/8, by a stat override (§10.4 layer 1).
fn vanilla_four_eight() -> Value {
    json!({ "def": VANILLA, "statsOverride": { "attack": 4, "health": 8 } })
}

#[test]
fn r2566000_a_starship_piece_with_no_death_is_priced_by_the_copy_it_fuses() {
    let state = board("ai-piece-own", json!([]), json!([AFTER_BURNER]), json!([]));
    let piece = unit(&state, AI, AFTER_BURNER);
    assert!(
        death_hook_of(&state, piece).is_none(),
        "After Burner has no Death"
    );
    assert!(is_starship_piece(&state, piece));
    let body = unit_worth(&state, piece, &AI_EVAL, Prices::NONE);
    let (prices, used) = priced(&state);
    let price = price_of(&prices, AFTER_BURNER);
    assert!(
        (price - AI_EVAL.slumber_waiting_share * body).abs() < 1e-9,
        "its death leaves a Dormant Starship holding its copy, at the slumber share: {price} against {body}"
    );
    assert!(price > 0.0, "{price}");
    assert!(
        (2..=AI_DEATH_PROBE_NODES).contains(&used),
        "a destroy and a remove at least: {used}"
    );
    let worth = unit_worth(&state, piece, &AI_EVAL, &prices_of(prices));
    assert!((worth - (body + AI_EVAL.death * price)).abs() < 1e-9, "{worth}");
}

#[test]
fn r2566000_killing_an_enemy_piece_is_worth_less_than_killing_a_vanilla_of_its_stats() {
    let state = board(
        "ai-piece-kill",
        json!([]),
        json!([]),
        json!([AFTER_BURNER, vanilla_four_eight()]),
    );
    let piece = unit(&state, HUMAN, AFTER_BURNER);
    let vanilla = unit(&state, HUMAN, VANILLA);
    assert_eq!(
        unit_worth(&state, piece, &AI_EVAL, Prices::NONE),
        unit_worth(&state, vanilla, &AI_EVAL, Prices::NONE),
        "unpriced, the two 4/8s are worth the same"
    );
    let (prices, _) = priced(&state);
    assert!(price_of(&prices, AFTER_BURNER) > 0.0, "{prices:?}");
    let prices = prices_of(prices);
    let (piece_kill, vanilla_kill) = (
        kill_worth(&state, piece, &AI_EVAL, &prices),
        kill_worth(&state, vanilla, &AI_EVAL, &prices),
    );
    assert!(
        piece_kill < vanilla_kill,
        "the copy it fuses into its owner's Starship comes off the kill: {piece_kill} against {vanilla_kill}"
    );
}

#[test]
fn r2566000_given_one_removal_the_ai_destroys_the_vanilla_and_not_the_piece() {
    register_cards();
    let library = json!([VANILLA, VANILLA, VANILLA, VANILLA]);
    for (at, enemy) in [
        ("piece-first", json!([AFTER_BURNER, vanilla_four_eight()])),
        ("vanilla-first", json!([vanilla_four_eight(), AFTER_BURNER])),
    ] {
        let seed = format!("ai-piece-removal-{at}");
        let state = scenario(json!({
            "seed": seed,
            "p1": { "hand": [HIT_JOB], "field": [], "library": library, "mana": 3 },
            "p2": { "hand": [VANILLA], "field": enemy, "library": library },
        }))
        .state()
        .clone();
        let vanilla = unit(&state, HUMAN, VANILLA).id.clone();
        let decision = decide(&state, AI, &mut AiOptions::new(create_rng(&seed, 0))).expect("the AI acts");
        assert!(
            matches!(
                &decision.action,
                ActionBody::Play { targets: Some(targets), .. }
                    if targets.first().cloned() == Some(Selection::Instance { instance_id: vanilla.clone() })
            ),
            "{at}: Hit Job takes the Vanilla, not the piece: {:?}",
            decision.action
        );
    }
}

#[test]
fn r2566000_every_death_is_probed_before_a_starship_piece() {
    let state = board(
        "ai-piece-order",
        json!([]),
        json!([AFTER_BURNER]),
        json!([TRIGOR]),
    );
    let (prices, _) = priced(&state);
    let order: Vec<&str> = prices.iter().map(|entry| entry.def_id.as_str()).collect();
    assert_eq!(
        order,
        [TRIGOR, AFTER_BURNER],
        "the enemy's Trigor first, though the AI's piece is listed ahead of it"
    );

    let (alone, nodes) = priced(&board("ai-piece-order", json!([]), json!([]), json!([TRIGOR])));
    assert_eq!(alone.len(), 1, "{alone:?}");
    let short = death_prices(&state, AI, &create_node_counter(nodes, None));
    let order: Vec<&str> = short.iter().map(|entry| entry.def_id.as_str()).collect();
    assert_eq!(
        order,
        [TRIGOR],
        "with only the Trigor's nodes, the Trigor is priced and the piece is not"
    );
    assert!(
        (price_of(&short, TRIGOR) - price_of(&prices, TRIGOR)).abs() < 1e-9,
        "the Trigor's price is the one it reads beside the piece"
    );
}

/// The AI's turn with `p1` and `p2` as their sides' setups (hand, field, backrow, mana), a stocked
/// library under each.
fn sides(seed: &str, p1: Value, p2: Value) -> GameState {
    register_cards();
    let side = |setup: Value| {
        let mut merged = serde_json::Map::new();
        merged.insert("library".to_string(), json!(LIBRARY));
        merged.extend(setup.as_object().expect("a side's setup").clone());
        Value::Object(merged)
    };
    scenario(json!({ "seed": seed, "p1": side(p1), "p2": side(p2) }))
        .state()
        .clone()
}

/// `player`'s backrow card of `def_id`.
fn backrow_card<'a>(state: &'a GameState, player: PlayerId, def_id: &str) -> &'a CardInstance {
    state.players[player]
        .backrow
        .iter()
        .flatten()
        .find(|card| card.def_id == def_id)
        .expect("the card is in the backrow")
}

/// `state` with `player`'s backrow card of `def_id` made Vanilla (§6.3): the same card with no Death.
fn vanilla_copy(state: &GameState, player: PlayerId, def_id: &str) -> GameState {
    let mut copy = state.clone();
    let card = copy.players[player]
        .backrow
        .iter_mut()
        .flatten()
        .find(|card| card.def_id == def_id)
        .expect("the card is in the backrow");
    card.vanilla = true;
    copy
}

/// `state` with every backrow card of `player`'s granted Permanent Underclassman's Book Death.
fn grant_book_death(state: &mut GameState, player: PlayerId) {
    for card in state.players[player].backrow.iter_mut().flatten() {
        card.grants = Some(vec![Grant {
            grant: BOOK_DEATH.to_string(),
            radiant: false,
            params: Default::default(),
        }]);
    }
}

/// The AI's evaluation of `state` passed `deaths`.
fn eval_with(state: &GameState, deaths: Vec<DeathPrice>) -> f64 {
    evaluate_priced(state, AI, NextSwing::Enemy, &AI_EVAL, &prices_of(deaths))
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} vs {b}");
}

#[test]
fn r2687000_a_readable_field_spell_death_counts_for_its_controller_on_either_backrow() {
    for (player, sign) in [(AI, 1.0), (HUMAN, -1.0)] {
        let setup = json!({ "backrow": [JOB_APPLICATION] });
        let state = if player == AI {
            sides("ai-death-backrow", setup, json!({}))
        } else {
            sides("ai-death-backrow", json!({}), setup)
        };
        let (prices, used) = priced(&state);
        let price = price_of(&prices, JOB_APPLICATION);
        assert!(
            price > 0.0,
            "{player:?}: its Death puts it back at Stage 1: {price}"
        );
        assert_eq!(used, 2, "{player:?}: a destroy and a remove");
        let vanilla = vanilla_copy(&state, player, JOB_APPLICATION);
        close(
            eval_with(&state, prices.clone()) - eval_with(&vanilla, prices.clone()),
            sign * AI_EVAL.death * price,
        );
        assert_eq!(
            eval_with(&vanilla, prices),
            evaluate(&vanilla, AI, NextSwing::Enemy, &AI_EVAL),
            "{player:?}: a Vanilla card has no Death to price"
        );
    }
}

#[test]
fn r2687000_a_brittle_field_spell_keeps_its_whole_death_price() {
    let state = sides(
        "ai-death-brittle",
        json!({ "backrow": [BAUBLE_BUBBLE, COMET_CRASH] }),
        json!({}),
    );
    for def_id in [BAUBLE_BUBBLE, COMET_CRASH] {
        assert_eq!(
            active_brittle_count(backrow_card(&state, AI, def_id)),
            Some(1),
            "{def_id}"
        );
    }
    let (prices, _) = priced(&state);
    let (bauble, comet) = (price_of(&prices, BAUBLE_BUBBLE), price_of(&prices, COMET_CRASH));
    assert!(bauble > 0.0 && comet > 0.0, "{prices:?}");
    close(
        eval_with(&state, prices) - evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL),
        AI_EVAL.death * (bauble + comet),
    );
}

#[test]
fn r2687000_a_hand_field_spell_is_priced_by_setting_it_in_the_backrow() {
    let held = sides(
        "ai-death-hand-spell",
        json!({ "hand": [BAUBLE_BUBBLE] }),
        json!({}),
    );
    let (prices, used) = priced(&held);
    let price = price_of(&prices, BAUBLE_BUBBLE);
    assert!(price > 0.0, "{prices:?}");
    assert_eq!(used, 3, "a spawn, a destroy and a remove");
    let set = sides(
        "ai-death-hand-spell",
        json!({ "backrow": [BAUBLE_BUBBLE] }),
        json!({}),
    );
    let (on_backrow, _) = priced(&set);
    close(price, price_of(&on_backrow, BAUBLE_BUBBLE));
}

#[test]
fn r2687000_destroying_an_enemy_field_spell_with_a_death_is_worth_more() {
    let state = sides(
        "ai-death-destroy",
        json!({}),
        json!({ "backrow": [BAUBLE_BUBBLE] }),
    );
    let (prices, _) = priced(&state);
    let price = price_of(&prices, BAUBLE_BUBBLE);
    let mut sandbox = state.clone();
    sandbox.sandbox = Some(true);
    for player in PLAYER_IDS {
        sandbox.players[player].auto_end_turn = Some(false);
    }
    let destroy = ActionBody::Sandbox {
        op: SandboxOp::Destroy {
            instance_id: backrow_card(&state, HUMAN, BAUBLE_BUBBLE).id.clone(),
        },
    };
    let after = simulate(
        &sandbox,
        AI,
        &destroy,
        &create_node_counter(AI_BUDGET.nodes, None),
        Prices::NONE,
    )
    .expect("a node")
    .expect("the destroy applies");
    let prices = prices_of(prices);
    let eval =
        |state: &GameState, prices: &Prices| evaluate_priced(state, AI, NextSwing::Enemy, &AI_EVAL, prices);
    let priced_change = eval(&after, &prices) - eval(&sandbox, &prices);
    let unpriced_change = eval(&after, Prices::NONE) - eval(&sandbox, Prices::NONE);
    close(priced_change - unpriced_change, AI_EVAL.death * price);
    assert!(
        priced_change > unpriced_change,
        "the Stockpiles it leaves its owner come off the destroy: {priced_change} against {unpriced_change}"
    );
}

#[test]
fn r2687000_an_enemy_face_down_card_is_never_probed() {
    let mut sheepish = sides("ai-death-face-down", json!({}), json!({ "backrow": [SHEEPISH] }));
    grant_book_death(&mut sheepish, HUMAN);
    let mut pawn = sheepish.clone();
    for card in pawn.players[HUMAN].backrow.iter_mut().flatten() {
        card.def_id = MY_PAWN.to_string();
    }
    assert_eq!(
        priced(&sheepish),
        (vec![], 0),
        "an enemy face-down card is never probed"
    );
    assert_eq!(priced(&pawn), (vec![], 0));
    assert_eq!(
        evaluate(&sheepish, AI, NextSwing::Enemy, &AI_EVAL),
        evaluate(&pawn, AI, NextSwing::Enemy, &AI_EVAL),
        "which trap sits face-down is no fact the AI has"
    );
    let held = vec![DeathPrice {
        def_id: SHEEPISH.to_string(),
        radiant: false,
        grants: backrow_card(&sheepish, HUMAN, SHEEPISH)
            .grants
            .clone()
            .unwrap_or_default(),
        stats_override: None,
        price: 10.0,
    }];
    assert_eq!(
        eval_with(&sheepish, held),
        evaluate(&sheepish, AI, NextSwing::Enemy, &AI_EVAL),
        "nor priced"
    );

    let mut own = sides("ai-death-face-down", json!({ "backrow": [SHEEPISH] }), json!({}));
    grant_book_death(&mut own, AI);
    let (prices, used) = priced(&own);
    assert!(used >= 2, "its own trap is one the AI reads: {used}");
    price_of(&prices, SHEEPISH);
}

#[test]
fn r2687000_the_narcor_project_is_probed_and_prices_nothing_outside_brittle() {
    let state = sides(
        "ai-death-narcor",
        json!({ "backrow": [NARCOR_PROJECT] }),
        json!({}),
    );
    let (prices, used) = priced(&state);
    assert_eq!(
        price_of(&prices, NARCOR_PROJECT),
        0.0,
        "a sandbox destroy is no Brittle crumble, so its Death pays nothing"
    );
    assert_eq!(used, 2, "a destroy and a remove");
}

#[test]
fn r2687000_backrow_deaths_follow_the_units_and_share_their_budget() {
    let state = sides(
        "ai-death-backrow-order",
        json!({
            "field": [TRIGOR, AFTER_BURNER],
            "backrow": [BAUBLE_BUBBLE],
            "hand": [BAUBLE_BUBBLE, COMET_CRASH],
        }),
        json!({ "field": [TRAINING_DUMMY], "backrow": [JOB_APPLICATION] }),
    );
    let (prices, used) = priced(&state);
    let order: Vec<&str> = prices.iter().map(|entry| entry.def_id.as_str()).collect();
    assert_eq!(
        order,
        [
            TRIGOR,
            TRAINING_DUMMY,
            BAUBLE_BUBBLE,
            JOB_APPLICATION,
            COMET_CRASH,
            AFTER_BURNER
        ],
        "the units, the backrow cards, the hand's Field Spells (the second Bauble Bubble priced once), \
         then the Starship Piece"
    );
    assert!(used <= AI_DEATH_PROBE_NODES, "{used}");

    let (_, nodes) = priced(&sides(
        "ai-death-backrow-order",
        json!({ "field": [TRIGOR] }),
        json!({ "field": [TRAINING_DUMMY] }),
    ));
    let short = death_prices(&state, AI, &create_node_counter(nodes, None));
    let order: Vec<&str> = short.iter().map(|entry| entry.def_id.as_str()).collect();
    assert_eq!(
        order,
        [TRIGOR, TRAINING_DUMMY],
        "with only the units' nodes, no backrow card is priced"
    );
}

#[test]
fn r2687000_decide_plays_comet_crash_and_bauble_bubble_when_nothing_else_is_worth_doing() {
    for (card, mana) in [(COMET_CRASH, 2), (BAUBLE_BUBBLE, 1)] {
        let seed = format!("ai-death-play-{card}");
        let state = sides(
            &seed,
            json!({ "hand": [card], "mana": mana }),
            json!({ "hand": [VANILLA] }),
        );
        let held = state.players[AI].hand[0].id.clone();
        for cursor in [0, 1] {
            let decision =
                decide(&state, AI, &mut AiOptions::new(create_rng(&seed, cursor))).expect("the AI acts");
            assert!(
                matches!(&decision.action, ActionBody::Play { instance_id, .. } if *instance_id == held),
                "{card} at cursor {cursor}: {:?}",
                decision.action
            );
        }
    }
}
