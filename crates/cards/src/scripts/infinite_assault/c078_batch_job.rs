//! SPEC §8.13 IA #78 Batch Job ((3) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Summon four Workers."
//!   Radiant: "Summon four Radiant Workers."
//!
//! IA #7 Bulk Order's summons of IA #78.1, one more of them: four into R64's leftmost free zones,
//! fewer on a fuller board (§3.2); the Radiant face's are Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-078";

/// IA #78.1 Worker.
const WORKER: &str = "infiniteassault-078-1";

/// "four Workers".
const WORKERS: usize = 4;

fn batch_job(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            (0..WORKERS)
                .map(|_| {
                    let mut args = json!({ "defId": WORKER, "player": "self" });
                    if radiant {
                        args["radiant"] = json!(true);
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
        base: batch_job(false),
        radiant: batch_job(true),
    }
}

// IA #78 Batch Job — SPEC §8.13 row 78.
#[cfg(test)]
mod tests {
    use super::{ID, WORKER};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn workers_after(radiant: bool, field: Value) -> (Scenario, Vec<CardInstance>) {
        let mut s = scenario(json!({ "p1": { "hand": [{ "def": ID, "radiant": radiant }], "field": field } }));
        s.play(ID, json!({}));
        let workers = active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == WORKER)
            .cloned()
            .collect();
        (s, workers)
    }

    mod base {
        use super::*;

        #[test]
        fn summons_four_base_workers() {
            let (s, workers) = workers_after(false, json!([]));
            assert_eq!(workers.len(), 4);
            for worker in nonempty(workers, "the Workers") {
                assert!(!worker.radiant);
                assert_eq!((s.stats(&worker).attack, s.stats(&worker).health), (1, 2));
            }
        }

        #[test]
        fn s3_2_a_fuller_board_takes_fewer() {
            let unit = "stand-in (1) 1/1";
            let (_, workers) = workers_after(false, json!([unit, unit]));
            assert_eq!(workers.len(), (UNIT_ZONES - 2) as usize);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn summons_four_radiant_workers() {
            let (s, workers) = workers_after(true, json!([]));
            assert_eq!(workers.len(), 4);
            for worker in nonempty(workers, "the Workers") {
                assert!(worker.radiant);
                assert_eq!((s.stats(&worker).attack, s.stats(&worker).health), (2, 4));
            }
        }
    }
}
