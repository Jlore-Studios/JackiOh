//! Card scopes (docs/classic-sets.md B3.3, B3.4, B5 E38, E39): the cards a verb reaches on the field,
//! in hands and in decks, in R242's order — public cards, then the owner's hidden ones, then the
//! decks' — with its filters, and every card of a hidden pile kept for a verb that must cue the whole
//! pile (R440); and who may read a card where it sits (R97, R177).
//!
//! Port of `packages/engine/test/effects-cardScope.test.ts`.

use jackioh_engine::effects::{CardScope, cards_in_card_scope, readers_of, unreadable_by};
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

/// TS `sinkFor(state)`, then `makeContext(sink, self, options)`, handed to `f`.
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
    HookOptions { controller: Some(player), ..Default::default() }
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
    fn r242_walks_public_cards_first_then_hands_and_face_down_traps_then_decks_the_active_side_first_in_each_group() {
        let mut state = game(PlayerId::P2);
        let deck1 = first_id(set_library(&mut state, PlayerId::P1, &[plain.id.clone()]));
        let deck2 = first_id(set_library(&mut state, PlayerId::P2, &[plain.id.clone()]));
        let hand1 = first_id(in_hand(&mut state, &plain.id, PlayerId::P1, 1));
        let hand2 = first_id(in_hand(&mut state, &plain.id, PlayerId::P2, 1));
        let unit1 = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 3), json!({}));
        let trap1 = put(&mut state, &blood_moon.id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        let field2 = put(&mut state, &tesla.id, slot(PlayerId::P2, Row::Backrow, 2), json!({}));
        find_instance_mut(&mut state, &field2.id).expect("the field trap").face_up = Some(true);
        let (ids, readers) = with_ctx(&mut state, None, as_player(PlayerId::P1), |ctx| {
            let walk = cards_in_card_scope(
                ctx,
                &scope(json!({ "side": "any", "zones": ["field", "hand", "library"] })),
                Default::default(),
            );
            let ids: Vec<String> = walk.iter().map(|entry| entry.card.id.clone()).collect();
            let readers: Vec<Value> = walk.iter().map(|entry| serde_json::to_value(&entry.readers).unwrap()).collect();
            (ids, readers)
        });
        assert_eq!(ids, vec![field2.id.clone(), unit1.id.clone(), hand2, trap1.id.clone(), hand1, deck2, deck1]);
        assert_eq!(
            Value::Array(readers),
            json!(["everyone", "everyone", "owner", "owner", "owner", "nobody", "nobody"])
        );
    }

    #[test]
    fn r440_filters_keep_the_matching_cards_whole_hidden_piles_keeps_every_hidden_card_the_rest_marked() {
        let mut state = game(PlayerId::P1);
        let unit = first_id(in_hand(&mut state, &plain.id, PlayerId::P1, 1));
        let spell = first_id(in_hand(&mut state, &echo_bolt.id, PlayerId::P1, 1));
        let field_spell = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let trap = put(&mut state, &blood_moon.id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        let the_scope = scope(json!({ "zones": ["field", "hand"], "types": ["Spell"] }));
        let (matching, whole) = with_ctx(&mut state, None, as_player(PlayerId::P1), |ctx| {
            let matching: Vec<String> = cards_in_card_scope(ctx, &the_scope, Default::default())
                .iter()
                .map(|entry| entry.card.id.clone())
                .collect();
            let whole: Vec<(String, bool)> =
                cards_in_card_scope(ctx, &the_scope, Some(&json_as(json!({ "wholeHiddenPiles": true }))))
                    .iter()
                    .map(|entry| (entry.card.id.clone(), entry.matches))
                    .collect();
            (matching, whole)
        });
        assert_eq!(matching, vec![spell.clone()]);
        // The unit on the field is public and left out; the face-down trap and the hand unit are kept, unmatched.
        assert_eq!(whole, vec![(trap.id.clone(), false), (unit, false), (spell, true)]);
        assert!(!whole.iter().any(|(id, _)| *id == field_spell.id));
    }

    #[test]
    fn a_card_dormant_under_a_stack_is_not_on_the_field_for_a_scope_the_side_and_rows_narrow_it() {
        // §3.2.
        let mut state = game(PlayerId::P1);
        let under = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut top = new_instance(&mut state, &stacker.id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        place_on_field(&mut state, &mut top, slot(PlayerId::P1, Row::Units, 1), json_as(json!({ "stack": true })));
        let back = put(&mut state, &tesla.id, slot(PlayerId::P1, Row::Backrow, 2), json!({}));
        let theirs = put(&mut state, &plain.id, slot(PlayerId::P2, Row::Units, 1), json!({}));
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
    fn r177_who_may_not_read_a_card_where_it_sits_both_for_a_deck_card_the_other_player_for_a_hand_card_or_a_face_down_trap(
    ) {
        let mut state = game(PlayerId::P1);
        let deck = set_library(&mut state, PlayerId::P1, &[plain.id.clone()]).into_iter().next();
        let held = in_hand(&mut state, &plain.id, PlayerId::P2, 1).into_iter().next();
        let trap = put(&mut state, &blood_moon.id, slot(PlayerId::P1, Row::Backrow, 1), json!({}));
        let unit = put(&mut state, &plain.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let (Some(deck), Some(held)) = (deck, held) else {
            panic!("no card");
        };
        let card = |state: &GameState, id: &str| find_instance(state, id).expect("the card").clone();
        assert_eq!(unreadable_by(&state, &card(&state, &deck.id)), vec![PlayerId::P1, PlayerId::P2]);
        assert_eq!(unreadable_by(&state, &card(&state, &held.id)), vec![PlayerId::P1]);
        assert_eq!(unreadable_by(&state, &card(&state, &trap.id)), vec![PlayerId::P2]);
        assert_eq!(unreadable_by(&state, &card(&state, &unit.id)), Vec::<PlayerId>::new());
        find_instance_mut(&mut state, &trap.id).expect("the trap").face_up = Some(true);
        assert_eq!(serde_json::to_value(readers_of(&state, &card(&state, &trap.id))).unwrap(), json!("everyone"));
    }
}
