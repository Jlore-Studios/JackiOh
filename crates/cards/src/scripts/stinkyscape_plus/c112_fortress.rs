//! SPEC §8.12 S+ #112 Fortress, a card of the held Stinkyscape+ set (R1420, R3535000). Built by S12 (#3545).
//! 6/3  
//! **Tribute 1**  
//! **Immutable**  
//! **Immune to spells**  
//! **Ranged**  
//! **Pierce**  
//! **Armor 2**  
//! **\~\~\~**  
//! 12/6  
//! **Tribute 1**  
//! **Immutable**  
//! **Immune to spells**  
//! **Ranged**  
//! **Pierce**  
//! **Rush**  
//! **Armor 4**
//!
//! Tribute 1 is the script's `tribute` (R101, R366); the other keywords and Armor 2 / 4 are catalog data.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-112";

/// "Tribute 1" (R101).
const TRIBUTE: i32 = 1;

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    // The Radiant face adds Rush and the stats, catalog data, and keeps the Tribute.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #112 Fortress — SPEC §8.12 row 112.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const FODDER: &str = "stand-in (0) 1/1";
    const WALL: &str = "stand-in (0) 40/60"; // strikes back for 40: only Ranged saves an attacker
    const ARMORED: &str = "stand-in (0) 40/60 Armor 5";
    const ATTACK: [i32; 2] = [6, 12];
    const HEALTH: [i32; 2] = [3, 6];

    /// The card on p1's field, p1 to move.
    fn on_field(radiant: bool, enemies: Value) -> Scenario {
        crate::weapon_board("s112", json!({ "field": [{ "def": ID, "radiant": radiant }] }), json!({ "field": enemies }))
    }

    /// The card in hand with `fodder` units to pay its Tribute.
    fn in_hand(radiant: bool, fodder: usize) -> Scenario {
        crate::weapon_board(
            "s112",
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
    fn tribute_1_is_paid_in_units() {
        for radiant in [false, true] {
            let mut s = in_hand(radiant, TRIBUTE as usize);
            assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE, "radiant {radiant}");
            s.expect_refused(|s| s.play(ID, json!({ "zone": 5 })));
            let fodder: Vec<String> = (1..=TRIBUTE).map(|lane| s.unit_at(P1, lane).id).collect();
            play(&mut s, TRIBUTE);
            for id in &fodder {
                s.expect_in_zone(id.as_str(), "graveyard");
            }
            assert_eq!(s.unit_at(P1, 1).def_id, ID, "radiant {radiant}");
        }
    }

    #[test]
    fn armor_is_printed() {
        assert_eq!(on_field(false, json!([])).stats(ID).armor, 2);
        assert_eq!(on_field(true, json!([])).stats(ID).armor, 4);
    }

    #[test]
    fn armor_reduces_a_hit() {
        let mut s = on_field(false, json!(["stand-in (0) 4/9"]));
        s.end_turn();
        let enemy = s.unit_at(P2, 1).id;
        s.attack(enemy.as_str(), ID);
        assert_eq!(s.stats(ID).health, 3 - (4 - 2));
    }

    #[test]
    fn pierces_armor() {
        for radiant in [false, true] {
            let mut s = on_field(radiant, json!([ARMORED]));
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            assert_eq!(s.stats(wall.as_str()).health, 60 - ATTACK[usize::from(radiant)], "radiant {radiant}");
        }
    }

    #[test]
    fn immune_to_spells_no_spell_reaches_it() {
        for radiant in [false, true] {
            let s = crate::weapon_board(
                "s112",
                json!({ "field": [{ "def": ID, "radiant": radiant }], "hand": ["stand-in (0) Spell"] }),
                json!({}),
            );
            let card = s.unit_at(P1, 1);
            let spell = s.card("stand-in (0) Spell").clone();
            assert!(immune_to_spells(s.state(), &card), "radiant {radiant}");
            assert!(spell_cannot_reach(s.state(), Some(&spell), &card), "radiant {radiant}");
        }
    }

    mod base {
        use super::*;

        #[test]
        fn cannot_attack_on_its_summon_turn() {
            let mut s = in_hand(false, 1);
            play(&mut s, 1);
            let wall = s.unit_at(P2, 1).id;
            s.expect_refused(|s| s.attack(ID, wall.as_str()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn rush_hits_a_unit_not_the_hero() {
            let mut s = in_hand(true, 1);
            play(&mut s, 1);
            s.expect_refused(|s| s.attack(ID, "hero"));
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            assert_eq!(s.stats(wall.as_str()).health, 60 - ATTACK[1]);
        }
    }
}
