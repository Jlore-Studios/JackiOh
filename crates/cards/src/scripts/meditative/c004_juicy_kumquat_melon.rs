//! Meditative #4 (stub: replaced by its build).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-004";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}
