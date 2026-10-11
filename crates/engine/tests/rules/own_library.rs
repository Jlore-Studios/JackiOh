//! SPEC §10.8, R310–R312: a player's own library as `viewFor` gives it to them — which cards are
//! left, never in what order, and only what they were shown going in (`ownLibrary.ts`).
//!
//! The engine owns the rule, so these tests drive it with the fixture catalog and the effect verbs
//! the real cards use; the cards that bring it about (#33, #42, #83, #87, #90) prove it again in
//! packages/cards.
//!
//! Port of `packages/engine/test/ownLibrary.test.ts`. TS's module `const` definitions are functions
//! here (`cheap()`), each building the same `CardDef` from the same literal.

use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use jackioh_engine::effects::{radiant_chance, set_radiant, shuffle_into, swap_library, transform};

use crate::rules::fixtures::catalog::{spell_def, unit_def, vanilla_deck};
use crate::rules::fixtures::harness::{new_game, set_library};

// Three cards that sort by cost, then name, then id, whatever order they lie in (R310).
fn cheap() -> CardDef {
    unit_def(
        801,
        json!({ "id": "lib-cheap", "name": "Zed the Cheap", "cost": 0 }),
    )
}
fn alpha() -> CardDef {
    unit_def(802, json!({ "id": "lib-alpha", "name": "Alpha", "cost": 2 }))
}
fn beta() -> CardDef {
    unit_def(803, json!({ "id": "lib-beta", "name": "Beta", "cost": 2 }))
}
/// Same name and cost as `beta`: the id breaks the tie.
fn beta_twin() -> CardDef {
    unit_def(804, json!({ "id": "lib-beta-2", "name": "Beta", "cost": 2 }))
}
/// An X card sorts at 0 and an embiggen card at its base price (R65).
fn x_spell() -> CardDef {
    spell_def(805, json!({ "id": "lib-x", "name": "X Marks", "cost": "X" }))
}
fn big_spell() -> CardDef {
    spell_def(
        806,
        json!({ "id": "lib-embiggen", "name": "Embiggen", "cost": { "base": 3, "embiggen": 5 } }),
    )
}
/// What the opponent's CN-Viral Injection names (#90): a card shuffled in by a play both saw.
fn virus() -> CardDef {
    spell_def(
        807,
        json!({ "id": "lib-virus", "name": "Virus (fixture)", "cost": 1, "token": true, "tags": ["Token"] }),
    )
}
fn legendary() -> CardDef {
    unit_def(
        808,
        json!({ "id": "lib-legend", "name": "Legend (fixture)", "cost": 4, "rarity": "Legendary" }),
    )
}

fn defs() -> Vec<CardDef> {
    vec![
        cheap(),
        alpha(),
        beta(),
        beta_twin(),
        x_spell(),
        big_spell(),
        virus(),
        legendary(),
    ]
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    state.turn = 3;
    state.active = P1;
    state.phase = Phase::Main;
    state
}

/// Apply one effect the way `resolve.ts` does, for `controller`, and hand back its events.
fn run(state: &mut GameState, effect: Effect, controller: PlayerId) -> Vec<GameEvent> {
    let mut events: Vec<GameEvent> = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(&mut *state, &mut events, &mut rng);
        let mut ctx = make_context(
            &mut sink,
            None,
            HookOptions {
                controller: Some(controller),
                ..HookOptions::default()
            },
        );
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

fn list_of(state: &GameState, viewer: PlayerId) -> LibraryView {
    view_for(state, viewer)
        .you
        .own_library
        .unwrap_or_else(|| panic!("{viewer}'s view carries no library list"))
}

fn total(list: &LibraryView) -> i32 {
    list.cards.iter().map(|entry| entry.count).sum::<i32>() + list.unknown
}

/// Every instance id in a library, which must never reach a view (§9.1, R97).
fn ids_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

fn entry(def_id: &str, radiant: bool, count: i32) -> LibraryEntryView {
    LibraryEntryView {
        def_id: def_id.to_string(),
        radiant,
        count,
        created: None,
    }
}

fn to_json(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).expect("serialises")
}

/// R310 the viewer's own library, without its order
mod r310_the_viewer_s_own_library_without_its_order {
    use super::*;

    #[test]
    fn r310_lists_the_viewer_s_own_library_by_definition_face_and_count_and_leaves_the_opponent_s_a_count() {
        let mut state = game("own-library");
        set_library(
            &mut state,
            P1,
            &[
                beta().id,
                cheap().id,
                alpha().id,
                beta().id,
                x_spell().id,
                big_spell().id,
                beta_twin().id,
            ],
        );
        let theirs = set_library(&mut state, P2, &[legendary().id, alpha().id]);
        let view = view_for(&state, P1);

        assert_eq!(
            view.you.own_library,
            Some(LibraryView {
                cards: vec![
                    // Printed cost first (R65: X is 0, embiggen its base price), then name, then id: the X
                    // card costs 0 like "Zed the Cheap" and goes first by name.
                    entry(&x_spell().id, false, 1),
                    entry(&cheap().id, false, 1),
                    entry(&alpha().id, false, 1),
                    entry(&beta().id, false, 2),
                    entry(&beta_twin().id, false, 1),
                    entry(&big_spell().id, false, 1),
                ],
                unknown: 0,
            })
        );
        assert_eq!(total(&list_of(&state, P1)), view.you.library_count);

        // The opponent's library is a count and nothing else (§9.1, §10.8).
        assert!(view.opponent.own_library.is_none());
        let opponent = serde_json::to_value(&view.opponent).expect("serialises");
        assert!(
            !opponent
                .as_object()
                .expect("an object")
                .contains_key("ownLibrary")
        );
        let serialized = to_json(&view);
        assert!(!serialized.contains(&format!("\"{}\"", legendary().id)));
        for id in ids_of(&theirs) {
            assert!(!serialized.contains(&format!("\"{id}\"")));
        }
    }

    #[test]
    fn r310_names_no_library_card_by_instance_and_gives_two_libraries_of_the_same_cards_in_different_orders_the_same_view()
     {
        let deck = [
            beta().id,
            cheap().id,
            alpha().id,
            beta().id,
            x_spell().id,
            big_spell().id,
            beta_twin().id,
            alpha().id,
        ];
        let mut a = game("own-library-order");
        set_library(&mut a, P1, &deck);
        // One card of each pair Radiant, so the face is part of the key and not only the id.
        {
            let first_beta = a
                .players
                .p1
                .library
                .iter_mut()
                .find(|card| card.def_id == beta().id)
                .expect("no beta");
            first_beta.radiant = true;
            first_beta.known_as = Some(KnownAs {
                def_id: beta().id,
                radiant: true,
            });
        }

        // The same cards, the same instances, in every other order the test tries.
        fn reversed(cards: &[CardInstance]) -> Vec<CardInstance> {
            let mut reversed = cards.to_vec();
            reversed.reverse();
            reversed
        }
        fn rotated(cards: &[CardInstance]) -> Vec<CardInstance> {
            let mut rotated = cards[3..].to_vec();
            rotated.extend_from_slice(&cards[..3]);
            rotated
        }
        // TS `(x, y) => (x.id < y.id ? 1 : -1)`: by id, descending.
        fn by_id_descending(cards: &[CardInstance]) -> Vec<CardInstance> {
            let mut sorted = cards.to_vec();
            sorted.sort_by(|x, y| y.id.cmp(&x.id));
            sorted
        }
        type Order = fn(&[CardInstance]) -> Vec<CardInstance>;
        let orders: [Order; 3] = [reversed, rotated, by_id_descending];
        for reorder in orders {
            let mut b = clone_state(&a);
            b.players.p1.library = reorder(&b.players.p1.library);
            assert_ne!(ids_of(&b.players.p1.library), ids_of(&a.players.p1.library));
            for viewer in [P1, P2] {
                assert_eq!(view_for(&b, viewer), view_for(&a, viewer));
            }
        }

        let view = view_for(&a, P1);
        let cards: Vec<LibraryEntryView> = view
            .you
            .own_library
            .as_ref()
            .map(|list| list.cards.clone())
            .unwrap_or_default();
        assert!(cards.contains(&entry(&beta().id, false, 1)));
        assert!(cards.contains(&entry(&beta().id, true, 1)));
        // Base before Radiant within one definition.
        let betas: Vec<bool> = cards
            .iter()
            .filter(|entry| entry.def_id == beta().id)
            .map(|entry| entry.radiant)
            .collect();
        assert_eq!(betas, vec![false, true]);
        // No instance id of the viewer's own library reaches the viewer (§9.1): a list, not the pile.
        let serialized = to_json(&view);
        for id in ids_of(&a.players.p1.library) {
            assert!(!serialized.contains(&format!("\"{id}\"")));
        }
        // An entry is three fields and no more: nothing a position, a live cost or a roll could ride on.
        for entry in &cards {
            let json = serde_json::to_value(entry).expect("serialises");
            let mut keys: Vec<String> = json.as_object().expect("an object").keys().cloned().collect();
            keys.sort();
            assert_eq!(keys, vec!["count", "defId", "radiant"]);
        }
    }
}

/// R311 what the owner was shown going in
mod r311_what_the_owner_was_shown_going_in {
    use super::*;

    #[test]
    fn r311_knows_the_starting_deck_and_a_mulligan_s_returns_through_setup() {
        let decks: (Vec<String>, Vec<String>) = (vanilla_deck(DECK_SIZE, 1), vanilla_deck(DECK_SIZE, 21));
        let mut state = begin_game(&new_game("own-library-setup", Some(decks.clone()))).state;
        // Before either mulligan: the deck minus the opening hand, every card known.
        for viewer in [P1, P2] {
            let list = list_of(&state, viewer);
            let hand: Vec<String> = state.players[viewer]
                .hand
                .iter()
                .map(|card| card.def_id.clone())
                .collect();
            let deck = if viewer == P1 { &decks.0 } else { &decks.1 };
            assert_eq!(list.unknown, 0);
            let mut listed: Vec<String> = list.cards.iter().map(|entry| entry.def_id.clone()).collect();
            listed.sort();
            let mut expected: Vec<String> = deck.iter().filter(|id| !hand.contains(id)).cloned().collect();
            expected.sort();
            assert_eq!(listed, expected);
        }

        // Both seats send their whole opening hand back: the returns are known too.
        for player in [P1, P2] {
            let prompt = mulligan_prompt_for(&state, player);
            assert!(prompt.is_some());
            let result = reduce(
                &state,
                &Action::new(
                    ActionBody::Mulligan { keep: Vec::new() },
                    player,
                    format!("own-{player}"),
                ),
            );
            assert_eq!(result.error, None);
            state = result.state;
        }
        for viewer in [P1, P2] {
            let list = list_of(&state, viewer);
            assert_eq!(list.unknown, 0);
            assert_eq!(total(&list), state.players[viewer].library.len() as i32);
            let mut in_library: Vec<String> = state.players[viewer]
                .library
                .iter()
                .map(|card| card.def_id.clone())
                .collect();
            in_library.sort();
            let mut listed: Vec<String> = list
                .cards
                .iter()
                .flat_map(|entry| std::iter::repeat_n(entry.def_id.clone(), entry.count.max(0) as usize))
                .collect();
            listed.sort();
            assert_eq!(listed, in_library);
        }
    }

    #[test]
    fn r311_lists_a_card_the_opponent_shuffled_in_by_an_open_play_with_the_face_it_went_in_with() {
        let mut state = game("own-library-shuffle");
        set_library(&mut state, P1, &[alpha().id, beta().id]);
        // p2 resolves #90's effect: a Radiant virus into p1's library (the Radiant face of the Injection).
        let events = run(
            &mut state,
            shuffle_into(json_as(
                json!({ "defId": virus().id, "count": 1, "player": "enemy", "radiant": true }),
            )),
            P2,
        );
        assert_eq!(
            events
                .iter()
                .map(|event| event.event_type().as_str())
                .collect::<Vec<_>>(),
            vec!["shuffledIn"]
        );

        assert_eq!(
            list_of(&state, P1),
            LibraryView {
                cards: vec![
                    entry(&virus().id, true, 1),
                    entry(&alpha().id, false, 1),
                    entry(&beta().id, false, 1)
                ],
                unknown: 0,
            }
        );
        // The shuffle's own event still names nothing, to either seat, and keeps the slot hidden (R97).
        for viewer in [P1, P2] {
            let events = view_for(&state, viewer).events;
            let shuffled: Vec<&GameEvent> = events
                .iter()
                .filter(|event| event.event_type() == GameEventType::ShuffledIn)
                .collect();
            assert!(
                shuffled
                    .iter()
                    .all(|event| matches!(event, GameEvent::ShuffledIn { def_id, .. } if def_id == "hidden"))
            );
        }
        // And p2, who played it, still reads p1's library as a count.
        assert!(view_for(&state, P2).opponent.own_library.is_none());
    }

    #[test]
    fn r311_keeps_the_face_a_card_went_in_with_when_it_turns_radiant_inside_the_library_where_nobody_sees_it()
    {
        let mut state = game("own-library-radiant");
        let library = set_library(&mut state, P1, &[alpha().id, beta().id]);
        let (first, _second) = match library.as_slice() {
            [first, second] => (first.clone(), second.clone()),
            _ => panic!("library"),
        };
        let before = list_of(&state, P1);

        // #28's pick and #42's roll, as the verbs the cards use.
        run(
            &mut state,
            set_radiant(json_as(json!({ "instanceId": first.id }))),
            P1,
        );
        run(
            &mut state,
            radiant_chance(json_as(json!({ "zone": "library", "chance": 1 }))),
            P1,
        );
        assert!(state.players.p1.library.iter().all(|card| card.radiant));

        // The list is what p1 was shown, so it has not moved.
        assert_eq!(list_of(&state, P1), before);
        assert!(before.cards.iter().all(|entry| !entry.radiant));
    }
}

/// R312 cards the owner was never shown
mod r312_cards_the_owner_was_never_shown {
    use super::*;

    #[test]
    fn r312_counts_a_swapped_library_as_unknown_to_its_new_owner_on_both_sides() {
        let mut state = game("own-library-swap");
        set_library(&mut state, P1, &[alpha().id, beta().id, cheap().id]);
        set_library(&mut state, P2, &[legendary().id, virus().id]);
        run(&mut state, swap_library(), P1);

        assert_eq!(
            list_of(&state, P1),
            LibraryView {
                cards: vec![],
                unknown: 2
            }
        );
        assert_eq!(
            list_of(&state, P2),
            LibraryView {
                cards: vec![],
                unknown: 3
            }
        );
        // p1 learns nothing of what p2 held, and p2 nothing of p1's old library.
        let mine = to_json(&view_for(&state, P1));
        assert!(!mine.contains(&format!("\"{}\"", legendary().id)));
        assert!(!mine.contains(&format!("\"{}\"", virus().id)));
        let theirs = to_json(&view_for(&state, P2));
        for id in [alpha().id, beta().id, cheap().id] {
            assert!(!theirs.contains(&format!("\"{id}\"")));
        }

        // A card that goes in openly afterwards is known again; the swapped ones stay unknown.
        run(
            &mut state,
            shuffle_into(json_as(json!({ "defId": alpha().id, "count": 1 }))),
            P1,
        );
        assert_eq!(
            list_of(&state, P1),
            LibraryView {
                cards: vec![entry(&alpha().id, false, 1)],
                unknown: 2
            }
        );
        // Swapped back, a library is not known again: what its first owner was shown went with it.
        run(&mut state, swap_library(), P1);
        assert_eq!(
            list_of(&state, P1),
            LibraryView {
                cards: vec![],
                unknown: 3
            }
        );
    }

    #[test]
    fn r312_counts_a_card_a_replace_put_in_the_library_as_unknown_c83() {
        let mut state = game("own-library-replace");
        let first = set_library(&mut state, P1, &[alpha().id, beta().id])
            .into_iter()
            .next()
            .expect("library");
        run(
            &mut state,
            transform(json_as(
                json!({ "instanceId": first.id, "defId": legendary().id, "radiant": true }),
            )),
            P1,
        );

        assert!(
            state
                .players
                .p1
                .library
                .iter()
                .map(|card| card.def_id.clone())
                .any(|def_id| def_id == legendary().id)
        );
        assert_eq!(
            list_of(&state, P1),
            LibraryView {
                cards: vec![entry(&beta().id, false, 1)],
                unknown: 1
            }
        );
        assert!(!to_json(&view_for(&state, P1)).contains(&format!("\"{}\"", legendary().id)));
    }

    #[test]
    fn r312_treats_a_library_card_with_no_record_as_unknown_so_a_path_that_forgets_to_record_shows_a_back() {
        let mut state = game("own-library-unrecorded");
        set_library(&mut state, P1, &[alpha().id, beta().id]);
        for card in state.players.p1.library.iter_mut() {
            card.known_as = None;
        }
        assert_eq!(
            own_library_view(&state, P1),
            LibraryView {
                cards: vec![],
                unknown: 2
            }
        );
        assert!(!to_json(&view_for(&state, P1)).contains(&format!("\"{}\"", alpha().id)));
    }
}
