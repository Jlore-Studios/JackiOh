//! SPEC §8.10 Stinkyscape 73 Paragon Mk2, a card of the Stinkyscape set.
//! (4) Unit, Legendary, 7/1 → 14/2: Charge, Flying, Ranged, Divine Shield.
//! Radiant: Charge, Flying, Ranged, Divine Shield, Pierce.
//!
//! Keywords only (§6.1): its script is empty. Since #2521 it is no Starship Piece: its death fuses
//! nothing (R1852000 reads the keyword).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-073";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats and adds Pierce: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #73 Paragon Mk2 — SPEC §8.10 row 73, BUILD M12 row S 73: "a 7/1 Charge Flying Ranged
// Divine Shield Unit; its Divine Shield takes the first hit; no Starship Piece, so its death fuses
// nothing (#2521); radiant 14/2 with Pierce (R346)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const PARAGON: &str = "stinkyscape-073";
    const STRATOS: &str = "stinkyscape-068"; // (1) 3/1 Flying, Ranged.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
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
    fn prints_divine_shield_7_1_radiant_14_2_with_pierce() {
        crate::register_all();
        let def = crate::card_def(PARAGON);
        assert_eq!(
            kinds_of(&def.base.keywords),
            vec!["Charge", "Flying", "Ranged", "Divine Shield"]
        );
        assert_eq!(
            kinds_of(&def.radiant.keywords),
            vec!["Charge", "Flying", "Ranged", "Divine Shield", "Pierce"]
        );
        assert_eq!((def.base.attack, def.base.health), (Some(7), Some(1)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(14), Some(2)));
    }

    #[test]
    fn divine_shield_takes_the_first_hit_then_ranged_takes_no_strike_back() {
        crate::register_all();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [ANCHOR], "field": [PARAGON], "library": filler(4) },
            "p2": { "hand": [ANCHOR], "field": [STRATOS, MENACE], "library": filler(4) },
        }));
        // A Unit without Flying or Ranged cannot attack it (R783002); a Flying one can, and its
        // 3-attack hit takes the Divine Shield instead: the Paragon stands undamaged, while the 3/1
        // Stratos dies to the 7-attack strike back (the Paragon has Ranged, so it strikes back a
        // Ranged attacker, R783000).
        s.expect_refused(|s| s.attack(MENACE, PARAGON));
        s.attack(STRATOS, PARAGON);
        s.expect_in_zone(PARAGON, "field");
        assert_eq!(s.card(PARAGON).damage, 0);
        assert!(!live_kinds(&s, PARAGON).contains(&"Divine Shield".to_string()));
        s.expect_in_zone(STRATOS, "graveyard");

        // With the shield gone, Ranged still takes no strike back from the 9/9 (R783000).
        s.end_turn();
        s.attack(PARAGON, MENACE);
        s.expect_in_zone(PARAGON, "field");
        assert_eq!(s.card(PARAGON).damage, 0);
        assert_eq!(s.card(MENACE).damage, 7);
    }

    #[test]
    fn its_death_fuses_nothing_it_is_no_starship_piece() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "field": [PARAGON], "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        let me = s.card(PARAGON).clone();
        assert!(!live_kinds(&s, PARAGON).contains(&"Starship Piece".to_string()));
        // A destroy is no damage: the Divine Shield does not stop it.
        s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

        s.expect_in_zone(&me, "graveyard");
        assert!(dormant_starship_of(s.state(), PlayerId::P1).is_none(), "no Dormant Starship");
        assert!(!s.events().iter().any(|event| matches!(event, GameEvent::StarshipFused { .. })));
    }

    #[test]
    fn radiant_is_a_14_2_with_pierce() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": PARAGON, "radiant": true }, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.play(PARAGON, json!({ "zone": 1 }));
        s.expect_stats(PARAGON, json!({ "attack": 14, "health": 2 }));
        assert!(live_kinds(&s, PARAGON).contains(&"Pierce".to_string()));
    }
}
