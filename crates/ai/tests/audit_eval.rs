//! Audit of the evaluation's worth and price side (`crates/ai/src/evaluate.rs`): exploratory.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

const AI: PlayerId = PlayerId::P1;
const HUMAN: PlayerId = PlayerId::P2;

const VANILLA: &str = "core-008";
const AFTER_BURNER: &str = "stinkyscape-090";
const TMACK: &str = "stinkyscape-094";
const MENACE: &str = "core-019";
const LIBRARY: &[&str] = &[VANILLA, VANILLA, VANILLA, VANILLA];

fn board(seed: &str, mut p1: Value, mut p2: Value) -> Scenario {
    jackioh_cards::register_all();
    p1["library"] = json!(LIBRARY);
    p2["library"] = json!(LIBRARY);
    scenario(json!({ "seed": seed, "p1": p1, "p2": p2 }))
}

fn sandbox(state: &GameState, op: SandboxOp) -> GameState {
    let mut world = state.clone();
    world.sandbox = Some(true);
    let counter = create_node_counter(1000, None);
    let mut next = simulate(&world, AI, &ActionBody::Sandbox { op }, &counter, Prices::NONE)
        .expect("nodes")
        .expect("the tool is accepted");
    next.sandbox = state.sandbox;
    next
}

#[test]
fn explore_dormant_starship_launch() {
    let s = board(
        "audit-eval-starship",
        json!({ "field": [AFTER_BURNER], "hand": [VANILLA], "mana": 4 }),
        json!({ "hand": [VANILLA], "field": [VANILLA] }),
    );
    let state = s.state().clone();
    let piece = active_units_of(&state, AI)[0].id.clone();
    let state = sandbox(&state, SandboxOp::Destroy { instance_id: piece });
    let ship = units_on_field_of(&state, AI)
        .into_iter()
        .find(|card| is_dormant_starship(card))
        .expect("a Dormant Starship")
        .clone();
    println!("ship def {} slumber {:?} mana {:?}", ship.def_id, ship.slumber, state.players[AI].mana);
    println!("acts_to_awaken {}", acts_to_awaken(&state, &ship));
    println!(
        "abilities asleep {:?}",
        subsystems::abilities_of(&state, &ship).iter().map(|a| a.id.clone()).collect::<Vec<_>>()
    );
    let mut awake = ship.clone();
    awake.slumber = None;
    println!(
        "abilities awake {:?}",
        subsystems::abilities_of(&state, &awake).iter().map(|a| a.id.clone()).collect::<Vec<_>>()
    );
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = activation_prices(&state, AI, &counter);
    println!("prices {prices:?} used {}", counter.used());
    let priced = Prices { activations: prices.clone(), ..Prices::default() };
    let gained = evaluate_priced(&state, AI, NextSwing::Enemy, &AI_EVAL, &priced)
        - evaluate_priced(&state, AI, NextSwing::Enemy, &AI_EVAL, Prices::NONE);
    println!("gained {gained}; activation weight {}", AI_EVAL.activation);
}

#[test]
fn explore_vanilla_frenzy() {
    let s = board(
        "audit-eval-frenzy",
        json!({ "field": [TMACK], "hand": [VANILLA] }),
        json!({ "hand": [VANILLA], "field": [] }),
    );
    let mut state = s.state().clone();
    let id = active_units_of(&state, AI)[0].id.clone();
    {
        let card = find_instance_mut(&mut state, &id).expect("card");
        card.vanilla = true;
        card.granted_keywords.push(Keyword::Frenzy);
    }
    let unit = find_instance(&state, &id).expect("card").clone();
    println!("view {:?}", unit_view(&state, &unit));
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = attack_hook_prices(&state, AI, &counter);
    println!("frenzy vanilla prices {prices:?} used {}", counter.used());

    // The same with a Vanilla'd Midrange Menace (Taunt) that was granted Frenzy.
    let s = board(
        "audit-eval-frenzy-2",
        json!({ "field": [MENACE], "hand": [VANILLA] }),
        json!({ "hand": [VANILLA], "field": [] }),
    );
    let mut state = s.state().clone();
    let id = active_units_of(&state, AI)[0].id.clone();
    {
        let card = find_instance_mut(&mut state, &id).expect("card");
        card.vanilla = true;
        card.granted_keywords.push(Keyword::Frenzy);
    }
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let prices = attack_hook_prices(&state, AI, &counter);
    println!("menace vanilla prices {prices:?} used {}", counter.used());
    let unit = find_instance(&state, &id).expect("card").clone();
    let priced = Prices { attack_hooks: prices, ..Prices::default() };
    println!(
        "worth priced {} unpriced {}",
        unit_worth(&state, &unit, &AI_EVAL, &priced),
        unit_worth(&state, &unit, &AI_EVAL, Prices::NONE)
    );
}

#[test]
fn explore_dead_in_hand() {
    let s = board(
        "audit-eval-dead",
        json!({ "hand": [VANILLA], "field": [] }),
        json!({ "hand": [VANILLA], "field": [] }),
    );
    let state = s.state().clone();
    println!(
        "turn {} ts {} mana {:?}",
        state.turn, state.players[AI].turns_started, state.players[AI].mana
    );
}
