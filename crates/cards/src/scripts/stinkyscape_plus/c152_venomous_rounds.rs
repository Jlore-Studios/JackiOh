//! SPEC §8.12 S+ #152 Venomous Rounds (Unit 2/1 → 4/2, cost 2, Rare), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Starship Piece, Equip
//!            Damage also applies Toxic {toxic}." — toxic 2
//!   Radiant: the same text — toxic 4
//!
//! Starship Piece and Equip are catalog keywords the engine plays: played onto one of your Units it fuses
//! into it (R1086), which then carries the rider. R3540002: each hit this card deals that its target
//! survives adds N Toxic after the hit; a hit that dealt nothing or killed its target adds nothing. N is
//! the declared `toxic` (R386): each face's flag holds that face's number, so an Equip adds its N to its
//! host's and a launched Starship carries it.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-152";

/// "Damage also applies Toxic 2": the base face's rider (R3540002).
const TOXIC: i32 = 2;

/// "Damage also applies Toxic 4": the Radiant face's rider (R3540002).
const RADIANT_TOXIC: i32 = 4;

fn rounds(toxic: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            toxic_on_damage: Some(toxic),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: rounds(TOXIC),
        radiant: rounds(RADIANT_TOXIC),
    }
}

// S+ #152 Venomous Rounds — SPEC §8.12 row 152 (R3540002, R1086).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{built_starship, hits, launch_starship, upgrade_number, weapon_board};
    use jackioh_engine::query::{hero_toxic_of, toxic_on};
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
            "s152",
            json!({ "field": [{ "def": ID, "radiant": radiant }] }),
            p2,
        )
    }

    fn toxic_of(s: &Scenario, id: &str) -> i32 {
        toxic_on(s.card(id))
    }

    /// Whether any Toxic was gained in `events`.
    fn toxified(events: &[GameEvent]) -> bool {
        events.iter().any(|event| {
            matches!(
                event,
                GameEvent::CounterChanged {
                    counter: CounterKind::Toxic,
                    ..
                }
            )
        })
    }

    /// R3540002: the attack hits for its attack and applies N Toxic, the target surviving.
    fn strikes(radiant: bool, attack: i32, toxic: i32) {
        let mut s = rounds_on(radiant, json!({ "field": [TARGET] }));
        let target = s.unit_at(P2, 1).id;
        s.attack(ID, target.as_str());
        assert_eq!(hits(s.last_events()), vec![(target.clone(), attack)], "radiant {radiant}");
        assert_eq!(toxic_of(&s, &target), toxic, "radiant {radiant}");
    }

    mod base {
        use super::*;

        #[test]
        fn r3540002_it_applies_toxic_to_what_it_hits_and_the_target_survives() {
            strikes(false, ATTACK, TOXIC);
        }

        #[test]
        fn r3540002_a_hit_that_kills_adds_nothing() {
            let mut s = rounds_on(false, json!({ "field": [DOOMED] }));
            let target = s.unit_at(P2, 1).id;
            s.attack(ID, target.as_str());
            assert_eq!(hits(s.last_events()), vec![(target, ATTACK)]);
            assert!(s.unit(P2, 1).is_none());
            assert!(!toxified(s.last_events()));
        }

        #[test]
        fn r3540002_it_applies_toxic_to_the_enemy_hero_it_hits() {
            let mut s = rounds_on(false, json!({}));
            s.attack(ID, "hero");
            assert_eq!(hits(s.last_events()), vec![("hero-p2".to_string(), ATTACK)]);
            assert_eq!(hero_toxic_of(s.state(), P2), TOXIC);
            s.expect_health(P2, HERO_HEALTH - ATTACK);
        }

        #[test]
        fn r3540002_equipped_into_a_unit_its_host_carries_the_rider() {
            let mut s = weapon_board(
                "s152-equip",
                json!({ "hand": [ID], "field": [HOST], "mana": 10 }),
                json!({ "field": [TARGET] }),
            );
            let host = s.unit_at(P1, 1).id;
            let target = s.unit_at(P2, 1).id;
            s.play(ID, json!({ "zone": 1, "equip": true }));
            let attack = s.stats(host.as_str()).attack;
            s.attack(host.as_str(), target.as_str());
            assert_eq!(hits(s.last_events()), vec![(target.clone(), attack)]);
            assert_eq!(toxic_of(&s, &target), TOXIC);
        }

        #[test]
        fn r3540002_a_launched_starship_carries_the_rider() {
            let (mut s, ship) = built_starship(
                "s152-ship",
                json!([{ "def": ID, "lane": 2 }]),
                json!({}),
                json!({}),
            );
            launch_starship(&mut s, &ship);
            // A piece fuses as its base version: the base N.
            assert_eq!(
                jackioh_engine::scripts::flags_of(s.state(), s.card(ship.as_str())).toxic_on_damage,
                Some(TOXIC)
            );
        }

        #[test]
        fn toxic_reads_through_param() {
            let mut s = rounds_on(false, json!({ "field": [TARGET] }));
            let toxic = upgrade_number(&mut s, ID, "toxic");
            let target = s.unit_at(P2, 1).id;
            s.attack(ID, target.as_str());
            assert_eq!(toxic_of(&s, &target), toxic);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3540002_it_applies_toxic_to_what_it_hits_twice_as_hard() {
            strikes(true, RADIANT_ATTACK, RADIANT_TOXIC);
        }

        #[test]
        fn its_stats_are_doubled() {
            let s = rounds_on(true, json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (4, 2));
        }
    }
}
