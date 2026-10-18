//! R1176000 (issue #1176): every card in Chinese is CN. A card instance whose `chinese` flag is set
//! (R1300: `translate`, the `chinese` riders, a copy of a Chinese card, a crafted Fuse of one) has the
//! CN tag wherever an instance's tags are read: `query::tags_of` adds it, so every reader sees it
//! (scopes, targets, plays counted by tag), and the view lists it as the card's tags. A card is CN
//! once, however many ways it is. Catalog pools read definitions and never see it. The tag follows the
//! flag: a copy that keeps the flag keeps it, a Transform's new card has neither, and a Fuse that keeps
//! an instance keeps that card's own language (R1300), so another ingredient's translation does not
//! make it CN. A replay of a game played before the rule, in which a Chinese card was played or read
//! by its tags, misses its final hash and is refused as `rules_changed` (R767000), never shown as
//! another game.

use jackioh_engine::effects::{destroy_all, grant_tag, summon_copy, transform, upgrade};
use jackioh_engine::play_counts::record_play;
use jackioh_engine::query::{played_this_game_with_tag, tags_of};
use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::vanilla_deck;
use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{in_hand, put, set_library, slot};
use crate::rules::fixtures::instance_data::{body, brittle_trap, instance_game, translator};
use crate::rules::fixtures::scripts::cn_virus;

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

/// Show the card in Chinese, as `translate` does.
fn translate_card(state: &mut GameState, id: &str) {
    run(
        state,
        jackioh_engine::effects::translate(json_as(json!({ "instanceId": id }))),
    );
    assert_eq!(live(state, id).chinese, Some(true), "{id} is in Chinese");
}

fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id)
        .cloned()
        .unwrap_or_else(|| panic!("no card {id} in the state"))
}

fn first(cards: Vec<CardInstance>) -> CardInstance {
    cards.into_iter().next().expect("a card")
}

fn js<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

fn is_cn(state: &GameState, id: &str) -> bool {
    tags_of(state, &live(state, id)).contains(&Tag::Cn)
}

mod r1176000_every_card_in_chinese_is_cn {
    use super::*;

    #[test]
    fn r1176000_the_translator_makes_every_card_it_reaches_cn_and_no_pool_changes() {
        let mut state = game("cn-translator");
        state.players.p1.mana = ManaState {
            current: 4,
            max: 4,
            ..state.players.p1.mana
        };
        let spell = first(in_hand(&mut state, &translator.id, P1, 1));
        in_hand(&mut state, &plain.id, P1, 1);
        in_hand(&mut state, &body.id, P2, 2);
        let pool_before = jackioh_engine::catalog::query(&json_as(json!({ "tags": ["CN"] })));
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
        let cards: Vec<&CardInstance> = [
            &state.players.p1.hand,
            &state.players.p2.hand,
            &state.players.p1.library,
            &state.players.p2.library,
        ]
        .into_iter()
        .flat_map(|pile| pile.iter())
        .collect();
        assert!(cards.len() > 3);
        for card in cards {
            assert_eq!(card.chinese, Some(true), "{}", card.id);
            assert_eq!(
                tags_of(&state, card),
                vec![Tag::Cn],
                "{} {} is CN",
                card.id,
                card.def_id
            );
            assert!(card.granted_tags.is_none(), "the tag is the flag's, not a grant");
        }
        // The Spell was resolving, in neither pile: still English, so not CN.
        assert!(!is_cn(&state, &spell.id));
        // A pool reads printed tags: the translated cards join no CN pool.
        let pool_after = jackioh_engine::catalog::query(&json_as(json!({ "tags": ["CN"] })));
        assert_eq!(js(&pool_after), js(&pool_before));
        assert!(
            !pool_after
                .iter()
                .any(|def| def.id == plain.id || def.id == body.id)
        );
    }

    #[test]
    fn r1176000_scopes_targets_and_plays_counted_by_tag_see_it() {
        let mut state = game("cn-reads");
        let translated = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let english = put(&mut state, &body.id, slot(P1, Row::Units, 2), json!({}));
        translate_card(&mut state, &translated.id);

        // A target declaration that bars CN refuses the translated unit and offers the other.
        let declarer = first(in_hand(&mut state, &plain.id, P1, 1));
        let decl = TargetDecl::target(
            1,
            1,
            json!({ "side": "any", "of": ["unit", "backrow"], "notTags": ["CN"] }),
        );
        let offered: Vec<String> = play_choices::legal_selections_for(&state, P1, &declarer, &decl)
            .into_iter()
            .filter_map(|selection| match selection {
                Selection::Instance { instance_id } => Some(instance_id),
                _ => None,
            })
            .collect();
        assert!(!offered.contains(&translated.id), "a CN card is refused");
        assert!(offered.contains(&english.id), "an untagged one is offered");

        // A play is counted by the tags the card had as it was played, CN among them.
        let held = first(in_hand(&mut state, &body.id, P1, 1));
        let english_play = live(&state, &held.id);
        record_play(&mut state, P1, &english_play);
        assert_eq!(played_this_game_with_tag(&state, P1, Tag::Cn), 0);
        translate_card(&mut state, &held.id);
        let chinese_play = live(&state, &held.id);
        record_play(&mut state, P1, &chinese_play);
        assert_eq!(played_this_game_with_tag(&state, P1, Tag::Cn), 1);

        // A card scope with `tags: [CN]` reaches the translated unit only.
        let events = run(
            &mut state,
            upgrade(json_as(
                json!({ "scope": { "side": "any", "zones": ["field"], "tags": ["CN"] }, "times": 1 }),
            )),
        );
        let upgraded: Vec<String> = events
            .iter()
            .filter_map(|event| match event {
                GameEvent::Upgraded { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(upgraded, vec![translated.id.clone()]);

        // A board scope with `notTags: [CN]` spares it and marks the other.
        run(
            &mut state,
            destroy_all(json_as(
                json!({ "side": "any", "rows": ["units", "backrow"], "notTags": ["CN"] }),
            )),
        );
        assert_eq!(live(&state, &translated.id).marked_destroyed, None);
        assert_eq!(live(&state, &english.id).marked_destroyed, Some(true));
    }

    #[test]
    fn r1176000_a_card_cn_twice_over_is_cn_once() {
        let mut state = game("cn-once");
        // Granted CN (M 35) and in Chinese: one CN.
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        run(
            &mut state,
            grant_tag(json_as(json!({ "instanceId": unit.id, "tag": "CN" }))),
        );
        translate_card(&mut state, &unit.id);
        assert_eq!(tags_of(&state, &live(&state, &unit.id)), vec![Tag::Cn]);
        // Printed CN and in Chinese: its printed tags, as they stand, and no view key.
        let virus = first(in_hand(&mut state, &cn_virus().id, P1, 1));
        translate_card(&mut state, &virus.id);
        assert_eq!(
            tags_of(&state, &live(&state, &virus.id)),
            vec![Tag::Token, Tag::Cn]
        );
        let mine = js(&view_for(&state, P1));
        let shown = mine["you"]["hand"]
            .as_array()
            .expect("a hand")
            .iter()
            .find(|card| card["instanceId"] == json!(virus.id))
            .cloned()
            .expect("the virus in hand");
        assert!(shown.get("tags").is_none(), "{shown}");
    }

    #[test]
    fn r1176000_the_view_lists_it_to_every_seat_that_reads_the_card() {
        let mut state = game("cn-views");
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        translate_card(&mut state, &unit.id);
        let held = first(in_hand(&mut state, &body.id, P1, 1));
        translate_card(&mut state, &held.id);
        let trap = put(&mut state, &brittle_trap.id, slot(P1, Row::Backrow, 1), json!({}));
        translate_card(&mut state, &trap.id);
        let deck = first(set_library(&mut state, P1, &[body.id.as_str()]));
        translate_card(&mut state, &deck.id);

        // The unit: both seats read it, so both views list its CN.
        let mine = js(&view_for(&state, P1));
        let theirs = js(&view_for(&state, P2));
        assert_eq!(mine["you"]["units"][0]["tags"], json!(["CN"]));
        assert_eq!(theirs["opponent"]["units"][0]["tags"], json!(["CN"]));
        // The hand card and the face-down trap: their controller's alone (R1301, R351).
        assert_eq!(mine["you"]["hand"][0]["tags"], json!(["CN"]));
        assert_eq!(mine["you"]["backrow"][0]["tags"], json!(["CN"]));
        assert_eq!(theirs["opponent"]["hand"], json!({ "count": 1 }));
        assert_eq!(theirs["opponent"]["backrow"][0]["faceDown"], json!(true));
        assert!(theirs["opponent"]["backrow"][0].get("tags").is_none());
        // The deck card is a count to both, and the opponent reads no tag but the unit's.
        let unit_tags = theirs["opponent"]["units"][0]["tags"].to_string();
        let without_unit = theirs
            .to_string()
            .replacen(&format!("\"tags\":{unit_tags}"), "", 1);
        assert!(!without_unit.contains("\"tags\""), "{theirs}");
    }

    #[test]
    fn r1176000_the_tag_follows_the_flag_through_copies_transforms_and_fuses() {
        let mut state = game("cn-copies");
        let source = put(&mut state, &plain.id, slot(P1, Row::Units, 2), json!({}));
        translate_card(&mut state, &source.id);
        // A copy keeps the flag, so it is CN.
        run(
            &mut state,
            summon_copy(json_as(
                json!({ "of": { "of": "instance", "instanceId": source.id } }),
            )),
        );
        let copy = card_at(&state, slot(P1, Row::Units, 1))
            .cloned()
            .expect("the copy");
        assert!(is_cn(&state, &copy.id));
        // A Transform's new card has neither (R1300, R923).
        run(
            &mut state,
            transform(json_as(json!({ "instanceId": source.id, "defId": body.id }))),
        );
        let new_card = card_at(&state, slot(P1, Row::Units, 2))
            .cloned()
            .expect("the new card");
        assert_eq!(new_card.def_id, body.id);
        assert!(!is_cn(&state, &new_card.id));

        let fuse_with = |state: &mut GameState, args: FuseArgs| -> Option<CardInstance> {
            let mut events = Vec::new();
            let mut rng = Rng::new(&state.seed, state.rng_cursor);
            let mut sink = EngineSink::new(state, &mut events, &mut rng);
            fuse(&mut sink, args)
        };
        // A Fuse that keeps an English card keeps its language: a Chinese ingredient does not make
        // it CN.
        let kept = live(&state, &new_card.id);
        let mut chinese = new_instance(&mut state, &plain.id, P1, Zone::Gone { player: P1 });
        chinese.chinese = Some(true);
        let fused = fuse_with(
            &mut state,
            FuseArgs {
                ingredients: vec![kept.clone(), chinese.clone()],
                target: Some(kept),
                ..FuseArgs::default()
            },
        )
        .expect("the fusion happens");
        assert_eq!(fused.id, new_card.id);
        assert!(!is_cn(&state, &fused.id));
        // A crafted card is Chinese when an ingredient was, so it is CN.
        let mut other = new_instance(&mut state, &plain.id, P1, Zone::Gone { player: P1 });
        other.chinese = Some(true);
        let english = new_instance(&mut state, &body.id, P1, Zone::Gone { player: P1 });
        let crafted = fuse_with(
            &mut state,
            FuseArgs {
                ingredients: vec![other, english],
                to_hand: Some(P1),
                ..FuseArgs::default()
            },
        )
        .expect("the crafted card");
        assert!(is_cn(&state, &crafted.id));
    }
}

mod r1176000_r767000_a_game_from_before_the_rule {
    use super::*;

    /// The build's catalog version, and the one the game below records.
    const BUILD: &str = "v-cn";

    /// A game from a deck of one translator and the vanilla fixtures against the vanilla fixtures,
    /// each seat taking its first legal play, then its first attack, then the end of its turn, until a
    /// card in Chinese has been played (`None` if the game ends first): its state, its log and its
    /// decks.
    fn played(seed: &str) -> Option<(GameState, Vec<Action>, (Vec<String>, Vec<String>))> {
        let mut mine = vec![translator.id.clone()];
        mine.extend(vanilla_deck(DECK_SIZE - 1, 1));
        let decks = (mine, vanilla_deck(DECK_SIZE, 21));
        let mut state = begin_game(&instance_game(seed, Some(decks.clone()))).state;
        let mut log: Vec<Action> = Vec::new();
        while played_this_game_with_tag(&state, P1, Tag::Cn) == 0 {
            if state.result.is_some() {
                return None;
            }
            let player = seat_to_act(&state).unwrap_or(state.active);
            let actions = legal_actions(&state, player);
            let first_of = |types: &[ActionType]| {
                actions
                    .iter()
                    .find(|action| types.contains(&action.action_type()))
                    .cloned()
            };
            // Every play there is before the turn ends, so the translator and the cards after it
            // are played; a prompt or the mulligan is answered with its first option.
            let chosen = first_of(&[ActionType::Mulligan, ActionType::Answer, ActionType::Play])
                .or_else(|| first_of(&[ActionType::Attack]))
                .or_else(|| first_of(&[ActionType::EndTurn]))
                .unwrap_or_else(|| panic!("no action to take for {player} in game {seed}"));
            let action = Action::new(chosen, player, format!("a{}", log.len()));
            let result = reduce(&state, &action);
            assert_eq!(result.error, None, "{seed}");
            log.push(action);
            state = result.state;
        }
        Some((state, log, decks))
    }

    #[test]
    fn r1176000_r767000_a_fold_that_misses_its_hash_is_refused_not_shown() {
        let (seed, (live, log, decks)) = (0..jackioh_engine::config::SEED_SEARCH_BOUND)
            .map(|n| format!("cn-replay-{n}"))
            .find_map(|seed| played(&seed).map(|game| (seed, game)))
            .expect("a game in which a Chinese card is played");
        // The same game as a build before the rule recorded it: the one thing that differs is the CN
        // the translated plays counted, since nothing else in these decks reads a tag.
        let mut before = live.clone();
        for player in PLAYER_IDS {
            if let Some(log) = before.players[player].game_log.as_mut() {
                log.played_by_tag.shift_remove(&Tag::Cn);
            }
        }
        let recorded_before = hash_state(&before);
        assert_ne!(recorded_before, hash_state(&live));

        let input = ReplayInput {
            seed,
            decks,
            log,
            ..Default::default()
        };
        let record = |hash: &str| ReplayRecord {
            catalog_version: BUILD.to_string(),
            final_hash: hash.to_string(),
        };
        // Recorded under the rule, the fold reaches its hash and the replay opens.
        assert!(matches!(
            replay_open(&input, &record(&hash_state(&live)), BUILD),
            ReplayOpen::Ready { .. }
        ));
        // Recorded before it, the fold misses the hash: refused as `rules_changed`, never shown.
        assert_eq!(
            replay_open(&input, &record(&recorded_before), BUILD),
            ReplayOpen::Refused {
                reason: ReplayRefusal::RulesChanged
            }
        );
    }
}
