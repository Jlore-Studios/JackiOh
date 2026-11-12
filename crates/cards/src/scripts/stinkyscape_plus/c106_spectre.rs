//! SPEC §8.12 S+ #106 Spectre, a card of the Stinkyscape+ set (R1420, R3535000). Built by S08 (#3555).
//! 3/12  
//! **Flexible**  
//! **Indestructible**  
//! **GIGA**  
//! **Windfury**  
//! **Ranged**  
//! **Pierce**  
//! \~\~\~  
//! 6/24  
//! **Flexible**  
//! **Indestructible**  
//! **GIGA**  
//! **Windfury**  
//! **Ranged**  
//! **Pierce**
//!
//! Every word is a catalog keyword (§6.1), so there is no script to run. GIGA anchors it in its unit
//! row despite Flexible (R3546000): Flexible changes nothing for it while it is GIGA.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-106";

pub fn script() -> CardScripts {
    // Keywords only: catalog data on both faces.
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #106 Spectre — SPEC §8.12 row S+ 106 (R346, R636, R783000, R3546000).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::weapon_board;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const ARMORED: &str = "stand-in (0) 1/30 Armor 5";
    const BIG: &str = "stand-in (0) 30/30";

    /// Spectre standing for p1, enemies standing for p2.
    fn standing(radiant: bool, seed: &str, enemies: Value) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            json!({ "field": enemies }),
        )
    }

    #[test]
    fn r3546000_it_is_offered_only_unit_row_anchors() {
        for radiant in [false, true] {
            let s = weapon_board(
                &format!("s106-anchors-{radiant}"),
                json!({ "hand": [{ "def": ID, "radiant": radiant }], "mana": 10 }),
                json!({}),
            );
            let id = s.card(ID).id.clone();
            let mut pairs: Vec<(Row, i32)> = legal_actions(s.state(), P1)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Play { instance_id, zone, .. } if instance_id == id => zone,
                    _ => None,
                })
                .map(|zone| (zone.row, zone.lane))
                .collect();
            pairs.sort();
            // A Unit anchors in its unit row despite Flexible (R3546000): lanes 1–4, no backrow.
            assert_eq!(
                pairs,
                vec![
                    (Row::Units, 1),
                    (Row::Units, 2),
                    (Row::Units, 3),
                    (Row::Units, 4),
                ],
                "radiant {radiant}"
            );
        }
    }

    #[test]
    fn windfury_ranged_pierce() {
        for radiant in [false, true] {
            let mut s = standing(radiant, &format!("s106-hits-{radiant}"), json!([ARMORED]));
            // A setup unit attacks once its sickness wears off.
            s.end_turn();
            s.end_turn();
            let target = s.unit_at(P2, 1).id;
            let hit = if radiant { 6 } else { 3 };
            s.attack(ID, target.as_str());
            // Pierce reads off the source (R346): the whole hit past Armor 5.
            assert_eq!(s.stats(target.as_str()).health, 30 - hit, "radiant {radiant}");
            // Windfury (R636): a second attack, and no strike back (Ranged, R783000).
            s.attack(ID, target.as_str());
            assert_eq!(s.stats(target.as_str()).health, 30 - 2 * hit, "radiant {radiant}");
            let full = if radiant { 24 } else { 12 };
            s.expect_stats(ID, json!({ "health": full }));
            // ... but no third.
            let target = s.unit_at(P2, 1).id;
            s.expect_refused(|s| s.attack(ID, target.as_str()));
        }
    }

    #[test]
    fn indestructible_survives_a_lethal_hit() {
        for radiant in [false, true] {
            let mut s = standing(radiant, &format!("s106-lethal-{radiant}"), json!([BIG]));
            s.end_turn();
            let big = s.unit_at(P2, 1).id;
            s.attack(big.as_str(), ID);
            s.expect_in_zone(ID, "field");
        }
    }
}
