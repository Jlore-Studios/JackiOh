//! IA #45 Dream Within a Dream (SPEC §8.13 row 45), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Summon a copy of every enemy Unit for yourself. They have Brittle 1."
//!   Radiant: "Summon a copy of every enemy Unit for yourself. They have Brittle 2."
//!
//! Every enemy Unit a Spell may reach is copied (R4453000); the copies land for their
//! controller (R57) and are given Brittle once they stand, one count on the base face and two
//! on the Radiant face. With no enemy Unit, nothing is summoned and nothing is given (R129).
//!
//! Engine: `summon_copy`, `give_brittle` (R4586009). Built by C2 (#4586).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-045";

/// "Brittle N" on the copies.
/// A Spell declares no number it prints unless the cross test names why it is fixed
/// (`tests/cross/params.rs`, R1748003): the two counts are that file's `FIXED` Brittle rows.
const BASE_BRITTLE: i32 = 1;
const RADIANT_BRITTLE: i32 = 2;

fn dream(brittle: i32) -> Script {
    Script {
        cry: Some(hook(move |_| {
            vec![
                for_each_card(ForEachCardArgs {
                    cards: Arc::new(|c: &mut EffectContext<'_>| {
                        let enemy = opponent_of(c.controller);
                        let source = c.self_.clone();
                        active_units_of(&*c.state, enemy)
                            .into_iter()
                            .filter(|card| {
                                !spell_cannot_reach(&*c.state, source.as_ref(), card)
                            })
                            .map(|card| card.id.clone())
                            .collect()
                    }),
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
        base: dream(BASE_BRITTLE),
        radiant: dream(RADIANT_BRITTLE),
    }
}

// IA #45 Dream Within a Dream — SPEC §8.13 row 45 (R4586009).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.
    const LEFT: &str = "core-001";
    const RIGHT: &str = "core-008";

    const ENEMIES: [&str; 2] = [LEFT, RIGHT];

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

    fn copies(s: &Scenario, brittle: i32) -> Vec<CardInstance> {
        let landed: Vec<CardInstance> = s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .filter(|card| ENEMIES.contains(&card.def_id.as_str()))
            .cloned()
            .collect();
        assert!(
            landed.iter().all(|card| card.brittle.map(|b| b.count) == Some(brittle)),
            "every copy has Brittle {brittle}"
        );
        landed
    }

    mod base {
        use super::*;

        #[test]
        fn r4586009_every_enemy_unit_is_copied_with_brittle_1() {
            let s = cast(false, "dream-base");
            let landed = copies(&s, 1);
            assert_eq!(landed.len(), 2, "both enemies copied");
            let mut defs: Vec<&str> = landed.iter().map(|card| card.def_id.as_str()).collect();
            defs.sort_unstable();
            assert_eq!(defs, ENEMIES);
        }

        #[test]
        fn r4586009_with_no_enemy_unit_nothing_happens() {
            let mut s = scenario(json!({
                "seed": "dream-empty",
                "p1": { "hand": [ID, STOCKPILE], "library": crate::filler(4) },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            s.play(ID, json!({}));
            assert_eq!(s.state().players[P1].units.iter().flatten().flatten().count(), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4586009_the_copies_have_brittle_2() {
            let s = cast(true, "dream-radiant");
            assert_eq!(copies(&s, 2).len(), 2);
        }
    }
}
