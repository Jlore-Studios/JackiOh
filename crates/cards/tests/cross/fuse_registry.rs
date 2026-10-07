//! A fused card's scripts across matches in one process (SPEC §9.2, §9.3, R77, R102, R179). Found by
//! the polish-4 edge-case hunt (docs/polish/4-edge-cases.md, lens L7); it failed before its fix.
//!
//! A fused definition is match state, but its scripts are code the process registers under the
//! definition's id. A server runs every match in one process and folds a match's log to rebuild it,
//! so two matches that fused different pairs into the same slot must not share an id: R179 has the id
//! name its ingredients, so the later fusion can no longer replace the earlier match's scripts.
//!
//! Port of `packages/cards/test/fuse-registry.test.ts` (SURFACE §4.1, §8). In Rust a fused card's
//! scripts are built on lookup from the state (SURFACE §6.6), so this holds by construction; the test
//! still proves it end to end.

use jackioh_engine::testkit::*;
use jackioh_engine::PlayerId::{P1, P2};

const SHREDDER: &str = "core-013";
const MENACE: &str = "core-019";
const POINTMASTER: &str = "core-020";
const SEVEN_SEVEN: &str = "core-025";
const RENO: &str = "core-053";
const UNLICENSED: &str = "core-085";
const LIBRARY: [&str; 6] = [RENO; 6];

use super::scenario;

fn unit_at(g: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
    match g.unit(player, lane) {
        Some(card) => card.clone(),
        None => panic!("setup: {player} should hold a unit in lane {lane}"),
    }
}

/// p1 plays Pointmaster into p2's Unlicensed Experimentation, which fuses it onto p2's lane-1 unit.
fn fused_onto(big_unit: &str) -> Scenario {
    let mut g = scenario(json!({
        "p1": { "hand": [POINTMASTER, RENO, RENO], "field": [{ "def": SEVEN_SEVEN, "lane": 5 }], "library": LIBRARY },
        "p2": {
            "hand": [RENO, RENO],
            "field": [{ "def": big_unit, "lane": 1 }],
            "backrow": [{ "def": UNLICENSED, "lane": 1 }],
            "library": LIBRARY,
        },
    }));
    g.play(POINTMASTER, json!({}));
    g
}

/// TS `JSON.parse(JSON.stringify(state))`: only the state's JSON is kept, as a match actor keeps it.
fn through_json(state: &GameState) -> GameState {
    let text = serde_json::to_string(state).expect("a state serialises");
    serde_json::from_str(&text).expect("a state's JSON parses back")
}

/// From a saved state: p1 ends the turn, then p2 does; p1's hero health after it.
fn hero_after_p2_ends_turn(saved: &GameState) -> i32 {
    let mut state = through_json(saved);
    for player in [P1, P2] {
        let result = reduce(&state, &Action::new(ActionBody::EndTurn, player, format!("fuse-{player}")));
        assert_eq!(result.error, None);
        state = result.state;
    }
    state.players.p1.hero.health
}

mod r179_a_fused_definitions_id_names_its_ingredients {
    use super::*;

    #[test]
    fn r179_r77_a_fused_card_keeps_its_own_scripts_when_another_match_in_the_same_process_fuses_a_different_pair_into_the_same_slot_9_3()
     {
        // Match A fuses Pointmaster onto Jlockeed Shredder-10 ("End of turn: deal 2 damage to each
        // enemy unit and the enemy hero"); only its JSON is kept, as a match actor keeps it (§9.3).
        let a = fused_onto(SHREDDER);
        let fused_a = a.card(&unit_at(&a, P2, 1).id).def_id.clone();
        assert!(a.state().transient_defs.contains_key(&fused_a));
        assert_eq!(fused_a, format!("t-1:{POINTMASTER}+{SHREDDER}"));
        let saved_a = through_json(a.state());
        assert_eq!(hero_after_p2_ends_turn(&saved_a), 28);

        // Match B, same process, same fusion slot, a different pair (Midrange Menace heals itself at the
        // end of the turn and deals no damage): a different id, so its scripts register beside A's.
        let b = fused_onto(MENACE);
        let fused_b = b.card(&unit_at(&b, P2, 1).id).def_id.clone();
        assert_eq!(fused_b, format!("t-1:{POINTMASTER}+{MENACE}"));
        assert_ne!(fused_b, fused_a);

        // Back in A, from the very same saved state: nothing about A changed, so neither may the result.
        assert_eq!(hero_after_p2_ends_turn(&saved_a), 28);
    }
}
