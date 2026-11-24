//! SPEC §8.13 IA #82 Fork Bomb ((2) Unit, Rare, 2/2 → 4/4), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Death: Summon two Forks."
//!   Radiant: "Death: Summon two Radiant Forks. They have Rush."
//!
//! Two summons of IA #82.1 from its Death into R64's leftmost free zones; the Radiant face's are
//! Radiant and given Rush as they land (S T-AI-8 Drone Uplink's `keywords`, as IA #7's are).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-082";

/// IA #82.1 Fork.
const FORK: &str = "infiniteassault-082-1";

/// "two Forks".
const FORKS: usize = 2;

fn fork_bomb(radiant: bool) -> Script {
    Script {
        death: Some(hook(move |_ctx| {
            (0..FORKS)
                .map(|_| {
                    let mut args = json!({ "defId": FORK, "player": "self" });
                    if radiant {
                        args["radiant"] = json!(true);
                        args["keywords"] = json!([{ "kind": "Rush" }]);
                    }
                    summon(json_as(args))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: fork_bomb(false),
        radiant: fork_bomb(true),
    }
}

// IA #82 Fork Bomb — SPEC §8.13 row 82.
#[cfg(test)]
mod tests {
    use super::{FORK, ID};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill

    fn forks_after(radiant: bool) -> (Scenario, Vec<CardInstance>) {
        let mut s = scenario(json!({
            "p1": { "hand": [HIT_JOB], "field": [{ "def": ID, "radiant": radiant }], "mana": 10 },
            "p2": { "library": crate::filler(2) },
        }));
        let bomb = s.unit_at(P1, 1);
        s.play(HIT_JOB, pick::targets(pick::instance(&bomb)));
        let forks = active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == FORK)
            .cloned()
            .collect();
        (s, forks)
    }

    mod base {
        use super::*;

        #[test]
        fn death_summons_two_forks_without_rush() {
            let (s, forks) = forks_after(false);
            assert_eq!(forks.len(), 2);
            for fork in nonempty(forks, "the Forks") {
                assert!(!fork.radiant);
                assert!(!has_keyword(&s.stats(&fork).keywords, KeywordKind::Rush));
                assert_eq!((s.stats(&fork).attack, s.stats(&fork).health), (1, 1));
            }
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn death_summons_two_radiant_forks_with_rush() {
            let (s, forks) = forks_after(true);
            assert_eq!(forks.len(), 2);
            for fork in nonempty(forks, "the Forks") {
                assert!(fork.radiant);
                assert!(has_keyword(&s.stats(&fork).keywords, KeywordKind::Rush));
                assert_eq!((s.stats(&fork).attack, s.stats(&fork).health), (2, 2));
            }
        }
    }
}
