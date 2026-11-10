//! Random cards are unique within one effect until their pool runs out (R3563000, issue #3563,
//! Stinkyscape+ E9): every effect that generates random cards draws unique within its run, repeating
//! only after the whole pool has come once. The rule ships behind `UNIQUE_RANDOM_DRAWS`, off until the
//! Stinkyscape+ release, so every test but the last switches it on through the testkit's override.
//!
//! Fixtures: the vanilla catalog plus `ud-` Units and Spells at cost 9, drawn through each verb.

use jackioh_engine::catalog::{pick_generated, query, unique_draws};
use jackioh_engine::config::{MarketLot, MarketPool, UNIQUE_RANDOM_DRAWS};
use jackioh_engine::effects::{
    add_random_from_catalog, cast_random, fill_board_random, shuffle_random_from_catalog, summon_random,
    transform_random,
};
use jackioh_engine::marketplace::roll_offers;
use jackioh_engine::rng::Rng;
use jackioh_engine::testkit::*;
use jackioh_engine::zones::{card_at, slots_of};

use crate::rules::fixtures::catalog::unit_def;
use crate::rules::fixtures::harness::{act, both, in_hand, new_game, past_the_mulligans, run_as_p1};

const P1: PlayerId = PlayerId::P1;
const SEEDS: usize = 60;

/// A `ud-<n>` Unit at cost 9, off every other fixture pool's cost.
fn unit(n: usize) -> CardDef {
    unit_def(35630 + n as i32, json!({ "id": format!("ud-{n}"), "cost": 9 }))
}

/// A `ud-spell-<n>` Spell at cost 9 that does nothing.
fn spell(n: usize) -> CardDef {
    json_as(json!({
        "id": format!("ud-spell-{n}"),
        "index": (35640 + n).to_string(),
        "name": format!("Unique Draws Spell {n}"),
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 9,
        "base": { "keywords": [], "text": format!("ud-spell-{n}") },
        "radiant": { "keywords": [], "text": format!("ud-spell-{n}") },
    }))
}

/// A cost-0 Spell `ud-trio` that does nothing on its own face.
fn trio_def() -> CardDef {
    json_as(json!({
        "id": "ud-trio",
        "index": "35650",
        "name": "Unique Draws Trio",
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": "ud-trio" },
        "radiant": { "keywords": [], "text": "ud-trio" },
    }))
}

/// A cost-0 Spell `ud-trio` whose Cry is three separate `summon_random` over `pool(3)`.
fn trio_script() -> Script {
    Script {
        cry: Some(hook(|_| {
            vec![
                summon_random(json_as(json!({ "query": pool(3) }))),
                summon_random(json_as(json!({ "query": pool(3) }))),
                summon_random(json_as(json!({ "query": pool(3) }))),
            ]
        })),
        ..Script::default()
    }
}

/// A setup game whose catalog holds `size` `ud-` Units and the three `ud-spell-` Spells, p1's hand
/// empty. Spells carry default scripts, so casts of them resolve.
fn game(seed: &str, size: usize) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for n in 1..=size {
        let def = unit(n);
        catalog.insert(def.id.clone(), def);
    }
    for n in 1..=3 {
        let def = spell(n);
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    for n in 1..=3 {
        scripts.insert(format!("ud-spell-{n}"), both(Script::default()));
    }
    register_scripts(scripts);
    state.players[P1].hand.clear();
    state
}

/// A pool naming the first `size` `ud-` Units by id.
fn pool(size: usize) -> Value {
    json!({ "defId": (1..=size).map(|n| format!("ud-{n}")).collect::<Vec<_>>() })
}

/// A pool naming the three `ud-spell-` Spells by id.
fn spell_pool() -> Value {
    json!({ "defId": (1..=3).map(|n| format!("ud-spell-{n}")).collect::<Vec<_>>() })
}

/// The def ids of the `AddedToHand` events, in order.
fn added(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::AddedToHand { def_id, .. } => Some(def_id.clone()),
            _ => None,
        })
        .collect()
}

/// The def ids on p1's unit zones, in slot order.
fn field_units(state: &GameState) -> Vec<String> {
    slots_of(P1, Row::Units)
        .iter()
        .filter_map(|slot| card_at(state, *slot))
        .map(|card| card.def_id.clone())
        .collect()
}

fn distinct(ids: &[String]) -> bool {
    let mut sorted = ids.to_vec();
    sorted.sort();
    sorted.dedup();
    sorted.len() == ids.len()
}

#[test]
fn r3563000_an_add_of_n_from_a_pool_of_at_least_n_gives_n_different_cards() {
    for n in 0..SEEDS {
        let seed = format!("r3563000-add-{n}");
        let run = || {
            let mut state = game(&seed, 6);
            let events = run_as_p1(
                &mut state,
                add_random_from_catalog(json_as(json!({ "query": pool(6), "count": 4 }))),
            );
            let four = added(&events);
            let events = run_as_p1(
                &mut state,
                add_random_from_catalog(json_as(json!({ "query": pool(6), "count": 6 }))),
            );
            (four, added(&events))
        };
        let (four, six) = with_unique_draws(true, run);
        assert_eq!(four.len(), 4, "{seed}");
        assert!(distinct(&four), "{seed}: {four:?}");
        assert_eq!(six.len(), 6, "{seed}");
        assert!(distinct(&six), "{seed}: {six:?}");
        // The same seed draws the same cards again.
        assert_eq!((four, six), with_unique_draws(true, run), "{seed}");
    }
}

#[test]
fn r3563000_a_smaller_pool_repeats_only_after_every_card_came_once() {
    let whole = vec!["ud-1".to_string(), "ud-2".to_string(), "ud-3".to_string()];
    let mut sevenths: Vec<String> = Vec::new();
    for n in 0..SEEDS {
        let seed = format!("r3563000-repeat-{n}");
        let drawn = with_unique_draws(true, || {
            let mut state = game(&seed, 3);
            let events = run_as_p1(
                &mut state,
                add_random_from_catalog(json_as(json!({ "query": pool(3), "count": 7 }))),
            );
            added(&events)
        });
        assert_eq!(drawn.len(), 7, "{seed}");
        let mut first = drawn[0..3].to_vec();
        first.sort();
        assert_eq!(first, whole, "{seed}: the first three cover the pool");
        let mut second = drawn[3..6].to_vec();
        second.sort();
        assert_eq!(second, whole, "{seed}: the next three cover it again");
        sevenths.push(drawn[6].clone());
    }
    sevenths.sort();
    sevenths.dedup();
    assert_eq!(sevenths, whole, "the seventh card takes every value");
}

#[test]
fn r3563000_sibling_summons_of_one_cry_share_the_draw() {
    for n in 0..SEEDS {
        let seed = format!("r3563000-trio-{n}");
        let ids = with_unique_draws(true, || {
            let mut state = past_the_mulligans(&seed);
            let mut catalog = registered_catalog().clone();
            for n in 1..=3 {
                let def = unit(n);
                catalog.insert(def.id.clone(), def);
            }
            let trio = trio_def();
            catalog.insert(trio.id.clone(), trio);
            register_catalog(catalog);
            let mut scripts = registered_scripts().clone();
            scripts.insert("ud-trio".to_string(), both(trio_script()));
            register_scripts(scripts);
            let card = state::new_instance(&mut state, "ud-trio", P1, Zone::Hand { player: P1 });
            state.players[P1].hand.push(card.clone());
            let next = act(
                &state,
                json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }),
            );
            let mut ids = field_units(&next);
            ids.sort();
            ids
        });
        assert_eq!(
            ids,
            vec!["ud-1".to_string(), "ud-2".to_string(), "ud-3".to_string()],
            "{seed}"
        );
    }
}

#[test]
fn r3563000_fill_shuffle_transform_and_cast_draw_unique() {
    for n in 0..SEEDS {
        let seed = format!("r3563000-verbs-{n}");
        with_unique_draws(true, || {
            // `fill_board_random` from six: every filled zone holds a different id.
            let mut state = game(&seed, 6);
            run_as_p1(
                &mut state,
                fill_board_random(json_as(json!({ "query": pool(6) }))),
            );
            let filled = field_units(&state);
            assert!(!filled.is_empty(), "{seed}: the board fills");
            assert!(distinct(&filled), "{seed}: {filled:?}");

            // `shuffle_random_from_catalog` of four from six: the new library cards differ.
            let mut state = game(&seed, 6);
            run_as_p1(
                &mut state,
                shuffle_random_from_catalog(json_as(json!({ "query": pool(6), "count": 4 }))),
            );
            let shuffled: Vec<String> = state.players[P1]
                .library
                .iter()
                .filter(|card| card.def_id.starts_with("ud-"))
                .map(|card| card.def_id.clone())
                .collect();
            assert_eq!(shuffled.len(), 4, "{seed}");
            assert!(distinct(&shuffled), "{seed}: {shuffled:?}");

            // `transform_random` over four hand cards: four different ids.
            let mut state = game(&seed, 6);
            in_hand(&mut state, "fx-1", P1, 4);
            run_as_p1(
                &mut state,
                transform_random(json_as(
                    json!({ "scope": { "side": "self", "zones": ["hand"] }, "query": pool(6) }),
                )),
            );
            let hand: Vec<String> = state.players[P1]
                .hand
                .iter()
                .map(|card| card.def_id.clone())
                .collect();
            assert_eq!(hand.len(), 4, "{seed}");
            assert!(distinct(&hand), "{seed}: {hand:?}");

            // `cast_random` of three from three: the graveyard holds each spell once.
            let mut state = past_the_mulligans(&seed);
            let mut catalog = registered_catalog().clone();
            for n in 1..=3 {
                let def = spell(n);
                catalog.insert(def.id.clone(), def);
            }
            register_catalog(catalog);
            let mut scripts = registered_scripts().clone();
            for n in 1..=3 {
                scripts.insert(format!("ud-spell-{n}"), both(Script::default()));
            }
            register_scripts(scripts);
            run_as_p1(
                &mut state,
                cast_random(json_as(json!({ "query": spell_pool(), "count": 3 }))),
            );
            let mut cast: Vec<String> = state.players[P1]
                .graveyard
                .iter()
                .map(|card| card.def_id.clone())
                .collect();
            cast.sort();
            assert_eq!(
                cast,
                vec![
                    "ud-spell-1".to_string(),
                    "ud-spell-2".to_string(),
                    "ud-spell-3".to_string()
                ],
                "{seed}"
            );
        });
    }
}

/// Two overlapping lots of the `ud-` ids: no roll offers a card twice.
const LOTS: &[MarketLot] = &[
    MarketLot {
        pool: MarketPool::Ids(&["ud-1", "ud-2", "ud-3", "ud-4"]),
        count: 2,
    },
    MarketLot {
        pool: MarketPool::Ids(&["ud-3", "ud-4", "ud-5", "ud-6"]),
        count: 2,
    },
];

#[test]
fn r3563000_a_marketplace_roll_offers_no_card_twice() {
    for n in 0..SEEDS {
        let seed = format!("r3563000-market-{n}");
        with_unique_draws(true, || {
            let state = game(&seed, 6);
            let mut rng = Rng::new(&seed, 0);
            let offers = roll_offers(&state, &mut rng, LOTS, P1);
            let ids: Vec<String> = offers.into_iter().map(|offer| offer.def_id).collect();
            assert_eq!(ids.len(), 4, "{seed}");
            assert!(distinct(&ids), "{seed}: {ids:?}");
        });
    }
}

#[test]
fn r3563000_off_until_the_release_draws_as_before() {
    assert!(!UNIQUE_RANDOM_DRAWS);
    assert!(!unique_draws());
    for n in 0..SEEDS {
        let seed = format!("r3563000-off-{n}");
        let mut state = game(&seed, 3);
        let defs = query(&json_as(pool(3)));
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut expected: Vec<String> = Vec::new();
        for _ in 0..7 {
            expected.push(
                pick_generated(&mut rng, &defs, Some(&state))
                    .map(|def| def.id.clone())
                    .expect("the pool holds cards"),
            );
        }
        let events = run_as_p1(
            &mut state,
            add_random_from_catalog(json_as(json!({ "query": pool(3), "count": 7 }))),
        );
        assert_eq!(added(&events), expected, "{seed}");
    }
    let repeats = (0..SEEDS).any(|n| {
        let seed = format!("r3563000-off-repeat-{n}");
        let mut state = game(&seed, 3);
        let events = run_as_p1(
            &mut state,
            add_random_from_catalog(json_as(json!({ "query": pool(3), "count": 2 }))),
        );
        !distinct(&added(&events))
    });
    assert!(repeats, "two draws from three repeat on some seed");
}
