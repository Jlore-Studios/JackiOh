//! Card scopes (docs/classic-sets.md B3.3, B3.4, B5 E38, E39): the cards a verb reaches on the field,
//! in hands and in decks, in R242's order — public cards, then the owner's hidden ones, then the
//! decks' — with its filters, and every card of a hidden pile kept for a verb that must cue the whole
//! pile (R440); and who may read a card where it sits (R97, R177).

use jackioh_engine::effects::{
    CardScope, CardScopeOptions, buff_all_units, cards_in_card_scope, readers_of, switch_all_positions,
    unreadable_by,
};
use jackioh_engine::resolve::apply_effects;
use jackioh_engine::testkit::*;

use crate::rules::fixtures::combat::{plain, stacker};
use crate::rules::fixtures::harness::{in_hand, put, set_library, slot};
use crate::rules::fixtures::instance_data::{blood_moon, echo_bolt, instance_game, tesla};

fn game(active: PlayerId) -> GameState {
    let mut state = instance_game("card-scope", None);
    state.turn = 3;
    state.active = active;
    state
}

fn with_ctx<R>(
    state: &mut GameState,
    self_: Option<CardInstance>,
    options: HookOptions,
    f: impl FnOnce(&mut EffectContext<'_>) -> R,
) -> R {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    let mut sink = EngineSink::new(state, &mut events, &mut rng);
    let mut ctx = make_context(&mut sink, self_.as_ref(), options);
    f(&mut ctx)
}

fn as_player(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..Default::default()
    }
}

fn first_id(cards: Vec<CardInstance>) -> String {
    cards.into_iter().next().expect("a card").id
}

fn scope(value: Value) -> CardScope {
    json_as(value)
}

mod card_scopes_r242_r440 {
    use super::*;

    #[test]
    fn r242_walks_public_cards_first_then_hands_and_face_down_traps_then_decks_the_active_side_first_in_each_group()
     {
        let mut state = game(PlayerId::P2);
        let deck1 = first_id(set_library(
            &mut state,
            PlayerId::P1,
            std::slice::from_ref(&plain.id),
        ));
        let deck2 = first_id(set_library(
            &mut state,
            PlayerId::P2,
            std::slice::from_ref(&plain.id),
        ));
        let hand1 = first_id(in_hand(&mut state, &plain.id, PlayerId::P1, 1));
        let hand2 = first_id(in_hand(&mut state, &plain.id, PlayerId::P2, 1));
        let unit1 = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 3),
            json!({}),
        );
        let trap1 = put(
            &mut state,
            &blood_moon.id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let field2 = put(
            &mut state,
            &tesla.id,
            slot(PlayerId::P2, Row::Backrow, 2),
            json!({}),
        );
        find_instance_mut(&mut state, &field2.id)
            .expect("the field trap")
            .face_up = Some(true);
        let (ids, readers) = with_ctx(&mut state, None, as_player(PlayerId::P1), |ctx| {
            let walk = cards_in_card_scope(
                ctx,
                &scope(json!({ "side": "any", "zones": ["field", "hand", "library"] })),
                Default::default(),
            );
            let ids: Vec<String> = walk.iter().map(|entry| entry.card.id.clone()).collect();
            let readers: Vec<Value> = walk
                .iter()
                .map(|entry| serde_json::to_value(entry.readers).unwrap())
                .collect();
            (ids, readers)
        });
        assert_eq!(
            ids,
            vec![
                field2.id.clone(),
                unit1.id.clone(),
                hand2,
                trap1.id.clone(),
                hand1,
                deck2,
                deck1
            ]
        );
        assert_eq!(
            Value::Array(readers),
            json!([
                "everyone", "everyone", "owner", "owner", "owner", "nobody", "nobody"
            ])
        );
    }

    #[test]
    fn r440_filters_keep_the_matching_cards_whole_hidden_piles_keeps_every_hidden_card_the_rest_marked() {
        let mut state = game(PlayerId::P1);
        let unit = first_id(in_hand(&mut state, &plain.id, PlayerId::P1, 1));
        let spell = first_id(in_hand(&mut state, &echo_bolt.id, PlayerId::P1, 1));
        let field_spell = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let trap = put(
            &mut state,
            &blood_moon.id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let the_scope = scope(json!({ "zones": ["field", "hand"], "types": ["Spell"] }));
        let (matching, whole) = with_ctx(&mut state, None, as_player(PlayerId::P1), |ctx| {
            let matching: Vec<String> = cards_in_card_scope(ctx, &the_scope, Default::default())
                .iter()
                .map(|entry| entry.card.id.clone())
                .collect();
            let whole: Vec<(String, bool)> = cards_in_card_scope(
                ctx,
                &the_scope,
                Some(&json_as(json!({ "wholeHiddenPiles": true }))),
            )
            .iter()
            .map(|entry| (entry.card.id.clone(), entry.matches))
            .collect();
            (matching, whole)
        });
        assert_eq!(matching, vec![spell.clone()]);
        // The public field unit is left out; the face-down trap and the hand unit are kept, unmatched.
        assert_eq!(
            whole,
            vec![(trap.id.clone(), false), (unit, false), (spell, true)]
        );
        assert!(!whole.iter().any(|(id, _)| *id == field_spell.id));
    }

    #[test]
    fn a_card_dormant_under_a_stack_is_not_on_the_field_for_a_scope_the_side_and_rows_narrow_it() {
        // §3.2.
        let mut state = game(PlayerId::P1);
        let under = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let mut top = new_instance(
            &mut state,
            &stacker.id,
            PlayerId::P1,
            Zone::Hand { player: PlayerId::P1 },
        );
        place_on_field(
            &mut state,
            &mut top,
            slot(PlayerId::P1, Row::Units, 1),
            json_as(json!({ "stack": true })),
        );
        let back = put(
            &mut state,
            &tesla.id,
            slot(PlayerId::P1, Row::Backrow, 2),
            json!({}),
        );
        let theirs = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P2, Row::Units, 1),
            json!({}),
        );
        let me = find_instance(&state, &top.id).cloned();
        with_ctx(&mut state, me, as_player(PlayerId::P1), |ctx| {
            let ids = |side: &str, rows: Option<Vec<&str>>, exclude_self: bool| -> Vec<String> {
                let mut asked = json!({ "side": side, "zones": ["field"] });
                if let Some(rows) = rows {
                    asked["rows"] = json!(rows);
                }
                if exclude_self {
                    asked["excludeSelf"] = json!(true);
                }
                cards_in_card_scope(ctx, &scope(asked), Default::default())
                    .iter()
                    .map(|entry| entry.card.id.clone())
                    .collect()
            };
            assert_eq!(ids("self", None, false), vec![top.id.clone(), back.id.clone()]);
            assert!(!ids("self", None, false).contains(&under.id));
            assert_eq!(ids("self", Some(vec!["units"]), false), vec![top.id.clone()]);
            assert_eq!(ids("self", None, true), vec![back.id.clone()]);
            assert_eq!(ids("enemy", None, false), vec![theirs.id.clone()]);
        });
    }

    #[test]
    fn r177_who_may_not_read_a_card_where_it_sits_both_for_a_deck_card_the_other_player_for_a_hand_card_or_a_face_down_trap()
     {
        let mut state = game(PlayerId::P1);
        let deck = set_library(&mut state, PlayerId::P1, std::slice::from_ref(&plain.id))
            .into_iter()
            .next();
        let held = in_hand(&mut state, &plain.id, PlayerId::P2, 1).into_iter().next();
        let trap = put(
            &mut state,
            &blood_moon.id,
            slot(PlayerId::P1, Row::Backrow, 1),
            json!({}),
        );
        let unit = put(
            &mut state,
            &plain.id,
            slot(PlayerId::P1, Row::Units, 1),
            json!({}),
        );
        let (Some(deck), Some(held)) = (deck, held) else {
            panic!("no card");
        };
        let card = |state: &GameState, id: &str| find_instance(state, id).expect("the card").clone();
        assert_eq!(
            unreadable_by(&state, &card(&state, &deck.id)),
            vec![PlayerId::P1, PlayerId::P2]
        );
        assert_eq!(unreadable_by(&state, &card(&state, &held.id)), vec![PlayerId::P1]);
        assert_eq!(unreadable_by(&state, &card(&state, &trap.id)), vec![PlayerId::P2]);
        assert_eq!(
            unreadable_by(&state, &card(&state, &unit.id)),
            Vec::<PlayerId>::new()
        );
        find_instance_mut(&mut state, &trap.id).expect("the trap").face_up = Some(true);
        assert_eq!(
            serde_json::to_value(readers_of(&state, &card(&state, &trap.id))).unwrap(),
            json!("everyone")
        );
    }
}

mod r4453000_a_spells_walks_pass_a_unit_immune_to_spells_by {
    use super::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    fn immune(state: &mut GameState, id: &str) {
        find_instance_mut(state, id)
            .expect("the card")
            .granted_keywords
            .push(Keyword::ImmuneToSpells);
    }

    /// A Spell resolving for P1, the source of every walk below that is a Spell's.
    fn spell(state: &mut GameState) -> CardInstance {
        new_instance(state, &echo_bolt.id, P1, Zone::Resolving { player: P1 })
    }

    fn walk(
        state: &mut GameState,
        self_: CardInstance,
        asked: Value,
        options: Option<CardScopeOptions>,
    ) -> Vec<(String, bool)> {
        with_ctx(state, Some(self_), as_player(P1), |ctx| {
            cards_in_card_scope(ctx, &scope(asked), options.as_ref())
                .iter()
                .map(|entry| (entry.card.id.clone(), entry.matches))
                .collect()
        })
    }

    #[test]
    fn r4453000_a_spells_card_scope_passes_an_immune_field_unit_by_and_reaches_its_hand_and_deck_copies() {
        let mut state = game(P1);
        let warded = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let open = put(&mut state, &plain.id, slot(P1, Row::Units, 2), json!({}));
        let held = first_id(in_hand(&mut state, &plain.id, P1, 1));
        let deck = first_id(set_library(&mut state, P1, std::slice::from_ref(&plain.id)));
        for id in [&warded.id, &held, &deck] {
            immune(&mut state, id);
        }
        let source = spell(&mut state);
        let ids: Vec<String> = walk(
            &mut state,
            source,
            json!({ "zones": ["field", "hand", "library"] }),
            None,
        )
        .into_iter()
        .map(|(id, _)| id)
        .collect();
        assert_eq!(
            ids,
            vec![open.id.clone(), held, deck],
            "the field's immune Unit is passed by"
        );
    }

    #[test]
    fn r4453000_a_units_card_scope_still_reaches_an_immune_field_unit() {
        let mut state = game(P1);
        let warded = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let open = put(&mut state, &plain.id, slot(P1, Row::Units, 2), json!({}));
        immune(&mut state, &warded.id);
        let me = find_instance(&state, &open.id).cloned().expect("the unit");
        let ids: Vec<String> = walk(&mut state, me, json!({ "zones": ["field"] }), None)
            .into_iter()
            .map(|(id, _)| id)
            .collect();
        assert_eq!(ids, vec![warded.id.clone(), open.id.clone()]);
    }

    #[test]
    fn r4453000_a_hidden_card_stays_in_a_whole_pile_walk() {
        // R440: the check marks a card unmatched rather than dropping it, so a face-down card the Spell
        // may not affect is still cued with its pile.
        let mut state = game(P1);
        let warded = put(&mut state, &plain.id, slot(P1, Row::Units, 1), json!({}));
        let trap = put(&mut state, &blood_moon.id, slot(P1, Row::Backrow, 1), json!({}));
        immune(&mut state, &warded.id);
        immune(&mut state, &trap.id);
        let source = spell(&mut state);
        let whole = walk(
            &mut state,
            source,
            json!({ "zones": ["field"] }),
            Some(CardScopeOptions {
                whole_hidden_piles: Some(true),
                ..Default::default()
            }),
        );
        assert_eq!(whole, vec![(trap.id.clone(), false)]);
    }

    fn buff_both(state: &mut GameState, self_: CardInstance) {
        with_ctx(state, Some(self_), as_player(P1), |ctx| {
            apply_effects(
                &[buff_all_units(json_as(
                    json!({ "side": "both", "attack": -2, "health": -2 }),
                ))],
                ctx,
            );
        });
    }

    fn buffs_of(state: &GameState, card: &CardInstance) -> (i32, i32) {
        let live = find_instance(state, &card.id).expect("the unit");
        (live.buffs.attack, live.buffs.health)
    }

    #[test]
    fn r4453000_buff_all_units_from_a_spell_passes_an_immune_unit_by() {
        let mut state = game(P1);
        let warded = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        let open = put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
        immune(&mut state, &warded.id);
        let source = spell(&mut state);
        buff_both(&mut state, source);
        assert_eq!(buffs_of(&state, &warded), (0, 0));
        assert_eq!(buffs_of(&state, &open), (-2, -2));
    }

    #[test]
    fn r4453000_buff_all_units_from_a_unit_or_a_field_trap_still_reaches_it() {
        let mut state = game(P1);
        let warded = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        immune(&mut state, &warded.id);
        let body = put(&mut state, &plain.id, slot(P1, Row::Units, 3), json!({}));
        buff_both(&mut state, body);
        assert_eq!(buffs_of(&state, &warded), (-2, -2), "a Unit's reaches it");
        // `is_spell_source` is a type test, so a Field Trap stands in for every source that is no Spell.
        let field = put(&mut state, &tesla.id, slot(P1, Row::Backrow, 2), json!({}));
        find_instance_mut(&mut state, &field.id)
            .expect("the field trap")
            .face_up = Some(true);
        let field = find_instance(&state, &field.id).cloned().expect("the field trap");
        buff_both(&mut state, field);
        assert_eq!(buffs_of(&state, &warded), (-4, -4), "a Field Trap's reaches it");
    }

    #[test]
    fn r4453000_switch_all_positions_from_a_spell_leaves_an_immune_unit_in_its_position() {
        let mut state = game(P1);
        let warded = put(&mut state, &plain.id, slot(P2, Row::Units, 1), json!({}));
        let open = put(&mut state, &plain.id, slot(P2, Row::Units, 2), json!({}));
        immune(&mut state, &warded.id);
        let position = |state: &GameState, card: &CardInstance| {
            find_instance(state, &card.id).expect("the unit").position
        };
        let (before, other) = (position(&state, &warded), position(&state, &open));
        let source = spell(&mut state);
        with_ctx(&mut state, Some(source), as_player(P1), |ctx| {
            apply_effects(&[switch_all_positions(json_as(json!({ "side": "both" })))], ctx);
        });
        assert_eq!(position(&state, &warded), before);
        assert_ne!(position(&state, &open), other);
    }
}
