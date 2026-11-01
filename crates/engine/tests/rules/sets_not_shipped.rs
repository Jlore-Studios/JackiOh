//! R1420: a set the catalog holds before it ships (`SHIPPED_SETS`). The gate's tests stand in for the
//! next set with fixture cards of the reserved Boss set, a set `SHIPPED_SETS` does not list. Such a
//! set's cards are in no pool that names no set, in no deck, and in no random deck, until the patch
//! that lists the set; a pool that names the set, or names its cards, reaches them; and the testkit's
//! preview opens the set on one thread, as the fuzz tool does. Also R1421 (a Prime pool holds the
//! Prime tokens) and R1422 (`anyTags`), the pool pieces the set's cards share, and R1371 (the newest
//! shipped set, which a preview never moves).
//! The Meditative (issue #496), Meditative+ (R1178000, issue #1173), Stinkyscape (R1857000) and
//! Training Grounds (R2194000, the newest) sets have shipped, so their fixture cards here draw, deal and
//! lean like Core's. Their tags (Potion, Stinkyscape's four, #2521's Resource and Faction) name no tribe.
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

/// Twenty-one shipped Core cards (a deck's worth and one), two cards and a token of the Boss set
/// (which does not ship), and for the pools a few cards and tokens of each shipped set: Prime and AI
/// generated tokens, a KY and a Book card, and each later set's own tags.
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
    defs.push(card("meditativeplus-001-1", "Meditative+", "1.1", &["AI"], true));
    defs.push(card("meditativeplus-001-2", "Meditative+", "1.2", &["AI"], true));
    defs.push(card(
        "stinkyscape-027",
        "Stinkyscape",
        "27",
        &["Drone Core"],
        false,
    ));
    defs.push(card("stinkyscape-050", "Stinkyscape", "50", &["Weapon"], false));
    defs.push(card("stinkyscape-091", "Stinkyscape", "91", &["Swarm"], false));
    defs.push(card(
        "stinkyscape-137-1",
        "Stinkyscape",
        "137.1",
        &["War Doctrine"],
        true,
    ));
    defs.push(card(
        "stinkyscape-t-ai-01",
        "Stinkyscape",
        "T-AI-1",
        &["AI"],
        true,
    ));
    defs.push(card(
        "traininggrounds-001",
        "Training Grounds",
        "1",
        &["Human"],
        false,
    ));
    defs.push(card(
        "traininggrounds-039",
        "Training Grounds",
        "39",
        &["Call to Chaos"],
        false,
    ));
    defs.push(card(
        "traininggrounds-039-1",
        "Training Grounds",
        "39.1",
        &[],
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

/// The Stinkyscape fixture's ids among `ids` (R1857000).
fn stinkyscape(ids: &[String]) -> Vec<String> {
    ids.iter()
        .filter(|id| id.starts_with("stinkyscape-"))
        .cloned()
        .collect()
}

/// The Training Grounds fixture's ids among `ids` (R2194000).
fn training_grounds(ids: &[String]) -> Vec<String> {
    ids.iter()
        .filter(|id| id.starts_with("traininggrounds-"))
        .cloned()
        .collect()
}

mod r1420_a_set_the_catalog_holds_before_it_ships {
    use super::*;

    /// The Meditative set ships (issue #553), in its catalog place after Classic+, and every set that
    /// ships is in the catalog order in that order. The reserved Boss sets ship no card.
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
            // The Meditative, Meditative+, Stinkyscape and Training Grounds sets keep their places
            // after Classic+ (B2.2), and a set the catalog order does not name sorts after every set
            // it does.
            let all = pool(json!({}));
            assert_eq!(
                &all[all.len() - 11..],
                [
                    "meditative-033",
                    "meditative-034",
                    "meditativeplus-011",
                    "meditativeplus-021",
                    "stinkyscape-027",
                    "stinkyscape-050",
                    "stinkyscape-091",
                    "traininggrounds-001",
                    "traininggrounds-039",
                    "boss-001",
                    "boss-002"
                ]
            );
        }
        assert!(!set_is_open(SetName::Boss));
        assert_eq!(unshipped(&pool(json!({}))), Vec::<String>::new());
        // A preview of a set that ships changes nothing: it was open already, so it adds nothing to
        // a preview of a set that has not shipped.
        {
            let before = {
                let _boss = preview_sets(&[SetName::Boss]);
                pool(json!({}))
            };
            let _preview = preview_sets(&[SetName::Meditative, SetName::Boss]);
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
        assert!(deckable(&catalog["meditative-033"]), "shipped since v0.3.0");
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
        // A shipped Meditative card passes L3; the Boss card does not.
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

mod r1178000_the_meditative_plus_set_ships_from_its_release_in_its_place_after_meditative {
    use super::*;

    /// Meditative+ is the fifth set of the catalog order, right after Meditative, and ships (#1187):
    /// it is open and a lean names it, as Stinkyscape's after it (#1884, R1857000). The reserved Boss
    /// set, which does not ship, has no lean.
    #[test]
    fn r1178000_meditative_plus_follows_meditative_in_the_catalog_order_and_ships_as_the_newest_set() {
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
        // Fifth: the Stinkyscape set follows it (R1857000).
        assert_eq!(CATALOG_SETS.get(4), Some(&SetName::MeditativePlus));
        assert!(set_ships(SetName::MeditativePlus));
        assert!(set_is_open(SetName::MeditativePlus));
        assert_eq!(SHIPPED_SETS.get(4), Some(&SetName::MeditativePlus));
        assert_eq!(SHIPPED_SETS.get(5), Some(&SetName::Stinkyscape));
        assert_eq!(
            LeanPick::parse("Meditative+"),
            Some(LeanPick::Set(SetName::MeditativePlus))
        );
        assert!(LeanPick::choices().contains(&LeanPick::Set(SetName::MeditativePlus)));
        assert_eq!(LeanPick::parse("Boss"), None);
        assert!(!LeanPick::choices().contains(&LeanPick::Set(SetName::Boss)));
        let _preview = preview_sets(&[SetName::Boss]);
        assert_eq!(
            newest_shipped_set(),
            SetName::TrainingGrounds,
            "a preview ships nothing"
        );
    }

    #[test]
    fn r1178000_a_pool_that_names_no_set_reaches_it_and_one_that_names_it_keeps_to_it() {
        setup();
        // Whatever tags it asks for, the set's own Potion included; its tokens only where a pool asks
        // for tokens (§5.1), or for the Prime or AI tag they carry (R1421), so a Human pool leaves
        // out Jazakus Prime.
        for (args, expected) in [
            (json!({}), vec!["meditativeplus-011", "meditativeplus-021"]),
            (json!({ "tags": ["CN"] }), vec!["meditativeplus-021"]),
            (json!({ "tags": ["Potion"] }), vec!["meditativeplus-011"]),
            (json!({ "tags": ["Prime"] }), vec!["meditativeplus-016-1"]),
            (
                json!({ "tags": ["AI"] }),
                vec!["meditativeplus-001-1", "meditativeplus-001-2"],
            ),
            (
                json!({ "anyTags": ["Potion", "Human"] }),
                vec!["meditativeplus-011"],
            ),
            (
                json!({ "withTokens": true }),
                vec![
                    "meditativeplus-001-1",
                    "meditativeplus-001-2",
                    "meditativeplus-011",
                    "meditativeplus-016-1",
                    "meditativeplus-021",
                ],
            ),
        ] {
            assert_eq!(meditative_plus(&pool(args.clone())), expected, "{args}");
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
        // In its catalog place, after Meditative and before Stinkyscape and Training Grounds, the
        // sets that ship after it (R1857000, R2194000); a preview of it adds nothing to one of a set
        // that has not shipped, since it is open already.
        let all = pool(json!({}));
        assert_eq!(
            &all[all.len() - 9..],
            [
                "meditative-033",
                "meditative-034",
                "meditativeplus-011",
                "meditativeplus-021",
                "stinkyscape-027",
                "stinkyscape-050",
                "stinkyscape-091",
                "traininggrounds-001",
                "traininggrounds-039"
            ]
        );
        let before = {
            let _boss = preview_sets(&[SetName::Boss]);
            pool(json!({}))
        };
        let _preview = preview_sets(&[SetName::MeditativePlus, SetName::Boss]);
        assert_eq!(pool(json!({})), before);
    }

    #[test]
    fn r1178000_a_deck_holds_its_cards_since_it_shipped_and_never_its_tokens() {
        setup();
        let catalog = defs();
        assert!(deckable(&catalog["meditativeplus-011"]));
        assert!(!deckable(&catalog["meditativeplus-016-1"]), "a Token never is");
        let mut deck: Vec<String> = (1..=19).map(|n| format!("core-{n:03}")).collect();
        deck.push("meditativeplus-011".to_string());
        assert!(validate_deck(&deck, &catalog, "p1", 20).is_ok());

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
        assert_eq!(l3, Vec::<&str>::new(), "{:?}", result.errors);
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
        // #1837's five tags follow it, and Token stays last (R1837000).
        assert_eq!(at(Tag::Terminus), at(Tag::Potion).map(|at| at + 1));
        assert_eq!(at(Tag::Token), Some(Tag::ALL.len() - 1));
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
        assert_eq!(newest, SetName::TrainingGrounds);
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
        // Meditative+ #16.1 Jazakus Prime among them since the set shipped (R1178000).
        assert_eq!(
            pool(json!({ "tags": ["Prime"] })),
            vec![
                "classicplus-038-1",
                "classicplus-046-1",
                "meditative-045-1",
                "meditativeplus-016-1"
            ]
        );
        // No other pool reaches them: a Felinor pool is cards only, and a Human pool leaves out the
        // Human Jazakus Prime (and holds Training Grounds #1 since that set shipped, #2204).
        assert_eq!(pool(json!({ "tags": ["Felinor"] })), Vec::<String>::new());
        assert_eq!(
            pool(json!({ "tags": ["Human"] })),
            vec!["core-022", "traininggrounds-001"]
        );
        // An AI pool is the AI generated cards, every one a token, alone or beside other tags.
        assert_eq!(
            pool(json!({ "tags": ["AI"] })),
            vec![
                "classicplus-t-ai-01",
                "meditativeplus-001-1",
                "meditativeplus-001-2",
                "stinkyscape-t-ai-01"
            ]
        );
        assert_eq!(
            pool(json!({ "anyTags": ["Human", "AI"] })),
            vec![
                "core-022",
                "classicplus-t-ai-01",
                "meditativeplus-001-1",
                "meditativeplus-001-2",
                "stinkyscape-t-ai-01",
                "traininggrounds-001"
            ]
        );
        // A set that has not shipped adds its Prime tokens only when previewed (R1420).
        let _preview = preview_sets(&[SetName::Boss]);
        assert_eq!(
            pool(json!({ "tags": ["Prime"] })),
            vec![
                "classicplus-038-1",
                "classicplus-046-1",
                "meditative-045-1",
                "meditativeplus-016-1",
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
            vec![
                "core-031",
                "meditative-033",
                "meditative-034",
                "meditativeplus-021"
            ]
        );
        // A Prime tag among them takes its tokens, as `tags` does (R1421).
        assert_eq!(
            pool(json!({ "anyTags": ["Prime", "Book"] })),
            vec![
                "classic-003",
                "classicplus-038-1",
                "classicplus-046-1",
                "meditative-045-1",
                "meditativeplus-016-1"
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
                "meditativeplus-021",
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
            Tag::Terminus,
            Tag::Quest,
            Tag::Otherworldly,
            Tag::OtherOrigins,
            Tag::JellyBean,
            Tag::DroneCore,
            Tag::Weapon,
            Tag::WarDoctrine,
            Tag::Swarm,
            Tag::Legacy,
            Tag::Resource,
            Tag::Faction,
            Tag::Token,
        ] {
            assert!(!TRIBAL_TAGS.contains(&tag), "{tag} is no tribe");
        }
    }
}

/// R1837000: the five tags #1837 puts on shipped cards ahead of the Stinkyscape set. Fixture cards
/// only (CLAUDE.md: the engine does not depend on crates/cards); the real cards' tags are
/// `crates/cards/tests/cross/catalog.rs`'s.
mod r1837000_terminus_quest_otherworldly_other_origins_and_jelly_bean {
    use super::*;

    const NEW: [(Tag, &str); 5] = [
        (Tag::Terminus, "Terminus"),
        (Tag::Quest, "Quest"),
        (Tag::Otherworldly, "Otherworldly"),
        (Tag::OtherOrigins, "Other-Origins"),
        (Tag::JellyBean, "Jelly Bean"),
    ];

    #[test]
    fn r1837000_the_five_parse_as_written_follow_potion_in_the_union_and_name_no_tribe() {
        let at = |tag: Tag| Tag::ALL.iter().position(|each| *each == tag);
        let potion = at(Tag::Potion).expect("Potion is in the union");
        for (offset, (tag, text)) in NEW.iter().enumerate() {
            assert_eq!(tag.as_str(), *text);
            assert_eq!(json_as::<Tag>(json!(text)), *tag);
            assert_eq!(at(*tag), Some(potion + 1 + offset), "{tag}");
            assert!(!TRIBAL_TAGS.contains(tag), "{tag} is no tribe");
            assert!(!POOL_TOKEN_TAGS.contains(tag), "{tag}'s pool takes no token");
        }
        // The Stinkyscape set's four follow them (R1857000), and Token stays last.
        assert_eq!(at(Tag::DroneCore), Some(potion + 1 + NEW.len()));
        assert_eq!(at(Tag::Token), Some(Tag::ALL.len() - 1));
    }

    /// A pool by one of the five holds that tag's cards and, as any tag outside `POOL_TOKEN_TAGS`,
    /// none of its tokens unless it asks for them: Other-Origins, carried by tokens alone, draws an
    /// empty pool until a card that reads it decides otherwise.
    #[test]
    fn r1837000_a_pool_by_one_of_them_holds_its_cards_and_none_of_its_tokens() {
        let defs: Vec<CardDef> = vec![
            card("core-026", "Core", "26", &["Jelly Bean"], false),
            card("core-027", "Core", "27", &["Jelly Bean"], false),
            card("core-100", "Core", "100", &["Terminus"], false),
            card("classic-090", "Classic", "90", &["Quickdraw", "Quest"], false),
            card("classicplus-032", "Classic+", "32", &["Otherworldly"], false),
            card("classicplus-032-1", "Classic+", "32.1", &["Other-Origins"], true),
            card("classicplus-032-2", "Classic+", "32.2", &["Other-Origins"], true),
        ];
        register_catalog(defs.into_iter().map(|def| (def.id.clone(), def)).collect());
        assert_eq!(
            pool(json!({ "tags": ["Jelly Bean"] })),
            vec!["core-026", "core-027"]
        );
        assert_eq!(pool(json!({ "tags": ["Terminus"] })), vec!["core-100"]);
        assert_eq!(pool(json!({ "tags": ["Quest"] })), vec!["classic-090"]);
        assert_eq!(pool(json!({ "tags": ["Otherworldly"] })), vec!["classicplus-032"]);
        assert_eq!(pool(json!({ "tags": ["Other-Origins"] })), Vec::<String>::new());
        assert_eq!(
            pool(json!({ "tags": ["Other-Origins"], "withTokens": true })),
            vec!["classicplus-032-1", "classicplus-032-2"]
        );
    }
}

/// R2511000: Legacy, the Training Grounds Legacy cards' tag (#41–#60), is a family tag no rule reads:
/// it parses as written, follows Swarm in the union with Token still last, names no tribe, and a pool
/// by it holds its cards and none of their tokens.
mod r2511000_the_legacy_tag {
    use super::*;

    #[test]
    fn r2511000_legacy_parses_as_written_follows_swarm_and_names_no_tribe() {
        let at = |tag: Tag| Tag::ALL.iter().position(|each| *each == tag);
        assert_eq!(Tag::Legacy.as_str(), "Legacy");
        assert_eq!(json_as::<Tag>(json!("Legacy")), Tag::Legacy);
        assert_eq!(at(Tag::Legacy), at(Tag::Swarm).map(|at| at + 1));
        assert_eq!(at(Tag::Token), Some(Tag::ALL.len() - 1));
        assert!(!TRIBAL_TAGS.contains(&Tag::Legacy));
        assert!(!POOL_TOKEN_TAGS.contains(&Tag::Legacy));

        let defs: Vec<CardDef> = vec![
            card("core-002", "Core", "2", &["Human"], false),
            card(
                "traininggrounds-041",
                "Training Grounds",
                "41",
                &["Human", "Legacy"],
                false,
            ),
            card(
                "traininggrounds-042",
                "Training Grounds",
                "42",
                &["Legacy"],
                false,
            ),
        ];
        register_catalog(defs.into_iter().map(|def| (def.id.clone(), def)).collect());
        assert_eq!(
            pool(json!({ "tags": ["Legacy"] })),
            vec!["traininggrounds-041", "traininggrounds-042"]
        );
        assert_eq!(
            pool(json!({ "tags": ["Human"] })),
            vec!["core-002", "traininggrounds-041"]
        );
    }
}

/// R1857000: the Stinkyscape set (issue #1836) was the third set R1420's gate held, as R1178000 said
/// of Meditative+: sixth in the catalog order, right after Meditative+, and in no pool that names no
/// set, no deck and no lean until its release (#1884) listed it in `SHIPPED_SETS`. Its four tags are
/// family tags that no rule reads (R1424).
mod r1857000_the_stinkyscape_set_ships_after_meditative_plus {
    use super::*;

    /// The set's four tags, as written.
    const TAGS: [(Tag, &str); 4] = [
        (Tag::DroneCore, "Drone Core"),
        (Tag::Weapon, "Weapon"),
        (Tag::WarDoctrine, "War Doctrine"),
        (Tag::Swarm, "Swarm"),
    ];

    #[test]
    fn r1857000_stinkyscape_follows_meditative_plus_and_ships() {
        assert_eq!(SetName::Stinkyscape.as_str(), "Stinkyscape");
        assert_eq!(json_as::<SetName>(json!("Stinkyscape")), SetName::Stinkyscape);
        let at = |set: SetName| CATALOG_SETS.iter().position(|each| *each == set);
        assert_eq!(
            at(SetName::Stinkyscape),
            at(SetName::MeditativePlus).map(|at| at + 1)
        );
        // Shipped: it is open and a lean names it.
        assert!(set_ships(SetName::Stinkyscape));
        assert_eq!(SHIPPED_SETS.get(5), Some(&SetName::Stinkyscape));
        assert!(set_is_open(SetName::Stinkyscape));
        assert_eq!(
            LeanPick::parse("Stinkyscape"),
            Some(LeanPick::Set(SetName::Stinkyscape))
        );
        assert!(LeanPick::choices().contains(&LeanPick::Set(SetName::Stinkyscape)));
        // A preview of it changes nothing, and a preview of a held set ships nothing.
        {
            let _preview = preview_sets(&[SetName::Stinkyscape, SetName::Boss]);
            assert!(set_is_open(SetName::Stinkyscape));
            assert_eq!(newest_shipped_set(), SetName::TrainingGrounds);
        }
        assert!(set_is_open(SetName::Stinkyscape));
    }

    #[test]
    fn r1857000_its_four_tags_parse_as_written_follow_jelly_bean_and_name_no_tribe() {
        let at = |tag: Tag| Tag::ALL.iter().position(|each| *each == tag);
        let bean = at(Tag::JellyBean).expect("Jelly Bean is in the union");
        for (offset, (tag, text)) in TAGS.iter().enumerate() {
            assert_eq!(tag.as_str(), *text);
            assert_eq!(json_as::<Tag>(json!(text)), *tag);
            assert_eq!(at(*tag), Some(bean + 1 + offset), "{tag}");
            assert!(!TRIBAL_TAGS.contains(tag), "{tag} is no tribe");
            assert!(!POOL_TOKEN_TAGS.contains(tag), "{tag}'s pool takes no token");
        }
        // R2511000: Legacy follows them, then issue #2521's Resource and Faction, and Token stays last.
        assert_eq!(at(Tag::Legacy), Some(bean + 1 + TAGS.len()));
        assert_eq!(at(Tag::Resource), Some(bean + 2 + TAGS.len()));
        assert_eq!(at(Tag::Token), Some(Tag::ALL.len() - 1));
    }

    #[test]
    fn r1857000_a_pool_that_names_no_set_reaches_it_and_one_that_names_it_keeps_to_it() {
        setup();
        // Whatever it asks for: its own tags, the AI generated cards (R1421) or every token; its
        // tokens only where a pool asks for tokens (§5.1) or for the AI tag they carry.
        for (args, expected) in [
            (
                json!({}),
                vec!["stinkyscape-027", "stinkyscape-050", "stinkyscape-091"],
            ),
            (json!({ "tags": ["Drone Core"] }), vec!["stinkyscape-027"]),
            (json!({ "tags": ["Weapon"] }), vec!["stinkyscape-050"]),
            (
                json!({ "anyTags": ["Swarm", "War Doctrine"] }),
                vec!["stinkyscape-091"],
            ),
            (json!({ "tags": ["AI"] }), vec!["stinkyscape-t-ai-01"]),
            (
                json!({ "withTokens": true }),
                vec![
                    "stinkyscape-027",
                    "stinkyscape-050",
                    "stinkyscape-091",
                    "stinkyscape-137-1",
                    "stinkyscape-t-ai-01",
                ],
            ),
        ] {
            assert_eq!(stinkyscape(&pool(args.clone())), expected, "{args}");
        }
        assert_eq!(
            pool(json!({ "set": "Stinkyscape" })),
            vec!["stinkyscape-027", "stinkyscape-050", "stinkyscape-091"]
        );
        assert_eq!(
            pool(json!({ "set": "Stinkyscape", "tags": ["War Doctrine"], "withTokens": true })),
            vec!["stinkyscape-137-1"]
        );
        assert_eq!(
            pool(json!({ "defId": ["stinkyscape-t-ai-01"] })),
            vec!["stinkyscape-t-ai-01"]
        );
        // In its catalog place, after Meditative+ and before Training Grounds, the set that ships
        // after it (R2194000), and the Boss set the order does not name.
        let _preview = preview_sets(&[SetName::Boss]);
        let all = pool(json!({}));
        assert_eq!(
            &all[all.len() - 9..],
            [
                "meditativeplus-011",
                "meditativeplus-021",
                "stinkyscape-027",
                "stinkyscape-050",
                "stinkyscape-091",
                "traininggrounds-001",
                "traininggrounds-039",
                "boss-001",
                "boss-002"
            ]
        );
    }

    #[test]
    fn r1857000_a_deck_holds_its_cards_since_it_shipped_and_never_its_tokens() {
        setup();
        let catalog = defs();
        assert!(deckable(&catalog["stinkyscape-027"]));
        assert!(!deckable(&catalog["stinkyscape-137-1"]), "a Token never is");
        assert!(!deckable(&catalog["stinkyscape-t-ai-01"]), "a Token never is");
        let mut deck: Vec<String> = (1..=19).map(|n| format!("core-{n:03}")).collect();
        deck.push("stinkyscape-050".to_string());
        assert!(validate_deck(&deck, &catalog, "p1", 20).is_ok());

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
        assert_eq!(l3, Vec::<&str>::new(), "{:?}", result.errors);
    }
}

/// Issue #2521: Resource (the six Stinkyscape Resources, Reknite to Red Narcor, every one a token)
/// and Faction (the five Faction cards, Stinkyscape #95 and #159 to #162) follow Legacy (R2511000),
/// itself after the Stinkyscape set's four, in the union, ahead of Token. Family tags that name no tribe
/// (R1424), whose pools take no token unless they ask for tokens, as every tag outside `POOL_TOKEN_TAGS`
/// (R382).
mod issue_2521_resource_and_faction_follow_legacy {
    use super::*;

    /// The two tags, as written, in the union's order.
    const TAGS: [(Tag, &str); 2] = [(Tag::Resource, "Resource"), (Tag::Faction, "Faction")];

    #[test]
    fn the_two_parse_as_written_follow_legacy_and_name_no_tribe() {
        let at = |tag: Tag| Tag::ALL.iter().position(|each| *each == tag);
        let legacy = at(Tag::Legacy).expect("Legacy is in the union");
        for (offset, (tag, text)) in TAGS.iter().enumerate() {
            assert_eq!(tag.as_str(), *text);
            assert_eq!(json_as::<Tag>(json!(text)), *tag);
            assert_eq!(at(*tag), Some(legacy + 1 + offset), "{tag}");
            assert!(!TRIBAL_TAGS.contains(tag), "{tag} is no tribe");
            assert!(!POOL_TOKEN_TAGS.contains(tag), "{tag}'s pool takes no token");
        }
        assert_eq!(at(Tag::Token), Some(legacy + 1 + TAGS.len()));
        assert_eq!(at(Tag::Token), Some(Tag::ALL.len() - 1));
    }

    /// A Faction card is a card a deck may hold; a Resource is a token, which a pool reaches only
    /// when it asks for tokens and no deck ever holds.
    #[test]
    fn a_faction_pool_holds_its_cards_and_a_resource_pool_its_tokens_only_when_it_asks_for_them() {
        let defs: CardDefs = [
            card("stinkyscape-095", "Stinkyscape", "95", &["Faction"], false),
            card("stinkyscape-159", "Stinkyscape", "159", &["Faction"], false),
            card(
                "stinkyscape-t-korrelite",
                "Stinkyscape",
                "T-korrelite",
                &["Resource"],
                true,
            ),
            card(
                "stinkyscape-t-narcor",
                "Stinkyscape",
                "T-narcor",
                &["Resource"],
                true,
            ),
        ]
        .into_iter()
        .map(|def| (def.id.clone(), def))
        .collect();
        assert!(deckable(&defs["stinkyscape-159"]));
        assert!(!deckable(&defs["stinkyscape-t-korrelite"]), "a Token never is");
        register_catalog(defs);
        assert_eq!(
            pool(json!({ "tags": ["Faction"] })),
            vec!["stinkyscape-095", "stinkyscape-159"]
        );
        assert_eq!(pool(json!({ "tags": ["Resource"] })), Vec::<String>::new());
        assert_eq!(
            pool(json!({ "tags": ["Resource"], "withTokens": true })),
            vec!["stinkyscape-t-korrelite", "stinkyscape-t-narcor"]
        );
        assert_eq!(
            pool(json!({ "anyTags": ["Resource", "Faction"] })),
            vec!["stinkyscape-095", "stinkyscape-159"]
        );
    }
}

/// R2194000: the Training Grounds set (issue #2193) was the fourth set R1420's gate held, as R1857000
/// said of Stinkyscape: seventh in the catalog order, right after Stinkyscape, and in no pool that
/// names no set, no deck and no lean until its release (#2204) listed it in `SHIPPED_SETS`. Since then
/// it ships as the newest set (R1371), and its pools reach it like every shipped set's (R2194001).
mod r2194000_the_training_grounds_set_ships_after_stinkyscape {
    use super::*;

    #[test]
    fn r2194000_training_grounds_follows_stinkyscape_and_ships_as_the_newest_set() {
        assert_eq!(SetName::TrainingGrounds.as_str(), "Training Grounds");
        assert_eq!(
            json_as::<SetName>(json!("Training Grounds")),
            SetName::TrainingGrounds
        );
        let at = |set: SetName| CATALOG_SETS.iter().position(|each| *each == set);
        assert_eq!(
            at(SetName::TrainingGrounds),
            at(SetName::Stinkyscape).map(|at| at + 1)
        );
        assert_eq!(CATALOG_SETS.last(), Some(&SetName::TrainingGrounds));
        // Shipped: it is open, it is the newest set, and a lean names it.
        assert!(set_ships(SetName::TrainingGrounds));
        assert_eq!(SHIPPED_SETS.last(), Some(&SetName::TrainingGrounds));
        assert!(set_is_open(SetName::TrainingGrounds));
        assert_eq!(newest_shipped_set(), SetName::TrainingGrounds);
        assert_eq!(
            LeanPick::parse("Training Grounds"),
            Some(LeanPick::Set(SetName::TrainingGrounds))
        );
        assert!(LeanPick::choices().contains(&LeanPick::Set(SetName::TrainingGrounds)));
        assert_eq!(LeanPick::Newest.resolve(), SetName::TrainingGrounds);
        // A preview of it changes nothing, and a preview of a held set ships nothing.
        {
            let _preview = preview_sets(&[SetName::TrainingGrounds, SetName::Boss]);
            assert!(set_is_open(SetName::TrainingGrounds));
            assert_eq!(newest_shipped_set(), SetName::TrainingGrounds);
        }
        assert!(set_is_open(SetName::TrainingGrounds));
    }

    #[test]
    fn r2194000_r2194001_a_pool_that_names_no_set_reaches_it_and_one_that_names_it_keeps_to_it() {
        setup();
        // Whatever an unnamed pool asks for: its tribe, its tag, every token; its token only where a
        // pool asks for tokens (§5.1).
        for (args, expected) in [
            (json!({}), vec!["traininggrounds-001", "traininggrounds-039"]),
            (json!({ "tags": ["Human"] }), vec!["traininggrounds-001"]),
            (json!({ "tags": ["Call to Chaos"] }), vec!["traininggrounds-039"]),
            (
                json!({ "withTokens": true }),
                vec![
                    "traininggrounds-001",
                    "traininggrounds-039",
                    "traininggrounds-039-1",
                ],
            ),
        ] {
            assert_eq!(training_grounds(&pool(args.clone())), expected, "{args}");
        }
        assert_eq!(
            pool(json!({ "set": "Training Grounds" })),
            vec!["traininggrounds-001", "traininggrounds-039"]
        );
        assert_eq!(
            pool(json!({ "set": "Training Grounds", "withTokens": true })),
            vec![
                "traininggrounds-001",
                "traininggrounds-039",
                "traininggrounds-039-1"
            ]
        );
        assert_eq!(
            pool(json!({ "defId": ["traininggrounds-039-1"] })),
            vec!["traininggrounds-039-1"]
        );
        // In its catalog place, last of the sets that ship: after Stinkyscape, before the Boss set
        // the order does not name.
        let _preview = preview_sets(&[SetName::Boss]);
        let all = pool(json!({}));
        assert_eq!(
            &all[all.len() - 5..],
            [
                "stinkyscape-091",
                "traininggrounds-001",
                "traininggrounds-039",
                "boss-001",
                "boss-002"
            ]
        );
    }

    #[test]
    fn r2194000_a_deck_holds_its_cards_since_it_shipped_and_never_its_token() {
        setup();
        let catalog = defs();
        assert!(deckable(&catalog["traininggrounds-001"]));
        assert!(!deckable(&catalog["traininggrounds-039-1"]), "a Token never is");
        let mut deck: Vec<String> = (1..=19).map(|n| format!("core-{n:03}")).collect();
        deck.push("traininggrounds-001".to_string());
        assert!(validate_deck(&deck, &catalog, "p1", 20).is_ok());

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
        assert_eq!(l3, Vec::<&str>::new(), "{:?}", result.errors);
    }
}

/// R2195000, R2511001: a Legacy card (Training Grounds #41–#60) is its own card, so a deck holds it
/// beside the Core card it copies: `create_game`'s check and the shared validator's L3 take the two
/// together, as they take either alone. R2195000's pair rule is superseded; a definition that still
/// names its Core card (`snapshotOf`, the catalogs shipped from v0.5.5 to v0.5.7) is judged the same.
mod r2195000_r2511001_a_deck_holds_a_core_card_and_its_legacy_card_together {
    use super::*;
    use jackioh_engine::validator::{CatalogSnapshot, DeckInput, LoadoutDeck, LoadoutRule};

    /// `defs()` with core-002's Legacy card beside it, naming core-002 as a carried catalog of
    /// v0.5.5 to v0.5.7 did (`marked`) or not, as the build's does.
    fn with_legacy(marked: bool) -> CardDefs {
        let mut catalog = defs();
        let mut legacy = card("traininggrounds-041", "Training Grounds", "41", &[], false);
        legacy.name = format!("Legacy: {}", catalog["core-002"].name);
        legacy.snapshot_of = marked.then(|| "core-002".to_string());
        catalog.insert(legacy.id.clone(), legacy);
        catalog
    }

    /// The validator's L3 messages for `deck`, every card of it owned.
    fn l3(deck: &[String], catalog: &CardDefs) -> Vec<String> {
        let collection: IndexMap<String, i32> = deck.iter().map(|id| (id.clone(), 1)).collect();
        let result = jackioh_engine::validator::validate_deck(&DeckInput {
            deck: LoadoutDeck {
                cards: deck.to_vec(),
                ..Default::default()
            },
            catalog: CatalogSnapshot {
                version: "test".to_string(),
                cards: catalog.clone(),
                banned: None,
            },
            collection,
        });
        result
            .errors
            .iter()
            .filter(|error| error.rule == LoadoutRule::L3)
            .map(|error| error.message.clone())
            .collect()
    }

    #[test]
    fn r2195000_r2511001_both_are_taken_together_and_either_alone() {
        for marked in [false, true] {
            register_catalog(with_legacy(marked));
            let catalog = with_legacy(marked);
            let core: Vec<String> = (1..=20).map(|n| format!("core-{n:03}")).collect();
            let mut both = core.clone();
            both[19] = "traininggrounds-041".to_string();
            let mut legacy = core.clone();
            legacy[1] = "traininggrounds-041".to_string();
            assert!(both.contains(&"core-002".to_string()));
            for deck in [&core, &legacy, &both] {
                assert!(
                    validate_deck(deck, &catalog, "p1", 20).is_ok(),
                    "{marked} {deck:?}"
                );
                assert_eq!(l3(deck, &catalog), Vec::<String>::new(), "{marked}");
            }
        }
    }
}
