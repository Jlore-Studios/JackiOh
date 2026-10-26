//! SPEC §8.10 Stinkyscape 90 After Burner, a card of the held Stinkyscape set (R1420).
//! STUB (#1857): no behaviour yet; S02 (#1859) writes its script and its tests
//! (CLAUDE.md rule 6). Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-090";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}
