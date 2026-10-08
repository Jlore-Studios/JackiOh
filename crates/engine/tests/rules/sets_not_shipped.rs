//! R1420: a set the catalog holds before it ships (`SHIPPED_SETS`), the Meditative set while issue
//! #496 builds it. Its cards are in no pool that names no set, in no deck, and in no random deck,
//! until the patch that lists the set; a pool that names the set, or names its cards, reaches them;
//! and the testkit's preview opens the set on one thread, as the fuzz tool does. Also R1421 (a Prime
//! pool holds the Prime tokens) and R1422 (`anyTags`), the pool pieces the set's cards share.
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
/// token, two Prime tokens and a KY and a Book card for the pools.
fn defs() -> CardDefs {
    let mut defs: Vec<CardDef> = (1..=21)
        .map(|n| card(&format!("core-{n:03}"), "Core", &n.to_string(), &[], false))
        .collect();
    defs.push(card("core-031", "Core", "31", &["KY"], false));
    defs.push(card("classic-003", "Classic", "3", &["Book"], false));
    defs.push(card("classicplus-038-1", "Classic+", "38.1", &["Prime"], true));
    defs.push(card("classicplus-046-1", "Classic+", "46.1", &["Felinor", "Prime"], true));
    defs.push(card("classicplus-t-ai-01", "Classic+", "T-AI-1", &["AI"], true));
    defs.push(card("core-022", "Core", "22", &["Human"], false));
    defs.push(card("meditative-033", "Meditative", "33", &["CN"], false));
    defs.push(card("meditative-034", "Meditative", "34", &["CN", "KY"], false));
    defs.push(card("meditative-045-1", "Meditative", "45.1", &["CN", "KY", "Prime"], true));
    defs.into_iter().map(|def| (def.id.clone(), def)).collect()
}

fn setup() {
    register_catalog(defs());
}

fn pool(args: Value) -> Vec<String> {
    query(&json_as(args)).iter().map(|def| def.id.clone()).collect()
}

fn meditative(ids: &[String]) -> Vec<String> {
    ids.iter().filter(|id| id.starts_with("meditative-")).cloned().collect()
}

mod r1420_a_set_the_catalog_holds_before_it_ships {
    use super::*;

    #[test]
    fn r1420_the_meditative_set_is_in_the_catalog_and_does_not_ship_yet() {
        assert!(!set_ships(SetName::Meditative));
        assert!(SHIPPED_SETS.iter().all(|set| CATALOG_SETS.contains(set)));
        assert!(CATALOG_SETS.contains(&SetName::Meditative));
    }

    #[test]
    fn r1420_a_pool_that_names_no_set_never_draws_a_card_of_a_set_that_has_not_shipped() {
        setup();
        assert_eq!(meditative(&pool(json!({}))), Vec::<String>::new());
        assert_eq!(meditative(&pool(json!({ "tags": ["CN"] }))), Vec::<String>::new());
        assert_eq!(meditative(&pool(json!({ "withTokens": true }))), Vec::<String>::new());
        assert_eq!(pool(json!({ "tags": ["KY"] })), vec!["core-031"]);
    }

    #[test]
    fn r1420_a_pool_that_names_the_set_or_its_cards_reaches_them() {
        setup();
        assert_eq!(
            pool(json!({ "set": "Meditative" })),
            vec!["meditative-033", "meditative-034"]
        );
        assert_eq!(
            pool(json!({ "defId": ["meditative-034", "meditative-045-1"] })),
            vec!["meditative-034", "meditative-045-1"]
        );
    }

    #[test]
    fn r1420_the_preview_opens_the_set_on_this_thread_until_its_guard_drops() {
        setup();
        {
            let _preview = preview_sets(&[SetName::Meditative]);
            assert!(set_is_open(SetName::Meditative));
            assert_eq!(
                meditative(&pool(json!({ "tags": ["CN"] }))),
                vec!["meditative-033", "meditative-034"]
            );
            // A set keeps its place in the catalog order: Meditative after Classic+ (B2.2).
            let all = pool(json!({}));
            assert_eq!(all.last().map(String::as_str), Some("meditative-034"));
        }
        assert!(!set_is_open(SetName::Meditative));
        assert_eq!(meditative(&pool(json!({}))), Vec::<String>::new());

        let other_thread = std::thread::spawn(|| {
            register_catalog(defs());
            let _preview = preview_every_set();
            set_is_open(SetName::Meditative)
        });
        assert!(other_thread.join().expect("the thread runs"));
        assert!(!set_is_open(SetName::Meditative), "a preview reaches no other thread");
    }

    #[test]
    fn r1420_no_deck_holds_a_card_of_a_set_that_has_not_shipped_unless_it_is_previewed() {
        setup();
        let catalog = defs();
        assert!(!deckable(&catalog["meditative-033"]));
        assert!(deckable(&catalog["core-001"]));
        assert!(!deckable(&catalog["classicplus-038-1"]), "a Token never is");

        let mut deck: Vec<String> = (1..=19).map(|n| format!("core-{n:03}")).collect();
        deck.push("meditative-033".to_string());
        let refused = validate_deck(&deck, &catalog, "p1", 20).expect_err("refused");
        assert!(refused.to_string().contains("has not shipped yet"), "{refused}");

        let _preview = preview_sets(&[SetName::Meditative]);
        assert!(deckable(&catalog["meditative-033"]));
        assert!(validate_deck(&deck, &catalog, "p1", 20).is_ok());
    }

    #[test]
    fn r1420_the_validator_refuses_a_queued_deck_that_holds_one_under_l3() {
        setup();
        let catalog = defs();
        let mut cards: Vec<String> = (1..=19).map(|n| format!("core-{n:03}")).collect();
        cards.push("meditative-034".to_string());
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
        assert!(l3[0].contains("the Meditative set has not shipped yet"), "{}", l3[0]);
    }
}

mod r1421_a_prime_or_ai_pool_holds_its_tokens {
    use super::*;

    #[test]
    fn r1421_a_prime_pool_is_the_prime_tokens_of_the_sets_that_ship() {
        setup();
        assert_eq!(
            pool(json!({ "tags": ["Prime"] })),
            vec!["classicplus-038-1", "classicplus-046-1"]
        );
        // No other pool reaches them: a Felinor pool is cards only.
        assert_eq!(pool(json!({ "tags": ["Felinor"] })), Vec::<String>::new());
        // An AI pool is the AI generated cards, every one a token, alone or beside other tags.
        assert_eq!(pool(json!({ "tags": ["AI"] })), vec!["classicplus-t-ai-01"]);
        assert_eq!(
            pool(json!({ "anyTags": ["Human", "AI"] })),
            vec!["core-022", "classicplus-t-ai-01"]
        );
        let _preview = preview_sets(&[SetName::Meditative]);
        assert_eq!(
            pool(json!({ "tags": ["Prime"] })),
            vec!["classicplus-038-1", "classicplus-046-1", "meditative-045-1"]
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
            vec!["core-031", "classic-003"]
        );
        assert_eq!(pool(json!({ "tags": ["KY", "Book"] })), Vec::<String>::new());
        let _preview = preview_sets(&[SetName::Meditative]);
        assert_eq!(
            pool(json!({ "anyTags": ["KY", "CN"] })),
            vec!["core-031", "meditative-033", "meditative-034"]
        );
        // A Prime tag among them takes its tokens, as `tags` does (R1421).
        assert_eq!(
            pool(json!({ "anyTags": ["Prime", "Book"] })),
            vec!["classic-003", "classicplus-038-1", "classicplus-046-1", "meditative-045-1"]
        );
    }
}

mod r1424_the_tribal_tags_name_peoples_and_factions {
    use super::*;

    #[test]
    fn r1424_the_tribes_are_human_felinor_ky_cn_and_jlockeed_and_no_family_or_mechanic_tag() {
        assert_eq!(TRIBAL_TAGS, &[Tag::Human, Tag::Felinor, Tag::Ky, Tag::Cn, Tag::Jlockeed]);
        for tag in [Tag::Book, Tag::Fruit, Tag::Pancake, Tag::Ai, Tag::Prime, Tag::Wincon, Tag::Token] {
            assert!(!TRIBAL_TAGS.contains(&tag), "{tag} is no tribe");
        }
    }
}
