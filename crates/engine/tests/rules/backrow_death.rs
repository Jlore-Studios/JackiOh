//! §4.5 step 3's Death for a backrow card (docs/classic-sets.md A5, Classic+ #61 Bauble Bubble, Classic+
//! #12.8 Frostspatula destroyed in the backrow): a Field Spell, Trap or Field Trap that prints Death
//! fires it when it goes from its backrow zone to a graveyard — destroyed or sacrificed, as a Unit's
//! does — in R68's order beside the units of its side, and never on a bounce or an exile (§6.2).
//!
//! Port of `packages/engine/test/backrow-death.test.ts`.

use serde::Serialize;

use jackioh_engine::effects::{bounce, exile};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{
    bauble, grunt, notes, playing, rattle, recorder, replays_to, storm, voidwalker,
};
use crate::rules::fixtures::harness::{events_of_type, in_hand, put, sink_for, slot};

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

fn by_id_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id}"))
}

/// `events.map((event) => event.instanceId)`, read off the events' JSON.
fn instance_ids(events: impl Serialize) -> Vec<String> {
    let json = serde_json::to_value(events).expect("events serialise");
    json.as_array()
        .map(|list| {
            list.iter()
                .map(|event| event["instanceId"].as_str().unwrap_or_default().to_string())
                .collect()
        })
        .unwrap_or_default()
}

mod section_4_5_step_3_death_for_a_collected_backrow_card {
    use super::*;

    #[test]
    fn a_destroyed_field_spell_fires_its_death_in_r68s_order_the_active_sides_units_its_backrow_then_the_other_side() {
        let mut state = playing("dc-backrow-death");
        let foe_back = put(&mut state, &bauble.id, slot(P2, Row::Backrow, 1), Default::default());
        let foe = put(&mut state, &rattle.id, slot(P2, Row::Units, 3), Default::default());
        let pop = put(&mut state, &bauble.id, slot(P1, Row::Backrow, 2), Default::default());
        let own = put(&mut state, &rattle.id, slot(P1, Row::Units, 4), Default::default());
        for card in [&foe_back, &foe, &pop, &own] {
            by_id_mut(&mut state, &card.id).marked_destroyed = Some(true);
        }
        let mut sink = sink_for(&mut state);
        state_check(&mut sink);
        assert_eq!(
            instance_ids(events_of_type(sink.events, GameEventType::Destroyed)),
            vec![own.id.clone(), pop.id.clone(), foe.id.clone(), foe_back.id.clone()]
        );
        assert_eq!(
            notes(sink.state),
            vec!["rattle:death:p1", "bauble:death", "rattle:death:p2", "bauble:death"]
        );
    }

    #[test]
    fn the_death_hooks_run_in_that_order_and_a_bounce_or_an_exile_is_no_death() {
        let mut state = playing("dc-backrow-order");
        let own = put(&mut state, &rattle.id, slot(P1, Row::Units, 1), Default::default());
        let pop = put(&mut state, &bauble.id, slot(P1, Row::Backrow, 1), Default::default());
        let foe = put(&mut state, &rattle.id, slot(P2, Row::Units, 1), Default::default());
        let spell = in_hand(&mut state, &storm.id, P1, 1).remove(0);
        by_id_mut(&mut state, &pop.id).marked_destroyed = Some(true);
        let mut game = recorder(&state);
        game.play(input(json!({ "type": "play", "instanceId": spell.id, "playerId": "p1" })));
        assert_eq!(
            notes(game.state()),
            vec!["rattle:death:p1", "bauble:death", "rattle:death:p2"]
        );
        let collected: Vec<bool> = [&own, &pop, &foe]
            .iter()
            .map(|card| {
                game.state().players[card.owner]
                    .graveyard
                    .iter()
                    .any(|c| c.id == card.id)
            })
            .collect();
        assert_eq!(collected, vec![true, true, true]);
        assert!(replays_to(&game.start, &game.log, game.state()));

        let mut quiet = playing("dc-backrow-no-death");
        let one = put(&mut quiet, &bauble.id, slot(P1, Row::Backrow, 1), Default::default());
        let two = put(&mut quiet, &bauble.id, slot(P1, Row::Backrow, 2), Default::default());
        let mut sink = sink_for(&mut quiet);
        let mut ctx = make_context(&mut sink, None, by(P1));
        apply_effects(
            &[
                bounce(json_as(json!({ "target": { "of": "instance", "instanceId": one.id } }))),
                exile(json_as(json!({ "target": { "of": "instance", "instanceId": two.id } }))),
            ],
            &mut ctx,
        );
        drop(ctx);
        state_check(&mut sink);
        assert!(notes(sink.state).is_empty());
    }

    #[test]
    fn r461_a_sacrificed_backrow_card_fires_its_death_one_exiled_instead_of_reaching_its_graveyard_does_not() {
        let mut state = playing("dc-backrow-sacrifice");
        let pop = put(&mut state, &bauble.id, slot(P1, Row::Backrow, 1), Default::default());
        let mut sink = sink_for(&mut state);
        sacrifice_now(&mut sink, &pop);
        assert_eq!(notes(sink.state), vec!["bauble:death"]);

        let mut walled = playing("dc-backrow-void");
        put(&mut walled, &voidwalker.id, slot(P2, Row::Units, 1), Default::default());
        let gone = put(&mut walled, &bauble.id, slot(P1, Row::Backrow, 1), Default::default());
        put(&mut walled, &grunt.id, slot(P1, Row::Units, 1), Default::default());
        by_id_mut(&mut walled, &gone.id).marked_destroyed = Some(true);
        let mut void_sink = sink_for(&mut walled);
        state_check(&mut void_sink);
        assert!(notes(void_sink.state).is_empty());
        let exiled: Vec<&str> = void_sink.state.players.p1.exile.iter().map(|card| card.id.as_str()).collect();
        assert_eq!(exiled, vec![gone.id.as_str()]);
        assert!(events_of_type(void_sink.events, GameEventType::Destroyed).is_empty());
    }
}
