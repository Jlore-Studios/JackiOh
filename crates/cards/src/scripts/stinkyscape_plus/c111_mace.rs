//! SPEC §8.12 S+ #111 Mace, a card of the held Stinkyscape+ set (R1420, R3535000). Built by S12 (#3545).
//! 2/2  
//! **Immutable**  
//! **Immune to spells**  
//! **Ranged**  
//! **Pierce**  
//! Damage also **Burns 1**  
//! **\~\~\~**  
//! 4/4  
//! **Immutable**  
//! **Immune to spells**  
//! **Ranged**  
//! **Rush**  
//! **Pierce**  
//! Damage also **Burns 2**
//!
//! Damage also Burns N is the rider (R3540002): the script's `burnsOnDamage` flag, N declared as `burns` per face
//! (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-111";

/// The printed Burns of each face, which the catalog declares as `burns` (R386); the engine reads the flag
/// through `declared_or`, which moves it only by what a tune moved the declared number.
const BASE_BURNS: i32 = 1;
const RADIANT_BURNS: i32 = 2;

/// R3540002: "Damage also Burns N" as the `burnsOnDamage` flag.
fn rides(burns: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
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

// S+ #111 Mace — SPEC §8.12 row 111.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const FODDER: &str = "stand-in (0) 1/1";
    const WALL: &str = "stand-in (0) 40/60"; // strikes back for 40: only Ranged saves an attacker
    const ARMORED: &str = "stand-in (0) 40/60 Armor 5";
    const ATTACK: [i32; 2] = [2, 4];
    const HEALTH: [i32; 2] = [2, 4];

    /// The card on p1's field, p1 to move.
    fn on_field(radiant: bool, enemies: Value) -> Scenario {
        crate::weapon_board("s111", json!({ "field": [{ "def": ID, "radiant": radiant }] }), json!({ "field": enemies }))
    }

    /// The card in hand with `fodder` units to pay its Tribute.
    fn in_hand(radiant: bool, fodder: usize) -> Scenario {
        crate::weapon_board(
            "s111",
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
    fn burns_the_hero_too() {
        for radiant in [false, true] {
            let mut s = on_field(radiant, json!([]));
            s.attack(ID, "hero");
            s.expect_health(P2, HERO_HEALTH - ATTACK[usize::from(radiant)]);
            assert_eq!(query::hero_burn_of(s.state(), P2), burns(radiant), "radiant {radiant}");
        }
    }

    #[test]
    fn immune_to_spells_no_spell_reaches_it() {
        for radiant in [false, true] {
            let s = crate::weapon_board(
                "s111",
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
