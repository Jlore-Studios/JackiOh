//! R3538000–R3538002, R3538006, R3538007 (issue #3538, Stinkyscape+ outside data): the winrate of
//! record and its 50% default, the tier of record and the pool that reads it, past versions (their
//! ids, their minting into the state, their scripts and their pools), the max-mana verb and the
//! winrate scope filter. Fixture `fx-` cards only (CLAUDE.md: the engine does not depend on
//! `crates/cards`); the real tables and catalogs are proved in `crates/cards/src/history.rs`.

use jackioh_engine::effects::{change_max_mana, destroy_all};
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{new_game, put, run_effect, setup_catalog, slot};

fn table(cards: Vec<(&str, i32, i32)>) -> WinRateTable {
    WinRateTable {
        patch: "v0.3.X".to_string(),
        source: WinRateSource::Provisional,
        cards: cards
            .into_iter()
            .map(|(id, wins, games)| WinRateRow {
                id: id.to_string(),
                wins,
                games,
            })
            .collect(),
    }
}

/// A past version of `fx-1` with `attack`/`health` doubled on each face, oldest first as pushed.
fn past_fx_1(version: &str, attack: i32) -> CardDef {
    let mut def = registered_catalog().get("fx-1").expect("fx-1").clone();
    def.id = format!("fx-1@{version}");
    def.base.attack = Some(attack);
    def.base.health = Some(attack);
    def.radiant.attack = Some(attack * 2);
    def.radiant.health = Some(attack * 2);
    def
}

fn pool_ids(args: Value) -> Vec<String> {
    query(&json_as::<CatalogQueryArgs>(args))
        .into_iter()
        .map(|def| def.id.clone())
        .collect()
}

#[test]
fn r3538000_a_row_under_the_floor_is_no_record_and_none_counts_as_half() {
    setup_catalog();
    register_win_rates(table(vec![("fx-1", 30, 40), ("fx-2", 0, 19)]));
    assert_eq!(win_rate_of_record("fx-1"), Some((30, 40)));
    assert_eq!(win_rate_of_record("fx-2"), None, "19 games is under the floor");
    assert_eq!(win_rate_of_record("fx-3"), None, "no row is no record");
    // No record counts as 50%: half is lower than 30/40, tied with half, and above nothing.
    assert!(lower_win_rate("fx-2", "fx-1"));
    assert!(lower_win_rate("fx-3", "fx-1"));
    assert!(
        !lower_win_rate("fx-2", "fx-3"),
        "half is not strictly lower than half"
    );
    assert!(!lower_win_rate("fx-1", "fx-2"));
    assert!(!lower_win_rate("fx-1", "fx-1"));
}

#[test]
fn r3538001_tier_of_record_follows_r3022000s_band_and_the_tier_query_reads_it() {
    setup_catalog();
    register_win_rates(table(vec![
        ("fx-1", 800, 1000),
        ("fx-2", 12, 20),
        ("fx-3", 510, 1000),
        ("fx-4", 480, 1000),
        ("fx-5", 8, 20),
        ("fx-6", 200, 1000),
    ]));
    assert_eq!(tier_of_record("fx-1"), Some(CardTier::S));
    assert_eq!(
        tier_of_record("fx-2"),
        Some(CardTier::A),
        "over the band, range reaches even"
    );
    assert_eq!(tier_of_record("fx-3"), Some(CardTier::B));
    assert_eq!(tier_of_record("fx-4"), Some(CardTier::C));
    assert_eq!(
        tier_of_record("fx-5"),
        Some(CardTier::D),
        "under the band, range reaches even"
    );
    assert_eq!(tier_of_record("fx-6"), Some(CardTier::F));
    assert_eq!(tier_of_record("fx-7"), None, "no record is no tier");
    assert_eq!(pool_ids(json!({ "tier": "S" })), vec!["fx-1".to_string()]);
    assert_eq!(
        pool_ids(json!({ "tier": ["A", "F"] })),
        vec!["fx-2".to_string(), "fx-6".to_string()]
    );
}

#[test]
fn r3538002_a_past_id_resolves_mints_into_the_state_and_runs_its_cards_script() {
    setup_catalog();
    register_past_defs(vec![past_fx_1("v1", 9)]);
    assert_eq!(split_past_id("fx-1@v1"), Some(("fx-1", "v1")));
    assert_eq!(split_past_id("fx-1"), None);
    assert_eq!(past_card_of("fx-1@v1"), "fx-1");
    assert_eq!(past_card_of("fx-1"), "fx-1");
    assert_eq!(past_ids_of("fx-1"), vec!["fx-1@v1".to_string()]);
    assert!(
        past_ids_of("fx-1@v1").is_empty(),
        "only the versions before its own"
    );
    assert_eq!(past_ids_of("fx-2"), Vec::<String>::new());
    let past = find_def(None, "fx-1@v1").expect("a past id resolves");
    assert_eq!(past.base.attack, Some(9));

    let mut state = new_game("past-mint", None);
    let card = put(&mut state, "fx-1@v1", slot(P1, Row::Units, 1), json!({}));
    assert_eq!(card.def_id.as_str(), "fx-1@v1");
    let minted = state
        .transient_defs
        .get("fx-1@v1")
        .expect("minted into the state");
    assert_eq!(minted.base.attack, Some(9));

    // No kept script under the override: a past version runs its card's own script.
    register_scripts(IndexMap::from([(
        "fx-1".to_string(),
        CardScripts {
            base: empty_script(),
            radiant: empty_script(),
        },
    )]));
    let past_entry = registered_entry("fx-1@v1").expect("a script");
    let own_entry = registered_entry("fx-1").expect("fx-1's script");
    assert!(std::ptr::eq(past_entry, own_entry));
}

#[test]
fn r3538002_a_past_pool_matches_def_ids_by_card_and_by_past_id() {
    setup_catalog();
    register_past_defs(vec![past_fx_1("v1", 9), past_fx_1("v2", 7)]);
    assert_eq!(
        pool_ids(json!({ "defId": ["fx-1"], "past": "only" })),
        vec!["fx-1@v1".to_string(), "fx-1@v2".to_string()]
    );
    assert_eq!(
        pool_ids(json!({ "defId": ["fx-1@v2"], "past": "only" })),
        vec!["fx-1@v2".to_string()],
        "a past id names only itself"
    );
    let pool = pool_ids(json!({ "past": "also", "excludeDefId": ["fx-1"] }));
    assert!(!pool.contains(&"fx-1".to_string()));
    assert!(
        !pool.contains(&"fx-1@v1".to_string()),
        "a card id keeps its past out too"
    );
    assert!(pool.contains(&"fx-2".to_string()));
}

#[test]
fn r3538002_a_query_without_tier_or_past_is_unchanged() {
    setup_catalog();
    register_past_defs(vec![past_fx_1("v1", 9)]);
    register_win_rates(table(vec![("fx-1", 800, 1000)]));
    let pool = query(&json_as::<CatalogQueryArgs>(json!({})));
    assert!(!pool.is_empty());
    assert!(
        pool.iter().all(|def| split_past_id(&def.id).is_none()),
        "no past version leaks into a plain pool"
    );
    assert!(
        pool.iter().all(|def| registered_catalog().contains_key(&def.id)),
        "every pick is still a registered card"
    );
}

#[test]
fn r3538006_change_max_mana_lasts_past_the_refresh_and_takes_current_mana_down() {
    let mut state = new_game("max-mana", None);
    state.players[P1].turns_started = 3;
    jackioh_engine::mana::refresh_mana(&mut state.players[P1]);
    let before = state.players[P1].mana.max;
    assert_eq!(state.players[P1].mana.current, before);
    run_effect(&mut state, change_max_mana(json_as(json!({ "amount": -1 }))), P1);
    assert_eq!(state.players[P1].mana.max, before - 1);
    assert_eq!(state.players[P1].mana.current, before - 1, "a loss comes off current too");
    jackioh_engine::mana::refresh_mana(&mut state.players[P1]);
    assert_eq!(state.players[P1].mana.max, before - 1, "still lowered past the refresh");
    assert_eq!(state.players[P1].mana.current, before - 1);
    run_effect(&mut state, change_max_mana(json_as(json!({ "amount": 2 }))), P1);
    assert_eq!(state.players[P1].mana.max, before + 1);
    assert_eq!(state.players[P1].mana.current, before - 1, "a gain adds no current mana");
}

#[test]
fn r3538007_win_rate_below_keeps_only_strictly_lower_cards() {
    setup_catalog();
    register_win_rates(table(vec![("fx-1", 800, 1000), ("fx-2", 200, 1000)]));
    let mut state = new_game("win-rate-below", None);
    let low = put(&mut state, "fx-2", slot(P1, Row::Units, 1), json!({}));
    let even = put(&mut state, "fx-3", slot(P1, Row::Units, 2), json!({}));
    let high = put(&mut state, "fx-1", slot(P2, Row::Units, 1), json!({}));
    // The bar is fx-3, which holds no row and counts as half: 20% goes, half and 80% stay.
    run_effect(
        &mut state,
        destroy_all(json_as(
            json!({ "side": "any", "rows": ["units"], "winRateBelow": "fx-3" }),
        )),
        P1,
    );
    let marked =
        |state: &GameState, id: &str| find_instance(state, id).and_then(|card| card.marked_destroyed);
    assert_eq!(marked(&state, &low.id), Some(true));
    assert_eq!(marked(&state, &even.id), None, "half is not strictly lower than half");
    assert_eq!(marked(&state, &high.id), None);

    // Against a record bar both directions read true: 20% and half go below 80%, and 80% stays.
    let mut state = new_game("win-rate-below-high-bar", None);
    let low = put(&mut state, "fx-2", slot(P1, Row::Units, 1), json!({}));
    let even = put(&mut state, "fx-3", slot(P1, Row::Units, 2), json!({}));
    let high = put(&mut state, "fx-1", slot(P2, Row::Units, 1), json!({}));
    run_effect(
        &mut state,
        destroy_all(json_as(json!({ "side": "any", "rows": ["units"], "winRateBelow": "fx-1" }))),
        P1,
    );
    assert_eq!(marked(&state, &low.id), Some(true));
    assert_eq!(marked(&state, &even.id), Some(true), "half is strictly lower than 80%");
    assert_eq!(marked(&state, &high.id), None);
}
