//! SPEC §8.10 Stinkyscape 69 Falcon, a card of the Stinkyscape set.
//! (2) Unit, Rare, 4/1 → 8/2: Charge, Flying.
//! Radiant: Charge, Flying, Pierce (8/2, ST9: Radiant stats doubled).
//!
//! Keywords only (§6.1): its script is empty. Since #2521 it is no Starship Piece: its death fuses
//! nothing (R1852000 reads the keyword).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-069";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats and adds Pierce: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #69 Falcon — SPEC §8.10 row 69, BUILD M12 row S 69 (#2521, R346).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const FALCON: &str = "stinkyscape-069";
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
    fn prints_charge_flying_4_1_radiant_8_2_with_pierce() {
        crate::register_all();
        let def = crate::card_def(FALCON);
        assert_eq!(kinds_of(&def.base.keywords), vec!["Charge", "Flying"]);
        assert_eq!(kinds_of(&def.radiant.keywords), vec!["Charge", "Flying", "Pierce"]);
        assert_eq!((def.base.attack, def.base.health), (Some(4), Some(1)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(8), Some(2)));
    }

    #[test]
    fn charge_may_attack_the_hero_the_turn_it_is_played() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [FALCON, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.play(FALCON, json!({ "zone": 1 }));
        s.attack(FALCON, "hero");

        s.expect_health(PlayerId::P2, 26);
    }

    #[test]
    fn its_death_fuses_nothing_it_is_no_starship_piece() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "field": [FALCON], "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        let me = s.card(FALCON).clone();
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
            "p1": { "hand": [{ "def": FALCON, "radiant": true }, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "field": [ARMORED], "library": filler(4) },
        }));
        s.play(FALCON, json!({ "zone": 1 }));
        // R346: the 8-attack hit ignores the 7 Armor, so the 7/7 takes 8.
        s.attack(FALCON, ARMORED);

        s.expect_in_zone(ARMORED, "graveyard");
        s.expect_in_zone(FALCON, "graveyard");
    }
}
