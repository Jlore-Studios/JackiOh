//! SPEC §8.12 S+ #153 Noxious Rounds (Unit 1/1 → 2/2, cost 4, Epic), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base and Radiant: "Starship Piece, Equip
//!            Damage dealt is Toxic."
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it fuses
//! into it (R1086), which then carries the conversion. R3540003: each hit this card deals turns into its
//! target's Toxic after Armor, equal to what the hit would have dealt, and deals nothing; a hit of 0 after
//! Armor converts nothing. The flag holds on both faces, so an Equip converts its host's hits and a
//! launched Starship carries it.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-153";

fn rounds() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            damage_is_toxic: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: rounds(),
        radiant: rounds(),
    }
}

// S+ #153 Noxious Rounds — SPEC §8.12 row 153 (R3540003, R1086).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{built_starship, hits, launch_starship, weapon_board};
    use jackioh_engine::query::{hero_toxic_of, toxic_on};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A target that takes the hit and lives, striking nothing back.
    const TARGET: &str = "stand-in (1) 0/30";
    /// An equipped host.
    const HOST: &str = "stand-in (1) 4/4 R12/12";

    /// The printed attack, and the Radiant face's.
    const ATTACK: i32 = 1;
    const RADIANT_ATTACK: i32 = 2;

    fn rounds_on(radiant: bool, p2: Value) -> Scenario {
        weapon_board(
            "s153",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            p2,
        )
    }

    fn toxic_of(s: &Scenario, id: &str) -> i32 {
        toxic_on(s.card(id))
    }

    /// R3540003: the attack deals nothing and applies the attack's attack as Toxic, the target surviving.
    fn converts(radiant: bool, attack: i32) {
        let mut s = rounds_on(radiant, json!({ "field": [TARGET] }));
        let target = s.unit_at(P2, 1).id;
        s.attack(ID, target.as_str());
        assert!(hits(s.last_events()).is_empty(), "radiant {radiant}");
        assert_eq!(toxic_of(&s, &target), attack, "radiant {radiant}");
        s.expect_stats(target.as_str(), json!({ "health": 30 }));
    }

    mod base {
        use super::*;

        #[test]
        fn r3540003_its_damage_is_the_targets_toxic_and_deals_nothing() {
            converts(false, ATTACK);
        }

        #[test]
        fn r3540003_it_applies_toxic_to_the_enemy_hero_instead_of_hurting_it() {
            let mut s = rounds_on(false, json!({}));
            s.attack(ID, "hero");
            assert!(hits(s.last_events()).is_empty());
            assert_eq!(hero_toxic_of(s.state(), P2), ATTACK);
            s.expect_health(P2, HERO_HEALTH);
        }

        #[test]
        fn r3540003_equipped_into_a_unit_its_host_converts_its_hits() {
            let mut s = weapon_board(
                "s153-equip",
                json!({ "hand": [ID], "field": [HOST], "mana": 10 }),
                json!({ "field": [TARGET] }),
            );
            let host = s.unit_at(P1, 1).id;
            let target = s.unit_at(P2, 1).id;
            s.play(ID, json!({ "zone": 1, "equip": true }));
            let attack = s.stats(host.as_str()).attack;
            s.attack(host.as_str(), target.as_str());
            assert!(hits(s.last_events()).is_empty());
            assert_eq!(toxic_of(&s, &target), attack);
        }

        #[test]
        fn r3540003_a_launched_starship_carries_the_conversion() {
            let (mut s, ship) = built_starship(
                "s153-ship",
                json!([{ "def": ID, "lane": 2 }]),
                json!({}),
                json!({}),
            );
            launch_starship(&mut s, &ship);
            assert_eq!(
                jackioh_engine::scripts::flags_of(s.state(), s.card(ship.as_str())).damage_is_toxic,
                Some(true)
            );
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3540003_its_damage_is_the_targets_toxic_twice_as_hard() {
            converts(true, RADIANT_ATTACK);
        }

        #[test]
        fn its_stats_are_doubled() {
            let s = rounds_on(true, json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (2, 2));
        }
    }
}
