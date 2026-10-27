//! SPEC §8.10 Stinkyscape 68 Startos, a card of the held Stinkyscape set (R1420).
//! (1) Unit, Common, 3/3 → 6/6: Starship Piece, Flying.
//!
//! Keywords only (§6.1): its script is empty. Starship Piece: its death fuses a copy into its
//! owner's Dormant Starship, summoned first if they have none (R1852000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-068";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face doubles the stats: catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// Stinkyscape #68 Startos — SPEC §8.10 row 68, BUILD M12 row S 68: "a vanilla 3/3 Flying Starship
// Piece: attacked only by Flying or Ranged Units in Attack Position (R783002); its death fuses a
// copy into its owner's Dormant Starship (R1852000); radiant 6/6".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::scenario;
    use crate::filler;

    const STARTOS: &str = "stinkyscape-068";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    fn open() -> PreviewGuard {
        preview_sets(&[SetName::Stinkyscape])
    }

    fn kinds_of(keywords: &[Keyword]) -> Vec<String> {
        keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    #[test]
    fn prints_starship_piece_and_flying_3_3_radiant_6_6() {
        crate::register_all();
        let def = crate::card_def(STARTOS);
        assert_eq!(kinds_of(&def.base.keywords), vec!["Starship Piece", "Flying"]);
        assert_eq!(kinds_of(&def.radiant.keywords), vec!["Starship Piece", "Flying"]);
        assert_eq!((def.base.attack, def.base.health), (Some(3), Some(3)));
        assert_eq!((def.radiant.attack, def.radiant.health), (Some(6), Some(6)));
    }

    #[test]
    fn flying_a_unit_without_flying_or_ranged_cannot_attack_it() {
        crate::register_all();
        let _preview = open();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "hand": [ANCHOR], "field": [STARTOS], "library": filler(4) },
            "p2": { "hand": [ANCHOR], "field": [VANILLA], "library": filler(4) },
        }));
        // R783002: only a Flying or Ranged Unit may attack it.
        s.expect_refused(|s| s.attack(VANILLA, STARTOS));
        s.expect_in_zone(STARTOS, "field");
    }

    #[test]
    fn death_fuses_a_copy_into_the_dormant_starship() {
        crate::register_all();
        let _preview = open();
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB, ANCHOR], "mana": 10, "field": [STARTOS], "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        let me = s.card(STARTOS).clone();
        s.play(HIT_JOB, json!({ "targets": pick::instance(&me) }));

        s.expect_in_zone(STARTOS, "graveyard");
        let ship = dormant_starship_of(s.state(), PlayerId::P1).cloned();
        assert!(ship.is_some(), "a piece that died fuses a copy into a Dormant Starship");
        assert_eq!(
            starship_pieces_of(&ship.expect("the ship"))
                .iter()
                .map(|piece| piece.def_id.as_str())
                .collect::<Vec<_>>(),
            vec![STARTOS]
        );
    }

    #[test]
    fn radiant_is_a_6_6_with_the_same_keywords() {
        crate::register_all();
        let _preview = open();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": STARTOS, "radiant": true }, ANCHOR], "mana": 10, "library": filler(4) },
            "p2": { "hand": [ANCHOR], "library": filler(4) },
        }));
        s.play(STARTOS, json!({ "zone": 1 }));
        s.expect_stats(STARTOS, json!({ "attack": 6, "health": 6 }));
    }
}
