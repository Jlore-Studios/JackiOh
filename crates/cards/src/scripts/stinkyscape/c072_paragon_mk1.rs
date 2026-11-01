//! SPEC §8.10 Stinkyscape 72 Paragon Mk1, a card of the Stinkyscape set.
//! (3) Unit, Epic, 6/1 → 12/2: Charge, Flying, Ranged.
//! Radiant: Charge, Flying, Ranged, Pierce.
//!
//! Keywords only (§6.1): its script is empty. Since #2521 it is no Starship Piece: its death fuses
//! nothing (R1852000 reads the keyword), and S #115 Scornboi's aura exiles it.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-072";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats and adds Pierce: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #72 Paragon Mk1 — SPEC §8.10 row 72, BUILD M12 row S 72: "a 6/1 Charge Flying Ranged
// Unit that takes no strike back from a defender without Ranged (R783000); no Starship Piece, so its
// death fuses nothing (#2521); radiant 12/2 with Pierce (R346)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const PARAGON: &str = "stinkyscape-072";
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const ARMORED: &str = "core-025"; // (4) Unit 7/7, Armor 7.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    fn kinds_of(keywords: &[Keyword]) -> Vec<String> {
        keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    #[test]
    fn prints_charge_flying_ranged_6_1_radiant_12_2_with_pierce() {
        crate::register_all();
        let def = crate::card_def(PARAGON);
        assert_eq!(kinds_of(&def.base.keywords), vec!["Charge", "Flying", "Ranged"]);
        assert_eq!(
            kinds_of(&def.radiant.keywords),
            vec!["Charge", "Flying", "Ranged", "Pierce"]
        );
        assert_eq!((def.base.attack, def.base.health), (Some(6), Some(1)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(12), Some(2)));
    }

    #[test]
    fn ranged_takes_no_strike_back_from_a_defender_without_ranged() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [PARAGON, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "field": [MENACE], "library": filler(4) },
        }));
        s.play(PARAGON, json!({ "zone": 1 }));
        // R783000: the 9/9 without Ranged strikes back nothing, so the 6/1 stands undamaged.
        s.attack(PARAGON, MENACE);

        s.expect_in_zone(PARAGON, "field");
        assert_eq!(s.card(PARAGON).damage, 0);
        s.expect_health(PlayerId::P2, 30);
        assert_eq!(s.card(MENACE).damage, 6);
    }

    #[test]
    fn its_death_fuses_nothing_it_is_no_starship_piece() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "field": [PARAGON], "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        let me = s.card(PARAGON).clone();
        assert!(!has_keyword(&s.stats(&me).keywords, KeywordKind::StarshipPiece));
        s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

        s.expect_in_zone(&me, "graveyard");
        assert!(dormant_starship_of(s.state(), PlayerId::P1).is_none(), "no Dormant Starship");
        assert!(!s.events().iter().any(|event| matches!(event, GameEvent::StarshipFused { .. })));
    }

    #[test]
    fn radiant_pierce_ignores_armor() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": PARAGON, "radiant": true }, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "field": [ARMORED], "library": filler(4) },
        }));
        s.play(PARAGON, json!({ "zone": 1 }));
        // R346: the 12-attack hit ignores the 7 Armor, so the 7/7 takes 12.
        s.attack(PARAGON, ARMORED);

        s.expect_in_zone(ARMORED, "graveyard");
        s.expect_in_zone(PARAGON, "field");
        assert_eq!(s.card(PARAGON).damage, 0);
    }
}
