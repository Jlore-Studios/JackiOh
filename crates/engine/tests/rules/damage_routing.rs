//! Damage routing (issue #3554, R3554005–R3554009): line damage, chain lightning, damage
//! chains, Fuse's lane-weighted attack and summoning onto an occupied zone, proved on the fixtures
//! of `fixtures/board_geometry.rs`, with no card yet.

use jackioh_engine::effects;
use jackioh_engine::subsystems::ai_policy::policy_actions;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;
use jackioh_engine::view_for::view_for;

use super::fixtures::board_geometry::*;
use super::fixtures::harness::{live, run_effect, slot};

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

/// The damage on the card `id` now.
fn damage_on(state: &GameState, id: &str) -> i32 {
    find_instance(state, id).map_or(0, |card| card.damage)
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

/// Where a card sits now, as "p2 units 5", for readable assertions.
fn where_is(state: &GameState, id: &str) -> String {
    match find_instance(state, id).map(|found| &found.zone) {
        None => "gone".to_string(),
        Some(Zone::Field { player, row, lane }) => format!("{player} {row} {lane}"),
        Some(zone) => zone.z().to_string(),
    }
}

/// Answer the open tile Discover with the cell (x, y) from the caster's seat: the offer is the
/// match rng's, so the wanted cell is put in the offer in place of the first cell offered.
fn pick_tile(s: &mut Scenario, x: i32, y: i32) {
    let wanted = format!("{x},{y}");
    let state = s.state_mut();
    let pending = state.pending.as_mut().expect("a tile Discover");
    if !pending.options.iter().any(|option| option.key == wanted) {
        pending.options[0] = PromptOption {
            key: wanted.clone(),
            label: format!("Lane {}, forced", x + 1),
            selection: Selection::Mode {
                option: wanted.clone(),
            },
            cost: None,
            radiant: None,
        };
    }
    s.answer(json!([wanted]));
}

mod r3554005_line_damage {
    use super::*;

    #[test]
    fn r3554005_line_hits_the_lattice_cells_with_pierce() {
        let mut s = board(
            "r3554005-lattice",
            json!({ "hand": [LANCER] }),
            json!({
                "field": [
                    field_unit(ARMORED, 3),
                    backrow_card(LANCER, 5),
                ],
            }),
        );
        s.play(LANCER, json!({ "zone": 1 }));
        // From lane 1 to the enemy backrow at lane 5: through lane 3, missing lane 2.
        pick_tile(&mut s, 4, 3);
        let (armored, back) = (id_at(&s, P2, 3), s.backrow(P2, 5).expect("the backrow lancer").id);
        assert_eq!(damage_on(s.state(), &armored), 3, "Pierce went through Armor");
        assert_eq!(damage_on(s.state(), &back), 3);
        // Both views get `lineDrawn`: positions only.
        for viewer in [P1, P2] {
            assert!(
                view_for(s.state(), viewer).events.iter().any(|event| matches!(
                    event,
                    GameEvent::LineDrawn { cells, .. } if cells.len() == 3
                )),
                "{viewer:?} saw the line"
            );
        }
    }

    #[test]
    fn r3554005_vertical_line() {
        let mut s = board(
            "r3554005-vertical",
            json!({ "hand": [LANCER] }),
            json!({ "field": [field_unit(BULWARK, 3), field_unit(PLAIN, 2)] }),
        );
        s.play(LANCER, json!({ "zone": 1 }));
        // Straight ahead at the enemy Units at lane 3: only that cell is on the line.
        pick_tile(&mut s, 2, 2);
        assert_eq!(damage_on(s.state(), &id_at(&s, P2, 3)), 3);
        assert_eq!(damage_on(s.state(), &id_at(&s, P2, 2)), 0);
    }

    #[test]
    fn r3554005_lancer_discovers_three_and_each_is_answerable() {
        let mut s = board("r3554005-offers", json!({ "hand": [LANCER] }), json!({}));
        s.play(LANCER, json!({ "zone": 1 }));
        let pending = s.state().pending.clone().expect("the tile Discover");
        assert_eq!(pending.options.len(), DISCOVER_OPTIONS as usize);
        // Every option is a legal answer, and the AI policy answers the Discover too.
        assert_eq!(
            legal_actions(s.state(), P1)
                .into_iter()
                .filter(|action| matches!(action, ActionBody::Answer { .. }))
                .count(),
            DISCOVER_OPTIONS as usize
        );
        assert!(
            policy_actions(s.state(), P1, Default::default())
                .iter()
                .any(|action| matches!(action, ActionBody::Answer { .. })),
            "the AI policy answers the Discover"
        );
        let timed_out = reduce(s.state(), &Action::new(ActionBody::Timeout, P1, "srv-t"));
        assert_eq!(timed_out.error, None);
        assert!(timed_out.state.pending.is_none());
    }

    #[test]
    fn r3554005_cleave_hits_each_neighbour_once() {
        let mut s = board(
            "r3554005-cleave",
            json!({ "hand": [VELA] }),
            json!({
                "field": [
                    field_unit(BULWARK, 1),
                    field_unit(GIGA, 2),
                ],
            }),
        );
        s.play(VELA, json!({ "zone": 1 }));
        // At the enemy Units at lane 1: the line hits it, the Cleave its GIGA neighbour once.
        pick_tile(&mut s, 0, 2);
        let (first, giga) = (id_at(&s, P2, 1), id_at(&s, P2, 2));
        assert_eq!(dealt_to(s.last_events(), &first), vec![3]);
        assert_eq!(dealt_to(s.last_events(), &giga), vec![3], "one Cleave hit");
    }
}

mod r3554006_chain_lightning {
    use super::*;

    #[test]
    fn r3554006_chain_lightning_hits_each_adjacent_unit_once() {
        let mut s = board(
            "r3554006-chain",
            json!({}),
            json!({
                "field": [
                    field_unit(PLAIN, 1),
                    field_unit(PLAIN, 2),
                    field_unit(PLAIN, 3),
                    field_unit(PLAIN, 5),
                    backrow_card(LANCER, 2),
                ],
            }),
        );
        let target = id_at(&s, P2, 2);
        run_effect(
            s.state_mut(),
            effects::chain_lightning(json_as(json!({
                "to": { "of": "instance", "instanceId": target },
                "amount": 2,
            }))),
            P1,
        );
        // The lane-1, lane-3 and backrow neighbours all took 2; the far lane none.
        for lane in [1, 2, 3] {
            assert_eq!(
                damage_on(s.state(), &id_at(&s, P2, lane)),
                2,
                "lane {lane} took 2"
            );
        }
        assert_eq!(
            damage_on(s.state(), &s.backrow(P2, 2).expect("the standing lancer").id),
            2,
            "a Unit standing in the backrow is hit"
        );
        assert_eq!(
            damage_on(s.state(), &id_at(&s, P2, 5)),
            0,
            "the far lane is not hit"
        );
    }

    #[test]
    fn r3554006_arc_cries_chain_lightning() {
        let mut s = board(
            "r3554006-arc",
            json!({ "hand": [ARC] }),
            json!({ "field": [field_unit(PLAIN, 2)] }),
        );
        let target = id_at(&s, P2, 2);
        s.play(
            ARC,
            json!({ "targets": [{ "pick": "instance", "instanceId": target }] }),
        );
        assert_eq!(damage_on(s.state(), &target), 2);
    }
}

mod r3554007_damage_chains {
    use super::*;

    fn chain(s: &mut Scenario, side: &str, kind: &str) {
        run_effect(
            s.state_mut(),
            effects::chain_units(json_as(json!({ "side": side, "kind": kind }))),
            P1,
        );
    }

    fn hit(s: &mut Scenario, id: &str, amount: i32) -> Vec<GameEvent> {
        run_effect(
            s.state_mut(),
            effects::damage(json_as(json!({
                "to": { "of": "instance", "instanceId": id },
                "amount": amount,
            }))),
            P1,
        )
    }

    #[test]
    fn r3554007_shared_chain_echoes_what_was_dealt() {
        let mut s = board(
            "r3554007-shared",
            json!({}),
            json!({ "field": [field_unit(ARMORED, 1), field_unit(PLAIN, 2)] }),
        );
        chain(&mut s, "enemy", "shared");
        let (armored, plain) = (id_at(&s, P2, 1), id_at(&s, P2, 2));
        // 5 past Armor 2 is 3 dealt — and 3 echoed.
        let events = hit(&mut s, &armored, 5);
        assert_eq!(dealt_to(&events, &armored), vec![3]);
        assert_eq!(dealt_to(&events, &plain), vec![3]);
    }

    #[test]
    fn r3554007_split_chain_divides_rounded_down() {
        let mut s = board(
            "r3554007-split",
            json!({
                "field": [field_unit(BULWARK, 1), field_unit(BULWARK, 2), field_unit(BULWARK, 4)],
            }),
            json!({}),
        );
        chain(&mut s, "self", "split");
        let ids = [id_at(&s, P1, 1), id_at(&s, P1, 2), id_at(&s, P1, 4)];
        hit(&mut s, &ids[0], 7);
        // 7 among 3 is 2 each.
        for id in &ids {
            assert_eq!(damage_on(s.state(), id), 2, "{id} took its share");
        }
    }

    #[test]
    fn r3554007_later_units_and_returned_cards_are_unchained() {
        let mut s = board(
            "r3554007-later",
            json!({ "field": [field_unit(BULWARK, 1), field_unit(BULWARK, 2)] }),
            json!({}),
        );
        chain(&mut s, "self", "split");
        // A later summon is in no chain.
        run_effect(
            s.state_mut(),
            effects::summon(json_as(json!({ "defId": PLAIN, "lane": 4 }))),
            P1,
        );
        let (first, second, later) = (id_at(&s, P1, 1), id_at(&s, P1, 2), id_at(&s, P1, 4));
        hit(&mut s, &first, 6);
        assert_eq!(damage_on(s.state(), &first), 3);
        assert_eq!(damage_on(s.state(), &second), 3);
        assert_eq!(damage_on(s.state(), &later), 0);
        // Nor is a card that left and came back: bounced and replayed, it takes its hit whole.
        run_effect(
            s.state_mut(),
            effects::bounce(json_as(
                json!({ "target": { "of": "instance", "instanceId": second } }),
            )),
            P1,
        );
        assert_eq!(where_is(s.state(), &second), "hand");
        s.play(second.as_str(), json!({ "zone": 2 }));
        hit(&mut s, &second, 6);
        assert_eq!(damage_on(s.state(), &second), 6);
        assert_eq!(damage_on(s.state(), &first), 3, "no share reached back");
    }
}

mod r3554008_sweep {
    use super::*;

    #[test]
    fn r3554008_sweep_hits_lane_times_five_adjacent_times_three() {
        let mut s = board(
            "r3554008-sweep",
            json!({ "field": [field_unit(FUSE, 3)] }),
            json!({
                "field": [
                    field_unit(BULWARK, 1),
                    field_unit(BULWARK, 2),
                    field_unit(BULWARK, 3),
                    field_unit(BULWARK, 4),
                ],
            }),
        );
        let fuse = id_at(&s, P1, 3);
        let (far, near_left, mid, near_right) = (
            id_at(&s, P2, 1),
            id_at(&s, P2, 2),
            id_at(&s, P2, 3),
            id_at(&s, P2, 4),
        );
        s.attack(fuse.as_str(), mid.as_str());
        assert_eq!(damage_on(s.state(), &mid), 10, "its lane takes five times");
        assert_eq!(damage_on(s.state(), &near_left), 6, "adjacent takes three times");
        assert_eq!(damage_on(s.state(), &near_right), 6, "adjacent takes three times");
        assert_eq!(damage_on(s.state(), &far), 0, "a farther lane takes nothing");
    }
}

mod r3554009_occupied_summon {
    use super::*;

    #[test]
    fn r3554009_summon_exiles_the_pile_below_and_crater_locks() {
        let mut s = board(
            "r3554009-below",
            json!({ "field": [field_unit(PLAIN, 2)] }),
            json!({}),
        );
        let below = id_at(&s, P1, 2);
        run_effect(
            s.state_mut(),
            effects::summon(json_as(json!({
                "defId": CRATER,
                "lane": 2,
                "exileBelow": true,
            }))),
            P1,
        );
        // The pile below was exiled, the Crater stands on its zone, andLocked it.
        assert_eq!(where_is(s.state(), &below), "exile");
        let crater = s.unit(P1, 2).expect("the crater").id.clone();
        assert_eq!(live(s.state(), &crater).def_id, CRATER);
        assert!(
            is_locked(s.state(), slot(P1, Row::Units, 2)),
            "Crater locks its zone"
        );
    }
}
