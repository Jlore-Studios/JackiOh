//! SPEC §8.14 FvH #11 Ban Hammer ((1) Unit, Rare, 1/1 → 2/2), a card of the held Felinors vs Humans set
//! (R5235000).
//!   Base:    "Equip, Poisonous"
//!   Radiant: "Equip, Rush, Poisonous"
//!
//! Engine: every word is a catalog keyword (§6.1), so there is no script to run. Equip (R1086) offers the
//! play onto one of its controller's acting, non-Immutable Units, where it resolves on top and fuses into
//! the host (R102): +1/+1 and Poisonous (Radiant +2/+2, Rush and Poisonous). Played into a zone of its own
//! it is a plain Poisonous Unit. Declares no numbers. Built by S01 (#5251).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-011";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face's Rush and doubled line are catalog data, which the Equip fusion reads.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// FvH #11 Ban Hammer — SPEC §8.14 row 11 (R1086, R102).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const HOST: &str = "stand-in (1) 3/3";
    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn holding(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": field,
                "library": filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn has(s: &Scenario, lane: i32, kind: KeywordKind) -> bool {
        has_keyword(&s.stats(s.unit_at(P1, lane)).keywords, kind)
    }

    #[test]
    fn declares_no_tuned_numbers() {
        crate::register_all();
        assert!(crate::card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r1086_equipped_onto_a_unit_it_fuses_its_line_and_poisonous_into_the_host() {
            let mut s = holding(false, json!([HOST]));
            let host = s.unit_at(P1, 1);
            s.play(ID, json!({ "zone": 1, "equip": true }));
            let fused = s.unit_at(P1, 1);
            assert_eq!(fused.id, host.id, "the host keeps its zone");
            s.expect_stats(&fused, json!({ "attack": 4, "health": 4 }));
            assert!(has(&s, 1, KeywordKind::Poisonous));
            assert!(!has(&s, 1, KeywordKind::Rush), "the base face gives no Rush");
        }

        #[test]
        fn played_into_its_own_zone_it_is_a_poisonous_unit() {
            let mut s = holding(false, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            assert_eq!(s.unit_at(P1, 1).def_id, ID);
            assert!(has(&s, 1, KeywordKind::Poisonous));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1086_equipped_onto_a_unit_it_fuses_two_two_rush_and_poisonous_into_the_host() {
            let mut s = holding(true, json!([HOST]));
            let host = s.unit_at(P1, 1);
            s.play(ID, json!({ "zone": 1, "equip": true }));
            let fused = s.unit_at(P1, 1);
            assert_eq!(fused.id, host.id, "the host keeps its zone");
            s.expect_stats(&fused, json!({ "attack": 5, "health": 5 }));
            assert!(has(&s, 1, KeywordKind::Poisonous));
            assert!(has(&s, 1, KeywordKind::Rush));
        }

        #[test]
        fn played_into_its_own_zone_it_is_a_rush_poisonous_unit() {
            let mut s = holding(true, json!([]));
            s.play(ID, json!({ "zone": 1 }));
            assert!(has(&s, 1, KeywordKind::Poisonous));
            assert!(has(&s, 1, KeywordKind::Rush));
        }
    }
}
