//! FvH #36 Heartrock (SPEC §8.14 row 36): (2) Field Spell, Rare.
//!   Base:    "End of turn: If you spent all your mana this turn, summon a random Asteroid."
//!   Radiant: "End of turn: If you spent all your mana this turn, summon a random Radiant Asteroid."
//!
//! Engine: End of turn of its controller (R62): while `unspent_mana_of` is 0 as the step runs (the turn
//! will not Float, R1563000; a turn with no mana to spend counts), `summon_random` from the four Asteroid
//! Units, S #169 Tiny, S #170 Medium, S #171 Large and S #172 Huge Asteroid, named by id (never the Spell
//! S #97 Asteroid Field), into its controller's leftmost open unit zone (R64), summoned so no Cry; with no
//! open zone nothing (R129). Radiant: a Radiant one. Declares no numbers.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-036";

/// "A random Asteroid": the four Asteroid Units of the Stinkyscape set.
const ASTEROIDS: [&str; 4] = [
    "stinkyscape-169",
    "stinkyscape-170",
    "stinkyscape-171",
    "stinkyscape-172",
];
/// "Spent all your mana": no mana left unspent.
const NONE_LEFT: i32 = 0;

fn face(radiant: bool) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            if unspent_mana_of(&*ctx.state, ctx.controller) > NONE_LEFT {
                return vec![];
            }
            vec![summon_random(json_as(json!({
                "query": { "defId": ASTEROIDS },
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// FvH #36 Heartrock — SPEC §8.14 row 36 (R62, R1563000, R64, R129).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// A textless Spell each seat can always play while it has mana, so no turn ends by itself (§2.5).
    const SPELL: &str = "stand-in (1) Spell";
    const BODY: &str = "stand-in (1) 1/1";
    /// How many seeds the random Asteroid is drawn under.
    const SEEDS: usize = 6;

    fn heartrock(seed: &str, radiant: bool, mana: i32, field: Value) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [SPELL],
                "backrow": [{ "def": ID, "radiant": radiant, "faceUp": true }],
                "field": field,
                "mana": mana,
                "library": filler(6),
            },
            "p2": { "hand": [SPELL], "library": filler(6) },
        }))
    }

    /// The Asteroids the last step summoned, by def id and face, with whose side they landed on.
    fn asteroids(s: &Scenario) -> Vec<(PlayerId, String, bool)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { player, instance_id, def_id, .. } if ASTEROIDS.contains(&def_id.as_str()) => {
                    Some((*player, def_id.clone(), s.card(instance_id.as_str()).radiant))
                }
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r62_r64_with_all_its_mana_spent_its_controllers_end_of_turn_summons_a_random_asteroid() {
            for n in 0..SEEDS {
                let seed = format!("fvh-036-base-{n}");
                let mut s = heartrock(&seed, false, 0, json!([]));
                s.end_turn();
                let summoned = asteroids(&s);
                assert_eq!(summoned.len(), 1, "{seed}: {summoned:?}");
                let (player, def_id, radiant) = &summoned[0];
                assert_eq!(*player, P1, "{seed}");
                assert!(!radiant, "{seed}");
                assert_eq!(s.unit(P1, 1).map(|unit| unit.def_id), Some(def_id.clone()), "{seed}");
            }
        }

        #[test]
        fn r1563000_a_turn_that_ends_with_mana_unspent_summons_nothing() {
            let mut s = heartrock("fvh-036-float", false, 1, json!([]));
            s.end_turn();
            assert!(asteroids(&s).is_empty());
            assert!(s.unit(P1, 1).is_none());
        }

        #[test]
        fn r62_the_opponents_end_of_turn_summons_nothing_however_they_spent() {
            let mut s = scenario(json!({
                "active": "p2",
                "p1": {
                    "hand": [SPELL],
                    "backrow": [{ "def": ID, "faceUp": true }],
                    "library": filler(6),
                },
                "p2": { "hand": [SPELL], "mana": 0, "library": filler(6) },
            }));
            s.end_turn();
            assert!(asteroids(&s).is_empty());
            assert!(s.unit(P1, 1).is_none() && s.unit(P2, 1).is_none());
        }

        #[test]
        fn r129_with_no_open_unit_zone_nothing_is_summoned() {
            let mut s = heartrock("fvh-036-full", false, 0, json!([BODY, BODY, BODY, BODY, BODY]));
            s.end_turn();
            assert!(asteroids(&s).is_empty());
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r62_with_all_its_mana_spent_it_summons_a_random_radiant_asteroid() {
            for n in 0..SEEDS {
                let seed = format!("fvh-036-radiant-{n}");
                let mut s = heartrock(&seed, true, 0, json!([]));
                s.end_turn();
                let summoned = asteroids(&s);
                assert_eq!(summoned.len(), 1, "{seed}: {summoned:?}");
                assert!(summoned[0].2, "{seed}: a Radiant Asteroid");
                assert_eq!(summoned[0].0, P1, "{seed}");
            }
        }
    }
}
