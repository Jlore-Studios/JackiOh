//! R1420's preview seam: under the `testkit` feature, a thread may treat sets that do not ship yet as
//! if they did, so a test, the fuzz tool or a development run can deal and generate the cards of a
//! set while it is being built. It is a thread-local the engine's catalog consults only under this
//! feature, in the spirit of `seams.rs` and SURFACE §8's registry override: each `#[test]` is its own
//! thread, so a preview a test sets never reaches another test, and a production build has no such
//! state at all (CLAUDE.md rule 4).
//!
//! What a preview opens is exactly what shipping opens: pools that name no set (R380) and the
//! deckability check (`catalog::deckable`). It changes no card, no state and no replay: a game a
//! previewing thread played replays the same on a thread that previews the same sets.

use std::cell::Cell;

use crate::wire::{CATALOG_SETS, SetName};

thread_local! {
    static PREVIEWED: Cell<Vec<SetName>> = const { Cell::new(Vec::new()) };
}

/// Whether this thread previews `set` (R1420).
pub fn previewed(set: SetName) -> bool {
    previewed_sets().contains(&set)
}

/// The sets this thread previews, in catalog order.
pub fn previewed_sets() -> Vec<SetName> {
    PREVIEWED.with(|cell| {
        let sets = cell.take();
        cell.set(sets.clone());
        sets
    })
}

/// Previews `sets` on this thread until the returned guard drops, when the sets previewed before
/// come back. Guards nest.
#[must_use = "the preview ends when the guard drops"]
pub fn preview_sets(sets: &[SetName]) -> PreviewGuard {
    let before = previewed_sets();
    let mut now = before.clone();
    for set in sets {
        if !now.contains(set) {
            now.push(*set);
        }
    }
    now.sort();
    PREVIEWED.with(|cell| cell.set(now));
    PreviewGuard { before }
}

/// Previews every set the catalog orders (`CATALOG_SETS`) on this thread until the guard drops: the
/// fuzz tool's default, so random games reach every card in the catalog (R1420).
#[must_use = "the preview ends when the guard drops"]
pub fn preview_every_set() -> PreviewGuard {
    preview_sets(&CATALOG_SETS)
}

/// Restores the sets previewed before `preview_sets` when it drops.
pub struct PreviewGuard {
    before: Vec<SetName>,
}

impl Drop for PreviewGuard {
    fn drop(&mut self) {
        let before = std::mem::take(&mut self.before);
        PREVIEWED.with(|cell| cell.set(before));
    }
}
