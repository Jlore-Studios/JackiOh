//! §4.5's check at a point of an effect list, then the rest on a stay that begins after it —
//! `afterStateCheck` (effects/afterCheck.ts), the verb Classic #43 Plague Nuke needs ("Destroy all
//! Units. Gain 1 mana for each Plague Counter … Then summon … each of those Units … from its owner's
//! graveyard"). SPEC §4.5; R59, R78, R113, R174. The real card's test (packages/cards/test/classic/
//! 043-plague-nuke.test.ts) covers the card's cases again through the card.
//!
//! Fixtures are this file's own: defs are prefixed `ac-` and indexed from 5300, so they cannot collide
//! with another test file's catalog (BUILD §0).
//!
//! Port of `packages/engine/test/effects-after-check.test.ts`.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::effects::{ForEachCardArgs, after_state_check, destroy_all, for_each_card, gain_mana, summon};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, slot};

const LOG_CARD: &str = "ac-log";
const ASKER: &str = "ac-asker";
const QUIET: &str = "ac-quiet";
/// Destroy all Units, then — after the check — note what the graveyards hold and gain 1 mana.
const SWEEP: &str = "ac-sweep";
/// Destroy all Units, then — after the check — summon every Unit card in p2's graveyard for p1.
const SWEEP_AND_TAKE: &str = "ac-sweep-take";
/// The control: the same summon in the same list with no check between, aimed at the ids read first.
const SWEEP_NO_CHECK: &str = "ac-sweep-no-check";

/// TS `nextIndex = 5300`, bumped once per `def` call in declaration order.
fn def(name: &str, type_: &str, index: u32) -> CardDef {
    let face = if type_ == "Unit" {
        json!({ "attack": 1, "health": 1, "keywords": [], "text": name })
    } else {
        json!({ "keywords": [], "text": name })
    };
    json_as(json!({
        "id": format!("ac-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (after check)"),
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": face.clone(),
        "radiant": face,
    }))
}

fn defs() -> Vec<CardDef> {
    vec![
        def("log", "Field Spell", 5301),
        def("asker", "Unit", 5302),
        def("quiet", "Unit", 5303),
        def("sweep", "Spell", 5304),
        def("sweep-take", "Spell", 5305),
        def("sweep-no-check", "Spell", 5306),
    ]
}

const NOTE_LANE: usize = 5;

fn write(state: &mut GameState, entry: &str) {
    let Some(log) = state.players[PlayerId::P1].backrow.get_mut(NOTE_LANE - 1).and_then(|slot| slot.as_mut()) else {
        return;
    };
    let mut steps: Vec<Value> = log.memory.get("steps").and_then(|steps| steps.as_array()).cloned().unwrap_or_default();
    steps.push(json!(entry));
    log.memory.insert("steps".to_string(), Value::Array(steps));
}

fn notes(state: &GameState) -> Vec<String> {
    let log = state.players[PlayerId::P1].backrow.get(NOTE_LANE - 1).and_then(|slot| slot.as_ref());
    log.and_then(|log| log.memory.get("steps"))
        .and_then(|steps| steps.as_array())
        .map(|steps| steps.iter().filter_map(|step| step.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn note(entry: impl Into<String>) -> Effect {
    let entry: String = entry.into();
    Effect::new("ac:note", move |ctx| {
        write(ctx.state, &entry);
    })
}

fn ask_controller() -> Effect {
    Effect::new("ac:ask", |ctx| {
        let resume = resume_self(ctx, "asked", IndexMap::new());
        let player = ctx.controller;
        let _ = open_prompt(
            ctx,
            OpenPromptArgs {
                player,
                kind: PromptKind::Target,
                aim: None,
                prompt: "the dying card asks".to_string(),
                options: vec![PromptOption {
                    key: "none".to_string(),
                    label: "nothing".to_string(),
                    selection: Selection::None,
                    cost: None,
                    radiant: None,
                }],
                min: None,
                max: None,
                budget: None,
                owner: None,
                resume,
            },
        );
    })
}

fn both(script: Script) -> CardScripts {
    CardScripts { base: script.clone(), radiant: script }
}

/// The ids of `player`'s graveyard cards that are this file's Units.
fn unit_cards_in(ctx: &EffectContext, player: PlayerId) -> Vec<String> {
    zone_cards(ctx.state, player, ZoneName::Graveyard)
        .into_iter()
        .filter(|card| card.def_id == ASKER || card.def_id == QUIET)
        .map(|card| card.id.clone())
        .collect()
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts: IndexMap<String, CardScripts> = IndexMap::new();
    scripts.insert(
        ASKER.to_string(),
        both(Script {
            death: Some(hook(|_ctx| vec![note("ask"), ask_controller(), note("tail")])),
            resume: IndexMap::from([("asked", hook(|_ctx| vec![note("answered")]))]),
            ..Script::default()
        }),
    );
    scripts.insert(QUIET.to_string(), both(Script::default()));
    scripts.insert(
        SWEEP.to_string(),
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![
                    destroy_all(json_as(json!({ "side": "any" }))),
                    after_state_check(|ctx| {
                        vec![
                            note(format!("rest:{}", unit_cards_in(ctx, PlayerId::P2).len())),
                            gain_mana(json_as(json!({ "amount": 1 }))),
                        ]
                    }),
                ]
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        SWEEP_AND_TAKE.to_string(),
        both(Script {
            cry: Some(hook(|_ctx| {
                vec![
                    destroy_all(json_as(json!({ "side": "any" }))),
                    after_state_check(|ctx| {
                        unit_cards_in(ctx, PlayerId::P2)
                            .into_iter()
                            .map(|id| {
                                summon(json_as(json!({
                                    "instance": { "of": "instance", "instanceId": id },
                                    "player": "self",
                                })))
                            })
                            .collect()
                    }),
                ]
            })),
            ..Script::default()
        }),
    );
    scripts.insert(
        SWEEP_NO_CHECK.to_string(),
        both(Script {
            cry: Some(hook(|ctx| {
                let ids: Vec<String> = ctx.state.players[PlayerId::P2]
                    .units
                    .iter()
                    .flat_map(|pile| pile.iter().flatten().map(|card| card.id.clone()))
                    .collect();
                vec![
                    destroy_all(json_as(json!({ "side": "any" }))),
                    for_each_card(ForEachCardArgs {
                        cards: Arc::new(move |_ctx| ids.clone()),
                        each: Arc::new(|instance_id: &str| {
                            summon(json_as(json!({
                                "instance": { "of": "instance", "instanceId": instance_id },
                                "player": "self",
                            })))
                        }),
                    }),
                ]
            })),
            ..Script::default()
        }),
    );
    scripts
}

fn game(seed: &str) -> GameState {
    let state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    registry.extend(scripts());
    register_scripts(registry);
    state
}

static NONCE: AtomicU32 = AtomicU32::new(0);

/// `reduce` with a fresh nonce; `body` is the TS `ActionInput` literal.
fn act_result(state: &GameState, body: Value) -> ReduceResult {
    let nonce = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let mut action = body;
    action["nonce"] = json!(format!("ac{nonce}"));
    reduce(state, &json_as::<Action>(action))
}

fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&game(seed)).state;
    for player_id in [PlayerId::P1, PlayerId::P2] {
        let keep: Vec<String> = state.players[player_id].hand.iter().map(|card| card.id.clone()).collect();
        let result = act_result(&state, json!({ "type": "mulligan", "keep": keep, "playerId": player_id }));
        if let Some(error) = result.error {
            panic!("{error}");
        }
        state = result.state;
    }
    put(&mut state, LOG_CARD, slot(PlayerId::P1, Row::Backrow, NOTE_LANE as i32));
    state
}

fn play_spell(state: &mut GameState, def_id: &str) -> ReduceResult {
    let Some(card) = in_hand(state, def_id, PlayerId::P1, 1).into_iter().next() else {
        panic!("no spell");
    };
    let result = act_result(state, json!({ "type": "play", "instanceId": card.id, "playerId": "p1" }));
    if let Some(error) = &result.error {
        panic!("{error}");
    }
    result
}

/// TS `Array.prototype.lastIndexOf`: -1 when absent.
fn last_index_of(kinds: &[GameEventType], kind: GameEventType) -> i64 {
    kinds.iter().rposition(|k| *k == kind).map_or(-1, |i| i as i64)
}

/// `state.players[player].units[lane - 1]?.[0]?.id`.
fn top_id(state: &GameState, player: PlayerId, lane: usize) -> Option<String> {
    state.players[player].units[lane - 1].as_ref().and_then(|pile| pile.first()).map(|card| card.id.clone())
}

mod after_state_check_c_43_plague_nuke {
    use super::*;

    #[test]
    fn r59_runs_the_check_at_its_point_of_the_list_the_rest_finds_the_destroyed_units_in_their_graveyard_after_their_deaths(
    ) {
        let mut state = playing("after-check-order");
        put(&mut state, QUIET, slot(PlayerId::P2, Row::Units, 1));
        put(&mut state, QUIET, slot(PlayerId::P2, Row::Units, 2));

        let ReduceResult { state: after, events, .. } = play_spell(&mut state, SWEEP);

        assert_eq!(notes(&after), vec!["rest:2".to_string()]);
        let kinds: Vec<GameEventType> = events.iter().map(|event| event.event_type()).collect();
        assert!(
            last_index_of(&kinds, GameEventType::Destroyed) < last_index_of(&kinds, GameEventType::ManaChanged)
        );
        assert_eq!(events_of_type(&events, GameEventType::Destroyed).len(), 2);
    }

    #[test]
    fn r174_the_rest_is_a_new_stay_a_unit_the_check_sent_to_its_graveyard_is_nameable_there_by_id() {
        let mut state = playing("after-check-stay");
        let a = put(&mut state, QUIET, slot(PlayerId::P2, Row::Units, 1));
        let b = put(&mut state, QUIET, slot(PlayerId::P2, Row::Units, 3));

        let after = play_spell(&mut state, SWEEP_AND_TAKE).state;

        assert_eq!(top_id(&after, PlayerId::P1, 1), Some(a.id.clone()));
        assert_eq!(top_id(&after, PlayerId::P1, 2), Some(b.id.clone()));
        let quiet_in_graveyard: Vec<&CardInstance> =
            after.players[PlayerId::P2].graveyard.iter().filter(|card| card.def_id == QUIET).collect();
        assert_eq!(quiet_in_graveyard, Vec::<&CardInstance>::new());
    }

    #[test]
    fn the_control_without_the_check_in_between_the_same_summon_finds_the_units_still_on_the_field_and_takes_nothing() {
        let mut state = playing("after-check-control");
        put(&mut state, QUIET, slot(PlayerId::P2, Row::Units, 1));

        let after = play_spell(&mut state, SWEEP_NO_CHECK).state;

        assert!(after.players[PlayerId::P1].units.iter().all(|pile| pile.is_none()));
        assert_eq!(after.players[PlayerId::P2].graveyard.iter().filter(|card| card.def_id == QUIET).count(), 1);
    }

    #[test]
    fn r113_a_death_hook_asking_inside_the_check_parks_the_rest_behind_the_pass_after_the_answer_the_rest_runs_once_and_the_pause_survives_json(
    ) {
        let mut state = playing("after-check-pause");
        put(&mut state, ASKER, slot(PlayerId::P2, Row::Units, 1));
        put(&mut state, QUIET, slot(PlayerId::P2, Row::Units, 2));

        let paused = play_spell(&mut state, SWEEP).state;

        assert_eq!(notes(&paused), vec!["ask".to_string()]);
        assert!(paused.pending.is_some());
        assert_eq!(owed_work(&paused, Some(DEATHS_WORK)).len(), 1);
        assert!(paused.work.len() >= 2);

        let revived: GameState = serde_json::from_value(serde_json::to_value(&paused).unwrap()).unwrap();
        assert_eq!(revived, paused);
        let Some(pending) = revived.pending.clone() else {
            panic!("a prompt");
        };
        let answered = act_result(
            &revived,
            json!({
                "type": "answer",
                "choiceId": pending.id,
                "selection": [{ "pick": "none" }],
                "playerId": pending.player_id,
            }),
        );

        assert_eq!(answered.error, None);
        assert_eq!(
            notes(&answered.state),
            vec!["ask".to_string(), "answered".to_string(), "tail".to_string(), "rest:2".to_string()]
        );
        assert!(answered.state.pending.is_none());
        assert!(answered.state.work.is_empty());
        let events: &Vec<GameEvent> = &answered.events;
        assert!(!events_of_type(events, GameEventType::ManaChanged).is_empty());
    }
}
