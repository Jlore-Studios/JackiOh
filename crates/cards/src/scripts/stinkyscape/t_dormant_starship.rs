//! SPEC §8.10 Stinkyscape T-dormant-starship Dormant Starship, a card of the held Stinkyscape set (R1420).
//! Its hull flags only so far; S02 (#1859) writes its script and its tests
//! (CLAUDE.md rule 6). Its catalog row, sound, flavour and Chinese entries are already written.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-dormant-starship";

pub fn script() -> CardScripts {
    // R1852000: the hull the engine summons; Dormant until its launch (R1839000), its one ability the
    // engine's launch (`starship::launch_decl`). S02 (#1859) adds its tests.
    let base = Script {
        static_flags: Some(StaticFlags {
            slumber: Some(SlumberText::default()),
            starship_hull: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    CardScripts {
        radiant: base.clone(),
        base,
    }
}
