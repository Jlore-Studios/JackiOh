//! C+ #54 Book of Books (SPEC §8.7 row 54). (1) Spell, Book, Epic.
//!   Base:    "Add {books|random Book|random Books} to your hand. Each costs (0) and is Temporary."
//!            — books 2
//!   Radiant: "Add {books|random Radiant Book|random Radiant Books} to your hand. Each costs (0) and
//!            is Temporary."
//!   Engine:  "Non-token Books of every set (R380) but this one (R387), repeats allowed (R60);
//!            `costOverride` 0 and Temporary (R637), so an unplayed Book is discarded at the end of the
//!            turn; the hand cap burns extras (§2.4). Tunes: books 2 ↑."
//!
//! One tag pool; `addRandomFromCatalog` leaves out the card running the script by its def id (R387) and
//! prices only a card that reached the hand (§2.4, R4).

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-054";

fn book_of_books(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "tags": ["Book"] },
                "count": param(&*ctx, "books"),
                // "Each costs (0)": the declared number `setCost` (R386).
                "costOverride": param(&*ctx, "setCost"),
                "radiant": radiant,
                "temporary": true,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: book_of_books(false),
        radiant: book_of_books(true),
    }
}

// C+ #54 Book of Books — SPEC §8.7 row 54, BUILD M9 Classic+ row C+ 54: "Adds 2 random non-token Books
// of any set, never Book of Books (R387), which cost (0) (`costOverride` 0); repeats allowed; a full
// hand burns; hidden from the opponent (R97); the count reads through `param()`; radiant the Books are
// Radiant". R637: each Book is Temporary, in both faces, and is discarded at the end of the turn.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const BOOK: &str = "classicplus-054";
    const FILLER: &str = "core-005";

    use crate::scenario;

    /// TS `book({ radiant?, seed?, fillers? })`: `None` is the TS default.
    fn book(radiant: bool, seed: Option<&str>, fillers: Option<usize>) -> Scenario {
        let mut hand = vec![json!({ "def": BOOK, "radiant": radiant })];
        hand.extend((0..fillers.unwrap_or(1)).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": seed.unwrap_or("book-of-books"),
            "p1": { "hand": hand },
            "p2": { "hand": [FILLER] },
        }))
    }

    /// The `addedToHand` events of p1 in the last step, as `(instanceId, defId)`.
    fn added(s: &Scenario) -> Vec<(String, String)> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: PlayerId::P1, instance_id, def_id } => {
                    Some((instance_id.clone(), def_id.clone()))
                }
                _ => None,
            })
            .collect()
    }

    fn added_cards(s: &Scenario) -> Vec<CardInstance> {
        added(s).iter().map(|(id, _)| s.card(id).clone()).collect()
    }

    /// The view of p1's own hand (a list for its owner).
    fn own_hand(s: &Scenario) -> Vec<CardView> {
        match s.view(PlayerId::P1).you.hand {
            HandView::Cards(cards) => cards,
            HandView::Count { .. } => Vec::new(),
        }
    }

    fn is_hidden(event: &GameEvent) -> bool {
        matches!(event, GameEvent::AddedToHand { instance_id, def_id, .. } if instance_id == "hidden" && def_id == "hidden")
    }

    #[test]
    fn is_a_1_spell_book() {
        crate::register_all();
        let def = crate::card_def(super::ID);
        assert_eq!(def.id, BOOK);
        assert_eq!(def.tags, vec![Tag::Book]);
    }

    mod base {
        use super::*;

        #[test]
        fn adds_2_random_books_to_your_hand_each_costing_0() {
            let mut s = book(false, None, None);
            s.play(BOOK, json!({}));
            let books = added_cards(&s);
            assert_eq!(books.len(), 2);
            for card in &books {
                assert_eq!(card.zone.z(), ZoneName::Hand);
                assert!(def_of(Some(s.state()), &card.def_id).tags.contains(&Tag::Book));
                assert_eq!(card.cost_override, Some(0));
                assert!(!card.radiant);
            }
            let mine = own_hand(&s);
            assert!(books.iter().all(|card| mine.iter().any(|view| view.instance_id == card.id && view.cost == 0)));
        }

        #[test]
        fn r380_r387_over_many_seeds_books_of_several_sets_never_a_token_never_book_of_books() {
            let mut seen: IndexSet<String> = IndexSet::new();
            for i in 0..80 {
                let mut s = book(false, Some(&format!("books-{i}")), None);
                s.play(BOOK, json!({}));
                for (_, def_id) in added(&s) {
                    seen.insert(def_id);
                }
            }
            assert!(!seen.contains(BOOK));
            let pool: Vec<String> = query(&json_as(json!({ "tags": ["Book"] })))
                .iter()
                .map(|entry| entry.id.clone())
                .collect();
            for id in &seen {
                assert!(pool.contains(id), "{id} in the Book pool");
            }
            let fresh = book(false, None, None);
            let sets: IndexSet<SetName> = seen.iter().map(|id| def_of(Some(fresh.state()), id).set).collect();
            assert!(sets.contains(&SetName::Classic));
            assert!(sets.contains(&SetName::ClassicPlus));
            assert!(seen.iter().all(|id| !def_of(Some(fresh.state()), id).token));
        }

        #[test]
        fn r60_repeats_are_allowed() {
            let mut repeated = false;
            let mut i = 0;
            while i < 80 && !repeated {
                let mut s = book(false, Some(&format!("repeat-{i}")), None);
                i += 1;
                s.play(BOOK, json!({}));
                let found = added(&s);
                repeated = match (found.first(), found.get(1)) {
                    (Some(a), Some(b)) => a.1 == b.1,
                    _ => false,
                };
            }
            assert!(repeated);
        }

        #[test]
        fn s2_4_r4_a_full_hand_burns_the_books_which_keep_no_0_price() {
            let mut s = book(false, None, Some(9));
            s.play(BOOK, json!({}));
            let burned: Vec<String> = s
                .last_events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::Burned { instance_id, .. } => Some(instance_id.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(burned.len(), 1);
            let lost = s.card(&burned[0]).clone();
            assert_eq!(lost.zone.z(), ZoneName::Graveyard);
            assert_eq!(lost.cost_override, None);
            // R637: Temporary is for the card's stay in a hand, so a burned Book carries none.
            assert_eq!(lost.granted_keywords, Vec::<Keyword>::new());
            assert_eq!(added(&s).len(), 1);
        }

        #[test]
        fn r637_each_book_is_temporary_discarded_from_your_hand_at_the_end_of_your_turn_the_filler_kept() {
            let mut s = book(false, None, None);
            s.play(BOOK, json!({}));
            let books = added_cards(&s);
            assert_eq!(books.len(), 2);
            for card in &books {
                assert_eq!(card.granted_keywords, vec![Keyword::Temporary]);
            }
            let view = own_hand(&s);
            assert!(books.iter().all(|card| view.iter().any(|entry| {
                entry.instance_id == card.id
                    && entry.keywords.as_ref().is_some_and(|keywords| keywords.contains(&Keyword::Temporary))
            })));

            s.end_turn();
            for card in &books {
                assert_eq!(s.card(&card.id).zone.z(), ZoneName::Graveyard);
            }
            let hand: Vec<String> = s.hand(PlayerId::P1).into_iter().map(|card| card.def_id).collect();
            assert_eq!(hand, vec![FILLER.to_string()]);
        }

        #[test]
        fn r97_the_opponent_sees_the_adds_under_the_sentinel() {
            let mut s = book(false, None, None);
            s.play(BOOK, json!({}));
            let ids: Vec<String> = added(&s).into_iter().map(|(id, _)| id).collect();
            let theirs = s.view(PlayerId::P2);
            let events: Vec<&GameEvent> = theirs
                .events
                .iter()
                .filter(|event| matches!(event, GameEvent::AddedToHand { player: PlayerId::P1, .. }))
                .collect();
            assert_eq!(events.len(), 2);
            for event in events {
                assert!(is_hidden(event));
            }
            let text = serde_json::to_string(&theirs).unwrap();
            for id in &ids {
                assert!(!text.contains(&format!("\"{id}\"")));
            }
        }

        #[test]
        fn r386_an_upgrade_adds_3_a_degrade_1_never_fewer() {
            let mut up = book(false, None, None);
            step_param(up.card_mut(BOOK), "books", 1);
            up.play(BOOK, json!({}));
            assert_eq!(added(&up).len(), 3);

            let mut down = book(false, None, None);
            step_param(down.card_mut(BOOK), "books", -3);
            down.play(BOOK, json!({}));
            assert_eq!(added(&down).len(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_the_books_are_radiant_and_cost_0_never_book_of_books() {
            for i in 0..30 {
                let mut s = book(true, Some(&format!("rbooks-{i}")), None);
                s.play(BOOK, json!({}));
                let books = added_cards(&s);
                assert_eq!(books.len(), 2);
                for card in &books {
                    assert!(card.radiant);
                    assert_eq!(card.cost_override, Some(0));
                    assert_ne!(card.def_id, BOOK);
                }
            }
        }

        #[test]
        fn r637_the_radiant_books_are_temporary_too() {
            let mut s = book(true, Some("rtemporary"), None);
            s.play(BOOK, json!({}));
            let books = added_cards(&s);
            assert_eq!(books.len(), 2);
            for card in &books {
                assert_eq!(card.granted_keywords, vec![Keyword::Temporary]);
            }

            s.end_turn();
            for card in &books {
                assert_eq!(s.card(&card.id).zone.z(), ZoneName::Graveyard);
            }
        }

        #[test]
        fn s9_3_the_same_seed_adds_the_same_books() {
            let mut a = book(true, Some("same"), None);
            let mut b = book(true, Some("same"), None);
            a.play(BOOK, json!({}));
            b.play(BOOK, json!({}));
            let defs = |s: &Scenario| -> Vec<String> { added(s).into_iter().map(|(_, def_id)| def_id).collect() };
            assert_eq!(defs(&a), defs(&b));
        }

        #[test]
        fn r386_a_degrade_makes_the_books_cost_1_and_an_upgrade_finds_the_cost_at_its_floor_of_0() {
            let mut s = book(false, None, None);
            assert!(!crate::can_upgrade_number(&s, BOOK, "setCost"));
            assert_eq!(crate::degrade_number(&mut s, BOOK, "setCost"), 1);
            s.play(BOOK, json!({}));
            let made = added_cards(&s);
            assert_eq!(made.len(), 2);
            assert!(made.iter().all(|card| card.cost_override == Some(1)));
        }
    }
}
