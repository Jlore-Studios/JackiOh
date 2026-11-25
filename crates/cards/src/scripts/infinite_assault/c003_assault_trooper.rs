//! SPEC §8.13 IA #3 Assault Trooper ((1) Unit, Common, 3/2 → 6/4), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Rush"
//!   Radiant: "Rush, First Strike"
//!
//! Keywords only, so the script is empty: they are the catalog's data (§6.1).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-003";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds First Strike and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #3 Assault Trooper — SPEC §8.13 row 3.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const TARGET: &str = "stand-in (1) 2/5";
    const FILLER: &str = "stand-in (0) Spell";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": crate::filler(2) },
            "p2": { "hand": [FILLER], "field": [TARGET], "library": crate::filler(2) },
        }))
    }

    fn has(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> bool {
        has_keyword(&s.stats(card).keywords, kind)
    }

    mod base {
        use super::*;

        #[test]
        fn rush_attacks_a_unit_the_turn_it_lands() {
            let mut s = board(false);
            s.play(ID, json!({}));
            let trooper = s.unit_at(P1, 1);
            assert!(has(&s, &trooper, KeywordKind::Rush));
            assert!(!has(&s, &trooper, KeywordKind::FirstStrike));
            let target = s.unit_at(P2, 1);
            s.attack(&trooper, &target);
            s.expect_stats(&target, json!({ "health": 2 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn rush_and_first_strike_take_no_hit_back_from_what_it_kills() {
            let mut s = board(true);
            s.play(ID, json!({}));
            let trooper = s.unit_at(P1, 1);
            assert!(has(&s, &trooper, KeywordKind::Rush));
            assert!(has(&s, &trooper, KeywordKind::FirstStrike));
            let target = s.unit_at(P2, 1);
            s.attack(&trooper, &target);
            s.expect_in_zone(&target, "graveyard");
            s.expect_stats(&trooper, json!({ "attack": 6, "health": 4 }));
        }
    }
}
