//! SPEC §8.12 S+ #116 Horizon, a card of the held Stinkyscape+ set (R1420, R3535000). Built by S12 (#3545).
//! 3/1  
//! **Flying**  
//! **Ranged**  
//! **Rush**  
//! **Can’t Defend**  
//! \~\~\~  
//! 6/2  
//! **Flying**  
//! **Ranged**  
//! **Rush**  
//! **Reborn**  
//! **Can’t Defend**
//!
//! Its keywords are catalog data (Flying, Ranged, Rush, Can't Defend); the Radiant face adds Reborn.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-116";

pub fn script() -> CardScripts {
    let base = Script::default();
    // Keywords are catalog data; the Radiant face only adds Rush / Reborn.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #116 Horizon — SPEC §8.12 row 116.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const FODDER: &str = "stand-in (0) 1/1";
    const WALL: &str = "stand-in (0) 40/60"; // strikes back for 40: only Ranged saves an attacker
    const ATTACK: [i32; 2] = [3, 6];
    const HEALTH: [i32; 2] = [1, 2];

    /// The card on p1's field, p1 to move.
    fn on_field(radiant: bool, enemies: Value) -> Scenario {
        crate::weapon_board("s116", json!({ "field": [{ "def": ID, "radiant": radiant }] }), json!({ "field": enemies }))
    }

    /// The card in hand with `fodder` units to pay its Tribute.
    fn in_hand(radiant: bool, fodder: usize) -> Scenario {
        crate::weapon_board(
            "s116",
            json!({ "hand": [{ "def": ID, "radiant": radiant }], "field": vec![FODDER; fodder] }),
            json!({ "field": [WALL] }),
        )
    }

    /// Plays it into lane 1, paying the units in lanes 1..=`tribute`.
    fn play(s: &mut Scenario, tribute: i32) {
        let tributes: Vec<String> = (1..=tribute).map(|lane| s.unit_at(P1, lane).id).collect();
        let opts = if tributes.is_empty() { json!({ "zone": 1 }) } else { json!({ "zone": 1, "tributes": tributes }) };
        s.play(ID, opts);
    }

    #[test]
    fn ranged_takes_no_strike_back() {
        for radiant in [false, true] {
            let mut s = on_field(radiant, json!([WALL]));
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            assert_eq!(s.stats(ID).health, HEALTH[usize::from(radiant)], "radiant {radiant}");
            assert_eq!(s.stats(wall.as_str()).health, 60 - ATTACK[usize::from(radiant)], "radiant {radiant}");
        }
    }

    #[test]
    fn flying_only_ranged_or_flying_attackers() {
        for radiant in [false, true] {
            let mut s = on_field(radiant, json!(["stand-in (0) 3/9", "stand-in (0) 9/9 Ranged"]));
            s.end_turn();
            let (grounded, ranged) = (s.unit_at(P2, 1).id, s.unit_at(P2, 2).id);
            s.expect_refused(|s| s.attack(grounded.as_str(), ID));
            s.attack(ranged.as_str(), ID);
            let left = s.unit(P1, 1).map_or(0, |unit| s.stats(&unit).health);
            assert!(left < HEALTH[usize::from(radiant)], "radiant {radiant}: the Ranged attacker reached it");
        }
    }

    #[test]
    fn cant_defend() {
        for radiant in [false, true] {
            let mut s = on_field(radiant, json!([WALL]));
            s.expect_refused(|s| s.switch_position(ID));
        }
    }

    #[test]
    fn rush_hits_a_unit_not_the_hero() {
        for radiant in [false, true] {
            let mut s = in_hand(radiant, 0);
            play(&mut s, 0);
            s.expect_refused(|s| s.attack(ID, "hero"));
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            assert_eq!(s.stats(wall.as_str()).health, 60 - ATTACK[usize::from(radiant)], "radiant {radiant}");
        }
    }

    mod base {
        use super::*;

        #[test]
        fn dies_for_good() {
            let mut s = on_field(false, json!(["stand-in (0) 9/9 Ranged"]));
            s.end_turn();
            let killer = s.unit_at(P2, 1).id;
            s.attack(killer.as_str(), ID);
            s.expect_in_zone(ID, "graveyard");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn reborn_returns_at_1_health_without_reborn() {
            let mut s = on_field(true, json!(["stand-in (0) 9/9 Ranged"]));
            s.end_turn();
            let killer = s.unit_at(P2, 1).id;
            s.attack(killer.as_str(), ID);
            assert_eq!(s.unit_at(P1, 1).def_id, ID);
            assert_eq!(s.stats(ID).health, 1);
            assert!(!has_keyword(&s.stats(ID).keywords, KeywordKind::Reborn));
        }
    }
}
