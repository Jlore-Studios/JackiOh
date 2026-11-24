//! SPEC §8.13 IA #163 Quarantine Wall ((4) Unit, Epic, 0/20 → 0/40), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt, Can't attack. Start of turn: Your hero gains {armor} Armor." (armor 3)
//!   Radiant: "Taunt, Can't attack. Start of turn: Your hero gains {armor} Armor." (armor 6)
//!
//! Engine: at the start of its controller's turn, the controller's hero gains the declared {armor}
//! Armor (§4.4 step 2). One script: the Radiant face's 6 is its declared param; Taunt, Can't attack
//! and the doubled stats are the catalog's data.

use jackioh_engine::effects::gain_hero_armor;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-163";

fn wall() -> Script {
    Script {
        start_of_turn: Some(hook(|ctx| {
            vec![gain_hero_armor(json_as(json!({
                "amount": param(ctx, "armor"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: the Radiant face's 6 is its declared `armor`, read off the running face.
    let base = wall();
    CardScripts {
        base: base.clone(),
        radiant: base,
    }
}

// IA #163 Quarantine Wall — SPEC §8.13 row 163.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "quarantine-wall",
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "library": crate::filler(3),
                "mana": 8,
            },
            "p2": {
                "field": ["stand-in (1) 2/2"],
                "hand": ["stand-in (0) Spell"],
                "library": crate::filler(3),
            },
        }))
    }

    fn armor(s: &Scenario) -> i32 {
        s.state().players[P1].hero.armor
    }

    mod base {
        use super::*;

        #[test]
        fn a_0_20_taunt_whose_turn_start_gains_3_armor() {
            let mut s = board(false);
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(&s.unit_at(P1, 1), json!({ "attack": 0, "health": 20 }));
            assert_eq!(armor(&s), 0);
            s.end_turn();
            s.end_turn();
            assert_eq!(armor(&s), 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_0_40_taunt_whose_turn_start_gains_6_armor() {
            let mut s = board(true);
            s.play(ID, json!({ "zone": 1 }));
            s.expect_stats(&s.unit_at(P1, 1), json!({ "attack": 0, "health": 40 }));
            s.end_turn();
            s.end_turn();
            assert_eq!(armor(&s), 6);
        }
    }
}
