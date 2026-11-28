//! SPEC §8.14 FvH #88 Cattail ((2) Unit, Felinor, Common, 5/3 → 10/6), a card of the held Felinors vs Humans
//! set (R5235000).
//!   Base:    "Flexible, Ranged, Pierce\nCan only be mistaken for a Felinor when beneficial."
//!   Radiant: "Flexible, Ranged, Pierce, Cleave\nCan only be mistaken for a Felinor when beneficial."
//!
//! Its keywords are catalog data. FV2 #88: it prints the Felinor tag, so every helpful read sees it (auras,
//! counts, conditions, recursion, random Felinor pools once the set ships), and `harm_ignores_tags`
//! [Felinor] lets a harmful filter whose `tags` name Felinor pass it by, in every zone (R5237009, a narrower
//! M #25, R940). A "non-Felinor" harm passes it by too, since it carries the tag. Declares no numbers.
//! Built by S07 (#5257).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-088";

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            harm_ignores_tags: Some(vec![Tag::Felinor]),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    // The same script: the Radiant face's Cleave and stats are catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #88 Cattail — SPEC §8.14 row 88.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FLAGBEARER: &str = "classicplus-046"; // Felinor Flagbearer: Aura: your other Felinors have +aura/+aura
    const BIG_FELINOR: &str = "core-043"; // Big Felinor: Cry: destroy a random non-Felinor Unit
    const VANILLA: &str = "core-008"; // Mr. Vanilla: a Human, no Felinor
    const FILLER: &str = "core-005"; // Stockpile: a card to hold

    fn cattail(s: &Scenario, player: PlayerId) -> CardInstance {
        must(
            active_units_of(s.state(), player)
                .into_iter()
                .find(|unit| unit.def_id == ID)
                .cloned(),
            "the Cattail on the field",
        )
    }

    fn has(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> bool {
        s.stats(card).keywords.iter().any(|keyword| keyword.kind() == kind)
    }

    fn harm_naming_felinor_passes_it_by(radiant: bool) {
        let s = scenario(json!({
            "p1": { "field": [{ "def": ID, "radiant": radiant }, "core-t-felinor"], "hand": [FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        let me = cattail(&s, P1);
        assert!(
            tribal_hate_cannot_reach(s.state(), &me, Some(&[Tag::Felinor][..]), None),
            "a harmful filter naming Felinor passes it by (R5237009)"
        );
        let token = s.unit_at(P1, 2);
        assert!(
            !tribal_hate_cannot_reach(s.state(), &token, Some(&[Tag::Felinor][..]), None),
            "a true Felinor beside it is reached"
        );
        assert!(
            !tribal_hate_cannot_reach(s.state(), &me, Some(&[Tag::Human][..]), None),
            "harm naming another tag reaches it"
        );
    }

    mod base {
        use super::*;

        #[test]
        fn flexible_ranged_and_pierce_reach_play() {
            let s = scenario(json!({ "p1": { "field": [ID], "hand": [FILLER] }, "p2": { "hand": [FILLER] } }));
            let me = cattail(&s, P1);
            for kind in [KeywordKind::Flexible, KeywordKind::Ranged, KeywordKind::Pierce] {
                assert!(has(&s, &me, kind), "{kind:?}");
            }
            assert!(!has(&s, &me, KeywordKind::Cleave), "Cleave is the Radiant face's");
        }

        #[test]
        fn r5237009_harm_naming_felinor_passes_it_by() {
            harm_naming_felinor_passes_it_by(false);
        }

        #[test]
        fn r5237009_a_helpful_felinor_aura_reaches_it() {
            let mut s = scenario(json!({
                "p1": { "field": [ID, FLAGBEARER], "hand": [FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            let me = cattail(&s, P1);
            s.expect_stats(&me, json!({ "attack": 5 + 1, "health": 3 + 1 }));
        }

        #[test]
        fn r5237009_non_felinor_harm_passes_it_by_as_it_carries_the_tag() {
            let mut s = scenario(json!({
                "p1": { "hand": [BIG_FELINOR, FILLER], "field": [ID], "mana": 10 },
                "p2": { "hand": [FILLER], "field": [VANILLA] },
            }));
            let me = cattail(&s, P1);
            let vanilla = s.unit_at(P2, 1);
            s.play(BIG_FELINOR, json!({}));
            s.expect_in_zone(&me, "field");
            s.expect_in_zone(&vanilla, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn flexible_ranged_pierce_and_cleave_reach_play() {
            let s = scenario(json!({
                "p1": { "field": [{ "def": ID, "radiant": true }], "hand": [FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            let me = cattail(&s, P1);
            for kind in [KeywordKind::Flexible, KeywordKind::Ranged, KeywordKind::Pierce, KeywordKind::Cleave] {
                assert!(has(&s, &me, kind), "{kind:?}");
            }
        }

        #[test]
        fn r5237009_harm_naming_felinor_passes_it_by() {
            harm_naming_felinor_passes_it_by(true);
        }

        #[test]
        fn r5237009_a_helpful_felinor_aura_reaches_it() {
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": ID, "radiant": true }, FLAGBEARER], "hand": [FILLER] },
                "p2": { "hand": [FILLER] },
            }));
            let me = cattail(&s, P1);
            s.expect_stats(&me, json!({ "attack": 10 + 1, "health": 6 + 1 }));
        }
    }
}
