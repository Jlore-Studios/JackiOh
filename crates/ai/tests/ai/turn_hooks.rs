//! A backrow card's turn hooks in the AI's evaluation (R2477000): at a decision's root the AI fires
//! once each start-of-turn and end-of-turn hook of its own backrow, and of its own hand's Field Spells
//! (set first), and prices the firing as its controller's evaluation after it less before it, each
//! crystal of mana gained at `AI_EVAL.next_turn_mana`; `material` adds `AI_EVAL.turn_hook` of the price
//! for each own readable backrow card, so a Field Spell that pays every turn is worth more than its
//! cost. A hook that prompts is not priced, the enemy's backrow is priced as before, the frozen
//! baseline reads no price, and none is held outside a decision.
//!
//! The cards: Mana Well (core-006), (3) "Start of turn: gain 1 mana", Radiant 2; Rush Token Farm
//! (core-058), (2) "Start of turn: summon a Rush Token"; Unbiased Immigration (core-059), (2) "Start of
//! turn: add a random card to your hand"; Masochism Mask (core-065), (1) "Start of turn: choose one: …",
//! a prompt; Ace in the Hole (classic-065), (2) a Trap, "End of turn: Flip a coin. …"; the Field Spells
//! with no turn hook The Power to Thrive (classic-081, 2), Level Up! (traininggrounds-031, 3) and Mutate
//! Spell (classic-078, 1); and Mr. Vanilla (core-008), a (1) 4/4 with no text.

use std::panic::{AssertUnwindSafe, catch_unwind};

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, register_cards};

const MANA_WELL: &str = "core-006";
const RUSH_TOKEN_FARM: &str = "core-058";
const UNBIASED_IMMIGRATION: &str = "core-059";
const MASOCHISM_MASK: &str = "core-065";
const ACE_IN_THE_HOLE: &str = "classic-065";
const POWER_TO_THRIVE: &str = "classic-081";
const LEVEL_UP: &str = "traininggrounds-031";
const MUTATE_SPELL: &str = "classic-078";
const VANILLA: &str = "core-008";
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

/// The AI's evaluation of `state` with `prices` held.
fn eval_with(state: &GameState, prices: Vec<TurnHookPrice>) -> f64 {
    with_turn_hook_prices(prices, || evaluate(state, AI, NextSwing::Enemy, &AI_EVAL))
}

/// The AI's evaluation of `state` with the prices its own root would hold.
fn held_eval(state: &GameState) -> f64 {
    eval_with(state, priced(state).0)
}

/// `a` and `b` as equal, to the evaluation's rounding.
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} vs {b}");
}

/// The AI's turn 5 (three crystals) with a Mana Well and three crystals to play it with, nothing else
/// in hand and nothing on either board, the human holding a Mr. Vanilla; the AI at `tier`.
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
        [ACE_IN_THE_HOLE, MANA_WELL]
            .map(|def_id| TurnHookPrice {
                def_id: def_id.to_string(),
                radiant: false,
                price: 10.0,
            })
            .to_vec()
    };
    let enemy = board(
        "ai-hook-enemy",
        json!([]),
        json!([]),
        json!([ACE_IN_THE_HOLE, MANA_WELL]),
    );
    assert_eq!(priced(&enemy), (vec![], 0), "the enemy's backrow is never probed");
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
    let held = with_turn_hook_prices(prices.clone(), || {
        evaluate(&state, AI, NextSwing::Enemy, &GREEDY_EVAL)
    });
    assert_eq!(held, greedy, "the frozen baseline reads no price");

    let before = evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL);
    decide(&state, AI, &mut AiOptions::new(create_rng("ai-hook-decide", 0))).expect("the AI acts");
    assert_eq!(
        evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL),
        before,
        "the decision put its prices back"
    );
    let unwound = catch_unwind(AssertUnwindSafe(|| {
        with_turn_hook_prices(prices, || panic!("a panic inside the decision"))
    }));
    assert!(unwound.is_err());
    assert_eq!(
        evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL),
        before,
        "a panic put the prices back too"
    );
}
