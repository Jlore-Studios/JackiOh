//! S #74 Paragon-X (SPEC §8.10 row 74): (4) Unit, Mythic, 10/1 → 20/2, of the Stinkyscape set.
//!   Base:    "Tribute 2, Charge, Flying, Ranged, Divine Shield, Immutable, Immune to Spells"
//!   Radiant: "Tribute 2, Charge, Flying, Ranged, Divine Shield, Immutable, Immune to Spells, Pierce"
//! Engine: its keywords are catalog data (§6.1): Charge, Flying, Ranged, Divine Shield, Immutable (R23,
//! R781001) and Immune to Spells (E35, `restrictions::spell_cannot_reach`): no Spell may target it,
//! and a Spell's sweep passes it by, its damage and its destroys alike, its Divine Shield kept; a Unit's
//! or a Trap's lands. Tribute 2 is its play's cost (§6.3, `StaticFlags.tribute`, R101). Since #2521 it
//! prints Immune to Spells in place of its Immune to Spell Damage flag (R1849000) and is no Starship
//! Piece, so its death fuses nothing. Radiant: Pierce, the same keywords. Through v0.5.11 it carried
//! the flag, which `script_v0_5_11` keeps (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-074";

/// §6.3 "Tribute 2": two of your Units, or one Sheep Token, which is worth 2 (§3.2).
const TRIBUTE_COST: i32 = 2;

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    // The Radiant face adds Pierce, catalog data, and keeps the Tribute.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

/// R767000: through v0.5.11 it printed no Immune to Spells and carried Immune to Spell Damage as a flag
/// (R1849000: a Spell's hit on it was no damage instance; a Spell's destroy still reached it);
/// `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_5_11() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            immune_to_spell_damage: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// S #74 Paragon-X — SPEC §8.10 row 74, BUILD M12 row S 74.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // (1) Spell: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const POWDER_SPRAY: &str = "classicplus-012-4"; // (1) Spell: Deal 3 damage to each enemy.
    const SORCERER: &str = "core-068"; // (2) 5/5, Cry: Deal 4 damage.
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const ARMAGEDDON: &str = "core-088"; // (4) Spell: Destroy all permanents.

    fn holding(radiant: bool, p2_hand: Value) -> Scenario {
        crate::scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [VANILLA, VANILLA],
                "library": crate::filler(4),
            },
            "p2": { "hand": p2_hand, "library": crate::filler(4) },
        }))
    }

    fn on_the_field(radiant: bool, p2_hand: Value) -> Scenario {
        let mut s = holding(radiant, p2_hand);
        let tributes: Vec<String> = [1, 2].map(|lane| s.unit_at(P1, lane).id).to_vec();
        s.play(ID, json!({ "zone": 1, "tributes": tributes }));
        s.end_turn();
        s
    }

    fn has(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> bool {
        has_keyword(&s.stats(card).keywords, kind)
    }

    fn hit(s: &Scenario, card: &CardInstance) -> bool {
        s.last_events().iter().any(|event| matches!(
            event,
            GameEvent::Damage { target_id, .. } if *target_id == card.id
        ))
    }

    mod base {
        use super::*;

        #[test]
        fn refused_without_two_tributes() {
            let mut s = holding(false, json!([]));
            let one = s.unit_at(P1, 1).id;
            s.expect_refused_with(|s| s.play(ID, json!({ "zone": 3 })), "needs Tribute 2");
            s.expect_refused_with(|s| s.play(ID, json!({ "zone": 3, "tributes": [one] })), "needs Tribute 2");
            let tributes: Vec<String> = [1, 2].map(|lane| s.unit_at(P1, lane).id).to_vec();
            s.play(ID, json!({ "zone": 3, "tributes": tributes }));
            let paragon = s.unit_at(P1, 3);
            assert_eq!(paragon.def_id, ID);
            assert!(s.unit(P1, 1).is_none() && s.unit(P1, 2).is_none(), "both tributes went");
            s.attack(&paragon, "hero");
            s.expect_health(P2, HERO_HEALTH - 10);
        }

        #[test]
        fn immune_to_spells_no_spell_may_target_it() {
            let mut s = on_the_field(false, json!([HIT_JOB, FILLER]));
            let paragon = s.unit_at(P1, 1);
            assert!(has(&s, &paragon, KeywordKind::ImmuneToSpells));
            assert!(immune_to_spells(s.state(), &paragon));
            let spell = s.card(HIT_JOB).clone();
            assert!(spell_cannot_reach(s.state(), Some(&spell), &paragon));
            s.expect_refused(|s| s.play(HIT_JOB, json!({ "targets": pick::instance(&paragon) })));
            s.expect_in_zone(&paragon, "field");
        }

        #[test]
        fn a_damaging_spells_sweep_skips_it_and_keeps_divine_shield() {
            let mut s = on_the_field(false, json!([POWDER_SPRAY]));
            let paragon = s.unit_at(P1, 1);
            assert!(has(&s, &paragon, KeywordKind::DivineShield));
            s.play(POWDER_SPRAY, json!({}));
            // The hero still takes the sweep's 3; the Paragon is passed by, no instance at all.
            s.expect_health(P1, HERO_HEALTH - 3);
            s.expect_stats(&paragon, json!({ "attack": 10, "health": 1 }));
            assert!(has(&s, &paragon, KeywordKind::DivineShield), "no instance, so no shield spent");
            assert!(!hit(&s, &paragon));
        }

        #[test]
        fn a_spells_destroy_all_passes_it_by() {
            let mut s = on_the_field(false, json!([ARMAGEDDON]));
            let paragon = s.unit_at(P1, 1);
            s.play(ARMAGEDDON, json!({}));
            s.expect_in_zone(&paragon, "field");
        }

        #[test]
        fn a_units_hit_lands() {
            let mut s = on_the_field(false, json!([SORCERER]));
            let paragon = s.unit_at(P1, 1);
            s.play(SORCERER, json!({ "zone": 1, "targets": pick::instance(&paragon) }));
            assert!(!has(&s, &paragon, KeywordKind::DivineShield), "the Cry's hit took the shield");
            s.expect_stats(&paragon, json!({ "health": 1 }));
        }

        #[test]
        fn its_death_fuses_nothing_it_is_no_starship_piece() {
            // Two Units' Cries: the first takes the Divine Shield, the second kills it.
            let mut s = on_the_field(false, json!([SORCERER, SORCERER]));
            let paragon = s.unit_at(P1, 1);
            assert!(!has(&s, &paragon, KeywordKind::StarshipPiece));
            s.play(SORCERER, json!({ "zone": 1, "targets": pick::instance(&paragon) }));
            s.play(SORCERER, json!({ "zone": 2, "targets": pick::instance(&paragon) }));
            s.expect_in_zone(&paragon, "graveyard");
            assert!(dormant_starship_of(s.state(), P1).is_none(), "no piece died, so no Dormant Starship");
            assert!(!s.events().iter().any(|event| matches!(event, GameEvent::StarshipFused { .. })));
        }

        #[test]
        fn r767000_the_script_kept_for_v0_5_11_spares_it_a_spells_damage_by_its_flag() {
            // As v0.5.11 printed it: no Immune to Spells, the flag alone (R1849000).
            let old = |def: &mut CardDef| {
                for face in [&mut def.base, &mut def.radiant] {
                    face.keywords.retain(|keyword| keyword.kind() != KeywordKind::ImmuneToSpells);
                }
            };
            crate::under_kept_as(ID, super::super::script_v0_5_11(), old, || {
                let mut s = on_the_field(false, json!([POWDER_SPRAY]));
                let paragon = s.unit_at(P1, 1);
                assert!(!immune_to_spells(s.state(), &paragon));
                s.play(POWDER_SPRAY, json!({}));
                s.expect_stats(&paragon, json!({ "health": 1 }));
                assert!(has(&s, &paragon, KeywordKind::DivineShield));
                assert!(!hit(&s, &paragon));
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_20_2_with_pierce_and_immune_to_spells() {
            let mut s = on_the_field(true, json!([POWDER_SPRAY]));
            let paragon = s.unit_at(P1, 1);
            s.expect_stats(&paragon, json!({ "attack": 20, "health": 2 }));
            assert!(has(&s, &paragon, KeywordKind::Pierce));
            assert!(has(&s, &paragon, KeywordKind::ImmuneToSpells));
            // The same keyword: a Spell's sweep passes it by.
            s.play(POWDER_SPRAY, json!({}));
            s.expect_stats(&paragon, json!({ "health": 2 }));
            assert!(has(&s, &paragon, KeywordKind::DivineShield));
            assert!(!hit(&s, &paragon));
        }

        #[test]
        fn its_death_fuses_nothing() {
            let mut s = on_the_field(true, json!([SORCERER, SORCERER]));
            let paragon = s.unit_at(P1, 1);
            s.play(SORCERER, json!({ "zone": 1, "targets": pick::instance(&paragon) }));
            s.play(SORCERER, json!({ "zone": 2, "targets": pick::instance(&paragon) }));
            s.expect_in_zone(&paragon, "graveyard");
            assert!(dormant_starship_of(s.state(), P1).is_none());
        }
    }
}
