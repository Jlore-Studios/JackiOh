//! R4808000: what `search_signature` and `unit_lanes` sign beyond the board's face (issue #4808): the
//! last Spell while a copier of it is in the seat's reach (C #57 Echo), the damage chain a Churnwalker
//! put a Unit in (S+ #7), and the lane of a Dormant card and of an animated card's home zone. Each part
//! only when there is one, so every other position signs as before.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, register_cards};

const WILDFIRE: &str = "classic-055";
const ECHO: &str = "classic-057";
const COIN: &str = "core-t-coin";
const CHURNWALKER: &str = "stinkyscapeplus-007";
const SHIP_SMITH: &str = "stinkyscape-017";
const WARAXE: &str = "meditative-066";
const SEVEN: &str = "core-025";
const VANILLA: &str = "core-008";
const LIBRARY: [&str; 3] = [VANILLA, VANILLA, VANILLA];

fn id_of(state: &GameState, def_id: &str) -> String {
    let side = &state.players[AI];
    side.hand
        .iter()
        .chain(side.graveyard.iter())
        .find(|card| card.def_id == def_id)
        .unwrap_or_else(|| panic!("no {def_id}"))
        .id
        .clone()
}

/// The seat's legal plays of `def_id`, from its hand or graveyard.
fn plays(state: &GameState, def_id: &str) -> Vec<ActionBody> {
    let id = id_of(state, def_id);
    legal_actions(state, AI)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == id))
        .collect()
}

fn in_lane(state: &GameState, def_id: &str, lane: i32) -> ActionBody {
    plays(state, def_id)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Play { zone: Some(zone), .. } if zone.lane == lane))
        .unwrap_or_else(|| panic!("{def_id} has no play into lane {lane}"))
}

fn at_enemy_hero(state: &GameState, def_id: &str) -> ActionBody {
    plays(state, def_id)
        .into_iter()
        .find(|action| {
            matches!(action, ActionBody::Play { targets: Some(targets), .. }
                if targets.iter().any(|picked| matches!(picked, Selection::Hero { player } if *player == PlayerId::P2)))
        })
        .unwrap_or_else(|| panic!("{def_id} cannot aim at the enemy hero"))
}

fn step(state: &GameState, action: &ActionBody, n: usize) -> GameState {
    let next = reduce(state, &Action::new(action.clone(), AI, format!("r4808000:{n}")));
    assert!(next.error.is_none(), "{:?}", next.error);
    next.state
}

fn keys(actions: &[ActionBody]) -> Vec<String> {
    actions.iter().map(action_key).collect()
}

/// p1 holds C #55 Book of Wildfire, `extra` and The Coin with 1 mana, against a hero at 8.
fn spells_board(extra: Value, graveyard: Value) -> GameState {
    register_cards();
    scenario(json!({
        "seed": "r4808000-echo",
        "p1": { "hand": [WILDFIRE, extra, COIN], "graveyard": graveyard, "library": LIBRARY, "mana": 1 },
        "p2": { "hand": [VANILLA], "library": LIBRARY, "health": 8 },
    }))
    .state()
    .clone()
}

/// Coin then Wildfire at the enemy hero, and Wildfire at the enemy hero then Coin.
fn two_orders(root: &GameState) -> (GameState, GameState) {
    let coin_first = step(root, &plays(root, COIN)[0], 1);
    let coin_then_fire = step(&coin_first, &at_enemy_hero(&coin_first, WILDFIRE), 2);
    let fire_first = step(root, &at_enemy_hero(root, WILDFIRE), 3);
    let fire_then_coin = step(&fire_first, &plays(&fire_first, COIN)[0], 4);
    (coin_then_fire, fire_then_coin)
}

#[test]
fn r4808000_two_orders_of_two_spells_that_leave_echo_two_texts_sign_apart() {
    let (coin_then_fire, fire_then_coin) = two_orders(&spells_board(json!(ECHO), json!([])));
    assert_ne!(
        keys(&legal_actions(&coin_then_fire, AI)),
        keys(&legal_actions(&fire_then_coin, AI)),
        "Echo is Wildfire after one order and The Coin after the other"
    );
    assert_ne!(
        search_signature(&coin_then_fire, AI),
        search_signature(&fire_then_coin, AI)
    );
}

#[test]
fn r4808000_an_echo_in_the_graveyard_signs_the_last_spell_and_none_in_reach_signs_it_not() {
    let (one, other) = two_orders(&spells_board(json!(VANILLA), json!([ECHO])));
    assert_ne!(search_signature(&one, AI), search_signature(&other, AI));
    let (one, other) = two_orders(&spells_board(json!(VANILLA), json!([])));
    assert_eq!(search_signature(&one, AI), search_signature(&other, AI));
}

#[test]
fn r4808000_the_lethal_solver_finds_coin_wildfire_echo() {
    let root = spells_board(json!(ECHO), json!([]));
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let lethal = find_lethal(
        std::slice::from_ref(&root),
        AI,
        &counter,
        AI_BUDGET.nodes,
        Prices::NONE,
    );
    assert!(
        lethal.is_some(),
        "no lethal found after {} of {} nodes",
        counter.used(),
        AI_BUDGET.nodes
    );
}

#[test]
fn r4808000_decide_plays_the_coin_first_when_only_that_order_wins() {
    let root = spells_board(json!(ECHO), json!([]));
    let mut options = AiOptions::with_budget(create_rng("r4808000", 0), AI_BUDGET);
    let decision = decide(&root, AI, &mut options).expect("a decision");
    assert_eq!(
        action_key(&decision.action),
        action_key(&plays(&root, COIN)[0]),
        "{:?}: {:?}",
        decision.reason,
        keys(&decision.line)
    );
}

#[test]
fn r4808000_the_two_modes_of_a_churnwalker_sign_apart() {
    register_cards();
    let root = scenario(json!({
        "seed": "r4808000-chain",
        "p1": { "hand": [CHURNWALKER], "field": [{ "def": VANILLA, "lane": 1 }, { "def": SEVEN, "lane": 2 }],
            "library": LIBRARY, "mana": 10 },
        "p2": { "hand": [VANILLA], "library": LIBRARY,
            "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }] },
    }))
    .state()
    .clone();
    let all = plays(&root, CHURNWALKER);
    let mode_of = |action: &ActionBody| match action {
        ActionBody::Play {
            modes: Some(modes), ..
        } => modes.first().cloned(),
        _ => None,
    };
    let enemy = all
        .iter()
        .find(|action| mode_of(action).as_deref() == Some("enemy"))
        .expect("enemy")
        .clone();
    let mut friendly = enemy.clone();
    if let ActionBody::Play { modes, .. } = &mut friendly {
        *modes = Some(vec!["friendly".to_string()]);
    }
    assert!(all.contains(&friendly));
    let (a, b) = (step(&root, &enemy, 1), step(&root, &friendly, 2));
    let attack = legal_actions(&a, AI)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Attack { target_id, .. } if target_id != "hero-p2"))
        .expect("an attack on a unit");
    assert_ne!(
        evaluate(&step(&a, &attack, 3), AI, NextSwing::Enemy, &AI_EVAL),
        evaluate(&step(&b, &attack, 4), AI, NextSwing::Enemy, &AI_EVAL),
        "the two modes play differently"
    );
    assert_ne!(search_signature(&a, AI), search_signature(&b, AI));
    assert_ne!(lane_free_signature(&a, AI), lane_free_signature(&b, AI));
}

#[test]
fn r4808000_a_dormant_units_two_lanes_are_lane_variants() {
    register_cards();
    let root = scenario(json!({
        "seed": "r4808000-dormant",
        "p1": { "hand": [SHIP_SMITH, VANILLA], "library": LIBRARY, "mana": 10 },
        "p2": { "hand": [VANILLA], "library": LIBRARY },
    }))
    .state()
    .clone();
    let (a, b) = (
        step(&root, &in_lane(&root, SHIP_SMITH, 1), 1),
        step(&root, &in_lane(&root, SHIP_SMITH, 3), 2),
    );
    assert_ne!(a.players[AI].units, b.players[AI].units, "two placements");
    assert_ne!(unit_lanes(&a), unit_lanes(&b));
    assert_eq!(lane_free_signature(&a, AI), lane_free_signature(&b, AI));
}

#[test]
fn r4808000_an_animated_cards_two_homes_are_lane_variants() {
    register_cards();
    let root = scenario(json!({
        "seed": "r4808000-home",
        "p1": { "hand": [WARAXE], "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 3 }],
            "library": LIBRARY, "mana": 10 },
        "p2": { "hand": [VANILLA], "library": LIBRARY },
    }))
    .state()
    .clone();
    let (a, b) = (
        step(&root, &in_lane(&root, WARAXE, 1), 1),
        step(&root, &in_lane(&root, WARAXE, 3), 2),
    );
    let waraxe = id_of(&root, WARAXE);
    let home = |state: &GameState| home_of(state, &waraxe).map(|home| home.zone.lane);
    assert_eq!(
        (home(&a), home(&b)),
        (Some(1), Some(3)),
        "animated off two backrow lanes"
    );
    let lane = |state: &GameState| {
        find_instance(state, &waraxe)
            .and_then(|card| slot_of(state, card))
            .map(|slot| slot.lane)
    };
    assert_eq!(lane(&a), lane(&b), "into one unit lane");
    assert_eq!(search_signature(&a, AI), search_signature(&b, AI));
    assert_ne!(unit_lanes(&a), unit_lanes(&b));
    assert_eq!(lane_free_signature(&a, AI), lane_free_signature(&b, AI));
}
