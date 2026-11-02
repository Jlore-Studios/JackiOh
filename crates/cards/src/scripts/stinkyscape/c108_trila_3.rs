//! SPEC §8.10 Stinkyscape 108 Trila 3, a card of the Stinkyscape set.
//! (3) Unit, Common, 7/7 → 14/14: Starship Piece, Ranged, Windfury.
//! Radiant: Starship Piece, Ranged, Rush, Windfury.
//!
//! Keywords only (§6.1): its script is empty. Starship Piece: its death fuses a copy into its
//! owner's Dormant Starship, summoned first if they have none (R1852000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-108";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats and adds Rush: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #108 Trila 3 — SPEC §8.10 row 108, BUILD M12 row S 108 (R783000).
#[cfg(test)]
mod tests {
    use super::*;

    use crate::scenario;
    use crate::filler;

    const TRILA: &str = "stinkyscape-108";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    fn kinds_of(keywords: &[Keyword]) -> Vec<String> {
        keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    #[test]
    fn prints_ranged_windfury_7_7_radiant_14_14_with_rush() {
        crate::register_all();
        let def = crate::card_def(TRILA);
        assert_eq!(kinds_of(&def.base.keywords), vec!["Starship Piece", "Ranged", "Windfury"]);
        assert_eq!(
            kinds_of(&def.radiant.keywords),
            vec!["Starship Piece", "Ranged", "Rush", "Windfury"]
        );
        assert_eq!((def.base.attack, def.base.health), (Some(7), Some(7)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(14), Some(14)));
    }

    #[test]
    fn windfury_attacks_twice_a_turn() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ANCHOR], "field": [TRILA], "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.attack(TRILA, "hero");
        s.expect_health(PlayerId::P2, 23);
        s.attack(TRILA, "hero");
        s.expect_health(PlayerId::P2, 16);
    }

    #[test]
    fn radiant_rush_may_attack_a_unit_the_turn_it_is_played() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": TRILA, "radiant": true }, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "field": [VANILLA], "library": filler(4) },
        }));
        s.play(TRILA, json!({ "zone": 1 }));
        s.expect_refused(|s| s.attack(TRILA, "hero"));
        s.attack(TRILA, VANILLA);
        s.expect_in_zone(VANILLA, "graveyard");
    }
}
