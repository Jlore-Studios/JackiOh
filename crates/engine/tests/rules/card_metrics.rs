//! R1846000–R1846002 (issue #1846, Stinkyscape's card metrics): the BUSTED and complicated pools, which
//! `crates/cards/build.rs` compiles in from every script's code lines; `everySet` (Stinkyscape #109's).
//! Fixtures only (CLAUDE.md: the engine does not depend on crates/cards); the real lists are proved in
//! `crates/cards/tests/cross/card_metrics.rs`.

use jackioh_engine::testkit::*;

fn face() -> Value {
    json!({ "keywords": [], "text": "fixture" })
}

fn card(id: &str, set: &str, index: &str, token: bool) -> CardDef {
    json_as(json!({
        "id": id,
        "index": index,
        "name": format!("Metrics fixture {id}"),
        "set": set,
        "type": "Spell",
        "tags": if token { vec!["Token"] } else { vec![] },
        "rarity": if token { "Token" } else { "Common" },
        "token": token,
        "cost": 1,
        "base": face(),
        "radiant": face(),
    }))
}

/// Core cards on each side of both thresholds, a long Core token, a long card of each single-player set
/// and of Custom, standing in for a set the catalog holds before it ships (R1420).
fn defs() -> CardDefs {
    [
        card("core-001", "Core", "1", false),
        card("core-002", "Core", "2", false),
        card("core-003", "Core", "3", false),
        card("core-004", "Core", "4", false),
        card("core-005", "Core", "5", false),
        card("core-005-1", "Core", "5.1", true),
        card("boss-001", "Boss", "1", false),
        card("bossx-001", "Boss-X", "1", false),
        card("held-001", "Custom", "1", false),
    ]
    .into_iter()
    .map(|def| (def.id.clone(), def))
    .collect()
}

fn code_lines() -> Vec<(&'static str, usize)> {
    vec![
        ("core-001", BUSTED_CODE_LINES + 1),
        ("core-002", BUSTED_CODE_LINES),
        ("core-003", COMPLICATED_CODE_LINES + 1),
        ("core-004", COMPLICATED_CODE_LINES),
        ("core-005", 3),
        ("core-005-1", BUSTED_CODE_LINES * 2),
        ("boss-001", BUSTED_CODE_LINES * 2),
        ("bossx-001", BUSTED_CODE_LINES * 2),
        ("held-001", BUSTED_CODE_LINES * 2),
    ]
}

fn setup() {
    register_catalog(defs());
    register_code_lines(code_lines());
}

fn pool(args: Value) -> Vec<String> {
    query(&json_as::<CatalogQueryArgs>(args))
        .into_iter()
        .map(|def| def.id.clone())
        .collect()
}

#[test]
fn r1846000_the_thresholds_are_the_designers_numbers() {
    // "Over 100 lines" is BUSTED and "over 60 lines" complicated: a card on the number is neither.
    assert_eq!(BUSTED_CODE_LINES, 100);
    assert_eq!(COMPLICATED_CODE_LINES, 60);
}

#[test]
fn r1846000_a_card_over_a_threshold_is_in_its_list_and_one_on_it_is_not() {
    setup();
    assert!(is_busted("core-001"));
    assert!(!is_busted("core-002"), "on the threshold is not over it");
    assert!(
        is_complicated("core-002"),
        "a card past 60 lines is complicated, BUSTED or not"
    );
    assert!(is_complicated("core-003"));
    assert!(!is_complicated("core-004"), "on the threshold is not over it");
    assert!(!is_busted("core-005") && !is_complicated("core-005"));
    assert_eq!(code_lines_of("core-003"), Some(COMPLICATED_CODE_LINES + 1));
    // A definition the metrics do not list (a fused or crafted one, a fixture) has no count.
    assert_eq!(code_lines_of("t-1:core-001+core-002"), None);
    assert!(!is_busted("t-1:core-001+core-002"));
    clear_overrides();
}

#[test]
fn r1846000_the_query_draws_the_busted_and_complicated_pools_from_the_sets_that_ship() {
    setup();
    // A pool that names no set draws from the sets that ship (R380, R1420), so the single-player sets
    // and the held set are out, and a token only joins a pool that takes tokens (§5.1).
    assert_eq!(pool(json!({ "busted": true })), vec!["core-001"]);
    assert_eq!(
        pool(json!({ "busted": true, "withTokens": true })),
        vec!["core-001", "core-005-1"]
    );
    assert_eq!(
        pool(json!({ "complicated": true })),
        vec!["core-001", "core-002", "core-003"]
    );
    // `false` forbids them, as `embiggen` and `tribute` do.
    assert_eq!(
        pool(json!({ "busted": false })),
        vec!["core-002", "core-003", "core-004", "core-005"]
    );
    assert_eq!(
        pool(json!({ "complicated": false })),
        vec!["core-004", "core-005"]
    );
    // Unasked, they filter nothing.
    assert_eq!(pool(json!({})).len(), 5);
    // A pool that names a set keeps to it, shipped or not.
    assert_eq!(pool(json!({ "busted": true, "set": "Boss" })), vec!["boss-001"]);
    clear_overrides();
}

#[test]
fn r1846001_every_set_adds_the_single_player_sets_and_nothing_held() {
    setup();
    assert_eq!(SINGLE_PLAYER_SETS, [SetName::Boss, SetName::BossX]);
    assert!(SINGLE_PLAYER_SETS.iter().all(|set| !set_ships(*set)));
    // "Every BUSTED card the catalog holds, the single-player sets' included": the sets that ship,
    // then Boss and Boss-X, in `query`'s order. A set held before it ships stays out (R1420).
    assert_eq!(
        pool(json!({ "busted": true, "everySet": true })),
        vec!["core-001", "boss-001", "bossx-001"]
    );
    assert_eq!(
        pool(json!({ "everySet": true })),
        vec![
            "core-001",
            "core-002",
            "core-003",
            "core-004",
            "core-005",
            "boss-001",
            "bossx-001"
        ]
    );
    // `false` is the default: the sets that ship.
    assert_eq!(
        pool(json!({ "busted": true, "everySet": false })),
        pool(json!({ "busted": true }))
    );
    // A pool that names its set or its cards keeps to them.
    assert_eq!(
        pool(json!({ "busted": true, "everySet": true, "set": "Core" })),
        vec!["core-001"]
    );
    assert_eq!(
        pool(json!({ "everySet": true, "defId": ["held-001"] })),
        vec!["held-001"]
    );
    // Tokens keep §5.1's rule.
    assert_eq!(
        pool(json!({ "busted": true, "everySet": true, "withTokens": true })),
        vec!["core-001", "core-005-1", "boss-001", "bossx-001"]
    );
    // A held set a thread previews is open, so it joins with or without `everySet` (R1420).
    let _preview = preview_sets(&[SetName::Custom]);
    assert_eq!(
        pool(json!({ "busted": true, "everySet": true })),
        vec!["core-001", "boss-001", "bossx-001", "held-001"]
    );
    clear_overrides();
}

#[test]
fn r1846002_a_custom_card_counts_as_neither_whatever_a_table_lists() {
    setup();
    // Even a table listing a custom id (the build's never does) leaves a custom card neither, uncounted.
    let mut lines = code_lines();
    lines.push(("custom-long", BUSTED_CODE_LINES * 2));
    register_code_lines(lines);
    assert!(jackioh_engine::custom::is_custom_id("custom-long"));
    assert_eq!(code_lines_of("custom-long"), None);
    assert!(!is_busted("custom-long"));
    assert!(!is_complicated("custom-long"));
    clear_overrides();
}

/// R2173000: a fold under a shipped version that kept its own lists reads them, not the build's, and
/// a fold under one that kept none reads the build's (here the testkit's), as the build itself does.
mod r2173000_a_fold_reads_its_versions_own_lists {
    use super::*;

    #[test]
    fn r2173000_a_version_with_lists_reads_them_and_one_without_the_builds() {
        setup();
        // The version shipped with core-005 BUSTED and core-004 complicated: the build counts them short.
        let kept = CardLists {
            busted: vec!["core-005"],
            complicated: vec!["core-005", "core-004"],
        };
        register_shipped(vec![
            ShippedCatalog::new("v-lists", defs(), IndexMap::new()).with_card_lists(kept),
            ShippedCatalog::new("v-none", defs(), IndexMap::new()),
        ]);
        let with_lists = shipped_catalog("v-lists").expect("registered");
        let without = shipped_catalog("v-none").expect("registered");

        with_shipped(Some(with_lists), || {
            assert_eq!(pool(json!({ "busted": true })), vec!["core-005"]);
            assert_eq!(pool(json!({ "complicated": true })), vec!["core-004", "core-005"]);
            assert!(is_busted("core-005") && !is_busted("core-001"));
            assert!(is_complicated("core-004") && !is_complicated("core-003"));
            // The code lines stay the build's: no card reads a count.
            assert_eq!(code_lines_of("core-001"), Some(BUSTED_CODE_LINES + 1));
            assert_eq!(card_metrics().busted, ["core-005"]);
        });
        let build = card_metrics();
        assert_eq!(with_shipped(Some(without), card_metrics), build);
        with_shipped(Some(without), || {
            assert_eq!(pool(json!({ "busted": true })), vec!["core-001"]);
        });
        assert_eq!(pool(json!({ "busted": true })), vec!["core-001"]);
        clear_overrides();
    }
}
