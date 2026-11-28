//! FvH #19.1 Wall of Bricks (SPEC §8.14 row 19.1): (1) Unit, Token (printed Common), 0/5 → 0/10, the token
//! FvH #19 Trowel summons.
//!   Base:    "Can't attack, Brittle 1, Taunt"
//!   Radiant: "Can't attack, Brittle 2, Taunt"
//!
//! Every word is a catalog keyword (§6.1; Brittle R385, R638), so there is no script to run: the Radiant
//! face differs only in its catalog data (Brittle 2 and its stats) and runs the same empty script.
//! Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-019-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    // The Radiant face's Brittle 2 and 0/10 are catalog data only.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #19.1 Wall of Bricks — SPEC §8.14 row 19.1 (R385, R638). The keywords' own rule matrices live in
// the engine's rules tests; each face shows here that its printed keywords reach play.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    /// A textless Spell each seat can always play, so no turn ends by itself (§2.5).
    const SPELL: &str = "stand-in (1) Spell";
    const ATTACKER: &str = "stand-in (1) 3/3";

    /// p1 plays the Wall into lane 1; p2 holds a 3/3 on the field.
    fn walled(radiant: bool) -> (Scenario, String) {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, SPELL], "library": filler(6) },
            "p2": { "hand": [SPELL], "field": [ATTACKER], "library": filler(6) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        let wall = s.unit_at(P1, 1).id;
        (s, wall)
    }

    /// The Wall can never attack, and an enemy must attack it before the hero.
    fn cant_attack_and_taunts(s: &mut Scenario, wall: &str) {
        assert!(s.attacks_of(wall).is_empty(), "Can't attack");
        s.end_turn();
        s.expect_refused(|s| s.attack(ATTACKER, "hero"));
        s.attack(ATTACKER, wall);
        s.expect_in_zone(wall, "field");
    }

    mod base {
        use super::*;

        #[test]
        fn r385_its_printed_cant_attack_and_taunt_reach_play() {
            let (mut s, wall) = walled(false);
            cant_attack_and_taunts(&mut s, &wall);
            s.expect_stats(&wall, json!({ "health": 2, "maxHealth": 5 }));
        }

        #[test]
        fn r385_r638_its_brittle_1_crumbles_it_at_the_start_of_its_controllers_second_turn_on_the_field() {
            let (mut s, wall) = walled(false);
            s.end_turn();
            s.end_turn();
            assert!(s.unit(P1, 1).is_none_or(|unit| unit.id != wall));
            assert_eq!(s.count("crumbled"), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r385_its_printed_cant_attack_and_taunt_reach_play() {
            let (mut s, wall) = walled(true);
            cant_attack_and_taunts(&mut s, &wall);
            s.expect_stats(&wall, json!({ "health": 7, "maxHealth": 10 }));
        }

        #[test]
        fn r385_r638_its_brittle_2_holds_one_turn_longer() {
            let (mut s, wall) = walled(true);
            s.end_turn();
            s.end_turn();
            assert_eq!(s.unit(P1, 1).map(|unit| unit.id), Some(wall.clone()));
            assert_eq!(s.count("crumbled"), 0);
            s.end_turn();
            s.end_turn();
            assert!(s.unit(P1, 1).is_none_or(|unit| unit.id != wall));
            assert_eq!(s.count("crumbled"), 1);
        }
    }
}
