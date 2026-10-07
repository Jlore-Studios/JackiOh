//! Test seams for two TS test files that replaced engine code from outside (part 32's decision,
//! `.fullsend/notes/reconcile-decisions.md` `# engine green`). Compiled under the `testkit` feature
//! alone, beside SURFACE §8's registry override and in its spirit: each is a thread-local the engine
//! consults only under that feature, and each `#[test]` is its own thread, so a seam a test sets
//! never reaches another test.
//!
//! - `register_work_handler` is TS's `registerWorkHandler` (`work.ts`) for a test-made engine
//!   sequence (`packages/engine/test/pauses.test.ts`): `work::run_work_item` and `work::can_resume`
//!   consult the table when no built-in sequence owns the hook, before a card's own continuation, as
//!   TS's handler map came before its default handler.
//! - `mock_brittle_tick`, `mock_animate_at_turn_start` and `mock_return_at_cleanup` are TS's `vi.mock`
//!   of `brittle.ts`'s and `animated.ts`'s three turn stages (`packages/engine/test/turn-wiring.test.ts`):
//!   `turn.rs` runs the double in place of `brittle::brittle_tick`, `animated::animate_at_turn_start`
//!   and `animated::return_at_cleanup` while one is set, and the real stage otherwise.

use std::cell::Cell;
use std::rc::Rc;

use crate::script::EngineSink;
use crate::state::WorkItem;
use crate::wire::PlayerId;

/// TS `WorkHandler`: runs one owed item of the sequence it is registered for.
pub type WorkHandler = fn(&mut EngineSink<'_>, &WorkItem);

/// A stand-in for one turn stage: handed the turn's sink and the player whose turn it is.
pub type StageDouble = dyn Fn(&mut EngineSink<'_>, PlayerId);

thread_local! {
    static WORK_HANDLERS: Cell<Vec<(String, WorkHandler)>> = const { Cell::new(Vec::new()) };
    static BRITTLE_TICK: Cell<Option<Rc<StageDouble>>> = const { Cell::new(None) };
    static ANIMATE_AT_TURN_START: Cell<Option<Rc<StageDouble>>> = const { Cell::new(None) };
    static RETURN_AT_CLEANUP: Cell<Option<Rc<StageDouble>>> = const { Cell::new(None) };
}

/// TS `registerWorkHandler(hook, handler)` for this thread: `handler` runs every owed item whose
/// `resume.hook` is `hook` and that no engine sequence owns. Returns the handler it replaced.
pub fn register_work_handler(hook: &str, handler: WorkHandler) -> Option<WorkHandler> {
    WORK_HANDLERS.with(|cell| {
        let mut table = cell.take();
        let previous = match table.iter_mut().find(|(name, _)| name == hook) {
            Some(entry) => Some(std::mem::replace(&mut entry.1, handler)),
            None => {
                table.push((hook.to_string(), handler));
                None
            }
        };
        cell.set(table);
        previous
    })
}

/// TS `registerWorkHandler(hook, undefined)`: this thread's handler for `hook` goes. Returns it.
pub fn unregister_work_handler(hook: &str) -> Option<WorkHandler> {
    WORK_HANDLERS.with(|cell| {
        let mut table = cell.take();
        let previous = table
            .iter()
            .position(|(name, _)| name == hook)
            .map(|at| table.remove(at).1);
        cell.set(table);
        previous
    })
}

/// The handler this thread registered for `hook`, if any (what `work.rs` consults).
pub fn work_handler(hook: &str) -> Option<WorkHandler> {
    WORK_HANDLERS.with(|cell| {
        let table = cell.take();
        let found = table
            .iter()
            .find(|(name, _)| name == hook)
            .map(|(_, handler)| *handler);
        cell.set(table);
        found
    })
}

fn set_double(slot: &'static std::thread::LocalKey<Cell<Option<Rc<StageDouble>>>>, double: Rc<StageDouble>) {
    slot.with(|cell| cell.set(Some(double)));
}

fn double_of(slot: &'static std::thread::LocalKey<Cell<Option<Rc<StageDouble>>>>) -> Option<Rc<StageDouble>> {
    slot.with(|cell| {
        let double = cell.take();
        cell.set(double.clone());
        double
    })
}

/// `vi.mock` of `brittle::brittle_tick` for this thread: the start-of-turn Brittle stage runs `double`.
pub fn mock_brittle_tick(double: impl Fn(&mut EngineSink<'_>, PlayerId) + 'static) {
    set_double(&BRITTLE_TICK, Rc::new(double));
}

/// `vi.mock` of `animated::animate_at_turn_start` for this thread.
pub fn mock_animate_at_turn_start(double: impl Fn(&mut EngineSink<'_>, PlayerId) + 'static) {
    set_double(&ANIMATE_AT_TURN_START, Rc::new(double));
}

/// `vi.mock` of `animated::return_at_cleanup` for this thread.
pub fn mock_return_at_cleanup(double: impl Fn(&mut EngineSink<'_>, PlayerId) + 'static) {
    set_double(&RETURN_AT_CLEANUP, Rc::new(double));
}

/// The Brittle stage's double this thread set, if any (what `turn.rs` consults).
pub fn brittle_tick_double() -> Option<Rc<StageDouble>> {
    double_of(&BRITTLE_TICK)
}

/// The start-of-turn Animated stage's double this thread set, if any.
pub fn animate_at_turn_start_double() -> Option<Rc<StageDouble>> {
    double_of(&ANIMATE_AT_TURN_START)
}

/// The cleanup return's double this thread set, if any.
pub fn return_at_cleanup_double() -> Option<Rc<StageDouble>> {
    double_of(&RETURN_AT_CLEANUP)
}

/// Every seam back to the engine's own code for this thread: no test work handler, no stage double.
pub fn clear_seams() {
    WORK_HANDLERS.with(|cell| cell.set(Vec::new()));
    BRITTLE_TICK.with(|cell| cell.set(None));
    ANIMATE_AT_TURN_START.with(|cell| cell.set(None));
    RETURN_AT_CLEANUP.with(|cell| cell.set(None));
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::rng::Rng;
    use crate::state::{GameState, Resume};
    use crate::testkit::scenario::{Scenario, catalog_override, register_catalog, scenario};
    use crate::wire::{CardDefs, GameEvent};
    use crate::work::{can_resume, drain_work, push_work};

    const HOOK: &str = "__seam";

    /// The real catalog through the thread-local override (this crate cannot link `jackioh_cards`),
    /// a unit on each side, so `reduce` never auto-ends a turn under the test (§2.5), and a library
    /// on each, so a turn's draw takes no fatigue hit (§2.4).
    fn board() -> Scenario {
        if catalog_override().is_none() {
            let defs: CardDefs = serde_json::from_str(include_str!("../../../cards/catalog.json"))
                .expect("crates/cards/catalog.json parses as CardDefs");
            register_catalog(defs);
        }
        scenario(json!({
            "p1": { "field": [{ "def": "core-056", "lane": 5 }], "library": ["core-056", "core-056"] },
            "p2": { "field": [{ "def": "core-056", "lane": 5 }], "library": ["core-056", "core-056"] },
        }))
    }

    fn seam_resume() -> Resume {
        Resume {
            def_id: String::new(),
            hook: HOOK.to_string(),
            step: "seam".to_string(),
            radiant: false,
            instance_id: None,
            data: Default::default(),
        }
    }

    /// A test sequence's step: one point off its owner's hero, so the run shows on the board.
    fn hit_owner(sink: &mut EngineSink<'_>, item: &WorkItem) {
        sink.state.players[item.owner].hero.health -= 1;
    }

    /// Owe one item under `HOOK` to p2, then drain the queue.
    fn owe_and_drain(state: &mut GameState) {
        let seed = state.seed.clone();
        let mut events: Vec<GameEvent> = Vec::new();
        let mut rng = Rng::new(&seed, 0);
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        push_work(&mut sink, seam_resume(), Some(PlayerId::P2));
        drain_work(&mut sink);
    }

    #[test]
    fn a_registered_handler_is_found_replaced_and_removed() {
        assert!(work_handler(HOOK).is_none());
        assert!(register_work_handler(HOOK, hit_owner).is_none());
        assert!(work_handler(HOOK).is_some());
        assert!(register_work_handler(HOOK, hit_owner).is_some());
        assert!(unregister_work_handler(HOOK).is_some());
        assert!(work_handler(HOOK).is_none());
    }

    #[test]
    fn owed_work_under_a_registered_hook_is_resumable_and_runs_its_handler() {
        let mut state = board().state().clone();
        assert!(!can_resume(&state, &seam_resume()));
        register_work_handler(HOOK, hit_owner);
        assert!(can_resume(&state, &seam_resume()));
        let before = state.players[PlayerId::P2].hero.health;
        owe_and_drain(&mut state);
        assert!(state.work.is_empty());
        assert_eq!(state.players[PlayerId::P2].hero.health, before - 1);
    }

    #[test]
    #[should_panic(expected = "no handler for owed work")]
    fn owed_work_under_an_unregistered_hook_still_raises() {
        let mut state = board().state().clone();
        owe_and_drain(&mut state);
    }

    #[test]
    fn the_turn_runs_each_stage_double_in_place_of_its_stage_and_clear_seams_restores_them() {
        let mut s = board();
        mock_brittle_tick(|sink: &mut EngineSink<'_>, player: PlayerId| {
            sink.state.players[player].hero.health -= 1;
        });
        mock_animate_at_turn_start(|sink: &mut EngineSink<'_>, player: PlayerId| {
            sink.state.players[player].hero.health -= 2;
        });
        mock_return_at_cleanup(|sink: &mut EngineSink<'_>, player: PlayerId| {
            sink.state.players[player].hero.health -= 4;
        });
        let p1 = s.state().players[PlayerId::P1].hero.health;
        let p2 = s.state().players[PlayerId::P2].hero.health;
        // p1's cleanup runs the return double; p2's start runs the Brittle and the Animated doubles.
        s.end_turn();
        assert_eq!(s.state().players[PlayerId::P1].hero.health, p1 - 4);
        assert_eq!(s.state().players[PlayerId::P2].hero.health, p2 - 3);

        clear_seams();
        assert!(brittle_tick_double().is_none());
        assert!(animate_at_turn_start_double().is_none());
        assert!(return_at_cleanup_double().is_none());
        s.end_turn();
        assert_eq!(s.state().players[PlayerId::P1].hero.health, p1 - 4);
        assert_eq!(s.state().players[PlayerId::P2].hero.health, p2 - 3);
    }
}
