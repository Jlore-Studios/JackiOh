//! Classic #62 Living Bomb meets the AI (issue #4154, R4154402). Its base face's Cry places a Plague Counter
//! and its turn hooks destroy, at the start of each player's turn, every permanent that player controls with
//! one (R400); its Radiant face places one with an Activate and destroys only the opponent's. The search saw
//! the destroy only once a line ended its turn, while the evaluation it ranks open lines by read a counter as
//! Fed Fauci's banked mana, a gain for the side it sits on: so the beam kept the lines that put the counter on
//! the AI's own permanents and dropped the ones that aimed it at the enemy. Now a permanent a Living Bomb will
//! destroy at its controller's next turn start counts as a Brittle 1 one (`plague_doomed`), and a placement
//! prompt under a Living Bomb tries the enemy's permanents first.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, act, on_field, register_cards};

const BOMB: &str = "classic-062";
/// Core #25: a (4) 7/7 with Armor 7, the enemy's best permanent.
const BIG: &str = "core-025";
/// Core #8 Mr. Vanilla: a (1) 4/4 with no text.
const VANILLA: &str = "core-008";
/// Core #11 Tempo Timmy: a (1) 3/3 Rush, First Strike, the AI's own board.
const TIMMY: &str = "core-011";
/// Four cards a side, so a scenario stays under 20 instances (support.rs's header).
const LIBRARY: &[&str] = &[VANILLA, TIMMY, VANILLA, TIMMY];

/// The AI's turn with `ai` (its hand, field and backrow) against the human's 7/7 and Mr. Vanilla.
fn scene(seed: &str, ai: Value) -> GameState {
    register_cards();
    let mut p1 = ai;
    p1["library"] = json!(LIBRARY);
    scenario(json!({
        "seed": seed,
        "p1": p1,
        "p2": { "hand": ["core-005"], "field": [BIG, VANILLA], "library": LIBRARY },
    }))
    .state()
    .clone()
}

/// `player`'s unit of `def_id` on the field, its id.
fn unit_id(state: &GameState, player: PlayerId, def_id: &str) -> String {
    active_units_of(state, player)
        .into_iter()
        .find(|unit| unit.def_id == def_id)
        .map(|unit| unit.id.clone())
        .expect("the unit is on the field")
}

/// `state` with one Plague Counter on the instance `id`.
fn plagued(state: &GameState, id: &str) -> GameState {
    let mut next = state.clone();
    jackioh_engine::find_instance_mut(&mut next, id)
        .expect("the instance")
        .counters
        .plague = Some(1);
    next
}

fn worth(state: &GameState, id: &str) -> f64 {
    let unit = jackioh_engine::find_instance(state, id).expect("the unit");
    unit_worth(state, unit, &AI_EVAL, Prices::NONE)
}

#[test]
fn r4154402_a_plague_counter_under_a_living_bomb_is_a_destroy_waiting() {
    let base = scene("ai-bomb-doom", json!({ "field": [TIMMY], "backrow": [BOMB] }));
    let (enemy, own) = (unit_id(&base, HUMAN, BIG), unit_id(&base, AI, TIMMY));
    for id in [&enemy, &own] {
        // The base face destroys either side's plagued permanents at that side's turn start.
        let marked = plagued(&base, id);
        let unit = jackioh_engine::find_instance(&marked, id).expect("the unit");
        assert!(plague_doomed(&marked, unit), "{id}");
        // Brittle 1's share of the body, and no banked Plague Counter.
        let expected = (1.0 - AI_EVAL.brittle_discount) * worth(&base, id);
        assert!((worth(&marked, id) - expected).abs() < 1e-9, "{id}");
    }
    // The Radiant face destroys only the opponent's.
    let radiant = scene(
        "ai-bomb-doom",
        json!({ "field": [TIMMY], "backrow": [{ "def": BOMB, "radiant": true }] }),
    );
    let (enemy, own) = (unit_id(&radiant, HUMAN, BIG), unit_id(&radiant, AI, TIMMY));
    let marked = plagued(&plagued(&radiant, &enemy), &own);
    let doomed = |id: &str| {
        plague_doomed(
            &marked,
            jackioh_engine::find_instance(&marked, id).expect("the unit"),
        )
    };
    assert!(doomed(&enemy));
    assert!(!doomed(&own));
    // With no Living Bomb a counter is Fed Fauci's banked mana, as before.
    let bare = scene("ai-bomb-doom", json!({ "field": [TIMMY] }));
    let enemy = unit_id(&bare, HUMAN, BIG);
    let marked = plagued(&bare, &enemy);
    assert!(!plague_doomed(
        &marked,
        jackioh_engine::find_instance(&marked, &enemy).expect("the unit")
    ));
    assert!((worth(&marked, &enemy) - worth(&bare, &enemy) - AI_EVAL.plague_counter).abs() < 1e-9);
}

#[test]
fn r4154402_the_placement_prompt_under_a_living_bomb_tries_the_enemys_permanents_first() {
    let state = scene(
        "ai-bomb-order",
        json!({ "field": [TIMMY, TIMMY, TIMMY], "backrow": [{ "def": BOMB, "radiant": true }] }),
    );
    let activate = legal_actions(&state, AI)
        .into_iter()
        .find(|action| matches!(action, ActionBody::Activate { .. }))
        .expect("the Radiant face's Activate");
    let asked = act(&state, AI, activate);
    let aimed_at = |action: &ActionBody| match action {
        ActionBody::Answer { selection, .. } => selection.iter().find_map(|picked| match picked {
            Selection::Instance { instance_id } => {
                jackioh_engine::find_instance(&asked, instance_id).map(|card| card.controller)
            }
            _ => None,
        }),
        _ => None,
    };
    let sides: Vec<PlayerId> = candidate_actions(&asked, AI, Prices::NONE)
        .iter()
        .filter_map(aimed_at)
        .collect();
    let first_own = sides
        .iter()
        .position(|side| *side == AI)
        .expect("an own permanent is offered");
    assert!(first_own > 0, "{sides:?}");
    assert!(sides[..first_own].iter().all(|side| *side == HUMAN), "{sides:?}");
    assert!(sides[first_own..].iter().all(|side| *side == AI), "{sides:?}");
}

#[test]
fn r4154402_the_ai_plays_living_bomb_on_the_enemys_best_unit_never_its_own() {
    let state = scene(
        "ai-bomb-play",
        json!({ "hand": [BOMB], "field": [TIMMY, TIMMY, TIMMY] }),
    );
    let own: Vec<String> = active_units_of(&state, AI)
        .iter()
        .map(|unit| unit.id.clone())
        .collect();
    let turn = play_ai_turn(&state, AI, &mut AiOptions::new(create_rng("ai-bomb-play", 0)));
    let end = &turn.state;
    // The human's turn has started: the 7/7 carried the counter, and its Living Bomb hook destroyed it.
    assert!(!on_field(end, HUMAN, BIG), "{:?}", turn.decisions);
    for id in &own {
        let unit = jackioh_engine::find_instance(end, id).expect("the AI's unit is still on the field");
        assert_eq!(unit.counters.plague.unwrap_or(0), 0, "{id}");
    }
}

#[test]
fn r4154402_the_ai_aims_the_radiant_faces_activate_at_the_enemys_best_unit() {
    let state = scene(
        "ai-bomb-activate",
        json!({ "hand": [VANILLA], "field": [TIMMY, TIMMY], "backrow": [{ "def": BOMB, "radiant": true }] }),
    );
    let turn = play_ai_turn(&state, AI, &mut AiOptions::new(create_rng("ai-bomb-activate", 0)));
    assert!(!on_field(&turn.state, HUMAN, BIG), "{:?}", turn.decisions);
    assert!(
        active_units_of(&turn.state, AI)
            .iter()
            .all(|unit| unit.counters.plague.unwrap_or(0) == 0)
    );
}
