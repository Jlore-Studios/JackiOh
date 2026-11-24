//! Audit of the lethal solver and the static combat walk (`lethal.rs`, `evaluate.rs`'s Taunt walk).
//! Each test asserts the documented behaviour and FAILS on the code as it stands.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

const AI: PlayerId = PlayerId::P1;
const HUMAN: PlayerId = PlayerId::P2;

const DUELIST: &str = "core-045"; // Deft Duelist, 4/3 Charge, Deft
const DFENDER: &str = "core-001"; // Big D-fender, 0/7: declares no attack (R7)
const MENACE: &str = "core-019"; // Midrange Menace, 9/9 Taunt
const GAUNTLET: &str = "stinkyscape-157"; // The Gauntlet (R1849005)
const ZEPHRYS: &str = "core-097"; // Zephrys: one of AI_SEARCH.expensive_def_ids (100 extra nodes a play)

fn won_by(state: &GameState, seat: PlayerId) -> bool {
    state
        .result
        .as_ref()
        .is_some_and(|result| result.winner.player() == Some(seat))
}

/// One action through `reduce`; `nonce` is fresh at each call (the engine answers a repeated one unchanged).
fn step(state: &GameState, seat: PlayerId, action: ActionBody, nonce: &str) -> GameState {
    let stepped = reduce(state, &Action::new(action, seat, nonce));
    assert!(stepped.error.is_none(), "{:?}", stepped.error);
    stepped.state
}

/// Whether the AI wins by playing the Deft Duelist into unit lane `lane` and attacking the hero with it.
fn duelist_wins_from(state: &GameState, lane: i32) -> bool {
    let card = state.players[AI].hand.iter().find(|card| card.def_id == DUELIST).expect("in hand");
    let play = legal_actions(state, AI)
        .into_iter()
        .find(|action| {
            matches!(action, ActionBody::Play { instance_id, zone: Some(zone), face_down: None, .. }
                if *instance_id == card.id && zone.lane == lane && zone.row == Row::Units)
        })
        .expect("the play is legal");
    let played = step(state, AI, play, "audit-play");
    let hit = reduce(
        &played,
        &Action::new(
            ActionBody::Attack {
                attacker_id: card.id.clone(),
                target_id: "hero-p2".into(),
            },
            AI,
            "audit-hit",
        ),
    );
    hit.error.is_none() && won_by(&hit.state, AI)
}

fn solver_line(state: &GameState) -> Option<Vec<ActionBody>> {
    let counter = create_node_counter(AI_BUDGET.lethal_nodes, None);
    find_lethal(
        std::slice::from_ref(state),
        AI,
        &counter,
        AI_BUDGET.lethal_nodes,
        Prices::NONE,
    )
}

/// The AI holds a Deft Duelist (4/3 Charge); the enemy hero is at 4. The board is slashed at lane 3
/// (R3554003): the enemy's 9/9 Taunt stands in lane 2, on the Left half, so a Duelist played into lane 2
/// is walled and one played into lane 4, on the Right half, hits the hero and wins.
fn slashed_board() -> GameState {
    jackioh_cards::register_all();
    let mut s = scenario(json!({
        "seed": "audit-lanes", "active": AI, "turn": 9,
        "p1": { "hand": [DUELIST], "field": [{ "def": DFENDER, "lane": 1 }], "library": ["core-008", "core-008"] },
        "p2": { "field": [{ "def": MENACE, "lane": 2 }], "health": 4, "library": ["core-008", "core-008"] },
    }));
    s.state_mut().slashed_lane = Some(3);
    s.state().clone()
}

/// The same lethal under The Gauntlet (R1849005): lanes 1 to 3 hold the AI's 0/7s, so the Duelist goes into
/// lane 4, inside The Gauntlet, where it may attack no hero, or lane 5, outside it, where it wins.
fn gauntlet_board() -> GameState {
    jackioh_cards::register_all();
    scenario(json!({
        "seed": "audit-lanes", "active": AI, "turn": 9,
        "p1": { "hand": [DUELIST], "backrow": [GAUNTLET],
                "field": [{ "def": DFENDER, "lane": 1 }, { "def": DFENDER, "lane": 2 }, { "def": DFENDER, "lane": 3 }],
                "library": ["core-008", "core-008"] },
        "p2": { "health": 4, "library": ["core-008", "core-008"] },
    }))
    .state()
    .clone()
}

/// Finding 1. SPEC §9.9: the solver is "a bounded search for a line that wins this turn". `judge`
/// (lethal.rs:232) keys its visited set on `search_signature`, which leaves out the units' lanes
/// (simulate.rs:1273 `unit_lanes`: "what `search_signature` leaves out"), so the second lane
/// `candidate_actions` offers for a play is "a position already seen" and is never searched. Here the two
/// lanes differ in whether the unit may hit the hero, and the lethal is in the second. The solver spends 3
/// of its 300 nodes and answers "no lethal"; `lethal_hint` (R2102000: lit "while the human has a sure win
/// this turn") stays dark with nothing hidden on the board. `unstoppable.rs`'s `position_key` already
/// pairs the signature with `unit_lanes`.
#[test]
fn the_solver_finds_a_lethal_that_stands_in_the_second_lane_a_play_is_tried_in() {
    for (name, state, bad, good) in [
        ("slash", slashed_board(), 2, 4),
        ("gauntlet", gauntlet_board(), 4, 5),
    ] {
        // The engine: two actions win from the second lane, and none does from the first.
        assert!(duelist_wins_from(&state, good), "{name}");
        assert!(!duelist_wins_from(&state, bad), "{name}");
        // Both lanes are among the solver's moves, the losing one first.
        let lanes: Vec<i32> = candidate_actions(&state, AI, Prices::NONE)
            .iter()
            .filter_map(|action| match action {
                ActionBody::Play { zone: Some(zone), .. } => Some(zone.lane),
                _ => None,
            })
            .collect();
        assert_eq!(lanes, vec![bad, good], "{name}");
        assert!(solver_line(&state).is_some(), "{name}: find_lethal misses the lethal in lane {good}");
        assert!(lethal_hint(&state, AI, None), "{name}: the hint is dark on a sure two-action win");
    }
}

/// Finding 2. The Taunt walk counts a Cleave splash's Trample excess only on a neighbour that is itself a
/// Taunt the walk has still to reach (evaluate.rs:1954 `if j <= i || !has_keyword(.., Taunt) { continue }`).
/// §4.4 step 9 sends "its hit on the defender or its Cleave's (step 10)" beyond the struck unit's health to
/// its controller's hero, whatever that neighbour's keywords. A 5/4 Cleave, Trample attacker walled by a 0/9
/// Taunt with a 1/1 on each side deals the hero 4 + 4 = 8 in one attack; `face_threat` reads 0, so at 8 life
/// the evaluation sees no lethal threat (README: `face_threat` is "the face damage `attacker`'s board could
/// deal the other hero next turn").
#[test]
fn the_walk_counts_a_cleave_splashs_trample_excess_over_a_neighbour_without_taunt() {
    jackioh_cards::register_all();
    let mut s = scenario(json!({
        "seed": "audit-splash", "active": AI, "turn": 9,
        "p1": { "health": 8, "library": ["core-008", "core-008"], "field": [
            { "def": "stand-in (1) 1/1", "lane": 2 },
            { "def": "stand-in (1) 0/9 Taunt", "lane": 3 },
            { "def": "stand-in (1) 1/1", "lane": 4 },
        ] },
        "p2": { "library": ["core-008", "core-008"],
                "field": [{ "def": "stand-in (1) 5/4 Cleave, Trample", "lane": 3 }] },
    }));
    for player in PLAYER_IDS {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    let state = s.state().clone();

    // The engine: the AI passes, the one attack on the Taunt kills the AI's hero through both splashes.
    let passed = step(&state, AI, ActionBody::EndTurn, "audit-pass");
    let attacker = active_units_of(&passed, HUMAN)[0].id.clone();
    let taunt = active_units_of(&passed, AI)
        .into_iter()
        .find(|unit| unit_has(&passed, unit, KeywordKind::Taunt))
        .expect("the Taunt")
        .id
        .clone();
    let hit = step(
        &passed,
        HUMAN,
        ActionBody::Attack {
            attacker_id: attacker,
            target_id: taunt,
        },
        "audit-attack",
    );
    assert!(won_by(&hit, HUMAN), "the attack is lethal in the engine");

    // The static reads of the same attack: next turn's threat as the AI sees it now, and the gap the
    // attacker's own solver reads once it is on turn.
    assert_eq!(face_threat(&state, HUMAN), 8);
    assert!(ready_gap(&passed, HUMAN) <= 0.0, "ready_gap {}", ready_gap(&passed, HUMAN));
}

/// Finding 3. `find_lethal` is two walks: depth-first on `AI_SEARCH.lethal_quick_nodes` (40), then best-first
/// "on the rest of the allowance" (lethal.rs header). A step whose surcharge the quick slice cannot pay is
/// refused as `Some(Err)` (simulate.rs:772: "refuse the step like any refused action"), the depth-first
/// walk `continue`s past it (lethal.rs:259), runs out of moves and returns `Walk::Exhausted`, which
/// `find_lethal_with_quick_nodes` reads as "the tree was searched to the end" and returns `None`
/// (lethal.rs:386) with 299 of its 300 nodes unspent. The best-first walk, whose slice pays the surcharge,
/// never runs. Zephrys (100 extra nodes) with the enemy hero at 3 is a lethal the solver proves in 107
/// nodes when its depth-first stage is given none.
#[test]
fn a_step_the_quick_slice_cannot_pay_for_leaves_the_rest_of_the_allowance_to_the_second_walk() {
    jackioh_cards::register_all();
    let state = scenario(json!({
        "seed": "audit-zephrys", "active": AI, "turn": 9,
        "p1": { "hand": [ZEPHRYS], "library": ["core-008", "core-008"] },
        "p2": { "health": 3, "library": ["core-008", "core-008"] },
    }))
    .state()
    .clone();
    let limit = AI_BUDGET.lethal_nodes;

    // With no depth-first stage the lethal is proved well inside the allowance.
    let counter = create_node_counter(limit, None);
    let second_walk =
        find_lethal_with_quick_nodes(std::slice::from_ref(&state), AI, &counter, limit, 0, Prices::NONE);
    assert!(second_walk.is_some(), "the best-first walk proves it");
    assert!(counter.used() < limit);

    // The solver as `decide`, `lethal_hint` and `unstoppable_lethal` call it.
    let counter = create_node_counter(limit, None);
    let found = find_lethal(std::slice::from_ref(&state), AI, &counter, limit, Prices::NONE);
    assert!(
        found.is_some(),
        "find_lethal gave up after {} of {limit} nodes",
        counter.used()
    );
}
