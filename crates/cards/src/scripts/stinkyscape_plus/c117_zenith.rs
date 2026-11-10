//! SPEC §8.12 S+ #117 Zenith, a card of the held Stinkyscape+ set (R1420, R3535000). Built by S12 (#3545).
//! 5/3  
//! **Tribute 1**  
//! **Flying**  
//! **Ranged**  
//! **Pierce**  
//! Damage also**Burns 3**  
//! **Can’t Defend**  
//! \~\~\~  
//! 10/6  
//! **Tribute 1**  
//! **Flying**  
//! **Ranged**  
//! **Pierce**  
//! **Rush**  
//! Damage also **Burns 6**  
//! **Can’t Defend**
//!
//! Tribute 1 is the script's `tribute` (R101, R366); "Damage also Burns N" is the rider (R3540002), N declared as
//! `burns` (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-117";

/// "Tribute 1" (R101).
const TRIBUTE: i32 = 1;

/// The printed Burns of each face, which the catalog declares as `burns` (R386); the engine reads the flag
/// through `declared_or`, which moves it only by what a tune moved the declared number.
const BASE_BURNS: i32 = 3;
const RADIANT_BURNS: i32 = 6;

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

// S+ #117 Zenith — SPEC §8.12 row 117.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::play_choices::tribute_cost_of;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    const FODDER: &str = "stand-in (0) 1/1";
    const WALL: &str = "stand-in (0) 40/60"; // strikes back for 40: only Ranged saves an attacker
    const ARMORED: &str = "stand-in (0) 40/60 Armor 5";
    const ATTACK: [i32; 2] = [5, 10];
    const HEALTH: [i32; 2] = [3, 6];

    /// The card on p1's field, p1 to move.
    fn on_field(radiant: bool, enemies: Value) -> Scenario {
        crate::weapon_board("s117", json!({ "field": [{ "def": ID, "radiant": radiant }] }), json!({ "field": enemies }))
    }

    /// The card in hand with `fodder` units to pay its Tribute.
    fn in_hand(radiant: bool, fodder: usize) -> Scenario {
        crate::weapon_board(
            "s117",
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
    fn r386_an_upgrade_adds_more_burn() {
        for radiant in [false, true] {
            let mut s = on_field(radiant, json!([WALL]));
            let moved = crate::upgrade_number(&mut s, ID, "burns");
            assert!(moved > burns(radiant), "radiant {radiant}");
            let wall = s.unit_at(P2, 1).id;
            s.attack(ID, wall.as_str());
            assert_eq!(burn_of(&s, wall.as_str()), moved, "radiant {radiant}");
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
