//! Scraper engine tests (Infinite Assault #169–#192, R4592000–R4592007).
//!
//! Tests for the face-up play history, origin tracking, revealed hands, pick exception lists,
//! library copy cost riders, keyword copying, and transform into copies.

use jackioh_engine::cost_rules::rule_reaches;
use jackioh_engine::effects::choose::{PileSpec, PileZone};
use jackioh_engine::effects::targets::{PlayerSpec, TargetSpec};
use jackioh_engine::effects::{
    AddLibraryCopiesArgs, AddPlayCopiesArgs, ChoosePickArgs, GainKeywordsOfArgs, TransformIntoCopyArgs,
    add_library_copies, add_play_copies, choose_pick, gain_keywords_of, transform_into_copy,
};
use jackioh_engine::ownership::change_owner;
use jackioh_engine::play_counts::record_play;
use jackioh_engine::recursion_log::{RecursionLog, last_plays, last_unit_played, played_this_game};
use jackioh_engine::state::CostRule;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;
use jackioh_engine::wire::{CardType, HandView, Keyword, SetName};

use crate::rules::fixtures::damage_combat::{bolt, gambit, grunt, playing};
use crate::rules::fixtures::harness::{by, in_hand, live, live_mut, put, run_effect, sink_for, slot};

fn first(cards: Vec<CardInstance>) -> CardInstance {
    cards.into_iter().next().expect("at least one card")
}

/// A game that keeps the recursion log, as one holding an Infinite Assault card does (R4589000):
/// the one history the Scraper cards read (R4592000), and the gate of the origin stamp (R4592002).
fn assault_game(seed: &str) -> GameState {
    let mut state = playing(seed);
    state.players.p1.recursion = Some(RecursionLog::default());
    state
}

#[test]
fn r4592000_the_history_keeps_face_up_plays_in_order_and_never_a_set_trap() {
    let mut state = assault_game("scraper-history");
    state.turn = 1;

    let spell = first(in_hand(&mut state, &bolt.id, P1, 1));
    record_play(&mut state, P1, &spell);

    let unit = first(in_hand(&mut state, &grunt.id, P1, 1));
    record_play(&mut state, P1, &unit);

    let trap = first(in_hand(&mut state, &gambit.id, P1, 1));
    record_play(&mut state, P1, &trap);

    let history = played_this_game(&state, P1);
    assert_eq!(history.len(), 2, "Trap is never recorded face-up");
    assert_eq!(history[0].def_id, bolt.id);
    assert_eq!(history[0].type_, CardType::Spell);
    assert_eq!(history[0].turn, 1);
    assert_eq!(history[1].def_id, grunt.id);
    assert_eq!(history[1].type_, CardType::Unit);
    assert_eq!(history[1].turn, 1);

    let last_unit = last_unit_played(&state, P1).expect("last unit");
    assert_eq!(last_unit.def_id, grunt.id);

    let last = last_plays(&state, P1, 1);
    assert_eq!(last.len(), 1);
    assert_eq!(last[0].def_id, grunt.id);
}

#[test]
fn r4592000_nothing_is_recorded_in_a_game_without_an_assault_card() {
    let mut state = playing("scraper-held");
    let unit = first(in_hand(&mut state, &grunt.id, P1, 1));
    record_play(&mut state, P1, &unit);

    assert!(state.players.p1.recursion.is_none());
    assert!(played_this_game(&state, P1).is_empty());
}

#[test]
fn r4592000_add_play_copies_takes_the_newest_in_order_and_one_of_last_turn() {
    let mut state = assault_game("scraper-play-copies");
    state.turn = 1;
    let spell = first(in_hand(&mut state, &bolt.id, P2, 1));
    record_play(&mut state, P2, &spell);

    state.turn = 2;
    let unit = first(in_hand(&mut state, &grunt.id, P2, 1));
    record_play(&mut state, P2, &unit);

    state.turn = 3;
    // Add copies of last 2 plays
    let before_hand = state.players.p1.hand.len();
    run_effect(
        &mut state,
        add_play_copies(AddPlayCopiesArgs {
            of: PlayerSpec::Enemy,
            last: Some(2),
            last_turn: None,
            copies: Some(1),
            cost_mod: None,
        }),
        P1,
    );
    assert_eq!(state.players.p1.hand.len(), before_hand + 2);
    assert_eq!(state.players.p1.hand[before_hand].def_id, bolt.id);
    assert_eq!(state.players.p1.hand[before_hand + 1].def_id, grunt.id);

    // Turn 2 had grunt played by P2; turn 3 is now. Last turn was turn 2.
    let before_hand2 = state.players.p1.hand.len();
    run_effect(
        &mut state,
        add_play_copies(AddPlayCopiesArgs {
            of: PlayerSpec::Enemy,
            last: None,
            last_turn: Some(true),
            copies: Some(2),
            cost_mod: None,
        }),
        P1,
    );
    assert_eq!(state.players.p1.hand.len(), before_hand2 + 2);
    assert_eq!(state.players.p1.hand[before_hand2].def_id, grunt.id);
    assert_eq!(state.players.p1.hand[before_hand2 + 1].def_id, grunt.id);
}

#[test]
fn r4592002_a_stolen_or_swapped_card_started_in_the_other_deck_and_a_copy_never() {
    let mut state = assault_game("scraper-origin");

    // A dealt library card of P2
    let card = state.players.p2.library[0].clone();
    assert_eq!(card.owner, P2);
    assert_eq!(card.controller, P2);
    assert!(!jackioh_engine::origin::started_in_opponents_deck(&card));

    // P1 steals it (owner and controller become P1)
    let mut sink = sink_for(&mut state);
    let changed = change_owner(&mut sink, &card, P1);
    assert!(changed);
    let stolen = live(sink.state, &card.id);
    assert_eq!(stolen.controller, P1);
    assert!(jackioh_engine::origin::started_in_opponents_deck(&stolen));

    // A created card never started in opponent's deck
    let mut created = new_instance(sink.state, &grunt.id, P1, Zone::Hand { player: P1 });
    created.created = Some(true);
    assert!(!jackioh_engine::origin::started_in_opponents_deck(&created));
}

#[test]
fn r4592002_a_rule_of_cards_from_the_opponents_deck_reaches_only_them() {
    let state = assault_game("scraper-cost-rule");

    let rule = CostRule {
        from_opponents_deck: Some(true),
        amount: Some(-1),
        ..CostRule::default()
    };

    let mut from_opp = state.players.p2.library[0].clone();
    jackioh_engine::origin::note_origin(true, &mut from_opp);
    from_opp.owner = P1;
    from_opp.controller = P1;
    assert!(rule_reaches(&state, &rule, &from_opp));

    let mut own_card = state.players.p1.library[0].clone();
    jackioh_engine::origin::note_origin(true, &mut own_card);
    assert!(!rule_reaches(&state, &rule, &own_card));
}

#[test]
fn r4592003_a_revealed_hand_is_in_the_view_and_breaks_no_invariant() {
    let _open = preview_sets(&[SetName::InfiniteAssault]);
    let mut state = playing("scraper-revealed-hand");

    // Put a spy on P1's field with reveals_opponents_hand
    let _spy = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), json!({}));
    let mut scripts = registered_scripts().clone();
    scripts.insert(
        grunt.id.clone(),
        CardScripts {
            base: Script {
                static_flags: Some(StaticFlags {
                    reveals_opponents_hand: Some(true),
                    ..StaticFlags::default()
                }),
                ..Script::default()
            },
            radiant: Script::default(),
        },
    );
    register_scripts(scripts);

    // View for P1 sees P2's hand as Cards!
    let view = view_for(&state, P1);
    match &view.opponent.hand {
        HandView::Cards(cards) => {
            assert_eq!(cards.len(), state.players.p2.hand.len());
        }
        HandView::Count { .. } => panic!("expected HandView::Cards while Ground Truth acts"),
    }
    assert_eq!(view.opponent.hand_marked, None);

    let violations = hidden_information_violations(&state, P1, &view, &legal_actions(&state, P1));
    assert!(violations.is_empty(), "violations: {violations:?}");

    // Kill or remove the spy
    state.players.p1.units[0] = None;
    let view_after = view_for(&state, P1);
    match &view_after.opponent.hand {
        HandView::Count { count } => {
            assert_eq!(*count, state.players.p2.hand.len() as i32);
        }
        HandView::Cards(_) => panic!("expected HandView::Count after spy is gone"),
    }
    let violations_after = hidden_information_violations(&state, P1, &view_after, &legal_actions(&state, P1));
    assert!(
        violations_after.is_empty(),
        "violations after: {violations_after:?}"
    );
}

#[test]
fn r4592004_a_pick_never_offers_an_excepted_id() {
    let mut state = playing("scraper-pick-except");
    let c1 = state.players.p2.library[0].id.clone();
    let c2 = state.players.p2.library[1].id.clone();

    run_effect(
        &mut state,
        choose_pick(ChoosePickArgs {
            step: "pick".to_string(),
            from: vec![PileSpec {
                zone: PileZone::Library,
                player: Some(PlayerSpec::Enemy),
                top: Some(3),
            }],
            except: Some(vec![c1.clone()]),
            min: Some(1),
            max: 1,
            ..ChoosePickArgs::default()
        }),
        P1,
    );

    let pending = state.pending.expect("pending pick prompt");
    let options: Vec<String> = pending
        .options
        .iter()
        .filter_map(|opt| match &opt.selection {
            Selection::Instance { instance_id } => Some(instance_id.clone()),
            _ => None,
        })
        .collect();

    assert!(!options.contains(&c1), "excepted c1 should not be offered");
    assert!(options.contains(&c2), "c2 should be offered");
}

#[test]
fn r4592005_library_copies_carry_the_cost_rider() {
    let mut state = playing("scraper-library-cost");
    run_effect(
        &mut state,
        add_library_copies(AddLibraryCopiesArgs {
            of: PlayerSpec::Enemy,
            count: 1,
            brittle: None,
            cost_mod: Some(-1),
        }),
        P1,
    );

    let copy = state.players.p1.hand.last().expect("added copy");
    assert_eq!(copy.cost_mod, -1);
}

#[test]
fn r4592006_keywords_copied_are_printed_and_granted_never_brittle_or_a_spent_shield() {
    let mut state = playing("scraper-keyword-copy");
    let enemy = put(&mut state, &grunt.id, slot(P2, Row::Units, 1), json!({}));
    let enemy_live = live_mut(&mut state, &enemy.id);
    enemy_live.granted_keywords = vec![Keyword::Taunt, Keyword::DivineShield, Keyword::Brittle { n: 2 }];
    enemy_live.divine_shield_spent = Some(true);

    let friendly = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), json!({}));

    // Friendly runs gain_keywords_of
    let mut sink = sink_for(&mut state);
    apply_effects(
        &[gain_keywords_of(GainKeywordsOfArgs { every: None })],
        &mut make_context(&mut sink, Some(&friendly), by(P1)),
    );

    let live_friendly = live(&sink.state, &friendly.id);
    let kws = jackioh_engine::layers::card_keywords(sink.state, &live_friendly);
    assert!(kws.contains(&Keyword::Taunt));
    assert!(
        !kws.contains(&Keyword::DivineShield),
        "spent shield is not copied"
    );
    assert!(!kws.iter().any(|k| matches!(k, Keyword::Brittle { .. })));
}

#[test]
fn r4592007_a_transform_into_a_copy_keeps_r57_riders_and_immutable_refuses() {
    let mut state = playing("scraper-transform-copy");
    let enemy = put(&mut state, &grunt.id, slot(P2, Row::Units, 1), json!({}));
    let enemy_live = live_mut(&mut state, &enemy.id);
    enemy_live.buffs.attack = 2;
    enemy_live.buffs.health = 2;

    let friendly = put(&mut state, &grunt.id, slot(P1, Row::Units, 1), json!({}));

    run_effect(
        &mut state,
        transform_into_copy(TransformIntoCopyArgs {
            target: TargetSpec::Instance {
                instance_id: friendly.id.clone(),
            },
            of: TargetSpec::Instance {
                instance_id: enemy.id.clone(),
            },
            radiant: None,
        }),
        P1,
    );

    let transformed_id = state.players.p1.units[0]
        .as_ref()
        .unwrap()
        .first()
        .unwrap()
        .id
        .clone();
    assert_ne!(transformed_id, friendly.id);
    let live_transformed = live(&state, &transformed_id);
    assert_eq!(live_transformed.buffs.attack, 2);
    assert_eq!(live_transformed.buffs.health, 2);

    // If target is Immutable, transform is refused
    let immut_live = live_mut(&mut state, &transformed_id);
    immut_live.granted_keywords.push(Keyword::Immutable);

    run_effect(
        &mut state,
        transform_into_copy(TransformIntoCopyArgs {
            target: TargetSpec::Instance {
                instance_id: transformed_id.clone(),
            },
            of: TargetSpec::Instance {
                instance_id: enemy.id.clone(),
            },
            radiant: None,
        }),
        P1,
    );
    let after_immut = live(&state, &transformed_id);
    assert_eq!(after_immut.id, transformed_id, "Immutable refused transform");
}
