//! The target and scope vocabulary every other verb's arguments are written in (SPEC §3.1, §3.2,
//! §6.3, §10.6, R13, R68, R81). Two things are under test: `{ of: "instance" }`, the TargetSpec
//! variant that lets a verb name a card a script already holds the id of, and the board scope
//! (`cardsInScope`, `adjacentTo`, `matchesScope`) that the board-wide verbs are thin walks over.
//!
//! The scope lives here rather than on `TargetSpec` because `resolveTarget` answers with one
//! `DamageTarget | null`: every verb written in a TargetSpec — damage, destroy, buff, transform —
//! is single-target by construction, so a multi-card spec could not be threaded through them. The
//! fixture cards these tests need are registered here, so no shared fixture has to grow (BUILD §0).
//!
//! Port of `packages/engine/test/effects-targets.test.ts`.

use std::borrow::Borrow;

use jackioh_engine::effects::{
    BoardScope, TargetSpec, adjacent_to, cards_in_scope, instance_of, matches_scope, resolve_target,
};
use jackioh_engine::testkit::*;
use serde::Serialize;

use super::fixtures::catalog as catalog_fx;
use super::fixtures::harness::{PutOptions, new_game, put, slot};

// ---------------------------------------------------------------------------
// Fixture cards: two tags and a backrow type, so every filter has something to bite on.
// ---------------------------------------------------------------------------

fn human() -> CardDef {
    catalog_fx::unit_def(760, json!({ "id": "tg-human", "name": "Human (fixture)", "tags": ["Human"] }))
}

fn felinor() -> CardDef {
    catalog_fx::unit_def(761, json!({ "id": "tg-felinor", "name": "Felinor (fixture)", "tags": ["Felinor"] }))
}

fn untagged() -> CardDef {
    catalog_fx::unit_def(762, json!({ "id": "tg-untagged", "name": "Untagged (fixture)" }))
}

fn stackable() -> CardDef {
    catalog_fx::unit_def(763, json!({ "id": "tg-stackable", "name": "Stackable (fixture)" }))
}

/// TS `{ ...spellDef(764, { id: "tg-field", name: "Field Spell (fixture)" }), type: "Field Spell" }`.
fn field_spell() -> CardDef {
    let mut def = catalog_fx::spell_def(764, json!({ "id": "tg-field", "name": "Field Spell (fixture)" }));
    def.type_ = CardType::FieldSpell;
    def
}

/// TS `game(seed = "targets-test")`: every test here uses the default.
fn game() -> GameState {
    let state = new_game("targets-test", None);
    let mut catalog = registered_catalog().clone();
    for def in [human(), felinor(), untagged(), stackable(), field_spell()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    state
}

/// TS `ctxOf(state, self = null)`: a context over `sinkFor(state)` with p1 as the controller. A
/// Rust context borrows the state, so the test's reads run inside `f`.
fn with_ctx<R>(state: &mut GameState, self_: Option<&CardInstance>, f: impl FnOnce(&mut EffectContext<'_>) -> R) -> R {
    let self_ = self_.cloned();
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    let mut ctx = make_context(
        &mut sink,
        self_.as_ref(),
        HookOptions {
            controller: Some(PlayerId::P1),
            ..HookOptions::default()
        },
    );
    f(&mut ctx)
}

/// TS `idsOf(cards)`; takes the cards owned or lent.
fn ids_of<C: Borrow<CardInstance>>(cards: &[C]) -> Vec<String> {
    cards.iter().map(|card| card.borrow().id.clone()).collect()
}

/// `{ of: "instance", instanceId }`.
fn instance_spec(instance_id: &str) -> TargetSpec {
    json_as(json!({ "of": "instance", "instanceId": instance_id }))
}

/// A board scope from TS's object literal; `{}` is TS's default.
fn scope(literal: Value) -> BoardScope {
    json_as(literal)
}

/// TS held the live instance; Rust reads the card again by id.
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id).cloned().unwrap_or_else(|| panic!("no card {id} in the state"))
}

fn to_json<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

// ---------------------------------------------------------------------------
// `{ of: "instance" }`
// ---------------------------------------------------------------------------

mod target_spec_of_instance {
    use super::*;

    #[test]
    fn resolves_a_card_by_the_id_a_script_captured_wherever_it_is() {
        let mut state = game();
        let on_field = put(&mut state, &human().id, slot(PlayerId::P2, Row::Units, 3), PutOptions::default());
        let on_field = live(&state, &on_field.id);

        with_ctx(&mut state, None, |ctx| {
            match resolve_target(ctx, &instance_spec(&on_field.id)) {
                Some(DamageTarget::Unit { instance, .. }) => assert_eq!(instance, on_field),
                _ => panic!("expected {{ kind: \"unit\", instance }} for {}", on_field.id),
            }
            assert_eq!(
                to_json(instance_of(ctx, &instance_spec(&on_field.id))),
                to_json(Some(&on_field))
            );
        });
    }

    #[test]
    fn resolves_a_card_that_has_left_the_field_so_a_delayed_step_can_still_name_it_r76() {
        let mut state = game();
        let card = put(&mut state, &human().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());
        // The card dies; the id a Resume captured must still find it in the graveyard.
        let mut dead = live(&state, &card.id);
        state.players.p1.units[0] = None;
        dead.zone = Zone::Graveyard { player: PlayerId::P1 };
        state.players.p1.graveyard.push(dead.clone());

        let found = with_ctx(&mut state, None, |ctx| to_json(instance_of(ctx, &instance_spec(&card.id))));
        assert_eq!(found, to_json(Some(&dead)));
    }

    #[test]
    fn answers_null_for_an_id_nothing_holds_so_the_verb_fizzles_and_the_card_still_resolves() {
        let mut state = game();
        assert!(with_ctx(&mut state, None, |ctx| resolve_target(ctx, &instance_spec("c9999")).is_none()));
    }
}

// ---------------------------------------------------------------------------
// `cardsInScope`
// ---------------------------------------------------------------------------

mod cards_in_scope {
    use super::*;

    #[test]
    fn defaults_to_every_unit_on_both_sides_and_no_backrow_card() {
        let mut state = game();
        let mine = put(&mut state, &human().id, slot(PlayerId::P1, Row::Units, 2), PutOptions::default());
        let theirs = put(&mut state, &felinor().id, slot(PlayerId::P2, Row::Units, 1), PutOptions::default());
        let backrow = put(&mut state, &field_spell().id, slot(PlayerId::P1, Row::Backrow, 1), PutOptions::default());

        let found = with_ctx(&mut state, None, |ctx| ids_of(&cards_in_scope(ctx, &BoardScope::default())));
        assert!(found.contains(&mine.id));
        assert!(found.contains(&theirs.id));
        assert!(!found.contains(&backrow.id));
    }

    #[test]
    fn walks_r68_order_the_active_players_side_first_then_the_opponents_lane_1_upward() {
        let mut state = game();
        state.active = PlayerId::P2;
        let p1_lane1 = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());
        let p1_lane4 = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 4), PutOptions::default());
        let p2_lane2 = put(&mut state, &untagged().id, slot(PlayerId::P2, Row::Units, 2), PutOptions::default());
        let p2_lane5 = put(&mut state, &untagged().id, slot(PlayerId::P2, Row::Units, 5), PutOptions::default());

        // p2 is active, so its side comes first, and within each side the lanes ascend.
        assert_eq!(
            with_ctx(&mut state, None, |ctx| ids_of(&cards_in_scope(ctx, &BoardScope::default()))),
            vec![p2_lane2.id, p2_lane5.id, p1_lane1.id, p1_lane4.id]
        );
    }

    #[test]
    fn reads_side_relative_to_the_controller_not_to_the_active_player() {
        let mut state = game();
        state.active = PlayerId::P2;
        let mine = put(&mut state, &human().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());
        let theirs = put(&mut state, &human().id, slot(PlayerId::P2, Row::Units, 1), PutOptions::default());

        // controller is p1
        let (own, enemy) = with_ctx(&mut state, None, |ctx| {
            (
                ids_of(&cards_in_scope(ctx, &scope(json!({ "side": "self" })))),
                ids_of(&cards_in_scope(ctx, &scope(json!({ "side": "enemy" })))),
            )
        });
        assert_eq!(own, vec![mine.id]);
        assert_eq!(enemy, vec![theirs.id]);
    }

    #[test]
    fn covers_the_named_rows_so_a_permanents_scope_reaches_the_backrow_s6_3() {
        let mut state = game();
        let unit = put(&mut state, &human().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());
        let backrow = put(&mut state, &field_spell().id, slot(PlayerId::P1, Row::Backrow, 2), PutOptions::default());

        let found = with_ctx(&mut state, None, |ctx| {
            ids_of(&cards_in_scope(ctx, &scope(json!({ "side": "self", "rows": ["units", "backrow"] }))))
        });
        // Rows are walked in the order given: every unit lane, then every backrow lane.
        assert_eq!(found, vec![unit.id, backrow.id]);
    }

    #[test]
    fn keeps_tags_and_rejects_not_tags_by_the_definition() {
        let mut state = game();
        let the_human = put(&mut state, &human().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());
        put(&mut state, &felinor().id, slot(PlayerId::P1, Row::Units, 2), PutOptions::default());
        let the_untagged = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 3), PutOptions::default());

        let (tagged, not_felinor) = with_ctx(&mut state, None, |ctx| {
            (
                ids_of(&cards_in_scope(ctx, &scope(json!({ "side": "self", "tags": ["Human"] })))),
                ids_of(&cards_in_scope(ctx, &scope(json!({ "side": "self", "notTags": ["Felinor"] })))),
            )
        });
        assert_eq!(tagged, vec![the_human.id.clone()]);
        // "non-Felinor" keeps everything that is not tagged Felinor, the untagged card included (#43).
        assert_eq!(not_felinor, vec![the_human.id, the_untagged.id]);
    }

    #[test]
    fn filters_by_card_type() {
        let mut state = game();
        let unit = put(&mut state, &human().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());
        put(&mut state, &field_spell().id, slot(PlayerId::P1, Row::Backrow, 1), PutOptions::default());

        let found = with_ctx(&mut state, None, |ctx| {
            ids_of(&cards_in_scope(
                ctx,
                &scope(json!({ "side": "self", "rows": ["units", "backrow"], "types": ["Unit"] })),
            ))
        });
        assert_eq!(found, vec![unit.id]);
    }

    #[test]
    fn exclude_self_leaves_the_card_running_the_script_standing_100_all_other_permanents() {
        let mut state = game();
        let self_ = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());
        let other = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 2), PutOptions::default());

        let found = with_ctx(&mut state, Some(&self_), |ctx| {
            ids_of(&cards_in_scope(ctx, &scope(json!({ "side": "self", "excludeSelf": true }))))
        });
        assert_eq!(found, vec![other.id]);
        assert!(!found.contains(&self_.id));
    }

    #[test]
    fn matches_only_the_top_of_a_stack_pile_never_the_dormant_card_underneath_s3_2_r13() {
        let mut state = game();
        let ref_ = slot(PlayerId::P1, Row::Units, 1);
        let bottom = put(&mut state, &untagged().id, ref_, PutOptions::default());
        let top = state.players.p1.units[0].as_ref().and_then(|pile| pile.first()).cloned();
        assert_eq!(top, Some(live(&state, &bottom.id)));

        // A Stack card lands on the occupied zone and becomes the only active card there.
        let mut stacked = put(&mut state, &stackable().id, slot(PlayerId::P1, Row::Units, 2), PutOptions::default());
        state.players.p1.units[1] = None;
        stacked.zone = Zone::Field {
            player: PlayerId::P1,
            row: Row::Units,
            lane: 1,
        };
        assert!(place_on_field(&mut state, &mut stacked, ref_, PlaceOnFieldOptions { stack: Some(true) }));

        let found = with_ctx(&mut state, None, |ctx| ids_of(&cards_in_scope(ctx, &scope(json!({ "side": "self" })))));
        assert_eq!(found, vec![stacked.id.clone()]);
        assert!(!found.contains(&bottom.id));
    }

    #[test]
    fn returns_nothing_for_an_empty_board_rather_than_throwing() {
        let mut state = game();
        assert!(with_ctx(&mut state, None, |ctx| ids_of(&cards_in_scope(ctx, &BoardScope::default()))).is_empty());
    }
}

// ---------------------------------------------------------------------------
// `adjacentTo`
// ---------------------------------------------------------------------------

mod adjacent_to {
    use super::*;

    #[test]
    fn takes_lanes_n_1_and_n_1_on_the_targets_own_side_and_row_never_lane_n_s3_1() {
        let mut state = game();
        let left = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 2), PutOptions::default());
        let middle = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 3), PutOptions::default());
        let right = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 4), PutOptions::default());
        let far = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 5), PutOptions::default());

        let found = with_ctx(&mut state, None, |ctx| {
            ids_of(&adjacent_to(ctx, &instance_spec(&middle.id), &BoardScope::default()))
        });
        assert_eq!(found, vec![left.id, right.id]);
        assert!(!found.contains(&middle.id));
        assert!(!found.contains(&far.id));
    }

    #[test]
    fn never_crosses_to_the_other_side_even_in_the_facing_lane_s3_1() {
        let mut state = game();
        let middle = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 3), PutOptions::default());
        let facing = put(&mut state, &untagged().id, slot(PlayerId::P2, Row::Units, 3), PutOptions::default());
        let facing_neighbour = put(&mut state, &untagged().id, slot(PlayerId::P2, Row::Units, 2), PutOptions::default());

        let found = with_ctx(&mut state, None, |ctx| {
            ids_of(&adjacent_to(ctx, &instance_spec(&middle.id), &BoardScope::default()))
        });
        assert!(!found.contains(&facing.id));
        assert!(!found.contains(&facing_neighbour.id));
    }

    #[test]
    fn never_crosses_rows_so_adjacent_in_its_row_needs_no_extra_filter_34() {
        let mut state = game();
        let middle = put(&mut state, &field_spell().id, slot(PlayerId::P1, Row::Backrow, 2), PutOptions::default());
        let backrow_neighbour = put(&mut state, &field_spell().id, slot(PlayerId::P1, Row::Backrow, 1), PutOptions::default());
        let unit_beside = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());

        let found = with_ctx(&mut state, None, |ctx| {
            ids_of(&adjacent_to(ctx, &instance_spec(&middle.id), &BoardScope::default()))
        });
        assert_eq!(found, vec![backrow_neighbour.id]);
        assert!(!found.contains(&unit_beside.id));
    }

    #[test]
    fn clamps_at_the_ends_of_a_row() {
        let mut state = game();
        let first = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());
        let second = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 2), PutOptions::default());

        assert_eq!(
            with_ctx(&mut state, None, |ctx| {
                ids_of(&adjacent_to(ctx, &instance_spec(&first.id), &BoardScope::default()))
            }),
            vec![second.id]
        );
    }

    #[test]
    fn skips_empty_neighbouring_zones_and_applies_the_scopes_filters() {
        let mut state = game();
        put(&mut state, &felinor().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());
        let middle = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 2), PutOptions::default());
        let the_human = put(&mut state, &human().id, slot(PlayerId::P1, Row::Units, 3), PutOptions::default());

        let found = with_ctx(&mut state, None, |ctx| {
            ids_of(&adjacent_to(ctx, &instance_spec(&middle.id), &scope(json!({ "notTags": ["Felinor"] }))))
        });
        assert_eq!(found, vec![the_human.id]);
    }

    #[test]
    fn gives_a_card_that_is_off_the_field_no_neighbours() {
        let mut state = game();
        let card = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 2), PutOptions::default());
        put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());
        let mut dead = live(&state, &card.id);
        state.players.p1.units[1] = None;
        dead.zone = Zone::Graveyard { player: PlayerId::P1 };
        state.players.p1.graveyard.push(dead);

        assert!(
            with_ctx(&mut state, None, |ctx| {
                ids_of(&adjacent_to(ctx, &instance_spec(&card.id), &BoardScope::default()))
            })
            .is_empty()
        );
    }

    #[test]
    fn gives_a_hero_spec_no_neighbours_rather_than_throwing() {
        let mut state = game();
        put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());
        let hero: TargetSpec = json_as(json!({ "of": "enemyHero" }));
        assert!(with_ctx(&mut state, None, |ctx| ids_of(&adjacent_to(ctx, &hero, &BoardScope::default()))).is_empty());
    }
}

// ---------------------------------------------------------------------------
// `matchesScope`
// ---------------------------------------------------------------------------

mod matches_scope {
    use super::*;

    #[test]
    fn passes_a_card_no_filter_rejects_and_is_independent_of_where_the_card_sits() {
        let mut state = game();
        let card = put(&mut state, &human().id, slot(PlayerId::P2, Row::Units, 5), PutOptions::default());
        let card = live(&state, &card.id);

        with_ctx(&mut state, None, |ctx| {
            assert!(matches_scope(ctx, &card, &BoardScope::default()));
            assert!(matches_scope(ctx, &card, &scope(json!({ "tags": ["Human"] }))));
            assert!(!matches_scope(ctx, &card, &scope(json!({ "notTags": ["Human"] }))));
            assert!(!matches_scope(ctx, &card, &scope(json!({ "types": ["Spell"] }))));
        });
    }

    #[test]
    fn rejects_the_running_card_only_when_exclude_self_says_so() {
        let mut state = game();
        let self_ = put(&mut state, &untagged().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());
        let self_ = live(&state, &self_.id);

        with_ctx(&mut state, Some(&self_), |ctx| {
            assert!(matches_scope(ctx, &self_, &BoardScope::default()));
            assert!(!matches_scope(ctx, &self_, &scope(json!({ "excludeSelf": true }))));
        });
    }

    #[test]
    fn keeps_a_card_matching_any_one_of_several_tags() {
        let mut state = game();
        let the_felinor = put(&mut state, &felinor().id, slot(PlayerId::P1, Row::Units, 1), PutOptions::default());
        let the_felinor = live(&state, &the_felinor.id);
        assert!(with_ctx(&mut state, None, |ctx| {
            matches_scope(ctx, &the_felinor, &scope(json!({ "tags": ["Human", "Felinor"] })))
        }));
    }
}
