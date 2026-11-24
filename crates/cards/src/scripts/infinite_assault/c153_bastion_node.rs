//! SPEC §8.13 IA #153 Bastion Node ((4) Unit, Common, 4/12 → 8/24), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Taunt, Armor 3"
//!   Radiant: "Taunt, Armor 6, Immune to Spells"
//!
//! Keywords only, so the script is empty: they are the catalog's data (§6.1).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-153";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face adds Immune to Spells and doubles the stats and the Armor: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #153 Bastion Node — SPEC §8.13 row 153.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "stand-in (0) Spell";
    const HITTER: &str = "stand-in (2) 4/4";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": crate::filler(2), "mana": 8 },
            "p2": { "hand": [FILLER], "field": [HITTER], "library": crate::filler(2) },
        }))
    }

    fn has(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> bool {
        has_keyword(&s.stats(card).keywords, kind)
    }

    mod base {
        use super::*;

        #[test]
        fn a_4_12_taunt_that_armors_3() {
            let mut s = board(false);
            s.play(ID, json!({ "zone": 1 }));
            let node = s.unit_at(P1, 1);
            s.expect_stats(&node, json!({ "attack": 4, "health": 12 }));
            assert!(!has(&s, &node, KeywordKind::ImmuneToSpells));
            s.end_turn();
            let hitter = s.unit_at(P2, 1);
            s.attack(&hitter, &node);
            // 4 attack past 3 Armor: 1 lands.
            s.expect_stats(&node, json!({ "attack": 4, "health": 11 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn an_8_24_taunt_that_armors_6_and_is_immune_to_spells() {
            let mut s = board(true);
            s.play(ID, json!({ "zone": 1 }));
            let node = s.unit_at(P1, 1);
            s.expect_stats(&node, json!({ "attack": 8, "health": 24 }));
            assert!(has(&s, &node, KeywordKind::ImmuneToSpells));
            s.end_turn();
            let hitter = s.unit_at(P2, 1);
            s.attack(&hitter, &node);
            // 4 attack past 6 Armor: nothing lands.
            s.expect_stats(&node, json!({ "attack": 8, "health": 24 }));
        }
    }
}
