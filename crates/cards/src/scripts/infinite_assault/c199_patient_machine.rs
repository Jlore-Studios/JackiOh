//! IA #199 Patient Machine (SPEC §8.13 row 199), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Dormant for 1 turn."
//!   Radiant: "Dormant for 1 turn.\nWhen this awakens, deal {damage} damage to a random enemy."
//!
//! Engine: Dormant (R1839000), `StaticFlags.slumber { turns: 1 }`, awake at the start of its
//! controller's next turn (R1839001). The Radiant face's `when_this_awakens` (R1839002) deals the
//! declared `damage` to one random enemy still standing (`damage_random_enemies`). Built by C9
//! (#4593).

use jackioh_engine::effects::damage_random_enemies;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-199";

/// "Dormant for 1 turn": the status's printed count, no param.
const DORMANT_TURNS: i32 = 1;

fn patient(radiant_face: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            slumber: Some(SlumberText {
                turns: Some(DORMANT_TURNS),
                wakes_by: None,
            }),
            ..StaticFlags::default()
        }),
        triggers: if radiant_face {
            vec![when_this_awakens("awakens", |ctx, _event| {
                vec![damage_random_enemies(json_as(json!({
                    "count": 1,
                    "amount": param(&*ctx, "damage"),
                })))]
            })]
        } else {
            vec![]
        },
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: patient(false),
        radiant: patient(true),
    }
}

// IA #199 Patient Machine — SPEC §8.13 row 199.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn played(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        s
    }

    fn awake(s: &mut Scenario) {
        s.end_turn();
        s.end_turn();
        assert_eq!(s.state().active, P1);
    }

    mod base {
        use super::*;

        #[test]
        fn wakes_at_the_next_own_start_and_deals_nothing() {
            let mut s = played(false, "patient-base");
            assert!(is_slumbering(&s.unit_at(P1, 1)));
            awake(&mut s);
            assert!(!is_slumbering(&s.unit_at(P1, 1)));
            assert_eq!(s.state().players[P2].hero.health, HERO_HEALTH);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn wakes_at_the_next_own_start_and_hits_the_enemy_hero_on_an_empty_board() {
            let mut s = played(true, "patient-radiant");
            awake(&mut s);
            assert!(!is_slumbering(&s.unit_at(P1, 1)));
            assert_eq!(s.state().players[P2].hero.health, HERO_HEALTH - 4);
        }
    }
}
