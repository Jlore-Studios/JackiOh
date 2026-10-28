//! R1829000 (issue #1829): a card a Stack pile uncovers, as the cards above it leave, is summoning sick
//! as a card that entered the field on that turn is (§3.2, §4.1, R83): Rush lets it attack a unit and
//! not the hero, it keeps the exertion it spent before it was covered, and it attacks freely on its
//! controller's next turn.

mod r1829000_a_card_a_stack_pile_uncovers_is_summoning_sick {
    use jackioh_engine::effects::move_::bounce_card;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::rules::fixtures::combat::{plain, rusher, stacker};
    use jackioh_engine::subsystems::rotation::{RotationArgs, rotate_rings};

    use crate::rules::fixtures::harness::{Bench, setup_catalog, sink_for, slot};

    /// p1 holds `p1_hand` and has `p1_field`; p2 holds one plain unit and has another in lane 5. It is
    /// p1's main phase.
    fn board(seed: &str, p1_hand: Value, p1_field: Value) -> Scenario {
        setup_catalog();
        scenario(json!({
            "seed": seed,
            "p1": { "hand": p1_hand, "field": p1_field, "library": [plain.id, plain.id] },
            "p2": {
                "hand": [plain.id],
                "field": [{ "def": plain.id, "lane": 5 }],
                "library": [plain.id, plain.id],
            },
        }))
    }

    /// `under` in p1's lane 1 beneath a Stacker one hit from death (5/7 with 6 damage).
    fn pile(under: &str) -> Value {
        json!([{ "def": under, "lane": 1 }, { "def": stacker.id, "stack": true, "damage": 6 }])
    }

    fn top(s: &Scenario, player: PlayerId, lane: i32) -> CardInstance {
        s.unit(player, lane)
            .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
    }

    /// Bounce `card` to its owner's hand, as a card's script would.
    fn bounce(s: &mut Scenario, card: &CardInstance) {
        let mut sink = sink_for(s.state_mut());
        bounce_card(&mut sink, card);
    }

    #[test]
    fn r1829000_a_unit_uncovered_as_the_top_of_its_pile_dies_attacking_is_summoning_sick() {
        let mut s = board("r1829000-dies", json!([]), pile(&plain.id));
        let stack = top(&s, P1, 1);
        let under = beneath_at(
            s.state(),
            ZoneSlot {
                player: P1,
                row: Row::Units,
                lane: 1,
            },
        )[0]
        .clone();
        let enemy = top(&s, P2, 5);
        assert!(!is_sick(s.state(), &under));

        s.attack(&stack.id, &enemy.id);

        s.expect_in_zone(&stack.id, "graveyard");
        let resumed = top(&s, P1, 1);
        assert_eq!(resumed.id, under.id);
        assert_eq!(resumed.summoned_turn, Some(s.state().turn));
        assert!(is_sick(s.state(), &resumed));
        assert!(attack_targets(s.state(), &resumed).is_empty());
    }

    #[test]
    fn r1829000_a_unit_uncovered_on_the_opponents_turn_may_attack_on_its_controllers_next_turn() {
        let mut s = board("r1829000-their-turn", json!([]), pile(&plain.id));
        let stack = top(&s, P1, 1);
        let under = beneath_at(
            s.state(),
            ZoneSlot {
                player: P1,
                row: Row::Units,
                lane: 1,
            },
        )[0]
        .clone();
        s.end_turn();
        let enemy = top(&s, P2, 5);

        s.attack(&enemy.id, &stack.id);

        s.expect_in_zone(&stack.id, "graveyard");
        let resumed = top(&s, P1, 1);
        assert_eq!(resumed.id, under.id);
        assert_eq!(resumed.summoned_turn, Some(s.state().turn));
        s.end_turn();
        let resumed = top(&s, P1, 1);
        assert!(!is_sick(s.state(), &resumed));
        assert!(attack_targets(s.state(), &resumed).contains(&DamageTarget::Hero { player: P2 }));
    }

    #[test]
    fn r1829000_an_uncovered_rush_unit_may_attack_units_but_not_the_hero() {
        let mut s = board("r1829000-rush", json!([]), pile(&rusher.id));
        let stack = top(&s, P1, 1);

        bounce(&mut s, &stack);

        let resumed = top(&s, P1, 1);
        assert_eq!(resumed.def_id, rusher.id);
        assert!(is_sick(s.state(), &resumed));
        let targets = attack_targets(s.state(), &resumed);
        assert!(!targets.is_empty());
        assert!(
            targets
                .iter()
                .all(|target| matches!(target, DamageTarget::Unit { .. })),
            "{targets:?}"
        );
    }

    #[test]
    fn r1829000_an_uncovered_unit_keeps_the_attack_it_spent_before_it_was_covered() {
        let mut s = board(
            "r1829000-exertion",
            json!([stacker.id]),
            json!([{ "def": rusher.id, "lane": 1 }]),
        );
        let rush = top(&s, P1, 1);
        s.attack(&rush.id, "hero");
        s.play(stacker.id.as_str(), json!({ "zone": 1 }));
        let stack = top(&s, P1, 1);
        assert_eq!(stack.def_id, stacker.id);

        bounce(&mut s, &stack);

        let resumed = top(&s, P1, 1);
        assert_eq!(resumed.id, rush.id);
        assert!(is_sick(s.state(), &resumed));
        assert!(resumed.exertion.attacked);
        assert!(attack_targets(s.state(), &resumed).is_empty());
    }

    /// A Unit covered by a Stack card, its pile rotated a lane (R14) and its cover gone, all inside one
    /// action on a later turn than it entered, as seed 128 of the fuzz played it (Core #52 Silly Silas
    /// onto ZolarKeth's pile, then TG #14 Explosive Trap): the fuzz's monitor reads the uncovering in the
    /// lane the pile reached. Without the rotation's event the same state is an entry no event explains,
    /// which I4 reports.
    #[test]
    fn r1829000_the_fuzz_monitor_reads_an_uncovering_where_a_rotation_moved_the_pile() {
        // The monitor watches p1 play the Unit into lane 2, then a round of turns.
        let primed = || {
            let mut s = board("r1829000-rotated", json!([plain.id]), json!([]));
            let mut monitor = create_invariant_monitor(s.state());
            s.play(&plain.id, json!({ "zone": 2 }));
            assert_eq!(monitor.after(s.last_events(), s.state()), Vec::<String>::new());
            for _ in 0..2 {
                s.end_turn();
                monitor.after(s.last_events(), s.state());
            }
            assert_eq!(s.state().active, P1);
            (monitor, s.state().clone())
        };
        let (_, start) = primed();
        let under = card_at(&start, slot(P1, Row::Units, 2))
            .cloned()
            .expect("the Unit in lane 2");
        let mut state = start.clone();
        let mut bench = Bench::new(&state);
        let mut cover = new_instance(&mut state, &stacker.id, P1, Zone::Hand { player: P1 });
        let stacked = place_on_field(
            &mut state,
            &mut cover,
            slot(P1, Row::Units, 2),
            PlaceOnFieldOptions { stack: Some(true) },
        );
        assert!(stacked, "the Stacker tops the pile");
        bench.run(&mut state, |sink| {
            rotate_rings(
                sink,
                &RotationArgs {
                    direction: RotationDirection::Left,
                    perspective: P1,
                    radiant: None,
                },
            )
        });
        bench.run(&mut state, |sink| {
            let card = find_instance(sink.state, &cover.id).cloned().expect("the cover");
            bounce_card(sink, &card);
        });
        let resumed = find_instance(&state, &under.id).expect("uncovered");
        assert_ne!(
            slot_of(&state, resumed),
            Some(slot(P1, Row::Units, 2)),
            "the pile moved"
        );
        assert_eq!(resumed.summoned_turn, Some(state.turn));
        assert!(
            under.summoned_turn < Some(state.turn),
            "it entered on an earlier turn"
        );

        let entry_mismatches = |events: &[GameEvent]| -> Vec<String> {
            primed()
                .0
                .after(events, &state)
                .into_iter()
                .filter(|found| found.starts_with("I4 entry mismatch"))
                .collect()
        };
        assert_eq!(entry_mismatches(&bench.events), Vec::<String>::new());
        let unrotated: Vec<GameEvent> = bench
            .events
            .iter()
            .filter(|event| !matches!(event, GameEvent::Rotated { .. }))
            .cloned()
            .collect();
        assert_eq!(entry_mismatches(&unrotated).len(), 1, "{unrotated:?}");
    }
}
