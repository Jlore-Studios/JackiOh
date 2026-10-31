//! S #74 Paragon-X (SPEC §8.10 row 74): (4) Unit, Mythic, 10/1 → 20/2, of the Stinkyscape set.
//!   Base:    "Tribute 2, Starship Piece, Charge, Flying, Ranged, Divine Shield, Immutable / Immune to
//!            Spell Damage."
//!   Radiant: "Tribute 2, Starship Piece, Charge, Flying, Ranged, Divine Shield, Immutable, Pierce /
//!            Immune to Spell Damage."
//! Engine: its keywords are catalog data (§6.1): Starship Piece (R1852000: dying, a copy fuses into its
//! owner's Dormant Starship), Charge, Flying, Ranged, Divine Shield and Immutable (R23). Tribute 2 is its
//! play's cost (§6.3, `StaticFlags.tribute`, R101). Immune to Spell Damage (R1849000,
//! `StaticFlags.immune_to_spell_damage`): a Spell's hit on it is no damage instance, its Divine Shield
//! kept; a Unit's or a Trap's lands. Radiant: Pierce, the same flags.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-074";

/// §6.3 "Tribute 2": two of your Units, or one Sheep Token, which is worth 2 (§3.2).
const TRIBUTE_COST: i32 = 2;

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            immune_to_spell_damage: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    // The Radiant face adds Pierce, catalog data, and keeps both flags.
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
        fn a_spells_hit_deals_nothing_and_keeps_divine_shield() {
            // Immutable: no Spell may target it (R781001), so the Spell is an "each enemy" one.
            let mut s = on_the_field(false, json!([POWDER_SPRAY]));
            let paragon = s.unit_at(P1, 1);
            assert!(has(&s, &paragon, KeywordKind::DivineShield));
            s.play(POWDER_SPRAY, json!({}));
            s.expect_health(P1, HERO_HEALTH - 3);
            s.expect_stats(&paragon, json!({ "attack": 10, "health": 1 }));
            assert!(has(&s, &paragon, KeywordKind::DivineShield), "no instance, so no shield spent");
            assert!(!s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::Damage { target_id, .. } if *target_id == paragon.id
            )));
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
        fn dying_it_fuses_into_the_dormant_starship() {
            let mut s = on_the_field(false, json!([ARMAGEDDON]));
            let paragon = s.unit_at(P1, 1);
            s.play(ARMAGEDDON, json!({}));
            s.expect_in_zone(&paragon, "graveyard");
            assert!(s.last_events().iter().any(|event| matches!(
                event,
                GameEvent::StarshipFused { player: P1, def_id, .. } if def_id == ID
            )));
            let starship = dormant_starship_of(s.state(), P1).expect("a Dormant Starship was summoned").clone();
            assert_eq!(starship_pieces_of(&starship).len(), 1);
            assert!(!has(&s, &starship, KeywordKind::StarshipPiece), "R1852001: the hull is no piece");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn is_20_2_with_pierce() {
            let mut s = on_the_field(true, json!([POWDER_SPRAY]));
            let paragon = s.unit_at(P1, 1);
            s.expect_stats(&paragon, json!({ "attack": 20, "health": 2 }));
            assert!(has(&s, &paragon, KeywordKind::Pierce));
            s.play(POWDER_SPRAY, json!({}));
            s.expect_stats(&paragon, json!({ "health": 2 }));
            assert!(has(&s, &paragon, KeywordKind::DivineShield));
        }
    }
}
