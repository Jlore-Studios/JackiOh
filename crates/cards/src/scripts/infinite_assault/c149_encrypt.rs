//! SPEC §8.13 IA #149 Encrypt ((1) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Give a friendly Unit Divine Shield and +0/+{health}." (health 3)
//!   Radiant: "Give a friendly Unit Divine Shield and +0/+{health} and Immune to Spells." (health 6)
//!
//! Engine: the friendly Unit picked with the play (R656, aimed to help), granted Divine Shield and
//! buffed +0/+{health} as it resolves; on the Radiant face, Immune to Spells last.

use jackioh_engine::effects::{buff, grant_keyword};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-149";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Help),
        ..TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"] }))
    }]
}

fn encrypt(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let health = param(&*ctx, "health");
            let mut effects = vec![
                grant_keyword(json_as(
                    json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Divine Shield" } }),
                )),
                buff(json_as(json!({ "target": { "of": "chosen" }, "health": health }))),
            ];
            if radiant {
                effects.push(grant_keyword(json_as(
                    json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Immune to Spells" } }),
                )));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: encrypt(false),
        radiant: encrypt(true),
    }
}

// IA #149 Encrypt — SPEC §8.13 row 149.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [UNIT],
                "library": crate::filler(3),
            },
            "p2": { "field": [UNIT], "library": crate::filler(3) },
        }))
    }

    fn keywords_of(s: &Scenario, card: &CardInstance) -> Vec<Keyword> {
        s.stats(card).keywords
    }

    mod base {
        use super::*;

        #[test]
        fn divine_shield_and_0_3_no_immunity() {
            let mut s = board(false);
            let unit = s.unit_at(P1, 1);
            s.play(ID, pick::targets(pick::instance(&unit)));
            let buffed = s.unit_at(P1, 1);
            s.expect_stats(&buffed, json!({ "attack": 2, "health": 5 }));
            let keywords = keywords_of(&s, &buffed);
            assert!(has_keyword(&keywords, KeywordKind::DivineShield));
            assert!(!has_keyword(&keywords, KeywordKind::ImmuneToSpells));
            s.expect_in_zone(ID, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn divine_shield_and_0_6_and_immune_to_spells() {
            let mut s = board(true);
            let unit = s.unit_at(P1, 1);
            s.play(ID, pick::targets(pick::instance(&unit)));
            let buffed = s.unit_at(P1, 1);
            s.expect_stats(&buffed, json!({ "attack": 2, "health": 8 }));
            let keywords = keywords_of(&s, &buffed);
            assert!(has_keyword(&keywords, KeywordKind::DivineShield));
            assert!(has_keyword(&keywords, KeywordKind::ImmuneToSpells));
        }
    }
}
