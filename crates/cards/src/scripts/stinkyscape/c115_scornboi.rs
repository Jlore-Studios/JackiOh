//! S #115 Scornboi (SPEC §8.10 row 115): (4) Unit, Human, Acclaimed, Legendary, 12/9 → 24/18, of
//! the Stinkyscape set.
//!   Base:    "Tribute 3 / Aura: Every Unit that is neither Human, a Starship Piece nor a Starship is
//!            exiled."
//!   Radiant: "Tribute 3 / Aura: Every enemy Unit that is neither Human, a Starship Piece nor a
//!            Starship is exiled."
//! Engine: Tribute 3 is its play's cost, §6.3's `staticFlags.tribute` (R101), on both faces. The aura is
//! an exile aura (R1850002): the Units on both sides, `notTags` Human and `notKeywords` Starship Piece
//! and Starship, read through the layers (§10.4), so a granted tag or keyword spares a Unit; exiled at
//! each state check, never this card, a face-down or a Dormant one (a Dormant Starship is no Unit until
//! it launches, R1839000). Units only (ST9). The Radiant face reaches the enemy side only. Since #2521
//! fifteen Units are no Starship Pieces (S #68 Stratos, the Paragons, the Badgers, the Weapon hosts …),
//! so the aura exiles them as any other Unit with neither.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-115";

/// §6.3 "Tribute 3": three of your Units, or a Sheep Token and one more, worth 3 (R101).
const TRIBUTE_COST: i32 = 3;

fn scornboi(side: &str) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        exile_auras: vec![ExileAura {
            scope: json_as(json!({ "side": side, "notTags": ["Human"] })),
            cost: None,
            not_keywords: vec![KeywordKind::StarshipPiece, KeywordKind::Starship],
        }],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: scornboi("any"),
        radiant: scornboi("enemy"),
    }
}

// S #115 Scornboi — SPEC §8.10 row 115, BUILD M12 row S 115 (R1850002).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const VANILLA: &str = "core-008"; // (1) 4/4, a Human: a tribute.
    const MENACE: &str = "core-019"; // (3) 9/9, no tag.
    const CHASSIS: &str = "stinkyscape-048"; // (1) 4/1, a Starship Piece.
    const STRATOS: &str = "stinkyscape-068"; // (1) 3/1 Flying, Ranged: no piece since #2521.
    const PARAGON_MK1: &str = "stinkyscape-072"; // (3) 6/1 Charge, Flying, Ranged: no piece since #2521.
    const ARMOR: &str = "core-073"; // (2) Field Spell: no Unit.
    const FILLER: &str = "core-005";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [VANILLA, VANILLA, VANILLA, MENACE],
                "library": filler(4),
                "mana": 10,
            },
            "p2": {
                "hand": [FILLER],
                "field": [VANILLA, MENACE, CHASSIS],
                "backrow": [ARMOR],
                "library": filler(4),
            },
        }))
    }

    fn exiled(s: &Scenario) -> Vec<(PlayerId, String)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Exiled { instance_id, .. } => {
                    let card = s.card(instance_id.as_str());
                    Some((card.owner, card.def_id.clone()))
                }
                _ => None,
            })
            .collect()
    }

    fn play_scornboi(s: &mut Scenario) {
        let tributes = [s.unit_at(P1, 1).id, s.unit_at(P1, 2).id, s.unit_at(P1, 3).id];
        s.play(ID, json!({ "tributes": tributes, "zone": 1 }));
    }

    #[test]
    fn r101_it_needs_its_tribute_3() {
        let mut s = scenario(json!({
            "p1": { "hand": [ID, FILLER], "field": [VANILLA, VANILLA], "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.expect_refused(|s| s.play(ID, json!({})));
    }

    #[test]
    fn r1850002_every_unit_neither_human_nor_a_starship_piece_is_exiled_on_both_sides() {
        let mut s = board(false);
        play_scornboi(&mut s);
        assert_eq!(exiled(&s), vec![(P1, MENACE.to_string()), (P2, MENACE.to_string())]);
        assert_eq!(s.unit_at(P2, 1).def_id, VANILLA, "a Human stays");
        assert_eq!(s.unit_at(P2, 3).def_id, CHASSIS, "a Starship Piece stays");
        assert!(s.backrow(P2, 1).is_some(), "Units only: a Field Spell stays");
        s.expect_stats(ID, json!({ "attack": 12, "health": 9 }));
    }

    #[test]
    fn r1850002_stratos_and_paragon_mk1_no_starship_pieces_since_2521_are_exiled() {
        let mut s = scenario(json!({
            "p1": {
                "hand": [ID, FILLER],
                "field": [VANILLA, VANILLA, VANILLA],
                "library": filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "field": [STRATOS, PARAGON_MK1, CHASSIS], "library": filler(4) },
        }));
        play_scornboi(&mut s);
        assert_eq!(
            exiled(&s),
            vec![(P2, STRATOS.to_string()), (P2, PARAGON_MK1.to_string())]
        );
        assert_eq!(s.unit_at(P2, 3).def_id, CHASSIS, "a Starship Piece stays");
    }

    #[test]
    fn r1850002_a_granted_human_tag_spares_a_unit() {
        let mut s = board(false);
        let spared = s.unit_at(P2, 2).id;
        s.card_mut(spared.as_str()).granted_tags = Some(vec![Tag::Human]);
        play_scornboi(&mut s);
        assert_eq!(
            exiled(&s),
            vec![(P1, MENACE.to_string())],
            "only the untagged Menace"
        );
        assert_eq!(s.unit_at(P2, 2).id, spared, "the Menace given Human stays");
    }

    #[test]
    fn r275_radiant_24_18_exiles_the_enemys_alone() {
        let mut s = board(true);
        play_scornboi(&mut s);
        assert_eq!(exiled(&s), vec![(P2, MENACE.to_string())]);
        assert_eq!(s.unit_at(P1, 4).def_id, MENACE);
        s.expect_stats(ID, json!({ "attack": 24, "health": 18 }));
    }
}
