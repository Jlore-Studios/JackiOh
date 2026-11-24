//! Audit: the engine's special-purpose state machines (Fuse, Activate, Starships, Dormant, Evolve,
//! Quests, the hero power, ...). Each test asserts the ruled or printed behaviour and fails on the bug
//! it proves.

#![allow(unused_imports, dead_code)]

use jackioh_engine::testkit::*;

#[test]
fn smoke() {
    jackioh_cards::register_all();
}
