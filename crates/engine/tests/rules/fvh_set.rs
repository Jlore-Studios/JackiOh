//! F #5235's set (docs/felinors-vs-humans-set.md FV0, FV1, FV4; R5235000 to R5235002): Felinors vs Humans
//! is a `Held` set, its row and its share, the pools that never reach its cards and the preview that
//! opens it; the Plagiarized reprints, FvH #116 to #197, each its own card; and the three new tags.
//!
//! The pool and deck tests stand on this file's own fixtures (two cards of the set and a shipped Core
//! Felinor, over the engine's fixture catalog); the catalog tests read the real catalog
//! (`crates/cards/catalog.json`) and the real script registry (`jackioh_cards::register_all`).

use std::collections::BTreeSet;

use jackioh_engine::catalog::{deckable, set_is_open};
use jackioh_engine::state::validate_deck;
use jackioh_engine::testkit::*;
use jackioh_engine::wire::{SET_TABLE, SetRow, SetStage, set_row, shadow_ban_ceiling};

use crate::rules::fixtures::harness::{json_of, setup_catalog};

const FVH: SetName = SetName::FelinorsVsHumans;
/// How the set is written (FV1).
const FVH_NAME: &str = "Felinors vs Humans";
const SEGMENT: &str = "felinorsvshumans";
const ABBREVIATION: &str = "FvH";
/// Stinkyscape+'s share (FV1).
const SHARE: (usize, usize) = (1, 5);
/// FV1: 115 numbered cards, 22 tokens, 13 modules and 82 reprints.
const CATALOG_ENTRIES: usize = 232;
/// FV1: the set's non-token cards, the 115 numbered and the 82 reprints.
const NON_TOKEN_CARDS: usize = 197;
/// FV1: a fifth of `NON_TOKEN_CARDS`, rounded down.
const SHADOW_BAN_CEILING: usize = 39;
/// FV0's owner's call 2: the reprints are FvH #116 to #197.
const REPRINT_FIRST: u32 = 116;
const REPRINT_LAST: u32 = 197;
const REPRINTS: usize = 82;
/// FV4: a reprint's name is "Plagiarized: <IA name>".
const PLAGIARIZED_PREFIX: &str = "Plagiarized: ";
/// FV4: the one reprint with a second tag, Wincon.
const SINGULARITY: &str = "Plagiarized: Singularity";
const PLAGIARIZED: &str = "Plagiarized";
const WINCON: &str = "Wincon";
const ROBLOX_TOOL: &str = "Roblox-Tool";
const GOLEM: &str = "Golem";
/// R1420's refusal of a deck holding a card of a set that does not ship.
const NOT_SHIPPED: &str = "has not shipped yet";

/// A Felinor Unit of the set.
const FVH_CARD: &str = "felinorsvshumans-001";
/// A Felinor token of the set, sub-numbered as FV1 numbers one.
const FVH_TOKEN: &str = "felinorsvshumans-016-1";
/// A shipped Core Felinor Unit, which the same pools do reach.
const CORE_FELINOR: &str = "fvh-set-core-felinor";
/// Shipped vanilla Core Units of the fixture catalog (`fx-1`…), to fill a deck around one card.
const DECK_FILL: i32 = 19;

fn face() -> Value {
    json!({ "attack": 1, "health": 1, "keywords": [], "text": "set fixture" })
}

fn set_fixture(id: &str, set: &str, index: &str, token: bool) -> CardDef {
    let mut tags = vec!["Felinor"];
    if token {
        tags.push("Token");
    }
    json_as(json!({
        "id": id,
        "index": index,
        "name": format!("Set fixture {id}"),
        "set": set,
        "type": "Unit",
        "tags": tags,
        "rarity": if token { "Token" } else { "Common" },
        "token": token,
        "cost": 1,
        "base": face(),
        "radiant": face(),
    }))
}

/// The engine's fixture catalog with the set's two fixtures and the Core Felinor on top.
fn register() {
    setup_catalog();
    let mut catalog: CardDefs = registered_catalog().clone();
    for def in [
        set_fixture(FVH_CARD, FVH_NAME, "1", false),
        set_fixture(FVH_TOKEN, FVH_NAME, "16.1", true),
        set_fixture(CORE_FELINOR, "Core", "5235000", false),
    ] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
}

fn pool_ids(args: Value) -> Vec<String> {
    query(&json_as(args)).iter().map(|def| def.id.clone()).collect()
}

/// The set's cards among `ids`.
fn of_the_set(ids: &[String]) -> Vec<String> {
    ids.iter()
        .filter(|id| id.starts_with(&format!("{SEGMENT}-")))
        .cloned()
        .collect()
}

fn real_catalog() -> CardDefs {
    serde_json::from_str(include_str!("../../../cards/catalog.json"))
        .expect("crates/cards/catalog.json parses as CardDefs")
}

fn of_set(catalog: &CardDefs, set: SetName) -> Vec<&CardDef> {
    catalog.values().filter(|def| def.set == set).collect()
}

/// FvH #116 to #197: the set's entries whose index is a whole number in that range.
fn reprints(catalog: &CardDefs) -> Vec<&CardDef> {
    of_set(catalog, FVH)
        .into_iter()
        .filter(|def| {
            def.index
                .parse::<u32>()
                .is_ok_and(|number| (REPRINT_FIRST..=REPRINT_LAST).contains(&number))
        })
        .collect()
}

fn tag_names(def: &CardDef) -> BTreeSet<String> {
    json_of(&def.tags)
        .as_array()
        .map(|tags| {
            tags.iter()
                .filter_map(|tag| tag.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

fn names(list: &[&str]) -> BTreeSet<String> {
    list.iter().map(|name| name.to_string()).collect()
}

fn tag_named(name: &str) -> Option<Tag> {
    serde_json::from_value(json!(name)).ok()
}

mod r5235000_a_held_set {
    use super::*;

    #[test]
    fn r5235000_its_row_is_held_with_its_segment_abbreviation_and_share() {
        assert_eq!(
            *set_row(FVH),
            SetRow {
                set: FVH,
                stage: SetStage::Held,
                segment: SEGMENT,
                abbreviation: Some(ABBREVIATION),
                shadow_ban_share: Some(SHARE),
            }
        );
        assert_eq!(json_of(FVH), json!(FVH_NAME));
        assert_eq!(set_named(FVH_NAME), Some(FVH));
    }

    /// The set a name is written as, if any.
    fn set_named(name: &str) -> Option<SetName> {
        serde_json::from_value(json!(name)).ok()
    }

    /// Where `set` stands in `sets`.
    fn place(sets: &[SetName], set: SetName) -> Option<usize> {
        sets.iter().position(|listed| *listed == set)
    }

    #[test]
    fn r5235000_its_row_follows_the_infinite_assaults_and_it_follows_stinkyscape_plus_in_the_catalog_order() {
        let table: Vec<SetName> = SET_TABLE.iter().map(|row| row.set).collect();
        assert_eq!(
            place(&table, FVH),
            place(&table, SetName::InfiniteAssault).map(|at| at + 1)
        );
        assert_eq!(
            place(&CATALOG_SETS, FVH),
            place(&CATALOG_SETS, SetName::StinkyscapePlus).map(|at| at + 1)
        );
    }

    #[test]
    fn r5235000_its_shadow_ban_holds_at_most_a_fifth_of_its_197_cards() {
        assert_eq!(shadow_ban_ceiling(FVH, NON_TOKEN_CARDS), Some(SHADOW_BAN_CEILING));
    }

    #[test]
    fn r5235000_it_does_not_ship_and_is_not_open() {
        assert!(!set_ships(FVH));
        assert!(!SHIPPED_SETS.contains(&FVH));
        assert!(!set_is_open(FVH));
        assert_ne!(newest_shipped_set(), FVH);
    }

    #[test]
    fn r5235000_a_pool_that_names_no_set_never_draws_its_cards() {
        register();
        let ids = pool_ids(json!({}));
        assert!(
            ids.contains(&CORE_FELINOR.to_string()),
            "setup: the pool draws shipped cards"
        );
        assert_eq!(of_the_set(&ids), Vec::<String>::new());
    }

    #[test]
    fn r5235000_a_pool_with_tokens_never_draws_its_cards_or_tokens() {
        register();
        assert_eq!(
            of_the_set(&pool_ids(json!({ "withTokens": true }))),
            Vec::<String>::new()
        );
    }

    #[test]
    fn r5235000_a_tag_pool_never_draws_its_cards() {
        register();
        let ids = pool_ids(json!({ "tags": ["Felinor"] }));
        assert!(
            ids.contains(&CORE_FELINOR.to_string()),
            "setup: the tag pool draws"
        );
        assert_eq!(of_the_set(&ids), Vec::<String>::new());
    }

    #[test]
    fn r5235000_a_pool_that_names_the_set_or_its_cards_reaches_them() {
        register();
        assert_eq!(pool_ids(json!({ "set": FVH_NAME })), vec![FVH_CARD.to_string()]);
        let named: BTreeSet<String> = pool_ids(json!({ "defId": [FVH_CARD, FVH_TOKEN] }))
            .into_iter()
            .collect();
        assert_eq!(named, names(&[FVH_CARD, FVH_TOKEN]));
    }

    #[test]
    fn r5235000_no_card_of_it_is_deckable() {
        register();
        let catalog: CardDefs = registered_catalog().clone();
        assert!(!deckable(&catalog[FVH_CARD]));
        assert!(!deckable(&catalog[FVH_TOKEN]));
        assert!(deckable(&catalog[CORE_FELINOR]), "setup: a shipped card is");
    }

    #[test]
    fn r5235000_a_deck_holding_one_of_its_cards_is_refused() {
        register();
        let catalog: CardDefs = registered_catalog().clone();
        let mut deck: Vec<String> = (1..=DECK_FILL).map(|n| format!("fx-{n}")).collect();
        deck.push(CORE_FELINOR.to_string());
        let size = deck.len() as i32;
        assert!(
            validate_deck(&deck, &catalog, "p1", size).is_ok(),
            "setup: the shipped deck passes"
        );
        deck.pop();
        deck.push(FVH_CARD.to_string());
        let refused = validate_deck(&deck, &catalog, "p1", size).expect_err("refused");
        assert!(refused.to_string().contains(NOT_SHIPPED), "{refused}");
    }

    #[test]
    fn r5235000_preview_sets_opens_it_until_its_guard_drops() {
        register();
        {
            let _preview = preview_sets(&[FVH]);
            assert!(set_is_open(FVH));
            assert!(!set_ships(FVH), "a preview ships nothing");
            assert_eq!(of_the_set(&pool_ids(json!({}))), vec![FVH_CARD.to_string()]);
            assert!(deckable(&registered_catalog()[FVH_CARD]));
        }
        assert!(!set_is_open(FVH));
        assert_eq!(of_the_set(&pool_ids(json!({}))), Vec::<String>::new());
    }

    #[test]
    fn r5235000_a_preview_of_another_set_leaves_it_closed() {
        register();
        let _preview = preview_sets(&[SetName::Boss]);
        assert!(!set_is_open(FVH));
        assert_eq!(of_the_set(&pool_ids(json!({}))), Vec::<String>::new());
    }

    #[test]
    fn r5235000_the_catalog_holds_its_232_entries_under_its_segment() {
        let catalog = real_catalog();
        let entries = of_set(&catalog, FVH);
        assert_eq!(entries.len(), CATALOG_ENTRIES);
        assert_eq!(entries.iter().filter(|def| !def.token).count(), NON_TOKEN_CARDS);
        let strays: Vec<&str> = entries
            .iter()
            .filter(|def| !def.id.starts_with(&format!("{SEGMENT}-")))
            .map(|def| def.id.as_str())
            .collect();
        assert_eq!(strays, Vec::<&str>::new());
    }
}

mod r5235001_the_plagiarized_reprints {
    use super::*;

    #[test]
    fn r5235001_the_82_reprints_are_fvh_116_to_197_each_its_own_card_named_for_its_ia_card() {
        let catalog = real_catalog();
        let reprinted = reprints(&catalog);
        assert_eq!(reprinted.len(), REPRINTS);
        let numbers: BTreeSet<u32> = reprinted
            .iter()
            .filter_map(|def| def.index.parse().ok())
            .collect();
        assert_eq!(numbers, (REPRINT_FIRST..=REPRINT_LAST).collect::<BTreeSet<u32>>());
        for def in &reprinted {
            let number: u32 = def.index.parse().expect("a reprint's index is its number");
            assert_eq!(def.id, format!("{SEGMENT}-{number:03}"));
            let ia_name = def.name.strip_prefix(PLAGIARIZED_PREFIX).unwrap_or_else(|| {
                panic!(
                    "{} is named {:?}, not \"Plagiarized: <IA name>\"",
                    def.id, def.name
                )
            });
            let copied = catalog
                .values()
                .find(|other| other.set == SetName::InfiniteAssault && other.name == ia_name);
            assert!(
                copied.is_some_and(|ia| ia.id != def.id),
                "{}: no Infinite Assault card is named {ia_name:?}",
                def.id
            );
        }
    }

    #[test]
    fn r5235001_a_reprints_only_tag_is_plagiarized_and_singularitys_wincon_besides() {
        let catalog = real_catalog();
        let reprinted = reprints(&catalog);
        assert!(
            reprinted.iter().any(|def| def.name == SINGULARITY),
            "setup: Singularity is reprinted"
        );
        for def in reprinted {
            let expected = if def.name == SINGULARITY {
                names(&[PLAGIARIZED, WINCON])
            } else {
                names(&[PLAGIARIZED])
            };
            assert_eq!(tag_names(def), expected, "{}", def.id);
        }
    }

    #[test]
    fn r5235001_no_reprint_is_a_token() {
        let catalog = real_catalog();
        let tokens: Vec<&str> = reprints(&catalog)
            .into_iter()
            .filter(|def| def.token)
            .map(|def| def.id.as_str())
            .collect();
        assert_eq!(tokens, Vec::<&str>::new());
    }

    #[test]
    fn r5235001_no_infinite_assault_card_carries_plagiarized() {
        let catalog = real_catalog();
        let tagged: Vec<&str> = of_set(&catalog, SetName::InfiniteAssault)
            .into_iter()
            .filter(|def| def.tags.contains(&Tag::Plagiarized))
            .map(|def| def.id.as_str())
            .collect();
        assert_eq!(tagged, Vec::<&str>::new());
    }

    #[test]
    fn r5235001_each_reprint_has_a_script_under_its_own_id() {
        jackioh_cards::register_all();
        let catalog = real_catalog();
        let scripts = registered_scripts();
        let missing: Vec<&str> = reprints(&catalog)
            .into_iter()
            .filter(|def| !scripts.contains_key(def.id.as_str()))
            .map(|def| def.id.as_str())
            .collect();
        assert_eq!(missing, Vec::<&str>::new());
    }
}

mod r5235002_the_new_tags {
    use super::*;

    #[test]
    fn r5235002_plagiarized_roblox_tool_and_golem_are_tags() {
        assert_eq!(tag_named(PLAGIARIZED), Some(Tag::Plagiarized));
        for name in [PLAGIARIZED, ROBLOX_TOOL, GOLEM] {
            let tag = tag_named(name).unwrap_or_else(|| panic!("{name} is no tag"));
            assert_eq!(json_of(tag), json!(name), "{name} is written as named");
        }
    }

    #[test]
    fn r5235002_no_other_spelling_of_them_is_a_tag() {
        for misspelt in ["Roblox Tool", "RobloxTool", "Plagiarised", "Golems"] {
            assert_eq!(tag_named(misspelt), None, "{misspelt}");
        }
    }
}
