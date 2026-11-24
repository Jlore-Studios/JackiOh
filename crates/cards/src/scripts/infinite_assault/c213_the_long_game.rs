//! IA #213 The Long Game (SPEC §8.13 row 213), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Dormant for 2 turns.\nWhen this awakens, destroy every enemy Unit."
//!   Radiant: "Dormant for 2 turns.\nWhen this awakens, destroy every enemy Unit and draw a card."
//!
//! Engine: Dormant (R1839000), `StaticFlags.slumber { turns: 2 }`, awake at the start of its
//! controller's second turn after it arrived (R1839001). `when_this_awakens` (R1839002)
//! `destroy_all`s the enemy Units; the Radiant face then `draw`s one card. Built by C9 (#4593).

use jackioh_engine::effects::{destroy_all, draw};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-213";

/// "Dormant for 2 turns": the status's printed count, no param.
const DORMANT_TURNS: i32 = 2;

/// The Radiant face's draw.
const RADIANT_DRAWS: i32 = 1;

fn long_game(radiant_face: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            slumber: Some(SlumberText {
                turns: Some(DORMANT_TURNS),
                wakes_by: None,
            }),
            ..StaticFlags::default()
        }),
        triggers: vec![when_this_awakens("awakens", move |_ctx, _event| {
            let mut effects = vec![destroy_all(json_as(json!({
                "side": "enemy",
                "rows": ["units"],
            })))];
            if radiant_face {
                effects.push(draw(json_as(json!({ "count": RADIANT_DRAWS }))));
            }
            effects
        })],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: long_game(false),
        radiant: long_game(true),
    }
}

// IA #213 The Long Game — SPEC §8.13 row 213.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn awakened(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": {
                "field": [VANILLA],
                "hand": [FILLER],
                "library": crate::filler(4),
            },
        }));
        s.play(ID, json!({ "zone": 1 }));
        for _ in 0..4 {
            s.end_turn();
        }
        assert_eq!(s.state().active, P1);
        s
    }

    mod base {
        use super::*;

        #[test]
        fn destroys_every_enemy_unit_at_the_2nd_own_start() {
            let s = awakened(false, "long-game-base");
            assert!(s.unit(P2, 1).is_none());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn destroys_every_enemy_unit_and_draws_one_more_card() {
            let s = awakened(true, "long-game-radiant");
            assert!(s.unit(P2, 1).is_none());
            // Two turn draws plus the awakening's own: one more card in hand than the base
            // face's three.
            assert_eq!(s.hand(P1).len(), 4);
        }
    }
}
