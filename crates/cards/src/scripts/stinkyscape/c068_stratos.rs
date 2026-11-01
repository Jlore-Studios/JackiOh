//! SPEC §8.10 Stinkyscape 68 Stratos, a card of the Stinkyscape set.
//! (1) Unit, Common, 3/1 → 6/2: Flying, Ranged.
//!
//! Keywords only (§6.1): its script is empty. Flying: only a Flying or Ranged Unit in Attack Position
//! may attack it (R783002); Ranged: it takes no strike back from a defender without Ranged (R783000).
//! Since #2521 it is no Starship Piece: its death fuses nothing (R1852000 reads the keyword).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-068";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #68 Stratos — SPEC §8.10 row 68, BUILD M12 row S 68 (R783002, R783000, #2521).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const STRATOS: &str = "stinkyscape-068";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9 Taunt.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    fn kinds_of(keywords: &[Keyword]) -> Vec<String> {
        keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    #[test]
    fn prints_flying_and_ranged_3_1_radiant_6_2() {
        crate::register_all();
        let def = crate::card_def(STRATOS);
        assert_eq!(kinds_of(&def.base.keywords), vec!["Flying", "Ranged"]);
        assert_eq!(kinds_of(&def.radiant.keywords), vec!["Flying", "Ranged"]);
        assert_eq!((def.base.attack, def.base.health), (Some(3), Some(1)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(6), Some(2)));
    }

    #[test]
    fn flying_a_unit_without_flying_or_ranged_cannot_attack_it() {
        crate::register_all();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [ANCHOR], "field": [STRATOS], "library": filler(4) },
            "p2": { "hand": [ANCHOR], "field": [VANILLA], "library": filler(4) },
        }));
        // R783002: only a Flying or Ranged Unit may attack it.
        s.expect_refused(|s| s.attack(VANILLA, STRATOS));
        s.expect_in_zone(STRATOS, "field");
    }

    #[test]
    fn ranged_takes_no_strike_back_from_a_defender_without_ranged() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [ANCHOR], "field": [STRATOS], "library": filler(4) },
            "p2": { "hand": [ANCHOR], "field": [MENACE], "library": filler(4) },
        }));
        // R783000: the 9/9 without Ranged strikes back nothing, so the 3/1 stands undamaged.
        s.attack(STRATOS, MENACE);

        s.expect_in_zone(STRATOS, "field");
        assert_eq!(s.card(STRATOS).damage, 0);
        assert_eq!(s.card(MENACE).damage, 3);
    }

    #[test]
    fn its_death_fuses_nothing_it_is_no_starship_piece() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "field": [STRATOS], "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        let me = s.card(STRATOS).clone();
        assert!(!has_keyword(&s.stats(&me).keywords, KeywordKind::StarshipPiece));
        s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

        s.expect_in_zone(&me, "graveyard");
        assert!(
            dormant_starship_of(s.state(), PlayerId::P1).is_none(),
            "no piece died, so no Dormant Starship"
        );
        assert!(!s.events().iter().any(|event| matches!(event, GameEvent::StarshipFused { .. })));
    }

    #[test]
    fn radiant_is_a_6_2_with_the_same_keywords() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": STRATOS, "radiant": true }, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.play(STRATOS, json!({ "zone": 1 }));
        s.expect_stats(STRATOS, json!({ "attack": 6, "health": 2 }));
        let live = kinds_of(&s.stats(STRATOS).keywords);
        assert!(live.contains(&"Flying".to_string()) && live.contains(&"Ranged".to_string()), "{live:?}");
    }
}
