//! SPEC §8.12 S+ #150 Scorching Rounds (Unit 4/1 → 8/2, 3/1 → 6/2 until issue #4783, cost 2, Rare), a card of the
//! Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Starship Piece
//!            Damage also Burns {burns}." — burns 4
//!   Radiant: the same text — burns 8
//!
//! Starship Piece is a catalog keyword the engine plays, and Equip was one through v0.6.7k (R4285001): Equipped onto one of your Units it fuses
//! into it (R1086), which then carries the rider. R3540002: each hit this card deals that its target
//! survives adds N Burn after the hit; a hit that dealt nothing or killed its target adds nothing. N is
//! the declared `burns` (R386): each face's flag holds that face's number, so an Equip adds its N to its
//! host's and a launched Starship carries it.
//! Issue #4783 (Stinkyscape+ Balance Changes VIII) took the rider to 4 (Radiant 8) from 3 (6). The engine
//! reads the flag through `declared_or`, which moves it only by what a tune moved the declared number, so
//! the flag's own N is the script's: `script_v0_6_13c` keeps 3 (6) for the replays of those versions
//! (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-150";

/// "Damage also Burns 4": the base face's rider (R3540002).
const BURNS: i32 = 4;

/// "Damage also Burns 8": the Radiant face's rider (R3540002).
const RADIANT_BURNS: i32 = 8;

/// Burns 3 (Radiant 6), through v0.6.13c (issue #4783).
const BURNS_V0_6_13C: i32 = 3;
const RADIANT_BURNS_V0_6_13C: i32 = 6;

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

/// R767000: Burns 3 (Radiant 6) through v0.6.13c (issue #4783): the flag's printed number, which the
/// declared `burns` only moves by a tune, so the replays of those versions fold under it (`KEPT_SCRIPTS`).
pub fn script_v0_6_13c() -> CardScripts {
    CardScripts {
        base: rounds(BURNS_V0_6_13C),
        radiant: rounds(RADIANT_BURNS_V0_6_13C),
    }
}

// S+ #150 Scorching Rounds — SPEC §8.12 row 150 (R3540002, R1086).
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

    /// The printed attack, and the Radiant face's (3 and 6 until issue #4783).
    const ATTACK: i32 = 4;
    const RADIANT_ATTACK: i32 = 8;

    fn rounds_on(radiant: bool, p2: Value) -> Scenario {
        weapon_board(
            "s150",
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
                "s150-equip",
                json!({ "hand": [ID], "field": [HOST], "mana": 10 }),
                json!({ "field": [TARGET] }),
            );
            let host = s.unit_at(P1, 1).id;
            let target = s.unit_at(P2, 1).id;
            crate::give_equip(&mut s, ID);
            s.play(ID, json!({ "zone": 1, "equip": true }));
            let attack = s.stats(host.as_str()).attack;
            s.attack(host.as_str(), target.as_str());
            assert_eq!(hits(s.last_events()), vec![(target.clone(), attack)]);
            assert_eq!(burn_of(&s, &target), BURNS);
        }

        #[test]
        fn r3540002_a_launched_starship_carries_the_rider() {
            let (mut s, ship) = built_starship(
                "s150-ship",
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

        /// R767000: `run` under the script v0.6.13c ran and the `burns` it declared, 3 (Radiant 6).
        fn through_v0_6_13c<T>(run: impl FnOnce() -> T) -> T {
            crate::under_kept_as(
                ID,
                script_v0_6_13c(),
                |def| {
                    def.params = Some(vec![json_as(json!({
                        "key": "burns",
                        "base": BURNS_V0_6_13C,
                        "radiant": RADIANT_BURNS_V0_6_13C,
                        "better": "up",
                        "step": 1,
                        "min": 1,
                    }))]);
                },
                run,
            )
        }

        #[test]
        fn r767000_the_script_kept_for_v0_6_13c_burns_3_and_6_on_the_radiant_face() {
            through_v0_6_13c(|| {
                strikes(false, ATTACK, BURNS_V0_6_13C);
                strikes(true, RADIANT_ATTACK, RADIANT_BURNS_V0_6_13C);
            });
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
            assert_eq!((view.attack, view.health), (8, 2));
        }
    }
}
