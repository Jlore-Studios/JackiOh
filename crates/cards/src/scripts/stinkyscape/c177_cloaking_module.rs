//! S #177 Cloaking Module (SPEC §8.10 row 177): (2) Unit, Rare, 2/2 → 4/4, a card of the Stinkyscape
//! set (#2521).
//!   Base:    "Starship Piece, Equip, Cloaked, Immutable"
//!   Radiant: "Starship Piece, Equip, Cloaked, Immutable, Immune to Spells"
//!
//! Keywords only (§6.1): its script is empty. Equip (R1086, R77): played onto one of your Units it fuses
//! into it, Starship Piece only into a piece (R1852001). Cloaked (R783004–R783006): it may be set
//! face-down and revealed into a unit zone. Immutable (R23, R781001): no Spell, Field Spell or Field Trap
//! targets it in play, and nothing fuses onto it. Starship Piece: played alone, its death fuses its base
//! version into its owner's Dormant Starship (R1852000, R2521000). Immune to Spells (E35): no Spell
//! targets or affects it.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-177";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face differs in catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #177 Cloaking Module — SPEC §8.10 row 177, BUILD M12 row S 177: "played onto a Unit as an Equip, the
// host gains +2/+2 and stays no piece unless it was one (R1852001); set face-down by Cloaked and revealed,
// it takes a unit zone; played alone, its death fuses it into a Dormant Starship (R1852000); radiant 4/4
// with Immune to Spells: a Spell's sweep passes it by".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MODULE: &str = "stinkyscape-177";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const NETHER: &str = "core-088"; // (4) Spell: Destroy all permanents.
    const POWDER_SPRAY: &str = "classicplus-012-4"; // (1) Spell: Deal 3 damage to each enemy.
    const ANCHOR: &str = "core-010"; // (0) Spell (§2.5).

    fn kinds_of(keywords: &[Keyword]) -> Vec<String> {
        keywords
            .iter()
            .map(|keyword| keyword.kind().as_str().to_string())
            .collect()
    }

    fn standing(radiant: bool, hand: Value, p2_hand: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": hand, "field": [{ "def": MODULE, "radiant": radiant }], "mana": 10, "library": filler(4) },
            "p2": { "hand": p2_hand, "library": filler(4) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn prints_starship_piece_equip_cloaked_immutable_2_2_radiant_4_4_adds_immune_to_spells() {
            crate::register_all();
            let def = crate::card_def(MODULE);
            assert_eq!(
                kinds_of(&def.base.keywords),
                vec!["Starship Piece", "Equip", "Cloaked", "Immutable"]
            );
            assert_eq!(
                kinds_of(&def.radiant.keywords),
                vec!["Starship Piece", "Equip", "Cloaked", "Immutable", "Immune to Spells"]
            );
            assert_eq!((def.base.attack, def.base.health), (Some(2), Some(2)));
            assert_eq!((def.radiant.attack, def.radiant.health), (Some(4), Some(4)));
        }

        #[test]
        fn r1852001_equipped_onto_a_unit_it_fuses_and_the_host_stays_no_piece() {
            let mut s = scenario(json!({
                "p1": { "hand": [VANILLA, MODULE, ANCHOR], "mana": 10, "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.play(VANILLA, json!({ "zone": 1 }));
            let host = s.unit_at(P1, 1);
            s.play(MODULE, json!({ "zone": 1, "equip": true }));

            // R77: the host is kept, a 4/4 that is also a 2/2.
            assert!(
                s.last_events().iter().any(|event| event.event_type() == GameEventType::Fused),
                "the Equip fused"
            );
            assert_eq!(s.unit_at(P1, 1).id, host.id, "the host keeps its zone");
            s.expect_stats(&host, json!({ "attack": 6, "health": 6 }));
            // A piece equipped onto a Unit that is no piece leaves it no piece (R1852001).
            assert!(!has_keyword(&s.stats(&host).keywords, KeywordKind::StarshipPiece));
            assert!(s.unit(P1, 2).is_none(), "nothing else took a zone");
        }

        #[test]
        fn r783006_cloaked_it_is_set_face_down_and_revealed_into_a_unit_zone() {
            let mut s = scenario(json!({
                "p1": { "hand": [MODULE, ANCHOR], "mana": 10, "library": filler(4) },
                "p2": { "hand": [ANCHOR], "library": filler(4) },
            }));
            s.play(MODULE, json!({ "zone": 1, "row": "backrow", "faceDown": "cloaked" }));
            let set = s.backrow(P1, 1).expect("the set Module");
            assert_eq!(set.def_id, MODULE);
            assert!(set.set_as.is_some(), "set by its Cloaked");
            assert!(s.unit(P1, 1).is_none());

            s.activate(&set.id, json!({ "ability": jackioh_engine::alt_play::CLOAKED_REVEAL }));

            let unit = s.unit(P1, 1).expect("it took a unit zone");
            assert_eq!(unit.def_id, MODULE);
            s.expect_stats(&unit, json!({ "attack": 2, "health": 2 }));
        }

        #[test]
        fn r1852000_played_alone_its_death_fuses_it_into_a_dormant_starship() {
            // Immutable keeps a targeted Spell off it in play (R781001): a sweep destroys it.
            let mut s = standing(false, json!([NETHER, ANCHOR]), json!([ANCHOR]));
            let me = s.unit_at(P1, 1);
            assert!(has_keyword(&s.stats(&me).keywords, KeywordKind::StarshipPiece));
            s.play(NETHER, json!({}));

            s.expect_in_zone(&me, "graveyard");
            let ship = dormant_starship_of(s.state(), P1).cloned().expect("a Dormant Starship");
            let pieces: Vec<String> = starship_pieces_of(&ship).into_iter().map(|piece| piece.def_id).collect();
            assert_eq!(pieces, vec![MODULE.to_string()]);
        }

        #[test]
        fn r781001_immutable_no_spell_may_target_it_in_play() {
            let mut s = standing(false, json!([HIT_JOB, ANCHOR]), json!([ANCHOR]));
            let me = s.unit_at(P1, 1);
            assert!(!immune_to_spells(s.state(), &me), "the base face is not Immune to Spells");
            s.expect_refused(|s| s.play(HIT_JOB, json!({ "targets": pick::instance(&me) })));
            s.expect_in_zone(&me, "field");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_4_4_and_immune_to_spells_a_spells_sweep_passes_it_by() {
            let mut s = standing(true, json!([NETHER, ANCHOR]), json!([ANCHOR]));
            let me = s.unit_at(P1, 1);
            s.expect_stats(&me, json!({ "attack": 4, "health": 4 }));
            assert!(has_keyword(&s.stats(&me).keywords, KeywordKind::ImmuneToSpells));
            assert!(immune_to_spells(s.state(), &me));
            let spell = s.card(NETHER).clone();
            assert!(spell_cannot_reach(s.state(), Some(&spell), &me));

            // Destroy all permanents: its own Spell passes it by, so no piece dies and nothing fuses.
            s.play(NETHER, json!({}));
            s.expect_in_zone(&me, "field");
            assert!(dormant_starship_of(s.state(), P1).is_none());
        }

        #[test]
        fn an_enemy_spells_damage_passes_it_by() {
            let mut s = standing(true, json!([ANCHOR]), json!([POWDER_SPRAY, ANCHOR]));
            let me = s.unit_at(P1, 1);
            s.end_turn();
            s.play(POWDER_SPRAY, json!({}));
            s.expect_health(P1, HERO_HEALTH - 3);
            s.expect_stats(&me, json!({ "health": 4 }));
            assert!(!s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Damage { target_id, .. } if *target_id == me.id
            )));
            assert_eq!(s.state().active, P2);
        }
    }
}
