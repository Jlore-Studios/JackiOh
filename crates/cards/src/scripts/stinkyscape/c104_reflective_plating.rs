//! S #104 Reflective Plating (SPEC §8.10 row 104): (2) Unit, Rare, 0/6 → 0/12, of the
//! Stinkyscape set.
//!   Base:    "Starship Piece, Equip / Any damage this takes is dealt to the enemy hero."
//!   Radiant: "Starship Piece, Equip / Any damage this takes is dealt to the enemy hero and the attacker."
//! Engine: Starship Piece (R1852000) and Equip (R1086; it passes no Starship Piece to its host,
//! R1852001) are catalog keywords. `StaticFlags.reflects_damage` (R1849003): what a hit dealt it is dealt
//! again from it at §4.4 step 8a to its controller's opponent's hero; it still takes the hit, and a hit
//! that dealt nothing (a Divine Shield, Armor) reflects nothing. Equipped, the host carries the flag, the
//! Fuse keeping its scripts (R77). Radiant: `reflects_to_attacker` too, back to the attacker whose strike
//! it was while that attacker is on the field.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-104";

fn plating(to_attacker: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            reflects_damage: Some(true),
            reflects_to_attacker: to_attacker.then_some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: plating(false),
        radiant: plating(true),
    }
}

// S #104 Reflective Plating — SPEC §8.10 row 104, BUILD M12 row S 104.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;

    const FILLER: &str = "core-005"; // (1) Spell: a card in hand, so no turn ends by itself.
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const SHREDDER: &str = "core-013"; // (3) 6/10.
    const ECLIPSE: &str = "core-035"; // (1) Spell: Deal 3 damage.

    fn attacked_by(radiant: bool, attacker: &str) -> (Scenario, CardInstance, CardInstance) {
        let s = crate::scenario(json!({
            "active": "p2",
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "radiant": radiant }] },
            "p2": { "hand": [FILLER], "field": [attacker] },
        }));
        let plating = s.unit_at(P1, 1);
        let attacker = s.unit_at(P2, 1);
        (s, plating, attacker)
    }

    mod base {
        use super::*;

        #[test]
        fn a_hit_of_n_deals_n_to_the_enemy_hero() {
            let (mut s, plating, vanilla) = attacked_by(false, VANILLA);
            s.attack(&vanilla, &plating);
            s.expect_stats(&plating, json!({ "health": 2 }));
            s.expect_health(P2, HERO_HEALTH - 4);
            s.expect_health(P1, HERO_HEALTH);
            s.expect_stats(&vanilla, json!({ "health": 4 }));
        }

        #[test]
        fn a_divine_shielded_hit_reflects_nothing() {
            let (mut s, plating, vanilla) = attacked_by(false, VANILLA);
            s.card_mut(&plating.id).granted_keywords.push(Keyword::DivineShield);
            s.attack(&vanilla, &plating);
            s.expect_stats(&plating, json!({ "health": 6 }));
            s.expect_health(P2, HERO_HEALTH);
        }

        #[test]
        fn an_armored_hit_that_deals_nothing_reflects_nothing() {
            let (mut s, plating, vanilla) = attacked_by(false, VANILLA);
            s.card_mut(&plating.id).granted_keywords.push(Keyword::Armor { n: 4 });
            s.attack(&vanilla, &plating);
            s.expect_stats(&plating, json!({ "health": 6 }));
            s.expect_health(P2, HERO_HEALTH);
        }

        #[test]
        fn equipped_the_host_reflects() {
            let mut s = crate::scenario(json!({
                "p1": { "hand": [ID, ECLIPSE, FILLER], "field": [VANILLA] },
                "p2": { "hand": [FILLER] },
            }));
            let host = s.unit_at(P1, 1);
            s.play(ID, json!({ "zone": 1, "equip": true }));
            let fused = s.unit_at(P1, 1);
            assert_eq!(fused.id, host.id, "the host is kept (R1086)");
            s.expect_stats(&fused, json!({ "attack": 4, "health": 10 }));
            assert!(!has_keyword(&s.stats(&fused).keywords, KeywordKind::StarshipPiece), "R1852001");
            s.play(ECLIPSE, json!({ "targets": pick::instance(&fused) }));
            s.expect_stats(&fused, json!({ "health": 7 }));
            s.expect_health(P2, HERO_HEALTH - 3);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn the_attacker_is_struck_back() {
            let (mut s, plating, shredder) = attacked_by(true, SHREDDER);
            s.expect_stats(&plating, json!({ "attack": 0, "health": 12 }));
            s.attack(&shredder, &plating);
            s.expect_stats(&plating, json!({ "health": 6 }));
            s.expect_health(P2, HERO_HEALTH - 6);
            s.expect_stats(&shredder, json!({ "health": 4 }));
        }
    }
}
