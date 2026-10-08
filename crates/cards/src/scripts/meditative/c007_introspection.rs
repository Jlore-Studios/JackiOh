//! Meditative #7 (stub: replaced by its build).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-007";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}
