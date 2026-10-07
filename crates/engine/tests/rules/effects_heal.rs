//! The heal effect (BUILD M3-T1): §6.3's three readings of Heal — "heal X", "heal to full" and
//! "heal up to N" — over R19's target set, a unit capped at its max health and a hero uncapped (§3).
//!
//! Port of `packages/engine/test/effects-heal.test.ts`.

use jackioh_engine::effects::heal::heal;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{big_body, plain};
use crate::rules::fixtures::harness::{events_of_type, new_game, put, slot};

/// TS `sinkFor(state)`: the events and rng of a sink over `state`, the rng starting at the state's
/// cursor as reduce does. The state is lent to it call by call, so a test reads the state between
/// runs as TS read its live objects.
struct Sink {
    events: Vec<GameEvent>,
    rng: Rng,
}

fn sink_for(state: &GameState) -> Sink {
    Sink { events: Vec::new(), rng: Rng::new(&state.seed, state.rng_cursor) }
}

/// TS `run`'s options: `self` (the card under this id as it stands when the run starts; TS passed the
/// live object), and the rest handed to `makeContext` as they are.
#[derive(Default)]
struct RunOptions {
    self_: Option<String>,
    controller: Option<PlayerId>,
    targets: Option<Vec<Selection>>,
}

/// `effect.apply(makeContext(sink, self, rest))`.
fn run(sink: &mut Sink, state: &mut GameState, effect: Effect, options: RunOptions) {
    let self_: Option<CardInstance> =
        options.self_.as_ref().map(|id| find_instance(&*state, id).expect("the card is in the state").clone());
    let mut engine = EngineSink::new(state, &mut sink.events, &mut sink.rng);
    let mut ctx = make_context(
        &mut engine,
        self_.as_ref(),
        HookOptions { controller: options.controller, targets: options.targets, ..Default::default() },
    );
    (effect.apply)(&mut ctx);
}

fn on_instance(instance: &CardInstance) -> Option<Vec<Selection>> {
    Some(vec![Selection::Instance { instance_id: instance.id.clone() }])
}

fn self_is(card: &CardInstance) -> RunOptions {
    RunOptions { self_: Some(card.id.clone()), ..Default::default() }
}

fn as_player(player: PlayerId) -> RunOptions {
    RunOptions { controller: Some(player), ..Default::default() }
}

fn chosen(targets: Option<Vec<Selection>>) -> RunOptions {
    RunOptions { targets, ..Default::default() }
}

fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).expect("the card is in the state")
}

fn live_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).expect("the card is in the state")
}

/// `unitView(state, unit).health` for the card as it stands now.
fn health_of(state: &GameState, id: &str) -> i32 {
    unit_view(state, live(state, id)).health
}

fn healed(sink: &Sink) -> Value {
    serde_json::to_value(events_of_type(&sink.events, GameEventType::Healed)).expect("events serialise")
}

fn healed_count(sink: &Sink) -> usize {
    events_of_type(&sink.events, GameEventType::Healed).len()
}

fn heal_args(args: Value) -> Effect {
    heal(json_as(args))
}

mod heal_x_r19_m3_t1 {
    use super::*;

    #[test]
    fn r19_heal_x_takes_damage_off_a_unit_and_never_past_its_max_health() {
        let mut state = new_game("heal-unit", None);
        let unit = put(&mut state, &big_body.id, slot(PlayerId::P2, Row::Units, 1), json!({})); // 5/10
        live_mut(&mut state, &unit.id).damage = 6;
        let mut sink = sink_for(&state);

        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "chosen" }, "amount": 4 })), chosen(on_instance(&unit)));
        assert_eq!(live(&state, &unit.id).damage, 2);
        assert_eq!(health_of(&state, &unit.id), 8);

        // A bigger heal than there is damage stops at the max health and heals only what was missing.
        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "chosen" }, "amount": 99 })), chosen(on_instance(&unit)));
        assert_eq!(live(&state, &unit.id).damage, 0);
        assert_eq!(health_of(&state, &unit.id), 10);
        assert_eq!(
            healed(&sink),
            json!([
                { "type": "healed", "targetId": unit.id, "amount": 4 },
                { "type": "healed", "targetId": unit.id, "amount": 2 },
            ])
        );

        // An undamaged unit heals nothing and emits nothing.
        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "self" }, "amount": 5 })), self_is(&unit));
        assert_eq!(healed_count(&sink), 2);
    }

    /// TS: "§10.4: the cap a unit heals to is its buffed max health, read through the layers".
    #[test]
    fn the_cap_a_unit_heals_to_is_its_buffed_max_health_read_through_the_layers() {
        let mut state = new_game("heal-buffed", None);
        let unit = put(&mut state, &big_body.id, slot(PlayerId::P1, Row::Units, 1), json!({})); // 5/10
        live_mut(&mut state, &unit.id).buffs = AttackHealth { attack: 0, health: 5 }; // layer 4: max health 15
        live_mut(&mut state, &unit.id).damage = 12;
        let mut sink = sink_for(&state);

        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "self" }, "amount": 20 })), self_is(&unit));

        assert_eq!(live(&state, &unit.id).damage, 0);
        assert_eq!(health_of(&state, &unit.id), 15);
        assert_eq!(healed(&sink), json!([{ "type": "healed", "targetId": unit.id, "amount": 12 }]));
    }

    #[test]
    fn r19_heal_x_gives_a_hero_health_with_no_cap_on_either_side_5_47() {
        let mut state = new_game("heal-hero", None);
        let mut sink = sink_for(&state);

        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "selfHero" }, "amount": 20 })), as_player(PlayerId::P1));
        assert_eq!(state.players[PlayerId::P1].hero.health, HERO_HEALTH + 20);

        state.players[PlayerId::P2].hero.health = 8;
        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "enemyHero" }, "amount": 50 })), as_player(PlayerId::P1));
        assert_eq!(state.players[PlayerId::P2].hero.health, 58);

        run(
            &mut sink,
            &mut state,
            heal_args(json!({ "target": { "of": "chosen" }, "amount": 2 })),
            chosen(Some(vec![Selection::Hero { player: PlayerId::P2 }])),
        );
        assert_eq!(state.players[PlayerId::P2].hero.health, 60);
        let pairs: Vec<(Value, Value)> = events_of_type(&sink.events, GameEventType::Healed)
            .iter()
            .map(|event| {
                let event = serde_json::to_value(event).expect("an event serialises");
                (event["targetId"].clone(), event["amount"].clone())
            })
            .collect();
        assert_eq!(
            pairs,
            vec![(json!("hero-p1"), json!(20)), (json!("hero-p2"), json!(50)), (json!("hero-p2"), json!(2))]
        );
    }

    #[test]
    fn heals_nothing_when_there_is_no_target_and_a_non_positive_amount_does_nothing() {
        let mut state = new_game("heal-fizzle", None);
        let unit = put(&mut state, &big_body.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        live_mut(&mut state, &unit.id).damage = 3;
        let mut sink = sink_for(&state);

        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "chosen" }, "amount": 5 })), chosen(Some(vec![])));
        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "self" }, "amount": 5 })), RunOptions::default());
        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "self" }, "amount": 0 })), self_is(&unit));
        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "selfHero" }, "amount": -5 })), as_player(PlayerId::P1));

        assert_eq!(live(&state, &unit.id).damage, 3);
        assert_eq!(state.players[PlayerId::P1].hero.health, HERO_HEALTH);
        assert_eq!(sink.events.len(), 0);
    }
}

mod heal_to_full_and_heal_up_to_n_m3_t1 {
    use super::*;

    /// TS: "#19: heal to full removes all of a unit's damage".
    #[test]
    fn heal_to_full_removes_all_of_a_unit_s_damage_19() {
        let mut state = new_game("heal-full", None);
        let unit = put(&mut state, &big_body.id, slot(PlayerId::P1, Row::Units, 1), json!({})); // 5/10
        live_mut(&mut state, &unit.id).damage = 9;
        let mut sink = sink_for(&state);

        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "self" }, "toFull": true })), self_is(&unit));

        assert_eq!(live(&state, &unit.id).damage, 0);
        assert_eq!(health_of(&state, &unit.id), 10);
        assert_eq!(healed(&sink), json!([{ "type": "healed", "targetId": unit.id, "amount": 9 }]));

        // Already at full: nothing to remove, nothing emitted.
        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "self" }, "toFull": true })), self_is(&unit));
        assert_eq!(healed_count(&sink), 1);
    }

    /// TS: "§3: a hero has no maximum health, so heal to full leaves it alone".
    #[test]
    fn a_hero_has_no_maximum_health_so_heal_to_full_leaves_it_alone() {
        let mut state = new_game("heal-full-hero", None);
        state.players[PlayerId::P1].hero.health = 4;
        let mut sink = sink_for(&state);

        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "selfHero" }, "toFull": true })), as_player(PlayerId::P1));

        assert_eq!(state.players[PlayerId::P1].hero.health, 4);
        assert_eq!(sink.events.len(), 0);
    }

    /// TS: "§6.3: heal up to N raises a hero to N and never lowers one already above it".
    #[test]
    fn heal_up_to_n_raises_a_hero_to_n_and_never_lowers_one_already_above_it() {
        let mut state = new_game("heal-upto-hero", None);
        state.players[PlayerId::P1].hero.health = 12;
        let mut sink = sink_for(&state);

        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "selfHero" }, "upTo": 30 })), as_player(PlayerId::P1));
        assert_eq!(state.players[PlayerId::P1].hero.health, 30);
        assert_eq!(healed(&sink), json!([{ "type": "healed", "targetId": "hero-p1", "amount": 18 }]));

        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "selfHero" }, "upTo": 30 })), as_player(PlayerId::P1));
        assert_eq!(state.players[PlayerId::P1].hero.health, 30);
        assert_eq!(healed_count(&sink), 1);

        state.players[PlayerId::P2].hero.health = 45;
        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "enemyHero" }, "upTo": 30 })), as_player(PlayerId::P1));
        assert_eq!(state.players[PlayerId::P2].hero.health, 45);
    }

    /// TS: "§6.3: heal up to N on a unit stops at N, and at the unit's max health".
    #[test]
    fn heal_up_to_n_on_a_unit_stops_at_n_and_at_the_unit_s_max_health() {
        let mut state = new_game("heal-upto-unit", None);
        let unit = put(&mut state, &big_body.id, slot(PlayerId::P1, Row::Units, 1), json!({})); // 5/10
        live_mut(&mut state, &unit.id).damage = 8; // health 2
        let other = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 2), json!({})); // 3/3
        live_mut(&mut state, &other.id).damage = 2;
        let mut sink = sink_for(&state);

        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "self" }, "upTo": 6 })), self_is(&unit));
        assert_eq!(health_of(&state, &unit.id), 6);
        assert_eq!(live(&state, &unit.id).damage, 4);

        // Past the max health, the heal removes only the damage that is left (§6.3).
        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "self" }, "upTo": 30 })), self_is(&unit));
        assert_eq!(live(&state, &unit.id).damage, 0);
        assert_eq!(health_of(&state, &unit.id), 10);

        // A unit already at or above N is untouched.
        run(&mut sink, &mut state, heal_args(json!({ "target": { "of": "chosen" }, "upTo": 1 })), chosen(on_instance(&other)));
        assert_eq!(live(&state, &other.id).damage, 2);
        let amounts: Vec<Value> = events_of_type(&sink.events, GameEventType::Healed)
            .iter()
            .map(|event| serde_json::to_value(event).expect("an event serialises")["amount"].clone())
            .collect();
        assert_eq!(amounts, vec![json!(4), json!(4)]);
    }
}
