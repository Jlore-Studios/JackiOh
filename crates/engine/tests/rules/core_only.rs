//! A seat that plays only Core cards (SPEC §9.10; R2433000): the handicap's `coreOnly`, which the
//! tutorial opponent plays under (`AI_TUTORIAL`). The seat never plays, sets or casts a card whose set
//! is not Core, however the card reached its hand; what it holds on the field by other means is left
//! alone. Absent, the field reaches no game.
//!
//! Fixtures: the vanilla catalog (`fx-1` up, every one Core) and this file's `co-` cards, Classic
//! Units and Spells and three Core Spells whose Cry Discovers, summons from the hand or bounces.

use jackioh_engine::effects::{
    add_to_hand, bounce, chosen_options, discover_from_catalog, summon_random_from_hand,
};
use jackioh_engine::resolve::{CastOptions, cast_card};
use jackioh_engine::rng::Rng;
use jackioh_engine::script::EngineSink;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::{unit_def, vanilla_deck};
use crate::rules::fixtures::harness::{act, both, put, setup_catalog, slot};

const BARRED: PlayerId = PlayerId::P1;
const OTHER: PlayerId = PlayerId::P2;

/// Classic Units, all cost 0: what a Core-only seat may hold and never play.
const CLASSIC_UNITS: [&str; 6] = [
    "co-classic-1",
    "co-classic-2",
    "co-classic-3",
    "co-classic-4",
    "co-classic-5",
    "co-classic-6",
];
const CLASSIC_SPELL: &str = "co-classic-spell";
const CLASSIC_ON_DRAW: &str = "co-classic-on-draw";
const CORE_ON_DRAW: &str = "co-core-on-draw";
const DISCOVER: &str = "co-discover";
const SUMMON_FROM_HAND: &str = "co-summon-from-hand";
const BOUNCE_OWN: &str = "co-bounce";

fn spell(id: &str, index: i32, set: &str) -> CardDef {
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (core only)"),
        "set": set,
        "type": "Spell",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": id },
        "radiant": { "keywords": [], "text": id },
    }))
}

fn register() {
    setup_catalog();
    let mut catalog = registered_catalog().clone();
    for (n, id) in CLASSIC_UNITS.iter().enumerate() {
        let def = unit_def(3300 + n as i32, json!({ "id": id, "set": "Classic", "cost": 0 }));
        catalog.insert(def.id.clone(), def);
    }
    for (n, (id, set)) in [
        (CLASSIC_SPELL, "Classic"),
        (CLASSIC_ON_DRAW, "Classic"),
        (CORE_ON_DRAW, "Core"),
        (DISCOVER, "Core"),
        (SUMMON_FROM_HAND, "Core"),
        (BOUNCE_OWN, "Core"),
    ]
    .into_iter()
    .enumerate()
    {
        catalog.insert(id.to_string(), spell(id, 3310 + n as i32, set));
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    let on_draw = || Script {
        static_flags: Some(json_as(json!({ "castOnDraw": true }))),
        cry: Some(hook(|_| vec![])),
        ..Script::default()
    };
    scripts.insert(CLASSIC_ON_DRAW.to_string(), both(on_draw()));
    scripts.insert(CORE_ON_DRAW.to_string(), both(on_draw()));
    scripts.insert(
        DISCOVER.to_string(),
        both(Script {
            cry: Some(hook(|_| {
                vec![discover_from_catalog(json_as(
                    json!({ "step": "picked", "query": { "set": "Classic", "type": "Unit" } }),
                ))]
            })),
            resume: [(
                "picked",
                hook(|ctx| {
                    let picked = chosen_options(ctx).into_iter().next().unwrap_or_default();
                    vec![add_to_hand(json_as(json!({ "defId": picked })))]
                }),
            )]
            .into_iter()
            .collect(),
            ..Script::default()
        }),
    );
    scripts.insert(
        SUMMON_FROM_HAND.to_string(),
        both(Script {
            cry: Some(hook(|_| vec![summon_random_from_hand(json_as(json!({})))])),
            ..Script::default()
        }),
    );
    scripts.insert(
        BOUNCE_OWN.to_string(),
        both(Script {
            cry: Some(hook(|ctx| {
                let own: Vec<String> = active_units_of(&*ctx.state, ctx.controller)
                    .into_iter()
                    .map(|unit| unit.id.clone())
                    .collect();
                own.into_iter()
                    .map(|id| {
                        bounce(json_as(
                            json!({ "target": { "of": "instance", "instanceId": id } }),
                        ))
                    })
                    .collect()
            })),
            ..Script::default()
        }),
    );
    register_scripts(scripts);
}

/// The Core-only bar on a human's resources, so a test reads the bar and nothing else.
fn barred() -> Handicap {
    Handicap {
        core_only: Some(true),
        ..HUMAN_HANDICAP
    }
}

fn created(seed: &str, p1: Option<Handicap>, decks: Option<(Vec<String>, Vec<String>)>) -> GameState {
    register();
    let size = p1.map_or(DECK_SIZE, |h| h.deck_size);
    create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: decks.unwrap_or_else(|| (vanilla_deck(size, 1), vanilla_deck(DECK_SIZE, 1))),
        handicaps: Some(PerPlayerOpt { p1, p2: None }),
        ..Default::default()
    })
}

fn keep_both(mut state: GameState) -> GameState {
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
    state.players.p2.auto_end_turn = Some(false);
    state
}

/// p1's main phase on turn 1, p1 barred or not.
fn playing(seed: &str, bar: bool) -> GameState {
    keep_both(begin_game(&created(seed, bar.then(barred), None)).state)
}

fn to_hand(state: &mut GameState, player: PlayerId, def_id: &str) -> CardInstance {
    let card = state::new_instance(state, def_id, player, Zone::Hand { player });
    state.players[player].hand.push(card.clone());
    card
}

/// The instance ids `legal_actions` lists a play of for `player`.
fn listed_plays(state: &GameState, player: PlayerId) -> Vec<String> {
    legal_actions(state, player)
        .into_iter()
        .filter_map(|action| match action {
            ActionBody::Play { instance_id, .. } => Some(instance_id),
            _ => None,
        })
        .collect()
}

fn is_core(state: &GameState, def_id: &str) -> bool {
    find_def(Some(state), def_id).is_some_and(|def| def.set == SetName::Core)
}

fn play(state: &GameState, player: PlayerId, instance_id: &str) -> ReduceResult {
    reduce(
        state,
        &Action::new(
            json_as::<ActionBody>(json!({ "type": "play", "instanceId": instance_id })),
            player,
            format!("co-{instance_id}"),
        ),
    )
}

mod r2433000_a_seat_that_plays_only_core_cards {
    use super::*;

    #[test]
    fn r2433000_ai_tutorial_sets_it_no_tier_does_and_a_barred_seat_stores_it() {
        assert_eq!(AI_TUTORIAL.core_only, Some(true));
        for handicap in [
            HUMAN_HANDICAP,
            AI_DIFFICULTY.medium,
            AI_DIFFICULTY.hard,
            TUTORIAL_HUMAN,
        ] {
            assert_eq!(handicap.core_only, None);
        }
        let state = created("r2433000-stored", Some(barred()), None);
        assert_eq!(state.players[BARRED].handicap, Some(barred()));
        assert!(state::plays_only_core(&state, BARRED));
        assert!(!state::plays_only_core(&state, OTHER));
        let json = serde_json::to_value(state.players[BARRED].handicap).expect("serializes");
        assert_eq!(json["coreOnly"], json!(true));
    }

    #[test]
    fn r2433000_a_non_core_card_added_to_the_hand_is_never_a_legal_play_and_reduce_refuses_it() {
        for bar in [true, false] {
            let mut state = playing(&format!("r2433000-added-{bar}"), bar);
            let unit = to_hand(&mut state, BARRED, CLASSIC_UNITS[0]);
            let spell = to_hand(&mut state, BARRED, CLASSIC_SPELL);
            let core = to_hand(&mut state, BARRED, BOUNCE_OWN);
            let listed = listed_plays(&state, BARRED);
            assert!(listed.contains(&core.id), "bar {bar}: the Core card is listed");
            assert_eq!(listed.contains(&unit.id), !bar, "bar {bar}: the Classic Unit");
            assert_eq!(listed.contains(&spell.id), !bar, "bar {bar}: the Classic Spell");
            let result = play(&state, BARRED, &unit.id);
            assert_eq!(
                result.error.is_some(),
                bar,
                "bar {bar}: reduce on the Classic Unit"
            );
            if bar {
                assert!(
                    result.error.as_deref().is_some_and(|e| e.contains("R2433000")),
                    "{:?}",
                    result.error
                );
            }
        }
        // The other seat is never barred.
        let mut state = playing("r2433000-other", true);
        let theirs = to_hand(&mut state, OTHER, CLASSIC_UNITS[0]);
        state.active = OTHER;
        assert!(listed_plays(&state, OTHER).contains(&theirs.id));
    }

    #[test]
    fn r2433000_a_discovered_card_is_not_playable_by_the_barred_seat() {
        let mut state = playing("r2433000-discover", true);
        let discover = to_hand(&mut state, BARRED, DISCOVER);
        let mut state = act(
            &state,
            ActionInput {
                body: json_as(json!({ "type": "play", "instanceId": discover.id })),
                player_id: BARRED,
            },
        );
        let pending = state.pending.clone().expect("the Discover asks");
        state = act(
            &state,
            ActionInput {
                body: ActionBody::Answer {
                    choice_id: pending.id.clone(),
                    selection: vec![pending.options[0].selection.clone()],
                },
                player_id: BARRED,
            },
        );
        let found: Vec<CardInstance> = state.players[BARRED]
            .hand
            .iter()
            .filter(|card| !is_core(&state, &card.def_id))
            .cloned()
            .collect();
        assert_eq!(found.len(), 1, "the Discovered Classic Unit is in hand");
        assert!(!listed_plays(&state, BARRED).contains(&found[0].id));
    }

    #[test]
    fn r2433000_a_non_core_unit_on_the_field_is_left_alone_and_returned_to_hand_is_barred() {
        let mut state = playing("r2433000-returned", true);
        let unit = put(
            &mut state,
            CLASSIC_UNITS[0],
            slot(BARRED, Row::Units, 1),
            json!({}),
        );
        assert!(
            active_units_of(&state, BARRED)
                .iter()
                .any(|held| held.id == unit.id),
            "a Classic Unit on the field by other means stays"
        );
        let bounce = to_hand(&mut state, BARRED, BOUNCE_OWN);
        let state = act(
            &state,
            ActionInput {
                body: json_as(json!({ "type": "play", "instanceId": bounce.id })),
                player_id: BARRED,
            },
        );
        let back = state.players[BARRED]
            .hand
            .iter()
            .find(|card| card.def_id == CLASSIC_UNITS[0])
            .expect("the Unit is back in hand")
            .clone();
        assert!(!listed_plays(&state, BARRED).contains(&back.id));
    }

    #[test]
    fn r2433000_after_a_mulligan_no_non_core_card_dealt_is_a_legal_play() {
        // p1's deck holds six Classic Units: the deal and the redraw hand it some.
        let mut deck: Vec<String> = vanilla_deck(DECK_SIZE - CLASSIC_UNITS.len() as i32, 1);
        deck.extend(CLASSIC_UNITS.iter().map(|id| id.to_string()));
        let mut seen_non_core = 0;
        for n in 0..12 {
            let created = created(
                &format!("r2433000-mulligan-{n}"),
                Some(barred()),
                Some((deck.clone(), vanilla_deck(DECK_SIZE, 1))),
            );
            let mut state = begin_game(&created).state;
            // p1 throws its whole hand back and redraws; p2 keeps.
            state = act(
                &state,
                ActionInput {
                    body: ActionBody::Mulligan { keep: vec![] },
                    player_id: BARRED,
                },
            );
            state = keep_both_p2(state);
            let hand = state.players[BARRED].hand.clone();
            let listed = listed_plays(&state, BARRED);
            for card in &hand {
                if !is_core(&state, &card.def_id) {
                    seen_non_core += 1;
                    assert!(!listed.contains(&card.id), "game {n}: {}", card.def_id);
                }
            }
        }
        assert!(seen_non_core > 0, "the redraws dealt Classic cards");
    }

    fn keep_both_p2(state: GameState) -> GameState {
        let keep = state.players[OTHER]
            .hand
            .iter()
            .map(|card| card.id.clone())
            .collect();
        let mut state = act(
            &state,
            ActionInput {
                body: ActionBody::Mulligan { keep },
                player_id: OTHER,
            },
        );
        state.players.p1.auto_end_turn = Some(false);
        state
    }

    #[test]
    fn r2433000_a_non_core_card_that_casts_on_draw_is_drawn_and_a_core_one_still_casts() {
        for (def, cast) in [(CLASSIC_ON_DRAW, false), (CORE_ON_DRAW, true)] {
            let mut state = playing(&format!("r2433000-on-draw-{def}"), true);
            let card = state::new_instance(&mut state, def, BARRED, Zone::Library { player: BARRED });
            state.players[BARRED].library.insert(0, card.clone());
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut events = Vec::new();
            {
                let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
                jackioh_engine::draw::draw(&mut sink, BARRED, 1);
            }
            let in_hand = state.players[BARRED].hand.iter().any(|held| held.id == card.id);
            let played = events.iter().any(
                |event| matches!(event, GameEvent::CardPlayed { instance_id, .. } if *instance_id == card.id),
            );
            assert_eq!(played, cast, "{def}: cast on draw");
            assert_eq!(in_hand, !cast, "{def}: drawn into the hand");
        }
    }

    #[test]
    fn r2433000_an_effect_casts_or_summons_no_non_core_card_out_of_the_barred_hand() {
        // A cast out of the hand: nothing moves, nothing is played.
        let mut state = playing("r2433000-cast", true);
        let spell = to_hand(&mut state, BARRED, CLASSIC_SPELL);
        let mut rng = Rng::new(&state.seed, state.rng_cursor);
        let mut events = Vec::new();
        {
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            cast_card(&mut sink, &spell, CastOptions::default());
        }
        assert!(
            state.players[BARRED].hand.iter().any(|held| held.id == spell.id),
            "the Spell stays in hand"
        );
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, GameEvent::CardPlayed { .. }))
        );

        // A summon out of the hand takes only a Core Unit: here there is none, so nothing.
        let mut state = playing("r2433000-summon", true);
        state.players[BARRED].hand.clear();
        let unit = to_hand(&mut state, BARRED, CLASSIC_UNITS[0]);
        let summon = to_hand(&mut state, BARRED, SUMMON_FROM_HAND);
        let state = act(
            &state,
            ActionInput {
                body: json_as(json!({ "type": "play", "instanceId": summon.id })),
                player_id: BARRED,
            },
        );
        assert!(
            state.players[BARRED].hand.iter().any(|held| held.id == unit.id),
            "the Classic Unit stays"
        );
        assert!(active_units_of(&state, BARRED).is_empty());

        // Unbarred, the same summon takes it.
        let mut state = playing("r2433000-summon-open", false);
        state.players[BARRED].hand.clear();
        let unit = to_hand(&mut state, BARRED, CLASSIC_UNITS[0]);
        let summon = to_hand(&mut state, BARRED, SUMMON_FROM_HAND);
        let state = act(
            &state,
            ActionInput {
                body: json_as(json!({ "type": "play", "instanceId": summon.id })),
                player_id: BARRED,
            },
        );
        assert!(
            active_units_of(&state, BARRED)
                .iter()
                .any(|held| held.id == unit.id)
        );
    }

    #[test]
    fn r2433000_the_tutorial_opponent_on_the_random_policy_never_plays_a_non_core_card_and_unbarred_it_does()
    {
        // p2 is the tutorial opponent; its 12-card deck is half Classic.
        let mut deck: Vec<String> = vanilla_deck(6, 1);
        deck.extend(CLASSIC_UNITS.iter().map(|id| id.to_string()));
        let unbarred = Handicap {
            core_only: None,
            ..AI_TUTORIAL
        };
        let mut unbarred_played = 0;
        for (handicap, barred_run) in [(AI_TUTORIAL, true), (unbarred, false)] {
            for n in 0..10 {
                register();
                let seed = format!("r2433000-policy-{barred_run}-{n}");
                let mut state = begin_game(&create_game(&CreateGameOptions {
                    seed: seed.clone(),
                    decks: (vanilla_deck(DECK_SIZE, 1), deck.clone()),
                    handicaps: Some(PerPlayerOpt {
                        p1: None,
                        p2: Some(handicap),
                    }),
                    ..Default::default()
                }))
                .state;
                let mut policy = Rng::new(&format!("{seed}-policy"), 0);
                let mut steps = 0;
                while state.result.is_none() {
                    steps += 1;
                    assert!(steps < 5000, "{seed} ends");
                    let player = seat_to_act(&state).expect("a seat to act");
                    let body =
                        subsystems::ai_policy::choose_action(&state, player, &mut policy).expect("legal");
                    let result = reduce(&state, &Action::new(body, player, format!("cp{steps}")));
                    assert!(result.error.is_none(), "{seed}: {:?}", result.error);
                    for event in &result.events {
                        if let GameEvent::CardPlayed {
                            player: PlayerId::P2,
                            def_id,
                            ..
                        } = event
                            && !is_core(&result.state, def_id)
                        {
                            assert!(!barred_run, "{seed}: the tutorial opponent played {def_id}");
                            unbarred_played += 1;
                        }
                    }
                    state = result.state;
                }
            }
        }
        assert!(
            unbarred_played > 0,
            "unbarred, the same seat plays its Classic cards"
        );
    }
}

mod r2433000_hashes {
    use super::*;

    #[test]
    fn r2433000_the_field_reaches_no_game_that_does_not_set_it() {
        let plain = created("r2433000-plain", None, None);
        let unset = Handicap {
            core_only: Some(false),
            ..HUMAN_HANDICAP
        };
        let with_unset = created("r2433000-plain", Some(unset), None);
        assert!(
            with_unset.players.p1.handicap.is_none(),
            "a false bar stores nothing"
        );
        assert_eq!(hash_state(&with_unset), hash_state(&plain));
        for tier in [AI_DIFFICULTY.medium, AI_DIFFICULTY.hard, TUTORIAL_HUMAN] {
            let state = created("r2433000-tier", Some(tier), None);
            let json = serde_json::to_value(state.players.p1.handicap).expect("serializes");
            assert!(json.get("coreOnly").is_none(), "{json}");
        }
    }
}
