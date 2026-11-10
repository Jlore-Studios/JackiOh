//! SPEC §8.12 S+ #27.1 Imp (Unit token, Swarm 1/1 → 2/2, cost 1, printed Common), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Can't Defend, Charge"
//!   Radiant: "Can't Defend, Charge, Pierce"
//!
//! Keywords only, so the script is empty: they are the catalog's data (§6.1). Can't Defend keeps it in
//! Attack Position (R2657000) and Charge lets it attack Units and heroes on the turn it lands. It is the
//! token S+ #27 Gargantuar's Evolve summons and S+ #28 Zombie-Box's Gargantuar after it.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-027-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds Pierce and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #27.1 Imp — SPEC §8.12 row 27.1.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "stand-in (0) Spell";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    fn has(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> bool {
        has_keyword(&s.stats(card).keywords, kind)
    }

    mod base {
        use super::*;

        #[test]
        fn is_a_1_1_that_cant_defend_and_charges() {
            let mut s = board(false);
            s.play(ID, json!({}));
            let imp = s.unit_at(P1, 1);
            s.expect_stats(&imp, json!({ "attack": 1, "health": 1 }));
            assert!(has(&s, &imp, KeywordKind::CantDefend));
            assert!(has(&s, &imp, KeywordKind::Charge));
            assert!(!has(&s, &imp, KeywordKind::Pierce));
        }

        #[test]
        fn a_played_imp_attacks_the_hero_the_turn_it_lands() {
            let mut s = board(false);
            s.play(ID, json!({}));
            let imp = s.unit_at(P1, 1);
            s.attack(&imp, "hero");
            s.expect_health(P2, HERO_HEALTH - 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_a_2_2_with_pierce_added() {
            let mut s = board(true);
            s.play(ID, json!({}));
            let imp = s.unit_at(P1, 1);
            s.expect_stats(&imp, json!({ "attack": 2, "health": 2 }));
            assert!(has(&s, &imp, KeywordKind::CantDefend));
            assert!(has(&s, &imp, KeywordKind::Charge));
            assert!(has(&s, &imp, KeywordKind::Pierce));
        }

        #[test]
        fn a_played_imp_attacks_the_hero_the_turn_it_lands() {
            let mut s = board(true);
            s.play(ID, json!({}));
            let imp = s.unit_at(P1, 1);
            s.attack(&imp, "hero");
            s.expect_health(P2, HERO_HEALTH - 2);
        }
    }
}
