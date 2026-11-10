//! SPEC §8.12 S+ #110 Dagger, a card of the held Stinkyscape+ set (R1420, R3535000). Built by S12 (#3545).
//! 2/1  
//! **Immutable**  
//! **Immune to spells**  
//! **Ranged**  
//! **\~\~\~**  
//! 4/2  
//! **Immutable**  
//! **Immune to spells**  
//! **Rush**  
//! **Ranged**
//!
//! Its keywords, Immutable, Immune to Spells and Ranged, are catalog data; the Radiant face adds Rush.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-110";

pub fn script() -> CardScripts {
    let base = Script::default();
    // Keywords are catalog data; the Radiant face only adds Rush / Reborn.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #110 Dagger — SPEC §8.12 row 110.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const FODDER: &str = "stand-in (0) 1/1";
    const WALL: &str = "stand-in (0) 40/60"; // strikes back for 40: only Ranged saves an attacker
    const ATTACK: [i32; 2] = [2, 4];
    const HEALTH: [i32; 2] = [1, 2];

    /// The card on p1's field, p1 to move.
    fn on_field(radiant: bool, enemies: Value) -> Scenario {
        crate::weapon_board("s110", json!({ "field": [{ "def": ID, "radiant": radiant }] }), json!({ "field": enemies }))
    }

    /// The card in hand with `fodder` units to pay its Tribute.
    fn in_hand(radiant: bool, fodder: usize) -> Scenario {
        crate::weapon_board(
            "s110",
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
    fn immune_to_spells_no_spell_reaches_it() {
        for radiant in [false, true] {
            let s = crate::weapon_board(
                "s110",
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
            let mut s = in_hand(false, 0);
            play(&mut s, 0);
            let wall = s.unit_at(P2, 1).id;
            s.expect_refused(|s| s.attack(ID, wall.as_str()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn rush_hits_a_unit_not_the_hero() {
            let mut s = in_hand(true, 0);
            play(&mut s, 0);
            s.expect_refused(|s| s.attack(ID, "hero"));
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            assert_eq!(s.stats(wall.as_str()).health, 60 - ATTACK[1]);
        }
    }
}
