//! SPEC §8.13 IA #73 Worker Thread ((1) Unit, Common, 1/1 → 2/2), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Cry: Summon a copy of this."
//!   Radiant: "Cry: Summon two copies of this."
//!
//! Core #12 Duplicating Felinors' copy of itself (R57, R64): the copy keeps the face that cried, is a
//! summon that fires no Cry of its own (§6.2, R1), and fizzles on a full board.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-073";

/// "a copy", Radiant "two copies".
const COPIES: usize = 1;
const RADIANT_COPIES: usize = 2;

fn worker_thread(copies: usize) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            (0..copies)
                .map(|_| summon_copy(json_as(json!({ "of": { "of": "self" }, "player": "self" }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: worker_thread(COPIES),
        radiant: worker_thread(RADIANT_COPIES),
    }
}

// IA #73 Worker Thread — SPEC §8.13 row 73.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    fn threads_after(radiant: bool, field: Value) -> (Scenario, Vec<CardInstance>) {
        let mut s = scenario(json!({ "p1": { "hand": [{ "def": ID, "radiant": radiant }], "field": field } }));
        s.play(ID, json!({}));
        let threads = active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == ID)
            .cloned()
            .collect();
        (s, threads)
    }

    mod base {
        use super::*;

        #[test]
        fn summons_one_copy_and_the_copy_cries_nothing() {
            let (_, threads) = threads_after(false, json!([]));
            assert_eq!(threads.len(), 2);
            for thread in nonempty(threads, "the Worker Threads") {
                assert!(!thread.radiant);
            }
        }

        #[test]
        fn s3_2_a_full_board_takes_no_copy() {
            let unit = "stand-in (1) 1/1";
            let (_, threads) = threads_after(false, json!([unit, unit, unit, unit]));
            assert_eq!(threads.len(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn summons_two_radiant_copies() {
            let (s, threads) = threads_after(true, json!([]));
            assert_eq!(threads.len(), 3);
            for thread in nonempty(threads, "the Worker Threads") {
                assert!(thread.radiant);
                assert_eq!((s.stats(&thread).attack, s.stats(&thread).health), (2, 2));
            }
        }

        #[test]
        fn s3_2_a_fuller_board_takes_fewer() {
            let unit = "stand-in (1) 1/1";
            let (_, threads) = threads_after(true, json!([unit, unit, unit]));
            assert_eq!(threads.len(), (UNIT_ZONES - 3) as usize);
        }
    }
}
