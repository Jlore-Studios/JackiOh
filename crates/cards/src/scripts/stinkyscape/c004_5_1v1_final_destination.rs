//! SPEC §7 Stinkyscape 4.5 1v1 Final Destination, a token of the held Stinkyscape set (R1420).
//! STUB (#1857): no behaviour yet; S22 (#1870) writes its script and its tests
//! (CLAUDE.md rule 6). Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-004-5";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}
