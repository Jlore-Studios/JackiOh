//! SPEC §8.10 Stinkyscape 122 Vamp Rounds, a card of the Stinkyscape set.
//! (1) Unit, Common, 2/1 → 4/2: Starship Piece, Equip, Lifesteal.
//!
//! Keywords only (§6.1): its script is empty. Equip (R1086): played onto one of your Units it
//! fuses into it, the host keeping its zone and taking its stats, keywords and text (R77, R102),
//! Starship Piece only into a piece (R1852001); played alone its death fuses a copy into its
//! owner's Dormant Starship (R1852000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-122";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #122 Vamp Rounds — SPEC §8.10 row 122, BUILD M12 row S 122.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const VAMP: &str = "stinkyscape-122";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
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
    fn prints_lifesteal_2_1_radiant_4_2() {
        crate::register_all();
        let def = crate::card_def(VAMP);
        assert_eq!(kinds_of(&def.base.keywords), vec!["Starship Piece", "Equip", "Lifesteal"]);
        assert_eq!(kinds_of(&def.radiant.keywords), vec!["Starship Piece", "Equip", "Lifesteal"]);
        assert_eq!((def.base.attack, def.base.health), (Some(2), Some(1)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(4), Some(2)));
    }

    #[test]
    fn lifesteal_heals_its_controller_s_hero_what_it_dealt() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [VAMP, ANCHOR], "mana": 10, "health": 20, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "field": [VANILLA], "library": filler(4) },
        }));
        s.play(VAMP, json!({ "zone": 1 }));
        // Its 2 damage heals its
        // controller's hero 2 (§4.4 step 8), though it dies to the 4-attack strike back.
        s.end_turn();
        s.end_turn();
        s.attack(VAMP, VANILLA);

        s.expect_health(PlayerId::P1, 22);
        s.expect_in_zone(VAMP, "graveyard");
    }

    #[test]
    fn r1852001_equipped_onto_a_unit_the_host_gains_2_1_and_lifesteal_and_stays_no_piece() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [VANILLA, VAMP, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.play(VANILLA, json!({ "zone": 1 }));
        s.play(VAMP, json!({ "zone": 1, "equip": true }));

        let host = s.unit(PlayerId::P1, 1).expect("the fused host");
        s.expect_stats(host.id.as_str(), json!({ "attack": 6, "health": 5 }));
        let host_kinds = live_kinds(&s, host.id.as_str());
        assert!(host_kinds.contains(&"Lifesteal".to_string()), "{host_kinds:?}");
        // A piece equipped onto a Unit that is no piece leaves it no piece (R1852001).
        assert!(!host_kinds.contains(&"Starship Piece".to_string()), "{host_kinds:?}");
    }

    #[test]
    fn radiant_is_a_4_2_with_lifesteal() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": VAMP, "radiant": true }, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.play(VAMP, json!({ "zone": 1 }));
        s.expect_stats(VAMP, json!({ "attack": 4, "health": 2 }));
        assert!(live_kinds(&s, VAMP).contains(&"Lifesteal".to_string()));
    }
}
