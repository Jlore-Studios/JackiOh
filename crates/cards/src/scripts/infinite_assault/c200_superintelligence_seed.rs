//! IA #200 Superintelligence Seed (SPEC §8.13 row 200), a card of the held Infinite Assault
//! set (R4577000).
//!   Base:    "Dormant for 3 turns.\nWhen this awakens, transform it into an Ascendant."
//!   Radiant: "Dormant for 3 turns.\nWhen this awakens, transform it into a Radiant Ascendant."
//!
//! Engine: Dormant (R1839000), `StaticFlags.slumber { turns: 3 }`, awake at the start of its
//! controller's third turn after it arrived (R1839001). `when_this_awakens` (R1839002) Transforms
//! (§6.3) it into IA #200.1 Ascendant, Radiant on the Radiant face. Built by C9 (#4593).

use jackioh_engine::effects::transform;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-200";

/// "Dormant for 3 turns": the status's printed count, no param.
const DORMANT_TURNS: i32 = 3;

/// IA #200.1 Ascendant, the Unit the Seed becomes.
const ASCENDANT: &str = "infiniteassault-200-1";

fn seed(radiant_face: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            slumber: Some(SlumberText {
                turns: Some(DORMANT_TURNS),
                wakes_by: None,
            }),
            ..StaticFlags::default()
        }),
        triggers: vec![when_this_awakens("awakens", move |ctx, _event| {
            let Some(me) = ctx.live_self() else {
                return vec![];
            };
            let id = me.id.clone();
            vec![transform(json_as(json!({
                "instanceId": id,
                "defId": ASCENDANT,
                "radiant": radiant_face,
            })))]
        })],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: seed(false),
        radiant: seed(true),
    }
}

// IA #200 Superintelligence Seed — SPEC §8.13 row 200.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use super::{ASCENDANT, ID};
    use crate::scenario;

    const FILLER: &str = "core-005"; // Stockpile: a play always open, so no turn ends by itself.

    fn awakened(radiant: bool, seed: &str) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "library": crate::filler(4),
                "mana": 10,
            },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }));
        s.play(ID, json!({ "zone": 1 }));
        for _ in 0..6 {
            s.end_turn();
        }
        assert_eq!(s.state().active, P1);
        s
    }

    mod base {
        use super::*;

        #[test]
        fn becomes_a_plain_ascendant_after_3_own_starts() {
            let s = awakened(false, "seed-base");
            let unit = s.unit_at(P1, 1);
            assert_eq!(unit.def_id, ASCENDANT);
            assert!(!unit.radiant);
            let view = unit_view(s.state(), &unit);
            assert_eq!((view.attack, view.health), (12, 12));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn becomes_a_radiant_ascendant_after_3_own_starts() {
            let s = awakened(true, "seed-radiant");
            let unit = s.unit_at(P1, 1);
            assert_eq!(unit.def_id, ASCENDANT);
            assert!(unit.radiant);
            let view = unit_view(s.state(), &unit);
            assert_eq!((view.attack, view.health), (24, 24));
        }
    }
}
