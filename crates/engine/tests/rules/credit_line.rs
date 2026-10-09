//! Jlarna's credit line (R1223–R1225, Meditative #89): borrowing past current mana up to the
//! limit through the one affordability function, the repayment schedule with forgiveness (R1224),
//! and the shared line.

use jackioh_engine::subsystems::activate::why_cannot_activate_ability;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::credit::{
    act, act_result, body, in_hand, lender, lender_open, live, playing, put, slot, tapper,
};
use crate::rules::fixtures::harness::sink_for;

mod r1223_borrowing {
    use super::*;

    #[test]
    fn r1223_current_first_then_the_shortfall_is_borrowed() {
        let mut state = playing("credit-first");
        put(
            &mut state,
            &lender().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        // 2 current, a (3) body: the 2 go first and 1 is borrowed over four instalments.
        state.players.p1.mana.current = 2;
        let dealt = in_hand(&mut state, &body().id, PlayerId::P1, 1);
        let card = live(&state, &only(&dealt));
        assert_eq!(spendable_mana(&state, PlayerId::P1), 6);
        let after = act(
            &state,
            json!({
                "type": "play",
                "playerId": "p1",
                "instanceId": card.id,
                "zone": { "row": "units", "lane": 1 },
            }),
        );
        assert_eq!(after.players.p1.mana.current, 0);
        assert_eq!(after.players.p1.owed_instalments, Some(vec![1]));
        assert_eq!(after.players.p1.turn_log.mana_borrowed, Some(1));
        assert_eq!(after.players.p1.turn_log.borrowed_parts, Some(4));
        assert!(credit_used_this_turn(&after, PlayerId::P1));
    }

    #[test]
    fn r1223_owing_never_passes_the_limit_legal_actions_and_refusal_agree() {
        let mut state = playing("credit-limit");
        put(
            &mut state,
            &lender().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        state.players.p1.mana.current = 0;
        // Borrow 3 of the 4: 1, 1, 1 owed, 1 of credit left.
        let first = only(&in_hand(&mut state, &body().id, PlayerId::P1, 2));
        let card = live(&state, &first);
        let after = act(
            &state,
            json!({
                "type": "play",
                "playerId": "p1",
                "instanceId": card.id,
                "zone": { "row": "units", "lane": 1 },
            }),
        );
        assert_eq!(after.players.p1.owed_instalments, Some(vec![1, 1, 1]));
        assert_eq!(credit_available(&after, PlayerId::P1), 1);
        assert_eq!(spendable_mana(&after, PlayerId::P1), 1);

        // The second body needs 3 with only 1 spendable: the listing hides it, and the reducer
        // refuses it — the two agree.
        let second = after
            .players
            .p1
            .hand
            .iter()
            .find(|card| card.def_id == body().id)
            .cloned();
        let second = second.expect("the second body is still in hand");
        let listed: Vec<String> = legal_actions(&after, PlayerId::P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, .. } => Some(instance_id),
                _ => None,
            })
            .collect();
        assert!(!listed.contains(&second.id));
        let result = act_result(
            &after,
            json!({
                "type": "play",
                "playerId": "p1",
                "instanceId": second.id,
                "zone": { "row": "units", "lane": 2 },
            }),
        );
        assert!(
            result.error.is_some(),
            "the reducer refuses what the listing hides"
        );
    }

    #[test]
    fn r1223_two_lenders_share_one_line() {
        let mut state = playing("credit-shared");
        put(
            &mut state,
            &lender().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        put(
            &mut state,
            &lender_open().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        // Two Jlarnas do not raise the limit: still 4 over 4.
        let terms = credit_terms(&state, PlayerId::P1).expect("a line acts");
        assert_eq!(terms.limit, 4);
        assert_eq!(terms.instalments, 4);
        assert_eq!(credit_available(&state, PlayerId::P1), 4);
    }

    #[test]
    fn r1223_x_and_activate_costs_borrow() {
        let mut state = playing("credit-x-activate");
        put(
            &mut state,
            &lender().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let tapper = put(
            &mut state,
            &tapper().id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        state.players.p1.mana.current = 1;
        // A mana-2 Activate is usable on 1 mana: the other 1 is borrowed.
        assert!(why_cannot_activate_ability(&state, PlayerId::P1, &tapper.id, Some("tap")).is_ok());
        let after = act(
            &state,
            json!({ "type": "activate", "playerId": "p1", "instanceId": tapper.id, "ability": "tap" }),
        );
        assert_eq!(after.players.p1.mana.current, 0);
        assert_eq!(after.players.p1.owed_instalments, Some(vec![1]));
        // And an X may be chosen out of borrowed mana: 0 current + 3 of credit left.
        assert!(why_x_refused(&after, PlayerId::P1, 3, None).is_ok());
        assert!(why_x_refused(&after, PlayerId::P1, 4, None).is_err());
    }

    fn only<T: Clone>(items: &[T]) -> T {
        items.first().cloned().expect("expected at least one item")
    }
}

mod r1224_repayment {
    use super::*;

    #[test]
    fn r1224_split_larger_first() {
        assert_eq!(split_debt(4, 4), vec![1, 1, 1, 1]);
        assert_eq!(split_debt(3, 4), vec![1, 1, 1, 0]);
        assert_eq!(split_debt(2, 4), vec![1, 1, 0, 0]);
        assert_eq!(split_debt(6, 4), vec![2, 2, 1, 1]);
    }

    #[test]
    fn r1224_one_instalment_per_refresh_debts_add_up() {
        let mut state = playing("credit-add-up");
        put(
            &mut state,
            &lender().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        state.players.p1.mana.current = 0;
        // Borrow 3 this turn: 1, 1, 1.
        let owed = pay_on(&mut state, PlayerId::P1, 3);
        assert_eq!(owed, 3);
        assert_eq!(state.players.p1.owed_instalments, Some(vec![1, 1, 1]));
        // The next refresh takes the first instalment off what it gives.
        refresh_for(&mut state, PlayerId::P1, 4);
        assert_eq!(state.players.p1.mana.current, 3);
        assert_eq!(state.players.p1.owed_instalments, Some(vec![1, 1]));
        assert_eq!(state.players.p1.turn_log.mana_locked, Some(1));
    }

    #[test]
    fn r1224_a_short_refresh_forgives_the_rest() {
        let mut state = playing("credit-forgiven");
        put(
            &mut state,
            &lender().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        state.players.p1.owed_instalments = Some(vec![3, 1, 1, 1]);
        // A refresh that gives nothing forgives the instalment: mana stays 0 and the rest is dropped.
        refresh_for(&mut state, PlayerId::P1, 0);
        assert_eq!(state.players.p1.mana.current, 0);
        assert_eq!(state.players.p1.owed_instalments, Some(vec![1, 1, 1]));
        assert_eq!(state.players.p1.turn_log.mana_locked, None);
    }

    #[test]
    fn r1224_the_debt_outlives_the_lender() {
        let mut state = playing("credit-outlives");
        let lender = put(
            &mut state,
            &lender().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        state.players.p1.mana.current = 0;
        assert_eq!(pay_on(&mut state, PlayerId::P1, 2), 2);
        // The lender leaves; the schedule stays and the next refresh still collects.
        let gone = live(&state, &lender);
        {
            let mut sink = sink_for(&mut state);
            sacrifice_now(&mut sink, &gone);
        }
        assert!(!on_field(&state, &lender.id));
        assert_eq!(credit_terms(&state, PlayerId::P1), None);
        assert_eq!(owed_mana_of(&state, PlayerId::P1), 2);
        refresh_for(&mut state, PlayerId::P1, 4);
        assert_eq!(state.players.p1.mana.current, 3);
    }

    #[test]
    fn r1224_borrowing_on_the_opponents_turn() {
        let mut state = playing("credit-opponent-turn");
        put(
            &mut state,
            &lender().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        // P2 to act; P1 still borrows the same way, repaid from P1's own refreshes.
        state.active = PlayerId::P2;
        state.players.p1.mana.current = 1;
        assert_eq!(pay_on(&mut state, PlayerId::P1, 3), 2);
        assert_eq!(state.players.p1.owed_instalments, Some(vec![1, 1]));
        refresh_for(&mut state, PlayerId::P1, 4);
        assert_eq!(state.players.p1.mana.current, 3);
    }

    #[test]
    fn r1225_r1224_the_view_shows_the_offer_the_schedule_and_whether_it_was_used() {
        let mut state = playing("credit-view");
        put(
            &mut state,
            &lender().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let before = view_for(&state, PlayerId::P2)
            .opponent
            .credit
            .expect("a line acts");
        assert_eq!(before.available, Some(4));
        assert!(before.owed.is_empty());
        assert_eq!(before.used, Some(false), "the lapsing face shows the line unused");
        state.players.p1.mana.current = 0;
        assert_eq!(pay_on(&mut state, PlayerId::P1, 2), 2);
        // Both seats see it: the schedule is public, as Hinder's is.
        for viewer in [PlayerId::P1, PlayerId::P2] {
            let view = view_for(&state, viewer);
            let side = if viewer == PlayerId::P1 {
                view.you
            } else {
                view.opponent
            };
            let credit = side.credit.expect("a line acts");
            assert_eq!(credit.available, Some(2));
            assert_eq!(credit.owed, vec![1, 1]);
            assert_eq!(credit.used, Some(true), "still shown once the line is used");
        }
        // The next refresh locks the first instalment's crystal.
        refresh_for(&mut state, PlayerId::P1, 4);
        let credit = view_for(&state, PlayerId::P1).you.credit.expect("a line acts");
        assert_eq!(credit.locked, Some(1));
        assert_eq!(credit.owed, vec![1]);
    }

    #[test]
    fn r1224_a_game_without_credit_serializes_as_before() {
        let state = playing("credit-absent");
        let json = serde_json::to_value(&state).expect("a game serialises");
        let p1 = &json["players"]["p1"];
        assert_eq!(p1.get("owedInstalments"), None);
        assert_eq!(p1["turnLog"].get("manaBorrowed"), None);
        assert_eq!(p1["turnLog"].get("borrowedParts"), None);
        assert_eq!(p1["turnLog"].get("manaLocked"), None);
        assert_eq!(view_for(&state, PlayerId::P1).you.credit, None);
    }

    fn on_field(state: &GameState, id: &str) -> bool {
        find_instance(state, id).is_some_and(|card| matches!(card.zone, Zone::Field { .. }))
    }

    /// `pay_mana` straight on the state, as the pay steps call it.
    fn pay_on(state: &mut GameState, player: PlayerId, amount: i32) -> i32 {
        pay_mana(state, player, amount)
    }

    /// A refresh that gives `given` mana, as `refresh_mana` does past the max math.
    fn refresh_for(state: &mut GameState, player: PlayerId, given: i32) {
        let side = &mut state.players[player];
        side.mana.current = given;
        side.mana.max = given;
        side.mana.next_turn_mod = 0;
        take_instalment(side);
    }
}
