//! SPEC §8.13 IA #115 Phoenix Protocol ((4) Unit, Epic, 7/7 → 14/14), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Reborn\nCry: Give your other Units Reborn."
//!   Radiant: "Reborn\nCry: Give your other Units Reborn and Divine Shield."
//!
//! IA #9 Heavy Lifter Mk.II's shape over its own side's Units but itself (`cards_in_scope` with
//! `excludeSelf`): a granted Reborn on each, lasting while it stays in every zone (R78). The
//! Radiant face grants Divine Shield too. Its own Reborn is the catalog's.

use jackioh_engine::effects::grant_keyword;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-115";

fn phoenix(shield: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects: Vec<Effect> = cards_in_scope(ctx, &json_as(json!({ "side": "self", "excludeSelf": true })))
                .iter()
                .map(|unit| {
                    grant_keyword(json_as(json!({
                        "target": { "of": "instance", "instanceId": unit.id },
                        "keyword": { "kind": "Reborn" },
                    })))
                })
                .collect();
            if shield {
                let more: Vec<Effect> =
                    cards_in_scope(ctx, &json_as(json!({ "side": "self", "excludeSelf": true })))
                        .iter()
                        .map(|unit| {
                            grant_keyword(json_as(json!({
                                "target": { "of": "instance", "instanceId": unit.id },
                                "keyword": { "kind": "Divine Shield" },
                            })))
                        })
                        .collect();
                effects.extend(more);
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: phoenix(false),
        radiant: phoenix(true),
    }
}

// IA #115 Phoenix Protocol — SPEC §8.13 row 115.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";

    /// p1 plays the Protocol beside one own Unit, p2 holding another.
    fn risen(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": [UNIT],
                "library": crate::filler(3),
                "mana": 10,
            },
            "p2": { "field": [UNIT], "library": crate::filler(3) },
        }));
        s.play(ID, json!({ "zone": 2 }));
        s
    }

    fn keywords_of(s: &Scenario, player: PlayerId, lane: i32) -> Vec<Keyword> {
        s.stats(s.unit_at(player, lane)).keywords
    }

    mod base {
        use super::*;

        #[test]
        fn cry_gives_the_other_units_reborn() {
            let s = risen(false);
            assert!(has_keyword(&keywords_of(&s, P1, 1), KeywordKind::Reborn));
            assert!(!has_keyword(&keywords_of(&s, P2, 1), KeywordKind::Reborn));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn cry_gives_the_other_units_reborn_and_divine_shield() {
            let s = risen(true);
            let keywords = keywords_of(&s, P1, 1);
            assert!(has_keyword(&keywords, KeywordKind::Reborn));
            assert!(has_keyword(&keywords, KeywordKind::DivineShield));
        }
    }
}
