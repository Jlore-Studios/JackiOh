//! `jackioh_engine::testkit` (feature `testkit`, SURFACE §8): `packages/cards/test/_harness.ts`
//! (`scenario`), `_invariants.ts` (`invariants`, the I1–I4 monitor `cargo jackioh fuzz` runs) and
//! `_glow.ts` (`glow`) in Rust, plus the whole engine (`pub use crate::*`), so a ported test names
//! every engine item — `make_context`, `new_instance`, `place_on_field`, `move_to_zone`, `settle` …
//! — as `jackioh_engine::testkit::*` gives it. Written once by part 1; the three files are part 5's.
//!
//! `register_scripts` and `register_catalog` are the harness's (`scenario.rs`): they set the
//! thread-local override that `catalog::registered_catalog()` and `scripts::script_of()` consult
//! first under this feature (SURFACE §8), and they win here over the production
//! `scripts::register_scripts`/`catalog::register_catalog` that `crate::*` also brings.
//!
//! The effects library is not globbed in (its `draw`, `add_to_hand`, `gain_mana`, … share names with
//! engine functions): a test names `effects::<verb>` or imports the verbs it uses.

pub mod glow;
pub mod invariants;
pub mod scenario;

pub use crate::*;

pub use glow::*;
pub use invariants::*;
pub use scenario::*;

// The thread-local override wins over the production registries `crate::*` re-exports (SURFACE §8).
pub use scenario::{register_catalog, register_scripts};

pub use crate::prelude::json_as;
pub use indexmap::{IndexMap, IndexSet};
pub use serde_json::{self, Value, json};
