//! R1420: a set the catalog holds before it ships (`SHIPPED_SETS`). The Meditative set was one while
//! issue #496 built it and shipped in v0.3.5 (issue #553), so it now draws, deals and leans like Core;
//! the gate stays for the next set, which these tests stand in for with fixture cards of the reserved
//! Boss set, a set `SHIPPED_SETS` does not list. Such a set's cards are in no pool that names no set,
//! in no deck, and in no random deck, until the patch that lists the set; a pool that names the set,
//! or names its cards, reaches them; and the testkit's preview opens the set on one thread, as the
//! fuzz tool does. Also R1421 (a Prime pool holds the Prime tokens) and R1422 (`anyTags`), the pool
//! pieces the set's cards share, and R1371 (the newest shipped set, which a preview never moves).
//! R1178000: the Meditative+ set (issue #1173) is the next held set, in the catalog order after
//! Meditative until its release (#1187), and its fixture cards here are held by the same gate; its
//! Potion tag names no tribe.
//! Fixture definitions only (CLAUDE.md: the engine does not depend on crates/cards).

use jackioh_engine::catalog::{deckable, set_is_open};
use jackioh_engine::state::validate_deck;
use jackioh_engine::testkit::*;

fn face() -> Value {
    json!({ "keywords": [], "text": "fixture" })
}

fn card(id: &str, set: &str, index: &str, tags: &[&str], token: bool) -> CardDef {
    let mut tags: Vec<&str> = tags.to_vec();
    if token {
        tags.push("Token");
    }
    json_as(json!({
        "id": id,
        "index": index,
        "name": format!("Set fixture {id}"),
        "set": set,
        "type": "Spell",
        "tags": tags,
        "rarity": if token { "Token" } else { "Common" },
        "token": token,
        "cost": 1,
        "base": face(),
        "radiant": face(),
    }))
}

/// Twenty-one shipped Core cards (a deck's worth and one), two Meditative cards and a Meditative
/// token (shipped since v0.3.5), two cards and a token of the Boss set (which does not ship), two
/// cards and a token of the Meditative+ set (held, R1178000), two Prime tokens and a KY and a Book
/// card for the pools.
fn defs() -> CardDefs {
    let mut defs: Vec<CardDef> = (1..=21)
        .map(|n| card(&format!("core-{n:03}"), "Core", &n.to_string(), &[], false))
        .collect();
    defs.push(card("core-031", "Core", "31", &["KY"], false));
    defs.push(card("classic-003", "Classic", "3", &["Book"], false));
    defs.push(card("classicplus-038-1", "Classic+", "38.1", &["Prime"], true));
    defs.push(card(
        "classicplus-046-1",
        "Classic+",
        "46.1",
        &["Felinor", "Prime"],
        true,
    ));
    defs.push(card("classicplus-t-ai-01", "Classic+", "T-AI-1", &["AI"], true));
    defs.push(card("core-022", "Core", "22", &["Human"], false));
    defs.push(card("meditative-033", "Meditative", "33", &["CN"], false));
    defs.push(card("meditative-034", "Meditative", "34", &["CN", "KY"], false));
    defs.push(card(
        "meditative-045-1",
        "Meditative",
        "45.1",
        &["CN", "KY", "Prime"],
        true,
    ));
    defs.push(card("boss-001", "Boss", "1", &["CN"], false));
    defs.push(card("boss-002", "Boss", "2", &["CN", "KY"], false));
    defs.push(card("boss-002-1", "Boss", "2.1", &["CN", "KY", "Prime"], true));
    defs.push(card(
        "meditativeplus-011",
        "Meditative+",
        "11",
        &["Potion"],
        false,
    ));
    defs.push(card("meditativeplus-021", "Meditative+", "21", &["CN"], false));
    defs.push(card(
        "meditativeplus-016-1",
        "Meditative+",
        "16.1",
        &["Human", "Prime"],
        true,
    ));
    defs.into_iter().map(|def| (def.id.clone(), def)).collect()
}

fn setup() {
    register_catalog(defs());
}

fn pool(args: Value) -> Vec<String> {
    query(&json_as(args)).iter().map(|def| def.id.clone()).collect()
}

fn meditative(ids: &[String]) -> Vec<String> {
    ids.iter()
        .filter(|id| id.starts_with("meditative-"))
        .cloned()
        .collect()
}

/// The Boss fixture's ids among `ids`: the cards of a set that does not ship.
fn unshipped(ids: &[String]) -> Vec<String> {
    ids.iter().filter(|id| id.starts_with("boss-")).cloned().collect()
}

/// The Meditative+ fixture's ids among `ids` (R1178000).
fn meditative_plus(ids: &[String]) -> Vec<String> {
    ids.iter()
        .filter(|id| id.starts_with("meditativeplus-"))
        .cloned()
        .collect()
}

mod r1420_a_set_the_catalog_holds_before_it_ships {
    use super::*;

    /// The release (v0.3.5, issue #553) listed the Meditative set: it ships, in its catalog place
    /// after Classic+, and every set that ships is in the catalog order in that order. The reserved
    /// Boss sets ship no card.
    #[test]
    fn r1420_the_meditative_set_shipped_in_v0_3_5_and_keeps_its_place_in_the_catalog_order() {
        assert!(set_ships(SetName::Meditative));
        assert!(set_is_open(SetName::Meditative));
        assert!(SHIPPED_SETS.iter().all(|set| CATALOG_SETS.contains(set)));
        let places: Vec<usize> = SHIPPED_SETS
            .iter()
            .map(|set| {
                CATALOG_SETS
                    .iter()
                    .position(|listed| listed == set)
                    .expect("ordered")
            })
            .collect();
        assert!(places.windows(2).all(|pair| pair[0] < pair[1]), "{places:?}");
        assert_eq!(
            CATALOG_SETS.iter().position(|set| *set == SetName::Meditative),
            CATALOG_SETS
                .iter()
                .position(|set| *set == SetName::ClassicPlus)
                .map(|at| at + 1)
        );
        for reserved in [SetName::Boss, SetName::BossX] {
            assert!(!set_ships(reserved));
            assert!(!set_is_open(reserved));
        }
    }

    #[test]
    fn r1420_a_pool_that_names_no_set_never_draws_a_card_of_a_set_that_has_not_shipped() {
        setup();
        assert_eq!(unshipped(&pool(json!({}))), Vec::<String>::new());
        assert_eq!(unshipped(&pool(json!({ "tags": ["CN"] }))), Vec::<String>::new());
        assert_eq!(
            unshipped(&pool(json!({ "withTokens": true }))),
            Vec::<String>::new()
        );
        // The Meditative set ships, so the same pools draw its cards (its token only where a pool
        // asks for tokens, §5.1).
        assert_eq!(
            meditative(&pool(json!({ "tags": ["CN"] }))),
            vec!["meditative-033", "meditative-034"]
        );
        assert_eq!(
            meditative(&pool(json!({ "withTokens": true }))),
            vec!["meditative-033", "meditative-034", "meditative-045-1"]
        );
        assert_eq!(
            pool(json!({ "tags": ["KY"] })),
            vec!["core-031", "meditative-034"]
        );
    }

    #[test]
    fn r1420_a_pool_that_names_the_set_or_its_cards_reaches_them() {
        setup();
        assert_eq!(pool(json!({ "set": "Boss" })), vec!["boss-001", "boss-002"]);
        assert_eq!(
            pool(json!({ "defId": ["boss-002", "boss-002-1"] })),
            vec!["boss-002", "boss-002-1"]
        );
        assert_eq!(
            pool(json!({ "set": "Meditative" })),
            vec!["meditative-033", "meditative-034"]
        );
    }

    #[test]
    fn r1420_the_preview_opens_the_set_on_this_thread_until_its_guard_drops() {
        setup();
        {
            let _preview = preview_sets(&[SetName::Boss]);
            assert!(set_is_open(SetName::Boss));
            assert!(!set_ships(SetName::Boss), "a preview ships nothing");
            assert_eq!(
                unshipped(&pool(json!({ "tags": ["CN"] }))),
                vec!["boss-001", "boss-002"]
            );
            // The Meditative set keeps its place after Classic+ (B2.2), and a set the catalog order
            // does not name sorts after every set it does.
            let all = pool(json!({}));
            assert_eq!(
                &all[all.len() - 4..],
                ["meditative-033", "meditative-034", "boss-001", "boss-002"]
            );
        }
        assert!(!set_is_open(SetName::Boss));
        assert_eq!(unshipped(&pool(json!({}))), Vec::<String>::new());
        // A preview of a set that ships changes nothing: it was open already.
        {
            let before = pool(json!({}));
            let _preview = preview_sets(&[SetName::Meditative]);
            assert_eq!(pool(json!({})), before);
        }

        // Nested guards restore what was open before them: `preview_every_set` (every set the
        // catalog orders) inside a narrower preview, then the narrower one, then nothing.
        {
            let _outer = preview_sets(&[SetName::Boss]);
            {
                let _inner = preview_every_set();
                assert!(set_is_open(SetName::Boss));
                assert!(!set_is_open(SetName::BossX), "the catalog order names no Boss-X");
            }
            assert!(set_is_open(SetName::Boss));
        }
        assert!(!set_is_open(SetName::Boss));
    }

    #[test]
    fn r1420_no_deck_holds_a_card_of_a_set_that_has_not_shipped_unless_it_is_previewed() {
        setup();
        let catalog = defs();
        assert!(!deckable(&catalog["boss-001"]));
        assert!(deckable(&catalog["core-001"]));
        assert!(deckable(&catalog["meditative-033"]), "shipped since v0.3.5");
        assert!(!deckable(&catalog["classicplus-038-1"]), "a Token never is");
        assert!(!deckable(&catalog["meditative-045-1"]), "a Token never is");

        let mut shipped: Vec<String> = (1..=19).map(|n| format!("core-{n:03}")).collect();
        shipped.push("meditative-033".to_string());
        assert!(validate_deck(&shipped, &catalog, "p1", 20).is_ok());

        let mut deck: Vec<String> = (1..=19).map(|n| format!("core-{n:03}")).collect();
        deck.push("boss-001".to_string());
        let refused = validate_deck(&deck, &catalog, "p1", 20).expect_err("refused");
        assert!(refused.to_string().contains("has not shipped yet"), "{refused}");

        let _preview = preview_sets(&[SetName::Boss]);
        assert!(deckable(&catalog["boss-001"]));
        assert!(validate_deck(&deck, &catalog, "p1", 20).is_ok());
    }

    #[test]
    fn r1420_the_validator_refuses_a_queued_deck_that_holds_one_under_l3() {
        setup();
        let catalog = defs();
        let mut cards: Vec<String> = (1..=18).map(|n| format!("core-{n:03}")).collect();
        // A shipped Meditative card passes L3 since v0.3.5; the Boss card does not.
        cards.push("meditative-034".to_string());
        cards.push("boss-002".to_string());
        let collection: IndexMap<String, i32> = cards.iter().map(|id| (id.clone(), 1)).collect();
        let result = jackioh_engine::validator::validate_deck(&jackioh_engine::validator::DeckInput {
            deck: jackioh_engine::validator::LoadoutDeck {
                cards,
                ..Default::default()
            },
            catalog: jackioh_engine::validator::CatalogSnapshot {
                version: "test".to_string(),
                cards: catalog,
                banned: None,
            },
            collection,
        });
        let l3: Vec<String> = result
            .errors
            .iter()
            .filter(|error| error.rule == jackioh_engine::validator::LoadoutRule::L3)
            .map(|error| error.message.clone())
            .collect();
        assert_eq!(l3.len(), 1, "{:?}", result.errors);
        assert!(l3[0].contains("the Boss set has not shipped yet"), "{}", l3[0]);
    }
}

mod r1178000_the_meditative_plus_set_is_held_by_r1420_s_gate_until_its_release {
    use super::*;

    /// Meditative+ is the fifth set of the catalog order, right after Meditative, and does not ship:
    /// it is neither open nor the newest set, and no lean names it.
    #[test]
    fn r1178000_meditative_plus_follows_meditative_in_the_catalog_order_and_does_not_ship() {
        assert_eq!(SetName::MeditativePlus.as_str(), "Meditative+");
        assert_eq!(
            CATALOG_SETS
                .iter()
                .position(|set| *set == SetName::MeditativePlus),
            CATALOG_SETS
                .iter()
                .position(|set| *set == SetName::Meditative)
                .map(|at| at + 1)
        );
        assert_eq!(CATALOG_SETS.last(), Some(&SetName::MeditativePlus));
        assert!(!set_ships(SetName::MeditativePlus));
        assert!(!set_is_open(SetName::MeditativePlus));
        assert!(!SHIPPED_SETS.contains(&SetName::MeditativePlus));
        assert_eq!(newest_shipped_set(), SetName::Meditative);
        assert_eq!(LeanPick::parse("Meditative+"), None);
        assert!(!LeanPick::choices().contains(&LeanPick::Set(SetName::MeditativePlus)));
        let _preview = preview_sets(&[SetName::MeditativePlus]);
        assert!(set_is_open(SetName::MeditativePlus));
        assert_eq!(
            newest_shipped_set(),
            SetName::Meditative,
            "a preview ships nothing"
        );
    }

    #[test]
    fn r1178000_no_pool_that_names_no_set_reaches_it_and_one_that_names_it_does() {
        setup();
        for args in [
            json!({}),
            json!({ "tags": ["CN"] }),
            json!({ "tags": ["Potion"] }),
            json!({ "tags": ["Prime"] }),
            json!({ "anyTags": ["Potion", "Human"] }),
            json!({ "withTokens": true }),
        ] {
            assert_eq!(
                meditative_plus(&pool(args.clone())),
                Vec::<String>::new(),
                "{args}"
            );
        }
        assert_eq!(
            pool(json!({ "set": "Meditative+" })),
            vec!["meditativeplus-011", "meditativeplus-021"]
        );
        assert_eq!(
            pool(json!({ "set": "Meditative+", "tags": ["Potion"] })),
            vec!["meditativeplus-011"]
        );
        assert_eq!(
            pool(json!({ "defId": ["meditativeplus-016-1"] })),
            vec!["meditativeplus-016-1"]
        );
        // The preview opens it on this thread, in its catalog place: after Meditative, before the
        // Boss set, which the order does not name.
        let _preview = preview_sets(&[SetName::MeditativePlus, SetName::Boss]);
        assert_eq!(pool(json!({ "tags": ["Potion"] })), vec!["meditativeplus-011"]);
        let all = pool(json!({}));
        assert_eq!(
            &all[all.len() - 6..],
            [
                "meditative-033",
                "meditative-034",
                "meditativeplus-011",
                "meditativeplus-021",
                "boss-001",
                "boss-002"
            ]
        );
        // A Prime pool takes its Prime token (R1421) once the set is open.
        assert!(pool(json!({ "tags": ["Prime"] })).contains(&"meditativeplus-016-1".to_string()));
    }

    #[test]
    fn r1178000_no_deck_holds_one_until_it_ships_unless_it_is_previewed() {
        setup();
        let catalog = defs();
        assert!(!deckable(&catalog["meditativeplus-011"]));
        let mut deck: Vec<String> = (1..=19).map(|n| format!("core-{n:03}")).collect();
        deck.push("meditativeplus-011".to_string());
        let refused = validate_deck(&deck, &catalog, "p1", 20).expect_err("refused");
        assert!(
            refused
                .to_string()
                .contains("Meditative+, which has not shipped yet"),
            "{refused}"
        );

        let collection: IndexMap<String, i32> = deck.iter().map(|id| (id.clone(), 1)).collect();
        let result = jackioh_engine::validator::validate_deck(&jackioh_engine::validator::DeckInput {
            deck: jackioh_engine::validator::LoadoutDeck {
                cards: deck.clone(),
                ..Default::default()
            },
            catalog: jackioh_engine::validator::CatalogSnapshot {
                version: "test".to_string(),
                cards: catalog.clone(),
                banned: None,
            },
            collection,
        });
        let l3: Vec<&str> = result
            .errors
            .iter()
            .filter(|error| error.rule == jackioh_engine::validator::LoadoutRule::L3)
            .map(|error| error.message.as_str())
            .collect();
        assert_eq!(l3.len(), 1, "{:?}", result.errors);
        assert!(
            l3[0].contains("the Meditative+ set has not shipped yet"),
            "{}",
            l3[0]
        );

        let _preview = preview_sets(&[SetName::MeditativePlus]);
        assert!(deckable(&catalog["meditativeplus-011"]));
        assert!(!deckable(&catalog["meditativeplus-016-1"]), "a Token never is");
        assert!(validate_deck(&deck, &catalog, "p1", 20).is_ok());
    }

    /// The Potion tag is the set's one new tag: it parses as written and names no tribe (R1424).
    #[test]
    fn r1178000_the_potion_tag_is_a_family_tag_not_a_tribe() {
        assert_eq!(Tag::Potion.as_str(), "Potion");
        assert_eq!(json_as::<Tag>(json!("Potion")), Tag::Potion);
        assert!(!TRIBAL_TAGS.contains(&Tag::Potion));
        assert!(
            !POOL_TOKEN_TAGS.contains(&Tag::Potion),
            "a Potion pool holds cards only"
        );
        let at = |tag: Tag| Tag::ALL.iter().position(|each| *each == tag);
        assert_eq!(at(Tag::Potion), at(Tag::Wincon).map(|at| at + 1));
        assert_eq!(at(Tag::Token), at(Tag::Potion).map(|at| at + 1));
    }
}

mod r1371_the_newest_shipped_set {
    use super::*;

    #[test]
    fn r1371_the_newest_shipped_set_is_the_last_entry_of_shipped_sets_and_a_preview_never_moves_it() {
        let newest = newest_shipped_set();
        assert_eq!(SHIPPED_SETS.last(), Some(&newest));
        assert!(set_ships(newest));
        // No set that ships comes after it in the catalog order (R1420's `CATALOG_SETS`).
        let at = CATALOG_SETS
            .iter()
            .position(|set| *set == newest)
            .expect("the newest set is in the catalog order");
        assert!(CATALOG_SETS[at + 1..].iter().all(|set| !set_ships(*set)));
        // Classic+ until the release; the Meditative set from v0.3.5.
        assert_eq!(newest, SetName::Meditative);
        // A preview opens a set's pools on this thread; it ships nothing, so the newest set stays.
        let _preview = preview_sets(&[SetName::Boss]);
        assert_eq!(newest_shipped_set(), newest);
        assert_ne!(newest_shipped_set(), SetName::Boss);
    }
}

mod r1421_a_prime_or_ai_pool_holds_its_tokens {
    use super::*;

    #[test]
    fn r1421_a_prime_pool_is_the_prime_tokens_of_the_sets_that_ship() {
        setup();
        assert_eq!(
            pool(json!({ "tags": ["Prime"] })),
            vec!["classicplus-038-1", "classicplus-046-1", "meditative-045-1"]
        );
        // No other pool reaches them: a Felinor pool is cards only.
        assert_eq!(pool(json!({ "tags": ["Felinor"] })), Vec::<String>::new());
        // An AI pool is the AI generated cards, every one a token, alone or beside other tags.
        assert_eq!(pool(json!({ "tags": ["AI"] })), vec!["classicplus-t-ai-01"]);
        assert_eq!(
            pool(json!({ "anyTags": ["Human", "AI"] })),
            vec!["core-022", "classicplus-t-ai-01"]
        );
        // A set that has not shipped adds its Prime tokens only when previewed (R1420).
        let _preview = preview_sets(&[SetName::Boss]);
        assert_eq!(
            pool(json!({ "tags": ["Prime"] })),
            vec![
                "classicplus-038-1",
                "classicplus-046-1",
                "meditative-045-1",
                "boss-002-1"
            ]
        );
    }
}

mod r1422_any_tags_asks_for_at_least_one_of_its_tags {
    use super::*;

    #[test]
    fn r1422_any_tags_is_the_union_where_tags_is_the_intersection() {
        setup();
        assert_eq!(
            pool(json!({ "anyTags": ["KY", "Book"] })),
            vec!["core-031", "classic-003", "meditative-034"]
        );
        assert_eq!(pool(json!({ "tags": ["KY", "Book"] })), Vec::<String>::new());
        assert_eq!(
            pool(json!({ "anyTags": ["KY", "CN"] })),
            vec!["core-031", "meditative-033", "meditative-034"]
        );
        // A Prime tag among them takes its tokens, as `tags` does (R1421).
        assert_eq!(
            pool(json!({ "anyTags": ["Prime", "Book"] })),
            vec![
                "classic-003",
                "classicplus-038-1",
                "classicplus-046-1",
                "meditative-045-1"
            ]
        );
        // A set that has not shipped joins the union only when previewed (R1420).
        let _preview = preview_sets(&[SetName::Boss]);
        assert_eq!(
            pool(json!({ "anyTags": ["KY", "CN"] })),
            vec![
                "core-031",
                "meditative-033",
                "meditative-034",
                "boss-001",
                "boss-002"
            ]
        );
    }
}

mod r1424_the_tribal_tags_name_peoples_and_factions {
    use super::*;

    #[test]
    fn r1424_the_tribes_are_human_felinor_ky_cn_and_jlockeed_and_no_family_or_mechanic_tag() {
        assert_eq!(
            TRIBAL_TAGS,
            &[Tag::Human, Tag::Felinor, Tag::Ky, Tag::Cn, Tag::Jlockeed]
        );
        for tag in [
            Tag::Book,
            Tag::Fruit,
            Tag::Pancake,
            Tag::Ai,
            Tag::Prime,
            Tag::Wincon,
            Tag::Potion,
            Tag::Token,
        ] {
            assert!(!TRIBAL_TAGS.contains(&tag), "{tag} is no tribe");
        }
    }
}
