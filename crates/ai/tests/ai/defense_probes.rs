//! Fullsend 3's defensive cards (#3575) in the AI's probes. R3575600: a Unit in the seat's own hand has
//! its Activate priced as if it stood on the field (`activation_prices`, placed first as R923001 places
//! a hand Unit for its Death), so a body played for its Activate counts the use at once. R3575601: an
//! Activate that asks its controller a question after it is used (a Choose) is priced once its own
//! prompt is answered, not judged at the open prompt. R3575602: the reply prices the seat's own turn
//! hooks again on the board it leaves (`replied_turn_hook_prices`), where they fire next, so a heal counts
//! the damage the reply dealt.
//!
//! The cards: Cannon v3 (a 0/2 Starship Piece whose Activate shoots), Vela (S+ #126, a 0/10 whose
//! Activate chooses an enemy tile and deals 8 down the line), Mend Projector (S+ #41, a Field Spell, "End
//! of turn: Heal all Units to full"), the Sorcerer (a 5/5) and Mr. Vanilla (a 4/4).

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards, unit};

const CANNON: &str = "stinkyscape-052";
const VELA: &str = "stinkyscapeplus-126";
const MEND_PROJECTOR: &str = "stinkyscapeplus-041";
const SORCERER: &str = "core-068";
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

/// The AI's activation prices on `state` and the nodes their probes spent.
fn activations(state: &GameState) -> (Vec<ActivationPrice>, usize) {
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = activation_prices(state, AI, &counter);
    (prices, counter.used())
}

fn activation_of(prices: &[ActivationPrice], def_id: &str, controller: PlayerId) -> Option<f64> {
    prices
        .iter()
        .find(|entry| entry.def_id == def_id && entry.controller == controller)
        .map(|entry| entry.price)
}

fn turn_hook_of(prices: &Prices, def_id: &str) -> f64 {
    prices
        .turn_hooks
        .iter()
        .find(|entry| entry.def_id == def_id)
        .map_or(0.0, |entry| entry.price)
}

#[test]
fn r3575600_a_hand_unit_is_priced_by_its_activate_as_the_same_unit_on_the_field_is() {
    let in_hand = board(
        "ai-hand-activation",
        json!({ "hand": [CANNON] }),
        json!({ "field": [SORCERER] }),
    );
    let on_field = board(
        "ai-hand-activation",
        json!({ "field": [CANNON] }),
        json!({ "field": [SORCERER] }),
    );
    let (hand_prices, used) = activations(&in_hand);
    let (field_prices, _) = activations(&on_field);
    let from_hand = activation_of(&hand_prices, CANNON, AI).expect("the hand Cannon's Activate is priced");
    let from_field = activation_of(&field_prices, CANNON, AI).expect("the field Cannon's Activate is priced");
    assert!(
        (from_hand - from_field).abs() < 1e-9,
        "the same shot at the same 5/5: {from_hand} against {from_field}"
    );
    assert!(
        (1..=AI_ACTIVATION_HAND_PROBE_NODES).contains(&used),
        "a placement, two turn ends and a use, within the hand probe's cap: {used}"
    );
}

#[test]
fn r3575600_the_enemy_hand_is_never_probed() {
    let state = board(
        "ai-hand-activation-hidden",
        json!({ "field": [VANILLA] }),
        json!({ "hand": [CANNON], "field": [VANILLA] }),
    );
    assert_eq!(activations(&state), (vec![], 0));
}

#[test]
fn r3575601_an_activate_that_chooses_after_its_use_is_priced_once_its_prompt_is_answered() {
    let state = board(
        "ai-prompted-activation",
        json!({ "field": [VELA] }),
        json!({ "field": [VANILLA] }),
    );
    let (prices, _) = activations(&state);
    let price = activation_of(&prices, VELA, AI).expect("Vela's Activate is priced");
    let vanilla = unit_worth(&state, unit(&state, HUMAN, VANILLA), &AI_EVAL, Prices::NONE);
    assert!(
        price >= vanilla,
        "its 8 down the line destroys the enemy's 4/4: {price} against {vanilla}"
    );
}

#[test]
fn r3575602_the_reply_prices_an_own_heal_by_the_damage_it_left() {
    let undamaged = board(
        "ai-replied-heal",
        json!({ "backrow": [MEND_PROJECTOR], "field": [VANILLA] }),
        json!({ "field": [VANILLA] }),
    );
    let damaged = board(
        "ai-replied-heal",
        json!({ "backrow": [MEND_PROJECTOR], "field": [{ "def": VANILLA, "damage": 3 }] }),
        json!({ "field": [VANILLA] }),
    );
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let root = Prices {
        turn_hooks: turn_hook_prices(&undamaged, AI, &counter),
        ..Prices::default()
    };
    assert!(
        turn_hook_of(&root, MEND_PROJECTOR).abs() < 1e-9,
        "an undamaged board heals nothing: {root:?}"
    );
    let replied = create_node_counter(AI_BUDGET.nodes, None);
    let repriced =
        replied_turn_hook_prices(&damaged, AI, &replied, &root).expect("an own turn hook to price");
    let heal = turn_hook_of(&repriced, MEND_PROJECTOR);
    assert!(
        (heal - 3.0 * AI_EVAL.health).abs() < 1e-9,
        "the 3 damage the reply left, healed: {heal}"
    );
    assert!(
        replied.used() <= AI_TURN_HOOK_REPLY_PROBE_NODES,
        "within its cap: {}",
        replied.used()
    );
}

#[test]
fn r3575602_a_heal_the_enemy_also_holds_keeps_the_root_price() {
    let state = board(
        "ai-replied-heal-shared",
        json!({ "backrow": [MEND_PROJECTOR], "field": [{ "def": VANILLA, "damage": 3 }] }),
        json!({ "backrow": [MEND_PROJECTOR], "field": [VANILLA] }),
    );
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    assert_eq!(
        replied_turn_hook_prices(&state, AI, &counter, &Prices::default()),
        None
    );
    assert_eq!(counter.used(), 0);
}
