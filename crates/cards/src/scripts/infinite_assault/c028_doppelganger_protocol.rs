//! IA #28 Doppelganger Protocol (SPEC §8.13 row 28), a card of the held Infinite Assault
//! set (R4577000).
//!   Base:    "Summon a copy of a random enemy Unit. It has Brittle 1."
//!   Radiant: "Summon a copy of a random enemy Unit. It has Brittle 3."
//!
//! The pick is drawn among the enemy Units a Spell may reach (R4453000); a single candidate
//! draws no rng, and with none nothing is summoned (R800, R129). The copy lands for its
//! controller (R57) and is given Brittle once it stands (R4586009).
//!
//! Engine: `summon_copy`, `give_brittle` (R4586009). Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-028";

/// "Brittle N" on the copy: the Radiant face's 3 fades slower than the base face's 1.
/// A Spell declares no number it prints unless the cross test names why it is fixed
/// (`tests/cross/params.rs`, R1748003): the two counts are that file's `FIXED` Brittle rows.
const BASE_BRITTLE: i32 = 1;
const RADIANT_BRITTLE: i32 = 3;

/// The enemy Units a Spell may reach, by id.
fn reachable(c: &mut EffectContext<'_>) -> Vec<String> {
    let enemy = opponent_of(c.controller);
    let source = c.self_.clone();
    active_units_of(&*c.state, enemy)
        .into_iter()
        .filter(|card| !spell_cannot_reach(&*c.state, source.as_ref(), card))
        .map(|card| card.id.clone())
        .collect()
}

/// One random candidate's id, or the candidate's when only one stands (R800), or none.
fn pick(c: &mut EffectContext<'_>) -> Vec<String> {
    let pool = reachable(c);
    if pool.len() <= 1 {
        return pool;
    }
    c.sink.rng.pick(&pool).cloned().into_iter().collect()
}

fn protocol(brittle: i32) -> Script {
    Script {
        cry: Some(hook(move |_| {
            vec![
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|c: &mut EffectContext<'_>| pick(c)),
                    each: Arc::new(|instance_id: &str| {
                        summon_copy(json_as(json!({
                            "of": { "of": "instance", "instanceId": instance_id },
                            "player": "self",
                        })))
                    }),
                }),
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|ctx: &mut EffectContext<'_>| summoned_so_far(ctx)),
                    each: Arc::new(move |instance_id: &str| {
                        give_brittle(json_as(json!({ "instanceId": instance_id, "n": brittle })))
                    }),
                }),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: protocol(BASE_BRITTLE),
        radiant: protocol(RADIANT_BRITTLE),
    }
}

// IA #28 Doppelganger Protocol — SPEC §8.13 row 28 (R4586009).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const LEFT: &str = "core-001"; // A 1/7.
    const RIGHT: &str = "core-008"; // A 2/4.

    fn cast(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, STOCKPILE],
                "library": crate::filler(4),
            },
            "p2": { "field": [LEFT, RIGHT], "hand": [STOCKPILE], "library": crate::filler(4) },
        }));
        s.play(ID, json!({}));
        s
    }

    fn copies(s: &Scenario) -> Vec<CardInstance> {
        s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| [LEFT, RIGHT].contains(&card.def_id.as_str()))
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4586009_a_random_enemy_unit_is_copied_with_brittle_1() {
            let mut seen = [false, false];
            for seed in 0..16 {
                let s = cast(false, &format!("protocol-{seed}"));
                let landed = copies(&s);
                assert_eq!(landed.len(), 1, "one copy");
                let copy = &landed[0];
                assert_eq!(copy.brittle.map(|brittle| brittle.count), Some(1));
                match copy.def_id.as_str() {
                    LEFT => seen[0] = true,
                    RIGHT => seen[1] = true,
                    other => panic!("a copy of {other}"),
                }
            }
            assert!(seen[0] && seen[1], "both Units are drawn");
        }

        #[test]
        fn r4586009_with_no_enemy_unit_nothing_happens() {
            let mut s = scenario(json!({
                "seed": "protocol-empty",
                "p1": { "hand": [ID, STOCKPILE], "library": crate::filler(4) },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            s.play(ID, json!({}));
            assert!(copies(&s).is_empty());
            assert_eq!(s.state().players[P1].units.iter().flatten().flatten().count(), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586009_the_copy_has_brittle_3() {
            let s = cast(true, "protocol-radiant");
            let landed = copies(&s);
            assert_eq!(landed.len(), 1);
            assert_eq!(landed[0].brittle.map(|brittle| brittle.count), Some(3));
        }
    }
}
