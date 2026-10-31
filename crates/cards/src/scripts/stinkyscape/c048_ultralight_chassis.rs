//! SPEC §8.10 Stinkyscape 48 Ultralight Chassis, a card of the Stinkyscape set.
//! (1) Unit, Common, 4/1 → 8/2: Starship Piece, Equip, Rush.
//!
//! Keywords only (§6.1): its script is empty. Equip (R1086): played onto one of your Units it
//! fuses into it, the host keeping its zone and taking its stats, keywords and text (R77, R102),
//! Starship Piece only into a piece (R1852001); played alone its death fuses a copy into its
//! owner's Dormant Starship (R1852000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-048";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #48 Ultralight Chassis — SPEC §8.10 row 48, BUILD M12 row S 48 (R1852000, R1852001).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const CHASSIS: &str = "stinkyscape-048";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    fn kinds(s: &Scenario, card: &str) -> Vec<String> {
        s.stats(card)
            .keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    #[test]
    fn prints_rush_equip_and_starship_piece_4_1_radiant_8_2() {
        crate::register_all();
        let def = crate::card_def(CHASSIS);
        assert_eq!(
            kinds_of(&def.base.keywords),
            vec!["Starship Piece", "Equip", "Rush"]
        );
        assert_eq!(
            kinds_of(&def.radiant.keywords),
            vec!["Starship Piece", "Equip", "Rush"]
        );
        assert_eq!((def.base.attack, def.base.health), (Some(4), Some(1)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(8), Some(2)));
    }

    fn kinds_of(keywords: &[Keyword]) -> Vec<String> {
        keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    #[test]
    fn rush_may_attack_a_unit_the_turn_it_is_played_but_not_the_hero() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [CHASSIS, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "field": [VANILLA], "library": filler(4) },
        }));
        s.play(CHASSIS, json!({ "zone": 1 }));

        // §4.2: Rush cannot hit the hero on its summon turn, but it may hit a unit.
        s.expect_refused(|s| s.attack(CHASSIS, "hero"));
        s.attack(CHASSIS, VANILLA);
        s.expect_in_zone(VANILLA, "graveyard");
        s.expect_in_zone(CHASSIS, "graveyard");
    }

    #[test]
    fn death_fuses_a_copy_into_the_dormant_starship() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "field": [CHASSIS], "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        let me = s.card(CHASSIS).clone();
        s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

        s.expect_in_zone(CHASSIS, "graveyard");
        let ship = dormant_starship_of(s.state(), PlayerId::P1).cloned();
        assert!(ship.is_some(), "a piece that died fuses a copy into a Dormant Starship");
        assert_eq!(
            starship_pieces_of(&ship.expect("the ship"))
                .iter()
                .map(|piece| piece.def_id.as_str())
                .collect::<Vec<_>>(),
            vec![CHASSIS]
        );
    }

    #[test]
    fn r1852001_equipped_onto_a_unit_the_host_gains_4_1_and_rush_and_stays_no_piece() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [VANILLA, CHASSIS, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.play(VANILLA, json!({ "zone": 1 }));
        s.play(CHASSIS, json!({ "zone": 1, "equip": true }));

        let host = s.unit(PlayerId::P1, 1).expect("the fused host");
        s.expect_stats(host.id.as_str(), json!({ "attack": 8, "health": 5 }));
        let host_kinds = kinds(&s, host.id.as_str());
        assert!(host_kinds.contains(&"Rush".to_string()), "{host_kinds:?}");
        // A piece equipped onto a Unit that is no piece leaves it no piece (R1852001).
        assert!(!host_kinds.contains(&"Starship Piece".to_string()), "{host_kinds:?}");
    }
}
