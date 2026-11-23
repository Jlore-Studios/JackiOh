//! R3925000: the measurement seam behind `RESOLUTION_EVENT_CAP`'s value. `loop_cap.rs` notes each
//! interaction as it closes; this keeps the most events any one emitted in the process, with its turn and
//! kind, and how many reached the cap, which stay out of the high-water mark. `cargo jackioh` prints it
//! when `JACKIOH_LOOP_PEAK` is set (crates/tools/README.md). Process-wide atomics, so a run over many
//! threads keeps one mark; the rules never read it.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::wire::ActionType;

/// The high-water mark, packed `events << 24 | turn << 8 | kind` so `fetch_max` keeps the most events.
static PEAK: AtomicU64 = AtomicU64::new(0);

/// The interactions that reached the cap.
static CAPPED: AtomicUsize = AtomicUsize::new(0);

/// The most events one interaction emitted, and where.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoopPeak {
    pub events: usize,
    pub turn: i32,
    /// The interaction's kind; `None` for the setup and a Glitch's reset.
    pub kind: Option<ActionType>,
    /// How many interactions reached the cap.
    pub capped: usize,
}

/// An interaction closes with `events` emitted, on `turn`.
pub fn note(events: usize, turn: i32, kind: Option<ActionType>, capped: bool) {
    if capped {
        CAPPED.fetch_add(1, Ordering::Relaxed);
        return;
    }
    let code = kind
        .and_then(|kind| ActionType::ALL.iter().position(|each| *each == kind))
        .map_or(0, |at| at as u64 + 1);
    let turn = u64::from(turn.clamp(0, 0xFFFF) as u16);
    let packed = ((events as u64) << 24) | (turn << 8) | code;
    PEAK.fetch_max(packed, Ordering::Relaxed);
}

/// The mark so far.
pub fn peak() -> LoopPeak {
    let packed = PEAK.load(Ordering::Relaxed);
    let code = (packed & 0xFF) as usize;
    LoopPeak {
        events: (packed >> 24) as usize,
        turn: ((packed >> 8) & 0xFFFF) as i32,
        kind: code
            .checked_sub(1)
            .and_then(|at| ActionType::ALL.get(at).copied()),
        capped: CAPPED.load(Ordering::Relaxed),
    }
}
