//! B5 E7 set health and E8 heal into damage, the effect verbs (`effects/health.ts`). The replacement
//! that turns a heal into damage as it happens is `replacements.test.ts`'s; this file proves the verbs
//! a card writes: `setHealth` (Classic #29) and `convertHealing`.
//!
//! Port of `packages/engine/test/effects-health.test.ts`.

use jackioh_engine::effects::{convert_healing, heal, set_health};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{blood_moon, gambit, grunt, playing, recorder, replays_to, vital_kill};
use crate::rules::fixtures::harness::{events_of_type, in_hand, put, slot};

/// TS `sinkFor(state)`: the events and rng of a sink over `state`, the rng starting at the state's
/// cursor as reduce does. The state is lent to it call by call (`on`).
struct Sink {
    events: Vec<GameEvent>,
    rng: Rng,
}

fn sink_for(state: &GameState) -> Sink {
    Sink { events: Vec::new(), rng: Rng::new(&state.seed, state.rng_cursor) }
}

impl Sink {
    fn on<'a>(&'a mut self, state: &'a mut GameState) -> EngineSink<'a> {
        EngineSink::new(state, &mut self.events, &mut self.rng)
    }

    /// `applyEffects(effects, makeContext(sink, self))`, the card read as it stands now.
    fn apply(&mut self, state: &mut GameState, effects: &[Effect], self_id: Option<&str>) {
        let self_: Option<CardInstance> =
            self_id.map(|id| find_instance(&*state, id).expect("the card is in the state").clone());
        let mut engine = self.on(state);
        let mut ctx = make_context(&mut engine, self_.as_ref(), HookOptions::default());
        apply_effects(effects, &mut ctx);
    }
}

fn of_type(events: &[GameEvent], kind: GameEventType) -> Value {
    serde_json::to_value(events_of_type(events, kind)).expect("events serialise")
}

mod e7_set_health {
    use super::*;

    #[test]
    fn sets_either_hero_s_health_up_or_down_with_no_pipeline_no_damage_no_heal_no_replacement() {
        let mut state = playing("dc-set-health");
        let self_ = put(&mut state, &grunt().id, slot(PlayerId::P1, Row::Units, 1));
        put(&mut state, &gambit().id, slot(PlayerId::P1, Row::Backrow, 1));
        state.players[PlayerId::P1].hero.armor = 5;
        state.players[PlayerId::P2].hero.health = 40;
        let mut sink = sink_for(&state);
        {
            let mut engine = sink.on(&mut state);
            let mut ctx = make_context(&mut engine, Some(&self_), HookOptions::default());
            apply_effects(
                &[
                    set_health(json_as(json!({ "to": { "of": "enemyHero" }, "value": 13 }))),
                    set_health(json_as(json!({ "to": { "of": "selfHero" }, "value": 13 }))),
                ],
                &mut ctx,
            );
            assert_eq!(ctx.state.players[PlayerId::P2].hero.health, 13);
            assert_eq!(ctx.state.players[PlayerId::P1].hero.health, 13);
            // Down to 0 is no hit either: a Final Gambit has nothing to answer.
            apply_effects(&[set_health(json_as(json!({ "to": { "of": "selfHero" }, "value": 0 })))], &mut ctx);
            assert_eq!(ctx.state.players[PlayerId::P1].hero.health, 0);
        }
        assert_eq!(
            of_type(&sink.events, GameEventType::HealthSet),
            json!([
                { "type": "healthSet", "player": "p2", "health": 13, "sourceId": self_.id },
                { "type": "healthSet", "player": "p1", "health": 13, "sourceId": self_.id },
                { "type": "healthSet", "player": "p1", "health": 0, "sourceId": self_.id },
            ])
        );
        assert_eq!(of_type(&sink.events, GameEventType::Damage), json!([]));
        assert_eq!(of_type(&sink.events, GameEventType::Healed), json!([]));
        assert_eq!(of_type(&sink.events, GameEventType::TrapFired), json!([]));
    }

    #[test]
    fn a_target_that_is_not_a_hero_fizzles() {
        let mut state = playing("dc-set-health-unit");
        let self_ = put(&mut state, &grunt().id, slot(PlayerId::P1, Row::Units, 1));
        let mut sink = sink_for(&state);
        sink.apply(
            &mut state,
            &[set_health(json_as(json!({ "to": { "of": "self" }, "value": 13 })))],
            Some(self_.id.as_str()),
        );
        assert_eq!(sink.events, Vec::<GameEvent>::new());
    }

    #[test]
    fn r97_the_event_is_public_to_both_seats_and_the_game_replays() {
        let mut state = playing("dc-set-health-view");
        let Some(spell) = in_hand(&mut state, &vital_kill().id, PlayerId::P1, 1).into_iter().next() else {
            panic!("no spell");
        };
        let mut game = recorder(&state);
        game.play(json_as(json!({
            "type": "play",
            "instanceId": spell.id,
            "targets": [{ "pick": "hero", "player": "p2" }],
            "playerId": "p1",
        })));
        for viewer in [PlayerId::P1, PlayerId::P2] {
            let seen: Vec<Value> = view_for(&game.state(), viewer)
                .events
                .iter()
                .filter(|event| event.event_type() == GameEventType::HealthSet)
                .map(|event| serde_json::to_value(event).expect("an event serialises"))
                .collect();
            assert_eq!(seen, vec![json!({ "type": "healthSet", "player": "p2", "health": 13, "sourceId": spell.id })]);
        }
        assert!(replays_to(&game.start, &game.log, &game.state()));
    }
}

mod e8_convert_healing {
    use super::*;

    #[test]
    fn installs_a_this_turn_modifier_heals_on_its_controller_s_enemies_become_pierce_damage_from_the_card() {
        let mut state = playing("dc-convert");
        let moon = put(&mut state, &blood_moon().id, slot(PlayerId::P2, Row::Backrow, 2));
        find_instance_mut(&mut state, &moon.id).expect("the card is in the state").face_up = Some(true);
        let mut sink = sink_for(&state);
        sink.apply(&mut state, &[convert_healing(Default::default())], Some(moon.id.as_str()));
        let mods = serde_json::to_value(&state.players[PlayerId::P2].mods).expect("mods serialise");
        let Value::Array(mods) = mods else {
            panic!("a list of modifiers");
        };
        assert_eq!(mods.len(), 1);
        let mut only = mods[0].clone();
        // TS `id: expect.any(String)`.
        assert!(only.get("id").is_some_and(Value::is_string));
        if let Value::Object(fields) = &mut only {
            fields.remove("id");
        }
        assert_eq!(
            only,
            json!({ "kind": "healToDamage", "converterId": moon.id, "expiry": { "until": "thisTurn", "turn": state.turn } })
        );
        state.players[PlayerId::P1].hero.armor = 2;
        heal_hero(&mut sink.on(&mut state), PlayerId::P1, 7);
        assert_eq!(state.players[PlayerId::P1].hero.health, 23);
        // A unit of the enemy's: a heal of 3 on it deals 3.
        let body = put(&mut state, &grunt().id, slot(PlayerId::P1, Row::Units, 1));
        sink.apply(
            &mut state,
            &[heal(json_as(json!({ "target": { "of": "instance", "instanceId": body.id }, "amount": 3 })))],
            Some(body.id.as_str()),
        );
        assert_eq!(find_instance(&state, &body.id).expect("the card is in the state").damage, 3);
        // The badge both seats read says what it does (R169).
        let label = "Healing on your enemies deals Pierce damage instead";
        assert_eq!(
            view_for(&state, PlayerId::P2).you.modifiers.iter().map(|m| m.label.clone()).collect::<Vec<_>>(),
            vec![label.to_string()]
        );
        assert_eq!(
            view_for(&state, PlayerId::P1).opponent.modifiers.iter().map(|m| m.label.clone()).collect::<Vec<_>>(),
            vec![label.to_string()]
        );
    }

    #[test]
    fn a_converted_heal_is_a_new_damage_instance_divine_shield_and_the_lethal_window_meet_it() {
        let mut state = playing("dc-convert-lethal");
        let moon = put(&mut state, &blood_moon().id, slot(PlayerId::P2, Row::Backrow, 2));
        let mut sink = sink_for(&state);
        sink.apply(&mut state, &[convert_healing(Default::default())], Some(moon.id.as_str()));
        put(&mut state, &gambit().id, slot(PlayerId::P1, Row::Backrow, 1));
        state.players[PlayerId::P1].hero.health = 3;
        heal_hero(&mut sink.on(&mut state), PlayerId::P1, 5);
        // p1's Final Gambit sent the 5 to p2's hero.
        assert_eq!(state.players[PlayerId::P1].hero.health, 3);
        assert_eq!(state.players[PlayerId::P2].hero.health, 25);
        assert_eq!(events_of_type(&sink.events, GameEventType::Redirected).len(), 1);
    }

    #[test]
    fn with_no_card_to_come_from_it_does_nothing() {
        let mut state = playing("dc-convert-none");
        let mut sink = sink_for(&state);
        sink.apply(&mut state, &[convert_healing(Default::default())], None);
        assert_eq!(state.players[PlayerId::P1].mods, Vec::<PlayerModifier>::new());
        deal_damage(
            &mut sink.on(&mut state),
            DamageArgs { source: None, target: DamageTarget::Hero { player: PlayerId::P1 }, amount: 1, flags: None },
        );
        assert_eq!(state.players[PlayerId::P1].hero.health, 29);
    }
}
