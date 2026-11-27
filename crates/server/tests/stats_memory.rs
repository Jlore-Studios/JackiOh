//! #5129: what one card total costs the kept snapshot (R3067000), weighed on a counted heap.
//!
//! From about 19:09 UTC on 2026-11-24 the snapshot held about 330,000 totals on a 1 GiB game server. Each
//! `CardTotal` held its five sparse parts inline whether they were empty or not (R4552001, R4552002, R4672003,
//! R4673001), which made 1,600 bytes a total where 312 had been. Loading the snapshot got the server killed:
//! every /stats request answered 500, and every live game on the box was dropped, because a match start reads
//! the same snapshot (#4964). R4965000 bounds how many totals the snapshot keeps. This binary bounds what each
//! one costs, the heap included, which `sparse.rs`'s `size_of` test does not weigh.
//!
//! A binary of its own, because its allocator counts every allocation it makes: in the `server` binary the
//! count slowed the other tests by about a fifth.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

use indexmap::{IndexMap, IndexSet};
use jackioh_cards::deck_play_versions;
use jackioh_engine::wire::stats::{
    CardStats, CardTotal, GAME_MODES, GAME_SOURCES, PILOTS, StatsTotals, Tally, TotalsKey,
};
use jackioh_server::db::store::add_card_total;

thread_local! {
    /// The bytes this thread has allocated and not released. It goes below zero on a thread that releases
    /// what another allocated.
    static HELD: Cell<isize> = const { Cell::new(0) };
}

/// Adds `delta` to this thread's count. The count is a `const` `Cell` with no destructor, so reading it
/// allocates nothing and cannot recurse into the allocator. `try_with` skips the count for a thread whose
/// locals are already torn down.
fn count(delta: isize) {
    let _ = HELD.try_with(|held| held.set(held.get() + delta));
}

/// The bytes this thread holds on the heap now. The difference between two readings on the same thread is
/// what was allocated between them and is still held.
fn held() -> isize {
    HELD.with(Cell::get)
}

/// The system allocator, counting each thread's held bytes.
struct Counted;

// SAFETY: every call goes straight to `System` with the same arguments; only the count is added.
unsafe impl GlobalAlloc for Counted {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: the caller's `layout` contract is passed on unchanged.
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            count(layout.size() as isize);
        }
        ptr
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: as `alloc`.
        let ptr = unsafe { System.alloc_zeroed(layout) };
        if !ptr.is_null() {
            count(layout.size() as isize);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: `ptr` came from this allocator, so from `System`, with `layout`.
        unsafe { System.dealloc(ptr, layout) };
        count(-(layout.size() as isize));
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: as `dealloc`, and the caller's `new_size` contract is passed on unchanged.
        let moved = unsafe { System.realloc(ptr, layout, new_size) };
        if !moved.is_null() {
            count(new_size as isize - layout.size() as isize);
        }
        moved
    }
}

#[global_allocator]
static COUNTED: Counted = Counted;

/// What one card total with only its tallies may hold once the snapshot keeps it: the total itself, its entry
/// in its key's map and its id twice (the key and `stats.card`). It held 449 bytes when this was set
/// (2026-11-26, 64-bit). The totals that got the server killed held more than 1,600 bytes each inline alone.
const KEPT_TOTAL_BUDGET_BYTES: usize = 600;

/// A patch name as long as a shipped one.
const PATCH: &str = "v0.6.15";

/// Every deck card's `card_totals` row under every source, mode and pilot of one patch, as Postgres stores it
/// (`stats` as JSON), each with every tally a total always counts. 0.3% to 6% of staging's rows held a sparse
/// part (2026-11-25); these hold none, so what they cost is what every row costs.
fn rows() -> Vec<(TotalsKey, String, Vec<u8>)> {
    let plays = deck_play_versions();
    let cards: IndexSet<&str> = plays.iter().map(|play| play.card.as_str()).collect();
    assert!(cards.len() > 200, "{} deck cards", cards.len());
    let tally = |games: i32| Tally {
        games,
        wins: games / 2,
        draws: 1,
    };
    let mut rows = Vec::new();
    for &source in GAME_SOURCES {
        for &mode in GAME_MODES {
            for &pilot in PILOTS {
                let key = TotalsKey {
                    patch: PATCH.to_string(),
                    source,
                    mode,
                    pilot,
                };
                for card in &cards {
                    let stats = CardStats {
                        in_deck: tally(40),
                        opening_hand: tally(12),
                        going_first: tally(20),
                        going_second: tally(20),
                        played: tally(30),
                        drawn_not_played: tally(6),
                        played_radiant: tally(3),
                        played_normal: tally(27),
                        seen_radiant: tally(4),
                        seen_normal: tally(32),
                        ..CardTotal::new(card).stats
                    };
                    let stored = serde_json::to_vec(&stats).expect("a total's stats are stored as JSON");
                    rows.push((key.clone(), card.to_string(), stored));
                }
            }
        }
    }
    rows
}

#[test]
fn a_kept_card_total_costs_the_snapshot_at_most_its_budget_heap_included() {
    let rows = rows();

    // Kept as `stats_totals_load` (`db/pg.rs`) keeps each row it reads: its stats decoded straight into a
    // total, the total added under its key, and each key's map shrunk to fit once every row is in.
    let before = held();
    let mut totals = StatsTotals::default();
    for (key, card, stored) in &rows {
        let total = CardTotal {
            stats: serde_json::from_slice::<CardStats>(stored).expect("a stored total reads back"),
            ..CardTotal::new(card)
        };
        add_card_total(totals.cards.entry(key.clone()).or_default(), card.clone(), total);
    }
    for cards in totals.cards.values_mut() {
        cards.shrink_to_fit();
    }
    let kept = usize::try_from(held() - before).expect("the totals hold what they allocated");

    assert_eq!(
        totals.cards.values().map(IndexMap::len).sum::<usize>(),
        rows.len(),
        "every row is kept as a total of its own"
    );
    let per_total = kept / rows.len();
    assert!(
        per_total <= KEPT_TOTAL_BUDGET_BYTES,
        "a kept card total costs {per_total} bytes, over its budget of {KEPT_TOTAL_BUDGET_BYTES}: the snapshot \
         holds hundreds of thousands of them on a 1 GiB server (#4964, #5129). Hold a part most totals leave \
         empty in a `Sparse` (`jackioh_engine::wire::Sparse`), or raise the budget only with a measurement \
         that the server still loads the snapshot"
    );
}
