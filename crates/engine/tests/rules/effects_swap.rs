//! Swap (SPEC §6.3's Swap row, R73; BUILD M3-T1): #87 Pocket Chaos's three swaps — hero health,
//! the board lane by lane, and the libraries — with R12's ownership rule and its one exception,
//! R33's face-down trap and §3.2's Stack pile. The fixture Trap this file needs is registered here,
//! so no shared fixture has to grow for it (BUILD §0).
//!
//! Port of `packages/engine/test/effects-swap.test.ts`.

use jackioh_engine::effects::{swap, swap_board, swap_health, swap_library};
use jackioh_engine::testkit::*;
use serde::Serialize;

use super::fixtures::catalog as catalog_fx;
use super::fixtures::combat as combat_fx;
use super::fixtures::harness::{PutOptions, events_of_type, new_game, put, set_library, slot};

/// A face-down Trap for R33; #87's board swap moves the backrow too.
fn trap() -> CardDef {
    catalog_fx::spell_def(
        720,
        json!({ "id": "sw-trap", "index": "720", "name": "Swap Trap (fixture)", "type": "Trap" }),
    )
}

/// The shared fixture unit token (R11).
const TOKEN_ID: &str = "fx-token-rush";

/// TS `game(seed = "swap-test")`: every test here uses the default.
fn game() -> GameState {
    let mut state = new_game("swap-test", None);
    let mut catalog = registered_catalog().clone();
    let trap = trap();
    catalog.insert(trap.id.clone(), trap);
    register_catalog(catalog);
    state.turn = 3;
    state.active = PlayerId::P1;
    state
}

/// Apply one effect the way `resolve.ts` does, and hand back the events it emitted. TS's
/// `sinkFor(state)` is the sink built here: its rng starts at the state's cursor, as reduce does.
/// TS's `{ controller: "p1", ...options }`: the options' controller, else p1.
fn run(state: &mut GameState, effect: Effect, options: HookOptions) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let options = HookOptions {
            controller: options.controller.or(Some(PlayerId::P1)),
            ..options
        };
        let mut ctx = make_context(&mut sink, None, options);
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

/// A Stack card pushed onto an occupied unit zone (§3.2); the harness's `put` fills empty zones.
fn stack_onto(state: &mut GameState, def_id: &str, player: PlayerId, lane: i32) -> CardInstance {
    let mut card = new_instance(state, def_id, player, Zone::Hand { player });
    assert!(place_on_field(
        state,
        &mut card,
        slot(player, Row::Units, lane),
        PlaceOnFieldOptions { stack: Some(true) },
    ));
    card
}

/// Where a card sits now, as "p2 units 5", for readable assertions.
fn where_is(state: &GameState, card: &CardInstance) -> String {
    match find_instance(state, &card.id).map(|found| found.zone.clone()) {
        None => "gone".to_string(),
        Some(Zone::Field { player, row, lane }) => format!("{} {} {}", player.as_str(), row.as_str(), lane),
        Some(zone) => zone.z().as_str().to_string(),
    }
}

/// TS held the live instance and read it after an effect; Rust reads the card again by id.
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id).cloned().unwrap_or_else(|| panic!("no card {id} in the state"))
}

/// TS wrote through the live instance; Rust writes through the card found by id.
fn live_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id} in the state"))
}

fn to_json<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// TS `toMatchObject`: every key `expected` names is in `actual` with a matching value (objects
/// recursively, arrays element by element); `actual` may carry more.
fn matches_object(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => expected
            .iter()
            .all(|(key, value)| actual.get(key).is_some_and(|found| matches_object(found, value))),
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual.iter().zip(expected).all(|(found, value)| matches_object(found, value))
        }
        _ => actual == expected,
    }
}

fn ids_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn control_changed_ids(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::ControlChanged { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

fn bounced_ids(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Bounced { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

mod swap_s6_3_r73_m3_t1 {
    use super::*;

    #[test]
    fn r73_swaps_hero_health_and_leaves_each_heros_armor_where_it_was() {
        let mut state = game();
        state.players.p1.hero = HeroState { health: 12, armor: 3 };
        state.players.p2.hero = HeroState { health: 27, armor: 5 };

        let events = run(&mut state, swap_health(), HookOptions::default());

        assert_eq!(state.players.p1.hero.health, 27);
        assert_eq!(state.players.p2.hero.health, 12);
        // R73: armor stays with its hero, so only the two numbers changed places.
        assert_eq!(state.players.p1.hero.armor, 3);
        assert_eq!(state.players.p2.hero.armor, 5);

        // A swap is not damage, not a heal and not "lose health" (R18): one event, and that is all.
        assert_eq!(to_json(&events), json!([{ "type": "swapped", "what": "health" }]));
    }

    #[test]
    fn r73_swaps_board_contents_lane_by_lane_in_both_rows_with_control_moving_and_nothing_left_behind() {
        let mut state = game();
        let turn = state.turn;
        let mine = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 2), PutOptions::default());
        {
            let card = live_mut(&mut state, &mine.id);
            card.damage = 1;
            card.buffs = AttackHealth { attack: 3, health: 4 };
            card.counters = Counters {
                plague: Some(2),
                grade: None,
            };
            card.position = Some(Position::Def);
            card.summoned_turn = Some(turn - 1);
            card.exertion = Exertion {
                attacked: true,
                switched: false,
                attacks: None,
            };
        }
        let my_back = put(&mut state, &trap().id, slot(PlayerId::P1, Row::Backrow, 4), PutOptions::default());
        let theirs = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P2, Row::Units, 5), PutOptions::default());
        let their_back = put(&mut state, &trap().id, slot(PlayerId::P2, Row::Backrow, 1), PutOptions::default());

        let events = run(&mut state, swap_board(), HookOptions::default());

        // Lane-preserving: every card is in the same row and lane on the other side (R73, §3.1).
        assert_eq!(where_is(&state, &mine), "p2 units 2");
        assert_eq!(where_is(&state, &my_back), "p2 backrow 4");
        assert_eq!(where_is(&state, &theirs), "p1 units 5");
        assert_eq!(where_is(&state, &their_back), "p1 backrow 1");
        assert!(card_at(&state, slot(PlayerId::P1, Row::Units, 2)).is_none());
        assert!(card_at(&state, slot(PlayerId::P2, Row::Units, 5)).is_none());

        let mine_now = live(&state, &mine.id);
        let my_back_now = live(&state, &my_back.id);
        let theirs_now = live(&state, &theirs.id);
        let their_back_now = live(&state, &their_back.id);
        // Control changes, ownership does not (R12).
        assert_eq!([mine_now.controller, my_back_now.controller], [PlayerId::P2, PlayerId::P2]);
        assert_eq!([theirs_now.controller, their_back_now.controller], [PlayerId::P1, PlayerId::P1]);
        assert_eq!(
            [mine_now.owner, my_back_now.owner, theirs_now.owner, their_back_now.owner],
            [PlayerId::P1, PlayerId::P1, PlayerId::P2, PlayerId::P2]
        );

        // Nothing left the field, so R78's reset never ran: damage, buffs, counters and position came along.
        assert_eq!(mine_now.damage, 1);
        assert_eq!(mine_now.buffs, AttackHealth { attack: 3, health: 4 });
        assert_eq!(
            mine_now.counters,
            Counters {
                plague: Some(2),
                grade: None,
            }
        );
        assert_eq!(mine_now.position, Some(Position::Def));
        // R171: summoning sickness does not come along. Changing sides is an entry on this turn, with a
        // fresh exertion for the new controller.
        assert_eq!(mine_now.summoned_turn, Some(3));
        assert_eq!(
            mine_now.exertion,
            Exertion {
                attacked: false,
                switched: false,
                attacks: None,
            }
        );

        // §10.3: one `swapped`, then a `controlChanged` per card — no new event type is needed.
        assert_eq!(
            events.iter().map(GameEvent::event_type).collect::<IndexSet<_>>(),
            IndexSet::from([GameEventType::Swapped, GameEventType::ControlChanged])
        );
        assert_eq!(
            to_json(events_of_type(&events, GameEventType::Swapped)),
            json!([{ "type": "swapped", "what": "board" }])
        );
        assert_eq!(
            to_json(events_of_type(&events, GameEventType::ControlChanged)),
            json!([
                { "type": "controlChanged", "instanceId": mine.id, "controller": "p2", "row": "units", "lane": 2 },
                { "type": "controlChanged", "instanceId": my_back.id, "controller": "p2", "row": "backrow", "lane": 4 },
                { "type": "controlChanged", "instanceId": theirs.id, "controller": "p1", "row": "units", "lane": 5 },
                { "type": "controlChanged", "instanceId": their_back.id, "controller": "p1", "row": "backrow", "lane": 1 },
            ])
        );
    }

    #[test]
    fn r12_a_swapped_card_changes_controller_but_never_owner_so_it_still_leaves_to_its_owners_zones() {
        let mut state = game();
        let card = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());

        run(&mut state, swap_board(), HookOptions::default());
        let mut now = live(&state, &card.id);
        assert_eq!(now.controller, PlayerId::P2);
        assert_eq!(now.owner, PlayerId::P1);

        // Off the field a card always belongs to its owner (R12, §3.2), and R78 hands control back.
        move_to_zone(&mut state, &mut now, OffFieldZone::Graveyard, Default::default());
        assert_eq!(ids_of(&state.players.p1.graveyard), vec![card.id.clone()]);
        assert!(state.players.p2.graveyard.is_empty());
        assert_eq!(live(&state, &card.id).controller, PlayerId::P1);
    }

    #[test]
    fn r73_swaps_uneven_boards_so_an_empty_side_simply_receives_the_other_sides_cards() {
        let mut state = game();
        let units: Vec<CardInstance> = [1, 3, 4]
            .iter()
            .map(|lane| {
                put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, *lane), PutOptions::default())
            })
            .collect();
        let back = put(&mut state, &trap().id, slot(PlayerId::P1, Row::Backrow, 5), PutOptions::default());

        let events = run(&mut state, swap_board(), HookOptions::default());

        assert_eq!(
            units.iter().map(|card| where_is(&state, card)).collect::<Vec<_>>(),
            vec!["p2 units 1", "p2 units 3", "p2 units 4"]
        );
        assert_eq!(where_is(&state, &back), "p2 backrow 5");
        assert!(state.players.p1.units.iter().all(|pile| pile.is_none()));
        assert!(state.players.p1.backrow.iter().all(|card| card.is_none()));
        assert!(units.iter().all(|card| live(&state, &card.id).controller == PlayerId::P2));
        // Nothing came back the other way, and nothing was bounced or lost.
        assert_eq!(events_of_type(&events, GameEventType::ControlChanged).len(), 4);
        assert!(events_of_type(&events, GameEventType::Bounced).is_empty());
        assert!(state.players.p1.hand.is_empty());
    }

    #[test]
    fn r73_leaves_locks_with_their_zones_so_a_lock_never_travels_with_the_card_that_sat_under_it() {
        let mut state = game();
        let card = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());
        // §3.2: the current occupant is unaffected by the lock and the lock persists after it leaves.
        lock_zone(&mut state, slot(PlayerId::P1, Row::Units, 1));

        run(&mut state, swap_board(), HookOptions::default());

        assert_eq!(where_is(&state, &card), "p2 units 1");
        // The lock stayed on the zone it was set on; the card's new zone did not become Locked (R73).
        assert_eq!(state.players.p1.locks.units, vec![true, false, false, false, false]);
        assert_eq!(state.players.p2.locks.units, vec![false, false, false, false, false]);
        assert!(state.players.p1.locks.backrow.iter().all(|locked| !*locked));
    }

    #[test]
    fn r88_a_card_whose_destination_is_locked_bounces_to_its_owners_hand_as_r14_does_for_a_rotation() {
        // R88: R73 does not say what happens to a card swapped into a Locked
        // zone; §3.2's "accepts no summons" plus R14's answer for the other whole-board move is the
        // reading implemented here.
        let mut state = game();
        let blocked = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 3), PutOptions::default());
        {
            let card = live_mut(&mut state, &blocked.id);
            card.damage = 2;
            card.buffs = AttackHealth { attack: 1, health: 1 };
        }
        let other = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 2), PutOptions::default());
        lock_zone(&mut state, slot(PlayerId::P2, Row::Units, 3));

        let events = run(&mut state, swap_board(), HookOptions::default());

        assert_eq!(where_is(&state, &blocked), "hand");
        assert!(ids_of(&state.players.p1.hand).contains(&blocked.id));
        assert!(card_at(&state, slot(PlayerId::P2, Row::Units, 3)).is_none());
        assert_eq!(
            to_json(events_of_type(&events, GameEventType::Bounced)),
            json!([{ "type": "bounced", "instanceId": blocked.id, "defId": combat_fx::plain().id, "owner": "p1" }])
        );
        // It left the field, so R78's reset ran on the way to the hand.
        let blocked_now = live(&state, &blocked.id);
        assert_eq!(blocked_now.damage, 0);
        assert_eq!(blocked_now.buffs, AttackHealth { attack: 0, health: 0 });
        assert_eq!(blocked_now.controller, PlayerId::P1);
        // The rest of the board still swapped, and the bounced card changed no control.
        assert_eq!(where_is(&state, &other), "p2 units 2");
        assert_eq!(control_changed_ids(&events), vec![other.id.clone()]);
    }

    #[test]
    fn r33_a_swapped_face_down_trap_stays_face_down_and_is_readable_by_its_new_controller_only() {
        let mut state = game();
        let hidden = put(&mut state, &trap().id, slot(PlayerId::P1, Row::Backrow, 2), PutOptions::default());
        assert_eq!(live(&state, &hidden.id).face_up, None);
        // Before the swap only p1 may read it: p2 sees a face-down marker (§3, R33).
        assert!(matches_object(
            &to_json(view_for(&state, PlayerId::P1))["you"]["backrow"][1],
            &json!({ "defId": trap().id, "faceDown": false }),
        ));
        assert_eq!(
            to_json(view_for(&state, PlayerId::P2))["opponent"]["backrow"][1],
            json!({ "faceDown": true, "cost": 1 })
        );

        let events = run(&mut state, swap_board(), HookOptions::default());

        assert_eq!(where_is(&state, &hidden), "p2 backrow 2");
        let hidden_now = live(&state, &hidden.id);
        assert_eq!(hidden_now.controller, PlayerId::P2);
        assert_eq!(hidden_now.owner, PlayerId::P1);
        // The swap never flips the card: `controller` is what decides who may read it (R33).
        assert_eq!(hidden_now.face_up, None);
        assert!(matches_object(
            &to_json(view_for(&state, PlayerId::P2))["you"]["backrow"][1],
            &json!({ "defId": trap().id, "faceDown": false }),
        ));
        assert_eq!(
            to_json(view_for(&state, PlayerId::P1))["opponent"]["backrow"][1],
            json!({ "faceDown": true, "cost": 1 })
        );
        assert!(
            !serde_json::to_string(&view_for(&state, PlayerId::P1))
                .expect("serialises")
                .contains(&trap().id)
        );
        assert_eq!(
            to_json(events_of_type(&events, GameEventType::ControlChanged)),
            json!([
                { "type": "controlChanged", "instanceId": hidden.id, "controller": "p2", "row": "backrow", "lane": 2 },
            ])
        );
    }

    #[test]
    fn s3_2_a_stack_pile_swaps_whole_and_keeps_the_same_card_on_top() {
        let mut state = game();
        let under = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 5), PutOptions::default());
        live_mut(&mut state, &under.id).damage = 1;
        let top = stack_onto(&mut state, &combat_fx::stacker().id, PlayerId::P1, 5);

        let events = run(&mut state, swap_board(), HookOptions::default());

        assert!(pile_at(&state, slot(PlayerId::P1, Row::Units, 5)).is_none());
        assert_eq!(
            pile_at(&state, slot(PlayerId::P2, Row::Units, 5)).map(|pile| ids_of(pile)),
            Some(vec![top.id.clone(), under.id.clone()])
        );
        assert_eq!(
            card_at(&state, slot(PlayerId::P2, Row::Units, 5)).map(|card| card.id.clone()),
            Some(top.id.clone())
        );
        // The dormant card came along, kept its damage and changed control with the zone (§3.2, R73).
        assert_eq!(live(&state, &under.id).damage, 1);
        assert_eq!(live(&state, &top.id).controller, PlayerId::P2);
        assert_eq!(live(&state, &under.id).controller, PlayerId::P2);
        assert_eq!(control_changed_ids(&events), vec![top.id.clone(), under.id.clone()]);
        // Nothing beneath the top resumed, so no Stack note is kept against it (R212, `withPile`).
        assert!(
            state
                .field_exits
                .as_ref()
                .and_then(|exits| exits.uncovered.as_ref())
                .and_then(|uncovered| uncovered.get(&top.id))
                .is_none()
        );
    }

    #[test]
    fn r11_a_unit_token_that_cannot_land_ceases_to_exist_instead_of_reaching_a_hand() {
        let mut state = game();
        let token = put(&mut state, TOKEN_ID, slot(PlayerId::P1, Row::Units, 4), PutOptions::default());
        lock_zone(&mut state, slot(PlayerId::P2, Row::Units, 4));

        let events = run(&mut state, swap_board(), HookOptions::default());

        assert_eq!(bounced_ids(&events), vec![token.id.clone()]);
        assert!(find_instance(&state, &token.id).is_none());
        assert!(state.players.p1.hand.is_empty());
        assert!(state.players.p1.graveyard.is_empty());
        assert!(state.players.p2.graveyard.is_empty());
    }

    #[test]
    fn r73_swaps_libraries_whole_and_r12s_exception_gives_each_swapped_card_its_new_holder_as_owner() {
        let mut state = game();
        let plain_id = combat_fx::plain().id;
        let stacker_id = combat_fx::stacker().id;
        let trap_id = trap().id;
        let mine = set_library(&mut state, PlayerId::P1, &[plain_id.as_str(), stacker_id.as_str(), plain_id.as_str()]);
        let theirs = set_library(&mut state, PlayerId::P2, &[trap_id.as_str(), TOKEN_ID]);
        state.players.p1.fatigue_count = 2;
        state.players.p2.fatigue_count = 0;

        let events = run(&mut state, swap_library(), HookOptions::default());

        // The piles changed places whole and kept their order, so each top card is still the next draw.
        assert_eq!(ids_of(&state.players.p1.library), ids_of(&theirs));
        assert_eq!(ids_of(&state.players.p2.library), ids_of(&mine));

        // R12's one exception (R73): the owner becomes the player whose library now holds the card.
        let theirs_now: Vec<CardInstance> = theirs.iter().map(|card| live(&state, &card.id)).collect();
        let mine_now: Vec<CardInstance> = mine.iter().map(|card| live(&state, &card.id)).collect();
        assert!(theirs_now.iter().all(|card| card.owner == PlayerId::P1 && card.controller == PlayerId::P1));
        assert!(mine_now.iter().all(|card| card.owner == PlayerId::P2 && card.controller == PlayerId::P2));
        assert_eq!(
            theirs_now.iter().map(|card| card.zone.clone()).collect::<Vec<_>>(),
            vec![Zone::Library { player: PlayerId::P1 }; theirs.len()]
        );
        assert_eq!(
            mine_now.iter().map(|card| card.zone.clone()).collect::<Vec<_>>(),
            vec![Zone::Library { player: PlayerId::P2 }; mine.len()]
        );

        // R73: fatigue is the player's, not the library's, so the counters stay where they were.
        assert_eq!(state.players.p1.fatigue_count, 2);
        assert_eq!(state.players.p2.fatigue_count, 0);

        // R11 (decision): a unit-token card never left a library, so it is still there, now p1's.
        assert_eq!(
            find_instance(&state, &theirs[1].id).map(|card| card.zone.clone()),
            Some(Zone::Library { player: PlayerId::P1 })
        );

        assert_eq!(to_json(&events), json!([{ "type": "swapped", "what": "library" }]));
    }

    #[test]
    fn s6_3_swap_reads_the_choose_one_answer_and_swaps_nothing_when_the_answer_names_none_of_the_three() {
        let mut state = game();
        state.players.p1.hero.health = 10;
        state.players.p2.hero.health = 30;

        // R81: a play-time mode arrives in `ctx.modes` …
        assert_eq!(
            to_json(run(
                &mut state,
                swap(json_as(json!({}))),
                HookOptions {
                    modes: Some(vec!["health".to_string()]),
                    ..HookOptions::default()
                },
            )),
            json!([{ "type": "swapped", "what": "health" }])
        );
        assert_eq!(state.players.p1.hero.health, 30);

        // … and an answered "Choose one" prompt arrives in `ctx.targets` as a mode selection (§10.6).
        let answered = run(
            &mut state,
            swap(json_as(json!({}))),
            HookOptions {
                targets: Some(vec![Selection::Mode {
                    option: "health".to_string(),
                }]),
                ..HookOptions::default()
            },
        );
        assert_eq!(to_json(&answered), json!([{ "type": "swapped", "what": "health" }]));
        assert_eq!(state.players.p1.hero.health, 10);

        // No answer, or one naming something else, fizzles: nothing swaps and nothing is emitted.
        assert!(run(&mut state, swap(json_as(json!({}))), HookOptions::default()).is_empty());
        assert!(
            run(
                &mut state,
                swap(json_as(json!({}))),
                HookOptions {
                    modes: Some(vec!["mana".to_string()]),
                    ..HookOptions::default()
                },
            )
            .is_empty()
        );
        assert_eq!(
            to_json(run(&mut state, swap(json_as(json!({ "what": "board" }))), HookOptions::default())),
            json!([{ "type": "swapped", "what": "board" }])
        );
        assert_eq!(state.players.p1.hero.health, 10);
    }
}
