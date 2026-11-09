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

// Stinkyscape #72 Paragon Mk1 — SPEC §8.10 row 72, BUILD M12 row S 72 (R783000, #2521, R346).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const PARAGON: &str = "stinkyscape-072";
    const MENACE: &str = "core-019"; // Midrange Menace: a Unit, Taunt
    const ARMORED: &str = "stand-in (4) 7/7 Armor 7 R14/14 Armor 7, Reborn";
    const HIT_JOB: &str = "core-016"; // Hit Job: Destroy a Unit.
    const ANCHOR: &str = "core-010"; // Rapid Replenish: Spell (§2.5).

    #[test]
    fn ranged_takes_no_strike_back_from_a_defender_without_ranged() {
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
