//! R3563000 (issue #3563, Stinkyscape+ E9): the testkit's switch for `UNIQUE_RANDOM_DRAWS`,
//! which has been on since the Stinkyscape+ release (#3573), so a test that wants the old repeating draws
//! switches it off for one call. Set by `with_unique_draws` for that one call and put back after it (a panic
//! included). Not handed to another thread (`thread_overrides`): no test runs a unique draw there.

use std::cell::Cell;

thread_local! {
    static UNIQUE: Cell<Option<bool>> = const { Cell::new(None) };
}

/// R3563000: the unique-draws override on this thread, if any.
pub fn unique_draws_override() -> Option<bool> {
    UNIQUE.with(Cell::get)
}

/// R3563000: `run` with unique random draws switched `on`, the earlier value put back after it (a
/// panic included), so tests prove the rule while the release flag stays off.
pub fn with_unique_draws<T>(on: bool, run: impl FnOnce() -> T) -> T {
    struct Restore(Option<bool>);
    impl Drop for Restore {
        fn drop(&mut self) {
            UNIQUE.with(|cell| cell.set(self.0));
        }
    }
    let _restore = Restore(UNIQUE.with(|cell| cell.replace(Some(on))));
    run()
}
