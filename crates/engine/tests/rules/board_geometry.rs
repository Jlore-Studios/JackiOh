//! Board geometry (issue #3554, R3554000–R3554004): the Black Hole, the slashed board and the
//! halves they leave, proved on the fixtures of `fixtures/board_geometry.rs`, with no card yet.

use jackioh_engine::effects;
use jackioh_engine::geometry::BlackHole;
use jackioh_engine::layers::unit_view;
use jackioh_engine::state_check::state_check;
use jackioh_engine::subsystems::ai_policy::policy_actions;
use jackioh_engine::subsystems::rotation::rotate_rings;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use super::fixtures::board_geometry::*;
use super::fixtures::harness::{live, put, run_effect, run_on_sink as run, slot};

/// p1's main phase with `p1` and `p2` as given, plain cards to draw, and no turn that ends by
/// itself, so each test ends every turn it means to.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_geometry_fixtures();
    let mut p1 = p1;
    let mut p2 = p2;
    for side in [&mut p1, &mut p2] {
        if side.get("library").is_none() {
            side["library"] = json!([PLAIN, PLAIN, PLAIN, PLAIN]);
        }
    }
    let mut s = scenario(json!({ "seed": seed, "active": "p1", "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn field_unit(def: &str, lane: i32) -> Value {
    json!({ "def": def, "lane": lane })
}

fn backrow_card(def: &str, lane: i32) -> Value {
    json!({ "def": def, "lane": lane, "row": "backrow" })
}

/// The id of the unit at `player`'s lane.
fn id_at(s: &Scenario, player: PlayerId, lane: i32) -> String {
    s.unit(player, lane)
        .unwrap_or_else(|| panic!("no unit at {player:?}'s lane {lane}"))
        .id
}

/// Every zone a play of `id` is offered, as (row, lane) pairs.
fn play_zones(s: &Scenario, id: &str) -> Vec<(Row, i32)> {
    legal_actions(s.state(), P1)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Play {
                instance_id, zone, ..
            } if instance_id == id => zone,
            _ => None,
        })
        .map(|zone| (zone.row, zone.lane))
        .collect()
}

/// The damage the `Damage` events dealt to `id`, in order.
fn dealt_to(events: &[GameEvent], id: &str) -> Vec<i32> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Damage {
                target_id, amount, ..
            } if target_id == id => Some(*amount),
            _ => None,
        })
        .collect()
}

/// The damage on the card `id` now.
fn damage_on(state: &GameState, id: &str) -> i32 {
    find_instance(state, id).map_or(0, |card| card.damage)
}

/// Where a card sits now, as "p2 units 5", for readable assertions.
fn where_is(state: &GameState, id: &str) -> String {
    match find_instance(state, id).map(|found| &found.zone) {
        None => "gone".to_string(),
        Some(Zone::Field { player, row, lane }) => format!("{player} {row} {lane}"),
        Some(zone) => zone.z().to_string(),
    }
}

/// Whether `legal_actions` offers `player` an attack of `attacker` on `target_id`.
fn offers_attack(s: &Scenario, player: PlayerId, attacker: &str, target_id: &str) -> bool {
    legal_actions(s.state(), player).iter().any(|action| {
        matches!(
            action,
            ActionBody::Attack { attacker_id, target_id: offered }
                if attacker_id == attacker && offered == target_id
        )
    })
}

/// Declare `attacker`'s attack on the unit `target`, answering the refusal's text or `None`.
fn declare(state: &mut GameState, attacker: &str, target: &str) -> Option<String> {
    let attacker = live(state, attacker);
    let target = AttackTarget::Unit {
        instance: live(state, target),
    };
    run(state, |sink| {
        declare_attack(sink, &attacker, &target)
            .err()
            .map(|error| error.message)
    })
    .0
}

/// Every `Answer` body `legal_actions` offers `player`.
fn answers(state: &GameState, player: PlayerId) -> Vec<ActionBody> {
    legal_actions(state, player)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Answer { .. }))
        .collect()
}

mod r3554000_black_hole {
    use super::*;

    #[test]
    fn r3554000_opening_swallows_the_midlane_and_counts_a_pile_once() {
        let mut s = board(
            "r3554000-opening",
            json!({
                "hand": [HOLE],
                "field": [field_unit(PLAIN, 3), backrow_card(LANCER, 3)],
            }),
            json!({
                "field": [field_unit(PLAIN, 3), backrow_card(LANCER, 3)],
            }),
        );
        // A Stack pile in the midlane: two plains, one zone.
        let at = slot(P1, Row::Units, 3);
        let mut extra = new_instance(s.state_mut(), PLAIN, P1, Zone::Hand { player: P1 });
        assert!(
            place_on_field(
                s.state_mut(),
                &mut extra,
                at,
                PlaceOnFieldOptions { stack: Some(true) }
            ),
            "the second plain stacks onto the midlane"
        );
        let pile = id_at(&s, P1, 3);
        s.play(HOLE, json!({}));
        let state = s.state();
        // Five cards stood in the midlane; the pile counted once: four swallowed.
        assert_eq!(where_is(state, &pile), "exile");
        for (player, row, lane) in [
            (P1, Row::Units, 3),
            (P1, Row::Backrow, 3),
            (P2, Row::Units, 3),
            (P2, Row::Backrow, 3),
        ] {
            assert!(
                card_at(state, slot(player, row, lane)).is_none(),
                "{player} {row} {lane} stands empty"
            );
        }
        assert_eq!(
            state.black_hole.map(|hole| hole.swallowed_count()),
            Some(4),
            "the pile counted once"
        );
        assert_eq!(state.black_hole.map(|hole| hole.lane), Some(3));
    }

    #[test]
    fn r3554000_hole_is_no_play_zone_and_a_rotation_into_it_is_swallowed() {
        let mut s = board(
            "r3554000-no-play",
            json!({ "hand": [HOLE, PLAIN], "field": [field_unit(PLAIN, 2)] }),
            json!({}),
        );
        s.play(HOLE, json!({}));
        let plain = s
            .hand(P1)
            .iter()
            .find(|card| card.def_id == PLAIN)
            .expect("a plain in hand")
            .id
            .clone();
        assert!(
            play_zones(&s, &plain).iter().all(|(_, lane)| *lane != 3),
            "no play enters the hole: {:?}",
            play_zones(&s, &plain)
        );
        // A rotation into the hole lands, then the check swallows it.
        let at_lane_2 = id_at(&s, P1, 2);
        run(s.state_mut(), |sink| {
            rotate_rings(
                sink,
                &json_as(json!({ "direction": "right", "perspective": "p1" })),
            )
        });
        assert_eq!(where_is(s.state(), &at_lane_2), "p1 units 3");
        run(s.state_mut(), state_check);
        assert_eq!(where_is(s.state(), &at_lane_2), "exile");
        assert_eq!(s.state().black_hole.map(|hole| hole.swallowed_count()), Some(1));
    }

    #[test]
    fn r3554000_closes_at_capacity() {
        let mut s = board(
            "r3554000-capacity",
            json!({ "field": [field_unit(PLAIN, 3)] }),
            json!({ "field": [field_unit(PLAIN, 3)] }),
        );
        run_effect(s.state_mut(), effects::open_black_hole(json_as(json!({}))), P1);
        s.state_mut().black_hole = Some(BlackHole {
            lane: 3,
            swallowed: Some(9),
            pulls: None,
            capacity: None,
        });
        let first = id_at(&s, P1, 3);
        let second = id_at(&s, P2, 3);
        run(s.state_mut(), state_check);
        // At 9 swallowed with two cards in the lane, only one goes, and the hole closes.
        assert_eq!(where_is(s.state(), &first), "exile");
        assert_eq!(where_is(s.state(), &second), "p2 units 3");
        assert_eq!(s.state().black_hole, None);
    }

    #[test]
    fn r3561000_a_hole_opened_with_a_capacity_closes_there() {
        let mut s = board("r3561000-capacity", json!({}), json!({}));
        run_effect(s.state_mut(), effects::open_black_hole(json_as(json!({}))), P1);
        assert_eq!(
            s.state().black_hole.map(|hole| hole.closes_at()),
            Some(BLACK_HOLE_CAPACITY),
            "a hole opened with no number closes at the engine's"
        );
        run_effect(
            s.state_mut(),
            effects::open_black_hole(json_as(json!({ "capacity": 1 }))),
            P1,
        );
        assert_eq!(s.state().black_hole.map(|hole| hole.closes_at()), Some(1));
        assert_eq!(
            s.view(P2).black_hole.map(|hole| hole.capacity),
            Some(1),
            "the view shows the number the hole closes at"
        );
        let mine = put(s.state_mut(), PLAIN, slot(P1, Row::Units, 3), json!({})).id;
        let theirs = put(s.state_mut(), PLAIN, slot(P2, Row::Units, 3), json!({})).id;
        run(s.state_mut(), state_check);
        // One swallowed closes it: the second card in the lane stays.
        assert_eq!(where_is(s.state(), &mine), "exile");
        assert_eq!(where_is(s.state(), &theirs), "p2 units 3");
        assert_eq!(s.state().black_hole, None);
    }
}

mod r3554001_pull {
    use super::*;

    #[test]
    fn r3554001_pull_steps_toward_the_hole_nearest_first() {
        let mut s = board(
            "r3554001-pull",
            json!({
                "hand": [HOLE],
                "field": [
                    field_unit(PLAIN, 1),
                    field_unit(PLAIN, 2),
                    field_unit(PLAIN, 4),
                    field_unit(PLAIN, 5),
                ],
            }),
            json!({}),
        );
        s.play(HOLE, json!({}));
        let (one, two, four, five) = (
            id_at(&s, P1, 1),
            id_at(&s, P1, 2),
            id_at(&s, P1, 4),
            id_at(&s, P1, 5),
        );
        s.end_turn();
        s.end_turn();
        // p1's turn began again: lanes 2 and 4 were swallowed, 1 stepped to 2 and 5 to 4.
        assert_eq!(where_is(s.state(), &two), "exile");
        assert_eq!(where_is(s.state(), &four), "exile");
        assert_eq!(where_is(s.state(), &one), "p1 units 2");
        assert_eq!(where_is(s.state(), &five), "p1 units 4");
        assert_eq!(s.state().black_hole.map(|hole| hole.swallowed_count()), Some(2));
    }

    #[test]
    fn r3554001_radiant_pulls_only_the_opponent() {
        let mut s = board(
            "r3554001-radiant",
            json!({
                "hand": [{ "def": HOLE, "radiant": true }],
                "field": [field_unit(PLAIN, 2)],
            }),
            json!({ "field": [field_unit(PLAIN, 1), field_unit(PLAIN, 4)] }),
        );
        s.play(HOLE, json!({}));
        let mine = id_at(&s, P1, 2);
        let (near, far) = (id_at(&s, P2, 1), id_at(&s, P2, 4));
        s.end_turn();
        // The opponent's turn began: only their permanents stepped — lane 1 moved up, lane 4
        // was swallowed.
        assert_eq!(where_is(s.state(), &mine), "p1 units 2");
        assert_eq!(where_is(s.state(), &near), "p2 units 2");
        assert_eq!(where_is(s.state(), &far), "exile");
        s.end_turn();
        // My turn began: the Radiant hole pulls nothing of mine.
        assert_eq!(where_is(s.state(), &mine), "p1 units 2");
        assert_eq!(where_is(s.state(), &near), "p2 units 2");
    }

    #[test]
    fn r3554001_pull_passes_dormant_cards_by() {
        let mut s = board(
            "r3554001-dormant",
            json!({
                "hand": [HOLE],
                "field": [field_unit(PLAIN, 2)],
            }),
            json!({}),
        );
        // A Dormant card beside the hole: stamped Slumber by hand (white-box).
        let at = slot(P1, Row::Units, 4);
        let dormant = put(s.state_mut(), PLAIN, at, json!({}));
        find_instance_mut(s.state_mut(), &dormant.id)
            .expect("the unit")
            .slumber = Some(Slumber { turns: Some(2) });
        s.play(HOLE, json!({}));
        let mine = id_at(&s, P1, 2);
        s.end_turn();
        s.end_turn();
        assert_eq!(where_is(s.state(), &mine), "exile", "lane 2 was swallowed");
        assert_eq!(
            where_is(s.state(), &dormant.id),
            "p1 units 4",
            "the hole passes Dormant cards by"
        );
    }

    #[test]
    fn r3554001_giga_steps_whole_or_is_swallowed() {
        let mut s = board(
            "r3554001-giga",
            json!({ "field": [field_unit(GIGA, 1)] }),
            json!({}),
        );
        run_effect(s.state_mut(), effects::open_black_hole(json_as(json!({}))), P1);
        let giga = id_at(&s, P1, 1);
        s.end_turn();
        s.end_turn();
        // Its shifted lanes reached the hole: the whole GIGA went, counted once.
        assert_eq!(where_is(s.state(), &giga), "exile");
        assert_eq!(s.state().black_hole.map(|hole| hole.swallowed_count()), Some(1));
        for (row, lane) in [
            (Row::Units, 1),
            (Row::Units, 2),
            (Row::Backrow, 1),
            (Row::Backrow, 2),
        ] {
            assert!(
                is_empty(s.state(), slot(P1, row, lane)),
                "p1 {row} {lane} stands empty"
            );
        }
    }
}

mod r3554002_slash {
    use super::*;

    #[test]
    fn r3554002_slash_exiles_the_lane_and_voids_it() {
        let mut s = board(
            "r3554002-slash",
            json!({
                "hand": [SLASH, PLAIN],
                "field": [field_unit(PLAIN, 3), backrow_card(LANCER, 3)],
            }),
            json!({
                "field": [field_unit(PLAIN, 3), backrow_card(LANCER, 3)],
            }),
        );
        let (mine, theirs) = (id_at(&s, P1, 3), id_at(&s, P2, 3));
        s.play(SLASH, json!({}));
        assert_eq!(where_is(s.state(), &mine), "exile");
        assert_eq!(where_is(s.state(), &theirs), "exile");
        assert_eq!(s.state().slashed_lane, Some(3));
        // No play, summon or GIGA can enter the void lane, and a rotation bounces.
        let plain = s
            .hand(P1)
            .iter()
            .find(|card| card.def_id == PLAIN)
            .expect("a plain in hand")
            .id
            .clone();
        assert!(
            play_zones(&s, &plain).iter().all(|(_, lane)| *lane != 3),
            "no play enters the void: {:?}",
            play_zones(&s, &plain)
        );
        run_effect(
            s.state_mut(),
            effects::summon(json_as(json!({ "defId": PLAIN, "lane": 3 }))),
            P1,
        );
        assert!(
            card_at(s.state(), slot(P1, Row::Units, 3)).is_none(),
            "a summon into the void places nothing"
        );
        run_effect(
            s.state_mut(),
            effects::summon(json_as(json!({ "defId": GIGA, "lane": 3 }))),
            P1,
        );
        assert!(
            card_at(s.state(), slot(P1, Row::Units, 3)).is_none(),
            "a GIGA into the void places nothing"
        );
        // A rotation into the void bounces home instead of panicking.
        let at_lane_2 = put(s.state_mut(), PLAIN, slot(P1, Row::Units, 2), json!({}));
        run(s.state_mut(), |sink| {
            rotate_rings(
                sink,
                &json_as(json!({ "direction": "right", "perspective": "p1" })),
            )
        });
        assert_eq!(
            where_is(s.state(), &at_lane_2.id),
            "hand",
            "a rotation into the void bounces home"
        );
        // A second slash does nothing.
        run_effect(s.state_mut(), effects::slash_board(json_as(json!({}))), P1);
        assert_eq!(s.state().slashed_lane, Some(3));
    }

    #[test]
    fn r3554002_radiant_slash_lane_prompt_is_answered() {
        let mut s = board(
            "r3554002-prompt",
            json!({ "hand": [{ "def": SLASH, "radiant": true }] }),
            json!({}),
        );
        s.play(SLASH, json!({}));
        assert!(s.state().pending.is_some(), "the lane pick is open");
        // The options are in `legal_actions` and the AI policy answers too.
        assert_eq!(answers(s.state(), P1).len(), 3);
        assert!(
            policy_actions(s.state(), P1, Default::default())
                .iter()
                .any(|action| matches!(action, ActionBody::Answer { .. })),
            "the AI policy answers the pick"
        );
        // The clock answers the pick through the AI policy.
        let timed_out = reduce(s.state(), &Action::new(ActionBody::Timeout, P1, "srv-t"));
        assert_eq!(timed_out.error, None);
        assert!(
            timed_out.state.pending.is_none(),
            "the clock answers the pick through the AI policy"
        );
        assert!(
            SLASH_LANES.contains(&timed_out.state.slashed_lane.unwrap_or(0)),
            "the timeout slashed one of the three lanes"
        );
    }

    #[test]
    fn r3554002_radiant_slash_picks_its_lane() {
        let mut s = board(
            "r3554002-pick",
            json!({
                "hand": [{ "def": SLASH, "radiant": true }],
                "field": [field_unit(PLAIN, 2)],
            }),
            json!({ "field": [field_unit(PLAIN, 2)] }),
        );
        s.play(SLASH, json!({}));
        s.answer(json!(["4"]));
        assert_eq!(s.state().slashed_lane, Some(4));
        assert_eq!(where_is(s.state(), &id_at(&s, P1, 2)), "p1 units 2");
    }

    #[test]
    fn r3554002_lanes_with_cleave_across_crater_and_giga() {
        let mut s = board(
            "r3554002-lanes",
            json!({
                "hand": [SLASH],
                "field": [
                    field_unit(CLEAVER, 1),
                    field_unit(CRATER, 2),
                    field_unit(GIGA, 4),
                ],
            }),
            json!({
                "field": [
                    field_unit(BULWARK, 1),
                    field_unit(BULWARK, 2),
                    field_unit(BULWARK, 5),
                ],
            }),
        );
        s.play(SLASH, json!({}));
        // A Cleave within its half still splashes its neighbour.
        let (cleaver, first, second) = (id_at(&s, P1, 1), id_at(&s, P2, 1), id_at(&s, P2, 2));
        s.attack(cleaver.as_str(), first.as_str());
        assert_eq!(damage_on(s.state(), &first), 3);
        assert_eq!(damage_on(s.state(), &second), 3, "the Cleave crossed no slash");
        // A GIGA still meets a lane from either of its lanes, and not a farther one.
        let giga = live(s.state(), id_at(&s, P1, 4));
        let near = live(s.state(), id_at(&s, P2, 5));
        let far = live(s.state(), id_at(&s, P2, 2));
        assert!(lanes_meet(&giga, &near));
        assert!(!lanes_meet(&giga, &far));
        // A summoned Crater Locks the zone it arrives in.
        run_effect(
            s.state_mut(),
            effects::summon(json_as(json!({ "defId": CRATER, "lane": 4 }))),
            P2,
        );
        assert!(
            is_locked(s.state(), slot(P2, Row::Units, 4)),
            "Crater locks its zone"
        );
    }
}

mod r3554003_halves {
    use super::*;

    /// Force `attacker` onto the unit `target`: the damage events it emitted (none across the slash,
    /// which a forced attack passes over in silence).
    fn force(state: &mut GameState, attacker: &str, target: &str) -> Vec<GameEvent> {
        let attacker = live(state, attacker);
        let target = AttackTarget::Unit {
            instance: live(state, target),
        };
        run(state, |sink| force_attack(sink, &attacker, &target)).1
    }

    #[test]
    fn r3554003_attacks_taunt_auras_and_scopes_keep_to_their_half() {
        let mut s = board(
            "r3554003-halves",
            json!({
                "hand": [SLASH, CRIER],
                "field": [
                    field_unit(PLAIN, 1),
                    field_unit(AURA, 2),
                    field_unit(PLAIN, 5),
                ],
            }),
            json!({
                "field": [
                    field_unit(PLAIN, 1),
                    field_unit(TAUNT, 5),
                ],
            }),
        );
        s.play(SLASH, json!({}));
        let (mine_left, foe_left) = (id_at(&s, P1, 1), id_at(&s, P2, 1));
        let (mine_right, foe_right) = (id_at(&s, P1, 5), id_at(&s, P2, 5));
        // Attacks across the slash are refused; within a half they are offered.
        assert_eq!(
            declare(s.state_mut(), &mine_left, &foe_right),
            Some("that unit is across the slash".to_string())
        );
        assert!(!offers_attack(&s, P1, &mine_left, &foe_right));
        assert!(offers_attack(&s, P1, &mine_left, &foe_left));
        // A Taunt across the slash walls nothing on this half.
        assert!(offers_attack(&s, P1, &mine_left, &foe_left));
        // A forced attack obeys too: silence across the slash, damage within the half.
        assert!(force(s.state_mut(), &mine_left, &foe_right).is_empty());
        assert!(!force(s.state_mut(), &mine_left, &foe_left).is_empty());
        // The aura reaches its own half only: +1 beside it, none across.
        assert_eq!(unit_view(s.state(), &live(s.state(), &mine_left)).attack, 2);
        assert_eq!(unit_view(s.state(), &live(s.state(), &mine_right)).attack, 1);
        // A scope from one half takes nothing of the other: the crier played on the right.
        s.play(CRIER, json!({ "zone": 4 }));
        assert_eq!(dealt_to(s.last_events(), &foe_right), vec![1]);
        assert!(dealt_to(s.last_events(), &foe_left).is_empty());
    }

    #[test]
    fn r3554003_summons_keep_to_their_half() {
        let mut s = board(
            "r3554003-summon",
            json!({ "hand": [SLASH, MOTHER, MOTHER], "field": [field_unit(PLAIN, 4)] }),
            json!({}),
        );
        s.play(SLASH, json!({}));
        // The summoner's half is full: nothing lands, though the other half stands open.
        let first = s
            .hand(P1)
            .iter()
            .find(|card| card.def_id == MOTHER)
            .expect("a mother in hand")
            .id
            .clone();
        s.play(first.as_str(), json!({ "zone": 5 }));
        assert_eq!(
            unit_count(s.state(), P1),
            2,
            "the summon fizzled on its full half"
        );
        // With room on its half, the summon lands there.
        let before: Vec<String> = s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .map(|card| card.id.clone())
            .collect();
        let second = s
            .hand(P1)
            .iter()
            .find(|card| card.def_id == MOTHER)
            .expect("a mother in hand")
            .id
            .clone();
        s.play(second.as_str(), json!({ "zone": 1 }));
        assert_eq!(where_is(s.state(), &newest_plain(&s, &before)), "p1 units 2");
    }

    /// How many cards stand in `player`'s unit zones.
    fn unit_count(state: &GameState, player: PlayerId) -> usize {
        state.players[player].units.iter().flatten().flatten().count()
    }

    /// The id of the plain summoned since `before`: the summon's.
    fn newest_plain(s: &Scenario, before: &[String]) -> String {
        s.state().players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .find(|card| card.def_id == PLAIN && !before.contains(&card.id))
            .map(|card| card.id.clone())
            .expect("a summoned plain")
    }
}

mod r3554004_spell_half {
    use super::*;

    #[test]
    fn r3554004_untargeted_spell_asks_a_half_and_hits_only_it() {
        let mut s = board(
            "r3554004-untargeted",
            json!({ "hand": [SLASH, BLAST] }),
            json!({
                "field": [field_unit(BULWARK, 1), field_unit(BULWARK, 4)],
            }),
        );
        s.play(SLASH, json!({}));
        s.play(BLAST, json!({}));
        assert!(s.state().pending.is_some(), "the half pick is open");
        // The prompt is answerable by `legal_actions`, the policy and the timeout.
        assert_eq!(answers(s.state(), P1).len(), 2);
        assert!(
            policy_actions(s.state(), P1, Default::default())
                .iter()
                .any(|action| matches!(action, ActionBody::Answer { .. })),
            "the AI policy answers the pick"
        );
        let timed_out = reduce(s.state(), &Action::new(ActionBody::Timeout, P1, "srv-t"));
        assert_eq!(timed_out.error, None);
        assert!(timed_out.state.pending.is_none());
        let (left, right) = (id_at(&s, P2, 1), id_at(&s, P2, 4));
        let hit_left = damage_on(&timed_out.state, &left) > 0;
        let hit_right = damage_on(&timed_out.state, &right) > 0;
        assert!(hit_left != hit_right, "the blast hit exactly one half");
    }

    #[test]
    fn r3554004_targeted_spell_takes_its_targets_half_unasked() {
        let mut s = board(
            "r3554004-targeted",
            json!({ "hand": [SLASH, BOLT] }),
            json!({
                "field": [field_unit(BULWARK, 1), field_unit(BULWARK, 4)],
            }),
        );
        s.play(SLASH, json!({}));
        let right = id_at(&s, P2, 4);
        s.play(
            BOLT,
            json!({ "targets": [{ "pick": "instance", "instanceId": right }] }),
        );
        // No half prompt: the bolt took its target's half.
        assert!(s.state().pending.is_none());
        assert_eq!(damage_on(s.state(), &right), 3);
        assert_eq!(damage_on(s.state(), &id_at(&s, P2, 1)), 0);
    }

    #[test]
    fn r3561001_a_spell_that_slashes_the_board_asks_no_half() {
        let mut s = board(
            "r3561001-no-half",
            json!({ "hand": [{ "def": SLASH, "radiant": true }] }),
            json!({}),
        );
        s.play(SLASH, json!({}));
        assert!(s.state().pending.is_some(), "the lane pick is open");
        s.answer(json!(["4"]));
        assert_eq!(s.state().slashed_lane, Some(4));
        assert!(
            s.state().pending.is_none(),
            "the board it slashed asks no half: {:?}",
            s.state().pending
        );
    }
}
