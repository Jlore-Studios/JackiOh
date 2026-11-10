//! GIGA (issue #3546, R3546000–R3546004): a permanent two lanes wide in both rows, in both
//! lanes for every rule that reads a lane, and the mark that makes a card GIGA from the hand or on
//! embiggen. Proved on the fixtures of `fixtures/giga.rs`, with no card yet.

use jackioh_engine::effects::move_::bounce_card;
use jackioh_engine::effects::transform::vanilla;
use jackioh_engine::effects::{grant_keyword, grant_keyword_cards, steal, summon, swap_board};
use jackioh_engine::state_check::state_check;
use jackioh_engine::subsystems::rotation::rotate_rings;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;
use jackioh_engine::view_for::view_for;

use super::fixtures::giga::*;
use super::fixtures::harness::{Bench, live, run_effect, run_on_sink as run, slot};

/// p1's main phase with `p1` and `p2` as given, plain cards to draw, and no turn that ends by
/// itself, so each test ends every turn it means to.
fn board(seed: &str, p1: Value, p2: Value) -> Scenario {
    register_giga_fixtures();
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

/// Every zone a play of `id` is offered, as (row, lane) pairs.
fn zone_pairs(s: &Scenario, id: &str) -> Vec<(Row, i32)> {
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

/// Every unit lane a play of `id` is offered, sorted.
fn unit_lanes(s: &Scenario, id: &str) -> Vec<i32> {
    let mut lanes: Vec<i32> = zone_pairs(s, id)
        .into_iter()
        .filter(|(row, _)| *row == Row::Units)
        .map(|(_, lane)| lane)
        .collect();
    lanes.sort();
    lanes
}

/// Whether `legal_actions` offers `attacker` an attack on `target_id` for `player`.
fn offers(s: &Scenario, player: PlayerId, attacker: &str, target_id: &str) -> bool {
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

/// `force_attack` on the unit `target`, with both read live: the events it emitted.
fn force(state: &mut GameState, attacker: &str, target: &str) -> Vec<GameEvent> {
    let attacker = live(state, attacker);
    let target = AttackTarget::Unit {
        instance: live(state, target),
    };
    run(state, |sink| force_attack(sink, &attacker, &target)).1
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

fn field_unit(def: &str, lane: i32) -> Value {
    json!({ "def": def, "lane": lane })
}

fn backrow_card(def: &str, lane: i32) -> Value {
    json!({ "def": def, "lane": lane, "row": "backrow" })
}

fn card_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

/// Enough damage to put a unit at 0 health, read through the layers (§10.4).
fn lethal_damage(state: &mut GameState, id: &str) {
    let unit = find_instance(state, id).unwrap_or_else(|| panic!("no instance {id}"));
    let max_health = layers::unit_view(state, unit).max_health;
    card_mut(state, id).damage = max_health;
}

/// Where a card sits now, as "p2 units 5", for readable assertions.
fn where_is(state: &GameState, card: &CardInstance) -> String {
    match find_instance(state, &card.id).map(|found| &found.zone) {
        None => "gone".to_string(),
        Some(Zone::Field { player, row, lane }) => format!("{player} {row} {lane}"),
        Some(zone) => zone.z().to_string(),
    }
}

/// One `rotate_rings` over a fresh sink.
fn rotate(state: &mut GameState, direction: &str) {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let args = json!({ "direction": direction, "perspective": "p1" });
    rotate_rings(&mut EngineSink::new(state, &mut events, &mut rng), &json_as(args));
}

mod r3546000_four_zones {
    use super::*;

    #[test]
    fn r3546000_a_giga_is_offered_only_the_left_lanes_where_it_fits() {
        let s = board(
            "r3546000-offered",
            json!({ "mana": 4, "hand": [GIGA_UNIT], "field": [field_unit(PLAIN, 3)] }),
            json!({}),
        );
        let id = s.card(GIGA_UNIT).id.clone();
        assert_eq!(unit_lanes(&s, &id), vec![1, 4]);
    }

    #[test]
    fn r3546000_a_played_giga_fills_four_zones_and_nothing_enters_them() {
        let mut s = board(
            "r3546000-fills",
            json!({ "mana": 4, "hand": [GIGA_UNIT, PLAIN] }),
            json!({}),
        );
        s.play(GIGA_UNIT, json!({ "zone": 2 }));
        let state = s.state();
        for (row, lane) in [
            (Row::Units, 2),
            (Row::Units, 3),
            (Row::Backrow, 2),
            (Row::Backrow, 3),
        ] {
            assert!(
                !is_empty(state, slot(P1, row, lane)),
                "{row} {lane} is filled by the GIGA"
            );
        }
        let id = s.card(PLAIN).id.clone();
        assert_eq!(unit_lanes(&s, &id), vec![1, 4, 5]);
        run_effect(
            s.state_mut(),
            summon(json_as(json!({ "defId": PLAIN, "lane": 3 }))),
            P1,
        );
        assert!(
            card_at(s.state(), slot(P1, Row::Units, 3)).is_none(),
            "a summon into a covered lane places nothing"
        );
    }

    #[test]
    fn r3546000_a_giga_summon_takes_the_leftmost_fit_or_fails() {
        let mut s = board(
            "r3546000-summon-blocked",
            json!({ "field": [field_unit(PLAIN, 2), field_unit(PLAIN, 4)] }),
            json!({}),
        );
        run_effect(s.state_mut(), summon(json_as(json!({ "defId": GIGA_UNIT }))), P1);
        assert!(
            s.state().players[P1]
                .units
                .iter()
                .flatten()
                .flatten()
                .all(|card| card.def_id != GIGA_UNIT),
            "no fit anywhere, so nothing lands"
        );

        let mut s = board(
            "r3546000-summon-fit",
            json!({ "field": [field_unit(PLAIN, 1)] }),
            json!({}),
        );
        run_effect(s.state_mut(), summon(json_as(json!({ "defId": GIGA_UNIT }))), P1);
        let landed = card_at(s.state(), slot(P1, Row::Units, 2))
            .unwrap_or_else(|| panic!("a GIGA summon lands at the leftmost fit"));
        assert_eq!(landed.def_id, GIGA_UNIT);
        assert_eq!(landed.giga, Some(true));
    }

    #[test]
    fn r3546000_a_giga_field_spell_fills_the_unit_zones_of_its_lanes() {
        let s = board(
            "r3546000-field",
            json!({ "mana": 4, "hand": [PLAIN], "field": [backrow_card(GIGA_FIELD, 1)] }),
            json!({}),
        );
        let state = s.state();
        assert!(!is_empty(state, slot(P1, Row::Units, 1)));
        assert!(!is_empty(state, slot(P1, Row::Units, 2)));
        let id = s.card(PLAIN).id.clone();
        assert_eq!(unit_lanes(&s, &id), vec![3, 4, 5]);
    }

    #[test]
    fn r3546000_nothing_stacks_onto_a_giga() {
        let s = board(
            "r3546000-stack",
            json!({ "mana": 4, "hand": [STACKER], "field": [field_unit(GIGA_UNIT, 1)] }),
            json!({}),
        );
        let id = s.card(STACKER).id.clone();
        assert_eq!(unit_lanes(&s, &id), vec![3, 4, 5]);
    }
}

mod r3546001_both_lanes {
    use super::*;

    #[test]
    fn r3546001_cleave_on_a_giga_hits_the_units_beside_both_its_lanes() {
        let mut s = board(
            "r3546001-cleave-both",
            json!({ "field": [field_unit(CLEAVER, 1)] }),
            json!({ "field": [field_unit(GIGA_UNIT, 2), field_unit(PLAIN, 1), field_unit(PLAIN, 4)] }),
        );
        let cleaver = s.unit(P1, 1).expect("no cleaver in lane 1").id.clone();
        let giga = s.unit(P2, 2).expect("no GIGA in lane 2").id.clone();
        let left = s.unit(P2, 1).expect("no plain in lane 1").id.clone();
        let right = s.unit(P2, 4).expect("no plain in lane 4").id.clone();
        let events = force(s.state_mut(), &cleaver, &giga);
        assert_eq!(dealt_to(&events, &left), vec![3], "beside its first lane");
        assert_eq!(dealt_to(&events, &right), vec![3], "beside its second lane");
        assert_eq!(dealt_to(&events, &giga), vec![3], "the target takes the hit");
    }

    #[test]
    fn r3546001_a_cleave_beside_a_giga_hits_it_once() {
        let mut s = board(
            "r3546001-cleave-once",
            json!({ "field": [field_unit(CLEAVER, 3)] }),
            json!({ "field": [field_unit(GIGA_UNIT, 1), field_unit(PLAIN, 3)] }),
        );
        let cleaver = s.unit(P1, 3).expect("no cleaver in lane 3").id.clone();
        let target = s.unit(P2, 3).expect("no plain in lane 3").id.clone();
        let giga = s.unit(P2, 1).expect("no GIGA in lane 1").id.clone();
        let events = force(s.state_mut(), &cleaver, &target);
        assert_eq!(
            dealt_to(&events, &giga),
            vec![3],
            "a GIGA beside the target takes Cleave's hit once, not per lane"
        );
    }

    #[test]
    fn r3546001_only_units_in_this_lane_admits_a_giga_from_either_lane() {
        let mut s = board(
            "r3546001-lane-only",
            json!({ "field": [field_unit(GIGA_UNIT, 1)] }),
            json!({ "field": [field_unit(LANE_ONLY, 2), field_unit(LANE_ONLY, 3)] }),
        );
        let giga = s.unit(P1, 1).expect("no GIGA in lane 1").id.clone();
        let near = s.unit(P2, 2).expect("no lane-only in lane 2").id.clone();
        let far = s.unit(P2, 3).expect("no lane-only in lane 3").id.clone();
        assert!(offers(&s, P1, &giga, &near), "its second lane meets lane 2");
        assert!(!offers(&s, P1, &giga, &far), "neither lane meets lane 3");
        assert!(
            declare(s.state_mut(), &giga, &far)
                .is_some_and(|refusal| refusal.contains("only a unit in its lane")),
            "declared attacks are refused across lanes"
        );
    }

    #[test]
    fn r3546001_a_lane_read_finds_a_giga_in_both_lanes_of_its_row() {
        let s = board(
            "r3546001-reads",
            json!({ "field": [field_unit(GIGA_UNIT, 2)] }),
            json!({}),
        );
        let state = s.state();
        let giga = find_instance(state, &s.unit(P1, 2).expect("no GIGA in lane 2").id)
            .expect("no GIGA")
            .clone();
        assert_eq!(lanes_of(&giga), vec![2, 3]);
        assert_eq!(
            occupant_at(state, slot(P1, Row::Units, 3)).map(|card| card.id.clone()),
            Some(giga.id.clone()),
            "covered in its own row"
        );
        assert!(
            occupant_at(state, slot(P1, Row::Backrow, 2)).is_none(),
            "no unit-zone occupant in the backrow"
        );
        assert!(
            occupant_at(state, slot(P1, Row::Backrow, 3)).is_none(),
            "no unit-zone occupant in the backrow"
        );
        assert!(!lane_is_empty(state, 2), "its anchor lane");
        assert!(!lane_is_empty(state, 3), "its covered lane");
        assert!(lane_is_empty(state, 4), "a free lane");
    }

    #[test]
    fn r3546001_a_giga_attacks_once() {
        let mut s = board(
            "r3546001-once",
            json!({ "field": [field_unit(GIGA_UNIT, 1)] }),
            json!({ "field": [field_unit(PLAIN, 1)] }),
        );
        let giga = s.unit(P1, 1).expect("no GIGA in lane 1").id.clone();
        let plain = s.unit(P2, 1).expect("no plain in lane 1").id.clone();
        force(s.state_mut(), &giga, &plain);
        let state = s.state();
        assert_eq!(damage_on(state, &plain), 2, "one strike of 2");
        assert_eq!(damage_on(state, &giga), 1, "one strike back of 1");
    }
}

mod r3546002_taken_on_entry {
    use super::*;

    #[test]
    fn r3546002_a_hand_card_given_giga_plays_into_four_zones_and_a_bounce_ends_it() {
        let mut s = board("r3546002-grant", json!({ "mana": 4, "hand": [PLAIN] }), json!({}));
        run_effect(
            s.state_mut(),
            grant_keyword_cards(json_as(
                json!({ "scope": { "zones": ["hand"] }, "keyword": { "kind": "GIGA" } }),
            )),
            P1,
        );
        let id = s.card(PLAIN).id.clone();
        assert_eq!(
            unit_lanes(&s, &id),
            vec![1, 2, 3, 4],
            "granted GIGA plays GIGA-sized"
        );
        s.play(PLAIN, json!({ "zone": 1 }));
        let state = s.state();
        for (row, lane) in [
            (Row::Units, 1),
            (Row::Units, 2),
            (Row::Backrow, 1),
            (Row::Backrow, 2),
        ] {
            assert!(!is_empty(state, slot(P1, row, lane)), "{row} {lane} is filled");
        }
        let landed = live(s.state(), &id);
        assert_eq!(landed.giga, Some(true));
        run(s.state_mut(), |sink| {
            let card = find_instance(sink.state, &id).expect("no granted GIGA").clone();
            bounce_card(sink, &card);
        });
        let back = live(s.state(), &id);
        assert_eq!(back.zone.z(), ZoneName::Hand, "bounced to hand");
        assert_eq!(back.giga, None, "R78's reset ends the size");
        assert_eq!(unit_lanes(&s, &id), vec![1, 2, 3, 4, 5], "plain-sized again");
    }

    #[test]
    fn r3546002_an_embiggened_grower_is_giga_and_a_normal_one_is_not() {
        let mut s = board(
            "r3546002-grower",
            json!({ "mana": 4, "hand": [GROWER] }),
            json!({}),
        );
        let id = s.card(GROWER).id.clone();
        let mut normal: Vec<i32> = Vec::new();
        let mut big: Vec<i32> = Vec::new();
        for action in legal_actions(s.state(), P1) {
            if let ActionBody::Play {
                instance_id,
                zone,
                embiggen,
                ..
            } = action
                && instance_id == id
                && let Some(zone) = zone
                && zone.row == Row::Units
            {
                if embiggen == Some(true) {
                    big.push(zone.lane);
                } else {
                    normal.push(zone.lane);
                }
            }
        }
        normal.sort();
        big.sort();
        assert_eq!(normal, vec![1, 2, 3, 4, 5], "normal-sized");
        assert_eq!(big, vec![1, 2, 3, 4], "embiggened anchors only");
        s.play(GROWER, json!({ "zone": 4, "embiggen": true }));
        let landed = live(s.state(), &s.unit(P1, 4).expect("no grower in lane 4").id);
        assert_eq!(landed.giga, Some(true), "embiggened lands GIGA");
    }

    #[test]
    fn r3546002_vanilla_and_a_grant_on_the_field_change_no_size() {
        let mut s = board(
            "r3546002-fixed",
            json!({ "field": [field_unit(GIGA_UNIT, 1), field_unit(PLAIN, 4)] }),
            json!({}),
        );
        let giga = s.unit(P1, 1).expect("no GIGA in lane 1").id.clone();
        let plain = s.unit(P1, 4).expect("no plain in lane 4").id.clone();
        run_effect(s.state_mut(), vanilla(json_as(json!({ "instanceId": giga }))), P1);
        assert_eq!(live(s.state(), &giga).giga, Some(true), "Vanilla keeps the size");
        run_effect(
            s.state_mut(),
            grant_keyword(json_as(
                json!({ "target": { "of": "instance", "instanceId": plain }, "keyword": { "kind": "GIGA" } }),
            )),
            P1,
        );
        let granted = live(s.state(), &plain);
        assert!(
            granted.granted_keywords.contains(&Keyword::Giga),
            "the grant lands"
        );
        assert_eq!(granted.giga, None, "but a field grant changes no size");
    }
}

mod r3546003_moves_whole {
    use super::*;

    #[test]
    fn r3546003_a_rotation_carries_a_giga_whole_or_bounces_it() {
        let mut s = board(
            "r3546003-rotation",
            json!({ "field": [field_unit(GIGA_UNIT, 1), field_unit(GIGA_UNIT, 4)] }),
            json!({}),
        );
        let near = s.unit(P1, 1).expect("no GIGA in lane 1").id.clone();
        let far = s.unit(P1, 4).expect("no GIGA in lane 4").id.clone();
        rotate(s.state_mut(), "right");
        let state = s.state();
        assert_eq!(
            where_is(state, &live(state, &near)),
            "p1 units 2",
            "carried whole"
        );
        assert_eq!(live(state, &near).giga, Some(true));
        let bounced = live(state, &far);
        assert_eq!(bounced.zone.z(), ZoneName::Hand, "no whole landing, so home");
        assert!(!is_empty(state, slot(P1, Row::Units, 2)));
        assert!(!is_empty(state, slot(P1, Row::Units, 3)));
    }

    #[test]
    fn r3546003_a_swap_carries_a_giga_whole() {
        let mut s = board(
            "r3546003-swap",
            json!({ "field": [field_unit(GIGA_UNIT, 1)] }),
            json!({}),
        );
        let id = s.unit(P1, 1).expect("no GIGA in lane 1").id.clone();
        run_effect(s.state_mut(), swap_board(), P1);
        let state = s.state();
        let moved = live(state, &id);
        assert_eq!(where_is(state, &moved), "p2 units 1");
        assert_eq!(moved.controller, P2);
        assert_eq!(moved.giga, Some(true));
        assert!(!is_empty(state, slot(P2, Row::Units, 2)));
    }

    #[test]
    fn r3546003_a_stolen_giga_needs_four_free_zones() {
        let mut s = board(
            "r3546003-steal",
            json!({}),
            json!({ "field": [field_unit(GIGA_UNIT, 1)] }),
        );
        let id = s.unit(P2, 1).expect("no GIGA in lane 1").id.clone();
        run_effect(s.state_mut(), steal(json_as(json!({ "instanceId": id }))), P1);
        assert_eq!(where_is(s.state(), &live(s.state(), &id)), "p1 units 1");

        let mut s = board(
            "r3546003-steal-blocked",
            json!({ "field": [field_unit(PLAIN, 1), field_unit(PLAIN, 3), field_unit(PLAIN, 5)] }),
            json!({ "field": [field_unit(GIGA_UNIT, 1)] }),
        );
        let id = s.unit(P2, 1).expect("no GIGA in lane 1").id.clone();
        run_effect(s.state_mut(), steal(json_as(json!({ "instanceId": id }))), P1);
        let state = s.state();
        let stayed = live(state, &id);
        assert_eq!(where_is(state, &stayed), "p2 units 1", "no fit, so no steal");
        assert_eq!(stayed.controller, P2);
    }

    #[test]
    fn r3546003_a_reborn_giga_returns_into_its_four_zones() {
        let mut s = board(
            "r3546003-reborn",
            json!({ "field": [field_unit(REBORN_GIGA, 1)] }),
            json!({}),
        );
        let id = s.unit(P1, 1).expect("no Reborn GIGA in lane 1").id.clone();
        let state = s.state_mut();
        lethal_damage(state, &id);
        let mut bench = Bench::new(&*state);
        state_check(&mut bench.sink(state));
        let state = s.state();
        let back = live(state, &id);
        assert_eq!(where_is(state, &back), "p1 units 1");
        assert_eq!(back.giga, Some(true), "returns at its size");
        assert_eq!(back.reborn_spent, Some(true));
        assert_eq!(back.damage, 0, "a 1/1 returns at 1 health");
        assert!(!is_empty(state, slot(P1, Row::Units, 2)), "its cover holds");
        assert!(!is_empty(state, slot(P1, Row::Backrow, 1)), "its cover holds");
    }
}

mod r3546004_anchor_view {
    use super::*;

    #[test]
    fn r3546004_the_view_draws_a_giga_at_its_anchor() {
        let s = board(
            "r3546004-unit",
            json!({ "field": [field_unit(GIGA_UNIT, 1)] }),
            json!({}),
        );
        let view = view_for(s.state(), P2);
        assert_eq!(
            view.opponent.units[0].as_ref().expect("no anchor unit").giga,
            Some(true),
            "the flag rides the anchor"
        );
        assert!(view.opponent.units[1].is_none(), "covered units read empty");
        assert!(view.opponent.backrow[0].is_none(), "covered backrow reads empty");
        assert!(view.opponent.backrow[1].is_none(), "covered backrow reads empty");

        let s = board(
            "r3546004-facedown",
            json!({ "field": [backrow_card(GIGA_TRAP, 1)] }),
            json!({}),
        );
        let view = view_for(s.state(), P2);
        match &view.opponent.backrow[0] {
            Some(BackrowView::FaceDown(face)) => assert_eq!(
                face.giga,
                Some(true),
                "the size is public on a face-down card too"
            ),
            other => panic!("no face-down card at the anchor: {other:?}"),
        }
        assert!(view.opponent.backrow[1].is_none(), "covered backrow reads empty");
        assert!(view.opponent.units[0].is_none(), "covered units read empty");
        assert!(view.opponent.units[1].is_none(), "covered units read empty");
    }
}
