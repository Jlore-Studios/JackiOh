//! The slash in the AI's threat (#4725). On a slashed board (R3554003) a Unit attacks only Units on its own
//! half and a hero from either, so a Taunt across the slash walls nothing. The Taunt walk
//! (`soak_past_taunts`) already reads it with no flag: it binds an attacker to a Taunt only when
//! `attack_restriction` lets it attack that Taunt, which holds the engine's `geometry::reaches`. These tests
//! pin it for `face_threat`, `ready_gap` and `wall_switches`, which the other static reads (the threat and
//! pressure terms, the Duos pool loss) go through. The board is slashed at lane 3: lanes 1 and 2 are the
//! Left half, 4 and 5 the Right. Stand-ins: Mr. Vanilla, a 4/4; Midrange Menace, a 9/9 Taunt; Core #25, a 7/7
//! with Armor 7 (no Taunt until it switches to Defense Position); the Jlockeed Large Language Model token,
//! a 2/5 GIGA Taunt.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::{AI, HUMAN, register_cards, unit};

const VANILLA: &str = "core-008";
const MENACE: &str = "core-019";
const BIG: &str = "core-025";
const GIGA_WALL: &str = "stinkyscapeplus-t-ai-04";
/// The lane the slash voids.
const SLASH_LANE: i32 = 3;
/// What the engine refuses a hero attack behind a Taunt with (§4.2 step 3).
const TAUNT_WALL: &str = "a Taunt unit must be attacked first";

/// The AI (p1) against the human (p2), each a scenario side, on turn 9 with the AI to act; slashed at
/// `SLASH_LANE` when `slashed`.
fn duel(ai: Value, human: Value, slashed: bool) -> GameState {
    register_cards();
    let mut s = scenario(json!({ "seed": "ai-slash", "active": AI, "turn": 9, "p1": ai, "p2": human }));
    if slashed {
        s.state_mut().slashed_lane = Some(SLASH_LANE);
    }
    s.state().clone()
}

/// The face damage of the human's 4/4s in `lanes` against the AI's `wall` (a def and a lane; none with
/// `None`).
fn threat(wall: Option<(&str, i32)>, lanes: &[i32], slashed: bool) -> i32 {
    let ai = match wall {
        Some((def, lane)) => json!({ "field": [{ "def": def, "lane": lane }] }),
        None => json!({}),
    };
    let attackers: Vec<Value> = lanes
        .iter()
        .map(|lane| json!({ "def": VANILLA, "lane": lane }))
        .collect();
    face_threat(&duel(ai, json!({ "field": attackers }), slashed), HUMAN)
}

#[test]
fn a_taunt_across_the_slash_walls_nothing() {
    // The Taunt on the Right and the 4/4 on the Left, and the other way round: the 4/4 reaches the hero.
    assert_eq!(threat(Some((MENACE, 4)), &[1], true), 4);
    assert_eq!(threat(Some((MENACE, 2)), &[5], true), 4);
    // The same boards unslashed: the Taunt walls it.
    assert_eq!(threat(Some((MENACE, 4)), &[1], false), 0);
    assert_eq!(threat(Some((MENACE, 2)), &[5], false), 0);
    // Across the slash it is as if the Taunt were not there.
    assert_eq!(threat(Some((MENACE, 4)), &[1], true), threat(None, &[1], true));
}

#[test]
fn a_taunt_on_the_attackers_half_still_walls() {
    assert_eq!(threat(Some((MENACE, 2)), &[1], true), 0);
    assert_eq!(threat(Some((MENACE, 4)), &[5], true), 0);
    // A Taunt walls its own half's 4/4 alone: the other half's still reaches, where unslashed both are
    // walled.
    assert_eq!(threat(Some((MENACE, 2)), &[1, 5], true), 4);
    assert_eq!(threat(Some((MENACE, 4)), &[1, 5], true), 4);
    assert_eq!(threat(Some((MENACE, 2)), &[1, 5], false), 0);
}

#[test]
fn either_half_reaches_a_hero() {
    assert_eq!(threat(None, &[1, 5], true), 8);
    assert_eq!(threat(None, &[1, 2, 4, 5], true), 16);
    for lanes in [&[1, 5][..], &[1, 2, 4, 5][..]] {
        assert_eq!(threat(None, lanes, true), threat(None, lanes, false), "{lanes:?}");
    }
}

#[test]
fn a_giga_taunt_walls_only_the_half_it_stands_on() {
    // A GIGA covers two lanes and never the void one, so it stands on one half: lanes 1-2 or 4-5.
    for (lane, walled, open) in [(1, 1, 5), (4, 5, 1)] {
        let wall = Some((GIGA_WALL, lane));
        assert_eq!(threat(wall, &[walled], true), 0, "{lane}");
        assert_eq!(threat(wall, &[open], true), 4, "{lane}");
        assert_eq!(threat(wall, &[open], false), 0, "{lane}");
    }
}

/// `ready_gap` for the AI's 4/4 in lane 1 against the human at 10 life behind a Menace in `wall` (none
/// with `None`).
fn gap(wall: Option<i32>, slashed: bool) -> f64 {
    let human = match wall {
        Some(lane) => json!({ "health": 10, "field": [{ "def": MENACE, "lane": lane }] }),
        None => json!({ "health": 10 }),
    };
    let s = duel(
        json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
        human,
        slashed,
    );
    ready_gap(&s, AI)
}

#[test]
fn the_lethal_gap_reads_the_halves_too() {
    // Control: with no wall the 4/4 takes 4 of the 10.
    assert_eq!(gap(None, true), 6.0);
    // Across the slash the Taunt leaves the gap as the control has it; on the 4/4's half, or unslashed,
    // it walls all of it.
    assert_eq!(gap(Some(4), true), 6.0);
    assert_eq!(gap(Some(2), true), 10.0);
    assert_eq!(gap(Some(4), false), 10.0);
}

#[test]
fn the_engine_walls_the_same_boards() {
    let hero = AttackTarget::Hero { player: HUMAN };
    for (lane, slashed, walled) in [(4, true, false), (2, true, true), (4, false, true)] {
        let s = duel(
            json!({ "field": [{ "def": VANILLA, "lane": 1 }] }),
            json!({ "field": [{ "def": MENACE, "lane": lane }] }),
            slashed,
        );
        let refusal = why_cannot_attack(&s, unit(&s, AI, VANILLA), &hero)
            .err()
            .map(|refused| refused.message);
        match (walled, refusal) {
            (false, None) => {}
            (true, Some(message)) => assert_eq!(message, TAUNT_WALL, "{lane} {slashed}"),
            (walled, refusal) => panic!("{lane} {slashed}: walled {walled}, refused {refusal:?}"),
        }
    }
}

/// The 7/7's switch to Defense Position, which grants the Taunt (§4.1).
fn wall(state: &GameState) -> ActionBody {
    ActionBody::SwitchPosition {
        instance_id: unit(state, AI, BIG).id.clone(),
    }
}

/// At 4 life behind a 7/7 in lane 4 against one 4/4: the switch's Taunt (Right half) walls a 4/4 in lane
/// 5 or, unslashed, in lane 1, but not a 4/4 across the slash in lane 1, where the switch lowers nothing.
#[test]
fn a_wall_switch_is_offered_only_against_attackers_on_its_half() {
    for (lane, slashed, offered) in [(5, true, true), (1, true, false), (1, false, true)] {
        let s = duel(
            json!({ "health": 4, "field": [{ "def": BIG, "lane": 4 }] }),
            json!({ "field": [{ "def": VANILLA, "lane": lane }] }),
            slashed,
        );
        assert_eq!(face_threat(&s, HUMAN), 4, "{lane} {slashed}");
        let walls = wall_switches(&s, AI, &candidate_actions(&s, AI, Prices::NONE));
        let expected = if offered { vec![wall(&s)] } else { Vec::new() };
        assert_eq!(walls, expected, "{lane} {slashed}");
    }
}
