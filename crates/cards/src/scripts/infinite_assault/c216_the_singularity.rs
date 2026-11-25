//! IA #216 The Singularity (SPEC §8.13 row 216), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Summon a Singularity."
//!   Radiant: "Summon a Radiant Singularity."
//!
//! Engine: a Cry (R1): `summon` of IA #216.1 Singularity, Radiant on the Radiant face. Built by
//! C9 (#4593).

use jackioh_engine::effects::summon;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-216";

/// IA #216.1 Singularity, the token this summons.
const SINGULARITY: &str = "infiniteassault-216-1";

fn singularity(radiant_face: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![summon(json_as(json!({ "defId": SINGULARITY, "radiant": radiant_face })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: singularity(false),
        radiant: singularity(true),
    }
}

// IA #216 The Singularity — SPEC §8.13 row 216.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::{ID, SINGULARITY};
    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn cast(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.play(ID, json!({}));
        s
    }

    mod base {
        use super::*;

        #[test]
        fn summons_a_plain_singularity() {
            let s = cast(false, "singularity-base");
            let unit = s.unit_at(P1, 1);
            assert_eq!(unit.def_id, SINGULARITY);
            assert!(!unit.radiant);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn summons_a_radiant_singularity() {
            let s = cast(true, "singularity-radiant");
            let unit = s.unit_at(P1, 1);
            assert_eq!(unit.def_id, SINGULARITY);
            assert!(unit.radiant);
        }
    }
}
