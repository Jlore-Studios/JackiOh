//! SPEC §8.14 FvH #190 Plagiarized: Gestalt, a frozen copy of IA #93 Gestalt (R2195003, FV4), a card of the
//! held Felinors vs Humans set, built by #5235.
//!
//! The code is a copy of the IA card's as it stood when the set was founded, not a re-export, so a later IA
//! patch does not move it (R2195003). Its `ID` is its own, so every read of it names this card; it is named
//! `Plagiarized: Gestalt`, tagged Plagiarized only, and a token it makes stays the IA token. Its change from
//! IA #93: none. The IA file's notes, as they stood:
//!
//! SPEC §8.13 IA #93 Gestalt ((4) Unit, Epic, 2/2 → 4/4), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Fuse every other friendly Unit into this."
//!   Radiant: "Cry: Fuse every other friendly Unit into this. Then it gains Rush and Divine Shield."
//!
//! Every other friendly Unit in lane order, fused into itself (R77, R4588003): the fused-in cards
//! cease to exist with no Death, and a fused-in Cry never runs. The Radiant face then grants itself
//! Rush and Divine Shield, fused anything or not.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-190";

fn gestalt(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let Some(me) = ctx.self_.as_ref() else {
                return vec![];
            };
            let others: Vec<String> = active_units_of(ctx.state, ctx.controller)
                .iter()
                .filter(|unit| unit.id != me.id)
                .map(|unit| unit.id.clone())
                .collect();
            let mut effects = Vec::new();
            if !others.is_empty() {
                effects.push(fuse_cards(json_as(json!({
                    "instanceIds": others,
                    "targetInstanceId": me.id,
                }))));
            }
            if radiant {
                for keyword in [Keyword::Rush, Keyword::DivineShield] {
                    effects.push(grant_keyword(json_as(json!({
                        "target": { "of": "self" },
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
        base: gestalt(false),
        radiant: gestalt(true),
    }
}

// FvH #190 Plagiarized: Gestalt — SPEC §8.14 row 190; IA #93's tests (SPEC §8.13 row 93, R4588003), on this
// card.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const SMALL: &str = "stand-in (1) 2/2";

    fn after_play(radiant: bool, field: Value) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "field": field, "mana": 10 },
            "p2": { "library": crate::filler(2) },
        }));
        s.play(ID, json!({}));
        s
    }

    /// The fused survivor: Gestalt keeps itself and takes the others (R4588003), so one Unit stands.
    fn survivor(s: &Scenario) -> CardInstance {
        let units: Vec<CardInstance> = active_units_of(s.state(), P1).into_iter().cloned().collect();
        assert_eq!(units.len(), 1, "the others are gone");
        units.into_iter().next().expect("Gestalt keeps itself")
    }

    mod base {
        use super::*;

        #[test]
        fn r4588003_gestalt_keeps_the_sum() {
            let mut s = after_play(false, json!([SMALL, SMALL]));
            let kept = survivor(&s);
            s.expect_stats(&kept, json!({ "attack": 6, "health": 6 }));
            assert_eq!(s.count("destroyed"), 0, "a fused-in card does not die");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4588003_gestalt_keeps_the_sum() {
            let s = after_play(true, json!([SMALL, SMALL]));
            let kept = survivor(&s);
            assert!(has_keyword(&s.stats(&kept).keywords, KeywordKind::Rush));
            assert!(has_keyword(&s.stats(&kept).keywords, KeywordKind::DivineShield));
        }

        #[test]
        fn alone_it_still_gains_the_keywords() {
            let mut s = after_play(true, json!([]));
            let kept = survivor(&s);
            s.expect_stats(&kept, json!({ "attack": 4, "health": 4 }));
            assert!(has_keyword(&s.stats(&kept).keywords, KeywordKind::Rush));
            assert!(has_keyword(&s.stats(&kept).keywords, KeywordKind::DivineShield));
        }
    }
}
