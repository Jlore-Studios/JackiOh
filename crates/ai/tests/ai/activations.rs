//! A unit's Activate in the AI's evaluation (R2564000): at a decision's root the AI ends its own turn
//! and then the enemy's on a copy, tries each visible unit's Activate at the start of its controller's
//! next turn as the opponent's reply tries one, and prices the best use as its controller's evaluation
//! after it less before it; `unit_worth` adds `AI_EVAL.activation` of the price for the seat's units
//! and the enemy's alike, so a Cannon that destroys a Unit every turn is worth more than its body to
//! keep and to remove. A face-down or hand card is never probed, hidden cards change no decision
//! (R185), the frozen baseline reads no price, and none is held outside a decision.
//!
//! Cannon v3 is a Starship Piece: when it dies, a copy of it fuses into its owner's Dormant Starship
//! (R1852000), which can launch and fire it again. So the removal below is a transform, which is no
//! death, and the tribute is made beside a Voidwalker, whose aura exiles a card that would go to a
//! graveyard, so the tributed card never dies (R461). Either way the Activate is all that is weighed.
//!
//! The cards: Cannon v3 (stinkyscape-052), (2) 0/4 Starship Piece, "Activate: Deal 5 damage to an
//! enemy Unit", once a turn, 10 when Radiant; Polymorph (meditative-024), (3) "Transform a Unit into a
//! Sheep Token"; The Rock (core-066), (4) 10/10 Tribute 1, Indestructible; Prejudiced Postdoc
//! (core-061), (2) 2/4 with a Cry that finds no Human here; Twisted Sorcerer (core-068), (2) 5/5 with a
//! Cry; Voidwalker (classic-050), (2) 6/3, "Aura: Cards that would go to a graveyard are exiled
//! instead"; Midrange Menace (core-019), (3) 9/9 Taunt; Mr. Vanilla (core-008), a (1) 4/4 with no
//! text; and the (1) Traps Sheepish (core-041) and My Pawn (core-096).

use std::panic::{AssertUnwindSafe, catch_unwind};

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, on_field, register_cards};

const CANNON: &str = "stinkyscape-052";
const POLYMORPH: &str = "meditative-024";
const THE_ROCK: &str = "core-066";
const POSTDOC: &str = "core-061";
const SORCERER: &str = "core-068";
const VOIDWALKER: &str = "classic-050";
const MENACE: &str = "core-019";
const VANILLA: &str = "core-008";
const SHEEPISH: &str = "core-041";
const MY_PAWN: &str = "core-096";
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
fn priced(state: &GameState) -> (Vec<ActivationPrice>, usize) {
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = activation_prices(state, AI, &counter);
    (prices, counter.used())
}

/// `player`'s unit of `def_id` on the field.
fn unit<'a>(state: &'a GameState, player: PlayerId, def_id: &str) -> &'a CardInstance {
    active_units_of(state, player)
        .into_iter()
        .find(|unit| unit.def_id == def_id)
        .expect("the unit is on the field")
}

/// The price of `controller`'s `def_id`, which must be priced.
fn price_of(prices: &[ActivationPrice], def_id: &str, controller: PlayerId) -> f64 {
    prices
        .iter()
        .find(|entry| entry.def_id == def_id && entry.controller == controller)
        .map(|entry| entry.price)
        .unwrap_or_else(|| panic!("{controller}'s {def_id}'s Activate is priced: {prices:?}"))
}

/// The AI's Sorcerer against the human's Cannon v3 and Mr. Vanilla, the human holding a Mr. Vanilla
/// and `backrow`, the AI `hand`.
fn cannon_against_vanilla(seed: &str, hand: Value, backrow: Value) -> GameState {
    board(
        seed,
        json!({ "hand": hand, "field": [SORCERER] }),
        json!({ "hand": [VANILLA], "field": [CANNON, VANILLA], "backrow": backrow }),
    )
}

/// `state` with the AI's Sorcerer just summoned, so it cannot attack this turn.
fn sorcerer_resting(mut state: GameState) -> GameState {
    let sorcerer = unit(&state, AI, SORCERER).id.clone();
    let turn = state.turn;
    find_instance_mut(&mut state, &sorcerer)
        .expect("the Sorcerer is on the field")
        .summoned_turn = Some(turn);
    state
}

#[test]
fn r2564000_an_own_cannon_is_priced_by_the_unit_its_shot_destroys() {
    let state = board(
        "ai-activation-own",
        json!({ "field": [CANNON] }),
        json!({ "field": [SORCERER] }),
    );
    let cannon = unit(&state, AI, CANNON);
    let sorcerer = unit_worth(&state, unit(&state, HUMAN, SORCERER), &AI_EVAL);
    let body = unit_worth(&state, cannon, &AI_EVAL);
    let (prices, used) = priced(&state);
    let price = price_of(&prices, CANNON, AI);
    assert!(
        price >= sorcerer,
        "its shot destroys the enemy's 5/5: {price} against {sorcerer}"
    );
    assert!(
        (3..=AI_ACTIVATION_PROBE_NODES).contains(&used),
        "two turn ends and a use at least: {used}"
    );
    let worth = with_activation_prices(prices, || unit_worth(&state, cannon, &AI_EVAL));
    assert!(
        (worth - (body + AI_EVAL.activation * price)).abs() < 1e-9,
        "{worth} against {body} and {price}"
    );
}

#[test]
fn r2564000_an_enemy_cannon_is_worth_more_to_destroy_than_a_bigger_vanilla() {
    let state = cannon_against_vanilla("ai-activation-enemy", json!([]), json!([]));
    let (cannon, vanilla) = (unit(&state, HUMAN, CANNON), unit(&state, HUMAN, VANILLA));
    let (unpriced_cannon, unpriced_vanilla) = (
        kill_worth(&state, cannon, &AI_EVAL),
        kill_worth(&state, vanilla, &AI_EVAL),
    );
    assert!(
        unpriced_cannon < unpriced_vanilla,
        "unpriced, the 0/4 is the smaller kill: {unpriced_cannon} against {unpriced_vanilla}"
    );
    let unpriced_eval = evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL);
    let (prices, _) = priced(&state);
    assert!(price_of(&prices, CANNON, HUMAN) > 0.0);
    let (cannon_kill, vanilla_kill, eval) = with_activation_prices(prices, || {
        (
            kill_worth(&state, cannon, &AI_EVAL),
            kill_worth(&state, vanilla, &AI_EVAL),
            evaluate(&state, AI, NextSwing::Enemy, &AI_EVAL),
        )
    });
    assert!(
        cannon_kill > vanilla_kill,
        "the shot it fires each turn counts: {cannon_kill} against {vanilla_kill}"
    );
    assert!(
        eval < unpriced_eval,
        "an enemy Cannon counts for more against the AI: {eval} against {unpriced_eval}"
    );
}

#[test]
fn r2564000_a_board_without_an_activate_prices_nothing_and_spends_no_node() {
    let state = board(
        "ai-activation-none",
        json!({ "hand": [CANNON], "field": [VANILLA] }),
        json!({ "field": [VANILLA] }),
    );
    assert_eq!(priced(&state), (vec![], 0));
}

#[test]
fn r2564000_given_one_removal_the_ai_removes_the_enemy_cannon() {
    let state = sorcerer_resting(cannon_against_vanilla(
        "ai-activation-removal",
        json!([POLYMORPH]),
        json!([]),
    ));
    let cannon = unit(&state, HUMAN, CANNON).id.clone();
    let polymorph = state.players[AI].hand[0].id.clone();
    let turn = play_ai_turn(
        &state,
        AI,
        &mut AiOptions::new(create_rng("ai-activation-removal", 0)),
    );
    let targets = turn.actions.iter().find_map(|action| match &action.body {
        ActionBody::Play {
            instance_id, targets, ..
        } if *instance_id == polymorph => Some(targets.clone()),
        _ => None,
    });
    assert_eq!(
        targets,
        Some(Some(vec![Selection::Instance { instance_id: cannon }])),
        "{:?}",
        turn.actions
    );
}

#[test]
fn r2564000_the_ai_tributes_its_vanilla_before_its_cannon() {
    register_cards();
    let mut s = scenario(json!({
        "seed": "ai-activation-tribute",
        "p1": {
            "hand": [THE_ROCK],
            "field": [CANNON, POSTDOC, { "def": MENACE, "position": "DEF" }],
            "library": LIBRARY,
        },
        "p2": { "hand": [VANILLA], "field": [SORCERER, VOIDWALKER], "library": LIBRARY },
    }));
    let target = s.unit_at(HUMAN, 1).id.clone();
    s.activate(
        CANNON,
        json!({ "ability": "cannon", "targets": pick::id(&target) }),
    );
    let state = s.state().clone();
    assert_eq!(
        active_units_of(&state, HUMAN).len(),
        1,
        "the use is spent on the Sorcerer and the Voidwalker is left"
    );
    let postdoc = unit(&state, AI, POSTDOC).id.clone();
    let rock = state.players[AI].hand[0].id.clone();
    let turn = play_ai_turn(
        &state,
        AI,
        &mut AiOptions::new(create_rng("ai-activation-tribute", 0)),
    );
    let tributes = turn.actions.iter().find_map(|action| match &action.body {
        ActionBody::Play {
            instance_id,
            tributes,
            ..
        } if *instance_id == rock => Some(tributes.clone()),
        _ => None,
    });
    assert_eq!(tributes, Some(Some(vec![postdoc])), "{:?}", turn.actions);
    assert!(on_field(&turn.state, AI, CANNON), "the Cannon stays");
}

#[test]
fn r2564000_a_face_down_card_is_never_probed_and_hidden_cards_change_no_decision() {
    let hidden = board(
        "ai-activation-hidden",
        json!({ "field": [SORCERER] }),
        json!({ "hand": [CANNON], "backrow": [SHEEPISH], "field": [VANILLA] }),
    );
    assert_eq!(
        priced(&hidden),
        (vec![], 0),
        "a Cannon in the enemy's hand and its face-down Trap are never probed"
    );

    let a = sorcerer_resting(cannon_against_vanilla(
        "ai-activation-r185",
        json!([POLYMORPH]),
        json!([SHEEPISH]),
    ));
    let mut b = a.clone();
    b.players[HUMAN].hand[0].def_id = CANNON.to_string();
    let trap = b.players[HUMAN]
        .backrow
        .iter_mut()
        .flatten()
        .next()
        .expect("the enemy's Trap is set");
    trap.def_id = MY_PAWN.to_string();
    let decide_on = |state: &GameState| {
        decide(
            state,
            AI,
            &mut AiOptions::new(create_rng("ai-activation-r185", 0)),
        )
        .expect("the AI acts")
    };
    let (on_a, on_b) = (decide_on(&a), decide_on(&b));
    assert_eq!(on_a.action, on_b.action);
    assert_eq!(on_a.line, on_b.line);
}

#[test]
fn r2564000_the_probes_stay_within_their_cap_and_the_decision_within_its_budget() {
    let state = board(
        "ai-activation-budget",
        json!({ "field": [CANNON, { "def": CANNON, "radiant": true }] }),
        json!({ "field": [CANNON, { "def": CANNON, "radiant": true }, SORCERER] }),
    );
    let (_, used) = priced(&state);
    assert!((2..=AI_ACTIVATION_PROBE_NODES).contains(&used), "{used}");
    let decision = decide(
        &state,
        AI,
        &mut AiOptions::new(create_rng("ai-activation-budget", 0)),
    )
    .expect("the AI acts");
    assert!(decision.stats.nodes <= AI_BUDGET.nodes, "{:?}", decision.stats);
}

#[test]
fn r2564000_the_frozen_baseline_and_a_finished_decision_read_no_price() {
    assert_eq!(GREEDY_EVAL.activation, 0.0);
    let state = board(
        "ai-activation-decide",
        json!({ "hand": [VANILLA], "field": [CANNON] }),
        json!({ "field": [SORCERER] }),
    );
    let cannon = unit(&state, AI, CANNON);
    let (prices, _) = priced(&state);
    assert!(price_of(&prices, CANNON, AI) > 0.0);
    let greedy = unit_worth(&state, cannon, &GREEDY_EVAL);
    let held = with_activation_prices(prices.clone(), || unit_worth(&state, cannon, &GREEDY_EVAL));
    assert_eq!(held, greedy, "the frozen baseline reads no price");

    let before = unit_worth(&state, cannon, &AI_EVAL);
    decide(
        &state,
        AI,
        &mut AiOptions::new(create_rng("ai-activation-decide", 0)),
    )
    .expect("the AI acts");
    assert_eq!(
        unit_worth(&state, cannon, &AI_EVAL),
        before,
        "the decision put its prices back"
    );
    let unwound = catch_unwind(AssertUnwindSafe(|| {
        with_activation_prices(prices, || panic!("a panic inside the decision"))
    }));
    assert!(unwound.is_err());
    assert_eq!(
        unit_worth(&state, cannon, &AI_EVAL),
        before,
        "a panic put the prices back too"
    );
}
