//! SPEC §8.12 S+ #2 Lone Drifter (Unit, Swarm 2/2 → 4/4, cost 1, Common), a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Evolves when all your Unit zones are full.
//!            Evolve: Gain +4/+4."
//!   Radiant: "Evolves when all your Unit zones are full.
//!            Evolve: Gain +8/+8."
//!
//! "Evolves when" is a standing condition, read at the state check once the board has settled
//! (`evolves_when`, R3536001): no zone of the controller's unit row is empty (a pile counts once, a
//! Dormant card fills its zone, R1839000; a zone a GIGA covers or the slash voided is no room, so it is
//! full, R3546000, R3554002). The Evolve text runs once (R3536000): a permanent
//! `buff` of its own, the declared `buff` on the base face and `buff2` on the Radiant (the faces print
//! different numbers, so each reads its own key).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-002";

/// R3536001: no unit zone of the controller's side is empty, by the board's own reading (`is_empty`): a
/// zone a GIGA covers (R3546000) or the slash voided (R3554002) is no room for a card, so it counts as
/// full, as it does for every play and summon.
fn zones_full() -> HasHook {
    read_hook(|a| {
        slots_of(a.self_.controller, Row::Units)
            .into_iter()
            .all(|slot| !is_empty(&*a.state, slot))
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

        /// R3536001, R3546000: a GIGA Unit in lanes 4 and 5 leaves no zone of the row empty, so the row is
        /// full though only four piles hold a card.
        #[test]
        fn r3536001_a_giga_covering_the_last_zone_fills_the_row() {
            const GIGA: &str = "stinkyscapeplus-t-ai-04"; // Jlockeed Large Language Model: GIGA, Taunt
            let mut s = scenario(json!({
                "p1": {
                    "hand": [GIGA],
                    "field": [{ "def": ID }, VANILLA, VANILLA],
                    "mana": 10,
                    "library": crate::filler(4),
                },
                "p2": { "library": crate::filler(4) },
            }));
            let drifter = s.unit_at(P1, 1);
            assert_ne!(s.card(&drifter).evolved, Some(true), "two zones hold nothing");
            s.play(GIGA, json!({ "zone": 4 }));
            assert_eq!(s.card(&drifter).evolved, Some(true));
            s.expect_stats(&drifter, json!({ "attack": 6, "health": 6 }));
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
