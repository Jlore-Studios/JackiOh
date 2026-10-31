//! SPEC §8.10 Stinkyscape 100 AP Rounds, a card of the Stinkyscape set.
//! (1) Unit, Common, 3/1 → 6/2: Starship Piece, Equip, Pierce.
//!
//! Keywords only (§6.1): its script is empty. Equip (R1086): played onto one of your Units it
//! fuses into it, the host keeping its zone and taking its stats, keywords and text (R77, R102),
//! Starship Piece only into a piece (R1852001); played alone its death fuses a copy into its
//! owner's Dormant Starship (R1852000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-100";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #100 AP Rounds — SPEC §8.10 row 100, BUILD M12 row S 100 (R346).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const ROUNDS: &str = "stinkyscape-100";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const ARMORED: &str = "core-025"; // (4) Unit 7/7, Armor 7.
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
    fn prints_pierce_3_1_radiant_6_2() {
        crate::register_all();
        let def = crate::card_def(ROUNDS);
        assert_eq!(kinds_of(&def.base.keywords), vec!["Starship Piece", "Equip", "Pierce"]);
        assert_eq!(kinds_of(&def.radiant.keywords), vec!["Starship Piece", "Equip", "Pierce"]);
        assert_eq!((def.base.attack, def.base.health), (Some(3), Some(1)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(6), Some(2)));
    }

    #[test]
    fn pierce_ignores_armor() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ROUNDS, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "field": [ARMORED], "library": filler(4) },
        }));
        s.play(ROUNDS, json!({ "zone": 1 }));
        s.end_turn();
        s.end_turn();
        // R346: the 3-attack hit ignores the 7 Armor, so the 7/7 takes 3.
        s.attack(ROUNDS, ARMORED);

        assert_eq!(s.card(ARMORED).damage, 3);
    }

    #[test]
    fn r1852001_equipped_onto_a_unit_the_host_gains_3_1_and_pierce_and_stays_no_piece() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [VANILLA, ROUNDS, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.play(VANILLA, json!({ "zone": 1 }));
        s.play(ROUNDS, json!({ "zone": 1, "equip": true }));

        let host = s.unit(PlayerId::P1, 1).expect("the fused host");
        s.expect_stats(host.id.as_str(), json!({ "attack": 7, "health": 5 }));
        let host_kinds = live_kinds(&s, host.id.as_str());
        assert!(host_kinds.contains(&"Pierce".to_string()), "{host_kinds:?}");
        // A piece equipped onto a Unit that is no piece leaves it no piece (R1852001).
        assert!(!host_kinds.contains(&"Starship Piece".to_string()), "{host_kinds:?}");
    }

    #[test]
    fn radiant_is_a_6_2_with_pierce() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ROUNDS, "radiant": true }, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.play(ROUNDS, json!({ "zone": 1 }));
        s.expect_stats(ROUNDS, json!({ "attack": 6, "health": 2 }));
        assert!(live_kinds(&s, ROUNDS).contains(&"Pierce".to_string()));
    }
}
