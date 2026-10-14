// A fixture for `luau surface`, read as text and never compiled: a face that is built in another file.
use jackioh_engine::prelude::*;

pub const ID: &str = "fixture-unreadable";

pub fn script() -> CardScripts {
    CardScripts { base: shared::face(), radiant: Script::default() }
}
