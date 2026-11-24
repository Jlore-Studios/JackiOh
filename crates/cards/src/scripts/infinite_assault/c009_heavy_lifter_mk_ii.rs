//! SPEC §8.13 IA #9 Heavy Lifter Mk.II ((3) Unit, Common, 5/7 → 10/14), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt\nCry: Give your other Created Units +{buff}/+{buff}." (buff 1)
//!   Radiant: "Taunt\nCry: Give your other Created Units +{buff}/+{buff}." (buff 2)
//!
//! S #118 Secure System's scope of its own side's Units, this one left out, narrowed to the Created
//! ones (R943); a permanent buff on each. Taunt is the catalog's.

use jackioh_engine::effects::buff;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-009";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let amount = param(&*ctx, "buff");
            cards_in_scope(ctx, &json_as(json!({ "side": "self", "excludeSelf": true })))
                .iter()
                .filter(|unit| is_created(unit))
                .map(|unit| {
                    buff(json_as(json!({
                        "target": { "of": "instance", "instanceId": unit.id },
                        "attack": amount,
                        "health": amount,
                    })))
                })
                .collect()
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's +2/+2 is its declared `buff`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #9 Heavy Lifter Mk.II — SPEC §8.13 row 9.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";

    fn lifted(radiant: bool, created: bool) -> (Scenario, [CardInstance; 3], CardInstance) {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant, "created": created }],
                "field": [{ "def": UNIT, "created": true }, UNIT],
            },
            "p2": { "field": [{ "def": UNIT, "created": true }] },
        }));
        s.play(ID, json!({ "zone": 3 }));
        let units = [s.unit_at(P1, 1), s.unit_at(P1, 2), s.unit_at(P2, 1)];
        let lifter = s.unit_at(P1, 3);
        (s, units, lifter)
    }

    mod base {
        use super::*;

        #[test]
        fn cry_gives_your_other_created_units_1_1_and_no_one_else() {
            let (mut s, [made, dealt, enemy], lifter) = lifted(false, true);
            s.expect_stats(&made, json!({ "attack": 3, "health": 3 }));
            s.expect_stats(&dealt, json!({ "attack": 2, "health": 2 }));
            s.expect_stats(&enemy, json!({ "attack": 2, "health": 2 }));
            s.expect_stats(&lifter, json!({ "attack": 5, "health": 7 }));
            assert!(has_keyword(&s.stats(&lifter).keywords, KeywordKind::Taunt));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn cry_gives_your_other_created_units_2_2() {
            let (mut s, [made, dealt, enemy], lifter) = lifted(true, false);
            s.expect_stats(&made, json!({ "attack": 4, "health": 4 }));
            s.expect_stats(&dealt, json!({ "attack": 2, "health": 2 }));
            s.expect_stats(&enemy, json!({ "attack": 2, "health": 2 }));
            s.expect_stats(&lifter, json!({ "attack": 10, "health": 14 }));
        }
    }
}
