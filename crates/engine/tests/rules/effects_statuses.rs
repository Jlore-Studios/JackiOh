//! B5 E35 unit statuses (`effects/statuses.ts`): Berserk (Classic+ #19.2 sends Classic+ #19.5 there)
//! and "may attack again" (Classic+ #73.1's Classic Golem after a kill).
//!
//! Port of `packages/engine/test/effects-statuses.test.ts`.

use jackioh_engine::testkit::*;

use jackioh_engine::combat::can_attack;
use jackioh_engine::damage::DamageTarget;
use jackioh_engine::effects::{bounce, go_berserk, may_attack_again, vanilla};
use jackioh_engine::reduce::legal_actions;
use jackioh_engine::resolve::{HookOptions, apply_effects, make_context};
use jackioh_engine::restrictions::is_berserk;
use jackioh_engine::rng::Rng;
use jackioh_engine::script::EngineSink;
use jackioh_engine::state::{CardInstance, GameState, find_instance, find_instance_mut};
use jackioh_engine::view_for::view_for;

use super::fixtures::damage_combat::{bot_loser, grunt, playing, recorder, replays_to};
use super::fixtures::harness::{put, slot};

/// `fixtures/harness.ts`'s `eventsOfType`, as JSON: the events of one type, each as TS writes it.
fn of_type(events: &[GameEvent], kind: &str) -> Vec<Value> {
    events
        .iter()
        .map(|event| serde_json::to_value(event).expect("an event serialises"))
        .filter(|event| event["type"] == kind)
        .collect()
}

/// The card under `id` as it stands in the state now (TS read the live object).
fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).expect("the card is in the state")
}

fn controlled_by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

/// `{ target: { of: "instance", instanceId } }`.
fn on(id: &str) -> Value {
    json!({ "target": { "of": "instance", "instanceId": id } })
}

/// `{ kind: "hero", player: "p2" }`: the attack target `canAttack` is asked about.
fn enemy_hero() -> DamageTarget {
    DamageTarget::Hero { player: PlayerId::P2 }
}

mod e35_go_berserk {
    use super::*;

    #[test]
    fn sets_the_status_marks_it_in_both_views_and_a_berserk_bot_loser_attacks_its_own_hero_at_its_start_of_turn() {
        let mut state = playing("dc-berserk");
        let bot = put(&mut state, &bot_loser().id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut events = Vec::new();
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            {
                let mut ctx = make_context(&mut sink, None, controlled_by(PlayerId::P2));
                apply_effects(&[go_berserk(json_as(on(&bot.id)))], &mut ctx);
            }
            assert!(is_berserk(live(&*sink.state, &bot.id)));
            assert_eq!(
                of_type(&*sink.events, "marked"),
                vec![json!({ "type": "marked", "instanceId": bot.id, "mark": "berserk", "color": "red", "added": true })]
            );
            // A second call changes nothing and says nothing.
            {
                let mut ctx = make_context(&mut sink, None, HookOptions::default());
                apply_effects(&[go_berserk(json_as(on(&bot.id)))], &mut ctx);
            }
            assert_eq!(of_type(&*sink.events, "marked").len(), 1);
        }
        for viewer in [PlayerId::P1, PlayerId::P2] {
            let view = serde_json::to_value(view_for(&state, viewer)).expect("a view serialises");
            let side = if viewer == PlayerId::P1 { &view["you"] } else { &view["opponent"] };
            assert_eq!(side["units"][1]["berserk"], json!(true));
        }

        // Its card's own text makes the attacks: at p1's next start of turn, 5 to p1's own hero.
        let mut game = recorder(&state);
        game.play(json_as(json!({ "type": "endTurn", "playerId": "p1" })));
        assert_eq!(game.state().players.p1.hero.health, 30);
        let back = game.play(json_as(json!({ "type": "endTurn", "playerId": "p2" })));
        let after = game.state().clone();
        assert_eq!(after.players.p1.hero.health, 25);
        assert_eq!(
            of_type(&back.events, "attackDeclared"),
            vec![json!({ "type": "attackDeclared", "attackerId": bot.id, "targetId": "hero-p1", "forced": true })]
        );
        assert!(replays_to(&game.start, &game.log, &after));
    }

    #[test]
    fn a_unit_that_cant_go_berserk_does_not_and_the_status_goes_when_the_unit_leaves_the_field_not_with_its_text() {
        let mut state = playing("dc-berserk-never");
        let calm = put(
            &mut state,
            &bot_loser().id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({ "radiant": true }),
        );
        let wild = put(&mut state, &grunt().id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut events = Vec::new();
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let mut ctx = make_context(&mut sink, None, controlled_by(PlayerId::P1));
            apply_effects(&[go_berserk(json_as(on(&calm.id)))], &mut ctx);
            assert!(!is_berserk(live(&*ctx.state, &calm.id)));
            apply_effects(&[go_berserk(json_as(on(&wild.id)))], &mut ctx);
            // A Vanilla takes text; Berserk is not text.
            apply_effects(&[vanilla(json_as(on(&wild.id)))], &mut ctx);
            assert!(is_berserk(live(&*ctx.state, &wild.id)));
            // Leaving the field resets it (R78).
            apply_effects(&[bounce(json_as(on(&wild.id)))], &mut ctx);
        }
        assert_eq!(find_instance(&state, &wild.id).and_then(|card| card.berserk), None);
    }
}

mod e35_may_attack_again {
    use super::*;

    fn has_attack(now: &GameState, id: &str) -> bool {
        legal_actions(now, PlayerId::P1).iter().any(|action| {
            let action = serde_json::to_value(action).expect("an action serialises");
            action["type"] == "attack" && action["attackerId"] == id
        })
    }

    #[test]
    fn a_unit_that_has_attacked_gets_a_fresh_exertion_one_that_entered_this_turn_is_no_longer_sick() {
        let mut state = playing("dc-again");
        let striker = put(&mut state, &grunt().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let fresh = put(&mut state, &grunt().id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        let turn = state.turn;
        find_instance_mut(&mut state, &fresh.id).expect("the fresh unit").summoned_turn = Some(turn);
        let mut game = recorder(&state);
        game.play(json_as(json!({ "type": "attack", "attackerId": striker.id, "targetId": "hero-p2", "playerId": "p1" })));
        let mut now = game.state().clone();
        assert!(!has_attack(&now, &striker.id));
        assert!(!has_attack(&now, &fresh.id));

        assert!(
            find_instance(&now, &striker.id).is_some() && find_instance(&now, &fresh.id).is_some(),
            "units gone"
        );
        let mut rng = Rng::new(&now.seed, now.rng_cursor);
        let mut events = Vec::new();
        {
            let mut sink = EngineSink::new(&mut now, &mut events, &mut rng);
            let mut ctx = make_context(&mut sink, None, controlled_by(PlayerId::P1));
            apply_effects(
                &[
                    may_attack_again(json_as(on(&striker.id))),
                    may_attack_again(json_as(on(&fresh.id))),
                ],
                &mut ctx,
            );
        }
        assert!(has_attack(&now, &striker.id));
        // TS asked about the live object it had read before the effects ran, which the effect wrote
        // through: the card under its id now.
        assert!(can_attack(&now, live(&now, &fresh.id), &enemy_hero()));
    }

    #[test]
    fn it_lends_no_attack_a_unit_could_not_otherwise_make() {
        let mut state = playing("dc-again-def");
        let guard = put(&mut state, &grunt().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        find_instance_mut(&mut state, &guard.id).expect("the guard").position = Some(Position::Def);
        let guard_now = live(&state, &guard.id).clone();
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut events = Vec::new();
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let mut ctx = make_context(&mut sink, Some(&guard_now), HookOptions::default());
            apply_effects(&[may_attack_again(json_as(json!({ "target": { "of": "self" } })))], &mut ctx);
        }
        assert!(!can_attack(&state, live(&state, &guard.id), &enemy_hero()));
    }
}
