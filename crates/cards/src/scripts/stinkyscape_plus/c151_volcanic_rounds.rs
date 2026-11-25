//! SPEC §8.12 S+ #151 Volcanic Rounds (Unit 6/1 → 12/2, 5/1 → 10/2 until issue #4783, 3/1 → 6/2 through
//! v0.6.7k (issue #4285), cost 4, Epic), a card of the Stinkyscape+ set (R1420, R3535000).
//!   Base and Radiant: "Starship Piece
//!            Damage dealt also Burns."
//!
//! Starship Piece is a catalog keyword the engine plays, and Equip was one through v0.6.7k (R4285001):
//! Equipped onto one of your Units it fuses into it (R1086), which then carries the rider. R4783001: each
//! hit this card deals lands as damage, and then, if its target survived it, adds Burn equal to the damage
//! it dealt (§4.4 step 7, beside R3540002's riders); a hit that dealt nothing or killed its target adds
//! none. The flag holds on both faces, so an Equip carries it to its host and a launched Starship carries
//! it. Through v0.6.13c its text was "Damage dealt Burns": each hit turned into its target's Burn and dealt
//! nothing (R3540003), which `script_v0_6_13c` keeps (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-151";

/// R4783001: "Damage dealt also Burns".
fn rounds() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            damage_also_burns: Some(true),
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

/// R3540003: "Damage dealt Burns", the conversion v0.6.13c and before printed.
fn rounds_v0_6_13c() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            damage_burns: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

/// R767000: through v0.6.13c each hit it dealt turned into its target's Burn and dealt nothing (R3540003,
/// issue #4783); `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_13c() -> CardScripts {
    CardScripts {
        base: rounds_v0_6_13c(),
        radiant: rounds_v0_6_13c(),
    }
}

// S+ #151 Volcanic Rounds — SPEC §8.12 row 151 (R4783001, R3540003, R1086).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{built_starship, hits, launch_starship, weapon_board};
    use jackioh_engine::query::{burn_on, hero_burn_of};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A target that takes the hit and lives, striking nothing back.
    const TARGET: &str = "stand-in (1) 0/30";
    /// A target whose Armor takes 2 of the hit.
    const ARMORED: &str = "stand-in (1) 0/30 Armor 2";
    /// A target the hit kills.
    const DOOMED: &str = "stand-in (1) 0/1";
    /// An equipped host.
    const HOST: &str = "stand-in (1) 4/4 R12/12";

    /// The printed attack, and the Radiant face's (5 and 10 until issue #4783).
    const ATTACK: i32 = 6;
    const RADIANT_ATTACK: i32 = 12;
    /// What `ARMORED`'s Armor takes off a hit.
    const ARMOR: i32 = 2;
    /// `TARGET`'s health.
    const TARGET_HEALTH: i32 = 30;

    fn rounds_on(radiant: bool, p2: Value) -> Scenario {
        weapon_board(
            "s151",
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

    /// R4783001: the attack hits for its attack and then Burns as much, the target surviving.
    fn strikes(radiant: bool, attack: i32) {
        let mut s = rounds_on(radiant, json!({ "field": [TARGET] }));
        let target = s.unit_at(P2, 1).id;
        s.attack(ID, target.as_str());
        assert_eq!(hits(s.last_events()), vec![(target.clone(), attack)], "radiant {radiant}");
        assert_eq!(burn_of(&s, &target), attack, "radiant {radiant}");
        s.expect_stats(target.as_str(), json!({ "health": TARGET_HEALTH - attack }));
    }

    /// R3540003, the script kept for v0.6.13c: the attack deals nothing and Burns the attack's attack.
    fn converts(radiant: bool, attack: i32) {
        let mut s = rounds_on(radiant, json!({ "field": [TARGET] }));
        let target = s.unit_at(P2, 1).id;
        s.attack(ID, target.as_str());
        assert!(hits(s.last_events()).is_empty(), "radiant {radiant}");
        assert_eq!(burn_of(&s, &target), attack, "radiant {radiant}");
        s.expect_stats(target.as_str(), json!({ "health": TARGET_HEALTH }));
    }

    mod base {
        use super::*;

        #[test]
        fn r4783001_its_hit_deals_its_attack_and_then_burns_as_much() {
            strikes(false, ATTACK);
        }

        #[test]
        fn r4783001_it_burns_the_enemy_hero_as_much_as_it_hurt_it() {
            let mut s = rounds_on(false, json!({}));
            s.attack(ID, "hero");
            assert_eq!(hits(s.last_events()), vec![("hero-p2".to_string(), ATTACK)]);
            assert_eq!(hero_burn_of(s.state(), P2), ATTACK);
            s.expect_health(P2, HERO_HEALTH - ATTACK);
        }

        #[test]
        fn r4783001_the_burn_is_what_got_past_armor() {
            let mut s = rounds_on(false, json!({ "field": [ARMORED] }));
            let target = s.unit_at(P2, 1).id;
            s.attack(ID, target.as_str());
            assert_eq!(hits(s.last_events()), vec![(target.clone(), ATTACK - ARMOR)]);
            assert_eq!(burn_of(&s, &target), ATTACK - ARMOR);
        }

        #[test]
        fn r4783001_a_hit_that_kills_burns_nothing() {
            let mut s = rounds_on(false, json!({ "field": [DOOMED] }));
            let target = s.unit_at(P2, 1).id;
            s.attack(ID, target.as_str());
            assert_eq!(hits(s.last_events()), vec![(target, ATTACK)]);
            assert!(s.unit(P2, 1).is_none());
            assert!(!burned(s.last_events()));
        }

        #[test]
        fn r4783001_equipped_into_a_unit_its_host_burns_what_it_deals() {
            let mut s = weapon_board(
                "s151-equip",
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
            assert_eq!(burn_of(&s, &target), attack);
        }

        #[test]
        fn r4783001_a_launched_starship_carries_the_rider_and_no_conversion() {
            let (mut s, ship) = built_starship(
                "s151-ship",
                json!([{ "def": ID, "lane": 2 }]),
                json!({}),
                json!({}),
            );
            launch_starship(&mut s, &ship);
            let flags = jackioh_engine::scripts::flags_of(s.state(), s.card(ship.as_str()));
            assert_eq!(flags.damage_also_burns, Some(true));
            assert_eq!(flags.damage_burns, None);
        }

        #[test]
        fn r767000_r3540003_the_script_kept_for_v0_6_13c_converts_each_hit_into_burn() {
            crate::under_kept(ID, script_v0_6_13c(), || {
                converts(false, ATTACK);
                converts(true, RADIANT_ATTACK);
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4783001_its_hit_deals_its_attack_and_then_burns_as_much_twice_as_hard() {
            strikes(true, RADIANT_ATTACK);
        }

        #[test]
        fn its_stats_are_doubled() {
            let s = rounds_on(true, json!({}));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (RADIANT_ATTACK, 2));
        }
    }
}
