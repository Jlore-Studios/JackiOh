//! A seat that plays only the tutorial's cards (SPEC §9.10; R2533000, which replaces R2433000's
//! Core-only bar): the handicap's `tutorialCardsOnly`, which the tutorial opponent plays under
//! (`AI_TUTORIAL`). Its cards are TUTORIAL_CARD_SET's (Training Grounds) whose rarity is none of
//! TUTORIAL_BARRED_RARITIES (Legendary, Mythic), a token judged by its printed rarity.
//!
//! The seat never plays, sets or casts another card, however the card reached its hand: dealt,
//! drawn, Discovered, added, returned. `legal_actions` lists no such play, `reduce` refuses one, a
//! card drawn that casts on draw is drawn instead, and an effect that casts or summons a card out of
//! the seat's hand takes none. What the seat holds on the field by other means is left alone (all
//! R2433000 said, which still holds). Absent, the field reaches no game.
//!
//! Fixtures: the vanilla catalog (`fx-1` up, every one a Core Common) and this file's `tc-` cards:
//! Training Grounds Units of every rarity, a Training Grounds token printed Legendary and one printed
//! Common, a Classic Unit, and Spells whose Cry Discovers, summons from the hand or bounces.

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

/// Training Grounds Units the seat may play, all cost 0: Common, Rare and Epic.
const TG_COMMON: &str = "tc-tg-common";
const TG_RARE: &str = "tc-tg-rare";
const TG_EPIC: &str = "tc-tg-epic";
/// Training Grounds Units the seat may hold and never play, all cost 0.
const TG_LEGENDARY: &str = "tc-tg-legendary";
const TG_MYTHIC: &str = "tc-tg-mythic";
/// R2533000: a token is judged by its printed rarity (TG #39.1 Training-Grounds Golem's Legendary).
const TG_TOKEN_LEGENDARY: &str = "tc-tg-token-legendary";
const TG_TOKEN_COMMON: &str = "tc-tg-token-common";
/// Of other sets, all cost 0: a Core Unit (as the fixtures' `fx-` are) and a Classic one.
const CORE_UNIT: &str = "tc-core-unit";
const CLASSIC_UNIT: &str = "tc-classic-unit";

/// Units the seat may play, and those it may not.
const PLAYABLE: [&str; 4] = [TG_COMMON, TG_RARE, TG_EPIC, TG_TOKEN_COMMON];
const UNPLAYABLE: [&str; 5] = [
    TG_LEGENDARY,
    TG_MYTHIC,
    TG_TOKEN_LEGENDARY,
    CORE_UNIT,
    CLASSIC_UNIT,
];

const TG_SPELL: &str = "tc-tg-spell";
const CORE_SPELL: &str = "tc-core-spell";
const CORE_ON_DRAW: &str = "tc-core-on-draw";
const LEGENDARY_ON_DRAW: &str = "tc-legendary-on-draw";
const TG_ON_DRAW: &str = "tc-tg-on-draw";
/// Training Grounds Commons whose Cry Discovers a Core Unit, Discovers a Legendary or Mythic Training
/// Grounds Unit, summons a random Unit from the hand, or returns the seat's Units to its hand.
const DISCOVER_CORE: &str = "tc-discover-core";
const DISCOVER_SHOWPIECE: &str = "tc-discover-showpiece";
const SUMMON_FROM_HAND: &str = "tc-summon-from-hand";
const BOUNCE_OWN: &str = "tc-bounce";

const TG: &str = "Training Grounds";

fn spell(id: &str, index: i32, set: &str, rarity: &str) -> CardDef {
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (tutorial cards)"),
        "set": set,
        "type": "Spell",
        "tags": [],
        "rarity": rarity,
        "token": false,
        "cost": 0,
        "base": { "keywords": [], "text": id },
        "radiant": { "keywords": [], "text": id },
    }))
}

fn discover(query: Value) -> Script {
    Script {
        cry: Some(hook(move |_| {
            vec![discover_from_catalog(json_as(
                json!({ "step": "picked", "query": query.clone() }),
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
    }
}

fn register() {
    setup_catalog();
    let mut catalog = registered_catalog().clone();
    for (n, (id, set, rarity, printed)) in [
        (TG_COMMON, TG, "Common", None),
        (TG_RARE, TG, "Rare", None),
        (TG_EPIC, TG, "Epic", None),
        (TG_LEGENDARY, TG, "Legendary", None),
        (TG_MYTHIC, TG, "Mythic", None),
        (TG_TOKEN_LEGENDARY, TG, "Token", Some("Legendary")),
        (TG_TOKEN_COMMON, TG, "Token", Some("Common")),
        (CORE_UNIT, "Core", "Common", None),
        (CLASSIC_UNIT, "Classic", "Common", None),
    ]
    .into_iter()
    .enumerate()
    {
        let mut fields = json!({ "id": id, "set": set, "rarity": rarity, "cost": 0 });
        if let Some(printed) = printed {
            fields["token"] = json!(true);
            fields["printedRarity"] = json!(printed);
        }
        let def = unit_def(3300 + n as i32, fields);
        catalog.insert(def.id.clone(), def);
    }
    for (n, (id, set, rarity)) in [
        (TG_SPELL, TG, "Common"),
        (CORE_SPELL, "Core", "Common"),
        (CORE_ON_DRAW, "Core", "Common"),
        (LEGENDARY_ON_DRAW, TG, "Legendary"),
        (TG_ON_DRAW, TG, "Rare"),
        (DISCOVER_CORE, TG, "Common"),
        (DISCOVER_SHOWPIECE, TG, "Common"),
        (SUMMON_FROM_HAND, TG, "Common"),
        (BOUNCE_OWN, TG, "Common"),
    ]
    .into_iter()
    .enumerate()
    {
        catalog.insert(id.to_string(), spell(id, 3320 + n as i32, set, rarity));
    }
    register_catalog(catalog);
    let mut scripts = registered_scripts().clone();
    let on_draw = || Script {
        static_flags: Some(json_as(json!({ "castOnDraw": true }))),
        cry: Some(hook(|_| vec![])),
        ..Script::default()
    };
    for id in [CORE_ON_DRAW, LEGENDARY_ON_DRAW, TG_ON_DRAW] {
        scripts.insert(id.to_string(), both(on_draw()));
    }
    scripts.insert(
        DISCOVER_CORE.to_string(),
        both(discover(json!({ "set": "Core", "type": "Unit" }))),
    );
    scripts.insert(
        DISCOVER_SHOWPIECE.to_string(),
        both(discover(
            json!({ "set": TG, "type": "Unit", "rarity": ["Legendary", "Mythic"] }),
        )),
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

/// The bar on a human's resources, so a test reads the bar and nothing else.
fn barred() -> Handicap {
    Handicap {
        tutorial_cards_only: Some(true),
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

fn keep(state: &GameState, player: PlayerId) -> GameState {
    let keep = state.players[player]
        .hand
        .iter()
        .map(|card| card.id.clone())
        .collect();
    act(
        state,
        ActionInput {
            body: ActionBody::Mulligan { keep },
            player_id: player,
        },
    )
}

fn no_auto_end(mut state: GameState) -> GameState {
    state.players.p1.auto_end_turn = Some(false);
    state.players.p2.auto_end_turn = Some(false);
    state
}

/// p1's main phase on turn 1, p1 barred or not.
fn playing(seed: &str, bar: bool) -> GameState {
    // With auto-end on, a barred seat holding only the fixtures' Core cards would end its turn unasked.
    let state = no_auto_end(begin_game(&created(seed, bar.then(barred), None)).state);
    keep(&keep(&state, BARRED), OTHER)
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

fn tutorial_card(state: &GameState, def_id: &str) -> bool {
    find_def(Some(state), def_id).is_some_and(state::is_tutorial_card)
}

fn play_body(instance_id: &str) -> ActionBody {
    json_as(json!({ "type": "play", "instanceId": instance_id }))
}

fn play(state: &GameState, player: PlayerId, instance_id: &str) -> ReduceResult {
    reduce(
        state,
        &Action::new(play_body(instance_id), player, format!("tc-{instance_id}")),
    )
}

/// Plays `spell` from the barred hand and answers its Discover with option `pick`.
fn discovered(seed: &str, spell: &str, pick: usize) -> (GameState, Vec<CardInstance>) {
    let mut state = playing(seed, true);
    let before: Vec<String> = state.players[BARRED]
        .hand
        .iter()
        .map(|card| card.id.clone())
        .collect();
    let discover = to_hand(&mut state, BARRED, spell);
    let mut state = act(
        &state,
        ActionInput {
            body: play_body(&discover.id),
            player_id: BARRED,
        },
    );
    let pending = state.pending.clone().expect("the Discover asks");
    let option = pending.options[pick % pending.options.len()].selection.clone();
    state = act(
        &state,
        ActionInput {
            body: ActionBody::Answer {
                choice_id: pending.id.clone(),
                selection: vec![option],
            },
            player_id: BARRED,
        },
    );
    let found = state.players[BARRED]
        .hand
        .iter()
        .filter(|card| !before.contains(&card.id))
        .cloned()
        .collect();
    (state, found)
}

mod r2533000_a_seat_that_plays_only_the_tutorials_cards {
    use super::*;

    #[test]
    fn r2533000_ai_tutorial_sets_it_no_tier_does_and_a_barred_seat_stores_it() {
        assert_eq!(AI_TUTORIAL.tutorial_cards_only, Some(true));
        assert_eq!(TUTORIAL_CARD_SET, SetName::TrainingGrounds);
        assert_eq!(TUTORIAL_BARRED_RARITIES, &[Rarity::Legendary, Rarity::Mythic]);
        for handicap in [
            HUMAN_HANDICAP,
            AI_DIFFICULTY.medium,
            AI_DIFFICULTY.hard,
            TUTORIAL_HUMAN,
        ] {
            assert_eq!(handicap.tutorial_cards_only, None);
        }
        let state = created("r2533000-stored", Some(barred()), None);
        assert_eq!(state.players[BARRED].handicap, Some(barred()));
        assert!(state::plays_only_tutorial_cards(&state, BARRED));
        assert!(!state::plays_only_tutorial_cards(&state, OTHER));
        let json = serde_json::to_value(state.players[BARRED].handicap).expect("serializes");
        assert_eq!(json["tutorialCardsOnly"], json!(true));
        assert!(json.get("coreOnly").is_none(), "{json}");
    }

    #[test]
    fn r2533000_a_tutorial_card_is_training_grounds_and_neither_legendary_nor_mythic_a_token_by_its_printed_rarity()
     {
        register();
        let state = playing("r2533000-cards", true);
        for id in PLAYABLE {
            assert!(tutorial_card(&state, id), "{id}");
        }
        for id in UNPLAYABLE {
            assert!(!tutorial_card(&state, id), "{id}");
        }
    }

    #[test]
    fn r2533000_r2433000_a_card_added_to_the_hand_is_a_legal_play_only_when_it_is_a_tutorial_card() {
        for bar in [true, false] {
            let mut state = playing(&format!("r2533000-added-{bar}"), bar);
            let playable: Vec<CardInstance> = PLAYABLE
                .iter()
                .chain(&[TG_SPELL])
                .map(|id| to_hand(&mut state, BARRED, id))
                .collect();
            let unplayable: Vec<CardInstance> = UNPLAYABLE
                .iter()
                .chain(&[CORE_SPELL])
                .map(|id| to_hand(&mut state, BARRED, id))
                .collect();
            let listed = listed_plays(&state, BARRED);
            for card in &playable {
                assert!(listed.contains(&card.id), "bar {bar}: {} is listed", card.def_id);
            }
            for card in &unplayable {
                assert_eq!(listed.contains(&card.id), !bar, "bar {bar}: {}", card.def_id);
                let result = play(&state, BARRED, &card.id);
                assert_eq!(
                    result.error.is_some(),
                    bar,
                    "bar {bar}: reduce on {}",
                    card.def_id
                );
                if bar {
                    assert!(
                        result.error.as_deref().is_some_and(|e| e.contains("R2533000")),
                        "{:?}",
                        result.error
                    );
                }
            }
            assert!(
                play(&state, BARRED, &playable[0].id).error.is_none(),
                "bar {bar}: a Training Grounds Common is played"
            );
        }
        // The other seat is never barred.
        let mut state = playing("r2533000-other", true);
        let theirs = to_hand(&mut state, OTHER, TG_LEGENDARY);
        state.active = OTHER;
        assert!(listed_plays(&state, OTHER).contains(&theirs.id));
    }

    #[test]
    fn r2533000_r2433000_a_discovered_core_legendary_or_mythic_card_is_not_playable_by_the_barred_seat() {
        let mut seen = Vec::new();
        for (spell, picks) in [(DISCOVER_CORE, 1), (DISCOVER_SHOWPIECE, 3)] {
            for pick in 0..picks {
                let (state, found) = discovered(&format!("r2533000-discover-{spell}-{pick}"), spell, pick);
                assert_eq!(found.len(), 1, "{spell}: the Discovered Unit is in hand");
                let card = &found[0];
                assert!(!tutorial_card(&state, &card.def_id), "{}", card.def_id);
                assert!(
                    !listed_plays(&state, BARRED).contains(&card.id),
                    "{}",
                    card.def_id
                );
                seen.push(card.def_id.clone());
            }
        }
        assert!(
            seen.iter().any(|id| id == TG_LEGENDARY || id == TG_MYTHIC),
            "a Legendary or Mythic Training Grounds Unit was Discovered: {seen:?}"
        );
    }

    #[test]
    fn r2533000_r2433000_a_unit_on_the_field_is_left_alone_and_returned_to_hand_is_barred_unless_a_tutorial_card()
     {
        let mut state = playing("r2533000-returned", true);
        let held: Vec<CardInstance> = [CORE_UNIT, TG_LEGENDARY, TG_COMMON]
            .into_iter()
            .enumerate()
            .map(|(at, id)| put(&mut state, id, slot(BARRED, Row::Units, at as i32 + 1), json!({})))
            .collect();
        for unit in &held {
            assert!(
                active_units_of(&state, BARRED)
                    .iter()
                    .any(|on_field| on_field.id == unit.id),
                "{} on the field by other means stays",
                unit.def_id
            );
        }
        let bounce = to_hand(&mut state, BARRED, BOUNCE_OWN);
        let state = act(
            &state,
            ActionInput {
                body: play_body(&bounce.id),
                player_id: BARRED,
            },
        );
        let listed = listed_plays(&state, BARRED);
        for (def, playable) in [(CORE_UNIT, false), (TG_LEGENDARY, false), (TG_COMMON, true)] {
            let back = state.players[BARRED]
                .hand
                .iter()
                .find(|card| card.def_id == def)
                .unwrap_or_else(|| panic!("{def} is back in hand"));
            assert_eq!(listed.contains(&back.id), playable, "{def}");
        }
    }

    #[test]
    fn r2533000_r2433000_after_a_mulligan_only_the_tutorial_cards_dealt_are_legal_plays() {
        // p1's deck holds Core Units (the fixtures') and Training Grounds Units of every rarity: the
        // deal and the redraw hand it some of each.
        let mut deck: Vec<String> = vanilla_deck(DECK_SIZE - 8, 1);
        deck.extend(
            [
                TG_COMMON,
                TG_RARE,
                TG_EPIC,
                TG_LEGENDARY,
                TG_MYTHIC,
                CLASSIC_UNIT,
                TG_SPELL,
                CORE_SPELL,
            ]
            .iter()
            .map(|id| id.to_string()),
        );
        let (mut barred_seen, mut open_seen) = (0, 0);
        for n in 0..12 {
            let created = created(
                &format!("r2533000-mulligan-{n}"),
                Some(barred()),
                Some((deck.clone(), vanilla_deck(DECK_SIZE, 1))),
            );
            let state = no_auto_end(begin_game(&created).state);
            // p1 throws its whole hand back and redraws; p2 keeps.
            let state = act(
                &state,
                ActionInput {
                    body: ActionBody::Mulligan { keep: vec![] },
                    player_id: BARRED,
                },
            );
            let state = keep(&state, OTHER);
            let listed = listed_plays(&state, BARRED);
            for card in &state.players[BARRED].hand {
                let tutorial = tutorial_card(&state, &card.def_id);
                if tutorial {
                    open_seen += 1;
                } else {
                    barred_seen += 1;
                }
                // Every tutorial card in this deck costs 0, so each is a legal play.
                assert_eq!(listed.contains(&card.id), tutorial, "game {n}: {}", card.def_id);
            }
        }
        assert!(barred_seen > 0, "the redraws dealt cards the seat may not play");
        assert!(open_seen > 0, "the redraws dealt Training Grounds cards it may");
    }

    #[test]
    fn r2533000_r2433000_a_barred_card_that_casts_on_draw_is_drawn_and_a_tutorial_one_still_casts() {
        for (def, cast) in [
            (CORE_ON_DRAW, false),
            (LEGENDARY_ON_DRAW, false),
            (TG_ON_DRAW, true),
        ] {
            let mut state = playing(&format!("r2533000-on-draw-{def}"), true);
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
    fn r2533000_r2433000_an_effect_casts_or_summons_no_barred_card_out_of_the_barred_hand() {
        // A cast out of the hand: nothing moves, nothing is played.
        let mut state = playing("r2533000-cast", true);
        let spell = to_hand(&mut state, BARRED, CORE_SPELL);
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

        // A summon out of the hand takes only a tutorial Unit: here there is none, so nothing.
        let summoned = |bar: bool, held: &[&str]| {
            let mut state = playing(&format!("r2533000-summon-{bar}-{}", held.len()), bar);
            state.players[BARRED].hand.clear();
            for id in held {
                to_hand(&mut state, BARRED, id);
            }
            let summon = to_hand(&mut state, BARRED, SUMMON_FROM_HAND);
            let state = act(
                &state,
                ActionInput {
                    body: play_body(&summon.id),
                    player_id: BARRED,
                },
            );
            active_units_of(&state, BARRED)
                .iter()
                .map(|unit| unit.def_id.clone())
                .collect::<Vec<_>>()
        };
        assert!(summoned(true, &[CORE_UNIT, TG_LEGENDARY, TG_MYTHIC]).is_empty());
        // With a Training Grounds Common beside them, it takes that one.
        assert_eq!(
            summoned(true, &[CORE_UNIT, TG_LEGENDARY, TG_MYTHIC, TG_COMMON]),
            vec![TG_COMMON.to_string()]
        );
        // Unbarred, the same summon takes a barred card.
        assert_eq!(summoned(false, &[TG_LEGENDARY]), vec![TG_LEGENDARY.to_string()]);
    }

    #[test]
    fn r2533000_r2433000_the_tutorial_opponent_on_the_random_policy_never_plays_a_barred_card_and_unbarred_it_does()
     {
        // p2 is the tutorial opponent; its 12-card deck is Core Units (the fixtures'), Legendary and
        // Mythic Training Grounds Units, and its playable Training Grounds Units.
        let mut deck: Vec<String> = vanilla_deck(5, 1);
        deck.extend(
            [
                TG_LEGENDARY,
                TG_MYTHIC,
                CLASSIC_UNIT,
                TG_COMMON,
                TG_RARE,
                TG_EPIC,
                TG_SPELL,
            ]
            .iter()
            .map(|id| id.to_string()),
        );
        let unbarred = Handicap {
            tutorial_cards_only: None,
            ..AI_TUTORIAL
        };
        let (mut unbarred_played, mut barred_played) = (0, 0);
        for (handicap, barred_run) in [(AI_TUTORIAL, true), (unbarred, false)] {
            for n in 0..10 {
                register();
                let seed = format!("r2533000-policy-{barred_run}-{n}");
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
                    let result = reduce(&state, &Action::new(body, player, format!("tp{steps}")));
                    assert!(result.error.is_none(), "{seed}: {:?}", result.error);
                    for event in &result.events {
                        if let GameEvent::CardPlayed {
                            player: PlayerId::P2,
                            def_id,
                            ..
                        } = event
                        {
                            if tutorial_card(&result.state, def_id) {
                                barred_played += usize::from(barred_run);
                            } else {
                                assert!(!barred_run, "{seed}: the tutorial opponent played {def_id}");
                                unbarred_played += 1;
                            }
                        }
                    }
                    state = result.state;
                }
            }
        }
        assert!(
            barred_played > 0,
            "barred, the seat still plays its Training Grounds cards"
        );
        assert!(unbarred_played > 0, "unbarred, the same seat plays the others");
    }
}

mod r2533000_hashes {
    use super::*;

    #[test]
    fn r2533000_the_field_reaches_no_game_that_does_not_set_it() {
        let plain = created("r2533000-plain", None, None);
        let unset = Handicap {
            tutorial_cards_only: Some(false),
            ..HUMAN_HANDICAP
        };
        let with_unset = created("r2533000-plain", Some(unset), None);
        assert!(
            with_unset.players.p1.handicap.is_none(),
            "a false bar stores nothing"
        );
        assert_eq!(hash_state(&with_unset), hash_state(&plain));
        for tier in [AI_DIFFICULTY.medium, AI_DIFFICULTY.hard, TUTORIAL_HUMAN] {
            let state = created("r2533000-tier", Some(tier), None);
            let json = serde_json::to_value(state.players.p1.handicap).expect("serializes");
            assert!(json.get("tutorialCardsOnly").is_none(), "{json}");
        }
    }

    #[test]
    fn r2533000_a_handicap_stored_with_r2433000s_core_only_bar_reads_as_no_bar() {
        // A lesson saved under #2433's field folds with no bar now: the field is gone, not renamed,
        // so its Core cards, all it held then, are not barred by a rule it never played under.
        let json = json!({
            "deckSize": 12, "manaBonus": 0, "manaCap": 3, "extraOpeningCards": 0,
            "extraDrawsPerTurn": 0, "heroHealth": 20, "coreOnly": true
        });
        let read: Handicap = json_as(json);
        assert_eq!(read.tutorial_cards_only, None);
        assert_eq!(
            read,
            Handicap {
                tutorial_cards_only: None,
                deck_copies: None,
                ..AI_TUTORIAL
            }
        );
    }
}
