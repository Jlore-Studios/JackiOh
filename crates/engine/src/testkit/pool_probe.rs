//! The pool probe (#3117): a guard that catches a test whose pass depends on where a random pool's roll
//! lands. One more card in a pool moves every draw from it (R380); #848's `seed_where` and `cursor_where`
//! let a test find its roll at run time, and this finds the tests that still pin one.
//!
//! Under the `pool-probe` feature every pool `catalog::query` returns is rotated `POOL_PROBE_SHIFT` places,
//! so a seeded `rng.pick` over it lands on its neighbour, as it would after a card joined the pool. A test
//! that pinned a roll fails; `sh scripts/pool-probe.sh` runs the suites that draw from real cards under it.
//! The shift is a thread-local, 0 in every build without the feature, so no game, replay or hash changes.
//!
//! A rotation, and not an extra card in every pool: a duplicate entry would put a card twice in
//! `filler_deck` and in every deck the AI builds from a pool, and a distinct definition needs a script and
//! a catalog row in every set. A rotation changes no definition, script or deck and moves every draw.
//!
//! Which way a test goes when the probe fails it:
//! - it tests a pool's content or order: read the pool under `with_pool_shift(0, …)`, the pool as it is;
//! - it tests a roll: search for the roll with `seed_where` or `cursor_where`, or build the position directly.
//!
//! The probe does not reach a pool of one card, `roll_weighted`'s tables or the Marketplace's `pool_ids`.

use std::cell::Cell;

use crate::config::POOL_PROBE_SHIFT;
use crate::wire::CardDef;

thread_local! {
    static SHIFT: Cell<usize> = const { Cell::new(if cfg!(feature = "pool-probe") { POOL_PROBE_SHIFT } else { 0 }) };
}

/// How many places this thread's pools are rotated: `POOL_PROBE_SHIFT` under the `pool-probe` feature
/// until a `with_pool_shift` scopes another, 0 without it.
pub fn pool_shift() -> usize {
    SHIFT.with(Cell::get)
}

/// `catalog::query`'s last step: rotate the pool left by this thread's shift.
pub(crate) fn shift_pool(pool: &mut [&'static CardDef]) {
    let by = pool_shift() % pool.len().max(1);
    pool.rotate_left(by);
}

/// This thread's pool shift held at one value until the hold is dropped, the earlier shift then put back
/// (a panic included). `with_pool_shift` is the hold around one call; a test that cannot run in a closure
/// (an async one, on the thread its runtime polls it on) holds it for its length.
#[must_use = "the shift is put back when the hold is dropped"]
pub struct PoolShiftHold(usize);

impl Drop for PoolShiftHold {
    fn drop(&mut self) {
        SHIFT.with(|cell| cell.set(self.0));
    }
}

/// Rotates this thread's pools `shift` places until the returned hold is dropped. `hold_pool_shift(0)`
/// reads pools as they are, for a test of a pool's content or order that is async.
pub fn hold_pool_shift(shift: usize) -> PoolShiftHold {
    PoolShiftHold(SHIFT.with(|cell| cell.replace(shift)))
}

/// `run` with this thread's pools rotated `shift` places, the earlier shift put back after it (a panic
/// included). `with_pool_shift(0, …)` reads pools as they are, for a test of a pool's content or order.
pub fn with_pool_shift<T>(shift: usize, run: impl FnOnce() -> T) -> T {
    let _hold = hold_pool_shift(shift);
    run()
}

/// `ThreadOverrides::install`'s half: sets this thread's shift to the one `thread_overrides` read on
/// another, so a blocking job a test hands off draws from the pools its test does.
pub(crate) fn install_pool_shift(shift: usize) {
    SHIFT.with(|cell| cell.set(shift));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{CatalogQueryArgs, query};
    use crate::rng::Rng;
    use crate::testkit::scenario::{catalog_override, register_catalog};
    use crate::testkit::seed_search::seed_where;
    use crate::wire::{CardDefs, CardType};

    /// The real catalog through the thread-local override (this crate cannot link `jackioh_cards`), as
    /// `seams.rs`'s tests load it.
    fn load_catalog() {
        if catalog_override().is_none() {
            let defs: CardDefs = serde_json::from_str(include_str!("../../../cards/catalog.json"))
                .expect("crates/cards/catalog.json parses as CardDefs");
            register_catalog(defs);
        }
    }

    fn pool() -> Vec<&'static CardDef> {
        query(&CatalogQueryArgs::default())
    }

    fn ids(pool: &[&CardDef]) -> Vec<String> {
        pool.iter().map(|def| def.id.clone()).collect()
    }

    #[test]
    fn a_shifted_pool_keeps_its_cards_and_its_length() {
        load_catalog();
        let mut expected = ids(&with_pool_shift(0, pool));
        assert!(
            expected.len() > 1,
            "the catalog's default pool holds several cards"
        );
        let by = POOL_PROBE_SHIFT % expected.len();
        expected.rotate_left(by);
        assert_eq!(ids(&with_pool_shift(POOL_PROBE_SHIFT, pool)), expected);
    }

    #[test]
    fn the_shift_is_scoped_and_restored_even_after_a_panic() {
        let before = pool_shift();
        with_pool_shift(5, || {
            assert_eq!(pool_shift(), 5);
            with_pool_shift(0, || assert_eq!(pool_shift(), 0));
            assert_eq!(pool_shift(), 5);
            let panicked = std::panic::catch_unwind(|| {
                with_pool_shift(9, || {
                    assert_eq!(pool_shift(), 9);
                    panic!("a test panics inside a shifted pool");
                })
            });
            assert!(panicked.is_err());
            assert_eq!(pool_shift(), 5);
        });
        assert_eq!(pool_shift(), before);
    }

    #[test]
    fn installing_a_threads_overrides_carries_its_shift() {
        // What a test hands a blocking job (`thread_overrides` on the test's thread, `install` on the job's):
        // the job draws from the pools its test does. Both ends run here, the job's inside a scope of its own.
        let handed = with_pool_shift(3, crate::testkit::scenario::thread_overrides);
        with_pool_shift(0, || {
            assert_eq!(pool_shift(), 0);
            handed.install();
            assert_eq!(pool_shift(), 3);
        });
        assert_eq!(
            pool_shift(),
            if cfg!(feature = "pool-probe") {
                POOL_PROBE_SHIFT
            } else {
                0
            }
        );
    }

    #[test]
    fn the_default_shift_follows_the_feature() {
        let expected = if cfg!(feature = "pool-probe") {
            POOL_PROBE_SHIFT
        } else {
            0
        };
        assert_eq!(pool_shift(), expected);
    }

    /// A test that pins where a pool's roll lands is what the probe exists to fail: its pinned card is
    /// the neighbour's once the pool moves. A test that searches for its roll still finds one.
    #[test]
    fn a_test_that_pins_where_a_pool_roll_lands_fails_under_the_probe() {
        load_catalog();
        let roll = |seed: &str, shift: usize| {
            with_pool_shift(shift, || {
                let pool = pool();
                Rng::new(seed, 0)
                    .pick(&pool)
                    .map(|def| (def.id.clone(), def.type_))
            })
            .expect("the default pool is not empty")
        };
        let (pinned, _) = roll("pool-probe-canary", 0);
        let (moved, _) = roll("pool-probe-canary", POOL_PROBE_SHIFT);
        assert_ne!(
            pinned, moved,
            "the same roll lands elsewhere once the pool is rotated"
        );

        for shift in [0, POOL_PROBE_SHIFT] {
            let seed = seed_where("pool-probe-canary", |seed| roll(seed, shift).1 == CardType::Unit);
            assert_eq!(roll(&seed, shift).1, CardType::Unit);
        }
    }
}
