//! R452 meets E19 (R471, R689): "Place N Plague Counters" inside a random cast (Classic+ #47 Jogg's Box
//! casting Classic #70 Book of Plague) asks its caster nothing — every placement goes on one random
//! permanent — as a random cast makes every choice at random. Through fixture cards; the real cards'
//! tests cover the same case again (packages/cards/test/classic-plus/047-joggs-box.test.ts).
//!
//! Port of `packages/engine/test/effects-plague-random-cast.test.ts`.

use jackioh_engine::testkit::*;

use jackioh_engine::catalog::registered_catalog;
use jackioh_engine::effects::{CastRandomArgs, CastRandomCount, CastRandomQuery, cast_random, place_plague_tokens};
use jackioh_engine::resolve::{HookOptions, apply_effects, make_context};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::{CardScripts, EngineSink, Script, hook};
use jackioh_engine::scripts::registered_scripts;
use jackioh_engine::state::{GameState, new_instance};

use super::fixtures::harness::{new_game, put, set_library, slot};

fn spell(id: &str) -> CardDef {
    json_as(json!({
        "id": id,
        "index": id,
        "name": id,
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "keywords": [], "text": id },
        "radiant": { "keywords": [], "text": id },
    }))
}

fn plague_spell() -> CardDef {
    spell("pr-plague-fixture")
}

fn box_() -> CardDef {
    spell("pr-box-fixture")
}

const TOKENS: i32 = 3;

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in [plague_spell(), box_()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let placing = Script {
        cry: Some(hook(|_ctx| vec![place_plague_tokens(json_as(json!({ "count": TOKENS })))])),
        ..Script::default()
    };
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        plague_spell().id,
        CardScripts {
            base: placing.clone(),
            radiant: placing,
        },
    );
    register_scripts(scripts);
    state.turn = 4;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    set_library(&mut state, PlayerId::P1, &[] as &[&str]);
    state
}

fn cast_from_box(state: &mut GameState) {
    // TS built the sink first and the casting card second; the sink touches no id, so the order of
    // the two is the same game.
    let self_ = new_instance(state, &box_().id, PlayerId::P1, Zone::Resolving { player: PlayerId::P1 });
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut events = Vec::new();
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            Some(&self_),
            HookOptions {
                controller: Some(PlayerId::P1),
                ..Default::default()
            },
        );
        apply_effects(
            &[cast_random(CastRandomArgs {
                query: CastRandomQuery::Fixed(json_as(json!({ "defId": plague_spell().id }))),
                count: Some(CastRandomCount::Fixed(1)),
                radiant: None,
                how: json_as(json!({})),
            })],
            &mut ctx,
        );
    }
    state.rng_cursor = rng.cursor();
}

fn tokens_on_field(state: &GameState) -> i32 {
    [&state.players.p1, &state.players.p2]
        .iter()
        .flat_map(|side| side.units.iter().flatten().flatten())
        .map(|card| card.counters.plague.unwrap_or(0))
        .sum()
}

mod r452_r471_placements_inside_a_random_cast {
    use super::*;

    #[test]
    fn r452_r689_its_caster_is_never_asked_every_placement_lands_on_one_random_permanent_at_once() {
        let mut state = game("plague-random-cast");
        put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({}));
        put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({}));
        cast_from_box(&mut state);
        assert!(state.pending.is_none());
        assert_eq!(tokens_on_field(&state), TOKENS);
    }

    #[test]
    fn r129_with_no_permanent_on_the_field_the_placements_fizzle_and_nothing_is_asked() {
        let mut state = game("plague-random-empty");
        cast_from_box(&mut state);
        assert!(state.pending.is_none());
        assert_eq!(tokens_on_field(&state), 0);
    }

    #[test]
    fn r471_outside_a_random_cast_the_same_spell_still_asks_its_caster() {
        let mut state = game("plague-asked");
        put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({}));
        let self_ = new_instance(
            &mut state,
            &plague_spell().id,
            PlayerId::P1,
            Zone::Resolving { player: PlayerId::P1 },
        );
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut events = Vec::new();
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            let mut ctx = make_context(
                &mut sink,
                Some(&self_),
                HookOptions {
                    controller: Some(PlayerId::P1),
                    ..Default::default()
                },
            );
            apply_effects(&[place_plague_tokens(json_as(json!({ "count": TOKENS })))], &mut ctx);
        }
        assert_eq!(state.pending.as_ref().map(|pending| pending.player_id), Some(PlayerId::P1));
    }
}
