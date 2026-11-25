//! IA #42 Paranoia Engine (SPEC §8.13 row 42), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Aura: Your opponent's cards can't destroy or reveal your face-down cards.
//!            End of turn: Set a Figment."
//!   Radiant: "Aura: Your opponent's cards can't destroy or reveal your face-down cards.
//!            End of turn: Set two Figments."
//!
//! The ward is a static flag, read while this acts on its controller's field: `destroy`,
//! `destroyAll` and `reveal` run by an effect the opponent controls pass over its controller's
//! face-down cards — never its own controller's, and never Exile, bounce, steal or transform
//! (R4586004). Several wards are one. The end of its controller's turn sets face-down Figments
//! (`set_figments`); with no open backrow zone, nothing happens (R4586000). Radiant ones on the
//! Radiant face (§7).
//!
//! Engine: `StaticFlags.face_down_ward`, `summon` of a Trap, `setCostOverrideRandom` (R4586004,
//! R4586000). Built by C2 (#4586).

use jackioh_engine::prelude::*;

use crate::scripts::infinite_assault::c025_1_figment::set_figments;

pub const ID: &str = "infiniteassault-042";

fn engine(figments: i32, radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            face_down_ward: Some(true),
            ..StaticFlags::default()
        }),
        end_of_turn: Some(hook(move |_| set_figments(figments, radiant))),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: engine(1, false),
        radiant: engine(2, true),
    }
}

// IA #42 Paranoia Engine — SPEC §8.13 row 42 (R4586004, R4586000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FIGMENT: &str = "infiniteassault-025-1";
    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const JAMMED: &str = "core-036"; // Magic Jammed: Destroy a backrow card. Lock its zone.

    fn standing(radiant: bool, seed: &str) -> Scenario {
        scenario(json!({
            "seed": seed,
            "active": "p2",
            "p1": {
                "field": [{ "def": ID, "radiant": radiant }],
                "backrow": [FIGMENT],
                "hand": [JAMMED, STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": {
                "field": [{ "def": "core-008", "lane": 1 }],
                "hand": [JAMMED, STOCKPILE],
                "library": crate::filler(6),
            },
        }))
    }

    fn figments(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .backrow
            .iter()
            .filter_map(|slot| slot.clone())
            .filter(|card| card.def_id == FIGMENT)
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4586000_the_end_of_turn_sets_a_figment() {
            let mut s = standing(false, "engine-base");
            s.end_turn();
            s.end_turn();
            let set = figments(&s);
            assert_eq!(set.len(), 2, "the set one beside the new");
            assert!(set.iter().all(|card| !card.radiant), "plain on the base face");
        }

        #[test]
        fn r4586004_a_warded_face_down_card_survives_the_enemys_destroy() {
            let mut s = standing(false, "engine-ward");
            let figment = figments(&s)[0].id.clone();
            // p2's destroy passes over it; p1's own destroy still lands.
            let theirs = s.hand(P2).iter().find(|card| card.def_id == JAMMED).expect("p2's Jammed").id.clone();
            s.play(&theirs, pick::targets(pick::id(&figment)));
            s.expect_in_zone(&figment, "field");
            s.end_turn();
            let mine = s.hand(P1).iter().find(|card| card.def_id == JAMMED).expect("p1's Jammed").id.clone();
            s.play(&mine, pick::targets(pick::id(&figment)));
            s.expect_in_zone(&figment, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586000_the_end_of_turn_sets_two_figments() {
            let mut s = standing(true, "engine-radiant");
            s.end_turn();
            s.end_turn();
            let set = figments(&s);
            assert_eq!(set.len(), 3, "the set one beside two new");
            assert_eq!(
                set.iter().filter(|card| card.radiant).count(),
                2,
                "Radiant ones on the Radiant face"
            );
        }
    }
}
