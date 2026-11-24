//! SPEC §8.13 IA #76 Mob Mentality ((1) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Give your Units +{buff}/+{buff}." (buff 1)
//!   Radiant: "Give your Units +{buff}/+{buff}." (buff 2)
//!
//! A permanent `buffAllUnits` over its controller's side: every Unit they control gets the declared
//! buff as layer-4 stats (§10.4).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-076";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "buff");
            vec![buff_all_units(json_as(json!({ "attack": amount, "health": amount })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's +2/+2 is its declared `buff`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #76 Mob Mentality — SPEC §8.13 row 76.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "field": [UNIT, UNIT] },
            "p2": { "field": [UNIT] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn your_units_get_1_1_and_the_enemys_get_nothing() {
            let mut s = board(false);
            s.play(ID, json!({}));
            for lane in [1, 2] {
                s.expect_stats(s.unit_at(P1, lane), json!({ "attack": 3, "health": 3 }));
            }
            s.expect_stats(s.unit_at(P2, 1), json!({ "attack": 2, "health": 2 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn your_units_get_2_2() {
            let mut s = board(true);
            s.play(ID, json!({}));
            for lane in [1, 2] {
                s.expect_stats(s.unit_at(P1, lane), json!({ "attack": 4, "health": 4 }));
            }
        }
    }
}
