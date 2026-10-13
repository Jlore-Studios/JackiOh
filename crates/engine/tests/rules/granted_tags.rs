//! Granted tags (docs/meditative-set.md, group B's Systems; MD-B15, R923): `grant_tag` gives a card
//! instance a tag, and `query::tags_of` reads every tag the card carries — its definition's plus the
//! granted ones. Every instance-level tag read goes through it (targets, scopes, declarations,
//! Recruit filters, Discover picks, plays counted by tag); catalog pools never see granted tags.
//! They persist in every zone and through R78's and R766's resets; a copy keeps them, a Fuse unites
//! them, a Transform drops them and a Vanilla keeps them. The view lists them where they differ from
//! the definition's; a placeholder carries none; no event is sent.
//!
//! MD-B16, R924: a face-down card its chooser cannot read is a legal pick for a declaration that
//! filters by tag, whatever its tags, and fizzles if it fails the filter as the card resolves.

use jackioh_engine::effects::{
    add_library_copies, destroy_all, grant_tag, shuffle_into, summon_copy, transform, upgrade, vanilla,
};
use jackioh_engine::play_counts::record_play;
use jackioh_engine::query::{played_this_game_with_tag, tags_of};
use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::plain;
use crate::rules::fixtures::harness::{in_hand, put, set_library, slot};
use crate::rules::fixtures::instance_data::{body, brittle_trap, instance_game};

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

fn grant_id(id: &str) -> Effect {
    grant_tag(json_as(json!({ "instanceId": id, "tag": "CN" })))
}

fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id)
        .cloned()
        .unwrap_or_else(|| panic!("no card {id} in the state"))
}

fn js<T: serde::Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

fn first(cards: Vec<CardInstance>) -> CardInstance {
    cards.into_iter().next().expect("a card")
}

mod r923_granted_tags_md_b15 {
    use super::*;

    #[test]
    fn r923_grant_tag_adds_a_tag_once() {
        let mut state = game("granted-once");
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        assert_eq!(tags_of(&state, &unit), Vec::<Tag>::new());
        // No event: the grant is silent.
        assert_eq!(run(&mut state, grant_id(&unit.id)), Vec::<GameEvent>::new());
        assert_eq!(live(&state, &unit.id).granted_tags, Some(vec![Tag::Cn]));
        assert_eq!(tags_of(&state, &live(&state, &unit.id)), vec![Tag::Cn]);
        // A second grant of a tag the card carries changes nothing and reports nothing.
        assert_eq!(run(&mut state, grant_id(&unit.id)), Vec::<GameEvent>::new());
        assert_eq!(live(&state, &unit.id).granted_tags, Some(vec![Tag::Cn]));
    }

    #[test]
    fn r923_board_and_card_scopes_see_a_granted_tag() {
        let mut state = game("granted-scopes");
        let granted = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let untagged = put(&mut state, &body.id, slot(P1, Row::Units, 2), json!({}));
        run(&mut state, grant_id(&granted.id));
        // `destroy_all` over a board scope with `notTags: [CN]` spares the granted card and marks
        // the untagged one.
        run(
            &mut state,
            destroy_all(json_as(
                json!({ "side": "any", "rows": ["units", "backrow"], "notTags": ["CN"] }),
            )),
        );
        assert_eq!(live(&state, &granted.id).marked_destroyed, None);
        assert_eq!(live(&state, &untagged.id).marked_destroyed, Some(true));
        // `upgrade` over a card scope with `tags: [CN]` reaches the granted card only.
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
        assert_eq!(upgraded, vec![granted.id.clone()]);
    }

    #[test]
    fn r923_it_survives_both_resets_every_zone_and_a_json_round_trip() {
        let mut state = game("granted-resets");
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        run(&mut state, grant_id(&unit.id));
        let mut card = live(&state, &unit.id);
        reset_instance(&mut card);
        reset_price(&mut card);
        assert_eq!(card.granted_tags, Some(vec![Tag::Cn]));
        // R78: bounced to the hand, then to the graveyard, the exile pile and the library, the
        // granted tag stays with the card.
        for zone in [
            OffFieldZone::Hand,
            OffFieldZone::Graveyard,
            OffFieldZone::Exile,
            OffFieldZone::Library,
        ] {
            let mut moving = live(&state, &unit.id);
            move_to_zone(&mut state, &mut moving, zone, MoveToZoneOptions::default());
            assert_eq!(
                live(&state, &unit.id).granted_tags,
                Some(vec![Tag::Cn]),
                "{zone:?}"
            );
        }
        // A JSON round trip keeps it too.
        let back: GameState = serde_json::from_value(js(&state)).expect("a state survives JSON");
        assert_eq!(live(&back, &unit.id).granted_tags, Some(vec![Tag::Cn]));
    }

    #[test]
    fn r923_a_copy_keeps_it_a_transform_drops_it_a_vanilla_keeps_it() {
        let mut state = game("granted-copy");
        let source = put(&mut state, &plain.id, slot(P1, Row::Units, 2), json!({}));
        run(&mut state, grant_id(&source.id));
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
        assert_eq!(copy.granted_tags, Some(vec![Tag::Cn]));

        run(
            &mut state,
            transform(json_as(json!({ "instanceId": source.id, "defId": body.id }))),
        );
        let new_card = card_at(&state, slot(P1, Row::Units, 2))
            .cloned()
            .expect("the new card");
        assert_eq!(new_card.def_id, body.id);
        assert_ne!(new_card.id, source.id);
        assert_eq!(new_card.granted_tags, None);

        run(&mut state, vanilla(json_as(json!({ "instanceId": copy.id }))));
        assert!(live(&state, &copy.id).vanilla);
        assert_eq!(live(&state, &copy.id).granted_tags, Some(vec![Tag::Cn]));
    }

    #[test]
    fn r923_library_and_shuffled_copies_keep_it() {
        let mut state = game("granted-library-copies");
        let deck = first(set_library(&mut state, P2, &[plain.id.as_str()]));
        run(&mut state, grant_id(&deck.id));
        run(
            &mut state,
            add_library_copies(json_as(json!({ "of": "enemy", "count": 1 }))),
        );
        let copy = state.players.p1.hand.last().cloned().expect("a copy in hand");
        assert_eq!(copy.def_id, plain.id);
        assert_eq!(copy.granted_tags, Some(vec![Tag::Cn]));

        let held = first(in_hand(&mut state, &body.id, P1, 1));
        run(&mut state, grant_id(&held.id));
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
                .all(|card| card.granted_tags == Some(vec![Tag::Cn]))
        );
    }

    #[test]
    fn r923_a_fuse_unites_the_ingredients_granted_tags() {
        let mut state = game("granted-fuse");
        let kept = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        run(&mut state, grant_id(&kept.id));
        let mut other = new_instance(&mut state, &body.id, P1, Zone::Gone { player: P1 });
        other.granted_tags = Some(vec![Tag::Felinor]);
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
        // The kept card's own tag first, then the other ingredient's, in order.
        assert_eq!(
            live(&state, &kept.id).granted_tags,
            Some(vec![Tag::Cn, Tag::Felinor])
        );

        // No ingredient with a granted tag: the result carries none.
        let a = new_instance(&mut state, &plain.id, P1, Zone::Gone { player: P1 });
        let b = new_instance(&mut state, &body.id, P1, Zone::Gone { player: P1 });
        let bare = fuse_with(
            &mut state,
            FuseArgs {
                ingredients: vec![a, b],
                to_hand: Some(P1),
                ..FuseArgs::default()
            },
        )
        .expect("the crafted card");
        assert_eq!(live(&state, &bare.id).granted_tags, None);
    }

    #[test]
    fn r923_plays_counted_by_tag_count_a_granted_tag() {
        let mut state = game("granted-counts");
        let unit = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let played = live(&state, &unit.id);
        record_play(&mut state, P1, &played);
        assert_eq!(played_this_game_with_tag(&state, P1, Tag::Cn), 0);
        run(&mut state, grant_id(&unit.id));
        let played = live(&state, &unit.id);
        record_play(&mut state, P1, &played);
        assert_eq!(played_this_game_with_tag(&state, P1, Tag::Cn), 1);
    }

    #[test]
    fn r923_the_view_lists_tags_only_where_they_differ_and_never_on_a_hidden_card() {
        let mut state = game("granted-views");
        let held = first(in_hand(&mut state, &plain.id, P1, 2));
        let granted = state.players.p1.hand[0].clone();
        assert_eq!(held.id, granted.id);
        in_hand(&mut state, &body.id, P1, 1);
        run(&mut state, grant_id(&granted.id));
        let trap = put(&mut state, &brittle_trap.id, slot(P1, Row::Backrow, 1), json!({}));
        run(&mut state, grant_id(&trap.id));

        let mine = js(&view_for(&state, P1));
        assert_eq!(mine["you"]["hand"][0]["tags"], json!(["CN"]));
        // An ungranted card's view carries no `tags` key.
        assert!(mine["you"]["hand"][1].get("tags").is_none());
        // R351: the controller reads their own face-down trap, tags included.
        assert_eq!(mine["you"]["backrow"][0]["tags"], json!(["CN"]));

        // The opponent reads a back and a count: no tags anywhere.
        let theirs = js(&view_for(&state, P2));
        assert_eq!(theirs["opponent"]["hand"], json!({ "count": 3 }));
        assert_eq!(theirs["opponent"]["backrow"][0]["faceDown"], json!(true));
        assert!(!theirs.to_string().contains("\"tags\""), "{theirs}");
    }

    #[test]
    fn r923_no_granted_tag_serialises_without_the_key() {
        let mut state = game("granted-hash");
        let held = first(in_hand(&mut state, &plain.id, P1, 1));
        assert!(js(&held).get("grantedTags").is_none());
        run(&mut state, grant_id(&held.id));
        assert_eq!(js(&live(&state, &held.id))["grantedTags"], json!(["CN"]));
    }
}

mod r924_face_down_picks_md_b16 {
    use super::*;

    #[test]
    fn r924_an_enemy_face_down_backrow_card_is_offered_to_a_tag_filter() {
        let mut state = game("r924-offered");
        // A face-down trap its chooser cannot read, granted CN: still a legal pick for a
        // declaration that bars CN.
        let trap = put(&mut state, &brittle_trap.id, slot(P2, Row::Backrow, 1), json!({}));
        run(&mut state, grant_id(&trap.id));
        // A face-up granted-CN unit: the filter refuses it.
        let foe = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        run(&mut state, grant_id(&foe.id));
        // An untagged face-up unit: offered as usual.
        let other = put(&mut state, &body.id, slot(P2, Row::Units, 2), json!({}));
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
        assert!(offered.contains(&trap.id), "the face-down trap is offered");
        assert!(!offered.contains(&foe.id), "a face-up CN card is refused");
        assert!(offered.contains(&other.id), "an untagged unit is offered");
    }
}
