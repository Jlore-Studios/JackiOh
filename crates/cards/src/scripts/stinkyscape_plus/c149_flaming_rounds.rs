//! SPEC §8.12 S+ #149 Flaming Rounds (Unit 2/1 → 4/2, cost 1, Common), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Starship Piece, Equip
//!            Dealing damage Burns {burns}." — burns 2
//!   Radiant: the same text — burns 4
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it fuses
//! into it (R1086), which then carries the rider. R3540002: each hit this card deals that its target
//! survives adds N Burn after the hit; a hit that dealt nothing or killed its target adds nothing. N is
//! the declared `burns` (R386): each face's flag holds that face's number, so an Equip adds its N to its
//! host's and a launched Starship carries it.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-149";

/// "Dealing damage Burns 2": the base face's rider (R3540002).
const BURNS: i32 = 2;

/// "Dealing damage Burns 4": the Radiant face's rider (R3540002).
const RADIANT_BURNS: i32 = 4;

fn rounds(burns: i32) -> Script {
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
        base: rounds(BURNS),
        radiant: rounds(RADIANT_BURNS),
    }
}

// S+ #149 Flaming Rounds — SPEC §8.12 row 149 (R3540002, R1086).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{built_starship, hits, launch_starship, upgrade_number, weapon_board};
    use jackioh_engine::query::{burn_on, hero_burn_of};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A target that takes the hit and lives, striking nothing back.
    const TARGET: &str = "stand-in (1) 0/30";
    /// A target the hit kills.
    const DOOMED: &str = "stand-in (1) 0/1";
    /// An equipped host.
    const HOST: &str = "stand-in (1) 4/4 R12/12";

    /// The printed attack and rider, and the Radiant face's.
    const ATTACK: i32 = 2;
    const RADIANT_ATTACK: i32 = 4;

    fn rounds_on(radiant: bool, p2: Value) -> Scenario {
        weapon_board(
            "s149",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            p2,
        )
    }

    fn burn_of(s: &Scenario, id: &str) -> i32 {
        burn_on(s.card(id))
    }

    /// Whether any Burn was gained in `events`.
    fn burned(events: &[GameEvent]) -> bool {
        events.iter().any(|event| {
            matches!(
                event,
                GameEvent::CounterChanged {
                    counter: CounterKind::Burn,
                    ..
                }
            )
        })
    }

    /// R3540002: the attack hits for its attack and Burns N, the target surviving.
    fn strikes(radiant: bool, attack: i32, burns: i32) {
        let mut s = rounds_on(radiant, json!({ "field": [TARGET] }));
        let target = s.unit_at(P2, 1).id;
        s.attack(ID, target.as_str());
        assert_eq!(hits(s.last_events()), vec![(target.clone(), attack)], "radiant {radiant}");
        assert_eq!(burn_of(&s, &target), burns, "radiant {radiant}");
    }

    mod base {
        use super::*;

        #[test]
        fn r3540002_it_burns_what_it_hits_and_the_target_survives() {
            strikes(false, ATTACK, BURNS);
        }

        #[test]
        fn r3540002_a_hit_that_kills_adds_nothing() {
            let mut s = rounds_on(false, json!({ "field": [DOOMED] }));
            let target = s.unit_at(P2, 1).id;
            s.attack(ID, target.as_str());
            assert_eq!(hits(s.last_events()), vec![(target, ATTACK)]);
            assert!(s.unit(P2, 1).is_none());
            assert!(!burned(s.last_events()));
        }

        #[test]
        fn r3540002_it_burns_the_enemy_hero_it_hits() {
            let mut s = rounds_on(false, json!({}));
            s.attack(ID, "hero");
            assert_eq!(hits(s.last_events()), vec![("hero-p2".to_string(), ATTACK)]);
            assert_eq!(hero_burn_of(s.state(), P2), BURNS);
            s.expect_health(P2, HERO_HEALTH - ATTACK);
        }

        #[test]
        fn r3540002_equipped_into_a_unit_its_host_carries_the_rider() {
            let mut s = weapon_board(
                "s149-equip",
                json!({ "hand": [ID], "field": [HOST], "mana": 10 }),
                json!({ "field": [TARGET] }),
            );
            let host = s.unit_at(P1, 1).id;
            let target = s.unit_at(P2, 1).id;
            s.play(ID, json!({ "zone": 1, "equip": true }));
            let attack = s.stats(host.as_str()).attack;
            s.attack(host.as_str(), target.as_str());
            assert_eq!(hits(s.last_events()), vec![(target.clone(), attack)]);
            assert_eq!(burn_of(&s, &target), BURNS);
        }

        #[test]
        fn r3540002_a_launched_starship_carries_the_rider() {
            let (mut s, ship) = built_starship(
                "s149-ship",
                json!([{ "def": ID, "lane": 2 }]),
                json!({}),
                json!({}),
            );
            launch_starship(&mut s, &ship);
            // A piece fuses as its base version: the base N.
            assert_eq!(
                jackioh_engine::scripts::flags_of(s.state(), s.card(ship.as_str())).burns_on_damage,
                Some(BURNS)
            );
        }

        #[test]
        fn burns_reads_through_param() {
            let mut s = rounds_on(false, json!({ "field": [TARGET] }));
            let burns = upgrade_number(&mut s, ID, "burns");
            let target = s.unit_at(P2, 1).id;
            s.attack(ID, target.as_str());
            assert_eq!(burn_of(&s, &target), burns);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3540002_it_burns_what_it_hits_twice_as_hard() {
            strikes(true, RADIANT_ATTACK, RADIANT_BURNS);
        }

        #[test]
        fn its_stats_are_doubled() {
            let s = rounds_on(true, json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (4, 2));
        }
    }
}
