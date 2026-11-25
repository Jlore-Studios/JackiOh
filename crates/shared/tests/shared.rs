//! The shared crate's one integration-test binary (SURFACE §1). The test modules stay in their original
//! files so rulings' `proven_in` paths remain valid.
//!
//! No module in a test path may be named `fuzz`, `selfplay` or `positions`, or end in `fuzz`:
//! `.github/ci/rust-test.toml` skips those paths in the `tools, wasm` group.

#[path = "again.rs"]
mod again;
#[path = "board.rs"]
mod board;
#[path = "bots_boundary.rs"]
mod bots_boundary;
#[path = "bots_cosmetics.rs"]
mod bots_cosmetics;
#[path = "bots_decide.rs"]
mod bots_decide;
#[path = "bots_persona.rs"]
mod bots_persona;
#[path = "bots_pick.rs"]
mod bots_pick;
#[path = "bots_pool.rs"]
mod bots_pool;
#[path = "bots_signals.rs"]
mod bots_signals;
#[path = "boundary.rs"]
mod boundary;
#[path = "clock.rs"]
mod clock;
#[path = "daily.rs"]
mod daily;
#[path = "daily_invariants.rs"]
mod daily_invariants;
#[path = "duel.rs"]
mod duel;
#[path = "duos.rs"]
mod duos;
#[path = "economy.rs"]
mod economy;
#[path = "face.rs"]
mod face;
#[path = "glicko2.rs"]
mod glicko2;
#[path = "host_boundary.rs"]
mod host_boundary;
// The CI skip filter `fuzz::` also matches a module name ending in `fuzz`.
#[path = "host_fuzz.rs"]
mod host_soak;
#[path = "ladder.rs"]
mod ladder;
#[path = "loadout.rs"]
mod loadout;
#[path = "loadout_freeze.rs"]
mod loadout_freeze;
#[path = "log.rs"]
mod log;
#[path = "practice.rs"]
mod practice;
#[path = "ranked_game.rs"]
mod ranked_game;
#[path = "rematch.rs"]
mod rematch;
#[path = "season.rs"]
mod season;
#[path = "series.rs"]
mod series;
