//! SPEC §8.12 S+ #114 Reign, a card of the held Stinkyscape+ set (R1420, R3535000). Built by S12 (#3545).
//! 12/8  
//! **Tribute 3**  
//! **Immutable**  
//! **Immune to spells**  
//! **Ranged**  
//! **Windfury**  
//! **Pierce**  
//! **Cleave**  
//! **Armor 4**  
//! Damage also **Burns 4**  
//! **\~\~\~**  
//! 24/16  
//! **Tribute 3**  
//! **Immutable**  
//! **Immune to spells**  
//! **Ranged**  
//! **Windfury**  
//! **Pierce**  
//! **Rush**  
//! **Cleave**  
//! **Armor 4**  
//! Damage also **Burns 8**
//!
//! Tribute 3 is the script's `tribute` (R101, R366); "Damage also Burns N" is the rider (R3540002), N declared as
//! `burns` (R386), after every hit, Cleave's included.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-114";

/// "Tribute 3" (R101).
const TRIBUTE: i32 = 3;

/// The printed Burns of each face, which the catalog declares as `burns` (R386); the engine reads the flag
/// through `declared_or`, which moves it only by what a tune moved the declared number.
const BASE_BURNS: i32 = 4;
const RADIANT_BURNS: i32 = 8;

/// R3540002: "Damage also Burns N" as the `burnsOnDamage` flag.
fn rides(burns: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE),
            burns_on_damage: Some(burns),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: rides(BASE_BURNS),
        radiant: rides(RADIANT_BURNS),
    }
}

// S+ #114 Reign — SPEC §8.12 row 114.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const FODDER: &str = "stand-in (0) 1/1";
    const WALL: &str = "stand-in (0) 40/60"; // strikes back for 40: only Ranged saves an attacker
    const ARMORED: &str = "stand-in (0) 40/60 Armor 5";
    const ATTACK: [i32; 2] = [12, 24];
    const HEALTH: [i32; 2] = [8, 16];

    /// The card on p1's field, p1 to move.
    fn on_field(radiant: bool, enemies: Value) -> Scenario {
        crate::weapon_board("s114", json!({ "field": [{ "def": ID, "radiant": radiant }] }), json!({ "field": enemies }))
    }

    /// The card in hand with `fodder` units to pay its Tribute.
    fn in_hand(radiant: bool, fodder: usize) -> Scenario {
        crate::weapon_board(
            "s114",
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

    fn burn_of(s: &Scenario, id: &str) -> i32 {
        query::burn_on(s.card(id))
    }

    fn burns(radiant: bool) -> i32 {
        if radiant { RADIANT_BURNS } else { BASE_BURNS }
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
    fn tribute_3_is_paid_in_units() {
        for radiant in [false, true] {
            let mut s = in_hand(radiant, TRIBUTE as usize);
            assert_eq!(tribute_cost_of(s.state(), s.card(ID)), TRIBUTE, "radiant {radiant}");
            s.expect_refused(|s| s.play(ID, json!({ "zone": 5 })));
            if TRIBUTE > 1 {
                let one_short: Vec<String> = (1..TRIBUTE).map(|lane| s.unit_at(P1, lane).id).collect();
                s.expect_refused(|s| s.play(ID, json!({ "zone": 5, "tributes": one_short })));
            }
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
        assert_eq!(on_field(false, json!([])).stats(ID).armor, 4);
        assert_eq!(on_field(true, json!([])).stats(ID).armor, 4);
    }

    #[test]
    fn pierces_armor_and_burns() {
        for radiant in [false, true] {
            let mut s = on_field(radiant, json!([ARMORED]));
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            assert_eq!(s.stats(wall.as_str()).health, 60 - ATTACK[usize::from(radiant)], "radiant {radiant}");
            assert_eq!(burn_of(&s, wall.as_str()), burns(radiant), "radiant {radiant}");
        }
    }

    #[test]
    fn declared_burns_match_the_script() {
        for radiant in [false, true] {
            let s = on_field(radiant, json!([]));
            let declared = param_value(s.state(), Some(s.card(ID)), "burns", ParamValueOptions::default());
            assert_eq!(declared, burns(radiant), "radiant {radiant}");
        }
    }

    #[test]
    fn windfury_attacks_twice_and_no_third() {
        let mut s = on_field(false, json!([WALL]));
        let wall = s.unit_at(P2, 1).id;
        s.attack(ID, wall.as_str());
        s.attack(ID, wall.as_str());
        assert_eq!(s.stats(wall.as_str()).health, 60 - 2 * ATTACK[0]);
        s.expect_refused(|s| s.attack(ID, wall.as_str()));
    }

    #[test]
    fn cleave_burns_both_neighbours() {
        for radiant in [false, true] {
            let mut s = on_field(radiant, json!([WALL, WALL, WALL]));
            let lanes = [1, 2, 3].map(|lane| s.unit_at(P2, lane).id);
            s.attack(ID, lanes[1].as_str());
            for id in &lanes {
                assert_eq!(s.stats(id.as_str()).health, 60 - ATTACK[usize::from(radiant)], "radiant {radiant}");
                assert_eq!(burn_of(&s, id.as_str()), burns(radiant), "radiant {radiant}");
            }
        }
    }

    #[test]
    fn immune_to_spells_no_spell_reaches_it() {
        for radiant in [false, true] {
            let s = crate::weapon_board(
                "s114",
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
            let mut s = in_hand(false, 3);
            play(&mut s, 3);
            let wall = s.unit_at(P2, 1).id;
            s.expect_refused(|s| s.attack(ID, wall.as_str()));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn rush_hits_a_unit_not_the_hero() {
            let mut s = in_hand(true, 3);
            play(&mut s, 3);
            s.expect_refused(|s| s.attack(ID, "hero"));
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            assert_eq!(s.stats(wall.as_str()).health, 60 - ATTACK[1]);
        }
    }
}
