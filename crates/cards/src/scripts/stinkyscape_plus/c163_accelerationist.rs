//! SPEC §8.12 S+ #163 Accelerationist (Unit 6/6 → 12/12, cost 3, Common), a card of the held Stinkyscape+ set (R1420, R3535000).
//!   Base:    "Aura: Has +{buff}/+{buff} for every card that Evolved this game." (+2)
//!   Radiant: "Aura: Has +{buff2}/+{buff2} for every card that Evolved this game." (+4)
//!
//! Renumbered from the designer's second #92 (SP5). R3536004: each player counts the cards of theirs that
//! evolved this game (`evolutions_of`), so it counts both players' evolutions, its own included: the sum of
//! the two counts times the declared number, as a self aura (§10.4 layer 5) that gives nothing at 0.
//! Each evolving counts once, however often its text ran (R3536000, R3536003).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-163";

fn accelerationist(key: &'static str) -> Script {
    Script {
        aura: Some(aura_hook(move |args| {
            let evolved: i32 = PLAYER_IDS.iter().map(|player| evolutions_of(args.state, *player)).sum();
            if evolved == 0 {
                return Vec::new();
            }
            let bonus = evolved * param(&args, key);
            let id = args.self_.id.clone();
            vec![AuraEntry {
                applies: Box::new(move |card: &CardInstance| card.id == id),
                mod_: StatMod {
                    attack: Some(bonus),
                    max_health: Some(bonus),
                    ..StatMod::default()
                },
            }]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: accelerationist("buff"),
        radiant: accelerationist("buff2"),
    }
}

// S+ #163 Accelerationist — SPEC §8.12 row 163 (R3536004; R3542000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::{built_starship, launch_starship, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const EVOLUTIONARY: &str = "stinkyscapeplus-003"; // High Evolutionary: Cry: evolve a friendly permanent.
    const EXPLORER: &str = "stinkyscapeplus-075"; // J-lease S-Space EX-plorer: EX-plorer: evolves on your launch.
    const FLORA: &str = "stinkyscapeplus-087"; // Frontier Flora: evolves on either launch.
    const AP_ROUNDS: &str = "stinkyscape-100"; // AP Rounds: Starship Piece, keywords only

    fn accelerationist(radiant: bool) -> Value {
        json!({ "def": ID, "radiant": radiant })
    }

    /// p1 holds an Evolutionary, with an EX-plorer and an Accelerationist on the field.
    fn evolve_one(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "seed": "s163-evolve",
            "p1": {
                "mana": 10,
                "hand": [EVOLUTIONARY],
                "field": [EXPLORER, accelerationist(radiant)],
                "library": crate::filler(6),
            },
            "p2": { "hand": crate::filler(1), "library": crate::filler(4) },
        }));
        let explorer = s.unit_at(P1, 1);
        s.play(EVOLUTIONARY, json!({ "zone": 3, "targets": pick::instance(&explorer) }));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn two_more_per_evolution() {
            let mut s = evolve_one(false);
            assert_eq!(evolutions_of(s.state(), P1), 1);
            let this = s.unit_at(P1, 2);
            s.expect_stats(&this, json!({ "attack": 8, "health": 8 }));
        }

        #[test]
        fn an_opponents_evolution_counts() {
            let (mut s, ship) = built_starship(
                "s163-theirs",
                json!([{ "def": AP_ROUNDS, "lane": 1 }]),
                json!({ "field": [accelerationist(false)] }),
                json!({ "backrow": [{ "def": FLORA, "lane": 2 }] }),
            );
            let this = s.unit_at(P1, 2);
            launch_starship(&mut s, &ship);
            assert_eq!(evolutions_of(s.state(), P2), 1);
            assert_eq!(evolutions_of(s.state(), P1), 0);
            s.expect_stats(&this, json!({ "attack": 8, "health": 8 }));
        }

        #[test]
        fn no_evolution_no_bonus() {
            let mut s = scenario(json!({ "p1": { "field": [accelerationist(false)] } }));
            let this = s.unit_at(P1, 1);
            s.expect_stats(&this, json!({ "attack": 6, "health": 6 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn four_more_per_evolution() {
            let mut s = evolve_one(true);
            let this = s.unit_at(P1, 2);
            s.expect_stats(&this, json!({ "attack": 16, "health": 16 }));
        }
    }
}
