//! #71 Intern Stimmy (SPEC §8.3): "At the end of any turn, if your library has more cards than the
//! opponent's: Recruit a Unit costing 1 or less", radiant "2 or less". A Radiant cell that changes
//! only a number changes only that number (§8 Conventions), so only the cost ceiling moves.
//!
//! The engine's, not the card's: WHEN it fires (R62's end-of-turn trap window on BOTH turns, so the
//! trigger watches the plain `turnEnded`, which `traps.rs` withholds from immediate dispatch),
//! "NEVER consumed" (the Field Trap type, §5, §5.1, §3.2; face-up once fired, R33) and WHOSE
//! library and Recruit ("your" is the TRAP'S CONTROLLER, R62, R52; `ctx.controller` is that player).
//! The condition is a `when` predicate, which is what `traps.rs` reads (the only module that fires this
//! card). R61: `run` returning `[]` would flip this Field Trap face-up (R33) on every turn end it does
//! not answer; a predicate leaves it armed. R195, the yellow glow: in hand and backrow exactly when
//! `library_is_larger` holds, the same function the trap's `when` and `run` read (public counts, §9.1).

use jackioh_engine::effects::recruit;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-071";

/// §10.9: a hook may READ state to compute an effect's arguments; it never writes. The two library
/// sizes come through the read-only `zone_count` (BUILD M3-T1), "yours" being the trap's controller
/// (R62, R52); "more cards than" is strictly greater, so an equal count does nothing.
fn library_is_larger(state: &GameState, controller: PlayerId) -> bool {
    let mine = zone_count(state, controller, OffFieldZone::Library);
    let theirs = zone_count(state, opponent_of(controller), OffFieldZone::Library);
    mine > theirs
}

/// The cost limit is the whole of the radiant text: the declared number `costLimit` (R386), read off
/// the face that is up.
fn intern_stimmy() -> Script {
    let at_end_of_any_turn = TriggerDef::new("intern-stimmy-window", &[GameEventType::TurnEnded], |ctx, _event| {
        if library_is_larger(ctx.state, ctx.controller) {
            // R65: outside play an X-cost card counts as 0 and an embiggen card as its base price, which
            // is what `recruit`'s filter reads off the library (`query_cost` in effects/summon.rs).
            let max_cost = param(&*ctx, "costLimit");
            vec![recruit(json_as(json!({ "filter": { "type": "Unit", "costRange": { "max": max_cost } } })))]
        } else {
            vec![]
        }
    })
    .with_when(|ctx, _event| library_is_larger(ctx.state, ctx.controller));

    Script {
        triggers: vec![at_end_of_any_turn],
        // R195: hand and field alike — the condition is the board's, not the play's.
        condition_met: Some(condition_hook(|ctx| library_is_larger(ctx.state, ctx.controller))),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = intern_stimmy();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #71 Intern Stimmy — SPEC §8.3, BUILD M4-T4: "Trap window at the end of any turn with library >
// opponent's → recruit ≤1 (R62); fires again next qualifying turn; radiant ≤2".
//
// Library fillers are Spells (core-005, core-035), which Recruit never takes (§6.3); the Units are
// core-001 (cost 2), core-003 and core-004 (cost 1), none with a turn trigger, and Recruit fires no
// Cry (R1). Both sides keep a playable card in hand, since §2.5/R82 auto-ends a turn with none.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    /// `scenario(opts)` with the shipped cards registered first.
    fn setup(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    fn trap_firings(s: &Scenario) -> usize {
        s.events()
            .iter()
            .filter(|event| serde_json::to_value(event).expect("an event serialises")["type"] == "trapFired")
            .count()
    }

    #[test]
    fn r386_an_upgrade_recruits_a_2_cost_unit_and_a_radiant_degrade_skips_it() {
        for (radiant, upgrade, recruited) in [(false, true, "core-001"), (true, false, "core-003")] {
            let mut s = setup(json!({
                "seed": "core-071-tuned",
                "p1": {
                    "backrow": [{ "def": "core-071", "radiant": radiant }],
                    "library": ["core-001", "core-003", "core-005"],
                    "hand": ["core-005"],
                },
                "p2": { "library": ["core-005"], "hand": ["core-005"] },
            }));
            let moved = if upgrade {
                crate::upgrade_number(&mut s, "core-071", "costLimit")
            } else {
                crate::degrade_number(&mut s, "core-071", "costLimit")
            };
            assert_eq!(moved, if upgrade { 2 } else { 1 });
            s.end_turn();
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some(recruited.to_string()));
        }
    }

    mod intern_stimmy_base {
        use super::*;

        #[test]
        fn r62_fires_in_the_end_of_turn_trap_window_of_its_controllers_own_turn_and_recruits_a_unit_costing_1_or_less() {
            let mut s = setup(json!({
                "seed": "core-071-own-turn",
                "p1": {
                    "backrow": ["core-071"],
                    "library": ["core-001", "core-003", "core-005"],
                    "hand": ["core-005"],
                },
                "p2": { "library": ["core-005"], "hand": ["core-005"] },
            }));
            let trap = s.backrow(P1, 1);
            let library = s.pile(P1, "library");
            let big_dfender = library[0].clone();
            let righthouse = library[1].clone();
            assert_eq!(trap.map(|card| card.def_id), Some("core-071".to_string()));

            s.end_turn();

            // The condition held (3 cards to 1), so the trap fired and recruited.
            s.expect_events(json!(["turnEnded"]));
            s.expect_events(json!(["trapFired", "summoned"]));
            assert_eq!(trap_firings(&s), 1);
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some("core-003".to_string()));
            s.expect_in_zone(&righthouse, "field");

            // "Costing 1 or less": the 2-cost Unit sitting above it in the library is skipped, not taken.
            s.expect_in_zone(&big_dfender, "library");
        }

        #[test]
        fn r62_fires_on_the_opponents_turn_end_and_r52s_reading_of_your_makes_it_the_traps_controller_who_recruits() {
            let mut s = setup(json!({
                "seed": "core-071-enemy-turn",
                "active": "p2",
                "p1": {
                    "backrow": ["core-071"],
                    "library": ["core-001", "core-003", "core-005"],
                    "hand": ["core-005"],
                },
                "p2": { "library": ["core-005", "core-023"], "hand": ["core-005"] },
            }));

            // p2 ends the turn; p1's library (3) is still larger than p2's (2).
            s.end_turn();

            assert_eq!(trap_firings(&s), 1);
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some("core-003".to_string()));
            // The recruited Unit belongs to the trap's controller, not to the player who ended the turn.
            assert!(s.unit(P2, 1).is_none());
        }

        #[test]
        fn fires_again_on_the_next_qualifying_turn_end_because_a_field_trap_is_never_consumed_s5_1_r33() {
            let mut s = setup(json!({
                "seed": "core-071-again",
                "p1": {
                    "backrow": ["core-071"],
                    "library": ["core-001", "core-003", "core-004", "core-005", "core-035"],
                    "hand": ["core-005"],
                },
                "p2": { "library": ["core-005"], "hand": ["core-005"] },
            }));
            let trap = s.backrow(P1, 1).expect("setup: Intern Stimmy is in p1's backrow lane 1");

            // p1's turn ends: 5 cards to 1.
            s.end_turn();
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some("core-003".to_string()));

            // p2's turn ends: p1 still leads (4 cards to 0 after p2's own draw), so it fires a second time.
            s.end_turn();
            assert_eq!(s.unit(P1, 2).map(|card| card.def_id), Some("core-004".to_string()));

            assert_eq!(trap_firings(&s), 2);
            // Never consumed: still in the backrow, and R33 makes a fired Field Trap public.
            s.expect_in_zone(&trap, "field");
            assert_eq!(s.backrow(P1, 1).map(|card| card.id), Some(trap.id.clone()));
            assert_eq!(s.backrow(P1, 1).and_then(|card| card.face_up), Some(true));
        }

        #[test]
        fn does_nothing_when_the_library_is_not_strictly_larger_and_the_trap_stays_armed_and_face_down() {
            let mut s = setup(json!({
                "seed": "core-071-equal",
                "p1": { "backrow": ["core-071"], "library": ["core-001", "core-003"], "hand": ["core-005"] },
                "p2": { "library": ["core-005", "core-023"], "hand": ["core-005"] },
            }));
            let trap = s.backrow(P1, 1).expect("setup: Intern Stimmy is in p1's backrow lane 1");
            let righthouse = s.pile(P1, "library")[1].clone();

            // Two cards each: "more cards than" is not met.
            s.end_turn();

            assert_eq!(trap_firings(&s), 0);
            assert!(s.unit(P1, 1).is_none());
            s.expect_in_zone(&righthouse, "library");
            // A trap that did not answer the event is still armed, and still hidden (R33's contrapositive).
            s.expect_in_zone(&trap, "field");
            assert_ne!(s.backrow(P1, 1).and_then(|card| card.face_up), Some(true));
        }

        #[test]
        fn does_nothing_when_the_library_is_smaller() {
            let mut s = setup(json!({
                "seed": "core-071-behind",
                "p1": { "backrow": ["core-071"], "library": ["core-003"], "hand": ["core-005"] },
                "p2": { "library": ["core-005", "core-023", "core-035"], "hand": ["core-005"] },
            }));

            s.end_turn();

            assert_eq!(trap_firings(&s), 0);
            assert!(s.unit(P1, 1).is_none());
        }
    }

    mod intern_stimmy_radiant {
        use super::*;

        #[test]
        fn radiant_raises_the_ceiling_to_2_or_less_so_the_2_cost_unit_above_it_in_the_library_is_taken() {
            let mut s = setup(json!({
                "seed": "core-071-radiant",
                "p1": {
                    "backrow": [{ "def": "core-071", "radiant": true }],
                    "library": ["core-001", "core-003", "core-005"],
                    "hand": ["core-005"],
                },
                "p2": { "library": ["core-005"], "hand": ["core-005"] },
            }));
            let righthouse = s.pile(P1, "library")[1].clone();

            s.end_turn();

            assert_eq!(trap_firings(&s), 1);
            // Base takes core-003 out of this very library (see the first test); radiant reaches core-001.
            assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some("core-001".to_string()));
            s.expect_in_zone(&righthouse, "library");
        }

        #[test]
        fn radiant_keeps_the_condition_the_trap_window_and_the_never_consumed_of_the_base_face() {
            let mut s = setup(json!({
                "seed": "core-071-radiant-equal",
                "active": "p2",
                "p1": {
                    "backrow": [{ "def": "core-071", "radiant": true }],
                    "library": ["core-001", "core-003"],
                    "hand": ["core-005"],
                },
                "p2": { "library": ["core-005", "core-023"], "hand": ["core-005"] },
            }));
            let trap = s.backrow(P1, 1).expect("setup: Intern Stimmy is in p1's backrow lane 1");

            // Equal libraries on the opponent's turn end: the radiant face is no more eager than the base.
            s.end_turn();
            assert_eq!(trap_firings(&s), 0);
            assert!(s.unit(P1, 1).is_none());
            s.expect_in_zone(&trap, "field");
        }
    }
}
