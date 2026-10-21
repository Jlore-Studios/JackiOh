//! SPEC §14.10, R1300000: the dev accounts and their tools (`src/api/dev.rs`). `DEV_PROFILE_IDS`
//! is parsed at boot and a malformed id fails it; `/api/auth/me` says `isDev`; every dev route
//! answers anyone the list does not name 403 and writes nothing; each give, fill and clear writes
//! ledger rows marked `dev`, and a clear removes what it names and nothing else.
//!
//! The whole router over the fake store, from two accounts seeded active (`DEV`, listed, in upper
//! case, and `OTHER`, not), each with nothing until a test grants it. Expectations are computed
//! from the real catalog and the shipped cosmetics registry.

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::{Value, json};

use jackioh_engine::config::MAX_COPIES;
use jackioh_engine::set_ships;
use jackioh_server::app::{self, App, now_ms};
use jackioh_server::config::{CARD_TOKEN_BAR_SIZE, SET_TRACK_FIRST_LEVEL, SET_TRACK_LEVELS};
use jackioh_server::cosmetics::{CosmeticKind, shipped};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{CollectionEntry, CollectionGrant, Db};
use jackioh_server::economy::math::total_xp_for_level;
use jackioh_server::economy::store::{
    Counter, CounterEntry, CounterGrant, EconomyLedger, EconomyReason, InventoryItem, JloreReason,
    OwnedCosmetics, SetTrack,
};
use jackioh_server::env::{SERVER_ONLY_ENV_VARS, load_env, parse_dev_profile_ids};

use crate::support::deps::{TestAppOptions, add_user, call, test_app_with};

/// The dev account `DEV_PROFILE_IDS` lists.
const DEV: &str = "d0000000-0000-4000-8000-000000001300";
/// An active account it does not.
const OTHER: &str = "d0000000-0000-4000-8000-000000001301";

// ---------------------------------------------------------------------------------------------
// Plumbing (private copies: each test file of this binary keeps its own)
// ---------------------------------------------------------------------------------------------

/// A server environment that `load_env` takes, with `extra` on top (`env_github_issues.rs`'s).
fn env_with(extra: &[(&str, &str)]) -> IndexMap<String, String> {
    [
        ("SUPABASE_URL", "https://project.supabase.test"),
        (
            "SUPABASE_SECRET_KEY",
            "sb_secret_0123456789abcdefghijklmnopqrstuv",
        ),
        (
            "DATABASE_URL",
            "postgres://postgres:postgres@localhost:5432/jackioh",
        ),
        ("CODE_PEPPER", "a-pepper-of-at-least-thirty-two-characters"),
        ("CATALOG_VERSION", jackioh_cards::catalog_version()),
        ("PUBLIC_ORIGINS", "https://play.jackioh.test"),
        ("NODE_ENV", "test"),
    ]
    .into_iter()
    .chain(extra.iter().copied())
    .map(|(name, value)| (name.to_string(), value.to_string()))
    .collect()
}

fn fake(app: &App) -> Arc<tokio::sync::Mutex<FakeData>> {
    match &app.db {
        Db::Fake(data) => Arc::clone(data),
        _ => panic!("the test app runs on the fake store"),
    }
}

/// The app with `DEV` listed (in upper case, which the list reads as the same id) and both accounts
/// seeded active; their tokens, dev's first.
async fn dev_app() -> (Arc<App>, String, String) {
    let app = test_app_with(TestAppOptions {
        skip_fixtures: true,
        env: vec![("DEV_PROFILE_IDS".to_string(), DEV.to_uppercase())],
        ..TestAppOptions::default()
    })
    .await;
    let dev = account(&app, DEV, 1).await;
    let other = account(&app, OTHER, 2).await;
    (app, dev, other)
}

/// `id`, seeded active with a username, and its token.
async fn account(app: &Arc<App>, id: &str, n: u32) -> String {
    let user_id = format!("user-{n}");
    fake(app).lock().await.seed_profile(json!({
        "id": id,
        "userId": user_id,
        "status": "active",
        "username": format!("Dev{n}"),
        "usernamePrompted": true,
    }));
    add_user(app, &user_id, &format!("{user_id}@example.test"), true)
}

async fn api(app: &Arc<App>, method: &str, path: &str, token: &str, body: Value) -> (u16, Value) {
    let (status, _headers, body) = call(app, method, path, Some(token), body).await;
    (status, body)
}

/// `api`, which must answer 200.
async fn ok(app: &Arc<App>, path: &str, token: &str, body: Value) -> Value {
    let (status, answer) = api(app, "POST", path, token, body).await;
    assert_eq!(status, 200, "{path}: {answer}");
    answer
}

/// Everything §14.10's clears name, as the store holds it for `profile_id`: the cards owned (above
/// 0), the cosmetics, the Jlore, the counters above 0 and the tracks off their start.
#[derive(Clone, Debug, PartialEq)]
struct Holdings {
    cards: Vec<CollectionEntry>,
    cosmetics: OwnedCosmetics,
    jlore: i64,
    counters: Vec<CounterEntry>,
    tracks: Vec<SetTrack>,
}

impl Holdings {
    fn cards_empty(&self) -> bool {
        self.cards.is_empty()
    }
    fn cosmetics_empty(&self) -> bool {
        self.cosmetics.cards.is_empty() && self.cosmetics.account.is_empty()
    }
    fn jlore_empty(&self) -> bool {
        self.jlore == 0
    }
    fn counters_empty(&self) -> bool {
        self.counters.is_empty()
    }
    fn tracks_empty(&self) -> bool {
        self.tracks.is_empty()
    }
}

async fn holdings(app: &App, profile_id: &str) -> Holdings {
    let mut tx = app.db.begin(Some(profile_id)).await.expect("begin");
    let mut cards = tx.collection_get(profile_id).await.expect("the collection");
    cards.retain(|entry| entry.quantity > 0);
    cards.sort_by(|a, b| a.card_id.cmp(&b.card_id));
    let cosmetics = tx.economy_cosmetics_get(profile_id).await.expect("the cosmetics");
    let jlore = tx.economy_jlore_get(profile_id).await.expect("the wallet");
    let mut counters = tx.economy_counters_get(profile_id).await.expect("the counters");
    counters.retain(|entry| entry.value > 0);
    let mut tracks = tx.economy_tracks_get(profile_id).await.expect("the tracks");
    tracks.retain(|track| (track.xp, track.level, track.rewards_claimed) != (0, SET_TRACK_FIRST_LEVEL, 0));
    tx.commit().await.expect("commit");
    Holdings {
        cards,
        cosmetics,
        jlore,
        counters,
        tracks,
    }
}

async fn ledger(app: &App, profile_id: &str) -> EconomyLedger {
    let mut tx = app.db.begin(Some(profile_id)).await.expect("begin");
    let ledger = tx.economy_ledger_get(profile_id).await.expect("the ledger");
    tx.commit().await.expect("commit");
    ledger
}

async fn card_grants(app: &App, profile_id: &str) -> Vec<CollectionGrant> {
    fake(app).lock().await.grants_for(profile_id)
}

/// Every economy ledger row past `before`'s, table by table: `(table, reason is Dev, dev mark)`.
fn new_marks(before: &EconomyLedger, after: &EconomyLedger) -> Vec<(&'static str, bool, bool)> {
    let mut marks = Vec::new();
    let mut push = |table, rows: Vec<(bool, bool)>| {
        marks.extend(rows.into_iter().map(|(dev_reason, dev)| (table, dev_reason, dev)))
    };
    push(
        "jlore",
        after.jlore[before.jlore.len()..]
            .iter()
            .map(|row| (row.reason == JloreReason::Dev, row.dev))
            .collect(),
    );
    push(
        "inventory",
        after.inventory[before.inventory.len()..]
            .iter()
            .map(|row| (row.reason == EconomyReason::Dev, row.dev))
            .collect(),
    );
    push(
        "counters",
        after.counters[before.counters.len()..]
            .iter()
            .map(|row| (row.reason == EconomyReason::Dev, row.dev))
            .collect(),
    );
    push(
        "tracks",
        after.tracks[before.tracks.len()..]
            .iter()
            .map(|row| (row.reason == EconomyReason::Dev, row.dev))
            .collect(),
    );
    push(
        "quests",
        after.quests[before.quests.len()..]
            .iter()
            .map(|row| (row.reason == EconomyReason::Dev, row.dev))
            .collect(),
    );
    push(
        "cosmetics",
        after.cosmetics[before.cosmetics.len()..]
            .iter()
            .map(|row| (row.reason == EconomyReason::Dev, row.dev))
            .collect(),
    );
    marks
}

/// The cards a fill grants: §14's collectable ones (non-token, unbanned, of a set that ships).
fn collectable(app: &App) -> Vec<String> {
    app.catalog
        .card_ids
        .iter()
        .filter(|card_id| {
            !app.catalog.is_token(card_id)
                && !app.catalog.is_banned(card_id)
                && app
                    .catalog
                    .defs
                    .get(*card_id)
                    .is_some_and(|def| set_ships(def.set))
        })
        .cloned()
        .collect()
}

// ---------------------------------------------------------------------------------------------
// The list (§14.10: read at boot, a malformed id fails it)
// ---------------------------------------------------------------------------------------------

#[test]
fn r1300000_unset_or_blank_lists_no_dev_account() {
    assert_eq!(parse_dev_profile_ids(None), Ok(Vec::new()));
    assert_eq!(parse_dev_profile_ids(Some(" , ")), Ok(Vec::new()));
    let env = load_env(&env_with(&[])).expect("a valid environment");
    assert_eq!(env.dev_profile_ids, Vec::<String>::new());
    let env = load_env(&env_with(&[("DEV_PROFILE_IDS", "  ")])).expect("a valid environment");
    assert_eq!(env.dev_profile_ids, Vec::<String>::new());
}

#[test]
fn r1300000_parses_a_comma_separated_list_to_canonical_ids() {
    let listed = format!(
        " {} ,{},, {} ",
        DEV.to_uppercase(),
        OTHER.replace('-', ""),
        "D0000000-0000-4000-8000-000000001302"
    );
    let env = load_env(&env_with(&[("DEV_PROFILE_IDS", &listed)])).expect("a valid environment");
    assert_eq!(
        env.dev_profile_ids,
        vec![
            DEV.to_string(),
            OTHER.to_string(),
            "d0000000-0000-4000-8000-000000001302".to_string()
        ]
    );
    // The ids are account secret IDs: a debug print says how many, never which.
    let printed = format!("{env:?}");
    assert!(!printed.contains(DEV) && !printed.contains(OTHER), "{printed}");
}

#[test]
fn r1300000_a_malformed_id_fails_the_boot_naming_it() {
    let listed = format!("{DEV},not-a-uuid, 1234");
    let refused = load_env(&env_with(&[("DEV_PROFILE_IDS", &listed)])).expect_err("a malformed id");
    let message = refused.to_string();
    assert!(message.contains("DEV_PROFILE_IDS"), "{message}");
    assert!(message.contains("\"not-a-uuid\""), "{message}");
    assert!(message.contains("\"1234\""), "{message}");
    assert!(!message.contains(DEV), "a well-formed id is not named: {message}");

    let mut source = env_with(&[("DEV_PROFILE_IDS", "not-a-uuid")]);
    source.insert("E2E".to_string(), "1".to_string());
    let refused = app::load_server_env(&source).expect_err("the end-to-end server refuses it too");
    let message = format!("{refused:#}");
    assert!(
        message.contains("DEV_PROFILE_IDS") && message.contains("\"not-a-uuid\""),
        "{message}"
    );
}

#[test]
fn dev_profile_ids_is_server_only() {
    assert!(SERVER_ONLY_ENV_VARS.contains(&"DEV_PROFILE_IDS"));
}

// ---------------------------------------------------------------------------------------------
// The server decides (CLAUDE.md rule 7)
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn r1300000_me_says_is_dev_for_a_listed_account_only() {
    let (app, dev, other) = dev_app().await;
    let (status, me) = api(&app, "GET", "/api/auth/me", &dev, Value::Null).await;
    assert_eq!((status, &me["isDev"]), (200, &json!(true)), "{me}");
    let (status, me) = api(&app, "GET", "/api/auth/me", &other, Value::Null).await;
    assert_eq!((status, &me["isDev"]), (200, &json!(false)), "{me}");
}

#[tokio::test]
async fn r1300000_a_non_dev_gets_403_on_every_dev_route_and_nothing_is_written() {
    let (app, _dev, other) = dev_app().await;
    let routes: Vec<(&str, String)> = app::ROUTES
        .iter()
        .filter(|(_, path, _, _)| path.starts_with("/api/dev/"))
        .map(|(method, path, _, _)| {
            let path = path.replace(":part", "everything").replace(":bar", "card");
            (*method, path.replace(":questId", "win-1"))
        })
        .collect();
    assert_eq!(
        routes,
        vec![
            ("POST", "/api/dev/quests".to_string()),
            ("POST", "/api/dev/quests/win-1/complete".to_string()),
            ("POST", "/api/dev/xp".to_string()),
            ("POST", "/api/dev/jlore".to_string()),
            ("POST", "/api/dev/fill".to_string()),
            ("POST", "/api/dev/clear/everything".to_string()),
            ("POST", "/api/dev/bar/card".to_string()),
        ]
    );
    for (method, path) in &routes {
        let body = json!({ "setId": "Core", "xp": 100, "jlore": 50 });
        let (status, answer) = api(&app, method, path, &other, body).await;
        assert_eq!(status, 403, "{method} {path}: {answer}");
        assert_eq!(
            answer["error"]["code"],
            json!("forbidden"),
            "{method} {path}: {answer}"
        );
    }
    assert_eq!(ledger(&app, OTHER).await, EconomyLedger::default());
    assert_eq!(card_grants(&app, OTHER).await, Vec::<CollectionGrant>::new());
}

// ---------------------------------------------------------------------------------------------
// Give
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn r1300000_giving_xp_moves_the_track_through_the_level_curve_marked_dev() {
    let (app, dev, _other) = dev_app().await;
    let to_ten = total_xp_for_level(10);
    let answer = ok(
        &app,
        "/api/dev/xp",
        &dev,
        json!({ "setId": "Core", "xp": to_ten }),
    )
    .await;
    assert_eq!(
        answer["track"],
        json!({ "setId": "Core", "xp": to_ten, "level": 10, "rewardsClaimed": 0 })
    );
    let tracks = ledger(&app, DEV).await.tracks;
    assert_eq!(tracks.len(), 1);
    let row = &tracks[0];
    assert_eq!(
        (
            row.set_id.as_str(),
            row.xp,
            row.levels,
            row.rewards,
            row.reason,
            row.dev
        ),
        ("Core", to_ten, 9, 0, EconomyReason::Dev, true)
    );

    // §14.7: a track has `SET_TRACK_LEVELS` levels, however much XP it holds.
    let past_the_last = total_xp_for_level(SET_TRACK_LEVELS + 1);
    let answer = ok(
        &app,
        "/api/dev/xp",
        &dev,
        json!({ "setId": "Core", "xp": past_the_last }),
    )
    .await;
    assert_eq!(answer["track"]["level"], json!(SET_TRACK_LEVELS));
    assert_eq!(answer["track"]["xp"], json!(to_ten + past_the_last));
    assert_eq!(
        ledger(&app, DEV).await.tracks[1].levels,
        i64::from(SET_TRACK_LEVELS) - 10
    );

    // The server judges what it is sent: a set that does not ship, no set, and XP that is not a
    // positive whole number write nothing.
    for body in [
        json!({ "setId": "Boss", "xp": 100 }),
        json!({ "setId": "Nowhere", "xp": 100 }),
        json!({ "xp": 100 }),
        json!({ "setId": "Core", "xp": 0 }),
        json!({ "setId": "Core", "xp": -5 }),
        json!({ "setId": "Core", "xp": 1.5 }),
        json!({ "setId": "Core" }),
    ] {
        let (status, answer) = api(&app, "POST", "/api/dev/xp", &dev, body.clone()).await;
        assert_eq!(status, 400, "{body}: {answer}");
    }
    assert_eq!(ledger(&app, DEV).await.tracks.len(), 2);
}

#[tokio::test]
async fn r1300000_giving_jlore_adds_it_marked_dev() {
    let (app, dev, _other) = dev_app().await;
    assert_eq!(
        ok(&app, "/api/dev/jlore", &dev, json!({ "jlore": 50 })).await,
        json!({ "jlore": 50 })
    );
    assert_eq!(
        ok(&app, "/api/dev/jlore", &dev, json!({ "jlore": 25 })).await,
        json!({ "jlore": 75 })
    );
    let rows: Vec<(i64, JloreReason, bool)> = ledger(&app, DEV)
        .await
        .jlore
        .iter()
        .map(|row| (row.delta, row.reason, row.dev))
        .collect();
    assert_eq!(
        rows,
        vec![(50, JloreReason::Dev, true), (25, JloreReason::Dev, true)]
    );
    for body in [
        json!({ "jlore": 0 }),
        json!({ "jlore": 2.5 }),
        json!({ "jlore": "50" }),
    ] {
        let (status, answer) = api(&app, "POST", "/api/dev/jlore", &dev, body.clone()).await;
        assert_eq!(status, 400, "{body}: {answer}");
    }
    assert_eq!(holdings(&app, DEV).await.jlore, 75);
}

// ---------------------------------------------------------------------------------------------
// Fill
// ---------------------------------------------------------------------------------------------

#[tokio::test]
async fn r1300000_fill_grants_every_card_and_every_cosmetic_marked_dev() {
    let (app, dev, _other) = dev_app().await;
    let cards = collectable(&app);
    let registry = shipped().expect("the registry parses");
    let per_card = |kind: CosmeticKind| {
        matches!(
            kind,
            CosmeticKind::Foil | CosmeticKind::Finish | CosmeticKind::Border
        )
    };
    let card_cosmetics: Vec<&str> = registry
        .iter()
        .filter(|cosmetic| !cosmetic.default && per_card(cosmetic.kind))
        .map(|cosmetic| cosmetic.id.as_str())
        .collect();
    // R1313000: a full art is owned on the one card it names.
    let full_arts: Vec<(&str, &str)> = registry
        .iter()
        .filter(|cosmetic| cosmetic.kind == CosmeticKind::FullArt)
        .filter_map(|cosmetic| Some((cosmetic.card.as_deref()?, cosmetic.id.as_str())))
        .collect();
    let mut account_cosmetics: Vec<String> = registry
        .iter()
        .filter(|cosmetic| {
            !cosmetic.default && !per_card(cosmetic.kind) && cosmetic.kind != CosmeticKind::FullArt
        })
        .map(|cosmetic| cosmetic.id.clone())
        .collect();
    account_cosmetics.sort();
    assert!(
        !cards.is_empty()
            && !card_cosmetics.is_empty()
            && !full_arts.is_empty()
            && !account_cosmetics.is_empty()
    );
    let on_cards = cards.len() * card_cosmetics.len() + full_arts.len();

    let answer = ok(&app, "/api/dev/fill", &dev, json!({})).await;
    assert_eq!(
        answer,
        json!({
            "cards": cards.len(),
            "cosmetics": on_cards + account_cosmetics.len(),
        })
    );

    let held = holdings(&app, DEV).await;
    let mut expected_cards = cards.clone();
    expected_cards.sort();
    assert_eq!(
        held.cards
            .iter()
            .map(|entry| entry.card_id.clone())
            .collect::<Vec<_>>(),
        expected_cards
    );
    assert!(
        held.cards
            .iter()
            .all(|entry| entry.quantity == i64::from(MAX_COPIES))
    );
    assert_eq!(held.cosmetics.cards.len(), on_cards);
    assert_eq!(held.cosmetics.account, account_cosmetics);
    // No default: every account owns those with no row (§14.2).
    assert!(
        held.cosmetics.cards.iter().all(|owned| {
            card_cosmetics.contains(&owned.cosmetic_id.as_str())
                || full_arts.contains(&(owned.card_id.as_str(), owned.cosmetic_id.as_str()))
        }),
        "a default or unknown cosmetic was granted, or a full art on another card"
    );
    assert!(full_arts.iter().all(|(card, full_art)| {
        held.cosmetics
            .cards
            .iter()
            .any(|owned| owned.card_id == *card && owned.cosmetic_id == *full_art)
    }));

    let grants = card_grants(&app, DEV).await;
    assert_eq!(grants.len(), cards.len());
    assert!(
        grants
            .iter()
            .all(|grant| grant.reason == "dev" && grant.dev && grant.delta == i64::from(MAX_COPIES))
    );
    let after = ledger(&app, DEV).await;
    let marks = new_marks(&EconomyLedger::default(), &after);
    assert_eq!(marks.len(), on_cards + account_cosmetics.len());
    assert!(
        marks
            .iter()
            .all(|(table, dev_reason, dev)| *table == "cosmetics" && *dev_reason && *dev)
    );

    // What is owned is left as it is: a second fill writes nothing.
    assert_eq!(
        ok(&app, "/api/dev/fill", &dev, json!({})).await,
        json!({ "cards": 0, "cosmetics": 0 })
    );
    assert_eq!(card_grants(&app, DEV).await.len(), cards.len());
    assert_eq!(ledger(&app, DEV).await, after);
}

// ---------------------------------------------------------------------------------------------
// Clear
// ---------------------------------------------------------------------------------------------

/// A fresh app whose dev account holds something of every kind a clear names: a fill's cards and
/// cosmetics, Jlore, a track's XP, and a pity counter a pack opening counted (no dev tool sets
/// one).
async fn stocked() -> (Arc<App>, String) {
    let (app, dev, _other) = dev_app().await;
    ok(&app, "/api/dev/fill", &dev, json!({})).await;
    ok(&app, "/api/dev/jlore", &dev, json!({ "jlore": 500 })).await;
    ok(
        &app,
        "/api/dev/xp",
        &dev,
        json!({ "setId": "Core", "xp": total_xp_for_level(10) }),
    )
    .await;
    let mut tx = app.db.begin(Some(DEV)).await.expect("begin");
    tx.economy_counters_grant(&CounterGrant {
        profile_id: DEV.to_string(),
        counter: Counter::PityEpic,
        pack: String::new(),
        delta: 3,
        reason: EconomyReason::PackOpen,
        reference: None,
        dev: false,
        at: now_ms(),
    })
    .await
    .expect("the counter")
    .expect("a count");
    tx.commit().await.expect("commit");
    let held = holdings(&app, DEV).await;
    assert!(
        !held.cards_empty()
            && !held.cosmetics_empty()
            && !held.jlore_empty()
            && !held.counters_empty()
            && !held.tracks_empty(),
        "{held:?}"
    );
    (app, dev)
}

#[tokio::test]
async fn r1300000_each_clear_removes_only_what_it_names() {
    type Empty = fn(&Holdings) -> bool;
    let parts: [(&str, Empty); 5] = [
        ("cards", Holdings::cards_empty),
        ("cosmetics", Holdings::cosmetics_empty),
        ("jlore", Holdings::jlore_empty),
        ("counters", Holdings::counters_empty),
        ("tracks", Holdings::tracks_empty),
    ];
    for (part, emptied) in parts {
        let (app, dev) = stocked().await;
        let before = holdings(&app, DEV).await;
        let ledger_before = ledger(&app, DEV).await;
        let grants_before = card_grants(&app, DEV).await.len();

        let answer = ok(&app, &format!("/api/dev/clear/{part}"), &dev, json!({})).await;
        let after = holdings(&app, DEV).await;
        assert!(emptied(&after), "{part}: {after:?}");
        let named = |name: &str| name == part;
        assert!(named("cards") || after.cards == before.cards, "{part}: the cards");
        assert!(
            named("cosmetics") || after.cosmetics == before.cosmetics,
            "{part}: the cosmetics"
        );
        assert!(named("jlore") || after.jlore == before.jlore, "{part}: the Jlore");
        assert!(
            named("counters") || after.counters == before.counters,
            "{part}: the counters"
        );
        assert!(
            named("tracks") || after.tracks == before.tracks,
            "{part}: the tracks"
        );

        // Every row the clear wrote is a dev row, and it says how many it wrote.
        let grants = card_grants(&app, DEV).await;
        let new_grants = &grants[grants_before..];
        assert!(
            new_grants.iter().all(|grant| grant.reason == "dev" && grant.dev),
            "{part}"
        );
        let marks = new_marks(&ledger_before, &ledger(&app, DEV).await);
        assert!(
            marks.iter().all(|(_, dev_reason, dev)| *dev_reason && *dev),
            "{part}: {marks:?}"
        );
        let written = new_grants.len() + marks.len();
        assert!(written > 0, "{part}");
        assert_eq!(answer, json!({ "written": written }), "{part}");

        // A second clear finds nothing left to clear.
        assert_eq!(
            ok(&app, &format!("/api/dev/clear/{part}"), &dev, json!({})).await,
            json!({ "written": 0 }),
            "{part}"
        );
        if part == "cards" {
            let (status, collection) = api(&app, "GET", "/api/collection", &dev, Value::Null).await;
            assert_eq!(
                (status, &collection["entries"]),
                (200, &json!([])),
                "{collection}"
            );
        }
    }
}

#[tokio::test]
async fn r1300000_clearing_everything_empties_all_five_and_a_bogus_part_is_no_clear() {
    let (app, dev) = stocked().await;
    let (status, answer) = api(&app, "POST", "/api/dev/clear/bogus", &dev, json!({})).await;
    assert_eq!(status, 404, "{answer}");
    assert_eq!(answer["error"]["code"], json!("not_found"));

    let ledger_before = ledger(&app, DEV).await;
    ok(&app, "/api/dev/clear/everything", &dev, json!({})).await;
    let after = holdings(&app, DEV).await;
    assert!(
        after.cards_empty()
            && after.cosmetics_empty()
            && after.jlore_empty()
            && after.counters_empty()
            && after.tracks_empty(),
        "{after:?}"
    );
    let marks = new_marks(&ledger_before, &ledger(&app, DEV).await);
    for table in ["jlore", "counters", "tracks", "cosmetics"] {
        assert!(
            marks.iter().any(|(written, _, _)| *written == table),
            "{table}: {marks:?}"
        );
    }
    assert!(marks.iter().all(|(_, dev_reason, dev)| *dev_reason && *dev));
    // Its tracks are back at their start: the first level, no XP, nothing paid (§14.7).
    let mut tx = app.db.begin(Some(DEV)).await.expect("begin");
    let tracks = tx.economy_tracks_get(DEV).await.expect("the tracks");
    tx.commit().await.expect("commit");
    assert_eq!(
        tracks,
        vec![SetTrack {
            set_id: "Core".to_string(),
            xp: 0,
            level: SET_TRACK_FIRST_LEVEL,
            rewards_claimed: 0,
        }]
    );
}

// ---------------------------------------------------------------------------------------------
// A bar filled (R1310003)
// ---------------------------------------------------------------------------------------------

/// How many of `item` under `key` `profile_id` holds.
async fn held(app: &App, profile_id: &str, item: InventoryItem, key: &str) -> i64 {
    let mut tx = app.db.begin(Some(profile_id)).await.expect("begin");
    let inventory = tx.economy_inventory_get(profile_id).await.expect("the inventory");
    tx.commit().await.expect("commit");
    inventory
        .iter()
        .filter(|entry| entry.item == item && entry.key == key)
        .map(|entry| entry.count)
        .sum()
}

/// `counter`'s account-wide value for `profile_id`; a counter never written is 0.
async fn counter_of(app: &App, profile_id: &str, counter: Counter) -> i64 {
    let mut tx = app.db.begin(Some(profile_id)).await.expect("begin");
    let counters = tx.economy_counters_get(profile_id).await.expect("the counters");
    tx.commit().await.expect("commit");
    counters
        .iter()
        .find(|entry| entry.counter == counter && entry.pack.is_empty())
        .map_or(0, |entry| entry.value)
}

#[tokio::test]
async fn r1310003_filling_a_bar_pays_its_token_marked_dev() {
    let (app, dev, _other) = dev_app().await;

    // The card bar, part way to its size as a pack's duplicates leave it: the fill tops it up, it
    // pays one card token and keeps nothing.
    let mut tx = app.db.begin(Some(DEV)).await.expect("begin");
    tx.economy_counters_grant(&CounterGrant {
        profile_id: DEV.to_string(),
        counter: Counter::CardTokenBar,
        pack: String::new(),
        delta: CARD_TOKEN_BAR_SIZE / 2,
        reason: EconomyReason::PackOpen,
        reference: None,
        dev: false,
        at: now_ms(),
    })
    .await
    .expect("the counter")
    .expect("a count");
    tx.commit().await.expect("commit");
    let before = ledger(&app, DEV).await;
    let answer = ok(&app, "/api/dev/bar/card", &dev, json!({})).await;
    assert_eq!(answer, json!({ "written": 2 }));
    assert_eq!(held(&app, DEV, InventoryItem::CardToken, "").await, 1);
    assert_eq!(counter_of(&app, DEV, Counter::CardTokenBar).await, 0);
    let marks = new_marks(&before, &ledger(&app, DEV).await);
    assert_eq!(marks, vec![("inventory", true, true), ("counters", true, true)]);

    // An empty bar fills whole and pays again; its points need no row.
    ok(&app, "/api/dev/bar/card", &dev, json!({})).await;
    assert_eq!(held(&app, DEV, InventoryItem::CardToken, "").await, 2);

    // The cosmetic bar pays the cycle's steps in order (§14.5): a Rare token, then an Epic one,
    // the cycle's place moving on with each.
    let before = ledger(&app, DEV).await;
    ok(&app, "/api/dev/bar/cosmetic", &dev, json!({})).await;
    assert_eq!(held(&app, DEV, InventoryItem::CosmeticToken, "Rare").await, 1);
    assert_eq!(held(&app, DEV, InventoryItem::CosmeticToken, "Epic").await, 0);
    assert_eq!(counter_of(&app, DEV, Counter::CosmeticTokenCycle).await, 1);
    ok(&app, "/api/dev/bar/cosmetic", &dev, json!({})).await;
    assert_eq!(held(&app, DEV, InventoryItem::CosmeticToken, "Rare").await, 1);
    assert_eq!(held(&app, DEV, InventoryItem::CosmeticToken, "Epic").await, 1);
    assert_eq!(counter_of(&app, DEV, Counter::CosmeticTokenCycle).await, 2);
    assert_eq!(counter_of(&app, DEV, Counter::CosmeticTokenBar).await, 0);
    let marks = new_marks(&before, &ledger(&app, DEV).await);
    assert_eq!(marks.len(), 4, "{marks:?}");
    assert!(
        marks.iter().all(|(_, dev_reason, dev)| *dev_reason && *dev),
        "{marks:?}"
    );

    // A `:bar` that names no bar is no fill, and writes nothing.
    let before = ledger(&app, DEV).await;
    let (status, answer) = api(&app, "POST", "/api/dev/bar/bogus", &dev, json!({})).await;
    assert_eq!(status, 404, "{answer}");
    assert_eq!(answer["error"]["code"], json!("not_found"));
    assert_eq!(ledger(&app, DEV).await, before);
}
