//! SPEC §8.13 IA #85 Merge Conflict ((2) Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Fuse two friendly Units."
//!   Radiant: "Fuse two friendly Units. The result has Rush and Divine Shield."
//!
//! Two declared friendly Units (R703: the play needs both), aimed to help (R656). The first one
//! chosen is kept and the second fused into it (R77, R4588003): the fused-in card ceases to exist
//! with no Death, and its Cry never runs. The Radiant face then grants the result Rush and Divine
//! Shield.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-085";

fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl {
        aim: Some(TargetAim::Help),
        required: Some(true),
        ..TargetDecl::target(2, 2, json!({ "side": "ally", "of": ["unit"] }))
    }]
}

fn merge_conflict(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let Some(first) = instance_of(ctx, &TargetSpec::Chosen { index: None }) else {
                return vec![];
            };
            let Some(second) = instance_of(ctx, &TargetSpec::Chosen { index: Some(1) }) else {
                return vec![];
            };
            let mut effects = vec![fuse_cards(json_as(json!({
                "instanceIds": [second.id],
                "targetInstanceId": first.id,
            })))];
            if radiant {
                for keyword in [Keyword::Rush, Keyword::DivineShield] {
                    effects.push(grant_keyword(json_as(json!({
                        "target": { "of": "chosen", "index": 0 },
                        "keyword": keyword,
                    }))));
                }
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: merge_conflict(false),
        radiant: merge_conflict(true),
    }
}

// IA #85 Merge Conflict — SPEC §8.13 row 85 (R4588003).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const BIG: &str = "stand-in (2) 5/5";
    const SMALL: &str = "stand-in (1) 2/2";

    fn board(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "field": field },
            "p2": { "library": crate::filler(2) },
        }))
    }

    /// Fuse p1's lanes 1 and 2, in that order.
    fn fuse(s: &mut Scenario) {
        let (first, second) = (s.unit_at(P1, 1), s.unit_at(P1, 2));
        s.play(
            ID,
            pick::targets(json!([
                { "pick": "instance", "instanceId": first.id },
                { "pick": "instance", "instanceId": second.id },
            ])),
        );
    }

    fn keywords_of(s: &Scenario, unit: &CardInstance) -> Vec<Keyword> {
        s.stats(unit).keywords
    }

    mod base {
        use super::*;

        #[test]
        fn r4588003_the_first_unit_keeps_the_sum() {
            let mut s = board(false, json!([BIG, SMALL]));
            fuse(&mut s);
            let units: Vec<CardInstance> = active_units_of(s.state(), P1).into_iter().cloned().collect();
            assert_eq!(units.len(), 1, "the second is fused into the first");
            s.expect_stats(&units[0], json!({ "attack": 7, "health": 7 }));
            assert_eq!(s.count("destroyed"), 0, "a fused-in card does not die");
        }

        #[test]
        fn r4588003_one_friendly_unit_cannot_play_it() {
            let s = board(false, json!([BIG]));
            assert!(!s.can_play(ID));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_result_has_rush_and_divine_shield() {
            let mut s = board(true, json!([BIG, SMALL]));
            fuse(&mut s);
            let units: Vec<CardInstance> = active_units_of(s.state(), P1).into_iter().cloned().collect();
            assert_eq!(units.len(), 1);
            s.expect_stats(&units[0], json!({ "attack": 7, "health": 7 }));
            assert!(has_keyword(&keywords_of(&s, &units[0]), KeywordKind::Rush));
            assert!(has_keyword(&keywords_of(&s, &units[0]), KeywordKind::DivineShield));
        }
    }
}
