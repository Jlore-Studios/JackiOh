//! SPEC §8.10 Stinkyscape 98 Hull Booster, a card of the Stinkyscape set.
//! (1) Unit, Common, 1/1 → 2/2: Starship Piece, Equip, Armor 2 (Radiant Armor 4).
//!
//! Keywords only (§6.1): its script is empty. Equip (R1086): played onto one of your Units it
//! fuses into it, the host keeping its zone and taking its stats, keywords and text (R77, R102),
//! Starship Piece only into a piece (R1852001); played alone its death fuses a copy into its
//! owner's Dormant Starship (R1852000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-098";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats and the Armor: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #98 Hull Booster — SPEC §8.10 row 98, BUILD M12 row S 98 (§6.1).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const BOOSTER: &str = "stinkyscape-098";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const WEAK: &str = "core-007"; // (1) Unit 1/1.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    fn kinds_of(keywords: &[Keyword]) -> Vec<String> {
        keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    fn live_kinds(s: &Scenario, card: &str) -> Vec<String> {
        s.stats(card)
            .keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    #[test]
    fn prints_armor_2_1_1_radiant_armor_4_2_2() {
        crate::register_all();
        let def = crate::card_def(BOOSTER);
        assert_eq!(kinds_of(&def.base.keywords), vec!["Starship Piece", "Equip", "Armor"]);
        assert_eq!(kinds_of(&def.radiant.keywords), vec!["Starship Piece", "Equip", "Armor"]);
        assert!(def.base.keywords.contains(&Keyword::Armor { n: 2 }));
        assert!(def.radiant.keywords.contains(&Keyword::Armor { n: 4 }));
        assert_eq!((def.base.attack, def.base.health), (Some(1), Some(1)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(2), Some(2)));
    }

    #[test]
    fn armor_makes_each_hit_on_it_2_less() {
        crate::register_all();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [ANCHOR], "field": [BOOSTER], "library": filler(4) },
            "p2": { "hand": [ANCHOR], "field": [WEAK], "library": filler(4) },
        }));
        // A 1-attack hit through Armor 2 is nothing (R63): the 1/1 stands undamaged.
        s.attack(WEAK, BOOSTER);
        s.expect_in_zone(BOOSTER, "field");
        assert_eq!(s.card(BOOSTER).damage, 0);
    }

    #[test]
    fn r1852001_equipped_onto_a_unit_the_host_gains_1_1_and_armor_2_and_stays_no_piece() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [VANILLA, BOOSTER, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.play(VANILLA, json!({ "zone": 1 }));
        s.play(BOOSTER, json!({ "zone": 1, "equip": true }));

        let host = s.unit(PlayerId::P1, 1).expect("the fused host");
        s.expect_stats(host.id.as_str(), json!({ "attack": 5, "health": 5 }));
        assert_eq!(s.stats(host.id.as_str()).armor, 2);
        let host_kinds = live_kinds(&s, host.id.as_str());
        assert!(host_kinds.contains(&"Armor".to_string()), "{host_kinds:?}");
        // A piece equipped onto a Unit that is no piece leaves it no piece (R1852001).
        assert!(!host_kinds.contains(&"Starship Piece".to_string()), "{host_kinds:?}");
    }

    #[test]
    fn radiant_is_a_2_2_with_armor_4() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": BOOSTER, "radiant": true }, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.play(BOOSTER, json!({ "zone": 1 }));
        s.expect_stats(BOOSTER, json!({ "attack": 2, "health": 2 }));
        assert_eq!(s.stats(BOOSTER).armor, 4);
        assert!(live_kinds(&s, BOOSTER).contains(&"Armor".to_string()));
    }
}
