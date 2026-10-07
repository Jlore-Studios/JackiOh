//! Patch v0.2.0's pools (docs/classic-sets.md B2.2, B2.6, B4.1): one format across Core, Classic
//! and Classic+ (R380), the Fruit pool's Grapes and Dropshipping's "including tokens" (R382), and a
//! card that never generates itself, named by its id because an index repeats across sets (R387).
//! Fixture definitions only (CLAUDE.md: the engine does not depend on packages/cards).
//!
//! Port of `packages/engine/test/pools.test.ts`. TS's `beforeEach` is `setup()` at the top of each
//! test (the registry override is per thread, and each test runs on its own).

use std::borrow::Borrow;

use jackioh_engine::testkit::*;

/// TS `FACE`.
fn face() -> Value {
    json!({ "keywords": [], "text": "fixture" })
}

/// TS `{ ...base, ...extra }`: every key of `extra` written over `base`.
fn spread(mut base: Value, extra: Value) -> Value {
    if let (Some(target), Value::Object(extra)) = (base.as_object_mut(), extra) {
        for (key, value) in extra {
            target.insert(key, value);
        }
    }
    base
}

/// TS `card(id, set, index, overrides = {})`: `set` as the catalog spells it ("Classic+").
fn card(id: &str, set: &str, index: &str, overrides: Value) -> CardDef {
    json_as(spread(
        json!({
            "id": id,
            "index": index,
            "name": format!("Pool fixture {id}"),
            "set": set,
            "type": "Spell",
            "tags": [],
            "rarity": "Common",
            "token": false,
            "cost": 1,
            "base": face(),
            "radiant": face(),
        }),
        overrides,
    ))
}

fn token(id: &str, set: &str, index: &str, tags: &[&str]) -> CardDef {
    let mut all: Vec<&str> = tags.to_vec();
    all.push("Token");
    card(id, set, index, json!({ "tags": all, "rarity": "Token", "token": true }))
}

/// Three sets that reuse the index "43", a Fruit card in each, and tokens of both kinds.
fn defs() -> Vec<CardDef> {
    vec![
        card("core-043", "Core", "43", json!({})),
        card("core-047", "Core", "47", json!({ "tags": ["Fruit"] })),
        card("classic-043", "Classic", "43", json!({})),
        card("classic-010", "Classic", "10", json!({})),
        card("classicplus-043", "Classic+", "43", json!({})),
        card("classicplus-058", "Classic+", "58", json!({ "tags": ["Fruit"] })),
        card("classicplus-065", "Classic+", "65", json!({ "tags": ["Fruit"] })),
        token("classicplus-065-1", "Classic+", "65.1", &["Fruit"]),
        token("classicplus-065-2", "Classic+", "65.2", &["Fruit"]),
        token("core-065-1", "Core", "65.1", &[]),
        token("core-t-rush", "Core", "T-rush", &[]),
    ]
}

fn catalog_of(defs: Vec<CardDef>) -> CardDefs {
    defs.into_iter().map(|def| (def.id.clone(), def)).collect()
}

fn ids<D: Borrow<CardDef>>(defs: &[D]) -> Vec<String> {
    defs.iter().map(|def| def.borrow().id.clone()).collect()
}

/// A query from its TS object literal.
fn q(args: Value) -> CatalogQueryArgs {
    json_as(args)
}

/// TS `ids(query(args))`.
fn pool(args: Value) -> Vec<String> {
    ids(&query(&q(args)))
}

fn as_json(args: &CatalogQueryArgs) -> Value {
    serde_json::to_value(args).expect("serialises")
}

/// TS `beforeEach`.
fn setup() {
    // Registered in reverse, so the order `query` returns cannot be the registry's insertion order.
    let mut reversed = defs();
    reversed.reverse();
    register_catalog(catalog_of(reversed));
}

/// R380: one format, and a pool that names no set draws from every set
mod r380_one_format_and_a_pool_that_names_no_set_draws_from_every_set {
    use super::*;

    #[test]
    fn r380_a_plain_query_reaches_core_classic_and_classic_plus_set_by_set_in_set_order_then_by_index() {
        setup();
        assert_eq!(
            pool(json!({})),
            vec![
                "core-043",
                "core-047",
                "classic-010",
                "classic-043",
                "classicplus-043",
                "classicplus-058",
                "classicplus-065",
            ]
        );
    }

    #[test]
    fn r380_a_pool_that_names_its_set_keeps_to_it_and_a_list_of_sets_reads_as_or() {
        setup();
        assert_eq!(pool(json!({ "set": "Core" })), vec!["core-043", "core-047"]);
        assert_eq!(
            pool(json!({ "set": ["Classic", "Classic+"] })),
            vec!["classic-010", "classic-043", "classicplus-043", "classicplus-058", "classicplus-065"]
        );
    }

    #[test]
    fn r380_b2_2_an_index_names_a_card_only_within_its_set_so_43_never_finds_two_cards() {
        setup();
        let id_of = |set: SetName, index: &str| def_by_index(set, index).map(|def| def.id.clone());
        assert_eq!(id_of(SetName::Core, "43").as_deref(), Some("core-043"));
        assert_eq!(id_of(SetName::Classic, "43").as_deref(), Some("classic-043"));
        assert_eq!(id_of(SetName::ClassicPlus, "43").as_deref(), Some("classicplus-043"));
        // Core's #65.1 and Classic+'s #65.1 share an index and are two different tokens.
        assert_eq!(id_of(SetName::Core, "65.1").as_deref(), Some("core-065-1"));
        assert_eq!(id_of(SetName::ClassicPlus, "65.1").as_deref(), Some("classicplus-065-1"));
        assert_eq!(id_of(SetName::Classic, "65.1"), None);
    }
}

/// R382: the Fruit pool holds the Grapes, and only a pool that takes every token reaches others
mod r382_the_fruit_pool_holds_the_grapes_and_only_a_pool_that_takes_every_token_reaches_others {
    use super::*;

    #[test]
    fn r382_a_fruit_pool_is_the_non_token_fruit_cards_of_every_set_plus_the_fruit_tokens_the_grapes() {
        setup();
        assert_eq!(
            pool(json!({ "tags": ["Fruit"] })),
            vec!["core-047", "classicplus-058", "classicplus-065", "classicplus-065-1", "classicplus-065-2"]
        );
    }

    #[test]
    fn r382_the_grapes_are_in_no_other_pool_a_plain_query_a_type_pool_and_a_set_pool_leave_them_out() {
        setup();
        assert!(!pool(json!({})).contains(&"classicplus-065-1".to_string()));
        assert!(!pool(json!({ "type": "Spell" })).contains(&"classicplus-065-1".to_string()));
        assert!(!pool(json!({ "set": "Classic+" })).contains(&"classicplus-065-2".to_string()));
        // A token that is no Fruit stays out of the Fruit pool.
        assert!(!pool(json!({ "tags": ["Fruit"] })).contains(&"core-065-1".to_string()));
    }

    #[test]
    fn r382_with_tokens_takes_every_token_of_every_set_beside_the_cards_dropshipping_itself_excluded() {
        setup();
        let pool = ids(&query(&excluding_def_id(&q(json!({ "withTokens": true })), Some("classicplus-058"))));
        assert_eq!(
            pool,
            vec![
                "core-043",
                "core-047",
                "core-065-1",
                "core-t-rush",
                "classic-010",
                "classic-043",
                "classicplus-043",
                "classicplus-065",
                "classicplus-065-1",
                "classicplus-065-2",
            ]
        );
        assert!(!pool.contains(&"classicplus-058".to_string()));
    }
}

/// R387: a card never generates itself, named by its id
mod r387_a_card_never_generates_itself_named_by_its_id {
    use super::*;

    #[test]
    fn r387_exclude_def_id_removes_exactly_that_card_not_every_card_that_shares_its_index() {
        setup();
        let pool = pool(json!({ "excludeDefId": "classic-043" }));
        assert!(!pool.contains(&"classic-043".to_string()));
        assert!(pool.contains(&"core-043".to_string()));
        assert!(pool.contains(&"classicplus-043".to_string()));
    }

    #[test]
    fn r387_excluding_def_id_adds_the_running_card_s_id_to_what_the_caller_excluded_never_replacing_it() {
        setup();
        assert_eq!(
            as_json(&excluding_def_id(&q(json!({ "excludeDefId": "core-043" })), Some("classic-010"))),
            json!({ "excludeDefId": ["core-043", "classic-010"] })
        );
        // Already excluded: the query is handed back as it was.
        let asked = q(json!({ "excludeDefId": ["classic-010"] }));
        assert_eq!(as_json(&excluding_def_id(&asked, Some("classic-010"))), as_json(&asked));
        assert_eq!(as_json(&excluding_def_id(&asked, None)), as_json(&asked));
    }

    #[test]
    fn r387_a_fused_card_excludes_every_ingredient_s_definition_a_fused_ingredient_s_included() {
        setup();
        assert_eq!(
            fused_id_parts("t-2:(t-1:classic-043+core-047)+classicplus-058"),
            Some(vec!["t-1:classic-043+core-047".to_string(), "classicplus-058".to_string()])
        );
        assert_eq!(
            self_def_ids("t-2:(t-1:classic-043+core-047)+classicplus-058"),
            vec!["classic-043", "core-047", "classicplus-058"]
        );
        let pool = ids(&query(&excluding_def_id(
            &q(json!({})),
            Some("t-2:(t-1:classic-043+core-047)+classicplus-058"),
        )));
        assert_eq!(pool, vec!["core-043", "classic-010", "classicplus-043", "classicplus-065"]);
        // A catalog card stands for itself alone; a bare crafted id has no ingredients.
        assert_eq!(self_def_ids("classic-043"), vec!["classic-043"]);
        assert_eq!(fused_id_parts("t-3"), None);
    }
}
