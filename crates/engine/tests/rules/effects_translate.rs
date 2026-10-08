//! ME-CN's verb and flag (docs/meditative-set.md, group B's Systems, ME-CN; MD-B11, MD-B12): `translate`
//! shows a card in Chinese from now on, over a named card or a card scope. The flag is presentation
//! only (R1300): Immutable does not stop it, nothing removes it, a copy keeps it and a Transform's new
//! card is without it. It is public as the card is (R1301): on every view of a card the viewer may
//! read and on no hidden one, and `translated` reaches both players only where both read the card.

use jackioh_engine::effects::{add_library_copies, shuffle_into, summon_copy, transform, translate};
use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{in_hand, put, set_library, slot};
use crate::rules::fixtures::instance_data::{body, brittle_trap, instance_game, stoic, translator};

const P1: PlayerId = PlayerId::P1;
const P2: PlayerId = PlayerId::P2;

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

fn translate_id(id: &str) -> Effect {
    translate(json_as(json!({ "instanceId": id })))
}

fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id)
        .cloned()
        .unwrap_or_else(|| panic!("no card {id} in the state"))
}

fn mark(state: &mut GameState, id: &str) {
    find_instance_mut(state, id)
        .unwrap_or_else(|| panic!("no card {id} in the state"))
        .chinese = Some(true);
}

fn first(cards: Vec<CardInstance>) -> CardInstance {
    cards.into_iter().next().expect("a card")
}

fn js<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// The events of one action, as each player's view sends them.
fn sent(state: &mut GameState, events: Vec<GameEvent>, viewer: PlayerId) -> Value {
    state.applied = vec![AppliedAction {
        nonce: "t".to_string(),
        events,
    }];
    js(&view_for(state, viewer).events)
}

mod r1300_the_flag_and_its_verb {
    use super::*;

    #[test]
    fn r1300_translate_marks_a_named_field_card_and_both_players_see_it() {
        let mut state = game("translate-field");
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let events = run(&mut state, translate_id(&unit.id));
        assert_eq!(live(&state, &unit.id).chinese, Some(true));
        assert_eq!(
            events,
            vec![GameEvent::Translated {
                instance_id: unit.id.clone()
            }]
        );
        for viewer in [P1, P2] {
            let view = js(&view_for(&state, viewer));
            let side = if viewer == P1 { "you" } else { "opponent" };
            assert_eq!(view[side]["units"][0]["chinese"], json!(true));
        }
        // Already Chinese: nothing changes and nothing is reported.
        assert_eq!(run(&mut state, translate_id(&unit.id)), Vec::<GameEvent>::new());
    }

    #[test]
    fn r1300_over_a_scope_it_reaches_both_hands_and_decks_silently_r440() {
        let mut state = game("translate-scope");
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let mut reached = in_hand(&mut state, &plain.id, P1, 2);
        reached.extend(in_hand(&mut state, &body.id, P2, 1));
        reached.extend(set_library(
            &mut state,
            P1,
            &[plain.id.as_str(), body.id.as_str()],
        ));
        reached.extend(set_library(&mut state, P2, &[plain.id.as_str()]));
        let events = run(
            &mut state,
            translate(json_as(
                json!({ "scope": { "side": "any", "zones": ["hand", "library"] } }),
            )),
        );
        for card in &reached {
            assert_eq!(live(&state, &card.id).chinese, Some(true), "{}", card.id);
        }
        // The field is not in the scope.
        assert_eq!(live(&state, &unit.id).chinese, None);
        // Nobody reads both hands and both decks, so nothing is reported.
        assert_eq!(events, Vec::<GameEvent>::new());
    }

    #[test]
    fn r1300_the_translator_fixture_translates_both_hands_and_decks() {
        let mut state = game("translate-fixture");
        state.players.p1.mana = ManaState {
            current: 4,
            max: 4,
            ..state.players.p1.mana
        };
        let spell = first(in_hand(&mut state, &translator.id, P1, 1));
        in_hand(&mut state, &plain.id, P1, 1);
        in_hand(&mut state, &body.id, P2, 2);
        let play = legal_actions(&state, P1)
            .into_iter()
            .find(|action| matches!(action, ActionBody::Play { instance_id, .. } if *instance_id == spell.id))
            .expect("the translator is playable");
        let result = reduce(
            &state,
            &ActionInput {
                body: play,
                player_id: P1,
            }
            .with_nonce("translate".to_string()),
        );
        assert_eq!(result.error, None);
        let state = result.state;
        let piles = [
            &state.players.p1.hand,
            &state.players.p2.hand,
            &state.players.p1.library,
            &state.players.p2.library,
        ];
        assert!(piles.iter().all(|pile| !pile.is_empty()));
        let cards: Vec<&CardInstance> = piles.iter().flat_map(|pile| pile.iter()).collect();
        for card in cards {
            assert_eq!(card.chinese, Some(true), "{} {}", card.id, card.def_id);
        }
        // The Spell itself was resolving, in neither pile, so it stays as it was.
        assert_eq!(live(&state, &spell.id).chinese, None);
        assert!(
            !result
                .events
                .iter()
                .any(|event| event.event_type() == GameEventType::Translated)
        );
    }

    #[test]
    fn r1300_an_immutable_card_is_translated() {
        let mut state = game("translate-immutable");
        let unit = put(&mut state, &stoic.id, slot(P1, Row::Units, 1), json!({}));
        let events = run(&mut state, translate_id(&unit.id));
        assert_eq!(live(&state, &unit.id).chinese, Some(true));
        assert_eq!(events.len(), 1);
    }

    #[test]
    fn r1300_a_card_that_has_ceased_to_exist_is_left_alone() {
        let mut state = game("translate-gone");
        let mut held = first(in_hand(&mut state, &plain.id, P1, 1));
        state.players.p1.hand = vec![];
        held.zone = Zone::Gone { player: P1 };
        let events = run(
            &mut state,
            translate(json_as(
                json!({ "target": { "of": "instance", "instanceId": held.id } }),
            )),
        );
        assert_eq!(events, Vec::<GameEvent>::new());
        assert!(find_instance(&state, &held.id).is_none());
    }

    #[test]
    fn r1300_the_flag_survives_reset_instance_and_reset_price_and_every_zone() {
        let mut state = game("translate-resets");
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        run(&mut state, translate_id(&unit.id));
        let mut card = live(&state, &unit.id);
        reset_instance(&mut card);
        reset_price(&mut card);
        assert_eq!(card.chinese, Some(true));
        // R78: bounced to the hand, then to the graveyard and the exile pile, it stays Chinese.
        for zone in [
            OffFieldZone::Hand,
            OffFieldZone::Graveyard,
            OffFieldZone::Exile,
            OffFieldZone::Library,
        ] {
            let mut moving = live(&state, &unit.id);
            move_to_zone(&mut state, &mut moving, zone, MoveToZoneOptions::default());
            assert_eq!(live(&state, &unit.id).chinese, Some(true), "{zone:?}");
        }
    }

    #[test]
    fn r1300_a_card_never_translated_serialises_without_the_key() {
        let mut state = game("translate-hash");
        let held = first(in_hand(&mut state, &plain.id, P1, 1));
        assert!(js(&held).get("chinese").is_none());
        let record = FaceUpRecord {
            def_id: plain.id.clone(),
            radiant: false,
            type_: CardType::Unit,
            chinese: None,
        };
        assert!(js(&record).get("chinese").is_none());
        let view = js(&view_for(&state, P1));
        assert!(!view.to_string().contains("chinese"));
        run(&mut state, translate_id(&held.id));
        assert_eq!(js(&live(&state, &held.id))["chinese"], json!(true));
    }
}

mod r1300_copies_transforms_and_fuses_md_b11 {
    use super::*;

    #[test]
    fn r1300_a_copy_is_chinese_a_transform_s_new_card_is_not() {
        let mut state = game("translate-copy");
        let source = put(&mut state, &plain.id, slot(P1, Row::Units, 2), json!({}));
        mark(&mut state, &source.id);
        run(
            &mut state,
            summon_copy(json_as(
                json!({ "of": { "of": "instance", "instanceId": source.id } }),
            )),
        );
        let copy = card_at(&state, slot(P1, Row::Units, 1))
            .cloned()
            .expect("the copy");
        assert_ne!(copy.id, source.id);
        assert_eq!(copy.chinese, Some(true));

        run(
            &mut state,
            transform(json_as(json!({ "instanceId": source.id, "defId": body.id }))),
        );
        let new_card = card_at(&state, slot(P1, Row::Units, 2))
            .cloned()
            .expect("the new card");
        assert_eq!(new_card.def_id, body.id);
        assert_ne!(new_card.id, source.id);
        assert_eq!(new_card.chinese, None);
    }

    #[test]
    fn r1300_library_and_shuffled_copies_keep_it() {
        let mut state = game("translate-library-copies");
        let deck = first(set_library(&mut state, P2, &[plain.id.as_str()]));
        mark(&mut state, &deck.id);
        run(
            &mut state,
            add_library_copies(json_as(json!({ "of": "enemy", "count": 1 }))),
        );
        let copy = state.players.p1.hand.last().cloned().expect("a copy in hand");
        assert_eq!(copy.def_id, plain.id);
        assert_eq!(copy.chinese, Some(true));

        let held = first(in_hand(&mut state, &body.id, P1, 1));
        mark(&mut state, &held.id);
        set_library(&mut state, P1, &[] as &[&str]);
        run(
            &mut state,
            shuffle_into(json_as(
                json!({ "defId": body.id, "count": 2, "copyOf": held.id }),
            )),
        );
        assert_eq!(state.players.p1.library.len(), 2);
        assert!(
            state
                .players
                .p1
                .library
                .iter()
                .all(|card| card.chinese == Some(true))
        );
        // A card the text names, not a copy, is made as printed.
        run(
            &mut state,
            shuffle_into(json_as(json!({ "defId": body.id, "count": 1 }))),
        );
        assert_eq!(
            state
                .players
                .p1
                .library
                .iter()
                .filter(|card| card.chinese.is_none())
                .count(),
            1
        );
    }

    #[test]
    fn r1300_a_fuse_keeps_the_kept_card_s_flag_and_a_crafted_card_is_chinese_when_an_ingredient_was() {
        let mut state = game("translate-fuse");
        let kept = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        mark(&mut state, &kept.id);
        let other = new_instance(&mut state, &body.id, P1, Zone::Gone { player: P1 });
        let fuse_with = |state: &mut GameState, args: FuseArgs| -> Option<CardInstance> {
            let mut events = Vec::new();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            fuse(&mut sink, args)
        };
        let target = live(&state, &kept.id);
        let fused = fuse_with(
            &mut state,
            FuseArgs {
                ingredients: vec![target.clone(), other.clone()],
                target: Some(target),
                ..FuseArgs::default()
            },
        )
        .expect("the fusion happens");
        assert_eq!(fused.id, kept.id);
        assert_eq!(live(&state, &kept.id).chinese, Some(true));

        let mut chinese = new_instance(&mut state, &plain.id, P1, Zone::Gone { player: P1 });
        chinese.chinese = Some(true);
        let plain_one = new_instance(&mut state, &body.id, P1, Zone::Gone { player: P1 });
        let crafted = fuse_with(
            &mut state,
            FuseArgs {
                ingredients: vec![chinese, plain_one.clone()],
                to_hand: Some(P1),
                ..FuseArgs::default()
            },
        )
        .expect("the crafted card");
        assert_eq!(live(&state, &crafted.id).chinese, Some(true));

        let a = new_instance(&mut state, &plain.id, P1, Zone::Gone { player: P1 });
        let b = new_instance(&mut state, &body.id, P1, Zone::Gone { player: P1 });
        let english = fuse_with(
            &mut state,
            FuseArgs {
                ingredients: vec![a, b],
                to_hand: Some(P1),
                ..FuseArgs::default()
            },
        )
        .expect("the crafted card");
        assert_eq!(live(&state, &english.id).chinese, None);
    }
}

mod r1301_who_sees_it {
    use super::*;

    #[test]
    fn r1301_the_flag_is_on_every_readable_view_and_never_on_a_hidden_card() {
        let mut state = game("translate-views");
        let held = first(in_hand(&mut state, &plain.id, P1, 1));
        let trap = put(&mut state, &brittle_trap.id, slot(P1, Row::Backrow, 1), json!({}));
        let deck = first(set_library(&mut state, P1, &[plain.id.as_str()]));
        for id in [&held.id, &trap.id, &deck.id] {
            mark(&mut state, id);
        }
        state.pending = Some(PendingChoice {
            id: "q-test".into(),
            player_id: P1,
            kind: PromptKind::Target,
            prompt: "a question".into(),
            options: vec![PromptOption {
                key: format!("instance:{}", held.id),
                label: "a card".into(),
                selection: Selection::Instance {
                    instance_id: held.id.clone(),
                },
                cost: None,
                radiant: None,
            }],
            min: 1,
            max: 1,
            budget: None,
            resume: Resume {
                def_id: String::new(),
                hook: "resume".into(),
                step: "x".into(),
                radiant: false,
                instance_id: None,
                data: IndexMap::new(),
            },
        });

        let mine = js(&view_for(&state, P1));
        assert_eq!(mine["you"]["hand"][0]["chinese"], json!(true));
        // R351: the controller reads their own face-down trap.
        assert_eq!(mine["you"]["backrow"][0]["chinese"], json!(true));
        assert_eq!(mine["pending"]["options"][0]["chinese"], json!(true));
        // R311: the owner's library list keeps its records as they were shown.
        assert!(!mine["you"]["ownLibrary"].to_string().contains("chinese"));

        let theirs = js(&view_for(&state, P2));
        assert_eq!(theirs["opponent"]["hand"], json!({ "count": 1 }));
        assert_eq!(theirs["opponent"]["backrow"][0]["faceDown"], json!(true));
        assert!(!theirs.to_string().contains("chinese"), "{theirs}");
    }

    #[test]
    fn r1301_translated_reaches_both_players_with_the_id_hidden_where_unreadable() {
        let mut state = game("translate-event");
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let events = run(&mut state, translate_id(&unit.id));
        assert_eq!(
            sent(&mut state, events.clone(), P2),
            json!([{ "type": "translated", "instanceId": unit.id }])
        );
        // Bounced since, the card is in its owner's hand, which only its owner reads (R97).
        let mut moving = live(&state, &unit.id);
        move_to_zone(
            &mut state,
            &mut moving,
            OffFieldZone::Hand,
            MoveToZoneOptions::default(),
        );
        assert_eq!(
            sent(&mut state, events.clone(), P1),
            json!([{ "type": "translated", "instanceId": unit.id }])
        );
        assert_eq!(
            sent(&mut state, events, P2),
            json!([{ "type": "translated", "instanceId": HIDDEN_ID }])
        );
    }
}
