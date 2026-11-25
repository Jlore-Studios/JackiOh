//! R4803000, R602, R150: the redaction moves a Unit's Attack only where a hidden card changed the Attack
//! it shows. A Unit below 0 Attack that no hidden card touches keeps its buffs, so no simulation and no
//! lethal hint counts damage it does not deal; a hidden aura of an older shipped catalog that floors a
//! Unit at 0 leaves it at 0 in the AI's view, not at how far below 0 it went.

use jackioh_ai::*;
use jackioh_engine::testkit::*;

use super::support::AI;

/// `state` with the permanent Attack buff of `id` (a Unit with no buff or aura yet) set so that its raw
/// Attack, before §10.4's floor at 0, is `raw`.
fn set_raw_attack(state: &mut GameState, id: &str, raw: i32) {
    let printed = unit_view(state, find_instance(state, id).expect("the unit")).attack;
    find_instance_mut(state, id).expect("the unit").buffs.attack = raw - printed;
}

/// R4803000: Tempo Timmy at raw Attack −2 (shown 0, as C+ #50 Adaptive Growth's −2/−2 leaves a small
/// Unit), touched by no hidden card, leaves the redaction with its buffs and raw Attack as they were,
/// for either seat and on a second pass
#[test]
fn r4803000_redact_leaves_a_public_unit_with_attack_below_zero_as_it_is() {
    jackioh_cards::register_all();
    let mut state = scenario(json!({
        "seed": "redact-negative-attack",
        "active": "p1",
        "turn": 9,
        "p1": { "field": ["core-011"], "hand": ["core-008"], "mana": 3, "library": ["core-020", "core-053"] },
        "p2": { "field": ["core-019"], "library": ["core-005", "core-016"] },
    }))
    .state()
    .clone();
    let id = active_units_of(&state, AI)[0].id.clone();
    set_raw_attack(&mut state, &id, -2);
    let truth = find_instance(&state, &id).expect("Tempo Timmy").clone();
    assert_eq!(unclamped_attack(&state, &truth), -2);
    assert_eq!(unit_view(&state, &truth).attack, 0);

    for seat in PLAYER_IDS {
        let seen = redact(&state, seat);
        let seen_unit = find_instance(&seen, &id).expect("the unit is public").clone();
        assert_eq!(
            seen_unit.buffs,
            truth.buffs,
            "{}: redact changed the buffs of a Unit no hidden card touches",
            seat.as_str()
        );
        assert_eq!(unclamped_attack(&seen, &seen_unit), -2, "{}", seat.as_str());
        let twice = redact(&seen, seat);
        let twice_unit = find_instance(&twice, &id).expect("the unit is public");
        assert_eq!(twice_unit.buffs, truth.buffs, "{}, second pass", seat.as_str());
    }
}

/// p1, the seat to move in its main phase, holds Tempo Timmy at raw Attack `raw` and Core #14 Jlockeed's
/// Weapons ("Your Units have +1 Attack", 4 mana) with 4 mana; p2 is on 1 Health with no Units.
fn weapons_board(raw: i32) -> GameState {
    jackioh_cards::register_all();
    let mut state = scenario(json!({
        "seed": "redact-negative-attack-lethal",
        "active": "p1",
        "turn": 9,
        "p1": { "field": ["core-011"], "hand": ["core-014"], "mana": 4, "library": ["core-020", "core-053"] },
        "p2": { "health": 1, "library": ["core-005", "core-016"] },
    }))
    .state()
    .clone();
    let id = active_units_of(&state, AI)[0].id.clone();
    set_raw_attack(&mut state, &id, raw);
    state
}

/// R4803000, R2102000: at raw 0 the Weapons' +1 makes Timmy a 1-Attack swing at a hero on 1, and the
/// hint lights; at raw −1 or −2 Timmy still shows 0 Attack after them, so it stays dark
#[test]
fn r4803000_r2102000_lethal_hint_counts_no_damage_from_a_unit_below_zero_attack() {
    assert!(
        lethal_hint(&weapons_board(0), AI, None),
        "raw 0: the Weapons make a lethal"
    );
    for raw in [-1, -2] {
        assert!(
            !lethal_hint(&weapons_board(raw), AI, None),
            "raw {raw}: the hint reports a lethal the true state does not hold"
        );
    }
}

/// R4803000, R602, R767000: on v0.3.0c, whose C #88 Siphon Squad is live while set and unrevealed, p2's
/// face-down base Siphon Squad gives the AI's two Units −4 Attack (−2 per enemy Unit): Midrange Menace
/// 9 → 5, Tempo Timmy 3 → raw −1, shown 0. The redaction keeps what both show, and Timmy at raw 0: how
/// far below 0 the hidden card took it is not on the board (R185)
#[test]
fn r4803000_r602_on_v0_3_0c_a_hidden_aura_leaves_the_attack_it_shows_and_nothing_below_it() {
    jackioh_cards::register_all();
    let shipped = jackioh_cards::shipped_catalog("v0.3.0c").expect("v0.3.0c is carried");
    jackioh_engine::with_shipped(Some(shipped), || {
        let state = scenario(json!({
            "seed": "redact-negative-attack-siphon",
            "active": "p1",
            "turn": 9,
            "p1": { "field": ["core-019", "core-011"], "hand": ["core-008"], "mana": 3, "library": ["core-020", "core-053"] },
            "p2": { "backrow": [{ "def": "classic-088", "faceUp": false }], "library": ["core-005", "core-016"] },
        }))
        .state()
        .clone();
        let seen = redact(&state, AI);
        let read: Vec<(String, i32, i32, i32)> = active_units_of(&state, AI)
            .iter()
            .map(|unit| {
                let seen_unit = find_instance(&seen, &unit.id).expect("the unit is public");
                (
                    unit.def_id.clone(),
                    unclamped_attack(&state, unit),
                    unit_view(&seen, seen_unit).attack,
                    unclamped_attack(&seen, seen_unit),
                )
            })
            .collect();
        assert_eq!(
            read,
            vec![
                ("core-019".to_string(), 5, 5, 5),
                ("core-011".to_string(), -1, 0, 0),
            ]
        );
    });
}
