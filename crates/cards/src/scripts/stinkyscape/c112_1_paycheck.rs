//! SPEC §7 Stinkyscape 112.1 Paycheck, a token of the held Stinkyscape set (R1420).
//! STUB (#1857): no behaviour yet; S10 (#1865) writes its script and its tests
//! (CLAUDE.md rule 6). Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-112-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}
