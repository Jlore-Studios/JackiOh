//! R5268000: each shipped snapshot as the deltas under `patches/deltas/` fold to it, for the tests
//! that hold the build's history to the files: `history.rs`'s, and `tests/cross/card_text.rs`'s
//! through `#[path]`. `build.rs` compiles the deltas in as `$OUT_DIR/deltas.rs`; only tests
//! include this module, so no other build carries them.

use indexmap::IndexMap;
use serde::Deserialize;
use serde_json::Value;

include!(concat!(env!("OUT_DIR"), "/deltas.rs"));

/// One card's entry, its fields in the file's order.
pub type Entry = IndexMap<String, Value>;

/// A snapshot: the entries by id, in `order.json`'s order.
pub type Snapshot = IndexMap<String, Entry>;

#[derive(Deserialize)]
struct Delta {
    #[serde(default)]
    set: Snapshot,
    #[serde(default)]
    unset: Vec<String>,
}

/// The snapshot `version` shipped: the fold of its delta and every one before it. Panics on a
/// version that never shipped.
pub fn snapshot(version: &str) -> Snapshot {
    let order: Vec<String> = serde_json::from_str(ORDER).expect("patches/order.json is a list of ids");
    let mut folded = Snapshot::new();
    for (shipped, raw) in DELTAS {
        let delta: Delta = serde_json::from_str(raw)
            .unwrap_or_else(|error| panic!("patches/deltas/{shipped}.json is not a delta: {error}"));
        for id in &delta.unset {
            folded.shift_remove(id);
        }
        folded.extend(delta.set);
        if *shipped == version {
            return order
                .iter()
                .filter_map(|id| folded.get(id).map(|entry| (id.clone(), entry.clone())))
                .collect();
        }
    }
    panic!("{version} is no shipped version in patches/patches.json")
}
