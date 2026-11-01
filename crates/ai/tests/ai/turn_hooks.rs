//! A backrow card's turn hooks in the AI's evaluation (R2477000): the decision's root fires each
//! start-of-turn and end-of-turn hook of the seat's backrow and hand Field Spells once
//! (`turn_hook_prices`), and `material` adds `AI_EVAL.turn_hook` of the price. A hook that prompts is not
//! priced, the frozen baseline reads no price, and a call not passed a decision's prices reads none.
//! R2565000 prices the same hooks for a Unit on either field and for a Field Spell the seat can read in
//! the enemy's backrow too; a face-down card of the enemy's is never probed (R185).
//!
//! The cards: Mana Well (Radiant 2), Rush Token Farm, Unbiased Immigration, Masochism Mask (a prompt),
//! Ace in the Hole (a Trap), Gatling Pea, Gravedigger, Meltdown Layer (a token that harms its holder),
//! three Field Spells with no turn hook, and Mr. Vanilla.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards};

const MANA_WELL: &str = "core-006";
const RUSH_TOKEN_FARM: &str = "core-058";
const UNBIASED_IMMIGRATION: &str = "core-059";
const MASOCHISM_MASK: &str = "core-065";
const ACE_IN_THE_HOLE: &str = "classic-065";
const POWER_TO_THRIVE: &str = "classic-081";
const LEVEL_UP: &str = "traininggrounds-031";
const MUTATE_SPELL: &str = "classic-078";
const VANILLA: &str = "core-008";
const GATLING_PEA: &str = "meditative-013";
const GRAVEDIGGER: &str = "core-037";
const MELTDOWN_LAYER: &str = "stinkyscape-t-meltdown-layer";
const HIT_JOB: &str = "core-016";
const HARMONY_SPIRIT: &str = "meditativeplus-021";
const LIBRARY: &[&str] = &[VANILLA, VANILLA, VANILLA, VANILLA];

/// The AI's turn with `hand` and `backrow`, the human's `enemy` backrow and a Mr. Vanilla in the
/// human's hand.
fn board(seed: &str, hand: Value, backrow: Value, enemy: Value) -> GameState {
    register_cards();
    scenario(json!({
        "seed": seed,
        "p1": { "hand": hand, "backrow": backrow, "library": LIBRARY },
        "p2": { "hand": [VANILLA], "backrow": enemy, "library": LIBRARY },
    }))
    .state()
    .clone()
}

/// The AI's turn-hook prices on `state` and the nodes their probes spent.
fn priced(state: &GameState) -> (Vec<TurnHookPrice>, usize) {
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = turn_hook_prices(state, AI, &counter);
    (prices, counter.used())
}

/// The price of `def_id`'s face `radiant`, which must be priced.
fn price_of(prices: &[TurnHookPrice], def_id: &str, radiant: bool) -> f64 {
    prices
        .iter()
        .find(|entry| entry.def_id == def_id && entry.radiant == radiant)
        .map(|entry| entry.price)
        .unwrap_or_else(|| panic!("{def_id}'s turn hooks are priced: {prices:?}"))
}

/// `turn_hooks` as a decision's prices, with no other price.
fn prices_of(turn_hooks: Vec<TurnHookPrice>) -> Prices {
    Prices {
        turn_hooks,
        ..Prices::default()
    }
}

/// The AI's evaluation of `state` passed `prices`, as a decision passes them.
fn eval_with(state: &GameState, prices: Vec<TurnHookPrice>) -> f64 {
    let prices = Prices {
        turn_hooks: prices,
        ..Prices::default()
    };
    evaluate_priced(state, AI, NextSwing::Enemy, &AI_EVAL, &prices)
}

/// The AI's evaluation of `state` with the prices its own root would pass.
fn held_eval(state: &GameState) -> f64 {
    eval_with(state, priced(state).0)
}

/// `a` and `b` as equal, to the evaluation's rounding.
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} vs {b}");
}

/// The AI's turn 5 with a Mana Well and the three crystals to play it, nothing else in hand or on
/// either board; the AI at `tier`.
fn mana_well_on_curve(tier: Handicap) -> GameState {
    register_cards();
    let mut state = scenario(json!({
        "seed": "ai-hook-curve",
        "turn": 5,
        "p1": { "hand": [MANA_WELL], "mana": 3, "library": LIBRARY },
        "p2": { "hand": [VANILLA], "library": LIBRARY },
    }))
    .state()
    .clone();
    state.players[AI].handicap = Some(tier);
    state
}

#[test]
fn r2477000_an_own_mana_well_prices_the_crystal_it_gains() {
    let state = board("ai-hook-well", json!([]), json!([MANA_WELL]), json!([]));
    let (prices, used) = priced(&state);
    close(price_of(&prices, MANA_WELL, false), AI_EVAL.next_turn_mana);
    assert_eq!(used, 1, "one start-of-turn hook, one node");

    let radiant = board(
        "ai-hook-well",
        json!([]),
        json!([{ "def": MANA_WELL, "radiant": true }]),
        json!([]),
    );
    let (prices, _) = priced(&radiant);
    close(price_of(&prices, MANA_WELL, true), 2.0 * AI_EVAL.next_turn_mana);
}

#[test]
fn r2477000_an_own_hook_field_spell_scores_above_a_hookless_one_of_its_cost() {
    for (hooked, hookless) in [
        (RUSH_TOKEN_FARM, POWER_TO_THRIVE),
        (UNBIASED_IMMIGRATION, POWER_TO_THRIVE),
        (MANA_WELL, LEVEL_UP),
    ] {
        let with_hook = board("ai-hook-pair", json!([]), json!([hooked]), json!([]));
        let without = board("ai-hook-pair", json!([]), json!([hookless]), json!([]));
        close(
            evaluate(&with_hook, AI, NextSwing::Enemy, &AI_EVAL),
            evaluate(&without, AI, NextSwing::Enemy, &AI_EVAL),
        );
        let (prices, _) = priced(&with_hook);
        let price = price_of(&prices, hooked, false);
        assert!(price > 0.0, "{hooked}'s firing pays its owner: {price}");
        let (hooked_eval, hookless_eval) = (held_eval(&with_hook), held_eval(&without));
        assert!(
            hooked_eval > hookless_eval,
            "{hooked} {hooked_eval} against {hookless} {hookless_eval}"
        );
    }
}

#[test]
fn r2477000_a_masochism_mask_scores_no_more_than_a_hookless_one() {
    let mask = board("ai-hook-mask", json!([]), json!([MASOCHISM_MASK]), json!([]));
    let (prices, used) = priced(&mask);
    assert!(
        prices.is_empty(),
        "its hook prompts, so it is not priced: {prices:?}"
    );
    assert_eq!(used, 1);
    let mutate = board("ai-hook-mask", json!([]), json!([MUTATE_SPELL]), json!([]));
    let (mask_eval, mutate_eval) = (held_eval(&mask), held_eval(&mutate));
    assert!(mask_eval <= mutate_eval, "{mask_eval} against {mutate_eval}");
}

#[test]
fn r2477000_an_enemy_face_down_card_is_priced_as_before() {
    let held = || {
        [ACE_IN_THE_HOLE]
            .map(|def_id| TurnHookPrice {
                def_id: def_id.to_string(),
                radiant: false,
                price: 10.0,
            })
            .to_vec()
    };
    let enemy = board("ai-hook-enemy", json!([]), json!([]), json!([ACE_IN_THE_HOLE]));
    assert_eq!(
        priced(&enemy),
        (vec![], 0),
        "an enemy face-down card is never probed"
    );
    assert_eq!(
        eval_with(&enemy, held()),
        evaluate(&enemy, AI, NextSwing::Enemy, &AI_EVAL),
        "nor priced"
    );

    let own = board("ai-hook-enemy", json!([]), json!([ACE_IN_THE_HOLE]), json!([]));
    close(
        eval_with(&own, held()) - evaluate(&own, AI, NextSwing::Enemy, &AI_EVAL),
        AI_EVAL.turn_hook * 10.0,
    );
}

#[test]
fn r2477000_a_hand_field_spell_is_priced_by_setting_it_first() {
    let state = board("ai-hook-hand", json!([MANA_WELL]), json!([]), json!([]));
    let (prices, used) = priced(&state);
    close(price_of(&prices, MANA_WELL, false), AI_EVAL.next_turn_mana);
    assert_eq!(used, 2, "a spawn and a firing");
}

#[test]
fn r2477000_a_board_without_a_turn_hook_prices_nothing_and_spends_no_node() {
    let state = board(
        "ai-hook-none",
        json!([VANILLA, POWER_TO_THRIVE]),
        json!([LEVEL_UP]),
        json!([]),
    );
    assert_eq!(priced(&state), (vec![], 0));
}

#[test]
fn r2477000_the_probes_stay_within_their_cap_and_the_decision_within_its_budget() {
    let state = board(
        "ai-hook-budget",
        json!([MANA_WELL, { "def": MANA_WELL, "radiant": true }, UNBIASED_IMMIGRATION]),
        json!([RUSH_TOKEN_FARM, MASOCHISM_MASK, ACE_IN_THE_HOLE, LEVEL_UP]),
        json!([]),
    );
    let (_, used) = priced(&state);
    assert!((1..=AI_TURN_HOOK_PROBE_NODES).contains(&used), "{used}");
    let decision =
        decide(&state, AI, &mut AiOptions::new(create_rng("ai-hook-budget", 0))).expect("the AI acts");
    assert!(decision.stats.nodes <= AI_BUDGET.nodes, "{:?}", decision.stats);
}

#[test]
fn r2477000_decide_plays_mana_well_with_spare_mana_at_medium_and_hard() {
    for tier in [AI_DIFFICULTY.medium, AI_DIFFICULTY.hard] {
        let state = mana_well_on_curve(tier);
        let well = state.players[AI].hand[0].id.clone();
        for cursor in 0..2 {
            let decision = decide(
                &state,
                AI,
                &mut AiOptions::new(create_rng("ai-hook-curve", cursor)),
            )
            .expect("a decision");
            assert!(
                matches!(&decision.action, ActionBody::Play { instance_id, .. } if *instance_id == well),
                "{tier:?} cursor {cursor}: {:?}",
                decision.action
            );
        }
    }
}

#[test]
fn r2477000_the_frozen_baseline_and_a_finished_decision_read_no_price() {
    assert_eq!(GREEDY_EVAL.turn_hook, 0.0);
    let state = board(
        "ai-hook-decide",
        json!([VANILLA]),
        json!([RUSH_TOKEN_FARM]),
        json!([]),
    );
    let (prices, _) = priced(&state);
    assert!(price_of(&prices, RUSH_TOKEN_FARM, false) > 0.0);
    let greedy = evaluate(&state, AI, NextSwing::Enemy, &GREEDY_EVAL);
    let prices = Prices {
        turn_hooks: prices,
        ..Prices::default()
    };
    let held = evaluate_priced(&state, AI, NextSwing::Enemy, &GREEDY_EVAL, &prices);
    assert_eq!(held, greedy, "the frozen baseline reads no price");

    let before = evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL);
    decide(&state, AI, &mut AiOptions::new(create_rng("ai-hook-decide", 0))).expect("the AI acts");
    assert_eq!(
        evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL),
        before,
        "a decision leaves no price behind"
    );
}

/// The AI's turn with `p1` and `p2` as their sides' setups, a stocked library under each.
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

fn unit<'a>(state: &'a GameState, player: PlayerId, def_id: &str) -> &'a CardInstance {
    active_units_of(state, player)
        .into_iter()
        .find(|unit| unit.def_id == def_id)
        .expect("the unit is on the field")
}

#[test]
fn r2565000_a_unit_with_a_turn_hook_is_priced_on_either_field() {
    let own = sides("ai-hook-unit", json!({ "field": [GATLING_PEA] }), json!({}));
    let (prices, used) = priced(&own);
    assert!(price_of(&prices, GATLING_PEA, false) > 0.0, "{prices:?}");
    assert_eq!(used, 1, "one end-of-turn hook, one node");

    let enemy = sides("ai-hook-unit", json!({}), json!({ "field": [GATLING_PEA] }));
    let (prices, used) = priced(&enemy);
    assert!(
        price_of(&prices, GATLING_PEA, false) > 0.0,
        "the enemy's hit is priced from its side: {prices:?}"
    );
    assert_eq!(used, 1);

    let both = sides(
        "ai-hook-unit",
        json!({ "field": [GATLING_PEA] }),
        json!({ "field": [GATLING_PEA] }),
    );
    let (prices, used) = priced(&both);
    assert_eq!(
        prices.len(),
        1,
        "a card the same as one listed is not priced again: {prices:?}"
    );
    assert_eq!(used, 1);
}

#[test]
fn r2565000_unit_worth_carries_the_price_for_the_unit_s_controller() {
    for (seed, p1, p2, owner) in [
        ("ai-hook-worth", json!({ "field": [GATLING_PEA] }), json!({}), AI),
        (
            "ai-hook-worth",
            json!({}),
            json!({ "field": [GATLING_PEA] }),
            HUMAN,
        ),
    ] {
        let state = sides(seed, p1, p2);
        let pea = unit(&state, owner, GATLING_PEA);
        let (prices, _) = priced(&state);
        let price = price_of(&prices, GATLING_PEA, false);
        let prices = prices_of(prices);
        let bare = unit_worth(&state, pea, &AI_EVAL, Prices::NONE);
        let held = unit_worth(&state, pea, &AI_EVAL, &prices);
        close(held - bare, AI_EVAL.turn_hook * price);
        let kill_bare = kill_worth(&state, pea, &AI_EVAL, Prices::NONE);
        let kill_held = kill_worth(&state, pea, &AI_EVAL, &prices);
        close(kill_held - kill_bare, AI_EVAL.turn_hook * price);
        let greedy = unit_worth(&state, pea, &GREEDY_EVAL, &prices);
        close(greedy, unit_worth(&state, pea, &GREEDY_EVAL, Prices::NONE));
    }
}

#[test]
fn r2565000_an_enemy_engine_scores_below_a_blank_body_and_an_own_one_above() {
    let engine = sides("ai-hook-side", json!({}), json!({ "field": [GATLING_PEA] }));
    let (prices, _) = priced(&engine);
    let bare = evaluate(&engine, AI, NextSwing::Enemy, &AI_EVAL);
    assert!(
        eval_with(&engine, prices) < bare,
        "an enemy engine counts for its owner"
    );

    let own = sides("ai-hook-side", json!({ "field": [GATLING_PEA] }), json!({}));
    let (prices, _) = priced(&own);
    assert!(
        eval_with(&own, prices) > evaluate(&own, AI, NextSwing::Enemy, &AI_EVAL),
        "an own engine counts for the seat"
    );
}

#[test]
fn r2565000_an_enemy_face_up_field_spell_is_priced_a_face_down_trap_is_not() {
    let state = board(
        "ai-hook-readable",
        json!([]),
        json!([]),
        json!([ACE_IN_THE_HOLE, MANA_WELL]),
    );
    let (prices, used) = priced(&state);
    assert_eq!(used, 1, "the Field Spell's one hook, and nothing for the trap");
    assert_eq!(prices.len(), 1, "{prices:?}");
    assert!(
        price_of(&prices, MANA_WELL, false) > 0.0,
        "the enemy's mana is the enemy's gain"
    );
    let bare = evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL);
    close(
        bare - eval_with(&state, prices),
        AI_EVAL.turn_hook * AI_EVAL.next_turn_mana,
    );
    let trap = vec![TurnHookPrice {
        def_id: ACE_IN_THE_HOLE.to_string(),
        radiant: false,
        price: 10.0,
    }];
    close(eval_with(&state, trap), bare);
}

#[test]
fn r2565000_a_hand_unit_is_priced_by_placing_it_first() {
    let state = sides("ai-hook-hand-unit", json!({ "hand": [GATLING_PEA] }), json!({}));
    let (prices, used) = priced(&state);
    assert!(price_of(&prices, GATLING_PEA, false) > 0.0, "{prices:?}");
    assert_eq!(used, 2, "a spawn and a firing");

    let full = sides(
        "ai-hook-hand-unit",
        json!({ "hand": [GATLING_PEA], "field": [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA] }),
        json!({}),
    );
    assert_eq!(active_units_of(&full, AI).len(), 5);
    let (prices, _) = priced(&full);
    assert!(
        price_of(&prices, GATLING_PEA, false) > 0.0,
        "a full row still prices it: {prices:?}"
    );
}

#[test]
fn r2565000_a_hand_unit_without_a_turn_hook_is_not_probed() {
    let state = sides(
        "ai-hook-hand-plain",
        json!({ "hand": [VANILLA, HIT_JOB] }),
        json!({ "field": [VANILLA] }),
    );
    assert_eq!(priced(&state), (vec![], 0));
}

#[test]
fn r2565000_a_turn_hook_that_harms_its_holder_prices_below_zero() {
    let own = sides("ai-hook-harm", json!({ "field": [MELTDOWN_LAYER] }), json!({}));
    let (prices, _) = priced(&own);
    assert!(
        price_of(&prices, MELTDOWN_LAYER, false) < 0.0,
        "its end of turn costs it health: {prices:?}"
    );
    let worth = unit_worth(&own, unit(&own, AI, MELTDOWN_LAYER), &AI_EVAL, Prices::NONE);
    let held = unit_worth(&own, unit(&own, AI, MELTDOWN_LAYER), &AI_EVAL, &prices_of(prices));
    assert!(held < worth, "{held} against {worth}");

    let enemy = sides("ai-hook-harm", json!({}), json!({ "field": [MELTDOWN_LAYER] }));
    let (prices, _) = priced(&enemy);
    assert!(price_of(&prices, MELTDOWN_LAYER, false) < 0.0, "{prices:?}");
}

#[test]
fn r2565000_a_gravedigger_is_priced_by_the_card_it_returns() {
    let full = sides(
        "ai-hook-digger",
        json!({ "field": [GRAVEDIGGER], "graveyard": [VANILLA, VANILLA] }),
        json!({}),
    );
    let (prices, used) = priced(&full);
    assert!(price_of(&prices, GRAVEDIGGER, false) > 0.0, "{prices:?}");
    assert_eq!(used, 1);
}

#[test]
fn r2565000_the_probes_of_two_crowded_boards_stay_within_their_cap_and_the_decision_within_its_budget() {
    let state = sides(
        "ai-hook-crowd",
        json!({
            "hand": [GATLING_PEA, MANA_WELL, { "def": GRAVEDIGGER, "radiant": true }],
            "field": [GRAVEDIGGER, MELTDOWN_LAYER],
            "backrow": [RUSH_TOKEN_FARM],
            "graveyard": [VANILLA, VANILLA],
        }),
        json!({
            "field": [GATLING_PEA, { "def": GATLING_PEA, "radiant": true }, GRAVEDIGGER],
            "backrow": [UNBIASED_IMMIGRATION, ACE_IN_THE_HOLE],
            "graveyard": [VANILLA, VANILLA],
        }),
    );
    let (prices, used) = priced(&state);
    assert!((1..=AI_TURN_HOOK_PROBE_NODES).contains(&used), "{used}");
    assert!(
        prices.iter().all(|entry| entry.def_id != ACE_IN_THE_HOLE),
        "{prices:?}"
    );
    let decision =
        decide(&state, AI, &mut AiOptions::new(create_rng("ai-hook-crowd", 0))).expect("the AI acts");
    assert!(decision.stats.nodes <= AI_BUDGET.nodes, "{:?}", decision.stats);
}

/// The id of the card `action` plays, and the instance it targets.
fn played_target(action: &ActionBody) -> Option<&str> {
    match action {
        ActionBody::Play {
            targets: Some(targets),
            ..
        } => match targets.first() {
            Some(Selection::Instance { instance_id }) => Some(instance_id),
            _ => None,
        },
        _ => None,
    }
}

#[test]
fn r2565000_decide_removes_a_gatling_pea_before_a_vanilla_unit_of_better_stats() {
    // A 3/7 body is worth a little more than the Pea's 2/6 and Armor 2 (10.6 against 10) before its
    // turn hook, which the price lifts over it.
    let state = sides(
        "ai-hook-removal",
        json!({ "hand": [HIT_JOB], "mana": 3 }),
        json!({ "field": [GATLING_PEA, { "def": VANILLA, "statsOverride": { "attack": 3, "health": 7 } }] }),
    );
    let pea = unit(&state, HUMAN, GATLING_PEA).id.clone();
    for cursor in 0..2 {
        let decision = decide(
            &state,
            AI,
            &mut AiOptions::new(create_rng("ai-hook-removal", cursor)),
        )
        .expect("the AI acts");
        assert_eq!(
            played_target(&decision.action),
            Some(pea.as_str()),
            "cursor {cursor}: {:?}",
            decision.action
        );
    }
}

#[test]
fn r2565000_decide_trades_a_vanilla_unit_before_its_own_gravedigger() {
    // A 4/5 Mr. Vanilla and a Gravedigger are the same body; the enemy's Taunt 5/4 must be attacked
    // and kills what attacks it, so one of them trades. The Gravedigger, with cards in the graveyard to
    // return, is the one the AI keeps, wherever it stands in the row.
    let vanilla = json!({ "def": VANILLA, "statsOverride": { "attack": 4, "health": 5 } });
    let taunt = json!({ "def": HARMONY_SPIRIT, "statsOverride": { "attack": 5, "health": 4 } });
    for field in [json!([GRAVEDIGGER, vanilla]), json!([vanilla, GRAVEDIGGER])] {
        let state = sides(
            "ai-hook-trade",
            json!({ "field": field, "graveyard": [VANILLA, VANILLA] }),
            json!({ "field": [taunt] }),
        );
        let digger = unit(&state, AI, GRAVEDIGGER).id.clone();
        let (prices, _) = priced(&state);
        assert!(price_of(&prices, GRAVEDIGGER, false) > 0.0, "{prices:?}");
        for cursor in 0..2 {
            let decision = decide(
                &state,
                AI,
                &mut AiOptions::new(create_rng("ai-hook-trade", cursor)),
            )
            .expect("the AI acts");
            match &decision.action {
                ActionBody::Attack { attacker_id, .. } => {
                    assert_ne!(*attacker_id, digger, "cursor {cursor}: {:?}", decision.action)
                }
                other => panic!("cursor {cursor}: the AI trades: {other:?}"),
            }
        }
    }
}
