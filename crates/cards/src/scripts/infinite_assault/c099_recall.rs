//! SPEC §8.13 IA #99 Recall ((1) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Add a copy of a random friendly Unit that died this game to your hand."
//!   Radiant: "Add two copies of a random friendly Unit that died this game to your hand."
//!
//! IA #110 Infinite Loop's shape, adding to hand instead of summoning: one match-rng pick (R60)
//! over the controller's `deaths_this_game` log (R4589000), a fresh copy on the face it died with
//! (R4577002), two copies from the one pick on the Radiant face (R4589001). With no deaths it does
//! nothing (R129).

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-099";

/// "a copy", the base face's.
const COPIES: i32 = 1;
/// "two copies", the Radiant face's.
const RADIANT_COPIES: i32 = 2;

fn recall(copies: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let deaths = deaths_this_game(&*ctx.state, ctx.controller);
            let Some(picked) = ctx.rng.pick(&deaths).cloned() else {
                return vec![];
            };
            (0..copies)
                .map(|_| {
                    add_to_hand(json_as(json!({
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
        base: recall(COPIES),
        radiant: recall(RADIANT_COPIES),
    }
}

// IA #99 Recall — SPEC §8.13 row 99 (R4589001).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const FODDER: &str = "stand-in (1) 2/2";
    const FILLER: &str = "stand-in (0) Spell";

    /// p1's Fodder dies to Hit Job then Recall is cast, with IA #97 in the library to keep the log.
    fn recalled(radiant: bool) -> Scenario {
        let mut s = scenario(json!({
            "p1": {
                "hand": [HIT_JOB, { "def": ID, "radiant": radiant }],
                "field": [FODDER],
                "library": ["infiniteassault-097", FILLER, FILLER],
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(3) },
        }));
        let fodder = s.unit_at(P1, 1);
        s.play(HIT_JOB, pick::targets(pick::instance(&fodder)));
        s.play(ID, json!({}));
        s
    }

    fn recalled_count(s: &Scenario) -> usize {
        s.hand(P1).iter().filter(|card| card.def_id == FODDER).count()
    }

    mod base {
        use super::*;

        #[test]
        fn r4589001_adds_a_unit_that_died() {
            let s = recalled(false);
            assert_eq!(recalled_count(&s), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4589001_adds_two_copies_of_a_unit_that_died() {
            let s = recalled(true);
            assert_eq!(recalled_count(&s), 2);
        }
    }
}
