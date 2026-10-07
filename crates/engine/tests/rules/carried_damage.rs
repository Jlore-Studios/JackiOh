//! R446: a Unit a carrier holds is a Unit for every rule, so a hit lands on it — targeted
//! or swept — as on any acting Unit (`zones.actsOnField`, which knows carried Units; the damage
//! pipeline asks it rather than "the top of its unit zone"). Fixtures: `fixtures/field.ts`. And R53,
//! R446: it can't attack or be attacked, so a forced attack on "a random enemy" never draws it.
//!
//! Port of `packages/engine/test/carried-damage.test.ts`.

use jackioh_engine::effects::combat::forced_attack_random;
use jackioh_engine::effects::damage::damage_all;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::field::{act_result, flush, playing, tower};
use crate::rules::fixtures::harness::{in_hand, put, sink_for, slot};

/// An `ActionInput` from its TS object literal (SURFACE §8: an object literal ports as `json!`).
fn input(body: Value) -> ActionInput {
    json_as(body)
}

/// TS `makeContext(sink, null, { controller })`'s options.
fn by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

fn by_id<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

/// p1 plays a plain body on top of a Tower in backrow lane 2; the state after, and the carried Unit.
fn carried(seed: &str) -> (GameState, CardInstance) {
    let mut state = playing(seed);
    put(&mut state, &tower().id, slot(P1, Row::Backrow, 2), Default::default());
    let card = in_hand(&mut state, &plain().id, P1, None)
        .into_iter()
        .next()
        .expect("no plain body in hand");
    flush(&mut state, P1, None);
    let result = act_result(
        &state,
        input(json!({
            "type": "play", "instanceId": card.id, "zone": { "row": "backrow", "lane": 2 }, "playerId": "p1"
        })),
    );
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    let rider = find_instance(&result.state, &card.id).expect("the rider is gone").clone();
    (result.state, rider)
}

mod r446_a_carried_unit_takes_damage {
    use super::*;

    #[test]
    fn r446_a_hit_aimed_at_it_lands() {
        let (mut state, rider) = carried("carried-hit");
        let dealt = deal_damage(
            &mut sink_for(&mut state),
            DamageArgs {
                source: None,
                target: DamageTarget::Unit { instance: rider.clone() },
                amount: 2,
                flags: None,
            },
        );
        assert_eq!(dealt, 2);
        let view = unit_view(&state, by_id(&state, &rider.id));
        assert_eq!(view.health, view.max_health - 2);
    }

    #[test]
    fn r446_a_sweep_over_all_units_hits_it() {
        let (mut state, rider) = carried("carried-sweep");
        let mut sink = sink_for(&mut state);
        (damage_all(json_as(json!({ "amount": 1, "side": "enemy" }))).apply)(&mut make_context(
            sink.reborrow(),
            None,
            by(P2),
        ));
        let view = unit_view(sink.state, by_id(sink.state, &rider.id));
        assert_eq!(view.health, view.max_health - 1);
        assert!(sink.events.iter().any(|event| matches!(
            event,
            GameEvent::Damage { target_id, .. } if *target_id == rider.id
        )));
    }
}

mod r53_r446_a_carried_unit_is_out_of_a_random_forced_attack {
    use super::*;

    #[test]
    fn it_is_never_drawn_as_the_target_and_with_nothing_else_to_attack_no_roll_is_spent() {
        let (mut state, rider) = carried("carried-random-target");
        let striker = put(&mut state, &plain().id, slot(P2, Row::Units, 3), Default::default());
        assert!(random_attack_targets(&state, &striker, json_as(json!("enemyUnits"))).is_empty());
        assert_eq!(
            random_attack_targets(&state, &striker, json_as(json!("enemies"))),
            vec![AttackTarget::Hero { player: P1 }]
        );
        let mut sink = sink_for(&mut state);
        let cursor = sink.rng.cursor();
        (forced_attack_random(json_as(json!({ "attacker": { "of": "self" }, "among": "enemyUnits" }))).apply)(
            &mut make_context(sink.reborrow(), Some(&striker), Default::default()),
        );
        assert!(sink.events.is_empty());
        assert_eq!(sink.rng.cursor(), cursor);
        assert_eq!(by_id(sink.state, &rider.id).damage, 0);
    }

    #[test]
    fn carried_it_draws_no_target_of_its_own() {
        let (mut state, rider) = carried("carried-random-attacker");
        put(&mut state, &plain().id, slot(P2, Row::Units, 3), Default::default());
        assert!(random_attack_targets(&state, by_id(&state, &rider.id), json_as(json!("enemies"))).is_empty());
    }
}
