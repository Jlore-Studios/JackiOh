//! SPEC §7 Stinkyscape T-anomaly-found Anomaly Found!, a token of the held Stinkyscape set (R1420).
//! STUB (#1857): no behaviour yet; S20 (#1879) writes its script and its tests
//! (CLAUDE.md rule 6). Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-anomaly-found";

pub fn script() -> CardScripts {
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}
