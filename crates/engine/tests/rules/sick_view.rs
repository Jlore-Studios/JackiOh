// R849000 (issue #849): the view marks a summoning-sick unit (`UnitView.sick`, §4.1, §6.1) on both
// seats, and says why the viewer's own unit that may act cannot declare an attack now
// (`UnitView.attackReason`, §4.2 step 1, as `ActivationView.reason` does for an ability, R384). The
// web draws the zzz and the reason from these two fields and works neither out (CLAUDE.md rule 7).

mod r849000_the_view_marks_sleeping_units_and_says_why_a_unit_cannot_attack {
    use jackioh_engine::effects::steal::steal;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::rules::fixtures::combat::{charger, plain, rusher};
    use crate::rules::fixtures::harness::setup_catalog;

    const SLEEPING: &str = "that unit is summoning sick";
    const NOT_ATTACK_POSITION: &str = "only Attack-Position units may attack";

    /// p1 holds `p1_hand` and has `p1_field`; p2 has one plain unit in lane 5. It is p1's main phase.
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

    fn one_plain_on_lane_5() -> Value {
        json!([{ "def": plain.id, "lane": 5 }])
    }

    /// What `viewer` reads on the unit: its `sick` flag and its `attackReason`.
    fn marks_of(s: &Scenario, viewer: PlayerId, id: &str) -> (Option<bool>, Option<String>) {
        let view = s.view(viewer);
        let unit = view
            .you
            .units
            .iter()
            .chain(view.opponent.units.iter())
            .flatten()
            .find(|unit| unit.instance_id == id)
            .unwrap_or_else(|| panic!("{viewer:?} sees no unit {id}"));
        (unit.sick, unit.attack_reason.clone())
    }

    fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
        s.unit(player, lane)
            .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
            .id
    }

    #[test]
    fn r849000_a_unit_played_this_turn_is_sick_to_both_seats_and_says_so_to_its_controller() {
        let mut s = board("r849000-played", json!([plain.id]), one_plain_on_lane_5());
        s.play(plain.id.as_str(), json!({ "zone": 1 }));
        let played = id_at(&s, P1, 1);

        assert_eq!(
            marks_of(&s, P1, &played),
            (Some(true), Some(SLEEPING.to_string()))
        );
        assert_eq!(marks_of(&s, P2, &played), (Some(true), None));
    }

    #[test]
    fn r849000_a_charge_unit_and_a_rush_unit_played_this_turn_are_not_sick() {
        let mut s = board(
            "r849000-keywords",
            json!([charger.id, rusher.id]),
            one_plain_on_lane_5(),
        );
        s.play(charger.id.as_str(), json!({ "zone": 1 }));
        s.play(rusher.id.as_str(), json!({ "zone": 2 }));

        for lane in [1, 2] {
            let id = id_at(&s, P1, lane);
            assert_eq!(marks_of(&s, P1, &id), (None, None), "p1 reads lane {lane}");
            assert_eq!(marks_of(&s, P2, &id), (None, None), "p2 reads lane {lane}");
        }
    }

    #[test]
    fn r849000_a_unit_from_an_earlier_turn_is_not_sick_and_one_that_can_attack_carries_no_reason() {
        let s = board("r849000-earlier", json!([plain.id]), one_plain_on_lane_5());
        let veteran = id_at(&s, P1, 5);

        assert_eq!(marks_of(&s, P1, &veteran), (None, None));
        assert_eq!(marks_of(&s, P2, &veteran), (None, None));
    }

    #[test]
    fn r849000_r171_a_unit_stolen_this_turn_is_sick() {
        let mut s = board("r849000-stolen", json!([plain.id]), one_plain_on_lane_5());
        let victim = id_at(&s, P2, 5);

        // The effect as a card's script would run it (`rules/view_for.rs`'s `run`), p1 the taker.
        let state = s.state_mut();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut events: Vec<GameEvent> = Vec::new();
        {
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            let options = HookOptions {
                controller: Some(P1),
                ..Default::default()
            };
            let mut ctx = make_context(&mut sink, None, options);
            (steal(json_as(json!({ "instanceId": victim }))).apply)(&mut ctx);
        }
        state.rng_cursor = rng.cursor();

        assert_eq!(s.card(victim.as_str()).controller, P1);
        assert_eq!(
            marks_of(&s, P1, &victim),
            (Some(true), Some(SLEEPING.to_string()))
        );
        assert_eq!(marks_of(&s, P2, &victim), (Some(true), None));
    }

    #[test]
    fn r849000_a_defense_position_unit_says_only_attack_position_units_may_attack() {
        let s = board(
            "r849000-defense",
            json!([plain.id]),
            json!([{ "def": plain.id, "lane": 5, "position": "DEF" }]),
        );
        let guard = id_at(&s, P1, 5);

        assert_eq!(
            marks_of(&s, P1, &guard),
            (None, Some(NOT_ATTACK_POSITION.to_string()))
        );
        assert_eq!(marks_of(&s, P2, &guard), (None, None));
    }

    #[test]
    fn r849000_the_opponents_units_never_carry_a_reason() {
        let mut s = board("r849000-opponent", json!([plain.id]), one_plain_on_lane_5());
        s.play(plain.id.as_str(), json!({ "zone": 1 }));
        let mine = id_at(&s, P1, 1);
        let theirs = id_at(&s, P2, 5);
        s.card_mut(theirs.as_str()).position = Some(Position::Def);

        // p1's turn: p2's unit would refuse (Defense Position), and p1 is told nothing of it.
        assert_eq!(marks_of(&s, P1, &theirs), (None, None));
        assert_eq!(marks_of(&s, P2, &theirs), (None, None));
        assert_eq!(marks_of(&s, P2, &mine), (Some(true), None));

        // p2's turn: p2 reads the reason on its own unit and still none on p1's.
        s.state_mut().active = P2;
        assert_eq!(
            marks_of(&s, P2, &theirs),
            (None, Some(NOT_ATTACK_POSITION.to_string()))
        );
        assert_eq!(marks_of(&s, P2, &mine), (Some(true), None));
        assert_eq!(marks_of(&s, P1, &theirs), (None, None));
    }
}
