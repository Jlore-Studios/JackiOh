//! SPEC §8.13 IA #53 Overvolt Grunt ((1) Unit, Common, 4/3 → 8/6), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Deal {damage} damage to your hero." (damage 2)
//!   Radiant: "Cry: Deal {damage} damage to your hero." (damage 1)
//!
//! Its Cry deals the declared `damage` to its controller's hero (§4.4), read off the face that is
//! running, so one hook serves both faces.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-053";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![damage(json_as(
                json!({ "to": { "of": "selfHero" }, "amount": param(&*ctx, "damage") }),
            ))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 1 is its declared `damage`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #53 Overvolt Grunt — SPEC §8.13 row 53.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn its_cry_deals_2_to_your_hero() {
            let mut s = board(false);
            s.play(ID, json!({ "zone": 1 }));
            s.expect_health(P1, HERO_HEALTH - 2);
            s.expect_health(P2, HERO_HEALTH);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn its_cry_deals_1_to_your_hero() {
            let mut s = board(true);
            s.play(ID, json!({ "zone": 1 }));
            s.expect_health(P1, HERO_HEALTH - 1);
        }
    }
}
