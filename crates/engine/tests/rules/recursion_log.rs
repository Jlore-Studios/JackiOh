//! R4589000–R4589001: the recursion log (`recursion_log.rs`), proved on fixture scripts
//! (`fixtures::trigger_multipliers`), with `infiniteassault-097` in a library to turn the log on.

use jackioh_engine::recursion_log::{
    deaths_since_last_turn, deaths_this_game, deaths_this_turn, played_this_game,
};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use super::fixtures::trigger_multipliers::{BOLT, DIER, PAWN, register};

/// A board for p1, with a vanilla unit and a stocked library on each side so no turn auto-ends
/// (§2.5) and no draw fatigues; `p1` and `p2` add their own entries.
fn game(p1: Value, p2: Value) -> Scenario {
    register();
    let mut sides = json!({
        "p1": { "field": [{ "def": PAWN, "lane": 5 }], "library": [PAWN, PAWN, PAWN] },
        "p2": { "field": [{ "def": PAWN, "lane": 5 }], "library": [PAWN, PAWN, PAWN] },
    });
    for (seat, extra) in [("p1", p1), ("p2", p2)] {
        if let Value::Object(entries) = extra {
            for (key, value) in entries {
                let side = sides[seat].as_object_mut().expect("a side is an object");
                match (side.get_mut(&key), value) {
                    (Some(Value::Array(list)), Value::Array(more)) => list.extend(more),
                    (_, value) => {
                        side.insert(key, value);
                    }
                }
            }
        }
    }
    scenario(sides)
}

fn instance(card: &CardInstance) -> Value {
    json!({ "pick": "instance", "instanceId": card.id })
}

const RETRY_BOT: &str = "infiniteassault-097";

#[test]
fn r4589000_plays_and_unit_deaths_are_logged() {
    let mut s = game(
        json!({ "field": [{ "def": DIER }], "hand": [BOLT], "library": [RETRY_BOT] }),
        json!({}),
    );
    let dier = s.unit(P1, 1).expect("p1's dier");
    assert!(
        s.state().players[P1].recursion.is_none(),
        "no log before the first play"
    );
    s.play(BOLT, json!({ "targets": [instance(&dier)] }));
    let log = s.state().players[P1].recursion.clone().expect("the log is kept");
    assert_eq!(
        log.played
            .iter()
            .map(|record| record.def_id.clone())
            .collect::<Vec<_>>(),
        vec![BOLT.to_string()],
        "the face-up play is logged"
    );
    assert_eq!(
        log.deaths
            .iter()
            .map(|death| death.def_id.clone())
            .collect::<Vec<_>>(),
        vec![DIER.to_string()],
        "the Unit that died under p1 is logged"
    );
    assert!(!log.deaths[0].radiant, "it died on its base face");
    // The first write opens both sides' logs at once.
    let other = s.state().players[P2]
        .recursion
        .clone()
        .expect("p2's log opens too");
    assert!(other.played.is_empty() && other.deaths.is_empty());
    // The reads agree with the log.
    assert_eq!(played_this_game(s.state(), P1).len(), 1);
    assert_eq!(deaths_this_game(s.state(), P1).len(), 1);
    assert_eq!(deaths_this_turn(s.state(), P1).len(), 1);
    assert!(played_this_game(s.state(), P2).is_empty());
}

#[test]
fn r4589000_a_game_without_an_infinite_assault_card_logs_nothing() {
    let mut s = game(json!({ "hand": [BOLT] }), json!({}));
    s.play(
        BOLT,
        json!({ "targets": [json!({ "pick": "hero", "player": "p2" })] }),
    );
    assert!(s.state().players[P1].recursion.is_none());
    assert!(s.state().players[P2].recursion.is_none());
    assert!(deaths_this_game(s.state(), P1).is_empty());
}

#[test]
fn r4589000_a_trap_set_face_down_is_not_logged() {
    let mut s = game(
        json!({ "hand": [BOLT, "classic-014"], "library": [RETRY_BOT] }),
        json!({}),
    );
    s.play(
        BOLT,
        json!({ "targets": [json!({ "pick": "hero", "player": "p2" })] }),
    );
    s.play("classic-014", json!({ "zone": 1 }));
    let played = played_this_game(s.state(), P1);
    assert_eq!(
        played
            .iter()
            .map(|record| record.def_id.clone())
            .collect::<Vec<_>>(),
        vec![BOLT.to_string()],
        "the face-down set is no face-up play (R451)"
    );
}

#[test]
fn r4589001_since_last_turn_reads_this_turn_and_the_one_before() {
    let mut s = game(
        json!({ "field": [{ "def": DIER }], "hand": [BOLT], "library": [RETRY_BOT] }),
        json!({ "field": [{ "def": DIER }], "hand": [BOLT], "library": [PAWN] }),
    );
    let mine = s.unit(P1, 1).expect("p1's dier");
    s.play(BOLT, json!({ "targets": [instance(&mine)] }));
    s.end_turn();
    let theirs = s.unit(P2, 1).expect("p2's dier");
    s.play(BOLT, json!({ "targets": [instance(&theirs)] }));
    // On p2's turn: p1's death is the turn before, p2's is this turn.
    assert!(deaths_this_turn(s.state(), P1).is_empty());
    assert_eq!(deaths_since_last_turn(s.state(), P1).len(), 1);
    assert_eq!(deaths_this_turn(s.state(), P2).len(), 1);
    assert_eq!(deaths_since_last_turn(s.state(), P2).len(), 1);
    s.end_turn();
    // A turn later p1's death is older than "since your last turn"; p2's is the turn before.
    assert!(deaths_this_turn(s.state(), P1).is_empty());
    assert!(deaths_since_last_turn(s.state(), P1).is_empty());
    assert!(deaths_this_turn(s.state(), P2).is_empty());
    assert_eq!(deaths_since_last_turn(s.state(), P2).len(), 1);
}
