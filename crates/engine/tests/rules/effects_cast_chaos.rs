//! R28 meets R452: a Call to Chaos that a random cast makes (Classic+ #47 Jogg's Box's `castRandom`) is a
//! cast of the Call to Chaos chain, the first, so its own recursion stops at CALL_TO_CHAOS_CHAIN_CAP casts
//! in all. Through fixture cards; the real card's test covers the same case again
//! (packages/cards/test/classic-plus/047-joggs-box.test.ts).
//!
//! Port of `packages/engine/test/effects-cast-chaos.test.ts`.

use std::sync::mpsc;

use jackioh_engine::effects::cast_random;
use jackioh_engine::subsystems::call_to_chaos::{cast_random_call_to_chaos, chaos_chain_of};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, new_game, set_library};

/// TS `spell(id, extra)`: a Common 1-cost Spell named after its id, `extra`'s keys laid over it.
fn spell(id: &str, extra: Value) -> CardDef {
    let mut def = json!({
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
    });
    if let (Some(def), Some(extra)) = (def.as_object_mut(), extra.as_object()) {
        for (key, value) in extra {
            def.insert(key.clone(), value.clone());
        }
    }
    json_as(def)
}

const CHAOS: &str = "cc-chaos-fixture";
const BOX: &str = "cc-box-fixture";

fn chaos() -> CardDef {
    spell(CHAOS, json!({ "tags": ["Call to Chaos"], "rarity": "Legendary", "cost": 4 }))
}

fn box_def() -> CardDef {
    spell(BOX, json!({ "rarity": "Legendary", "cost": 4 }))
}

/// A game whose only Call to Chaos runs `cry`, and where the box's pool is that card alone.
fn game(seed: &str, cry: Hook) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    catalog.insert(CHAOS.to_string(), chaos());
    catalog.insert(BOX.to_string(), box_def());
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        CHAOS.to_string(),
        CardScripts {
            base: Script { cry: Some(cry.clone()), ..Script::default() },
            radiant: Script { cry: Some(cry), ..Script::default() },
        },
    );
    register_scripts(scripts);
    state.turn = 4;
    state.active = PlayerId::P1;
    state.phase = Phase::Main;
    set_library(&mut state, PlayerId::P1, &[]);
    state
}

fn cast_from_box(state: &mut GameState) -> Vec<GameEvent> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let self_ = new_instance(state, BOX, PlayerId::P1, Zone::Resolving { player: PlayerId::P1 });
    {
        let sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(sink, Some(self_), HookOptions { controller: Some(PlayerId::P1), ..Default::default() });
        apply_effects(
            &[cast_random(json_as(json!({ "query": { "tags": ["Call to Chaos"] }, "count": 1 })))],
            &mut ctx,
        );
    }
    state.rng_cursor = rng.cursor();
    events
}

mod cast_random_and_r28_s_call_to_chaos_chain_classic_47 {
    use super::*;

    #[test]
    fn r28_a_call_to_chaos_a_random_cast_makes_is_the_chain_s_first_cast() {
        let (depths, seen) = mpsc::channel::<i32>();
        let mut state = game(
            "box-chaos-first",
            hook(move |ctx| {
                depths.send(chaos_chain_of(ctx.live_self())).unwrap();
                vec![]
            }),
        );
        cast_from_box(&mut state);
        assert_eq!(seen.try_iter().collect::<Vec<i32>>(), vec![1]);
    }

    #[test]
    fn r28_a_chain_begun_by_a_random_cast_stops_at_call_to_chaos_chain_cap_casts_in_all() {
        let (depths, seen) = mpsc::channel::<i32>();
        let mut state = game(
            "box-chaos-chain",
            hook(move |ctx| {
                depths.send(chaos_chain_of(ctx.live_self())).unwrap();
                vec![cast_random_call_to_chaos()]
            }),
        );
        let events = cast_from_box(&mut state);
        assert_eq!(events_of_type(&events, GameEventType::CardPlayed).len(), CALL_TO_CHAOS_CHAIN_CAP as usize);
        assert_eq!(seen.try_iter().collect::<Vec<i32>>(), (1..=CALL_TO_CHAOS_CHAIN_CAP).collect::<Vec<i32>>());
    }
}
