//! `jackioh_engine::testkit` (feature `testkit`, SURFACE §8): `scenario` (the harness), `invariants` (the
//! I1–I4 monitor `cargo jackioh fuzz` runs) and `glow`, plus the whole engine (`pub use crate::*`).
//!
//! `register_scripts` and `register_catalog` (`scenario.rs`) set the thread-local override that
//! `catalog::registered_catalog()` and `scripts::script_of()` consult first under this feature, so they win
//! over the production versions `crate::*` also brings.
//!
//! `seed_search.rs` is #848's seed search: `seed_where` and `cursor_where` find the seed or rng cursor where
//! a pool's roll lands as a test needs. `preview.rs` is R1420's preview of a set that does not ship yet,
//! `duos_scenario.rs` #936's begun Duos game. `golden.rs` (§13.3, #1641) and `pick.rs` (the one-instance pick
//! builders, #1826) are not globbed in: callers name the module (`golden::<item>`). `must` (#2023) comes in
//! with the glob, the effects library (its verbs share names with engine functions) does not.

pub mod duos_scenario;
pub mod glow;
pub mod golden;
pub mod invariants;
pub mod pick;
pub mod preview;
pub mod scenario;
pub mod seams;
pub mod seed_search;

pub use crate::*;

pub use duos_scenario::*;
pub use glow::*;
pub use invariants::*;
pub use preview::*;
pub use scenario::*;
pub use seams::*;
pub use seed_search::*;

// The thread-local override wins over the production registries `crate::*` re-exports.
pub use scenario::{register_catalog, register_scripts, register_shipped};

pub use crate::prelude::json_as;
pub use indexmap::{IndexMap, IndexSet};
pub use serde_json::{self, Value, json};
