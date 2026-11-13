//! Burn and Toxic in the AI's evaluation (R3575300): a count on a Unit or a hero (§6.3, Stinkyscape+ E2,
//! R3540000) is damage on its way, so `unit_worth` takes its coming ticks off the Unit, the next in full
//! and each later one at `AI_EVAL.affliction_turn_share` a turn its owner may answer it in, and the hero's side of
//! the ledger takes them off its life. Before it, a count was worth nothing, so S+ #144 Fire Beam v4's
//! and #146 Scourge Mortar v3's Activates had no price (R2564000) and a hit #151 Volcanic Rounds turned
//! into Burn read as no hit at all. The frozen baseline prices none of it.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, register_cards, unit};

const BEAM: &str = "stinkyscapeplus-144";
const MORTAR: &str = "stinkyscapeplus-146";
const VOLCANIC: &str = "stinkyscapeplus-151";
const VANILLA: &str = "core-008";
const LIBRARY: &[&str] = &[VANILLA, VANILLA, VANILLA, VANILLA];

/// The AI's turn with the AI's side `p1` and the human's `p2`, each with `LIBRARY` added.
fn board(seed: &str, mut p1: Value, mut p2: Value) -> GameState {
    register_cards();
    p1["library"] = json!(LIBRARY);
    p2["library"] = json!(LIBRARY);
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
        .state()
        .clone()
}

/// The human's Mr. Vanilla, carrying `counters`.
fn vanilla_with(counters: Value) -> GameState {
    board(
        "r3575300-vanilla",
        json!({}),
        json!({ "field": [{ "def": VANILLA, "counters": counters }] }),
    )
}

fn worth(state: &GameState, w: &EvalWeights) -> f64 {
    unit_worth(state, unit(state, HUMAN, VANILLA), w, Prices::NONE)
}

fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} vs {b}");
}

#[test]
fn r3575300_a_burn_that_kills_at_its_next_tick_takes_the_whole_unit() {
    // Mr. Vanilla is a 4/4: Burn 4 kills it at the start of its owner's next turn, before it acts, so
    // the tick counts in full, and a Vanilla's Death leaves nothing.
    close(worth(&vanilla_with(json!({ "burn": 4 })), &AI_EVAL), 0.0);
}

#[test]
fn r3575300_a_burn_that_does_not_kill_takes_its_ticks_at_healths_worth_and_decays() {
    let clean = worth(&vanilla_with(json!({})), &AI_EVAL);
    // Burn 2: one tick of 2 in full, then nothing (2 − BURN_DECAY).
    let burned = worth(&vanilla_with(json!({ "burn": 2 })), &AI_EVAL);
    close(burned, clean - AI_EVAL.health * 2.0);
    // Burn 5 deals 5, then 2 a turn later: the second at the share. On a 4/4 the first kills.
    let two = board(
        "r3575300-tough",
        json!({}),
        json!({ "field": [{ "def": VANILLA, "counters": { "burn": 2 }, "statsOverride": { "attack": 4, "health": 9 } }] }),
    );
    let tough = board(
        "r3575300-tough",
        json!({}),
        json!({ "field": [{ "def": VANILLA, "statsOverride": { "attack": 4, "health": 9 } }] }),
    );
    let five = board(
        "r3575300-tough",
        json!({}),
        json!({ "field": [{ "def": VANILLA, "counters": { "burn": 5 }, "statsOverride": { "attack": 4, "health": 9 } }] }),
    );
    let s = AI_EVAL.affliction_turn_share;
    let h = AI_EVAL.health;
    close(worth(&two, &AI_EVAL), worth(&tough, &AI_EVAL) - 2.0 * h);
    close(
        worth(&five, &AI_EVAL),
        worth(&tough, &AI_EVAL) - 5.0 * h - s * 2.0 * h,
    );
}

#[test]
fn r3575300_toxic_lasts_until_it_kills_each_tick_after_the_next_further_off() {
    let clean = worth(&vanilla_with(json!({})), &AI_EVAL);
    let s = AI_EVAL.affliction_turn_share;
    let h = AI_EVAL.health;
    // Toxic 1 on a 4/4: three ticks of 1, the first in full, then the fourth kills it.
    let poisoned = worth(&vanilla_with(json!({ "toxic": 1 })), &AI_EVAL);
    let loss = h + s * h + s * s * h + s.powi(3) * (clean - 3.0 * h);
    close(poisoned, clean - loss);
}

#[test]
fn r3575300_the_frozen_baseline_prices_no_count() {
    let clean = worth(&vanilla_with(json!({})), &GREEDY_EVAL);
    close(
        worth(&vanilla_with(json!({ "burn": 4, "toxic": 2 })), &GREEDY_EVAL),
        clean,
    );
}

#[test]
fn r3575300_burn_and_toxic_on_the_enemy_hero_are_life_it_still_loses() {
    let state = board("r3575300-hero", json!({}), json!({}));
    let before = evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL);
    for counter in ["burn", "toxic"] {
        let mut afflicted = state.clone();
        match counter {
            "burn" => afflicted.players[HUMAN].hero_burn = Some(4),
            _ => afflicted.players[HUMAN].hero_toxic = Some(4),
        }
        let after = evaluate(&afflicted, AI, NextSwing::Enemy, &AI_EVAL);
        assert!(
            after > before,
            "the enemy hero's {counter} counts for the AI: {after} vs {before}"
        );
        close(evaluate(&afflicted, AI, NextSwing::Enemy, &GREEDY_EVAL), {
            evaluate(&state, AI, NextSwing::Enemy, &GREEDY_EVAL)
        });
        // And against the side that carries it.
        let mut own = state.clone();
        own.players[AI].hero_burn = afflicted.players[HUMAN].hero_burn;
        own.players[AI].hero_toxic = afflicted.players[HUMAN].hero_toxic;
        assert!(evaluate(&own, AI, NextSwing::Enemy, &AI_EVAL) < before);
    }
}

#[test]
fn r3575300_the_beam_and_the_mortar_have_a_price_now() {
    for def in [BEAM, MORTAR] {
        let state = board(
            "r3575300-price",
            json!({ "field": [{ "def": def }] }),
            json!({ "field": [VANILLA] }),
        );
        let counter = create_node_counter(AI_BUDGET.nodes, None);
        let prices = activation_prices(&state, AI, &counter);
        let price = prices
            .iter()
            .find(|entry| entry.def_id == def && entry.controller == AI)
            .map(|entry| entry.price);
        assert!(
            price.is_some_and(|price| price > 0.0),
            "{def}'s Activate is priced: {prices:?}"
        );
    }
}

#[test]
fn r3575300_a_hit_volcanic_rounds_turns_to_burn_still_counts() {
    // A Volcanic Rounds (3/1) on the field: its hit on the 4/4 is Burn 3, which deals 3 at the start of
    // its owner's turn.
    let state = board(
        "r3575300-volcanic",
        json!({ "field": [{ "def": VOLCANIC }] }),
        json!({ "field": [VANILLA] }),
    );
    let attacker = unit(&state, AI, VOLCANIC).id.clone();
    let target = unit(&state, HUMAN, VANILLA).id.clone();
    let before = evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL);
    let hit = act(
        &state,
        AI,
        ActionBody::Attack {
            attacker_id: attacker,
            target_id: target,
        },
    );
    let burn = unit(&hit, HUMAN, VANILLA).counters.burn;
    assert_eq!(burn, Some(3), "the hit is Burn, not damage");
    let after = evaluate(&hit, AI, NextSwing::Enemy, &AI_EVAL);
    let unpriced = EvalWeights {
        affliction_turn_share: 0.0,
        ..AI_EVAL
    };
    let after_unpriced = evaluate(&hit, AI, NextSwing::Enemy, &unpriced);
    assert!(
        after > after_unpriced,
        "the Burn counts for the attacker: {after} vs {after_unpriced} unpriced (before {before})"
    );
}

/// R3575301: a Fire Beam in the AI's own hand is priced as if placed, so a line that plays it reads its
/// Activate at once; the enemy's hand is never probed.
#[test]
fn r3575301_a_unit_in_the_seats_own_hand_has_its_activate_priced() {
    let state = board(
        "r3575301-hand",
        json!({ "hand": [BEAM], "field": [VANILLA] }),
        json!({ "hand": [MORTAR], "field": [VANILLA] }),
    );
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = activation_prices(&state, AI, &counter);
    assert!(
        prices
            .iter()
            .any(|entry| entry.def_id == BEAM && entry.controller == AI && entry.price > 0.0),
        "the hand Fire Beam is priced: {prices:?}"
    );
    assert!(
        !prices.iter().any(|entry| entry.def_id == MORTAR),
        "the enemy's hand is never probed: {prices:?}"
    );
    assert!(counter.used() <= AI_ACTIVATION_PROBE_NODES);
}

/// R3575301: a Fire Beam equipped into a host during the search lends the host its Activate (R1751000),
/// and the host, which its root never priced, reads the beam's price through the ingredient that prints
/// the ability (R3291000).
#[test]
fn r3575301_an_equips_host_reads_the_price_of_the_piece_it_fused() {
    let state = board(
        "r3575301-equip",
        json!({ "hand": [BEAM], "field": [VANILLA] }),
        json!({ "field": [VANILLA] }),
    );
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = Prices {
        activations: activation_prices(&state, AI, &counter),
        ..Prices::default()
    };
    let beam = prices
        .activations
        .iter()
        .find(|entry| entry.def_id == BEAM)
        .map(|entry| entry.price)
        .expect("the hand Fire Beam is priced");
    let equip = legal_actions(&state, AI)
        .into_iter()
        .find(|action| {
            matches!(
                action,
                ActionBody::Play {
                    equip: Some(true),
                    ..
                }
            )
        })
        .expect("the Fire Beam may be equipped into the Vanilla");
    let equipped = act(&state, AI, equip);
    // The fusion is a card of its own definition (R1086), the AI's one unit.
    let units = active_units_of(&equipped, AI);
    let [host] = units.as_slice() else {
        panic!("the host is the AI's one unit: {units:?}");
    };
    let priced = unit_worth(&equipped, host, &AI_EVAL, &prices);
    let unpriced = unit_worth(&equipped, host, &AI_EVAL, Prices::NONE);
    close(priced - unpriced, AI_EVAL.activation * beam);
}
