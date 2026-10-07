//! The Lock variants and Unlock (docs/classic-sets.md B5 E20; SPEC §3.2 Lock): a whole lane (Classic #71
//! Lane Eater), the zone a permanent was just played into (Classic #84 Lockdown, Classic+ #34 Memory
//! Leak), a random zone not already Locked (Classic+ #34), the firing trap's own zone (Classic+ #1 Doom
//! Shroom), and Unlock (Classic+ #77 Anti-Softlock) — each played through `reduce`, replayed, and read
//! from both seats. Fixtures: `fixtures/field.ts`.
//!
//! Port of `packages/engine/test/effects-locks.test.ts`.

use jackioh_engine::effects::counters::{lock, unlock};
use jackioh_engine::effects::locks::{
    lock_lane, lock_own_zone, lock_played_zone, lock_random_zone, unlock_all,
};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::field::{
    act_result, banner, doom, eater, flush, leak, lockdown, notes_of, playing, unlocker,
};
use crate::rules::fixtures::harness::{events_of_type, in_hand, put, slot};

/// What TS's `play` returns: the state after the play, its events, and the played card's id.
struct Played {
    state: GameState,
    events: Vec<GameEvent>,
    id: String,
}

/// TS `play(state, defId, zone?, { radiant })`: a card of `defId` put in p1's hand (Radiant when asked),
/// p1's mana flushed, and the card played through `reduce` into `zone` when one is named.
fn play(state: &mut GameState, def_id: &str, zone: Option<(Row, i32)>, radiant: bool) -> Played {
    let Some(card) = in_hand(state, def_id, PlayerId::P1, 1).into_iter().next() else {
        panic!("no card");
    };
    if radiant {
        find_instance_mut(state, &card.id)
            .expect("the card is in the hand")
            .radiant = true;
    }
    flush(state, PlayerId::P1, 10);
    let mut body = json!({ "type": "play", "instanceId": card.id, "playerId": "p1" });
    if let Some((row, lane)) = zone {
        body["zone"] = json!({ "row": row, "lane": lane });
    }
    let result = act_result(state, body);
    if let Some(error) = result.error {
        panic!("{error}");
    }
    Played {
        state: result.state,
        events: result.events,
        id: card.id,
    }
}

fn locked_zones(state: &GameState) -> Vec<String> {
    let mut zones = Vec::new();
    for player in [PlayerId::P1, PlayerId::P2] {
        for row in [Row::Units, Row::Backrow] {
            for zone in slots_of(player, row) {
                if is_locked(state, zone) {
                    zones.push(format!("{}:{}:{}", player.as_str(), row.as_str(), zone.lane));
                }
            }
        }
    }
    zones
}

/// TS `sinkFor(state)`: the events and rng of a sink over `state`, the rng starting at the state's
/// cursor as reduce does. The state is lent to it call by call (`on`).
struct Sink {
    events: Vec<GameEvent>,
    rng: Rng,
}

fn sink_for(state: &GameState) -> Sink {
    Sink {
        events: Vec::new(),
        rng: Rng::new(&state.seed, state.rng_cursor),
    }
}

impl Sink {
    fn on<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }

    /// `effect.apply(makeContext(sink, self, { controller: "p1" }))`.
    fn apply_as_p1(&mut self, state: &mut GameState, effect: Effect, self_: Option<&CardInstance>) {
        let mut engine = self.on(state);
        let mut ctx = make_context(
            &mut engine,
            self_,
            HookOptions {
                controller: Some(PlayerId::P1),
                ..Default::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
}

/// `cardAt(state, ref)?.id`.
fn id_at(state: &GameState, at: ZoneSlot) -> Option<String> {
    card_at(state, at).map(|card| card.id.clone())
}

fn pluck<T: serde::Serialize>(events: &T, key: &str) -> Vec<Value> {
    match serde_json::to_value(events).expect("events serialise") {
        Value::Array(items) => items
            .into_iter()
            .map(|item| item.get(key).cloned().unwrap_or(Value::Null))
            .collect(),
        other => panic!("expected a list of events, got {other}"),
    }
}

fn lane_zone(row: &str, lane: i32) -> Value {
    json!({ "zone": { "of": "lane", "row": row, "lane": lane } })
}

mod b5_e20_lock_a_lane {
    use super::*;

    #[test]
    fn lane_eater_s_cry_locks_the_four_zones_of_its_lane_itself_staying_in_its_locked_zone() {
        let mut state = playing("lock-lane");
        let out = play(&mut state, &eater.id, Some((Row::Units, 3)), false);
        assert_eq!(
            locked_zones(&out.state),
            vec!["p1:units:3", "p1:backrow:3", "p2:units:3", "p2:backrow:3"]
        );
        assert_eq!(
            id_at(&out.state, slot(PlayerId::P1, Row::Units, 3)),
            Some(out.id.clone())
        );
        assert_eq!(events_of_type(&out.events, GameEventType::Locked).len(), 4);
        // Both seats read the Locks (public, §10.8).
        for viewer in [PlayerId::P1, PlayerId::P2] {
            let view = view_for(&out.state, viewer);
            assert!(view.you.locks.units[2]);
            assert_eq!(events_of_type(&view.events, GameEventType::Locked).len(), 4);
        }
    }

    #[test]
    fn the_radiant_face_locks_only_the_enemy_side_of_the_lane_a_zone_locked_already_emits_nothing_more() {
        let mut state = playing("lock-lane-radiant");
        state.players[PlayerId::P2].locks.backrow[1] = true;
        let out = play(&mut state, &eater.id, Some((Row::Units, 2)), true);
        assert_eq!(locked_zones(&out.state), vec!["p2:units:2", "p2:backrow:2"]);
        assert_eq!(
            serde_json::to_value(events_of_type(&out.events, GameEventType::Locked))
                .expect("events serialise"),
            json!([{ "type": "locked", "player": "p2", "row": "units", "lane": 2 }])
        );
    }

    #[test]
    fn a_numbered_lane_and_a_scope_narrowed_to_one_row() {
        let mut state = playing("lock-lane-number");
        let mut sink = sink_for(&state);
        let mut engine = sink.on(&mut state);
        let mut ctx = make_context(
            &mut engine,
            None,
            HookOptions {
                controller: Some(PlayerId::P1),
                ..Default::default()
            },
        );
        (lock_lane(json_as(json!({ "lane": 5, "rows": ["backrow"] }))).apply)(&mut ctx);
        assert_eq!(locked_zones(&*ctx.state), vec!["p1:backrow:5", "p2:backrow:5"]);
        (lock_lane(json_as(json!({ "lane": 9 }))).apply)(&mut ctx);
        assert_eq!(locked_zones(&*ctx.state), vec!["p1:backrow:5", "p2:backrow:5"]);
    }
}

mod b5_e20_lock_the_zone_a_permanent_was_just_played_into {
    use super::*;

    #[test]
    fn lockdown_locks_the_zone_of_each_permanent_played_either_player_s_and_never_a_spell_s() {
        let mut state = playing("lock-played");
        put(
            &mut state,
            &lockdown.id,
            slot(PlayerId::P2, Row::Backrow, 1),
            json!({}),
        );
        let unit = play(&mut state, &plain.id, Some((Row::Units, 4)), false);
        let mut state = unit.state;
        assert_eq!(locked_zones(&state), vec!["p1:units:4"]);
        assert_eq!(
            id_at(&state, slot(PlayerId::P1, Row::Units, 4)),
            Some(unit.id.clone())
        );
        // A Locked zone takes no further play.
        let Some(next) = in_hand(&mut state, &plain.id, PlayerId::P1, 1).into_iter().next() else {
            return;
        };
        assert!(
            !legal_zones_for(&state, PlayerId::P1, &next, &[]).contains(&ZoneChoice {
                row: Row::Units,
                lane: 4
            })
        );
        let spell = play(&mut state, &leak.id, None, false);
        let ours: Vec<String> = locked_zones(&spell.state)
            .into_iter()
            .filter(|zone| zone.starts_with("p1"))
            .collect();
        assert_eq!(ours, vec!["p1:units:4"]);
    }

    #[test]
    fn reads_a_summoned_event_s_zone_and_locks_nothing_for_a_card_already_gone_from_where_it_landed() {
        let mut state = playing("lock-played-events");
        let mut sink = sink_for(&state);
        let summoned: GameEvent = json_as(json!({
            "type": "summoned", "player": "p2", "instanceId": "c999", "defId": plain.id, "row": "backrow", "lane": 2,
        }));
        sink.apply_as_p1(
            &mut state,
            lock_played_zone(json_as(json!({ "event": summoned }))),
            None,
        );
        assert_eq!(locked_zones(&state), vec!["p2:backrow:2"]);
        let gone: GameEvent = json_as(json!({
            "type": "cardPlayed", "player": "p1", "instanceId": "c998", "defId": plain.id, "costPaid": 1,
        }));
        sink.apply_as_p1(
            &mut state,
            lock_played_zone(json_as(json!({ "event": gone }))),
            None,
        );
        assert_eq!(locked_zones(&state), vec!["p2:backrow:2"]);
    }
}

mod b5_e20_lock_a_random_zone {
    use super::*;

    #[test]
    fn locks_one_of_the_opponent_s_zones_not_locked_already_from_the_match_rng_and_replays_to_the_same_zone()
    {
        let run = |seed: &str| -> (Vec<String>, String) {
            let mut state = playing(seed);
            for lane in 1..=5 {
                state.players[PlayerId::P2].locks.units[lane - 1] = true;
            }
            state.players[PlayerId::P2].locks.backrow[0] = true;
            let out = play(&mut state, &leak.id, None, false);
            (locked_zones(&out.state), hash_state(&out.state))
        };
        let live = run("lock-random");
        assert_eq!(live.0.len(), 7);
        let added: Vec<&String> = live
            .0
            .iter()
            .filter(|zone| zone.starts_with("p2:backrow") && zone.as_str() != "p2:backrow:1")
            .collect();
        assert_eq!(added.len(), 1);
        assert_eq!(run("lock-random"), live);
    }

    #[test]
    fn locks_nothing_and_draws_nothing_when_every_zone_of_the_scope_is_locked() {
        let mut state = playing("lock-random-none");
        for lane in 1..=5 {
            state.players[PlayerId::P2].locks.units[lane - 1] = true;
            state.players[PlayerId::P2].locks.backrow[lane - 1] = true;
        }
        let mut sink = sink_for(&state);
        let cursor = sink.rng.cursor();
        sink.apply_as_p1(
            &mut state,
            lock_random_zone(json_as(json!({ "side": "enemy" }))),
            None,
        );
        assert_eq!(sink.rng.cursor(), cursor);
        assert_eq!(sink.events, Vec::<GameEvent>::new());
    }

    #[test]
    fn an_occupied_zone_is_a_fair_pick_a_lock_evicts_nothing() {
        let mut state = playing("lock-random-occupied");
        for lane in 1..=5 {
            state.players[PlayerId::P2].locks.backrow[lane - 1] = true;
        }
        for lane in 2..=5 {
            state.players[PlayerId::P2].locks.units[lane - 1] = true;
        }
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let mut sink = sink_for(&state);
        sink.apply_as_p1(
            &mut state,
            lock_random_zone(json_as(json!({ "side": "enemy" }))),
            None,
        );
        assert!(is_locked(&state, slot(PlayerId::P2, Row::Units, 1)));
        assert_eq!(
            id_at(&state, slot(PlayerId::P2, Row::Units, 1)),
            Some(unit.id.clone())
        );
    }
}

mod b5_e20_lock_the_firing_trap_s_own_zone {
    use super::*;

    #[test]
    fn doom_shroom_fires_locks_its_own_zone_and_is_consumed_the_zone_staying_locked_behind_it() {
        let mut state = playing("lock-own");
        let trap = put(
            &mut state,
            &doom.id,
            slot(PlayerId::P2, Row::Backrow, 4),
            json!({}),
        );
        let out = play(&mut state, &plain.id, Some((Row::Units, 1)), false);
        assert!(is_locked(&out.state, slot(PlayerId::P2, Row::Backrow, 4)));
        assert_eq!(id_at(&out.state, slot(PlayerId::P2, Row::Backrow, 4)), None);
        let consumed = find_instance(&out.state, &trap.id);
        assert_eq!(consumed.map(|card| card.zone.z()), Some(ZoneName::Graveyard));
        assert_eq!(notes_of(consumed), Vec::<String>::new());
        assert_eq!(
            pluck(
                &events_of_type(&out.events, GameEventType::TrapFired),
                "instanceId"
            ),
            vec![json!(trap.id)]
        );
    }

    #[test]
    fn a_card_off_the_field_locks_nothing() {
        let mut state = playing("lock-own-gone");
        let mut card = put(
            &mut state,
            &banner.id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        state.players[PlayerId::P1].backrow[0] = None;
        card.zone = Zone::Graveyard { player: PlayerId::P1 };
        let mut sink = sink_for(&state);
        sink.apply_as_p1(&mut state, lock_own_zone(), Some(&card));
        assert_eq!(locked_zones(&state), Vec::<String>::new());
    }
}

mod b5_e20_unlock {
    use super::*;

    #[test]
    fn r68_unlock_every_zone_opens_each_locked_zone_once_in_r68_s_walk_and_the_zones_take_plays_again() {
        let mut state = playing("unlock-all");
        state.players[PlayerId::P1].locks.units[1] = true;
        state.players[PlayerId::P2].locks.backrow[4] = true;
        let mut out = play(&mut state, &unlocker.id, None, false);
        assert_eq!(locked_zones(&out.state), Vec::<String>::new());
        assert_eq!(
            serde_json::to_value(events_of_type(&out.events, GameEventType::Unlocked))
                .expect("events serialise"),
            json!([
                { "type": "unlocked", "player": "p1", "row": "units", "lane": 2 },
                { "type": "unlocked", "player": "p2", "row": "backrow", "lane": 5 },
            ])
        );
        for viewer in [PlayerId::P1, PlayerId::P2] {
            assert_eq!(
                events_of_type(&view_for(&out.state, viewer).events, GameEventType::Unlocked).len(),
                2
            );
        }
        let Some(next) = in_hand(&mut out.state, &plain.id, PlayerId::P1, 1)
            .into_iter()
            .next()
        else {
            return;
        };
        assert!(
            legal_zones_for(&out.state, PlayerId::P1, &next, &[]).contains(&ZoneChoice {
                row: Row::Units,
                lane: 2
            })
        );
    }

    #[test]
    fn the_single_unlock_opens_one_zone_an_open_zone_is_left_as_it_is_with_no_event() {
        let mut state = playing("unlock-one");
        let mut sink = sink_for(&state);
        {
            let mut engine = sink.on(&mut state);
            let mut ctx = make_context(
                &mut engine,
                None,
                HookOptions {
                    controller: Some(PlayerId::P1),
                    ..Default::default()
                },
            );
            (lock(json_as(lane_zone("units", 3))).apply)(&mut ctx);
            (unlock(json_as(lane_zone("units", 3))).apply)(&mut ctx);
            (unlock(json_as(lane_zone("units", 3))).apply)(&mut ctx);
            (unlock_all(json_as(json!({ "side": "enemy" }))).apply)(&mut ctx);
        }
        assert_eq!(locked_zones(&state), Vec::<String>::new());
        assert_eq!(
            sink.events.iter().map(GameEvent::event_type).collect::<Vec<_>>(),
            vec![GameEventType::Locked, GameEventType::Unlocked]
        );
    }
}
