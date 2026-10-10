//! `fuseCards` with a random ingredient (ME-FUSE-RANDOM): one card drawn uniformly from a scope —
//! an enemy permanent on the field (tops of piles, both rows, face-down cards included, never an
//! Immutable one) or a card of the opponent's library — fused into the kept target per R77 and
//! R102. With no ingredient, or an Immutable kept card, nothing fuses and nothing is drawn.
//! Meditative #47 饕餮 (MD-C20, R1020) is its user.

use jackioh_engine::effects::fuse_cards;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::damage_combat::{grunt, pawn, playing};
use crate::rules::fixtures::generation::{immutable, register_generation};
use crate::rules::fixtures::harness::{put, sink_for, slot};

/// TS `makeContext(sink, null, { controller })`'s options.
fn by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

fn enemy_field() -> Value {
    json!({ "side": "enemy", "zones": ["field"] })
}

fn fuse_random(state: &mut GameState, kept: &str, scope: Value) {
    let mut sink = sink_for(state);
    apply_effects(
        &[fuse_cards(json_as(json!({
            "targetInstanceId": kept,
            "randomIngredient": scope,
        })))],
        &mut make_context(&mut sink, None, by(P1)),
    );
}

/// The units and backrow cards on this side, in lane order.
fn field_count(state: &GameState, player: PlayerId) -> usize {
    let side = &state.players[player];
    side.units.iter().flatten().count() + side.backrow.iter().flatten().count()
}

mod fuse_random {
    use super::*;

    #[test]
    fn r1020_fuses_one_random_enemy_permanent_top_of_pile_face_down_included() {
        let mut ate_unit = false;
        let mut ate_trap = false;
        for n in 0..16 {
            let mut state = playing(&format!("dc-fuse-random-{n}"));
            let kept = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
            let unit = put(&mut state, &grunt.id, slot(P2, Row::Units, 1), Default::default());
            // A face-down Trap: no less an ingredient for being hidden.
            let trap = put(
                &mut state,
                &pawn.id,
                slot(P2, Row::Backrow, 1),
                Default::default(),
            );
            assert_eq!(field_count(&state, P2), 2);
            fuse_random(&mut state, &kept.id, enemy_field());
            // Exactly one enemy permanent went into the kept card, which stayed where it was.
            assert_eq!(field_count(&state, P2), 1, "{n}");
            let live = find_instance(&state, &kept.id).expect("the kept card");
            assert_eq!(live.zone.z().as_str(), "field");
            if find_instance(&state, &unit.id).is_none() {
                ate_unit = true;
                // 2/2 ate a 2/2: the stats are summed (R77).
                assert_eq!(jackioh_engine::layers::unit_view(&state, live).attack, 4);
            } else {
                assert!(find_instance(&state, &trap.id).is_none());
                ate_trap = true;
            }
        }
        assert!(ate_unit && ate_trap, "both permanents are eaten over 16 seeds");
    }

    #[test]
    fn r1020_never_an_immutable_card_and_none_fuses_nothing() {
        let mut state = playing("dc-fuse-random-immutable");
        register_generation();
        let kept = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
        put(
            &mut state,
            &immutable.id,
            slot(P2, Row::Units, 1),
            Default::default(),
        );
        let mut sink = sink_for(&mut state);
        let cursor = sink.rng.cursor();
        apply_effects(
            &[fuse_cards(json_as(json!({
                "targetInstanceId": kept.id,
                "randomIngredient": enemy_field(),
            })))],
            &mut make_context(&mut sink, None, by(P1)),
        );
        // The Immutable card stays, the kept card is untouched, and nothing was drawn.
        assert_eq!(field_count(sink.state, P2), 1);
        assert_eq!(jackioh_engine::layers::unit_view(sink.state, &kept).attack, 2);
        assert_eq!(sink.rng.cursor(), cursor);
    }

    #[test]
    fn r1020_an_immutable_kept_card_eats_nothing_and_draws_nothing() {
        let mut state = playing("dc-fuse-random-kept");
        register_generation();
        let kept = put(
            &mut state,
            &immutable.id,
            slot(P1, Row::Units, 1),
            Default::default(),
        );
        put(&mut state, &grunt.id, slot(P2, Row::Units, 1), Default::default());
        let mut sink = sink_for(&mut state);
        let cursor = sink.rng.cursor();
        apply_effects(
            &[fuse_cards(json_as(json!({
                "targetInstanceId": kept.id,
                "randomIngredient": enemy_field(),
            })))],
            &mut make_context(&mut sink, None, by(P1)),
        );
        assert_eq!(field_count(sink.state, P2), 1);
        assert_eq!(sink.rng.cursor(), cursor);
    }

    #[test]
    fn r1020_a_library_ingredient_leaves_the_library_and_its_id_stays_hidden() {
        let mut state = playing("dc-fuse-random-library");
        let kept = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), Default::default());
        let library: Vec<String> = state
            .players
            .p2
            .library
            .iter()
            .map(|card| card.id.clone())
            .collect();
        assert!(!library.is_empty());
        let mut sink = sink_for(&mut state);
        apply_effects(
            &[fuse_cards(json_as(json!({
                "targetInstanceId": kept.id,
                "randomIngredient": { "side": "enemy", "zones": ["library"] },
            })))],
            &mut make_context(&mut sink, None, by(P1)),
        );
        // Exactly one library card went into the fusion.
        let after: Vec<String> = sink
            .state
            .players
            .p2
            .library
            .iter()
            .map(|card| card.id.clone())
            .collect();
        assert_eq!(after.len(), library.len() - 1);
        let missing = library
            .iter()
            .find(|id| !after.contains(id))
            .expect("one card left");
        // Its id never reaches a view: no event names it.
        let events = serde_json::to_value(&*sink.events).expect("events are JSON");
        assert!(
            !events.to_string().contains(missing.as_str()),
            "the library card's id leaked"
        );
        assert!(find_instance(sink.state, &kept.id).is_some());
    }
}
