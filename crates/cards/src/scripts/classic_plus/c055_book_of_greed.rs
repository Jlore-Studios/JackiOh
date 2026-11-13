//! C+ #55 Book of Greed (SPEC §8.7 row 55). (1) Spell, Book, Epic.
//!   Base:    "Add {cards|random Legendary or Mythic card|…} to your hand." — cards 3
//!   Radiant: "Add {cards|random Radiant Legendary or Mythic card|…} to your hand."
//!   Engine:  "Non-token cards whose rarity is Legendary or Mythic, every set (R380); a token's rarity is
//!            "Token" and its `printedRarity` never feeds a pool (§5), so none is found; repeats allowed
//!            (R60); the hand cap burns extras. Tunes: cards 3 ↑."

use jackioh_engine::effects::add_random_from_catalog;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-055";

fn book_of_greed(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![add_random_from_catalog(json_as(json!({
                "query": { "rarity": ["Legendary", "Mythic"] },
                "count": param(&*ctx, "cards"),
                "radiant": radiant,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: book_of_greed(false),
        radiant: book_of_greed(true),
    }
}

// C+ #55 Book of Greed — SPEC §8.7 row 55, BUILD M9 Classic+ row C+ 55 (R97).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const BOOK: &str = "classicplus-055";
    const FILLER: &str = "core-005";

    use crate::scenario;

    fn book(radiant: bool, seed: Option<&str>, fillers: Option<usize>) -> Scenario {
        let mut hand = vec![json!({ "def": BOOK, "radiant": radiant })];
        hand.extend((0..fillers.unwrap_or(1)).map(|_| json!(FILLER)));
        scenario(json!({
            "seed": seed.unwrap_or("book-of-greed"),
            "p1": { "hand": hand },
            "p2": { "hand": [FILLER] },
        }))
    }

    /// S+ #121 Toxopid (Mythic), and the Crawler it adds to its owner's hand the first time it enters one
    /// (R3560000): that card is its own trigger's add, no pick of the Book's.
    const TOXOPID: &str = "stinkyscapeplus-121";
    const CRAWLER: &str = "stinkyscapeplus-120";

    /// The `addedToHand` events of p1 in the last step, as `(instanceId, defId)`, the Book's picks: a Toxopid
    /// among them leaves out the Crawler its arrival adds.
    fn added(s: &Scenario) -> Vec<(String, String)> {
        let found: Vec<(String, String)> = s
            .last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { player: PlayerId::P1, instance_id, def_id, .. } => {
                    Some((instance_id.clone(), def_id.clone()))
                }
                _ => None,
            })
            .collect();
        let toxopid = found.iter().any(|(_, def_id)| def_id == TOXOPID);
        found.into_iter().filter(|(_, def_id)| !toxopid || def_id != CRAWLER).collect()
    }

    fn added_cards(s: &Scenario) -> Vec<CardInstance> {
        added(s).iter().map(|(id, _)| s.card(id).clone()).collect()
    }

    fn legendary_or_mythic(rarity: Rarity) -> bool {
        rarity == Rarity::Legendary || rarity == Rarity::Mythic
    }

    #[test]
    fn is_a_1_spell_book_whose_text_agrees_with_its_count_at_every_value() {
        crate::register_all();
        let def = crate::card_def(super::ID);
        assert_eq!(def.id, BOOK);
        assert_eq!(
            fill_params(&def, FaceKind::Base, None),
            "Add 3 random Legendary or Mythic cards to your hand."
        );
        let one: IndexMap<String, i32> = IndexMap::from([("cards".to_string(), 1)]);
        assert_eq!(
            fill_params(&def, FaceKind::Base, Some(&one)),
            "Add 1 random Legendary or Mythic card to your hand."
        );
        assert_eq!(
            fill_params(&def, FaceKind::Radiant, Some(&one)),
            "Add 1 random Radiant Legendary or Mythic card to your hand."
        );
    }

    mod base {
        use super::*;

        #[test]
        fn adds_3_random_legendary_or_mythic_cards_to_your_hand() {
            let mut s = book(false, None, None);
            s.play(BOOK, json!({}));
            let cards = added_cards(&s);
            assert_eq!(cards.len(), 3);
            for card in &cards {
                assert!(legendary_or_mythic(def_of(Some(s.state()), &card.def_id).rarity));
                assert!(!card.radiant);
                assert_eq!(card.cost_override, None);
            }
        }

        #[test]
        fn r380_s5_over_many_seeds_every_set_never_a_token_whatever_rarity_it_prints() {
            let mut seen: IndexSet<String> = IndexSet::new();
            // Three picks a seed over 151 cards, three of them Meditative+'s: this many seeds bring every set up.
            for i in 0..300 {
                let mut s = book(false, Some(&format!("greed-{i}")), None);
                s.play(BOOK, json!({}));
                for (_, def_id) in added(&s) {
                    seen.insert(def_id);
                }
            }
            let fresh = book(false, None, None);
            let defs: Vec<CardDef> = seen.iter().map(|id| def_of(Some(fresh.state()), id).clone()).collect();
            assert!(defs.iter().all(|entry| !entry.token && legendary_or_mythic(entry.rarity)));
            assert!(!defs.iter().any(|entry| entry.printed_rarity.is_some()));
            let mut sets: Vec<String> = defs
                .iter()
                .map(|entry| entry.set.as_str().to_string())
                .collect::<IndexSet<String>>()
                .into_iter()
                .collect();
            sets.sort();
            // Meditative+ (#1187) by M+ #1, #24, #26; Stinkyscape (#1884); Training Grounds (#2204) by
            // #37, #38, #39, #40; Stinkyscape+ (#3573).
            assert_eq!(
                sets,
                vec![
                    "Classic".to_string(),
                    "Classic+".to_string(),
                    "Core".to_string(),
                    "Meditative".to_string(),
                    "Meditative+".to_string(),
                    "Stinkyscape".to_string(),
                    "Stinkyscape+".to_string(),
                    "Training Grounds".to_string(),
                ]
            );
        }

        #[test]
        fn s2_4_r4_a_full_hand_burns_what_does_not_fit() {
            let mut s = book(false, None, Some(8));
            s.play(BOOK, json!({}));
            assert_eq!(added(&s).len(), 2);
            assert_eq!(
                s.last_events().iter().filter(|event| matches!(event, GameEvent::Burned { .. })).count(),
                1
            );
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
            assert_eq!(events.len(), 3);
            for event in events {
                assert!(matches!(
                    event,
                    GameEvent::AddedToHand { instance_id, def_id, .. } if instance_id == "hidden" && def_id == "hidden"
                ));
            }
            let text = serde_json::to_string(&theirs).unwrap();
            for id in &ids {
                assert!(!text.contains(&format!("\"{id}\"")));
            }
        }

        #[test]
        fn r386_an_upgrade_adds_4_a_degrade_2_and_never_fewer_than_1() {
            let mut up = book(false, None, None);
            step_param(up.card_mut(BOOK), "cards", 1);
            up.play(BOOK, json!({}));
            assert_eq!(added(&up).len(), 4);

            let mut down = book(false, None, None);
            step_param(down.card_mut(BOOK), "cards", -1);
            down.play(BOOK, json!({}));
            assert_eq!(added(&down).len(), 2);

            let mut floor = book(false, None, None);
            step_param(floor.card_mut(BOOK), "cards", -9);
            floor.play(BOOK, json!({}));
            assert_eq!(added(&floor).len(), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r74_the_cards_are_radiant() {
            let mut s = book(true, None, None);
            s.play(BOOK, json!({}));
            let cards = added_cards(&s);
            assert_eq!(cards.len(), 3);
            assert!(cards.iter().all(|card| card.radiant));
            assert!(
                cards
                    .iter()
                    .all(|card| legendary_or_mythic(def_of(Some(s.state()), &card.def_id).rarity))
            );
        }

        /// R3563000: the cards of one play are different while the pool lasts (R60's repeats no longer stand
        /// for a card an effect generates): three picks from 151 are three cards; one seed adds the same ones.
        #[test]
        fn r3563000_the_picks_are_distinct_until_the_pool_runs_out_and_the_same_seed_adds_the_same_cards() {
            let mut a = book(true, Some("same"), None);
            let mut b = book(true, Some("same"), None);
            a.play(BOOK, json!({}));
            b.play(BOOK, json!({}));
            let defs = |s: &Scenario| -> Vec<String> { added(s).into_iter().map(|(_, def_id)| def_id).collect() };
            assert_eq!(defs(&a), defs(&b));
            for i in 0..200 {
                let mut s = book(true, Some(&format!("rep-{i}")), None);
                s.play(BOOK, json!({}));
                let ids = defs(&s);
                assert_eq!(ids.len(), 3, "rep-{i}");
                assert_eq!(ids.iter().collect::<IndexSet<_>>().len(), ids.len(), "rep-{i}: {ids:?}");
            }
        }
    }
}
