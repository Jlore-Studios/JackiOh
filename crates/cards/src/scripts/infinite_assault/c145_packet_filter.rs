//! SPEC §8.13 IA #145 Packet Filter ((1) Unit, Common, 1/4 → 2/8), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt"
//!   Radiant: "Taunt, Divine Shield"
//!
//! Keywords only, so the script is empty: they are the catalog's data (§6.1).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-145";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds Divine Shield and doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #145 Packet Filter — SPEC §8.13 row 145.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "stand-in (0) Spell";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": crate::filler(2) },
            "p2": { "hand": [FILLER], "library": crate::filler(2) },
        }))
    }

    fn has(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> bool {
        has_keyword(&s.stats(card).keywords, kind)
    }

    mod base {
        use super::*;

        #[test]
        fn a_1_4_taunt() {
            let mut s = board(false);
            s.play(ID, json!({ "zone": 1 }));
            let filter = s.unit_at(P1, 1);
            s.expect_stats(&filter, json!({ "attack": 1, "health": 4 }));
            assert!(has(&s, &filter, KeywordKind::Taunt));
            assert!(!has(&s, &filter, KeywordKind::DivineShield));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_2_8_taunt_with_divine_shield() {
            let mut s = board(true);
            s.play(ID, json!({ "zone": 1 }));
            let filter = s.unit_at(P1, 1);
            s.expect_stats(&filter, json!({ "attack": 2, "health": 8 }));
            assert!(has(&s, &filter, KeywordKind::Taunt));
            assert!(has(&s, &filter, KeywordKind::DivineShield));
        }
    }
}
