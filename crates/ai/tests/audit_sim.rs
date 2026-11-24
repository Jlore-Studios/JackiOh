//! Audit of move generation and simulation (`candidates.rs`, `simulate.rs`): each test asserts the
//! documented behaviour and fails on the code as it stands.
//!
//! All seven are one root cause: `search_signature` (simulate.rs `position_signature`, `card_signature`,
//! `unit_play_signature`) and `unit_lanes` leave out state that changes what a position offers or is
//! worth, so the beam's duplicate check (search.rs `seen`) and the lethal solver's visited set
//! (lethal.rs `judge`) take two different positions for one and drop the later line.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

const AI: PlayerId = PlayerId::P1;

const WILDFIRE: &str = "classic-055";
const ECHO: &str = "classic-057";
const COIN: &str = "core-t-coin";
const GIGA_IFY: &str = "stinkyscapeplus-023";
const CHURNWALKER: &str = "stinkyscapeplus-007";
const SHIP_SMITH: &str = "stinkyscape-017";
const TRAP: &str = "core-018";
const SEVEN: &str = "core-025";
const VANILLA: &str = "core-008";

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

fn aims_at(action: &ActionBody, target: &str) -> bool {
    match action {
        ActionBody::Play { targets: Some(targets), .. } => targets.iter().any(|picked| match picked {
            Selection::Instance { instance_id } => instance_id == target,
            Selection::Hero { player } => format!("hero-{}", player.as_str()) == target,
            _ => false,
        }),
        _ => false,
    }
}

fn step(state: &GameState, action: &ActionBody, n: usize) -> GameState {
    let next = reduce(state, &Action::new(action.clone(), AI, format!("audit:{n}")));
    assert!(next.error.is_none(), "{:?}", next.error);
    next.state
}

fn keys(actions: &[ActionBody]) -> Vec<String> {
    actions.iter().map(action_key).collect()
}

/// p1 holds C #55 Book of Wildfire ((1): deal 4 to a declared target), C #57 Echo ((1): "has the text of
/// the last Spell either player played") and The Coin ((0): gain 1 mana this turn), with 1 mana, against
/// a hero at 8: Coin, Wildfire at the hero, Echo (now Wildfire) at the hero wins.
fn echo_board() -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "audit-sim-echo",
        "p1": { "hand": [WILDFIRE, ECHO, COIN], "library": [VANILLA, VANILLA, VANILLA], "mana": 1 },
        "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA], "health": 8 },
    }))
    .state()
    .clone()
}

fn at_enemy_hero(state: &GameState, def_id: &str) -> ActionBody {
    plays(state, def_id)
        .into_iter()
        .find(|action| aims_at(action, "hero-p2"))
        .unwrap_or_else(|| panic!("{def_id} cannot aim at the enemy hero"))
}

/// README, move order: "The beam drops an open line whose position an earlier one reached (two orders of
/// the same moves into the same lanes are one position)". Coin then Wildfire, and Wildfire then Coin,
/// leave Echo with two different texts (`subsystems::copied_text_of` reads `GameState.last_spell`), so
/// they offer different moves and are two positions, as R3951100 rules for the last Feng Shui element.
#[test]
fn two_orders_of_two_spells_that_leave_echo_two_texts_sign_apart() {
    let root = echo_board();
    let coin_first = step(&root, &plays(&root, COIN)[0], 1);
    let coin_then_fire = step(&coin_first, &at_enemy_hero(&coin_first, WILDFIRE), 2);
    let fire_first = step(&root, &at_enemy_hero(&root, WILDFIRE), 3);
    let fire_then_coin = step(&fire_first, &plays(&fire_first, COIN)[0], 4);
    assert_ne!(
        keys(&legal_actions(&coin_then_fire, AI)),
        keys(&legal_actions(&fire_then_coin, AI)),
        "Echo is Wildfire after one order and The Coin after the other"
    );
    assert_ne!(
        search_signature(&coin_then_fire, AI),
        search_signature(&fire_then_coin, AI),
        "two positions that offer different moves sign alike"
    );
}

/// README: "a lethal the solver proves is played". The line Coin, Wildfire, Echo is a lethal inside the
/// solver's depth and budget (19 nodes are spent), but its walk reaches Wildfire-then-Coin first and
/// `judge` (lethal.rs) then takes Coin-then-Wildfire for a position already visited.
#[test]
fn the_lethal_solver_finds_coin_wildfire_echo() {
    let root = echo_board();
    // The line wins, on the true state.
    let a = step(&root, &plays(&root, COIN)[0], 1);
    let b = step(&a, &at_enemy_hero(&a, WILDFIRE), 2);
    let c = step(&b, &at_enemy_hero(&b, ECHO), 3);
    assert_eq!(c.result.as_ref().and_then(|result| result.winner.player()), Some(AI));

    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let lethal = find_lethal(std::slice::from_ref(&root), AI, &counter, AI_BUDGET.nodes, Prices::NONE);
    assert!(
        lethal.is_some(),
        "no lethal found after {} of {} nodes",
        counter.used(),
        AI_BUDGET.nodes
    );
}

/// The same board through `decide`: it plays Wildfire first, after which Echo can only copy The Coin and
/// the win is gone for the turn.
#[test]
fn decide_plays_the_coin_first_when_only_that_order_wins() {
    let root = echo_board();
    let mut options = AiOptions::with_budget(create_rng("audit-sim", 0), AI_BUDGET);
    let decision = decide(&root, AI, &mut options).expect("a decision");
    assert_eq!(
        action_key(&decision.action),
        action_key(&plays(&root, COIN)[0]),
        "{:?}: {:?}",
        decision.reason,
        keys(&decision.line)
    );
}

/// S+ #23 GIGA-ify: "Make a permanent in your hand GIGA and Indestructible." `card_signature` signs a hand
/// card without its `granted_keywords` (and `buffs`: S+ #100 Plesiosize), which `evaluate` reads
/// (evaluate.rs `face_uplift`: `stats_with_buffs`, `card_keywords`). Two positions that score differently
/// are not one position.
#[test]
fn hand_cards_that_differ_in_what_an_effect_gave_them_sign_apart() {
    jackioh_cards::register_all();
    let root = scenario(json!({
        "seed": "audit-sim",
        "p1": { "hand": [GIGA_IFY, TRAP, SEVEN], "library": [VANILLA, VANILLA, VANILLA], "mana": 10 },
        "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA] },
    }))
    .state()
    .clone();
    let all = plays(&root, GIGA_IFY);
    let on = |def_id: &str| {
        let target = id_of(&root, def_id);
        all.iter().find(|action| aims_at(action, &target)).expect("a target").clone()
    };
    let (trap, unit) = (step(&root, &on(TRAP), 1), step(&root, &on(SEVEN), 2));
    assert_ne!(
        evaluate(&trap, AI, NextSwing::Enemy, &AI_EVAL),
        evaluate(&unit, AI, NextSwing::Enemy, &AI_EVAL),
        "the evaluation tells the two apart"
    );
    assert_ne!(search_signature(&trap, AI), search_signature(&unit, AI));
}

/// The consequence in the beam: with 4-mana 7/7 (cost 4) too dear this turn, the only way to GIGA-ify
/// Mr. Vanilla (cost 1) and play it is the root play aimed at it, which the beam drops as a duplicate of
/// the play aimed at the 7/7, the first target in hand order.
#[test]
fn the_beam_tries_giga_ify_on_the_second_card_in_hand() {
    jackioh_cards::register_all();
    let probe = scenario(json!({ "p1": { "hand": [GIGA_IFY, SEVEN, VANILLA] }, "p2": {} }));
    let cost = |def_id: &str| effective_cost(probe.state(), probe.card(def_id), CostOptions::default());
    assert!(cost(SEVEN) > cost(VANILLA));
    let mana = cost(GIGA_IFY) + cost(VANILLA);
    let root = scenario(json!({
        "seed": "audit-sim",
        "p1": { "hand": [GIGA_IFY, SEVEN, VANILLA], "library": [VANILLA, VANILLA, VANILLA], "mana": mana },
        "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA] },
    }))
    .state()
    .clone();
    let vanilla = id_of(&root, VANILLA);
    assert!(
        root_candidate_actions(&root, AI, Prices::NONE).iter().any(|action| aims_at(action, &vanilla)),
        "the play is a root candidate"
    );
    let counter = create_node_counter(AI_BUDGET.nodes, None);
    let lines = beam_search(&root, AI, &counter, AI_BUDGET, Prices::NONE);
    assert!(
        lines.iter().any(|line| line.actions.iter().any(|action| aims_at(action, &vanilla))),
        "no line of {} aims GIGA-ify at Mr. Vanilla ({} nodes of {})",
        lines.len(),
        counter.used(),
        AI_BUDGET.nodes
    );
}

/// S+ #7 Churnwalker: "Cry: Choose one: Chain all enemy Units together, when one takes damage all other
/// chained Units also take the damage; Chain all friendly Units together, when one takes damage it is
/// split between all allies." The unit signature leaves out `shared_chain` and `split_chain`, so the two
/// modes of one play sign alike, although the same attack from each then gives two different boards.
#[test]
fn boards_that_differ_in_which_side_is_chained_sign_apart() {
    jackioh_cards::register_all();
    let root = scenario(json!({
        "seed": "audit-sim",
        "p1": { "hand": [CHURNWALKER], "field": [{ "def": VANILLA, "lane": 1 }, { "def": SEVEN, "lane": 2 }],
            "library": [VANILLA, VANILLA, VANILLA], "mana": 10 },
        "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA],
            "field": [{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 2 }] },
    }))
    .state()
    .clone();
    let all = plays(&root, CHURNWALKER);
    let mode_of = |action: &ActionBody| match action {
        ActionBody::Play { modes: Some(modes), .. } => modes.first().cloned(),
        _ => None,
    };
    let enemy = all.iter().find(|action| mode_of(action).as_deref() == Some("enemy")).expect("enemy").clone();
    let mut friendly = enemy.clone();
    if let ActionBody::Play { modes, .. } = &mut friendly {
        *modes = Some(vec!["friendly".to_string()]);
    }
    assert!(all.contains(&friendly));
    let (a, b) = (step(&root, &enemy, 1), step(&root, &friendly, 2));
    // The same attack on an enemy Unit from each: the chained enemies all take it in one and not the other.
    let attack = legal_actions(&a, AI)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Attack { target_id, .. } if target_id != "hero-p2"))
        .expect("an attack on a unit");
    assert_ne!(
        evaluate(&step(&a, &attack, 3), AI, NextSwing::Enemy, &AI_EVAL),
        evaluate(&step(&b, &attack, 4), AI, NextSwing::Enemy, &AI_EVAL),
        "the two modes play differently"
    );
    assert_ne!(
        (search_signature(&a, AI), unit_lanes(&a)),
        (search_signature(&b, AI), unit_lanes(&b)),
        "the beam's duplicate check takes the two modes for one position"
    );
}

/// simulate.rs `unit_lanes`: "The beam's duplicate check reads the two together, so two placements of one
/// play that differ only in lane stay two positions"; README: such a line "is a lane variant" that ends
/// its turn beside the kept line. S #17 Imprisoned Ship Smith enters Dormant, which `active_units_of`
/// leaves out, so `unit_lanes` lists no lane for it and `search_signature` signs it as `z<id>:<turns>`:
/// its two placements are an exact duplicate for search.rs `seen`, dropped before the lane-variant code.
#[test]
fn a_dormant_units_lane_is_part_of_the_beams_duplicate_check() {
    jackioh_cards::register_all();
    let root = scenario(json!({
        "seed": "audit-sim",
        "p1": { "hand": [SHIP_SMITH, VANILLA], "library": [VANILLA, VANILLA, VANILLA], "mana": 10 },
        "p2": { "hand": [VANILLA], "library": [VANILLA, VANILLA, VANILLA] },
    }))
    .state()
    .clone();
    let in_lane = |lane: i32| {
        plays(&root, SHIP_SMITH)
            .into_iter()
            .find(|action| matches!(action, ActionBody::Play { zone: Some(zone), .. } if zone.lane == lane))
            .expect("a lane")
    };
    let (a, b) = (step(&root, &in_lane(1), 1), step(&root, &in_lane(3), 2));
    assert_ne!(a.players[AI].units, b.players[AI].units, "two placements");
    assert_ne!(
        (search_signature(&a, AI), unit_lanes(&a)),
        (search_signature(&b, AI), unit_lanes(&b)),
        "the beam's duplicate check takes the two lanes for one position"
    );
}
