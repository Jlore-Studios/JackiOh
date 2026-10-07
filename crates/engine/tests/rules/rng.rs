// Port of `packages/engine/test/rng.test.ts`: the seeded rng (M1-T2).
//
// TS ran `fixtures/rng-child.ts` in a separate Node process to prove a resumed cursor reproduces the
// sequence anywhere. The Rust port asks the fixture's port (`fixtures/rng_child.rs`) for the same JSON
// it printed — `{ draws, cursor }` from `(seed, cursor, count)`, its argv — so the draws cross a
// serialised boundary built from nothing but the seed and the cursor (see the spec-gaps note).

use jackioh_engine::testkit::*;

use crate::rules::fixtures::rng_child;

/// TS `childDraws`: what the child prints for `(seed, cursor, count)`, parsed.
fn child_draws(seed: &str, cursor: u32, count: u32) -> (Vec<f64>, u32) {
    let out = rng_child::run(&[seed, &cursor.to_string(), &count.to_string()]);
    let parsed: Value = serde_json::from_str(&out).expect("the child prints JSON");
    let draws = parsed["draws"]
        .as_array()
        .expect("draws")
        .iter()
        .map(|draw| draw.as_f64().expect("a draw is a number"))
        .collect();
    let cursor = parsed["cursor"].as_u64().expect("cursor") as u32;
    (draws, cursor)
}

mod seeded_rng_m1_t2 {
    use super::*;

    #[test]
    fn is_deterministic_for_a_seed_and_advances_its_cursor() {
        let mut a = create_rng("seed-a", 0);
        let mut b = create_rng("seed-a", 0);
        assert_eq!([a.next(), a.next(), a.next()], [b.next(), b.next(), b.next()]);
        assert_eq!(a.cursor(), 3);
    }

    #[test]
    fn different_seeds_give_different_sequences() {
        assert_ne!(create_rng("seed-a", 0).next(), create_rng("seed-b", 0).next());
    }

    #[test]
    fn resumes_from_a_serialized_cursor_in_another_process() {
        let mut local = create_rng("match-7", 0);
        let first = vec![local.next(), local.next()];
        let mut resumed = create_rng("match-7", local.cursor());
        let rest = vec![resumed.next(), resumed.next()];

        let (draws, cursor) = child_draws("match-7", 0, 4);
        let mut all = first.clone();
        all.extend(rest.iter().copied());
        assert_eq!(draws, all);
        assert_eq!(cursor, 4);

        let (resumed_draws, _) = child_draws("match-7", 2, 2);
        assert_eq!(resumed_draws, rest);
    }

    #[test]
    fn int_stays_in_range_and_pick_handles_an_empty_list() {
        let mut rng = create_rng("ints", 0);
        for _ in 0..200 {
            let n = rng.int(5);
            assert!(n >= 0);
            assert!(n < 5);
        }
        assert_eq!(rng.int(0), 0);
        let empty: [&str; 0] = [];
        assert!(rng.pick(&empty).is_none());
        assert_eq!(rng.pick(&["only"]), Some(&"only"));
    }

    #[test]
    fn shuffle_keeps_every_element_and_depends_only_on_seed_and_cursor() {
        let list = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let once = create_rng("shuf", 0).shuffle(&list);
        let again = create_rng("shuf", 0).shuffle(&list);
        assert_eq!(once, again);
        let mut sorted = once.clone();
        sorted.sort();
        assert_eq!(sorted, list);
        assert_ne!(once, list);
    }

    #[test]
    fn coin_and_chance_sit_near_their_probabilities() {
        let mut rng = create_rng("coins", 0);
        let mut heads = 0;
        for _ in 0..2000 {
            if rng.coin() {
                heads += 1;
            }
        }
        assert!(heads > 900, "{heads}");
        assert!(heads < 1100, "{heads}");

        let mut rng2 = create_rng("chance", 0);
        let mut hits = 0;
        for _ in 0..2000 {
            if rng2.chance(0.3) {
                hits += 1;
            }
        }
        assert!(hits > 500, "{hits}");
        assert!(hits < 700, "{hits}");
        assert!(!create_rng("never", 0).chance(0.0));
        assert!(create_rng("always", 0).chance(1.0));
    }

    #[test]
    fn lucky_1_rolls_twice_and_keeps_the_better_result_lucky_0_rolls_once() {
        let mut rng = create_rng("lucky", 0);
        let better = |a: i32, b: i32| a.max(b);

        let mut calls = 0;
        let kept = rng.lucky(
            1,
            |_| {
                calls += 1;
                if calls == 1 { 3 } else { 9 }
            },
            better,
        );
        assert_eq!(kept, 9);
        assert_eq!(calls, 2);

        calls = 0;
        let kept = rng.lucky(
            0,
            |_| {
                calls += 1;
                if calls == 1 { 3 } else { 9 }
            },
            better,
        );
        assert_eq!(kept, 3);
        assert_eq!(calls, 1);

        calls = 0;
        rng.lucky(
            3,
            |_| {
                calls += 1;
                if calls == 1 { 3 } else { 9 }
            },
            better,
        );
        assert_eq!(calls, 4);
    }
}
