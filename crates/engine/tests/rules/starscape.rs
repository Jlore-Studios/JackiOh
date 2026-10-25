// The starscape map (#1847, R1847000–R1847002), Stinkyscape #148 Alteran Reach's, on a fixture Field
// Spell (`fixtures/starscape.rs`) before any card uses it: each player's one map, charted from the
// match rng at their first travel and public on both seats (R1847000); a travel is one node and its
// reward, the edge adds the named card and nothing goes past it (R1847001); a travel costs (1), the
// Radiant face's first each turn free, and `legal_actions`, the reducer and the view agree (R1847002).

use jackioh_engine::subsystems::starscape::{
    STARSCAPE_FREE_TRAVEL_ABILITY, STARSCAPE_TRAVEL_ABILITY, starscape_amount,
};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::starscape::{EDGE_ID, REACH_ID, register_starscape_fixtures};

/// `p1`'s turn 9, its main phase, a Reach in p1's backrow from lane 1 on for each of `faces` (its
/// Radiant face where true); plain cards to draw, 4 mana, and p2's hero at 100 so no reward ends the
/// game. Neither side's turn ends by itself (R82).
fn board_with(seed: &str, faces: &[bool]) -> Scenario {
    register_starscape_fixtures();
    let reaches: Vec<Value> = faces
        .iter()
        .map(|radiant| json!({ "def": REACH_ID, "radiant": radiant }))
        .collect();
    let mut s = scenario(json!({
        "seed": seed,
        "p1": { "backrow": reaches, "library": vec![plain.id.clone(); 20], "mana": 4 },
        "p2": { "library": vec![plain.id.clone(); 4], "health": 100 },
    }));
    s.state_mut().players.p1.auto_end_turn = Some(false);
    s.state_mut().players.p2.auto_end_turn = Some(false);
    s
}

fn board(seed: &str, radiant: bool) -> Scenario {
    board_with(seed, &[radiant])
}

/// 19 Heals and the edge: a map whose every node but the last gives the same thing.
fn heal_path() -> Vec<StarscapeReward> {
    let mut nodes = vec![StarscapeReward::Heal; (STARSCAPE_EDGE_TRAVELS - 1) as usize];
    nodes.push(StarscapeReward::Edge);
    nodes
}

/// Writes p1's map as charted, standing on `at`, last travelled on no turn.
fn set_map(s: &mut Scenario, nodes: Vec<StarscapeReward>, at: i32) {
    s.state_mut().players.p1.starscape = Some(StarscapeMap {
        nodes,
        at,
        travelled_on: 0,
    });
}

fn travel(s: &mut Scenario) -> &mut Scenario {
    s.activate(REACH_ID, json!({}))
}

fn at(s: &Scenario, player: PlayerId) -> i32 {
    s.state().players[player]
        .starscape
        .as_ref()
        .map_or(0, |map| map.at)
}

fn mana(s: &Scenario) -> i32 {
    s.state().players.p1.mana.current
}

/// The nodes the given events travelled to, in order.
fn travelled(events: &[GameEvent]) -> Vec<(PlayerId, i32, StarscapeReward)> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::StarscapeTravelled { player, node, reward } => Some((*player, *node, *reward)),
            _ => None,
        })
        .collect()
}

/// p1's own view of the Reach in backrow lane 1: its abilities as (ability, usable).
fn activations(s: &Scenario) -> Vec<(String, bool)> {
    activations_in(s, 1)
}

/// p1's own view of the Reach in backrow `lane`: its abilities as (ability, usable).
fn activations_in(s: &Scenario, lane: usize) -> Vec<(String, bool)> {
    let view = serde_json::to_value(s.view(P1)).expect("the view serialises");
    view["you"]["backrow"][lane - 1]["activations"]
        .as_array()
        .expect("the Reach lists its abilities")
        .iter()
        .map(|ability| {
            (
                ability["ability"].as_str().unwrap_or_default().to_string(),
                ability["usable"].as_bool().unwrap_or(false),
            )
        })
        .collect()
}

/// The abilities `legal_actions` offers p1 on the Reach.
fn legal_abilities(s: &Scenario) -> Vec<String> {
    let reach = s.backrow(P1, 1).expect("the Reach is in lane 1").id;
    legal_actions(s.state(), P1)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Activate {
                instance_id, ability, ..
            } if instance_id == reach => Some(ability.unwrap_or_default()),
            _ => None,
        })
        .collect()
}

fn in_hand(s: &Scenario, def_id: &str) -> bool {
    s.hand(P1).iter().any(|card| card.def_id == def_id)
}

#[test]
fn r1847000_the_first_travel_charts_the_map_from_the_match_rng() {
    let mut s = board("starscape-chart", false);
    assert_eq!(
        s.state().players.p1.starscape,
        None,
        "no map before the first travel"
    );
    let cursor = s.state().rng_cursor;
    travel(&mut s);
    let map = s
        .state()
        .players
        .p1
        .starscape
        .clone()
        .expect("the travel charts a map");
    assert_eq!(map.nodes.len() as i32, STARSCAPE_EDGE_TRAVELS);
    assert_eq!(
        map.nodes.last(),
        Some(&StarscapeReward::Edge),
        "the edge is the last node"
    );
    for node in &map.nodes[..map.nodes.len() - 1] {
        assert!(
            STARSCAPE_REWARDS.iter().any(|(reward, _)| reward == node),
            "{node:?} is a node reward"
        );
    }
    assert_eq!(map.at, 1);
    assert!(
        s.state().rng_cursor > cursor,
        "the chart draws from the match rng"
    );
    assert_eq!(travelled(s.last_events()), vec![(P1, 1, map.nodes[0])]);

    let mut again = board("starscape-chart", false);
    travel(&mut again);
    assert_eq!(
        again.state().players.p1.starscape.as_ref().map(|map| &map.nodes),
        Some(&map.nodes),
        "the same seed charts the same map"
    );
}

#[test]
fn r1847000_the_map_is_the_players_whichever_card_travels_it() {
    let mut s = board_with("starscape-owner", &[false, false]);
    let first = s.backrow(P1, 1).expect("a Reach in lane 1");
    let second = s.backrow(P1, 2).expect("a Reach in lane 2");
    s.activate(&first, json!({}));
    s.activate(&second, json!({}));
    assert_eq!(at(&s, P1), 2, "both Reaches moved p1's one map");
    assert_eq!(s.state().players.p2.starscape, None, "p2 has no map");
}

#[test]
fn r1847000_the_map_is_public_on_both_seats_and_absent_before_a_travel() {
    // D14: a game that never travels serializes, hashes and shows as before.
    let mut s = board("starscape-view", false);
    let raw = serde_json::to_value(s.state()).expect("the state serialises");
    assert_eq!(raw["players"]["p1"].get("starscape"), None);
    assert_eq!(raw["players"]["p2"].get("starscape"), None);
    for viewer in [P1, P2] {
        let view = serde_json::to_value(s.view(viewer)).expect("the view serialises");
        assert_eq!(view["you"].get("starscape"), None);
        assert_eq!(view["opponent"].get("starscape"), None);
    }

    travel(&mut s);
    let own = s.view(P1).you.starscape.expect("p1 sees their map");
    assert_eq!(
        s.view(P2).opponent.starscape,
        Some(own.clone()),
        "p2 sees the same map"
    );
    assert_eq!(own.at, 1);
    let map = s.state().players.p1.starscape.clone().expect("a map");
    assert_eq!(
        own.nodes.iter().map(|node| node.reward).collect::<Vec<_>>(),
        map.nodes
    );
    for node in &own.nodes {
        assert_eq!(node.amount, starscape_amount(node.reward));
    }
    assert_eq!(
        own.nodes.last().and_then(|node| node.amount),
        None,
        "the edge has no amount"
    );
}

#[test]
fn r1847001_each_reward_does_what_its_node_says() {
    let mut s = board("starscape-rewards", false);
    let mut nodes = vec![
        StarscapeReward::Heal,
        StarscapeReward::Draw,
        StarscapeReward::Mana,
        StarscapeReward::Damage,
    ];
    nodes.extend(vec![StarscapeReward::Heal; (STARSCAPE_EDGE_TRAVELS - 5) as usize]);
    nodes.push(StarscapeReward::Edge);
    set_map(&mut s, nodes, 0);

    let health = s.state().players.p1.hero.health;
    travel(&mut s);
    assert_eq!(
        s.state().players.p1.hero.health,
        health + starscape_amount(StarscapeReward::Heal).unwrap_or(0),
        "Heal heals p1's hero"
    );

    let hand = s.hand(P1).len();
    travel(&mut s);
    assert_eq!(
        s.hand(P1).len() as i32,
        hand as i32 + starscape_amount(StarscapeReward::Draw).unwrap_or(0),
        "Draw draws"
    );

    let before = mana(&s);
    travel(&mut s);
    assert_eq!(
        mana(&s),
        before - STARSCAPE_TRAVEL_MANA + starscape_amount(StarscapeReward::Mana).unwrap_or(0),
        "Mana gives back mana"
    );

    let enemy = s.state().players.p2.hero.health;
    travel(&mut s);
    assert_eq!(
        s.state().players.p2.hero.health,
        enemy - starscape_amount(StarscapeReward::Damage).unwrap_or(0),
        "Damage hits the enemy hero"
    );
    assert_eq!(
        travelled(s.events()),
        vec![
            (P1, 1, StarscapeReward::Heal),
            (P1, 2, StarscapeReward::Draw),
            (P1, 3, StarscapeReward::Mana),
            (P1, 4, StarscapeReward::Damage),
        ]
    );
}

#[test]
fn r1847001_twenty_travels_reach_the_edge_and_no_travel_goes_past_it() {
    let mut s = board("starscape-edge", false);
    s.state_mut().players.p1.mana.current = STARSCAPE_EDGE_TRAVELS * STARSCAPE_TRAVEL_MANA * 2;
    for _ in 0..STARSCAPE_EDGE_TRAVELS {
        travel(&mut s);
    }
    let nodes: Vec<i32> = travelled(s.events())
        .into_iter()
        .map(|(_, node, _)| node)
        .collect();
    assert_eq!(nodes, (1..=STARSCAPE_EDGE_TRAVELS).collect::<Vec<_>>());
    assert_eq!(at(&s, P1), STARSCAPE_EDGE_TRAVELS);
    assert!(in_hand(&s, EDGE_ID), "the edge adds its card");

    s.expect_refused_with(travel, "can't be activated now");
    assert_eq!(legal_abilities(&s), Vec::<String>::new(), "no travel is offered");
    assert_eq!(
        activations(&s),
        vec![(STARSCAPE_TRAVEL_ABILITY.to_string(), false)],
        "the ability stays listed, unusable"
    );
}

#[test]
fn r1847001_the_edge_adds_the_named_card_and_a_full_hand_burns_it() {
    let mut s = board("starscape-edge-card", false);
    set_map(&mut s, heal_path(), STARSCAPE_EDGE_TRAVELS - 1);
    travel(&mut s);
    assert!(in_hand(&s, EDGE_ID), "the edge's card is in the hand");

    register_starscape_fixtures();
    let mut full = scenario(json!({
        "seed": "starscape-edge-burn",
        "p1": {
            "backrow": [REACH_ID],
            "hand": vec![plain.id.clone(); HAND_CAP as usize],
            "library": vec![plain.id.clone(); 4],
        },
        "p2": { "library": vec![plain.id.clone(); 4] },
    }));
    full.state_mut().players.p1.auto_end_turn = Some(false);
    set_map(&mut full, heal_path(), STARSCAPE_EDGE_TRAVELS - 1);
    travel(&mut full);
    assert!(!in_hand(&full, EDGE_ID), "a full hand has no room for it");
    assert!(
        full.last_events()
            .iter()
            .any(|event| matches!(event, GameEvent::Burned { def_id, .. } if def_id == EDGE_ID)),
        "it burns (§2.4)"
    );
    assert_eq!(
        at(&full, P1),
        STARSCAPE_EDGE_TRAVELS,
        "the edge is reached all the same"
    );
}

#[test]
fn r1847002_the_base_face_costs_1_mana_a_travel() {
    let mut s = board("starscape-price", false);
    set_map(&mut s, heal_path(), 0);
    s.state_mut().players.p1.mana.current = 2 * STARSCAPE_TRAVEL_MANA;
    travel(&mut s);
    travel(&mut s);
    assert_eq!(mana(&s), 0);
    assert_eq!(at(&s, P1), 2);
    s.expect_refused_with(travel, "more than your mana");
}

#[test]
fn r1847002_the_radiant_face_travels_free_first_each_turn() {
    let mut s = board("starscape-free", true);
    set_map(&mut s, heal_path(), 0);
    s.state_mut().players.p1.mana.current = STARSCAPE_TRAVEL_MANA;
    travel(&mut s);
    assert_eq!(
        mana(&s),
        STARSCAPE_TRAVEL_MANA,
        "the first travel this turn is free"
    );
    travel(&mut s);
    assert_eq!(mana(&s), 0, "the second costs its mana");
    s.expect_refused_with(travel, "more than your mana");

    s.end_turn().end_turn();
    assert_eq!(s.state().active, P1);
    s.state_mut().players.p1.mana.current = 0;
    travel(&mut s);
    assert_eq!(at(&s, P1), 3, "a new turn's first travel is free again");
}

#[test]
fn r1847002_legal_actions_the_view_and_the_reducer_agree() {
    let mut s = board("starscape-agree", true);
    set_map(&mut s, heal_path(), STARSCAPE_EDGE_TRAVELS - 2);

    assert_eq!(
        activations(&s),
        vec![(STARSCAPE_FREE_TRAVEL_ABILITY.to_string(), true)]
    );
    assert_eq!(
        legal_abilities(&s),
        vec![STARSCAPE_FREE_TRAVEL_ABILITY.to_string()]
    );

    travel(&mut s);
    assert_eq!(
        activations(&s),
        vec![(STARSCAPE_TRAVEL_ABILITY.to_string(), true)]
    );
    assert_eq!(legal_abilities(&s), vec![STARSCAPE_TRAVEL_ABILITY.to_string()]);

    travel(&mut s);
    assert_eq!(at(&s, P1), STARSCAPE_EDGE_TRAVELS);
    assert_eq!(
        activations(&s),
        vec![(STARSCAPE_TRAVEL_ABILITY.to_string(), false)]
    );
    assert_eq!(legal_abilities(&s), Vec::<String>::new());
    s.expect_refused_with(travel, "can't be activated now");
}

#[test]
fn r1847002_the_free_travel_is_the_players_first_by_any_card() {
    let mut s = board_with("starscape-first", &[false, true]);
    set_map(&mut s, heal_path(), 0);
    s.state_mut().players.p1.mana.current = STARSCAPE_TRAVEL_MANA;
    assert_eq!(
        activations_in(&s, 2),
        vec![(STARSCAPE_FREE_TRAVEL_ABILITY.to_string(), true)],
        "the Radiant Reach offers the free travel before any travel this turn"
    );
    let base = s.backrow(P1, 1).expect("the base Reach in lane 1");
    let radiant = s.backrow(P1, 2).expect("the Radiant Reach in lane 2");
    s.activate(&base, json!({}));
    assert_eq!(mana(&s), 0, "the base face's travel costs its mana");
    assert_eq!(
        activations_in(&s, 2),
        vec![(STARSCAPE_TRAVEL_ABILITY.to_string(), false)],
        "the player has travelled this turn, so the Radiant Reach offers only the paid travel"
    );
    s.expect_refused_with(|s| s.activate(&radiant, json!({})), "more than your mana");
    assert_eq!(at(&s, P1), 1);
}
