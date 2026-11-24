//! SPEC §8.13 IA #154.1 Decoy Server ((2) Unit token, 0/8 → 0/16), Honeypot Protocol's token,
//! a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt"
//!   Radiant: "Taunt"
//!
//! Keywords only, so the script is empty: they are the catalog's data (§6.1).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-154-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #154.1 Decoy Server — SPEC §8.13 row 154.1.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "stand-in (0) Spell";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "field": [{ "def": ID, "radiant": radiant }], "library": crate::filler(2) },
            "p2": { "hand": [FILLER], "library": crate::filler(2) },
        }))
    }

    fn has(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> bool {
        has_keyword(&s.stats(card).keywords, kind)
    }

    mod base {
        use super::*;

        #[test]
        fn a_0_8_taunt() {
            let mut s = board(false);
            let decoy = s.unit_at(P1, 1);
            s.expect_stats(&decoy, json!({ "attack": 0, "health": 8 }));
            assert!(has(&s, &decoy, KeywordKind::Taunt));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn a_0_16_taunt() {
            let mut s = board(true);
            let decoy = s.unit_at(P1, 1);
            s.expect_stats(&decoy, json!({ "attack": 0, "health": 16 }));
            assert!(has(&s, &decoy, KeywordKind::Taunt));
        }
    }
}
