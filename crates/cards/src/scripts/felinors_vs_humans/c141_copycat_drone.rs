//! SPEC §8.14 FvH #141 Plagiarized: Copycat Drone, a frozen copy of IA #173 Copycat Drone (R2195003, FV4), a
//! card of the held Felinors vs Humans set, built by #5235.
//!
//! The code is a copy of the IA card's as it stood when the set was founded, not a re-export, so a later IA
//! patch does not move it (R2195003). Its `ID` is its own, so every read of it names this card; it is named
//! `Plagiarized: Copycat Drone`, tagged Plagiarized only, and a token it makes stays the IA token. Its change
//! from IA #173: none. The IA file's notes, as they stood:
//!
//! SPEC §8.13 IA #173 Copycat Drone ((2) Unit, Common, 2/3 → 4/6), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Transform this into a copy of the last Unit your opponent played."
//!   Radiant: "Cry: Transform this into a Radiant copy of the last Unit your opponent played."
//!
//! Built by C8 (#4592). R4592000, R4592007.

use jackioh_engine::effects::{TransformArgs, TransformTarget, transform};
use jackioh_engine::recursion_log::last_unit_played;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-141";

fn copycat(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let opp = opponent_of(ctx.controller);
            let Some(last) = last_unit_played(ctx.state, opp) else {
                return vec![];
            };
            vec![transform(TransformArgs {
                target: TransformTarget {
                    target: Some(TargetSpec::SelfCard),
                    instance_id: None,
                },
                def_id: last.def_id.clone(),
                radiant: if radiant {
                    Some(true)
                } else {
                    Some(last.radiant)
                },
                cost_override: None,
                chinese: last.chinese,
            })]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: copycat(false),
        radiant: copycat(true),
    }
}

// FvH #141 Plagiarized: Copycat Drone — SPEC §8.14 row 141; IA #173's tests (SPEC §8.13 row 173), on this
// card.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const FILLER: &str = "core-005";
    const TARGET_UNIT: &str = "core-019"; // Midrange Menace

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "mana": 10 },
            "p2": { "hand": [TARGET_UNIT], "mana": 10 },
            "active": "p2",
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r4592007_transforms_into_copy_of_last_unit_opponent_played() {
            let mut s = board(false);
            s.play(TARGET_UNIT, json!({ "zone": 1 }));
            s.end_turn();
            s.play(ID, json!({ "zone": 1 }));
            let transformed = s.unit_at(P1, 1);
            assert_eq!(transformed.def_id, TARGET_UNIT);
            assert!(!transformed.radiant);
        }

        #[test]
        fn with_no_unit_played_by_opponent_does_not_transform() {
            let mut s = board(false);
            s.end_turn();
            s.play(ID, json!({ "zone": 1 }));
            let drone = s.unit_at(P1, 1);
            assert_eq!(drone.def_id, ID);
            s.expect_stats(&drone, json!({ "attack": 2, "health": 3 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4592007_transforms_into_radiant_copy() {
            let mut s = board(true);
            s.play(TARGET_UNIT, json!({ "zone": 1 }));
            s.end_turn();
            s.play(ID, json!({ "zone": 1 }));
            let transformed = s.unit_at(P1, 1);
            assert_eq!(transformed.def_id, TARGET_UNIT);
            assert!(transformed.radiant);
        }
    }
}
