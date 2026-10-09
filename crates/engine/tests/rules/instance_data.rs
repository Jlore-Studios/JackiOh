//! Port of `packages/engine/test/instance-data.test.ts`.
//!
//! Instance data end to end (docs/classic-sets.md B5 E38, B3.4, R386, R440): buffs and granted keywords
//! given in a hand or a deck ride the card onto the field; the E38 scope verbs report only what both
//! players read; KY's Constant and a Degrade after a target prompt pause, survive a JSON round trip and
//! resume identically; and whole games dealt the instance-data fixtures fold back from their logs to
//! the same state (§9.3), with a JSON round trip before every action.

use std::sync::atomic::{AtomicU32, Ordering};

use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{in_hand, put, set_library, slot};
use crate::rules::fixtures::instance_data::{
    CONSTANT_STEP, body, constant, instance_deck, instance_game, military, nerfer, numbered, numbered_body,
    register_instance_fixtures,
};

/// TS's module-level `let nonce`; an atomic so that tests running side by side never share a nonce.
static NONCE: AtomicU32 = AtomicU32::new(0);

fn act(state: &GameState, action: ActionInput, fixed: Option<&str>) -> GameState {
    let n = NONCE.fetch_add(1, Ordering::Relaxed) + 1;
    let nonce = fixed.map_or_else(|| format!("id{n}"), str::to_string);
    let result = reduce(state, &action.with_nonce(nonce));
    if let Some(error) = result.error {
        panic!("{error}");
    }
    result.state
}

fn input(player: PlayerId, action: ActionBody) -> ActionInput {
    ActionInput {
        body: action,
        player_id: player,
    }
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// p1's main phase on turn 1, past the mulligans, with mana to spend.
fn playing(seed: &str) -> GameState {
    let mut state = begin_game(&instance_game(seed, None)).state;
    let keep = ids(&state.players.p1.hand);
    state = act(&state, input(PlayerId::P1, ActionBody::Mulligan { keep }), None);
    let keep = ids(&state.players.p2.hand);
    state = act(&state, input(PlayerId::P2, ActionBody::Mulligan { keep }), None);
    state.players.p1.mana = ManaState {
        current: 4,
        max: 4,
        ..state.players.p1.mana
    };
    state
}

/// One effect applied by p1 through a context of its own; the rng's cursor is written back, as TS did.
fn run(state: &mut GameState, effect: &Effect) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = resolve::make_context(
            &mut sink,
            None,
            resolve::HookOptions {
                controller: Some(PlayerId::P1),
                ..Default::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn play_of(
    state: &GameState,
    card: &CardInstance,
    pick: Option<&dyn Fn(&ActionBody) -> bool>,
) -> ActionInput {
    let play = legal_actions(state, PlayerId::P1).into_iter().find(|action| {
        matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == card.id)
            && pick.is_none_or(|pick| pick(action))
    });
    let Some(play) = play else {
        panic!("no play for {}", card.def_id)
    };
    input(PlayerId::P1, play)
}

/// `JSON.stringify(a.targets).includes(id)`.
fn targets_name(action: &ActionBody, id: &str) -> bool {
    match action {
        ActionBody::Play { targets, .. } => serde_json::to_string(targets)
            .expect("targets serialise")
            .contains(id),
        _ => false,
    }
}

fn on_field(state: &GameState, id: &str) -> CardInstance {
    state
        .players
        .p1
        .units
        .iter()
        .flat_map(|pile| pile.iter().flatten())
        .find(|card| card.id == id)
        .expect("expected the card on the field")
        .clone()
}

/// The card as the state holds it now (TS read its live object).
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id).expect("the card is in the game").clone()
}

/// `JSON.parse(JSON.stringify(state))`.
fn round_trip(state: &GameState) -> GameState {
    serde_json::from_value(serde_json::to_value(state).expect("a state serialises")).expect("a state parses")
}

fn first(cards: Vec<CardInstance>) -> CardInstance {
    cards.into_iter().next().expect("no card")
}

/// Every event of the actions applied since `before` (`state.applied.slice(before)`).
fn applied_since(state: &GameState, before: usize) -> Vec<GameEvent> {
    state
        .applied
        .iter()
        .skip(before)
        .flat_map(|entry| entry.events.iter().cloned())
        .collect()
}

/// The last applied action's events (`state.applied.at(-1)?.events ?? []`).
fn last_applied(state: &GameState) -> Vec<GameEvent> {
    state
        .applied
        .last()
        .map(|entry| entry.events.clone())
        .unwrap_or_default()
}

mod b5_e38_buffs_and_keywords_in_a_hand_or_a_deck_ride_onto_the_field {
    use super::*;

    #[test]
    fn e38_a_buff_and_a_keyword_given_to_a_hand_card_are_on_it_when_it_is_played() {
        let mut state = playing("e38-hand");
        let card = first(in_hand(&mut state, &plain.id, PlayerId::P1, 1));
        run(
            &mut state,
            &effects::buff(json_as(
                json!({ "target": { "of": "instance", "instanceId": card.id }, "attack": 2, "health": 1 }),
            )),
        );
        run(
            &mut state,
            &effects::grant_keyword(json_as(
                json!({ "target": { "of": "instance", "instanceId": card.id }, "keyword": { "kind": "Rush" } }),
            )),
        );
        let HandView::Cards(hand) = view_for(&state, PlayerId::P1).you.hand else {
            panic!("own hand is a list")
        };
        // R243: its owner sees what it is made of now.
        let seen = hand
            .iter()
            .find(|c| c.instance_id == card.id)
            .expect("the card is in the hand");
        assert_eq!(seen.attack, Some(5));
        assert_eq!(seen.health, Some(4));
        assert_eq!(seen.keywords, Some(vec![Keyword::Rush]));
        let play = play_of(&state, &live(&state, &card.id), None);
        state = act(&state, play, None);
        let unit = on_field(&state, &card.id);
        let view = layers::unit_view(&state, &unit);
        assert_eq!((view.attack, view.max_health), (5, 4));
        assert!(layers::unit_view(&state, &unit).keywords.contains(&Keyword::Rush));
    }

    #[test]
    fn e38_given_in_a_deck_they_ride_the_draw_into_the_hand_and_a_recruit_onto_the_field() {
        let mut state = playing("e38-deck");
        let library = set_library(&mut state, PlayerId::P1, &[plain.id.clone(), body.id.clone()]);
        let (Some(first), Some(second)) = (library.first().cloned(), library.get(1).cloned()) else {
            panic!("no card")
        };
        let events = run(
            &mut state,
            &effects::buff_cards(json_as(json!({ "scope": { "zones": ["library"] }, "attack": 1 }))),
        );
        run(
            &mut state,
            &effects::grant_keyword_cards(json_as(
                json!({ "scope": { "zones": ["library"] }, "keyword": { "kind": "Pierce" } }),
            )),
        );
        // R440: the deck changed silently.
        assert!(events.is_empty());
        {
            // `drawCards(sinkFor(state), "p1", 1)`: TS did not write the cursor back.
            let mut events = Vec::new();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(&mut state, &mut events, &mut rng);
            draw::draw(&mut sink, PlayerId::P1, 1);
        }
        let first = live(&state, &first.id);
        assert_eq!(first.zone.z(), ZoneName::Hand);
        assert_eq!(first.buffs, AttackHealth { attack: 1, health: 0 });
        assert_eq!(first.granted_keywords, vec![Keyword::Pierce]);
        run(&mut state, &effects::recruit(json_as(json!({}))));
        let recruited = on_field(&state, &second.id);
        assert_eq!(layers::unit_view(&state, &recruited).attack, 4);
        assert!(
            layers::unit_view(&state, &recruited)
                .keywords
                .contains(&Keyword::Pierce)
        );
    }

    #[test]
    fn e38_they_go_when_the_card_leaves_the_field_r78_and_when_a_hand_card_reaches_a_graveyard_r215() {
        let mut state = playing("e38-reset");
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let held = first(in_hand(&mut state, &plain.id, PlayerId::P1, 1));
        for card in [&unit, &held] {
            run(
                &mut state,
                &effects::buff(json_as(
                    json!({ "target": { "of": "instance", "instanceId": card.id }, "attack": 2 }),
                )),
            );
            run(
                &mut state,
                &effects::grant_keyword(json_as(
                    json!({ "target": { "of": "instance", "instanceId": card.id }, "keyword": { "kind": "Taunt" } }),
                )),
            );
        }
        let mut moving = live(&state, &unit.id);
        zones::move_to_zone(
            &mut state,
            &mut moving,
            zones::OffFieldZone::Hand,
            Default::default(),
        );
        let mut moving = live(&state, &held.id);
        zones::move_to_zone(
            &mut state,
            &mut moving,
            zones::OffFieldZone::Graveyard,
            Default::default(),
        );
        for card in [&unit, &held] {
            let now = live(&state, &card.id);
            assert_eq!(now.buffs, AttackHealth { attack: 0, health: 0 });
            assert_eq!(now.granted_keywords, Vec::<Keyword>::new());
        }
    }

    /// R1438: a `lucky` rider gives the card Lucky X once it is in the hand, as a granted keyword that
    /// adds to the Lucky it prints (Lucky 1 + 1 = 2), shown to its owner and to no one else, with no
    /// event; it rides onto the field (E38). A full hand burns the card, which is given nothing.
    #[test]
    fn r1438_a_lucky_rider_lands_in_hand_adds_to_printed_lucky_and_rides_onto_the_field() {
        let mut state = playing("r1438-lucky");
        let rider = effects::add_to_hand(json_as(json!({ "defId": numbered_body.id, "lucky": 1 })));
        let events = run(&mut state, &rider);
        let card = state.players.p1.hand.last().cloned().expect("the card is in the hand");
        assert_eq!(card.def_id, numbered_body.id);
        assert_eq!(card.granted_keywords, vec![Keyword::Lucky { n: 1 }]);
        assert_eq!(query::lucky_on(&state, &card), 2);
        assert!(
            !events
                .iter()
                .any(|event| matches!(event, GameEvent::KeywordGranted { .. }))
        );
        let HandView::Cards(hand) = view_for(&state, PlayerId::P1).you.hand else {
            panic!("own hand is a list")
        };
        let seen = hand
            .iter()
            .find(|c| c.instance_id == card.id)
            .expect("the card is in the hand");
        let shown = seen.keywords.clone().expect("its owner sees the Lucky it was given");
        assert_eq!(tuning::numbered_sum(&shown, KeywordKind::Lucky), Some(2));
        let count = state.players.p1.hand.len();
        assert_eq!(
            serde_json::to_value(&view_for(&state, PlayerId::P2).opponent.hand).expect("a view serialises"),
            json!({ "count": count })
        );

        let play = play_of(&state, &card, None);
        state = act(&state, play, None);
        let unit = on_field(&state, &card.id);
        assert_eq!(query::lucky_on(&state, &unit), 2);
        let keywords = layers::unit_view(&state, &unit).keywords;
        assert_eq!(tuning::numbered_sum(&keywords, KeywordKind::Lucky), Some(2));

        let room = config::HAND_CAP - state.players.p1.hand.len() as i32;
        in_hand(&mut state, &body.id, PlayerId::P1, room);
        run(&mut state, &rider);
        let burned = state
            .players
            .p1
            .graveyard
            .last()
            .cloned()
            .expect("the full hand burned the card");
        assert_eq!(burned.def_id, numbered_body.id);
        assert!(burned.granted_keywords.is_empty());
        assert_eq!(query::lucky_on(&state, &burned), 1);
    }

    #[test]
    fn r440_a_scope_over_the_field_a_hand_and_a_deck_reports_its_public_cards_only() {
        let mut state = playing("e38-military");
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let hand_units = in_hand(&mut state, &body.id, PlayerId::P1, 2);
        let spell = first(in_hand(&mut state, &military.id, PlayerId::P1, 1));
        let before = state.applied.len();
        let play = play_of(&state, &spell, None);
        state = act(&state, play, None);
        let events = applied_since(&state, before);
        let buffed: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                GameEvent::Buffed { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(buffed, vec![unit.id.clone()]);
        let granted: Vec<String> = events
            .iter()
            .filter_map(|e| match e {
                GameEvent::KeywordGranted { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(granted, vec![unit.id.clone()]);
        for card in &hand_units {
            let held = state.players.p1.hand.iter().find(|c| c.id == card.id);
            assert_eq!(held.map(|c| c.buffs), Some(AttackHealth { attack: 2, health: 0 }));
            assert_eq!(
                held.map(|c| c.granted_keywords.clone()),
                Some(vec![Keyword::Rush])
            );
        }
        for card in &state.players.p1.library {
            let is_unit = catalog::registered_catalog()
                .get(&card.def_id)
                .is_some_and(|def| def.type_ == CardType::Unit);
            assert_eq!(card.buffs.attack, if is_unit { 2 } else { 0 });
        }
    }
}

mod classic_plus_41_and_a_degrade_after_a_prompt_pauses_and_replays_r113_s9_3_r386 {
    use super::*;

    #[test]
    fn r386_kys_constants_base_face_sets_a_random_number_on_the_chosen_hand_card_to_3() {
        let mut state = playing("constant-base");
        let spell = first(in_hand(&mut state, &constant.id, PlayerId::P1, 1));
        let target = first(in_hand(&mut state, &numbered.id, PlayerId::P1, 1));
        let before = state.applied.len();
        let names_target = |action: &ActionBody| targets_name(action, &target.id);
        let play = play_of(&state, &spell, Some(&names_target));
        state = act(&state, play, None);
        let changed: Vec<GameEvent> = applied_since(&state, before)
            .into_iter()
            .filter(|e| matches!(e, GameEvent::NumberChanged { .. }))
            .collect();
        assert_eq!(changed.len(), 1);
        let Some(GameEvent::NumberChanged {
            instance_id,
            value,
            hidden_from,
            ..
        }) = changed.first()
        else {
            panic!("a numberChanged")
        };
        assert_eq!(*instance_id, target.id);
        assert_eq!(*value, 3);
        assert_eq!(*hidden_from, Some(vec![PlayerId::P2]));
    }

    #[test]
    fn r386_the_radiant_faces_discover_pauses_survives_json_and_resumes_to_the_same_state_in_the_live_game_and_a_copy()
     {
        let mut state = playing("constant-radiant");
        let spell = first(in_hand(&mut state, &constant.id, PlayerId::P1, 1));
        let target = first(in_hand(&mut state, &numbered.id, PlayerId::P1, 1));
        find_instance_mut(&mut state, &spell.id).expect("in hand").radiant = true;
        let names_target = |action: &ActionBody| targets_name(action, &target.id);
        let play = play_of(&state, &live(&state, &spell.id), Some(&names_target));
        state = act(&state, play, None);
        let pending = state.pending.clone().expect("expected the Discover");
        assert_eq!(pending.kind, PromptKind::Discover);
        assert_eq!(pending.options.len(), 3);
        assert_eq!(pending.resume.step, CONSTANT_STEP);
        // Labels are words and numbers, never a raw `{key}` placeholder.
        for option in &pending.options {
            assert!(
                !option.label.contains('{') && !option.label.contains('}'),
                "{}",
                option.label
            );
        }
        let other = view_for(&state, PlayerId::P2);
        assert_eq!(
            serde_json::to_value(&other.pending).expect("a view serialises"),
            json!({ "forYou": false, "pendingFor": "p1" })
        );

        let copy = round_trip(&state);
        let option = pending.options.first().expect("no option");
        let answer = input(
            PlayerId::P1,
            ActionBody::Answer {
                choice_id: pending.id.clone(),
                selection: vec![option.selection.clone()],
            },
        );
        let live_state = act(&state, answer.clone(), Some("a"));
        let resumed = act(&copy, answer, Some("a"));
        assert_eq!(resumed, live_state);
        let held = live_state.players.p1.hand.iter().find(|c| c.id == target.id);
        assert!(held.is_some());
        assert!(live_state.work.is_empty());
        let changed: Vec<i32> = last_applied(&live_state)
            .iter()
            .filter_map(|e| match e {
                GameEvent::NumberChanged { value, .. } => Some(*value),
                _ => None,
            })
            .collect();
        assert_eq!(changed.len(), 1);
        assert_eq!(changed.first(), Some(&3));
    }

    #[test]
    fn r386_a_degrade_after_a_target_prompt_resumes_the_same_from_a_json_copy() {
        let mut state = playing("nerf-pause");
        let victim = put(&mut state, &body.id, slot(PlayerId::P2, Row::Units, 2), json!({}));
        let spell = first(in_hand(&mut state, &nerfer.id, PlayerId::P1, 1));
        let play = play_of(&state, &spell, None);
        state = act(&state, play, None);
        let pending = state.pending.clone().expect("expected the target prompt");
        let option = pending
            .options
            .iter()
            .find(
                |o| matches!(&o.selection, Selection::Instance { instance_id } if *instance_id == victim.id),
            )
            .expect("expected the victim offered");
        let copy = round_trip(&state);
        let answer = input(
            PlayerId::P1,
            ActionBody::Answer {
                choice_id: pending.id.clone(),
                selection: vec![option.selection.clone()],
            },
        );
        let live_state = act(&state, answer.clone(), Some("b"));
        assert_eq!(act(&copy, answer, Some("b")), live_state);
        let degraded: Vec<(String, Option<Vec<PlayerId>>)> = last_applied(&live_state)
            .iter()
            .filter_map(|e| match e {
                GameEvent::Degraded {
                    instance_id,
                    hidden_from,
                    ..
                } => Some((instance_id.clone(), hidden_from.clone())),
                _ => None,
            })
            .collect();
        assert_eq!(degraded.len(), 3);
        assert!(
            degraded
                .iter()
                .all(|(id, hidden)| *id == victim.id && hidden.is_none())
        );
    }
}

struct PlayedOut {
    state: GameState,
    log: Vec<Action>,
    decks: (Vec<String>, Vec<String>),
    tuned: usize,
}

/// §10.7's random policy over the instance-data decks, with a JSON round trip of the state before every
/// action, so every pause the fixtures open — Discovers, target prompts, Deaths that ask — is crossed
/// as plain data. Returns the log and the final state.
fn play_out(seed: &str) -> PlayedOut {
    register_instance_fixtures();
    let decks = (instance_deck(PlayerId::P1), instance_deck(PlayerId::P2));
    let created = create_game(&CreateGameOptions {
        seed: seed.to_string(),
        decks: decks.clone(),
        ..Default::default()
    });
    let mut state = begin_game(&created).state;
    let mut policy = Rng::new(&format!("policy-{seed}"), 0);
    let mut log: Vec<Action> = Vec::new();
    let mut tuned = 0;
    let mut step = 0;
    while state.result.is_none() {
        assert!(step <= 3000, "game {seed} did not finish");
        step += 1;
        state = round_trip(&state);
        let player = seat_to_act(&state).expect("a seat to act");
        let actions: Vec<ActionBody> = legal_actions(&state, player)
            .into_iter()
            .filter(|action| {
                !matches!(
                    action,
                    ActionBody::Concede | ActionBody::OfferDraw | ActionBody::AnswerDraw { .. }
                )
            })
            .collect();
        let others: Vec<&ActionBody> = actions
            .iter()
            .filter(|action| !matches!(action, ActionBody::EndTurn))
            .collect();
        let end_turn = actions
            .iter()
            .find(|action| matches!(action, ActionBody::EndTurn));
        let chosen: ActionBody =
            if others.is_empty() || (end_turn.is_some() && policy.chance(AI_END_TURN_PROBABILITY)) {
                match end_turn {
                    Some(end) => end.clone(),
                    None => others[policy.int(others.len() as i32) as usize].clone(),
                }
            } else {
                others[policy.int(others.len() as i32) as usize].clone()
            };
        let action = Action::new(chosen, player, format!("g{}", log.len()));
        let result = reduce(&state, &action);
        if let Some(error) = &result.error {
            panic!("{} rejected in {seed}: {error}", action.action_type());
        }
        log.push(action);
        tuned += result
            .events
            .iter()
            .filter(|e| {
                matches!(
                    e,
                    GameEvent::Degraded { .. } | GameEvent::Upgraded { .. } | GameEvent::NumberChanged { .. }
                )
            })
            .count();
        state = result.state;
    }
    PlayedOut {
        state,
        log,
        decks,
        tuned,
    }
}

mod s9_3_games_dealt_the_instance_data_fixtures_replay_exactly_r386_r385 {
    use super::*;

    #[test]
    fn r386_six_seeded_games_fold_back_from_their_logs_to_the_same_state_and_they_did_change_cards() {
        let mut tuned = 0;
        for seed in ["id-1", "id-2", "id-3", "id-4", "id-5", "id-6"] {
            let played = play_out(seed);
            let folded = fold(&FoldArgs {
                seed: seed.to_string(),
                decks: played.decks.clone(),
                log: played.log.clone(),
                catalog: Some(catalog::registered_catalog().clone()),
                ..Default::default()
            });
            assert!(folded.errors.is_empty(), "{seed}");
            assert_eq!(hash_state(&folded.state), hash_state(&played.state), "{seed}");
            tuned += played.tuned;
        }
        assert!(tuned > 0);
    }
}
