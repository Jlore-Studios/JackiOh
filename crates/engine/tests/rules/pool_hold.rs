//! A lesson's random pools (SPEC §5.1, §9.10; R2201001): the handicap's `poolHold`. While either
//! seat sets it, every random pool the game draws from, the weighted tables and the Marketplace
//! included, leaves out Legendary and Mythic cards. An unheld game still offers them.
//!
//! The hold is read through a thread-local that `reduce`, `reduce_unpiloted` and `begin_game` set
//! for their one call and put back after it (`catalog::with_pool_hold`), so held and unheld games
//! stepped one after the other on one thread, a nested step and a panic all read their own game's.
//!
//! Fixtures: the vanilla catalog plus this file's `ph-` cards: cost-9 Units and Spells of every
//! rarity (the pools), and four cost-0 Spells whose Cry draws from them through each verb.

use std::panic::{AssertUnwindSafe, catch_unwind};

use jackioh_engine::catalog::{HELD_RARITIES, pool_held, roll_weighted, with_pool_hold};
use jackioh_engine::config::{MarketLot, MarketPool, RollOdds};
use jackioh_engine::effects::{add_random_from_catalog, cast_random, discover_from_catalog, summon_random};
use jackioh_engine::marketplace::roll_offers;
use jackioh_engine::rng::Rng;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::{unit_def, vanilla_deck};
use crate::rules::fixtures::harness::{act, both, setup_catalog};

const RARITIES: [&str; 5] = ["Common", "Rare", "Epic", "Legendary", "Mythic"];
const SEEDS: usize = 60;

fn pool_unit(rarity: &str, index: i32) -> CardDef {
    unit_def(
        index,
        json!({ "id": format!("ph-unit-{}", rarity.to_lowercase()), "rarity": rarity, "cost": 9 }),
    )
}

fn spell(id: &str, index: i32, rarity: &str, cost: i32) -> CardDef {
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (pool hold)"),
        "set": "Core",
        "type": "Spell",
        "tags": [],
        "rarity": rarity,
        "token": false,
        "cost": cost,
        "base": { "keywords": [], "text": id },
        "radiant": { "keywords": [], "text": id },
    }))
}

fn verbs() -> Vec<(&'static str, Script)> {
    vec![
        (
            "ph-add",
            Script {
                cry: Some(hook(|_| {
                    vec![add_random_from_catalog(json_as(
                        json!({ "query": { "type": "Unit", "cost": 9 }, "count": 3 }),
                    ))]
                })),
                ..Script::default()
            },
        ),
        (
            "ph-summon",
            Script {
                cry: Some(hook(|_| {
                    vec![summon_random(json_as(
                        json!({ "query": { "type": "Unit", "cost": 9 } }),
                    ))]
                })),
                ..Script::default()
            },
        ),
        (
            "ph-discover",
            Script {
                cry: Some(hook(|_| {
                    vec![discover_from_catalog(json_as(
                        json!({ "step": "picked", "query": { "type": "Unit", "cost": 9 } }),
                    ))]
                })),
                resume: [("picked", hook(|_| vec![]))].into_iter().collect(),
                ..Script::default()
            },
        ),
        (
            "ph-cast",
            Script {
                cry: Some(hook(|_| {
                    vec![cast_random(json_as(
                        json!({ "query": { "type": "Spell", "cost": 9 } }),
                    ))]
                })),
                ..Script::default()
            },
        ),
    ]
}

fn register() {
    setup_catalog();
    let mut catalog = registered_catalog().clone();
    for (n, rarity) in RARITIES.iter().enumerate() {
        let unit = pool_unit(rarity, 3100 + n as i32);
        catalog.insert(unit.id.clone(), unit);
    }
    for (n, rarity) in ["Common", "Legendary", "Mythic"].iter().enumerate() {
        let id = format!("ph-spell-{}", rarity.to_lowercase());
        catalog.insert(id.clone(), spell(&id, 3110 + n as i32, rarity, 9));
    }
    for (n, (id, _)) in verbs().iter().enumerate() {
        catalog.insert(id.to_string(), spell(id, 3120 + n as i32, "Common", 0));
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    for (id, script) in verbs() {
        scripts.insert(id.to_string(), both(script));
    }
    register_scripts(scripts);
}

fn human(hold: bool) -> Handicap {
    Handicap {
        pool_hold: if hold { Some(true) } else { None },
        ..TUTORIAL_HUMAN
    }
}

/// A game in p1's main phase on turn 1, p1 the held (or unheld) human.
fn playing(seed: &str, hold: bool) -> GameState {
    register();
    let created = create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: (
            vanilla_deck(TUTORIAL_HUMAN.deck_size, 1),
            vanilla_deck(DECK_SIZE, 1),
        ),
        handicaps: Some(PerPlayerOpt {
            p1: Some(human(hold)),
            p2: None,
        }),
        ..Default::default()
    });
    let mut state = begin_game(&created).state;
    assert!(!pool_held(), "begin_game puts the hold back");
    for player in [PlayerId::P1, PlayerId::P2] {
        let keep = state.players[player]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        state = act(
            &state,
            ActionInput {
                body: ActionBody::Mulligan { keep },
                player_id: player,
            },
        );
    }
    state.players.p1.auto_end_turn = Some(false);
    state
}

fn played(seed: &str, hold: bool, verb: &str) -> GameState {
    let mut state = playing(seed, hold);
    let card = state::new_instance(
        &mut state,
        verb,
        PlayerId::P1,
        Zone::Hand { player: PlayerId::P1 },
    );
    state.players.p1.hand.push(card.clone());
    let next = act(
        &state,
        ActionInput {
            body: json_as(json!({ "type": "play", "instanceId": card.id })),
            player_id: PlayerId::P1,
        },
    );
    assert!(!pool_held(), "reduce puts the hold back");
    next
}

/// Which of the held rarities' pool cards the state names anywhere.
fn held_seen(state: &GameState) -> Vec<&'static str> {
    let text = serde_json::to_string(state).expect("serializes");
    [
        "ph-unit-legendary",
        "ph-unit-mythic",
        "ph-spell-legendary",
        "ph-spell-mythic",
    ]
    .into_iter()
    .filter(|id| text.contains(id))
    .collect()
}

fn any_pool_card_seen(state: &GameState) -> bool {
    let text = serde_json::to_string(state).expect("serializes");
    text.contains("ph-unit-") || text.contains("ph-spell-")
}

mod r2201001_a_held_game_draws_no_legendary_or_mythic_card {
    use super::*;

    #[test]
    fn r2201001_add_summon_discover_and_cast_random_never_offer_one_held_and_still_can_unheld_stepped_one_after_the_other()
     {
        for (verb, _) in verbs() {
            let mut unheld_seen = 0;
            for n in 0..SEEDS {
                let seed = format!("r2201001-{verb}-{n}");
                // A held step, then an unheld one, on the same thread.
                let held = played(&seed, true, verb);
                assert!(state::holds_pools(&held));
                assert!(any_pool_card_seen(&held), "{seed}: the held pool still draws");
                assert_eq!(held_seen(&held), Vec::<&str>::new(), "{seed}: held");
                let unheld = played(&seed, false, verb);
                assert!(!state::holds_pools(&unheld));
                if !held_seen(&unheld).is_empty() {
                    unheld_seen += 1;
                }
            }
            assert!(
                unheld_seen > 0,
                "{verb}: an unheld pool offers Legendary or Mythic cards"
            );
        }
    }

    #[test]
    fn r2201001_every_query_shape_leaves_them_out_under_the_hold_but_a_lookup_of_one_card() {
        register();
        let held = playing("r2201001-query", true);
        let unheld = playing("r2201001-query", false);
        let shapes: Vec<Value> = vec![
            json!({ "type": "Unit", "cost": 9 }),
            json!({ "cost": 9 }),
            json!({ "rarity": ["Legendary", "Mythic"] }),
            json!({}),
            json!({ "defId": ["ph-unit-common", "ph-unit-legendary"] }),
        ];
        let held_ids = |state: &GameState, shape: &Value| -> Vec<String> {
            with_pool_hold(state, || {
                query(&json_as(shape.clone()))
                    .into_iter()
                    .filter(|def| HELD_RARITIES.contains(&def.rarity))
                    .map(|def| def.id.clone())
                    .collect()
            })
        };
        for shape in &shapes {
            assert_eq!(held_ids(&held, shape), Vec::<String>::new(), "held {shape}");
            assert!(!held_ids(&unheld, shape).is_empty(), "unheld {shape}");
        }
        // A query naming one card is a lookup of it, not a pool.
        let lookup = json!({ "defId": "ph-unit-legendary" });
        assert_eq!(held_ids(&held, &lookup), vec!["ph-unit-legendary".to_string()]);
    }

    #[test]
    fn r2201001_a_weighted_table_and_the_marketplace_roll_none_under_the_hold() {
        register();
        let held = playing("r2201001-tables", true);
        let unheld = playing("r2201001-tables", false);
        const TABLE: &[RollOdds] = &[
            RollOdds {
                def_id: "ph-unit-common",
                percent: 50,
            },
            RollOdds {
                def_id: "ph-unit-legendary",
                percent: 30,
            },
            RollOdds {
                def_id: "ph-unit-mythic",
                percent: 20,
            },
        ];
        const ONLY_HELD: &[RollOdds] = &[RollOdds {
            def_id: "ph-unit-mythic",
            percent: 100,
        }];
        const LOTS: &[MarketLot] = &[MarketLot {
            pool: MarketPool::Ids(&[
                "ph-unit-common",
                "ph-unit-rare",
                "ph-unit-legendary",
                "ph-unit-mythic",
            ]),
            count: 3,
        }];
        let mut unheld_rolled = 0;
        for n in 0..SEEDS {
            let mut rng = Rng::new(&format!("r2201001-roll-{n}"), 0);
            let rolled = with_pool_hold(&held, || roll_weighted(&mut rng, TABLE, 0));
            assert_eq!(rolled, "ph-unit-common", "held roll {n}");
            let offers = with_pool_hold(&held, || roll_offers(&held, &mut rng, LOTS, PlayerId::P1));
            let ids: Vec<&str> = offers.iter().map(|offer| offer.def_id.as_str()).collect();
            assert_eq!(ids.len(), 2, "held offer {n}: {ids:?}");
            assert!(
                ids.iter()
                    .all(|id| !id.contains("legendary") && !id.contains("mythic")),
                "{ids:?}"
            );

            let rolled = with_pool_hold(&unheld, || roll_weighted(&mut rng, TABLE, 0));
            let offers = with_pool_hold(&unheld, || roll_offers(&unheld, &mut rng, LOTS, PlayerId::P1));
            if rolled != "ph-unit-common" || offers.iter().any(|offer| offer.def_id.contains("legendary")) {
                unheld_rolled += 1;
            }
        }
        assert!(unheld_rolled > 0, "an unheld table still rolls them");
        let mut rng = Rng::new("r2201001-only-held", 0);
        assert_eq!(
            with_pool_hold(&held, || roll_weighted(&mut rng, ONLY_HELD, 0)),
            ""
        );
    }
}

mod r2201001_the_hold_is_the_calls_own {
    use super::*;

    #[test]
    fn r2201001_a_nested_step_reads_its_own_games_hold_and_the_outer_one_is_put_back() {
        register();
        let held = playing("r2201001-nested", true);
        let unheld = playing("r2201001-nested", false);
        assert!(!pool_held(), "outside every call");
        with_pool_hold(&held, || {
            assert!(pool_held());
            with_pool_hold(&unheld, || assert!(!pool_held(), "the inner game is unheld"));
            assert!(pool_held(), "the outer hold is put back");
            // A whole reduce of an unheld game inside a held step, as the AI's search runs one.
            let result = reduce(
                &unheld,
                &Action::new(ActionBody::EndTurn, PlayerId::P1, "nested-1"),
            );
            assert!(result.error.is_none());
            assert!(pool_held(), "the outer hold survives the inner reduce");
        });
        with_pool_hold(&unheld, || {
            let result = reduce_unpiloted(&held, &Action::new(ActionBody::EndTurn, PlayerId::P1, "nested-2"));
            assert!(result.error.is_none());
            assert!(!pool_held(), "and the reverse");
        });
        assert!(!pool_held());
    }

    #[test]
    fn r2201001_a_panic_inside_a_held_call_puts_the_hold_back() {
        register();
        let held = playing("r2201001-panic", true);
        let caught = catch_unwind(AssertUnwindSafe(|| {
            with_pool_hold(&held, || {
                assert!(pool_held());
                panic!("a step that panics");
            })
        }));
        assert!(caught.is_err());
        assert!(!pool_held(), "the hold is put back after the panic");
    }

    #[test]
    fn r2201001_create_game_then_begin_game_leave_nothing_held() {
        register();
        let created = create_game(&CreateGameOptions {
            seed: "r2201001-setup".to_string(),
            decks: (
                vanilla_deck(TUTORIAL_HUMAN.deck_size, 1),
                vanilla_deck(DECK_SIZE, 1),
            ),
            handicaps: Some(PerPlayerOpt {
                p1: Some(human(true)),
                p2: None,
            }),
            ..Default::default()
        });
        assert!(!pool_held(), "create_game draws from no pool and sets nothing");
        assert_eq!(created.players.p1.handicap.and_then(|h| h.pool_hold), Some(true));
        let begun = begin_game(&created).state;
        assert!(!pool_held(), "begin_game puts it back");
        assert!(state::holds_pools(&begun));
    }
}

mod r2386001_a_held_game_ranks_no_legendary_or_mythic_card {
    use super::*;

    fn row(id: &str, wins: i32, games: i32) -> WinRateRow {
        WinRateRow {
            id: id.to_string(),
            wins,
            games,
        }
    }

    fn table(rows: Vec<WinRateRow>) -> WinRateTable {
        WinRateTable {
            patch: "v0.2.7".to_string(),
            source: WinRateSource::Provisional,
            cards: rows,
        }
    }

    #[test]
    fn r2386001_the_leader_and_the_trailer_are_the_best_and_the_worst_of_the_cards_a_held_pool_keeps() {
        register();
        let held = playing("r2386001-ends", true);
        let unheld = playing("r2386001-ends", false);
        // The Legendary leads the table and the Mythic trails it.
        register_win_rates(table(vec![
            row("ph-unit-legendary", 40, 40),
            row("ph-unit-epic", 30, 40),
            row("ph-unit-common", 20, 40),
            row("ph-unit-rare", 10, 40),
            row("ph-unit-mythic", 0, 40),
        ]));
        let ends = |state: &GameState| {
            with_pool_hold(state, || (win_rate_leader("none"), win_rate_trailer(&["none"])))
        };
        assert_eq!(
            ends(&held),
            (Some("ph-unit-epic".to_string()), Some("ph-unit-rare".to_string()))
        );
        // The same table in an unheld game, stepped after the held one: nothing is left out.
        assert_eq!(
            ends(&unheld),
            (
                Some("ph-unit-legendary".to_string()),
                Some("ph-unit-mythic".to_string())
            )
        );
        // With only held cards ranked, a held game names none; an unheld one still does.
        register_win_rates(table(vec![
            row("ph-unit-legendary", 40, 40),
            row("ph-unit-mythic", 0, 40),
        ]));
        assert_eq!(ends(&held), (None, None));
        assert_eq!(
            ends(&unheld),
            (
                Some("ph-unit-legendary".to_string()),
                Some("ph-unit-mythic".to_string())
            )
        );
    }
}
