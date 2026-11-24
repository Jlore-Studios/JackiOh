//! SPEC §8.13 IA #67 Burning Ambition ((2) Field Spell, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Start of turn: Deal {damage} damage to your hero and gain {mana} mana this turn." (damage 2, mana 1)
//!   Radiant: "Start of turn: Deal {damage} damage to your hero and gain {mana} mana this turn." (damage 2, mana 2)
//!
//! Core #6 Mana Well's shape with a price: at its controller's start of turn (§2.2, §6.2, R68) the
//! declared `damage` to its controller's hero, then the declared `mana` gained this turn
//! (`gain_mana`, §2.3), in the text's order.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-067";

pub fn script() -> CardScripts {
    let base = Script {
        start_of_turn: Some(hook(|ctx| {
            vec![
                damage(json_as(
                    json!({ "to": { "of": "selfHero" }, "amount": param(&*ctx, "damage") }),
                )),
                gain_mana(json_as(json!({ "amount": param(&*ctx, "mana") }))),
            ]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 2 is its declared `mana`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #67 Burning Ambition — SPEC §8.13 row 67.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    /// The Ambition face-up in p1's backrow, and p1's turn started.
    fn turn_starts(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant }],
                "library": crate::filler(2),
            },
            "p2": { "library": crate::filler(2) },
        }));
        s.start_turn();
        s
    }

    mod base {
        use super::*;

        #[test]
        fn at_your_start_of_turn_2_to_your_hero_and_1_mana() {
            let mut s = turn_starts(false);
            s.expect_health(P1, HERO_HEALTH - 2);
            s.expect_health(P2, HERO_HEALTH);
            s.expect_mana(P1, 5);
        }

        #[test]
        fn r68_nothing_at_the_opponents_start_of_turn() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": { "backrow": [ID], "library": crate::filler(2) },
                "p2": { "library": crate::filler(2) },
            }));
            s.start_turn();
            s.expect_health(P1, HERO_HEALTH);
            s.expect_mana(P1, 4);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn at_your_start_of_turn_2_to_your_hero_and_2_mana() {
            let mut s = turn_starts(true);
            s.expect_health(P1, HERO_HEALTH - 2);
            s.expect_mana(P1, 6);
        }
    }
}
