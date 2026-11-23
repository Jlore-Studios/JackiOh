//! R3925000, §10.3: the loop cap, one rule that ends every endless resolution loop.
//!
//! An *interaction* is what starts a resolution loop: an action a seat takes (`reduce`), each answer a
//! `timeout` gives and the End turn it takes (R79), each autopilot (R1183002) or AI (R44) action, a turn
//! the engine ends for a seat (R82, R456), and the setup and a Glitch's reset (R676), which are no seat's.
//! Each one starts a count of the events emitted since (`begin_interaction`), every `GameEvent` counted.
//! When the count reaches `RESOLUTION_EVENT_CAP`, or one loop goes round that many times (the `passes` of
//! `halted`, for a loop that could spin without an event), the cap trips (`trip`):
//!  - the trigger queue and every event not yet offered are dropped;
//!  - a public `loopCapped` names the interaction's seat;
//!  - a last state check collects the dead with no Death, Reborn, Starship fusion or "would die"
//!    replacement, and its hero check still ends a game that has ended (`state_check::capped_check`).
//!
//! Card text is then off until the next interaction: no event is offered, no trap fires, no queued trigger
//! runs, and an effect list stops at its next effect, as at a game's end (R216). Owed work still drains,
//! so the engine's own steps under way (a play's last steps, a combat, a turn's end) finish with their
//! cards' text off. A prompt that opens before the cap is a pause, not a trip, and its answer is the next
//! interaction; so is the answer to one an engine step opens after it.
//!
//! The count lives on the sink's `Frontier`, never in state, so a game that never reaches the cap hashes
//! as before. The per-card terminators (R28, R58, R452, R2067000, R2197004, …) stay: this is their
//! backstop, and `STATE_CHECK_PASS_CAP` and `MAX_WORK_STEPS` are assertions it trips before.

use crate::config::RESOLUTION_EVENT_CAP;
use crate::script::EngineSink;
use crate::wire::{ActionType, GameEvent, PlayerId};

/// A new interaction begins (`by` is its seat and kind, `None` for the setup and a Glitch's reset): the
/// one before it closes, and the count starts over. One that ended capped offers none of the events it
/// left, so its text stays off for them.
pub fn begin_interaction(sink: &mut EngineSink<'_>, by: Option<(PlayerId, ActionType)>) {
    note_peak(sink);
    if capped(sink) {
        offer_nothing(sink);
    }
    let from = sink.events.len();
    let frontier = sink.frontier.get_mut();
    frontier.capped = false;
    frontier.interaction_from = from;
    frontier.interaction = by;
}

/// The events a nested `reduce` emitted and the sink has just copied in (an AI turn's, `ai_policy.rs`)
/// were that one's count, not this interaction's.
pub fn skip_nested(sink: &mut EngineSink<'_>) {
    let from = sink.events.len();
    sink.frontier.get_mut().interaction_from = from;
}

/// The last interaction of a `reduce` closes.
pub fn end_interaction(sink: &EngineSink<'_>) {
    note_peak(sink);
}

/// The events the interaction now resolving has emitted.
pub fn events_since(sink: &EngineSink<'_>) -> usize {
    sink.events
        .len()
        .saturating_sub(sink.frontier.get().interaction_from)
}

/// Whether the interaction now resolving has reached the cap, its card text off.
pub fn capped(sink: &EngineSink<'_>) -> bool {
    sink.frontier.get().capped
}

/// Trip the cap if the interaction has reached it, by its events or by `passes`, the turns of the loop
/// asking. A prompt open or a game over is a pause or an end, never a trip.
pub fn trip_if_due(sink: &mut EngineSink<'_>, passes: usize) {
    if capped(sink) || crate::work::paused(sink) {
        return;
    }
    if events_since(sink) < RESOLUTION_EVENT_CAP && passes < RESOLUTION_EVENT_CAP {
        return;
    }
    trip(sink);
}

/// A loop's top: trip the cap if due, and whether the loop must stop because the cap is reached.
pub fn halted(sink: &mut EngineSink<'_>, passes: usize) -> bool {
    trip_if_due(sink, passes);
    capped(sink)
}

/// The trip: the rest is dropped, `loopCapped` says so, and the capped check judges the board, so a
/// `gameOver` it brings is still the interaction's last event (R216).
fn trip(sink: &mut EngineSink<'_>) {
    sink.frontier.get_mut().capped = true;
    let player = sink
        .frontier
        .get()
        .interaction
        .map_or(sink.state.active, |(seat, _)| seat);
    sink.events.push(GameEvent::LoopCapped { player });
    offer_nothing(sink);
    crate::state_check::state_check(sink);
    offer_nothing(sink);
}

/// Capped: every event emitted so far is passed over unoffered, and nothing waits to be offered or to
/// run: the frontier's list, the trigger queue, and the gone killers kept for a `destroyed` (R2396000).
pub fn offer_nothing(sink: &mut EngineSink<'_>) {
    let len = sink.events.len();
    sink.frontier.get_mut().collected = len;
    sink.state.dispatch.clear();
    sink.state.trigger_queue.clear();
    sink.state.kill_snapshots = None;
}

/// `triggers::settle` once capped: what is owed drains with its cards' text off (a resolving Spell, a
/// combat, a turn's end), and the capped check judges the board it leaves.
pub fn settle_capped(sink: &mut EngineSink<'_>) {
    offer_nothing(sink);
    if crate::work::paused(sink) {
        return;
    }
    crate::work::drain_work(sink);
    offer_nothing(sink);
    if sink.state.result.is_some() {
        return;
    }
    crate::state_check::state_check(sink);
    offer_nothing(sink);
}

/// The measurement seam behind the cap's value (`testkit::loop_peak`, `JACKIOH_LOOP_PEAK`): the
/// interaction closing, its count and kind.
#[cfg(feature = "testkit")]
fn note_peak(sink: &EngineSink<'_>) {
    let frontier = sink.frontier.get();
    crate::testkit::loop_peak::note(
        events_since(sink),
        sink.state.turn,
        frontier.interaction.map(|(_, kind)| kind),
        frontier.capped,
    );
}

#[cfg(not(feature = "testkit"))]
fn note_peak(_sink: &EngineSink<'_>) {}
