//! Plague Counters on an instance and Locks on a zone (BUILD M3-T1): §6.3's Plague Counter row with
//! R78's reset, and §3.2's Lock, which the current occupant survives and which outlives it.
//!
//! Port of `packages/engine/test/effects-counters.test.ts`.

use jackioh_engine::effects::{clear_plague, lock, plague};
use jackioh_engine::testkit::*;
use jackioh_engine::{
    PlayerId::{P1, P2},
    Row::{Backrow, Units},
};

use super::fixtures::combat::plain;
use super::fixtures::harness::{events_of_type, new_game, put, slot};
use super::fixtures::scripts::mana_well;

/// TS `sinkFor(state)` (fixtures/harness.ts): the state, a fresh event list and an rng at the state's
/// cursor, as `reduce` starts one. A sink borrows all three, so they live here and `sink()` lends them
/// out, built from part 1's frozen `EngineSink::new` and `Rng::new`.
struct Bench<'a> {
    state: &'a mut GameState,
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Bench<'_> {
    fn sink(&mut self) -> EngineSink<'_> {
        EngineSink::new(self.state, &mut self.events, &mut self.rng)
    }
}

fn sink_for(state: &mut GameState) -> Bench<'_> {
    let rng = Rng::new(&state.seed, state.rng_cursor);
    Bench { state, events: Vec::new(), rng }
}

/// TS `run`'s options: `{ self?: CardInstance | null; controller?: PlayerId; targets?: Selection[] }`.
#[derive(Default)]
struct RunOptions {
    self_: Option<CardInstance>,
    controller: Option<PlayerId>,
    targets: Option<Vec<Selection>>,
}

/// TS `effect.apply(makeContext(sink, self, rest))`. TS handed over the live card object; the card
/// is looked up again by id, so the context sees it as it stands now.
fn run(sink: &mut EngineSink<'_>, effect: Effect, options: RunOptions) {
    let RunOptions { self_, controller, targets } = options;
    let self_ = self_.map(|card| find_instance(sink.state, &card.id).cloned().unwrap_or(card));
    let mut ctx = make_context(
        sink.reborrow(),
        self_,
        HookOptions { controller, targets, ..Default::default() },
    );
    (effect.apply)(&mut ctx);
}

fn on_instance(instance: &CardInstance) -> Vec<Selection> {
    vec![Selection::Instance { instance_id: instance.id.clone() }]
}

fn hand_card(state: &mut GameState, def_id: &str) -> CardInstance {
    let card = new_instance(state, def_id, P1, Zone::Hand { player: P1 });
    state.players.p1.hand.push(card.clone());
    card
}

/// The card as it stands in the state now (TS held the live object).
fn live<'a>(state: &'a GameState, card: &CardInstance) -> &'a CardInstance {
    find_instance(state, &card.id).unwrap_or_else(|| panic!("{} is nowhere", card.id))
}

fn json_of<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// `eventsOfType(events, "counterChanged").map((e) => e.value)`.
fn counter_values(events: &[GameEvent]) -> Vec<i32> {
    events_of_type(events, GameEventType::CounterChanged)
        .iter()
        .map(|event| match event {
            GameEvent::CounterChanged { value, .. } => *value,
            other => panic!("not a counterChanged: {other:?}"),
        })
        .collect()
}

mod plague_s6_3_plague_counter_m3_t1 {
    use super::*;

    #[test]
    fn c91_adds_plague_counters_any_number_of_them_and_reports_the_new_count() {
        let mut state = new_game("plague-add", None);
        let unit = put(&mut state, &plain.id, slot(P1, Units, 1), json!({}));
        let mut sink = sink_for(&mut state);

        run(
            &mut sink.sink(),
            plague(json_as(json!({ "target": { "of": "chosen" }, "amount": 1 }))),
            RunOptions { targets: Some(on_instance(&unit)), ..Default::default() },
        );
        run(
            &mut sink.sink(),
            plague(json_as(json!({ "target": { "of": "self" }, "amount": 1 }))),
            RunOptions { self_: Some(unit.clone()), ..Default::default() },
        );
        run(
            &mut sink.sink(),
            plague(json_as(json!({ "target": { "of": "self" }, "amount": 5 }))),
            RunOptions { self_: Some(unit.clone()), ..Default::default() },
        );

        assert_eq!(live(sink.state, &unit).counters.plague, Some(7));
        // R471: every gain is a placement, reported with how many tokens it put there.
        assert_eq!(
            json_of(&events_of_type(&sink.events, GameEventType::CounterChanged)),
            json!([
                { "type": "counterChanged", "instanceId": unit.id, "counter": "plague", "value": 1, "placed": 1 },
                { "type": "counterChanged", "instanceId": unit.id, "counter": "plague", "value": 2, "placed": 1 },
                { "type": "counterChanged", "instanceId": unit.id, "counter": "plague", "value": 7, "placed": 5 },
            ])
        );
    }

    #[test]
    fn takes_tokens_off_floors_the_count_at_0_and_clears_the_counter_outright() {
        let mut state = new_game("plague-clear", None);
        let card = put(&mut state, &mana_well().id, slot(P1, Backrow, 1), json!({}));
        let mut sink = sink_for(&mut state);
        let as_self = || RunOptions { self_: Some(card.clone()), ..Default::default() };

        run(&mut sink.sink(), plague(json_as(json!({ "target": { "of": "self" }, "amount": 3 }))), as_self());
        run(&mut sink.sink(), plague(json_as(json!({ "target": { "of": "self" }, "amount": -2 }))), as_self());
        assert_eq!(live(sink.state, &card).counters.plague, Some(1));

        // Removing more than the card has leaves it at 0, not below.
        run(&mut sink.sink(), plague(json_as(json!({ "target": { "of": "self" }, "amount": -9 }))), as_self());
        assert_eq!(live(sink.state, &card).counters.plague, None);
        assert_eq!(counter_values(&sink.events), vec![3, 1, 0]);

        // A count that does not change emits nothing: no tokens to remove, none to clear.
        run(&mut sink.sink(), plague(json_as(json!({ "target": { "of": "self" }, "amount": -1 }))), as_self());
        run(&mut sink.sink(), clear_plague(json_as(json!({ "target": { "of": "self" } }))), as_self());
        assert_eq!(events_of_type(&sink.events, GameEventType::CounterChanged).len(), 3);

        run(&mut sink.sink(), plague(json_as(json!({ "target": { "of": "self" }, "amount": 4 }))), as_self());
        run(&mut sink.sink(), clear_plague(Default::default()), as_self());
        assert_eq!(live(sink.state, &card).counters.plague, None);
        assert_eq!(counter_values(&sink.events), vec![3, 1, 0, 4, 0]);
    }

    #[test]
    fn r78_plague_counters_reset_when_the_card_leaves_the_field() {
        let mut state = new_game("plague-leaves", None);
        let unit = put(&mut state, &plain.id, slot(P1, Units, 1), json!({}));
        let mut sink = sink_for(&mut state);

        run(
            &mut sink.sink(),
            plague(json_as(json!({ "target": { "of": "self" }, "amount": 3 }))),
            RunOptions { self_: Some(unit.clone()), ..Default::default() },
        );
        assert_eq!(live(sink.state, &unit).counters.plague, Some(3));

        let moving = live(sink.state, &unit).clone();
        move_to_zone(sink.state, &moving, ZoneName::Graveyard, Default::default());
        assert_eq!(live(sink.state, &unit).counters.plague, None);
    }

    #[test]
    fn does_nothing_without_a_card_a_hero_selection_an_empty_selection_or_a_zero_amount() {
        let mut state = new_game("plague-fizzle", None);
        let unit = put(&mut state, &plain.id, slot(P1, Units, 1), json!({}));
        let mut sink = sink_for(&mut state);

        run(
            &mut sink.sink(),
            plague(json_as(json!({ "target": { "of": "enemyHero" }, "amount": 2 }))),
            RunOptions { controller: Some(P1), ..Default::default() },
        );
        run(
            &mut sink.sink(),
            plague(json_as(json!({ "target": { "of": "chosen" }, "amount": 2 }))),
            RunOptions { targets: Some(vec![]), ..Default::default() },
        );
        run(
            &mut sink.sink(),
            plague(json_as(json!({ "target": { "of": "self" }, "amount": 0 }))),
            RunOptions { self_: Some(unit.clone()), ..Default::default() },
        );

        assert_eq!(live(sink.state, &unit).counters.plague, None);
        assert_eq!(sink.events.len(), 0);
    }
}

mod lock_s3_2_m3_t1 {
    use super::*;

    #[test]
    fn s3_2_the_current_occupant_is_unaffected_and_the_lock_outlives_it() {
        let mut state = new_game("lock-occupant", None);
        let occupant = put(&mut state, &plain.id, slot(P2, Units, 2), json!({}));
        let mut sink = sink_for(&mut state);

        run(
            &mut sink.sink(),
            lock(json_as(json!({ "zone": { "of": "chosen" } }))),
            RunOptions { targets: Some(on_instance(&occupant)), controller: Some(P1), ..Default::default() },
        );

        // The occupant stays where it was, with its stats and its zone untouched.
        assert_eq!(card_at(sink.state, slot(P2, Units, 2)).map(|card| card.id.clone()), Some(occupant.id.clone()));
        assert_eq!(live(sink.state, &occupant).zone, Zone::Field { player: P2, row: Units, lane: 2 });
        assert!(is_locked(sink.state, slot(P2, Units, 2)));
        assert!(!is_open(sink.state, slot(P2, Units, 2)));
        assert_eq!(
            json_of(&events_of_type(&sink.events, GameEventType::Locked)),
            json!([{ "type": "locked", "player": "p2", "row": "units", "lane": 2 }])
        );

        // Once it leaves, the lock lasts until the game ends — but R688 lets placements in: a Lock
        // refuses plays, never summons or moves.
        let moving = live(sink.state, &occupant).clone();
        move_to_zone(sink.state, &moving, ZoneName::Graveyard, Default::default());
        assert!(card_at(sink.state, slot(P2, Units, 2)).is_none());
        assert!(is_locked(sink.state, slot(P2, Units, 2)));
        let newcomer = new_instance(&mut *sink.state, &plain.id, P2, Zone::Hand { player: P2 });
        assert!(place_on_field(sink.state, newcomer.clone(), slot(P2, Units, 2), Default::default()));
        assert_eq!(card_at(sink.state, slot(P2, Units, 2)).map(|card| card.id.clone()), Some(newcomer.id.clone()));
        assert!(is_locked(sink.state, slot(P2, Units, 2)));
    }

    #[test]
    fn c36_magic_jammed_locks_a_named_backrow_zone_and_locking_it_again_changes_nothing() {
        let mut state = new_game("lock-backrow", None);
        let trap = put(&mut state, &mana_well().id, slot(P2, Backrow, 4), json!({}));
        let mut sink = sink_for(&mut state);

        run(
            &mut sink.sink(),
            lock(json_as(json!({ "zone": { "of": "chosen" } }))),
            RunOptions { targets: Some(on_instance(&trap)), controller: Some(P1), ..Default::default() },
        );
        let moving = live(sink.state, &trap).clone();
        move_to_zone(sink.state, &moving, ZoneName::Graveyard, Default::default());

        assert!(is_locked(sink.state, slot(P2, Backrow, 4)));
        // Only that row and lane: the unit zone in the same lane is untouched.
        assert!(!is_locked(sink.state, slot(P2, Units, 4)));
        assert!(!is_locked(sink.state, slot(P1, Backrow, 4)));

        run(
            &mut sink.sink(),
            lock(json_as(json!({ "zone": { "of": "lane", "player": "enemy", "row": "backrow", "lane": 4 } }))),
            RunOptions { controller: Some(P1), ..Default::default() },
        );
        assert_eq!(events_of_type(&sink.events, GameEventType::Locked).len(), 1);
    }

    #[test]
    fn s3_1_locks_a_lane_by_index_and_r64s_placement_then_skips_it() {
        let mut state = new_game("lock-lane", None);
        let mut sink = sink_for(&mut state);

        run(
            &mut sink.sink(),
            lock(json_as(json!({ "zone": { "of": "lane", "row": "units", "lane": 1 } }))),
            RunOptions { controller: Some(P1), ..Default::default() },
        );

        assert_eq!(
            json_of(&events_of_type(&sink.events, GameEventType::Locked)),
            json!([{ "type": "locked", "player": "p1", "row": "units", "lane": 1 }])
        );
        assert_eq!(first_free_zone(sink.state, P1, Units).map(|zone| zone.lane), Some(2));
        assert_eq!(first_free_zone(sink.state, P2, Units).map(|zone| zone.lane), Some(1));
    }

    #[test]
    fn locks_the_zone_the_card_running_the_effect_sits_in_s3_1_this_lane() {
        let mut state = new_game("lock-self", None);
        let self_card = put(&mut state, &plain.id, slot(P1, Units, 3), json!({}));
        let mut sink = sink_for(&mut state);

        run(
            &mut sink.sink(),
            lock(json_as(json!({ "zone": { "of": "self" } }))),
            RunOptions { self_: Some(self_card.clone()), ..Default::default() },
        );

        assert!(is_locked(sink.state, slot(P1, Units, 3)));
        assert_eq!(events_of_type(&sink.events, GameEventType::Locked).len(), 1);
    }

    #[test]
    fn does_nothing_when_the_zone_cannot_be_named_no_card_an_off_field_card_a_bad_lane() {
        let mut state = new_game("lock-fizzle", None);
        let in_hand_card = hand_card(&mut state, &plain.id);
        let mut sink = sink_for(&mut state);

        run(
            &mut sink.sink(),
            lock(json_as(json!({ "zone": { "of": "self" } }))),
            RunOptions { self_: None, ..Default::default() },
        );
        run(
            &mut sink.sink(),
            lock(json_as(json!({ "zone": { "of": "chosen" } }))),
            RunOptions { targets: Some(vec![]), ..Default::default() },
        );
        run(
            &mut sink.sink(),
            lock(json_as(json!({ "zone": { "of": "chosen" } }))),
            RunOptions { targets: Some(on_instance(&in_hand_card)), ..Default::default() },
        );
        run(
            &mut sink.sink(),
            lock(json_as(json!({ "zone": { "of": "lane", "row": "units", "lane": 6 } }))),
            RunOptions { controller: Some(P1), ..Default::default() },
        );
        run(
            &mut sink.sink(),
            lock(json_as(json!({ "zone": { "of": "lane", "row": "backrow", "lane": 0 } }))),
            RunOptions { controller: Some(P1), ..Default::default() },
        );

        assert_eq!(sink.events.len(), 0);
        assert_eq!(
            json_of(&sink.state.players.p1.locks),
            json!({
                "units": [false, false, false, false, false],
                "backrow": [false, false, false, false, false],
            })
        );
    }
}
