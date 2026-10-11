//! Pick N different entries of a list (`doRandomEntries`): N different entries (R60) drawn from
//! the match rng as the effect resolves, resolved in the list's order and named to both players
//! before they resolve (R436); N at or above the list's length does every entry with no rng draw.
//! Meditative #49.1 YileGPT Unleashed (MD-C23, R1023) is its user.

use jackioh_engine::effects::{DoRandomEntriesArgs, RandomEntry, do_random_entries};
use jackioh_engine::testkit::PlayerId::P1;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{answer, ask_controller, note, notes, playing, round_trip};
use crate::rules::fixtures::harness::sink_for;

/// TS `makeContext(sink, null, { controller })`'s options.
fn by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

/// Five entries noting `entry-0` … `entry-4`, in list order.
fn five() -> Vec<RandomEntry> {
    (0..5)
        .map(|index| RandomEntry {
            label: format!("entry {index}"),
            effects: vec![note(format!("entry-{index}"))],
        })
        .collect()
}

/// The labels the roll announced, in order.
fn announced(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::ChaosRolled { effects, .. } => Some(effects.clone()),
            _ => None,
        })
        .flatten()
        .collect()
}

/// The picked indices behind the `entry-<n>` notes, in note order.
fn picked(state: &GameState) -> Vec<usize> {
    notes(state)
        .iter()
        .filter_map(|entry| entry.strip_prefix("entry-")?.parse::<usize>().ok())
        .collect()
}

mod pick_n {
    use super::*;

    #[test]
    fn r1023_draws_n_different_entries_resolved_in_list_order() {
        let mut seen: Vec<Vec<usize>> = Vec::new();
        for n in 0..16 {
            let mut state = playing(&format!("dc-pick-n-{n}"));
            let mut sink = sink_for(&mut state);
            let before = sink.rng.cursor();
            apply_effects(
                &[do_random_entries(DoRandomEntriesArgs {
                    entries: five(),
                    count: 2,
                })],
                &mut make_context(&mut sink, None, by(P1)),
            );
            // Two draws were taken from the match rng.
            assert!(sink.rng.cursor() > before);
            let got = picked(sink.state);
            assert_eq!(got.len(), 2);
            // Different entries (R60), resolved in the list's order whatever order they drew in.
            let mut sorted = got.clone();
            sorted.sort();
            sorted.dedup();
            assert_eq!(sorted.len(), 2);
            assert_eq!(got, sorted);
            seen.push(got);
        }
        // The draw varies: more than one pair comes up over 16 seeds.
        seen.sort();
        seen.dedup();
        assert!(seen.len() > 1, "{seen:?}");
    }

    #[test]
    fn r1023_chaos_rolled_names_them_before_they_resolve() {
        let mut state = playing("dc-pick-announce");
        let mut sink = sink_for(&mut state);
        apply_effects(
            &[do_random_entries(DoRandomEntriesArgs {
                entries: five(),
                count: 2,
            })],
            &mut make_context(&mut sink, None, by(P1)),
        );
        // One announcement, naming the picked entries' printed clauses in the order they resolve.
        let rolls: Vec<Vec<String>> = sink
            .events
            .iter()
            .filter_map(|event| match event {
                GameEvent::ChaosRolled { effects, .. } => Some(effects.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(rolls.len(), 1);
        let got = picked(sink.state);
        assert_eq!(
            rolls[0],
            got.iter()
                .map(|index| format!("entry {index}"))
                .collect::<Vec<_>>()
        );
        assert_eq!(announced(sink.events).len(), 2);
    }

    #[test]
    fn r1023_n_at_or_above_the_list_does_every_entry_with_no_rng_draw() {
        for count in [5, 6, 99] {
            let mut state = playing(&format!("dc-pick-all-{count}"));
            let mut sink = sink_for(&mut state);
            let before = sink.rng.cursor();
            apply_effects(
                &[do_random_entries(DoRandomEntriesArgs {
                    entries: five(),
                    count,
                })],
                &mut make_context(&mut sink, None, by(P1)),
            );
            // Every entry, in list order — and the rng cursor never moved.
            assert_eq!(picked(sink.state), vec![0, 1, 2, 3, 4]);
            assert_eq!(sink.rng.cursor(), before, "count {count}");
            assert_eq!(announced(sink.events).len(), 5);
        }
    }

    /// A sequence a test made, resumed through the seams handler below (R113): the same roll,
    /// rebuilt, re-entered where the pause left it — the memo picks again, never the rng.
    const ROLL_HOOK: &str = "testRoll";

    fn roll_entries() -> Vec<RandomEntry> {
        (0..3)
            .map(|index| RandomEntry {
                label: format!("entry {index}"),
                effects: if index == 1 {
                    vec![note("entry-1".to_string()), ask_controller("answered")]
                } else {
                    vec![note(format!("entry-{index}"))]
                },
            })
            .collect()
    }

    fn roll_effect() -> Effect {
        do_random_entries(DoRandomEntriesArgs {
            entries: roll_entries(),
            count: 3,
        })
    }

    fn roll_plan(owner: PlayerId) -> WorkPlan {
        WorkPlan {
            def_id: String::new(),
            hook: ROLL_HOOK.to_string(),
            step: "hook".to_string(),
            radiant: false,
            instance_id: None,
            data: IndexMap::new(),
            owner,
        }
    }

    fn roll_handler(sink: &mut EngineSink<'_>, item: &WorkItem) {
        let plan = roll_plan(item.owner);
        let mut ctx = make_context(
            sink,
            None,
            HookOptions {
                controller: Some(item.owner),
                ..HookOptions::default()
            },
        );
        run_resumable_list(&mut ctx, &plan, vec![roll_effect()], paused_of(&item.resume.data));
    }

    #[test]
    fn r1023_a_pause_resumes_the_same_picks_after_a_json_round_trip() {
        register_work_handler(ROLL_HOOK, roll_handler);
        let mut state = playing("dc-pick-pause");
        let status = {
            let mut sink = sink_for(&mut state);
            let mut ctx = make_context(&mut sink, None, by(P1));
            run_resumable_list(&mut ctx, &roll_plan(P1), vec![roll_effect()], None)
        };
        // The second entry asked with the third still left: the walk parked behind the prompt.
        assert_eq!(status, ListStatus::Parked);
        assert_eq!(notes(&state), vec!["entry-0", "entry-1"]);
        assert!(state.pending.is_some());
        let resumed = answer(&round_trip(&state)).state;
        // The same picks after the JSON round trip: entry-2 still resolves, nothing re-rolled.
        assert_eq!(notes(&resumed), vec!["entry-0", "entry-1", "entry-2"]);
        assert!(resumed.work.is_empty());
        unregister_work_handler(ROLL_HOOK);
    }
}
