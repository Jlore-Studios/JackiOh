//! SPEC §8.10 Stinkyscape 101 Overdrive Module, a card of the Stinkyscape set.
//! (2) Unit, Rare, 5/5 → 10/10: Starship Piece, Equip, Brittle 2, Reborn.
//! Radiant: Starship Piece, Equip, Brittle 4, Reborn.
//!
//! Keywords only (§6.1): its script is empty. Equip (R1086): played onto one of your Units it
//! fuses into it, the host keeping its zone and taking its stats, keywords and text (R77, R102),
//! Starship Piece only into a piece (R1852001); played alone its death fuses a copy into its
//! owner's Dormant Starship (R1852000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-101";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats and lengthens the Brittle: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #101 Overdrive Module — SPEC §8.10 row 101, BUILD M12 row S 101 (R1852000, §6.1).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const MODULE: &str = "stinkyscape-101";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
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
    fn prints_brittle_2_reborn_5_5_radiant_brittle_4_10_10() {
        crate::register_all();
        let def = crate::card_def(MODULE);
        assert_eq!(
            kinds_of(&def.base.keywords),
            vec!["Starship Piece", "Equip", "Brittle", "Reborn"]
        );
        assert_eq!(
            kinds_of(&def.radiant.keywords),
            vec!["Starship Piece", "Equip", "Brittle", "Reborn"]
        );
        assert!(def.base.keywords.contains(&Keyword::Brittle { n: 2 }));
        assert!(def.radiant.keywords.contains(&Keyword::Brittle { n: 4 }));
        assert_eq!((def.base.attack, def.base.health), (Some(5), Some(5)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(10), Some(10)));
    }

    #[test]
    fn reborn_returns_it_at_1_health_with_brittle_2_ticking() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "field": [MODULE], "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        // The printed Brittle starts when it enters the field (§6.1).
        assert_eq!(s.card(MODULE).brittle.as_ref().map(|brittle| brittle.count), Some(2));
        let me = s.card(MODULE).clone();
        s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

        // Reborn returns it at 1 health; its death still fused a copy (R1852000).
        let back = s.unit(PlayerId::P1, 1).expect("reborn at 1 health");
        assert_eq!(back.def_id, MODULE);
        s.expect_stats(back.id.as_str(), json!({ "attack": 5, "health": 1, "maxHealth": 5 }));
        assert!(dormant_starship_of(s.state(), PlayerId::P1).is_some());
    }

    #[test]
    fn r1852001_equipped_onto_a_unit_the_host_gains_5_5_and_reborn_and_stays_no_piece() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [VANILLA, MODULE, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.play(VANILLA, json!({ "zone": 1 }));
        s.play(MODULE, json!({ "zone": 1, "equip": true }));

        let host = s.unit(PlayerId::P1, 1).expect("the fused host");
        s.expect_stats(host.id.as_str(), json!({ "attack": 9, "health": 9 }));
        let host_kinds = live_kinds(&s, host.id.as_str());
        assert!(host_kinds.contains(&"Reborn".to_string()), "{host_kinds:?}");
        // A piece equipped onto a Unit that is no piece leaves it no piece (R1852001).
        assert!(!host_kinds.contains(&"Starship Piece".to_string()), "{host_kinds:?}");
    }

    #[test]
    fn radiant_is_a_10_10_with_brittle_4_and_reborn() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": MODULE, "radiant": true }, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.play(MODULE, json!({ "zone": 1 }));
        s.expect_stats(MODULE, json!({ "attack": 10, "health": 10 }));
        assert_eq!(s.card(MODULE).brittle.as_ref().map(|brittle| brittle.count), Some(4));
        assert!(live_kinds(&s, MODULE).contains(&"Reborn".to_string()));
    }
}
