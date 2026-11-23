//! SPEC §8.13 IA #10 Assault Carrier ((4) Unit, Common, 7/8 → 14/16), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Death: Summon two Drone Escorts."
//!   Radiant: "Death: Summon two Radiant Drone Escorts."
//!
//! Two summons of IA #10.1 from its Death, each into R64's leftmost free zone, the first where the
//! Carrier stood.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-010";

/// IA #10.1 Drone Escort.
const ESCORT: &str = "infiniteassault-010-1";

/// "two Drone Escorts".
const ESCORTS: usize = 2;

fn carrier(radiant: bool) -> Script {
    Script {
        death: Some(hook(move |_ctx| {
            (0..ESCORTS)
                .map(|_| summon(json_as(json!({ "defId": ESCORT, "player": "self", "radiant": radiant }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: carrier(false),
        radiant: carrier(true),
    }
}

// IA #10 Assault Carrier — SPEC §8.13 row 10.
#[cfg(test)]
mod tests {
    use super::{ESCORT, ID};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill

    fn escorts_after_its_death(radiant: bool) -> Vec<CardInstance> {
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB], "field": [{ "def": ID, "radiant": radiant }], "mana": 10 },
        }));
        let carrier = s.unit_at(P1, 1);
        s.play(HIT_JOB, pick::targets(pick::instance(&carrier)));
        s.expect_in_zone(&carrier, "graveyard");
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == ESCORT)
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn death_summons_two_drone_escorts() {
            let escorts = escorts_after_its_death(false);
            assert_eq!(escorts.len(), 2);
            for escort in nonempty(escorts, "the Escorts") {
                assert!(!escort.radiant);
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn death_summons_two_radiant_drone_escorts() {
            let escorts = escorts_after_its_death(true);
            assert_eq!(escorts.len(), 2);
            for escort in nonempty(escorts, "the Escorts") {
                assert!(escort.radiant);
            }
        }
    }
}
