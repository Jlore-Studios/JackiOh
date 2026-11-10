//! SPEC §8.12 S+ #2 Lone Drifter (Unit, Swarm 2/2 → 4/4, cost 1, Common), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Evolves when all your Unit zones are full.
//!            Evolve: Gain +4/+4."
//!   Radiant: "Evolves when all your Unit zones are full.
//!            Evolve: Gain +8/+8."
//!
//! "Evolves when" is a standing condition, read at the state check once the board has settled
//! (`evolves_when`, R3536001): every one of the controller's unit zones holds a card (a pile counts
//! once, a Dormant card fills its zone, R1839000). The Evolve text runs once (R3536000): a permanent
//! `buff` of its own, the declared `buff` on the base face and `buff2` on the Radiant (the faces print
//! different numbers, so each reads its own key).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-002";

/// R3536001: every unit zone of the controller's side holds a card.
fn zones_full() -> HasHook {
    read_hook(|a| {
        a.state.players[a.self_.controller]
            .units
            .iter()
            .filter(|pile| pile.as_ref().is_some_and(|pile| !pile.is_empty()))
            .count()
            == UNIT_ZONES as usize
    })
}

/// "Evolve: Gain +N/+N": the face's own declared number, `key`.
fn gain(key: &'static str) -> Hook {
    hook(move |ctx| {
        let amount = param(ctx, key);
        vec![buff(json_as(json!({
            "target": { "of": "self" },
            "attack": amount,
            "health": amount,
        })))]
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            evolves_when: Some(zones_full()),
            evolve: Some(gain("buff")),
            ..Script::default()
        },
        radiant: Script {
            evolves_when: Some(zones_full()),
            evolve: Some(gain("buff2")),
            ..Script::default()
        },
    }
}

// S+ #2 Lone Drifter — SPEC §8.12 row 2.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    fn board(radiant: bool, others: usize) -> Scenario {
        let mut field = vec![json!({ "def": ID, "radiant": radiant })];
        field.extend((0..others).map(|_| json!(VANILLA)));
        scenario(json!({
            "p1": { "hand": [VANILLA], "field": field, "library": crate::filler(4) },
            "p2": { "library": crate::filler(4) },
        }))
    }

    fn evolved_events(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| event.event_type() == GameEventType::Evolved)
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn five_full_zones_evolve_it_to_6_6() {
            let mut s = board(false, 3);
            let drifter = s.unit_at(P1, 1);
            s.expect_stats(&drifter, json!({ "attack": 2, "health": 2 }));
            assert_ne!(
                s.card(&drifter).evolved,
                Some(true),
                "four of five zones hold nothing"
            );
            s.play(VANILLA, json!({ "zone": 5 }));
            assert_eq!(s.card(&drifter).evolved, Some(true));
            s.expect_stats(&drifter, json!({ "attack": 6, "health": 6 }));
            assert_eq!(evolved_events(&s), 1);
            assert!(s.state().pending.is_none(), "it opens no prompt");
        }

        #[test]
        fn four_zones_evolve_nothing() {
            let mut s = board(false, 3);
            let drifter = s.unit_at(P1, 1);
            s.end_turn();
            s.end_turn();
            assert_ne!(s.card(&drifter).evolved, Some(true));
            s.expect_stats(&drifter, json!({ "attack": 2, "health": 2 }));
            assert_eq!(evolved_events(&s), 0);
        }

        #[test]
        fn it_evolves_once_and_a_full_row_gives_it_no_second_buff() {
            let mut s = board(false, 4);
            let drifter = s.unit_at(P1, 1);
            assert_eq!(
                s.card(&drifter).evolved,
                Some(true),
                "the setup's state check evolves it"
            );
            // The setup's queued Evolve text drains on the first step.
            s.end_turn();
            s.expect_stats(&drifter, json!({ "attack": 6, "health": 6 }));
            s.end_turn();
            s.end_turn();
            s.expect_stats(&drifter, json!({ "attack": 6, "health": 6 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn five_full_zones_evolve_it_to_12_12() {
            let mut s = board(true, 3);
            let drifter = s.unit_at(P1, 1);
            s.expect_stats(&drifter, json!({ "attack": 4, "health": 4 }));
            s.play(VANILLA, json!({ "zone": 5 }));
            assert_eq!(s.card(&drifter).evolved, Some(true));
            s.expect_stats(&drifter, json!({ "attack": 12, "health": 12 }));
            assert_eq!(evolved_events(&s), 1);
            assert!(s.state().pending.is_none());
        }

        #[test]
        fn four_zones_evolve_nothing() {
            let mut s = board(true, 3);
            let drifter = s.unit_at(P1, 1);
            s.end_turn();
            s.end_turn();
            assert_ne!(s.card(&drifter).evolved, Some(true));
            s.expect_stats(&drifter, json!({ "attack": 4, "health": 4 }));
        }
    }
}
