//! SPEC §8.13 IA #110 Infinite Loop ((3) Field Spell, Rare), a card of the held Infinite Assault set (R4577000).
//!   Base:    "End of turn: Summon a copy of a random friendly Unit that died this turn."
//!   Radiant: "End of turn: Summon two copies of a random friendly Unit that died this turn."
//!
//! One match-rng pick (R60) over the controller's `deaths_this_turn` log (R4589000, R4589001), then
//! a fresh copy of it on the face it died with (R4577002) — two copies from the one pick on the
//! Radiant face (R4589001). With no deaths it does nothing (R129); the summon has no Cry (R1).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-110";

/// "a copy", the base face's.
const COPIES: i32 = 1;
/// "two copies", the Radiant face's.
const RADIANT_COPIES: i32 = 2;

fn infinite_loop(copies: i32) -> Script {
    Script {
        end_of_turn: Some(hook(move |ctx| {
            let deaths = deaths_this_turn(&*ctx.state, ctx.controller);
            let Some(picked) = ctx.rng.pick(&deaths).cloned() else {
                return vec![];
            };
            (0..copies)
                .map(|_| {
                    summon(json_as(json!({
                        "defId": picked.def_id.clone(),
                        "radiant": picked.radiant,
                    })))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: infinite_loop(COPIES),
        radiant: infinite_loop(RADIANT_COPIES),
    }
}

// IA #110 Infinite Loop — SPEC §8.13 row 110 (R4589001).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const FODDER: &str = "stand-in (1) 2/2";
    const FILLER: &str = "stand-in (0) Spell";

    /// p1's Loop stands in the backrow and its Fodder in lane 1; the Fodder dies, then the turn ends.
    fn looped(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [HIT_JOB],
                "field": [FODDER],
                "backrow": [{ "def": ID, "radiant": radiant }],
                "library": ["infiniteassault-097", FILLER, FILLER],
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(3) },
        }));
        let fodder = s.unit_at(P1, 1);
        s.play(HIT_JOB, pick::targets(pick::instance(&fodder)));
        s
    }

    fn copies(s: &Scenario) -> usize {
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == FODDER)
            .count()
    }

    mod base {
        use super::*;

        #[test]
        fn r4589001_summons_a_unit_that_died_this_turn() {
            let mut s = looped(false);
            s.end_turn();
            assert_eq!(copies(&s), 1);
        }

        #[test]
        fn no_death_no_summon() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [FILLER],
                    "field": [FODDER],
                    "backrow": [{ "def": ID }],
                    "library": ["infiniteassault-097", FILLER, FILLER],
                },
                "p2": { "hand": [FILLER], "library": crate::filler(3) },
            }));
            s.end_turn();
            assert_eq!(copies(&s), 1, "only the Fodder that never died stands");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4589001_summons_two_copies_of_a_unit_that_died_this_turn() {
            let mut s = looped(true);
            s.end_turn();
            assert_eq!(copies(&s), 2);
        }
    }
}
