//! `jackioh_engine::testkit` (feature `testkit`, SURFACE §8): the harness (`scenario`), the fuzz's
//! invariant monitor, `glow`, #848's seed search (a seed or rng cursor found at run time, so no test
//! pins one a pool change would move), the test seams, R1420's preview of a set that does not ship yet
//! and #936's begun Duos game, plus the whole engine (`pub use crate::*`), so a test names every engine
//! item as `jackioh_engine::testkit::*` gives it.
//!
//! `register_scripts` and `register_catalog` are the harness's: they set the thread-local override
//! that `catalog::registered_catalog()` and `scripts::script_of()` consult first under this feature.
//!
//! Not globbed in, so a caller names the module: `golden` (the golden replay's pure half, §13.3,
//! #1641), `pick` (the one-instance pick builders, #1826) and the effects library, whose `draw`,
//! `add_to_hand`, `gain_mana`, … share names with engine functions. `scenario.rs`'s `must` (#2023)
//! comes in with the glob.

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
