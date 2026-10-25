//! R1829000 (issue #1829): a card a Stack pile uncovers, as the cards above it leave, is summoning sick
//! as a card that entered the field on that turn is (§3.2, §4.1, R83): Rush lets it attack a unit,
//! nothing lets it attack the hero, it keeps the exertion it spent before it was covered, and it
//! attacks freely on its controller's next turn.

mod r1829000_a_card_a_stack_pile_uncovers_is_summoning_sick {
    use jackioh_engine::effects::move_::bounce_card;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::rules::fixtures::combat::{plain, rusher, stacker};
    use crate::rules::fixtures::harness::{setup_catalog, sink_for};

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
        let under = beneath_at(s.state(), ZoneSlot { player: P1, row: Row::Units, lane: 1 })[0].clone();
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
        let under = beneath_at(s.state(), ZoneSlot { player: P1, row: Row::Units, lane: 1 })[0].clone();
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
}
