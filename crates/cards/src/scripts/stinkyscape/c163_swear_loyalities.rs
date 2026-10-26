//! SPEC §8.10 Stinkyscape 163 Swear Loyalities, a card of the held Stinkyscape set (R1420).
//! STUB (#1857): no behaviour yet; S24 (#1880) writes its script and its tests
//! (CLAUDE.md rule 6). Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-163";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}
