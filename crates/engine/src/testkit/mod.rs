//! `jackioh_engine::testkit` (feature `testkit`, §8): `scenario` (the harness), `invariants` (the I1–I4
//! monitor `cargo jackioh fuzz` runs) and `glow`, plus the whole engine (`pub use crate::*`), so a test
//! names every engine item as `jackioh_engine::testkit::*` gives it.
//!
//! `register_scripts` and `register_catalog` are the harness's (`scenario.rs`): they set the
//! thread-local override that `catalog::registered_catalog()` and `scripts::script_of()` consult
//! first under this feature, and they win here over the production versions `crate::*` also brings.
//!
//! `seams.rs` holds the two test seams made from outside the engine: a test-made work handler and
//! stand-ins for three turn stages. `preview.rs` is R1420's preview of a set that does not ship yet.
//!
//! The effects library is not globbed in (its `draw`, `add_to_hand`, `gain_mana`, … share names with
//! engine functions): a test names `effects::<verb>` or imports the verbs it uses.

pub mod glow;
pub mod invariants;
pub mod preview;
pub mod scenario;
pub mod seams;

pub use crate::*;

pub use glow::*;
pub use invariants::*;
pub use preview::*;
pub use scenario::*;
pub use seams::*;

// The thread-local override wins over the production registries `crate::*` re-exports.
pub use scenario::{register_catalog, register_scripts};

pub use crate::prelude::json_as;
pub use indexmap::{IndexMap, IndexSet};
pub use serde_json::{self, Value, json};
