//! The seed search (issue #848): a test that needs a random roll to land one way finds the seed or the rng
//! cursor at run time, instead of pinning one that every card or keyword joining a pool moves. Both
//! searches are deterministic (a fixed prefix or seed, tried in order), so a test replays the same
//! position every run; each is bounded by `SEED_SEARCH_BOUND` and panics naming what it searched when
//! nothing matches.

use crate::config::SEED_SEARCH_BOUND;

/// The first seed `<prefix>-0`, `<prefix>-1`, … that `accept` takes, for a test whose position needs a
/// roll to land one way. `accept` builds the position from the seed and reads where the roll landed; the
/// test then builds the same position on the seed it got and runs its assertions as they are.
pub fn seed_where(prefix: &str, accept: impl Fn(&str) -> bool) -> String {
    (0..SEED_SEARCH_BOUND)
        .map(|i| format!("{prefix}-{i}"))
        .find(|seed| accept(seed))
        .unwrap_or_else(|| {
            panic!(
                "no seed among the first {SEED_SEARCH_BOUND} (\"{prefix}-0\" to \"{prefix}-{}\") is accepted",
                SEED_SEARCH_BOUND - 1
            )
        })
}

/// The first rng cursor below `SEED_SEARCH_BOUND` that `accept` takes, for a test that sets
/// `state.rng_cursor` so a seed's next roll is the one it needs.
pub fn cursor_where(seed: &str, accept: impl Fn(u32) -> bool) -> u32 {
    let bound = u32::try_from(SEED_SEARCH_BOUND).unwrap_or(u32::MAX);
    (0..bound).find(|cursor| accept(*cursor)).unwrap_or_else(|| {
        panic!(
            "no cursor among the first {SEED_SEARCH_BOUND} (0 to {}) is accepted from seed \"{seed}\"",
            SEED_SEARCH_BOUND - 1
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::Rng;

    fn panic_message(run: impl FnOnce() + std::panic::UnwindSafe) -> String {
        let payload = std::panic::catch_unwind(run).err();
        payload
            .and_then(|payload| payload.downcast_ref::<String>().cloned())
            .unwrap_or_default()
    }

    #[test]
    fn seed_where_returns_the_first_seed_accepted() {
        let found = seed_where("seed-search", |seed| seed.ends_with("-7"));
        assert_eq!(found, "seed-search-7");
    }

    #[test]
    fn seed_where_names_the_prefix_and_the_bound_when_nothing_matches() {
        let message = panic_message(|| {
            seed_where("seed-search-none", |_| false);
        });
        assert!(message.contains("seed-search-none-0"), "{message}");
        assert!(
            message.contains(&format!("seed-search-none-{}", SEED_SEARCH_BOUND - 1)),
            "{message}"
        );
    }

    #[test]
    fn cursor_where_returns_the_first_cursor_accepted_and_replays() {
        let rolls_even = |cursor: u32| Rng::new("cursor-search", cursor).int(2) == 0;
        let found = cursor_where("cursor-search", rolls_even);
        assert_eq!(found, cursor_where("cursor-search", rolls_even));
        assert!(rolls_even(found));
        assert!((0..found).all(|cursor| !rolls_even(cursor)));
    }

    #[test]
    fn cursor_where_names_the_seed_and_the_bound_when_nothing_matches() {
        let message = panic_message(|| {
            cursor_where("cursor-search-none", |_| false);
        });
        assert!(message.contains("cursor-search-none"), "{message}");
        assert!(message.contains(&SEED_SEARCH_BOUND.to_string()), "{message}");
    }
}
