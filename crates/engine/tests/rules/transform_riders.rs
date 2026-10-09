//! Transform riders and the hand-wide transform (docs/meditative-set.md, group B's Systems; MD-B10,
//! R921): `transform` and `transform_random` carry `costOverride` (a price, only in a hand or a
//! deck) and `chinese` (anywhere). With `scope` and `same: true`, `transform_random` replaces every
//! card of the scope with new cards of one shared definition — drawn from the definitions every old
//! card could take — each in its place (R31, R671; an Immutable hand card too, R35), each a card
//! generated into a hand or a deck for its own R673 Glitch roll. Without them it draws exactly as
//! before; an empty scope draws nothing (R129). The `arrives` rider is proved with `enters_hand`
//! (R925).

use jackioh_engine::config::{GLITCH_DEF_ID, GLITCH_ODDS_DENOMINATOR};
use jackioh_engine::effects::{transform, transform_random};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::spell_def;
use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{in_hand, put, set_library, slot};
use crate::rules::fixtures::instance_data::{body, instance_game, stoic};

const P1: PlayerId = PlayerId::P1;

fn game(seed: &str) -> GameState {
    let mut state = instance_game(seed, None);
    state.turn = 5;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

/// The effect applied for p1 on a sink over `state`, the rng cursor written back; its events.
fn run(state: &mut GameState, effect: Effect) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(P1),
                ..HookOptions::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id)
        .cloned()
        .unwrap_or_else(|| panic!("no card {id} in the state"))
}

fn first(cards: Vec<CardInstance>) -> CardInstance {
    cards.into_iter().next().expect("a card")
}

mod r921_transform_riders_md_b10 {
    use super::*;

    #[test]
    fn r921_same_replaces_a_whole_hand_with_one_definition_in_place() {
        let mut state = game("riders-same");
        // The hand in a known order: plain, the Immutable stoic, body.
        let before: Vec<String> = [&plain.id, &stoic.id, &body.id]
            .into_iter()
            .map(|def| first(in_hand(&mut state, def, P1, 1)).id)
            .collect();
        run(
            &mut state,
            transform_random(json_as(json!({
                "scope": { "side": "self", "zones": ["hand"] },
                "same": true,
                "query": { "defId": ["id-plain", "id-body"] },
                "radiant": true,
                "costOverride": 0,
            }))),
        );
        let hand = &state.players.p1.hand;
        assert_eq!(hand.len(), 3);
        // One definition for the whole hand, from the pool — the Immutable stoic replaced too.
        let defs: Vec<&str> = hand.iter().map(|card| card.def_id.as_str()).collect();
        assert!(defs.iter().all(|def| *def == defs[0]), "{defs:?}");
        assert!(defs[0] == plain.id || defs[0] == body.id);
        for card in hand {
            // New instances, Radiant, costing (0), in place.
            assert!(!before.contains(&card.id));
            assert!(card.radiant);
            assert_eq!(card.cost_override, Some(0));
        }
    }

    #[test]
    fn r921_an_empty_scope_draws_nothing() {
        let mut state = game("riders-empty");
        in_hand(&mut state, &plain.id, P1, 2);
        let cursor = state.rng_cursor;
        // No CN card in the hand: the scope is empty, so no definition is drawn (R129).
        run(
            &mut state,
            transform_random(json_as(json!({
                "scope": { "side": "self", "zones": ["hand"], "tags": ["CN"] },
                "same": true,
                "query": { "defId": ["id-plain", "id-body"] },
                "radiant": true,
                "costOverride": 0,
            }))),
        );
        assert_eq!(state.rng_cursor, cursor);
        assert_eq!(state.players.p1.hand.len(), 2);
        assert!(state.players.p1.hand.iter().all(|card| card.def_id == plain.id));
    }

    #[test]
    fn r921_cost_override_prices_only_hand_or_deck_replacements_and_chinese_marks_any() {
        // On the field the price is refused and the flag lands.
        let mut state = game("riders-field");
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        run(
            &mut state,
            transform(json_as(
                json!({ "instanceId": unit.id, "defId": body.id, "costOverride": 3, "chinese": true }),
            )),
        );
        let new_card = card_at(&state, slot(P1, Row::Units, 1))
            .cloned()
            .expect("the new card");
        assert_eq!(new_card.def_id, body.id);
        assert_eq!(new_card.cost_override, None);
        assert_eq!(new_card.chinese, Some(true));

        // In a deck the same riders both land.
        let mut state = game("riders-deck");
        let deck = first(set_library(&mut state, P1, &[plain.id.as_str()]));
        run(
            &mut state,
            transform(json_as(
                json!({ "instanceId": deck.id, "defId": body.id, "costOverride": 3, "chinese": true }),
            )),
        );
        let replaced = live(&state, &state.players.p1.library[0].id);
        assert_eq!(replaced.def_id, body.id);
        assert_eq!(replaced.cost_override, Some(3));
        assert_eq!(replaced.chinese, Some(true));
    }

    #[test]
    fn r921_transform_takes_the_riders() {
        let mut state = game("riders-one");
        let held = first(in_hand(&mut state, &plain.id, P1, 1));
        run(
            &mut state,
            transform(json_as(
                json!({ "instanceId": held.id, "defId": body.id, "costOverride": 5, "chinese": true }),
            )),
        );
        let new_card = state.players.p1.hand.last().cloned().expect("the new card");
        assert_eq!(new_card.def_id, body.id);
        assert_ne!(new_card.id, held.id);
        assert_eq!(new_card.cost_override, Some(5));
        assert_eq!(new_card.chinese, Some(true));
    }

    /// A Glitch the catalog holds, so `glitch_or_not` can name it (glitch.rs's shape); half the
    /// System plays it takes to force one, so each held replacement rolls on its own.
    fn glitch_def() -> CardDef {
        let mut card = serde_json::to_value(spell_def(
            9001,
            json!({
                "id": GLITCH_DEF_ID,
                "index": "T-glitch",
                "name": "Glitch",
                "set": "Classic",
                "type": "Unit",
                "tags": ["Token"],
                "rarity": "Token",
                "token": true,
            }),
        ))
        .expect("serialisable");
        if let Some(card) = card.as_object_mut() {
            card.insert("cost".to_string(), json!(0));
            card.insert(
                "base".to_string(),
                json!({ "attack": 0, "health": 1, "keywords": [], "text": "" }),
            );
            card.insert(
                "radiant".to_string(),
                json!({ "attack": 0, "health": 2, "keywords": [], "text": "" }),
            );
        }
        json_as(card)
    }

    fn glitchy_game(seed: &str) -> GameState {
        let mut state = game(seed);
        let mut catalog = registered_catalog().clone();
        catalog.insert(GLITCH_DEF_ID.to_string(), glitch_def());
        register_catalog(catalog);
        state.system_plays = Some(GLITCH_ODDS_DENOMINATOR / 2);
        state
    }

    #[test]
    fn r921_each_held_replacement_rolls_glitch_on_its_own() {
        // One shared draw, three independent Glitch rolls: over seeds, Glitch replacements and
        // shared-definition replacements both occur — and in a mixed hand every non-Glitch card is
        // the one shared definition.
        let mut saw_glitch = false;
        let mut saw_shared = false;
        let mut saw_mixed = false;
        for n in 0..32 {
            let mut state = glitchy_game(&format!("riders-glitch-{n}"));
            for def in [&plain.id, &stoic.id, &body.id] {
                in_hand(&mut state, def, P1, 1);
            }
            run(
                &mut state,
                transform_random(json_as(json!({
                    "scope": { "side": "self", "zones": ["hand"] },
                    "same": true,
                    "query": { "defId": ["id-plain", "id-body"] },
                }))),
            );
            let hand = &state.players.p1.hand;
            assert_eq!(hand.len(), 3);
            let glitches = hand.iter().filter(|card| card.def_id == GLITCH_DEF_ID).count();
            let rest: Vec<&str> = hand
                .iter()
                .filter(|card| card.def_id != GLITCH_DEF_ID)
                .map(|card| card.def_id.as_str())
                .collect();
            saw_glitch = saw_glitch || glitches > 0;
            saw_shared = saw_shared || !rest.is_empty();
            if glitches > 0 && !rest.is_empty() {
                saw_mixed = true;
                assert!(rest.iter().all(|def| *def == rest[0]), "{rest:?}");
            }
        }
        assert!(saw_glitch, "some replacement is Glitch over 32 seeds");
        assert!(saw_shared, "some replacement keeps the shared draw over 32 seeds");
        assert!(
            saw_mixed,
            "some hand mixes Glitch and shared replacements over 32 seeds"
        );
    }
}
