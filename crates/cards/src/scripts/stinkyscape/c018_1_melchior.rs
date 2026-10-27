//! SPEC §7 Stinkyscape 18.1 MELCHIOR, a token of the held Stinkyscape set (R1420).
//! STUB (#1857): no behaviour yet; S11 (#1866) writes its script and its tests
//! (CLAUDE.md rule 6). Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-018-1";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}
