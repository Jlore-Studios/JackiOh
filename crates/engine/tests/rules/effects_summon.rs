//! Summon, Recruit and "fill your board" (SPEC §6.3, §3.2, §7, R64, BUILD M3-T1).
//! The fixture defs and scripts these tests need are registered here, on top of the shared fixture
//! catalog, so no shared fixture has to grow for them (CLAUDE.md, BUILD §0).
//!
//! Port of `packages/engine/test/effects-summon.test.ts`.

use jackioh_engine::effects::{
    damage, fill_board, fill_board_random, recruit, summon, summon_random_from_hand,
};
use jackioh_engine::testkit::*;
use serde::Serialize;

use super::fixtures::harness::{events_of_type, in_hand, new_game, put, set_library, slot};

// ---------------------------------------------------------------------------
// Fixture cards.
// ---------------------------------------------------------------------------

/// TS `defOfKind`. TS numbered each def from a module counter (`nextIndex`, starting at 700 and
/// raised before each def); the index each def drew is written out at its call.
fn def_of_kind(name: &str, index: i32, type_: &str, overrides: Value) -> CardDef {
    let mut def = json!({
        "id": format!("sm-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (summon)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 2, "health": 2, "keywords": [], "text": name },
        "radiant": { "attack": 4, "health": 4, "keywords": [], "text": format!("{name} radiant") },
    });
    spread(&mut def, overrides);
    json_as(def)
}

/// TS `{ ...base, ...overrides }`: the override's keys replace the base's.
fn spread(target: &mut Value, overrides: Value) {
    if let (Some(into), Value::Object(from)) = (target.as_object_mut(), overrides) {
        for (key, value) in from {
            into.insert(key, value);
        }
    }
}

/// #12-style: a Cry that pings the enemy hero, so "a summon fires no Cry" is observable (R1).
fn crier() -> CardDef {
    def_of_kind("crier", 701, "Unit", json!({}))
}

/// §3.2: a Trap enters the backrow face-down.
fn trap() -> CardDef {
    def_of_kind(
        "trap",
        702,
        "Trap",
        json!({ "base": { "keywords": [], "text": "trap" }, "radiant": { "keywords": [], "text": "trap" } }),
    )
}

/// §3.2: a Field Spell is public to both players.
fn field_spell() -> CardDef {
    def_of_kind(
        "field-spell",
        703,
        "Field Spell",
        json!({ "base": { "keywords": [], "text": "field" }, "radiant": { "keywords": [], "text": "field" } }),
    )
}

/// A Spell, which §5.1 never puts on the field.
fn spell() -> CardDef {
    def_of_kind(
        "spell",
        704,
        "Spell",
        json!({ "base": { "keywords": [], "text": "spell" }, "radiant": { "keywords": [], "text": "spell" } }),
    )
}

/// A tagged permanent for Recruit's filter.
fn felinor() -> CardDef {
    def_of_kind("felinor", 705, "Unit", json!({ "tags": ["Felinor"] }))
}

/// A cost-4 permanent, for the cost half of Recruit's filter (R65).
fn pricey() -> CardDef {
    def_of_kind("pricey", 706, "Unit", json!({ "cost": 4 }))
}

/// TS `tokenDef("rush")` from `./fixtures/catalog`, read only for its id: the vanilla fixture
/// catalog's shared Rush Token (`fx-token-<name>`), which `newGame` registers.
const RUSH_TOKEN: &str = "fx-token-rush";

fn defs() -> Vec<CardDef> {
    vec![crier(), trap(), field_spell(), spell(), felinor(), pricey()]
}

fn both(script: Script) -> CardScripts {
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts = IndexMap::new();
    scripts.insert(
        crier().id,
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![damage(json_as(
                    json!({ "to": { "of": "enemyHero" }, "amount": 3 }),
                ))]
            })),
            ..Script::default()
        }),
    );
    scripts
}

/// A fresh game whose catalog and script registry also carry this file's fixtures. TS's default
/// seed, "effects-summon", is the one every test here uses.
fn game() -> GameState {
    let mut state = new_game("effects-summon", None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    registry.extend(scripts());
    register_scripts(registry);
    state.turn = 3;
    state
}

/// TS `RunOptions = { controller?, self?, targets? }`.
#[derive(Default)]
struct RunOptions {
    controller: Option<PlayerId>,
    self_: Option<CardInstance>,
    targets: Option<Vec<Selection>>,
}

/// Apply one effect the way a script's hook would, and hand back the events it emitted. TS's
/// `sinkFor(state)` is the sink built here: its rng starts at the state's cursor, as reduce does.
fn run(state: &mut GameState, effect: Effect, options: RunOptions) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            options.self_.as_ref(),
            HookOptions {
                controller: Some(options.controller.unwrap_or(PlayerId::P1)),
                targets: options.targets,
                ..HookOptions::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

/// The def id in each of the row's five lanes, `null` where it is empty (as JSON, so an assertion
/// reads like TS's array).
fn lanes_of(state: &GameState, player: PlayerId, row: Row) -> Value {
    to_json(
        [1, 2, 3, 4, 5]
            .iter()
            .map(|lane| card_at(state, slot(player, row, *lane)).map(|card| card.def_id.clone()))
            .collect::<Vec<Option<String>>>(),
    )
}

fn in_graveyard(state: &mut GameState, def_id: &str, player: PlayerId) -> CardInstance {
    let card = new_instance(state, def_id, player, Zone::Graveyard { player });
    state.players[player].graveyard.push(card.clone());
    card
}

/// TS `string[]` for the harness's `setLibrary`, from the ids as written.
fn owned(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

/// TS held the live instance and read it after an effect; Rust reads the card again by id.
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id)
        .cloned()
        .unwrap_or_else(|| panic!("no card {id} in the state"))
}

fn to_json<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

fn summoned_lanes(events: &[GameEvent]) -> Vec<i32> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Summoned { lane, .. } => Some(*lane),
            _ => None,
        })
        .collect()
}

fn summoned_ids(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Summoned { instance_id, .. } => Some(instance_id.clone()),
            _ => None,
        })
        .collect()
}

fn summoned_def_ids(events: &[GameEvent]) -> Vec<String> {
    events
        .iter()
        .filter_map(|event| match event {
            GameEvent::Summoned { def_id, .. } => Some(def_id.clone()),
            _ => None,
        })
        .collect()
}

fn ids_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn def_ids_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.def_id.clone()).collect()
}

// ---------------------------------------------------------------------------
// summon
// ---------------------------------------------------------------------------

mod summon_s6_3_m3_t1 {
    use super::*;

    #[test]
    fn r64_takes_the_leftmost_empty_unlocked_zone_of_its_row_with_no_zone_named() {
        let mut state = game();
        put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({}));
        put(&mut state, "fx-2", slot(PlayerId::P1, Row::Units, 3), json!({}));

        run(
            &mut state,
            summon(json_as(json!({ "defId": "fx-5" }))),
            RunOptions::default(),
        );

        assert_eq!(
            lanes_of(&state, PlayerId::P1, Row::Units),
            json!(["fx-1", "fx-5", "fx-2", null, null])
        );
    }

    #[test]
    fn r64_skips_a_locked_zone_and_one_reserved_for_a_reborn_unit() {
        let mut state = game();
        lock_zone(&mut state, slot(PlayerId::P1, Row::Units, 1));
        reserve_zone(&mut state, slot(PlayerId::P1, Row::Units, 2));

        run(
            &mut state,
            summon(json_as(json!({ "defId": "fx-5" }))),
            RunOptions::default(),
        );

        assert_eq!(
            lanes_of(&state, PlayerId::P1, Row::Units),
            json!([null, null, "fx-5", null, null])
        );
    }

    #[test]
    fn s3_2_a_summon_into_a_full_row_fails_silently_and_creates_nothing() {
        let mut state = game();
        for lane in [1, 2, 3, 4, 5] {
            put(
                &mut state,
                "fx-1",
                slot(PlayerId::P1, Row::Units, lane),
                json!({}),
            );
        }
        let ids = state.next_id;

        let events = run(
            &mut state,
            summon(json_as(json!({ "defId": "fx-5" }))),
            RunOptions::default(),
        );

        assert!(events_of_type(&events, GameEventType::Summoned).is_empty());
        assert_eq!(state.next_id, ids);
        assert_eq!(
            lanes_of(&state, PlayerId::P1, Row::Units),
            json!(["fx-1", "fx-1", "fx-1", "fx-1", "fx-1"])
        );
    }

    #[test]
    fn r47_a_lane_named_summon_fizzles_on_an_occupied_zone() {
        let mut state = game();
        put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 2), json!({}));

        let on_occupied = run(
            &mut state,
            summon(json_as(json!({ "defId": "fx-5", "lane": 2 }))),
            RunOptions::default(),
        );
        assert!(events_of_type(&on_occupied, GameEventType::Summoned).is_empty());
        let on_empty = run(
            &mut state,
            summon(json_as(json!({ "defId": "fx-5", "lane": 5 }))),
            RunOptions::default(),
        );
        assert_eq!(events_of_type(&on_empty, GameEventType::Summoned).len(), 1);
        assert_eq!(
            lanes_of(&state, PlayerId::P1, Row::Units),
            json!([null, "fx-1", null, null, "fx-5"])
        );
    }

    #[test]
    fn r688_a_lane_named_summon_enters_a_locked_zone_only_plays_refuse_one() {
        let mut state = game();
        lock_zone(&mut state, slot(PlayerId::P1, Row::Units, 4));

        let events = run(
            &mut state,
            summon(json_as(json!({ "defId": "fx-5", "lane": 4 }))),
            RunOptions::default(),
        );
        assert_eq!(events_of_type(&events, GameEventType::Summoned).len(), 1);
        assert_eq!(
            lanes_of(&state, PlayerId::P1, Row::Units),
            json!([null, null, null, "fx-5", null])
        );
    }

    #[test]
    fn emits_summoned_with_the_zone_it_landed_in_and_gives_the_card_summoning_sickness() {
        let mut state = game();

        let events = run(
            &mut state,
            summon(json_as(json!({ "defId": "fx-7", "lane": 4 }))),
            RunOptions::default(),
        );
        let card = card_at(&state, slot(PlayerId::P1, Row::Units, 4)).cloned();

        assert!(card.is_some());
        let card = card.expect("the summoned card");
        assert_eq!(card.summoned_turn, Some(state.turn));
        assert_eq!(
            to_json(events_of_type(&events, GameEventType::Summoned)),
            json!([
                { "type": "summoned", "player": "p1", "instanceId": card.id, "defId": "fx-7", "row": "units", "lane": 4 },
            ])
        );
    }

    #[test]
    fn s7_summons_a_token_with_a_stats_override_which_the_layers_read() {
        let mut state = game();

        run(
            &mut state,
            summon(json_as(
                json!({ "defId": RUSH_TOKEN, "statsOverride": { "attack": 5, "health": 5 } }),
            )),
            RunOptions::default(),
        );
        let token = card_at(&state, slot(PlayerId::P1, Row::Units, 1)).cloned();

        assert_eq!(token.as_ref().map(|card| card.def_id.as_str()), Some(RUSH_TOKEN));
        assert_eq!(
            token.as_ref().and_then(|card| card.stats_override),
            Some(AttackHealth { attack: 5, health: 5 })
        );
        let token = token.expect("the token");
        assert_eq!(unit_view(&state, &token).attack, 5);
        assert_eq!(unit_view(&state, &token).max_health, 5);
    }

    #[test]
    fn r1_fires_no_cry() {
        let mut state = game();

        let events = run(
            &mut state,
            summon(json_as(json!({ "defId": crier().id }))),
            RunOptions::default(),
        );

        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 1)).map(|card| card.def_id.clone()),
            Some(crier().id)
        );
        assert!(events_of_type(&events, GameEventType::Damage).is_empty());
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
    }

    #[test]
    fn s5_1_refuses_to_summon_a_spell() {
        let mut state = game();

        let events = run(
            &mut state,
            summon(json_as(json!({ "defId": spell().id }))),
            RunOptions::default(),
        );

        assert!(events_of_type(&events, GameEventType::Summoned).is_empty());
        assert_eq!(
            lanes_of(&state, PlayerId::P1, Row::Units),
            json!([null, null, null, null, null])
        );
        assert_eq!(
            lanes_of(&state, PlayerId::P1, Row::Backrow),
            json!([null, null, null, null, null])
        );
    }

    #[test]
    fn r33_a_summoned_trap_enters_the_backrow_face_down_while_a_field_spell_is_public() {
        let mut state = game();

        run(
            &mut state,
            summon(json_as(json!({ "defId": trap().id }))),
            RunOptions::default(),
        );
        run(
            &mut state,
            summon(json_as(json!({ "defId": field_spell().id }))),
            RunOptions::default(),
        );

        assert_eq!(
            lanes_of(&state, PlayerId::P1, Row::Backrow),
            json!([trap().id, field_spell().id, null, null, null])
        );
        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Backrow, 1)).and_then(|card| card.face_up),
            None
        );
        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Backrow, 2)).and_then(|card| card.face_up),
            Some(true)
        );
    }

    #[test]
    fn s3_2_a_stack_summon_enters_an_occupied_unit_zone_and_becomes_the_piles_top_card() {
        let mut state = game();
        let under = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({}));
        find_instance_mut(&mut state, &under.id).expect("placed").damage = 2;

        let events = run(
            &mut state,
            summon(json_as(json!({ "defId": "fx-5", "lane": 1, "stack": true }))),
            RunOptions::default(),
        );

        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 1)).map(|card| card.def_id.clone()),
            Some("fx-5".to_string())
        );
        assert_eq!(
            state.players.p1.units[0].as_ref().map(|pile| def_ids_of(pile)),
            Some(vec!["fx-5".to_string(), "fx-1".to_string()])
        );
        assert_eq!(live(&state, &under.id).damage, 2);
        assert_eq!(summoned_lanes(&events), vec![1]);
    }

    #[test]
    fn s6_3_moves_a_card_that_already_exists_from_the_graveyard_onto_the_field() {
        let mut state = game();
        let dead = in_graveyard(&mut state, "fx-9", PlayerId::P1);

        let events = run(
            &mut state,
            summon(json_as(json!({ "instance": { "of": "chosen" } }))),
            RunOptions {
                targets: Some(vec![Selection::Instance {
                    instance_id: dead.id.clone(),
                }]),
                ..RunOptions::default()
            },
        );

        assert!(state.players.p1.graveyard.is_empty());
        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 1)).map(|card| card.id.clone()),
            Some(dead.id.clone())
        );
        assert_eq!(
            live(&state, &dead.id).zone,
            Zone::Field {
                player: PlayerId::P1,
                row: Row::Units,
                lane: 1,
            }
        );
        assert_eq!(summoned_ids(&events), vec![dead.id.clone()]);
    }

    #[test]
    fn r12_summoning_an_enemy_owned_card_puts_it_under_the_summoners_control_owner_unchanged() {
        let mut state = game();
        let theirs = in_graveyard(&mut state, "fx-9", PlayerId::P2);

        run(
            &mut state,
            summon(json_as(json!({ "instance": { "of": "chosen" } }))),
            RunOptions {
                controller: Some(PlayerId::P1),
                targets: Some(vec![Selection::Instance {
                    instance_id: theirs.id.clone(),
                }]),
                ..RunOptions::default()
            },
        );

        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 1)).map(|card| card.id.clone()),
            Some(theirs.id.clone())
        );
        let theirs = live(&state, &theirs.id);
        assert_eq!(theirs.controller, PlayerId::P1);
        assert_eq!(theirs.owner, PlayerId::P2);
    }
}

// ---------------------------------------------------------------------------
// recruit
// ---------------------------------------------------------------------------

mod recruit_s6_3_m3_t1 {
    use super::*;

    #[test]
    fn s6_3_takes_the_first_permanent_from_the_top_and_leaves_the_rest_of_the_library_in_order() {
        let mut state = game();
        // setLibrary hands back the library array itself, so the ids are snapshotted before the move.
        let ids = ids_of(&set_library(
            &mut state,
            PlayerId::P1,
            &owned(&["fx-1", "fx-2", "fx-3"]),
        ));

        let events = run(&mut state, recruit(json_as(json!({}))), RunOptions::default());

        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 1)).map(|card| card.id.clone()),
            Some(ids[0].clone())
        );
        assert_eq!(
            ids_of(&state.players.p1.library),
            vec![ids[1].clone(), ids[2].clone()]
        );
        assert_eq!(summoned_def_ids(&events), vec!["fx-1".to_string()]);
    }

    #[test]
    fn s6_3_skips_spells_and_every_permanent_the_filter_rejects_keeping_the_librarys_order() {
        let mut state = game();
        let spell_id = spell().id;
        let felinor_id = felinor().id;
        let ids = ids_of(&set_library(
            &mut state,
            PlayerId::P1,
            &owned(&[spell_id.as_str(), "fx-1", felinor_id.as_str(), "fx-2"]),
        ));

        run(
            &mut state,
            recruit(json_as(json!({ "filter": { "tags": ["Felinor"] } }))),
            RunOptions::default(),
        );

        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 1)).map(|card| card.def_id.clone()),
            Some(felinor_id)
        );
        assert_eq!(
            ids_of(&state.players.p1.library),
            vec![ids[0].clone(), ids[1].clone(), ids[3].clone()]
        );
    }

    #[test]
    fn r65_filters_on_the_printed_cost_and_stops_at_the_first_match() {
        let mut state = game();
        let pricey_id = pricey().id;
        set_library(
            &mut state,
            PlayerId::P1,
            &owned(&["fx-1", pricey_id.as_str(), "fx-2"]),
        );

        run(
            &mut state,
            recruit(json_as(json!({ "filter": { "cost": 4 } }))),
            RunOptions::default(),
        );

        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 1)).map(|card| card.def_id.clone()),
            Some(pricey_id)
        );
        assert_eq!(
            def_ids_of(&state.players.p1.library),
            vec!["fx-1".to_string(), "fx-2".to_string()]
        );
    }

    #[test]
    fn s6_3_a_recruited_trap_enters_the_backrow_face_down() {
        let mut state = game();
        let trap_id = trap().id;
        set_library(&mut state, PlayerId::P1, &owned(&["fx-1", trap_id.as_str()]));

        run(
            &mut state,
            recruit(json_as(json!({ "filter": { "type": "Trap" } }))),
            RunOptions::default(),
        );

        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Backrow, 1)).map(|card| card.def_id.clone()),
            Some(trap_id)
        );
        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Backrow, 1)).and_then(|card| card.face_up),
            None
        );
        assert_eq!(def_ids_of(&state.players.p1.library), vec!["fx-1".to_string()]);
    }

    #[test]
    fn r227_a_recruited_trap_takes_a_fresh_id_as_it_goes_face_down_and_its_summoned_event_names_the_old_one()
    {
        let mut state = game();
        let trap_id = trap().id;
        let held = ids_of(&set_library(
            &mut state,
            PlayerId::P1,
            &owned(&["fx-1", trap_id.as_str()]),
        ))[1]
            .clone();
        let next = format!("c{}", state.next_id);

        let events = run(
            &mut state,
            recruit(json_as(json!({ "filter": { "type": "Trap" } }))),
            RunOptions::default(),
        );

        let set = card_at(&state, slot(PlayerId::P1, Row::Backrow, 1)).cloned();
        assert_eq!(
            set.as_ref().map(|card| card.def_id.clone()),
            Some(trap_id.clone())
        );
        assert_eq!(set.as_ref().map(|card| card.id.clone()), Some(next.clone()));
        assert_ne!(set.as_ref().map(|card| card.id.clone()), Some(held.clone()));
        assert_eq!(
            to_json(events_of_type(&events, GameEventType::Summoned)),
            json!([
                {
                    "type": "summoned",
                    "player": "p1",
                    "instanceId": next,
                    "defId": trap_id,
                    "row": "backrow",
                    "lane": 1,
                    "formerId": held,
                },
            ])
        );
    }

    #[test]
    fn r227_a_recruited_unit_or_field_spell_keeps_its_id_only_a_card_going_face_down_takes_a_fresh_one() {
        let mut state = game();
        let field_id = field_spell().id;
        let ids = ids_of(&set_library(
            &mut state,
            PlayerId::P1,
            &owned(&["fx-1", field_id.as_str()]),
        ));
        let (unit_id, field_card_id) = (ids[0].clone(), ids[1].clone());

        let mut events = run(&mut state, recruit(json_as(json!({}))), RunOptions::default());
        events.extend(run(
            &mut state,
            recruit(json_as(json!({ "filter": { "type": "Field Spell" } }))),
            RunOptions::default(),
        ));

        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Units, 1)).map(|card| card.id.clone()),
            Some(unit_id)
        );
        assert_eq!(
            card_at(&state, slot(PlayerId::P1, Row::Backrow, 1)).map(|card| card.id.clone()),
            Some(field_card_id)
        );
        assert!(!events.iter().any(|event| matches!(
            event,
            GameEvent::Summoned {
                former_id: Some(_),
                ..
            }
        )));
    }

    #[test]
    fn s3_2_summons_nothing_when_no_permanent_matches_and_nothing_leaves_the_library() {
        let mut state = game();
        let spell_id = spell().id;
        set_library(
            &mut state,
            PlayerId::P1,
            &owned(&[spell_id.as_str(), spell_id.as_str()]),
        );

        let events = run(&mut state, recruit(json_as(json!({}))), RunOptions::default());

        assert!(events_of_type(&events, GameEventType::Summoned).is_empty());
        assert_eq!(state.players.p1.library.len(), 2);
    }

    #[test]
    fn s3_2_a_recruit_into_a_full_row_fizzles_and_the_card_stays_in_the_library() {
        let mut state = game();
        for lane in [1, 2, 3, 4, 5] {
            put(
                &mut state,
                "fx-20",
                slot(PlayerId::P1, Row::Units, lane),
                json!({}),
            );
        }
        set_library(&mut state, PlayerId::P1, &owned(&["fx-1"]));

        let events = run(&mut state, recruit(json_as(json!({}))), RunOptions::default());

        assert!(events_of_type(&events, GameEventType::Summoned).is_empty());
        assert_eq!(def_ids_of(&state.players.p1.library), vec!["fx-1".to_string()]);
    }
}

// ---------------------------------------------------------------------------
// fillBoard
// ---------------------------------------------------------------------------

mod fill_your_board_r64_s7_m3_t1 {
    use super::*;

    #[test]
    fn r64_summons_a_token_into_every_empty_unlocked_unit_zone_left_to_right() {
        let mut state = game();
        put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 2), json!({}));
        lock_zone(&mut state, slot(PlayerId::P1, Row::Units, 4));

        let events = run(
            &mut state,
            fill_board(json_as(json!({ "defId": RUSH_TOKEN }))),
            RunOptions::default(),
        );

        assert_eq!(
            lanes_of(&state, PlayerId::P1, Row::Units),
            json!([RUSH_TOKEN, "fx-1", RUSH_TOKEN, null, RUSH_TOKEN])
        );
        assert_eq!(summoned_lanes(&events), vec![1, 3, 5]);
    }

    #[test]
    fn s7_gives_every_token_it_makes_the_same_stats_override() {
        let mut state = game();

        run(
            &mut state,
            fill_board(json_as(
                json!({ "defId": RUSH_TOKEN, "statsOverride": { "attack": 5, "health": 5 } }),
            )),
            RunOptions::default(),
        );

        let stats: Vec<(i32, i32)> = [1, 2, 3, 4, 5]
            .iter()
            .map(|lane| {
                let card = card_at(&state, slot(PlayerId::P1, Row::Units, *lane))
                    .cloned()
                    .expect("a token in every lane");
                (
                    unit_view(&state, &card).attack,
                    unit_view(&state, &card).max_health,
                )
            })
            .collect();
        assert_eq!(stats, vec![(5, 5), (5, 5), (5, 5), (5, 5), (5, 5)]);
    }

    #[test]
    fn r64_fills_nothing_when_the_unit_row_is_full() {
        let mut state = game();
        for lane in [1, 2, 3, 4, 5] {
            put(
                &mut state,
                "fx-1",
                slot(PlayerId::P1, Row::Units, lane),
                json!({}),
            );
        }
        let ids = state.next_id;

        let events = run(
            &mut state,
            fill_board(json_as(json!({ "defId": RUSH_TOKEN }))),
            RunOptions::default(),
        );

        assert!(events_of_type(&events, GameEventType::Summoned).is_empty());
        assert_eq!(state.next_id, ids);
    }

    #[test]
    fn s7_fills_the_summoners_own_row_so_the_opponents_board_is_untouched() {
        let mut state = game();

        run(
            &mut state,
            fill_board(json_as(json!({ "defId": RUSH_TOKEN }))),
            RunOptions {
                controller: Some(PlayerId::P2),
                ..RunOptions::default()
            },
        );

        assert_eq!(
            lanes_of(&state, PlayerId::P2, Row::Units),
            json!([RUSH_TOKEN, RUSH_TOKEN, RUSH_TOKEN, RUSH_TOKEN, RUSH_TOKEN])
        );
        assert_eq!(
            lanes_of(&state, PlayerId::P1, Row::Units),
            json!([null, null, null, null, null])
        );
    }
}

// ---------------------------------------------------------------------------
// fillBoardRandom (R1080) and summonRandomFromHand (R1083).
// ---------------------------------------------------------------------------

/// Two 1/1 Unit fixtures, so the random pool below holds exactly two definitions.
fn one_one(name: &str, index: i32) -> CardDef {
    def_of_kind(
        name,
        index,
        "Unit",
        json!({
            "base": { "attack": 1, "health": 1, "keywords": [], "text": name },
            "radiant": { "attack": 2, "health": 2, "keywords": [], "text": name },
        }),
    )
}

/// `game()` plus the 1/1 fixtures, for the random-fill tests.
fn game_with_one_ones() -> GameState {
    let mut state = game();
    let mut catalog = registered_catalog().clone();
    for def in [one_one("uno", 801), one_one("dos", 802)] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    state
}

fn fill_random_query() -> Value {
    json!({ "query": { "type": "Unit", "stats": { "attack": 1, "health": 1 } } })
}

mod r1080_fill_board_random {
    use super::*;

    #[test]
    fn r1080_fill_board_random_one_pick_per_open_zone_left_to_right() {
        let mut state = game_with_one_ones();
        put(
            &mut state,
            "sm-crier",
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        put(
            &mut state,
            "sm-felinor",
            slot(PlayerId::P1, Row::Units, 3),
            json!({}),
        );

        let events = run(
            &mut state,
            fill_board_random(json_as(fill_random_query())),
            RunOptions::default(),
        );

        // Lanes 2, 4 and 5 each drew their own pick, left to right; every pick is a printed 1/1.
        assert_eq!(summoned_lanes(&events), vec![2, 4, 5]);
        for def_id in summoned_def_ids(&events) {
            assert!(def_id == "sm-uno" || def_id == "sm-dos");
        }
        let lanes = lanes_of(&state, PlayerId::P1, Row::Units);
        assert_eq!(lanes[0], json!("sm-crier"));
        assert_eq!(lanes[2], json!("sm-felinor"));
    }

    #[test]
    fn r1080_r129_a_full_board_draws_nothing() {
        let mut state = game_with_one_ones();
        for lane in 1..=5 {
            put(
                &mut state,
                "sm-crier",
                slot(PlayerId::P1, Row::Units, lane),
                json!({}),
            );
        }
        let cursor = state.rng_cursor;
        let ids = state.next_id;

        let events = run(
            &mut state,
            fill_board_random(json_as(fill_random_query())),
            RunOptions::default(),
        );

        assert!(events_of_type(&events, GameEventType::Summoned).is_empty());
        assert_eq!(state.rng_cursor, cursor);
        assert_eq!(state.next_id, ids);
    }
}

mod r1083_summon_random_from_hand {
    use super::*;

    #[test]
    fn r1083_summons_a_random_hand_unit_leftmost_with_no_cry() {
        let mut state = game();
        // The only Unit in hand is the crier: the pick is deterministic, and had its Cry fired the
        // enemy hero would be down 3.
        in_hand(&mut state, "sm-crier", PlayerId::P1, 1);
        in_hand(&mut state, "sm-spell", PlayerId::P1, 1);
        let source = put(
            &mut state,
            "sm-trap",
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );

        let events = run(
            &mut state,
            summon_random_from_hand(json_as(json!({}))),
            RunOptions {
                self_: Some(source.clone()),
                ..RunOptions::default()
            },
        );

        assert_eq!(
            lanes_of(&state, PlayerId::P1, Row::Units),
            json!(["sm-crier", null, null, null, null])
        );
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
        assert_eq!(summoned_def_ids(&events), vec!["sm-crier".to_string()]);
        // R1088: the summon names the card whose effect summoned it.
        let from: Vec<Option<String>> = events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { source_id, .. } => Some(source_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(from, vec![Some(source.id)]);
    }

    #[test]
    fn r1083_r129_no_zone_or_no_hand_unit_draws_nothing() {
        // No open zone: the hand Unit stays put and no pick is drawn.
        let mut state = game();
        in_hand(&mut state, "sm-crier", PlayerId::P1, 1);
        for lane in 1..=5 {
            put(
                &mut state,
                "sm-felinor",
                slot(PlayerId::P1, Row::Units, lane),
                json!({}),
            );
        }
        let cursor = state.rng_cursor;
        let events = run(
            &mut state,
            summon_random_from_hand(json_as(json!({}))),
            RunOptions::default(),
        );
        assert!(events_of_type(&events, GameEventType::Summoned).is_empty());
        assert_eq!(state.rng_cursor, cursor);
        assert_eq!(state.players[PlayerId::P1].hand.len(), 1);

        // No Unit in hand (a Spell only): likewise nothing.
        let mut state = game();
        in_hand(&mut state, "sm-spell", PlayerId::P1, 1);
        let cursor = state.rng_cursor;
        let events = run(
            &mut state,
            summon_random_from_hand(json_as(json!({}))),
            RunOptions::default(),
        );
        assert!(events_of_type(&events, GameEventType::Summoned).is_empty());
        assert_eq!(state.rng_cursor, cursor);
    }

    #[test]
    fn r1083_radiant_on_the_field() {
        let mut state = game();
        in_hand(&mut state, "sm-felinor", PlayerId::P1, 1);

        run(
            &mut state,
            summon_random_from_hand(json_as(json!({ "radiant": true }))),
            RunOptions::default(),
        );

        let unit = card_at(&state, slot(PlayerId::P1, Row::Units, 1)).expect("the summon");
        assert!(unit.radiant);
    }
}

/// R1088: `sourceId` is the client's report — the trigger loop holds the event without it.
mod r1088_summoned_source_id {
    use super::*;

    #[test]
    fn r1088_summoned_names_its_source_the_rules_read_drops_it() {
        let mut state = game();
        in_hand(&mut state, "sm-felinor", PlayerId::P1, 1);
        let source = put(
            &mut state,
            "sm-trap",
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );

        let events = run(
            &mut state,
            summon_random_from_hand(json_as(json!({}))),
            RunOptions {
                self_: Some(source),
                ..RunOptions::default()
            },
        );

        let summoned: Vec<GameEvent> = events_of_type(&events, GameEventType::Summoned);
        assert_eq!(summoned.len(), 1);
        let with_source = match &summoned[0] {
            GameEvent::Summoned { source_id, .. } => source_id.clone(),
            _ => panic!("not a summon"),
        };
        assert!(with_source.is_some());
        // The rules read it without the source: hashes and replays never see it.
        match summoned[0].as_rules_read() {
            GameEvent::Summoned { source_id, .. } => assert_eq!(source_id, None),
            _ => panic!("not a summon"),
        }
    }
}
