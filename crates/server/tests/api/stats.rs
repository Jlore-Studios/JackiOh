//! Public card and player statistics page tests (SPEC §9.11, R654).
//!
//! Port of `apps/server/test/api/stats.test.ts` (part 18). Three things differ from TS, each
//! because the Rust app is the whole server rather than a router over test deps:
//!
//! - The catalog is the real one (`jackioh_cards`), where TS swapped in six hand-written defs. The
//!   records play the real `core-001`..`core-004`, and the filter test reads its expectations off
//!   the catalog itself rather than off the six.
//! - The current patch is the build's own (`jackioh_cards::catalog_version()`, what TS's
//!   `loadCurrentPatch` read in production), where TS pinned `v0.2.0` through `deps.games`.
//! - The accounts are the E2E fixtures (`e2e-p1` active, `e2e-pending` pending), and the listed
//!   players are profiles made through the store, so their ids are the store's.
//!
//! TS also wrote five `mode: "tutorial"` records to prove the gate leaves them out. `GameMode` has
//! no such literal in Rust, so no such record can be built or stored; the counts they were excluded
//! from are asserted unchanged.

use std::sync::Arc;

use axum::http::HeaderMap;
use jackioh_server::app::App;
use jackioh_server::config::{
    CARD_STATS_CACHE_TTL_SECONDS, CARD_STATS_MIN_SAMPLE, PLAYER_STATS_BYTES_MAX,
    PLAYER_STATS_CACHE_TTL_SECONDS, PUBLIC_STATS_MIN_LIVE_GAMES, USERNAME_CHANGE_COOLDOWN_MS,
};
use jackioh_server::db::store::{UsernameClaim, UsernameClaimOutcome};
use jackioh_server::username::username_key;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use crate::support::deps::{call, test_app};

/// The active account's bearer token (`e2e-p1`).
const USER_TOKEN: &str = "e2e-token-p1";
/// The pending account's bearer token (`e2e-pending`).
const PENDING_TOKEN: &str = "e2e-token-pending";

/// The patch the server counts as current (TS `PATCH`).
fn current_patch() -> String {
    jackioh_cards::catalog_version().to_string()
}

/// A store value built from TS's own object literal.
fn from<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value).expect("a store value from its JSON")
}

/// A config count as a `usize`, whichever integer type `config.rs` gives it.
fn count<T>(value: T) -> usize
where
    usize: TryFrom<T>,
    <usize as TryFrom<T>>::Error: std::fmt::Debug,
{
    usize::try_from(value).expect("a count fits a usize")
}

/// TS `makeGameRecord`'s options: every one optional.
#[derive(Default)]
struct RecordOptions {
    source: Option<&'static str>,
    mode: Option<&'static str>,
    patch: Option<String>,
    p1_deck: Option<Vec<&'static str>>,
    p2_deck: Option<Vec<&'static str>>,
    winner: Option<&'static str>,
    p1_played: Option<Vec<&'static str>>,
    p2_played: Option<Vec<&'static str>>,
    p1_played_turns: Option<Vec<usize>>,
    p2_played_turns: Option<Vec<usize>>,
    p1_played_radiant: Option<Vec<bool>>,
    p2_played_radiant: Option<Vec<bool>>,
    turns: Option<i32>,
}

/// One seat's summary: the first three cards are the opening hand, the rest drawn.
fn seat(
    deck: &[&str],
    played: &[&str],
    played_turns: Option<Vec<usize>>,
    played_radiant: Option<Vec<bool>>,
) -> Value {
    let mut val = json!({
        "deck": deck,
        "opening": deck.iter().take(3).collect::<Vec<_>>(),
        "drawn": deck.iter().skip(3).collect::<Vec<_>>(),
        "played": played,
        "playedTurns": played_turns.unwrap_or_else(|| (1..=played.len()).collect()),
    });
    if let Some(flags) = played_radiant {
        val["playedRadiant"] = json!(flags);
    }
    val
}

/// A `GameRecord`, as the JSON the store takes it from.
fn make_game_record(id: &str, options: RecordOptions) -> Value {
    let p1_deck = options.p1_deck.unwrap_or_else(|| vec!["core-001", "core-002"]);
    let p2_deck = options.p2_deck.unwrap_or_else(|| vec!["core-003", "core-004"]);
    let source = options.source.unwrap_or("live");
    let patch = options.patch.unwrap_or_else(current_patch);
    let record_id = if source == "dev" {
        if id.starts_with("dev:") {
            id.to_string()
        } else {
            format!("dev:{patch}:{id}")
        }
    } else {
        id.to_string()
    };

    let p1_played = options
        .p1_played
        .unwrap_or_else(|| p1_deck.iter().take(2).copied().collect());
    let p2_played = options
        .p2_played
        .unwrap_or_else(|| p2_deck.iter().take(2).copied().collect());
    let pilot = if source == "dev" { "ai" } else { "human" };

    json!({
        "id": record_id,
        "source": source,
        "mode": options.mode.unwrap_or("bo1"),
        "patch": patch,
        "pilots": { "p1": pilot, "p2": pilot },
        "game": {
            "first": "p1",
            "winner": options.winner.unwrap_or("p1"),
            "reason": "hero-death",
            "turns": options.turns.unwrap_or(5),
            "seats": {
                "p1": seat(&p1_deck, &p1_played, options.p1_played_turns, options.p1_played_radiant),
                "p2": seat(&p2_deck, &p2_played, options.p2_played_turns, options.p2_played_radiant),
            }
        }
    })
}

/// `gameRecords.insert` for every record, in one transaction.
async fn insert_records(app: &App, records: Vec<Value>) {
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    for record in records {
        tx.game_records_insert(&from(record))
            .await
            .expect("gameRecords.insert");
    }
    tx.commit().await.expect("commit");
}

/// A new active profile that picked `username` at time 0, as TS's `seedProfile`; answers its id.
async fn seed_profile(app: &App, user_id: &str, username: &str) -> String {
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    let profile = tx
        .profiles_create(&from(json!({
            "userId": user_id,
            "email": format!("{user_id}@example.test"),
            "rating": 1000,
            "at": 0,
        })))
        .await
        .expect("profiles.create");
    tx.profiles_set_status(&profile.id, from(json!("active")))
        .await
        .expect("profiles.setStatus");
    tx.commit().await.expect("commit");
    claim_username(app, &profile.id, username, 0).await;
    profile.id
}

/// R1435: `profile_id` claims the bare base `base` at `at`, through the store as a save does.
async fn claim_username(app: &App, profile_id: &str, base: &str, at: i64) {
    let mut tx = app.db.begin(Some(profile_id)).await.expect("a store transaction");
    let claimed = tx
        .profiles_claim_username(&UsernameClaim {
            profile_id: profile_id.to_string(),
            base: base.to_string(),
            key: username_key(base),
            expected_tag: None,
            at,
            cooldown_ms: USERNAME_CHANGE_COOLDOWN_MS,
        })
        .await
        .expect("profiles.claimUsername");
    assert_eq!(claimed, UsernameClaimOutcome::Claimed { tag: None });
    tx.commit().await.expect("commit");
}

/// `playerStats.put(profileId, stats, isPrivate, at)`.
async fn put_player_stats(app: &App, profile_id: &str, stats: Value, is_private: bool, at: i64) {
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    tx.player_stats_put(profile_id, &from(stats), is_private, at)
        .await
        .expect("playerStats.put");
    tx.commit().await.expect("commit");
}

/// One request through the whole app (TS `router(jsonRequest(method, path, body, { token }))`).
async fn request(
    app: &Arc<App>,
    method: &str,
    path: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (u16, HeaderMap, Value) {
    call(app, method, path, token, body.unwrap_or(Value::Null)).await
}

/// `GET <path>` with no token, answering the status, headers and body.
async fn get_public(app: &Arc<App>, path: &str) -> (u16, HeaderMap, Value) {
    request(app, "GET", path, None, None).await
}

fn cache_control(headers: &HeaderMap) -> Option<&str> {
    headers.get("cache-control").and_then(|value| value.to_str().ok())
}

/// The entry of `list` whose `key` is `value`.
fn find<'a>(list: &'a Value, key: &str, value: &Value) -> Option<&'a Value> {
    list.as_array()
        .and_then(|entries| entries.iter().find(|entry| &entry[key] == value))
}

/// The ids of a response's cards, sorted.
fn ids_of(data: &Value) -> Vec<String> {
    let mut ids: Vec<String> = data["cards"]
        .as_array()
        .expect("cards is a list")
        .iter()
        .map(|card| card["id"].as_str().expect("a card id").to_string())
        .collect();
    ids.sort();
    ids
}

/// The catalog's deckable (non-token) cards, by id.
fn deckable() -> Map<String, Value> {
    let catalog: Map<String, Value> =
        serde_json::from_str(jackioh_cards::catalog_json()).expect("catalog.json");
    catalog
        .into_iter()
        .filter(|(_, def)| def["token"] != json!(true) && ships(def))
        .collect()
}

/// R1420: whether an entry's set ships: the server serves, seeds and counts only those.
fn ships(def: &Value) -> bool {
    serde_json::from_value::<jackioh_engine::SetName>(def["set"].clone())
        .map_or(true, jackioh_engine::set_ships)
}

/// The deckable ids `keep` accepts, sorted.
fn deckable_ids(keep: impl Fn(&Value) -> bool) -> Vec<String> {
    let mut ids: Vec<String> = deckable()
        .into_iter()
        .filter(|(_, def)| keep(def))
        .map(|(id, _)| id)
        .collect();
    ids.sort();
    ids
}

/// A def's cost as a number: `X` is 0 and an embiggen cost is its base, as the handler reads it.
fn numeric_cost(def: &Value) -> i64 {
    match &def["cost"] {
        Value::Number(cost) => cost.as_i64().unwrap_or(0),
        Value::Object(cost) => cost.get("base").and_then(Value::as_i64).unwrap_or(0),
        _ => 0,
    }
}

mod r654_public_card_and_player_stats {
    use super::*;

    #[tokio::test]
    async fn r654_get_api_stats_cards_pads_with_ai_development_runs_below_1000_live_games_provisional() {
        let app = test_app().await;
        // 999 live games and 50 dev games
        let mut records = Vec::new();
        for i in 1..=999 {
            records.push(make_game_record(
                &format!("live-{i}"),
                RecordOptions {
                    source: Some("live"),
                    p1_deck: Some(vec!["core-001", "core-002"]),
                    winner: Some(if i % 2 == 0 { "p1" } else { "p2" }),
                    ..RecordOptions::default()
                },
            ));
        }
        for i in 1..=50 {
            records.push(make_game_record(
                &format!("dev-{i}"),
                RecordOptions {
                    source: Some("dev"),
                    p1_deck: Some(vec!["core-001", "core-002"]),
                    winner: Some("p1"),
                    ..RecordOptions::default()
                },
            ));
        }
        // TS's 5 tutorial games (excluded from gate and figures) cannot be stored: see the header.
        insert_records(&app, records).await;

        let (status, headers, data) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(status, 200);
        assert_eq!(
            cache_control(&headers),
            Some(format!("public, max-age={CARD_STATS_CACHE_TTL_SECONDS}").as_str())
        );

        assert_eq!(data["gate"]["cleared"], json!(false));
        assert_eq!(data["gate"]["liveGames"], json!(999));
        assert_eq!(data["gate"]["minLiveGames"], json!(PUBLIC_STATS_MIN_LIVE_GAMES));
        assert_eq!(data["source"], "provisional");
        assert_eq!(data["sourceLabel"], "AI games + live games (provisional)");
        assert_eq!(data["minSample"], json!(CARD_STATS_MIN_SAMPLE));

        // AI games are included below 1000 live games (999 live + 50 dev = 1049 games; tutorial excluded)
        assert_eq!(data["totalGames"], json!(1049));
        let card001 = find(&data["cards"], "id", &json!("core-001")).expect("core-001 is listed");
        assert_eq!(card001["games"], json!(1049));
        assert_eq!(card001["hasEnoughGames"], json!(true));
    }

    #[tokio::test]
    async fn r654_get_api_stats_cards_switches_strictly_to_live_games_only_at_exactly_1000_live_games() {
        let app = test_app().await;
        // Exactly 1000 live games and 100 dev games
        let mut records = Vec::new();
        for i in 1..=1000 {
            records.push(make_game_record(
                &format!("live-{i}"),
                RecordOptions {
                    source: Some("live"),
                    p1_deck: Some(vec!["core-001", "core-002"]),
                    winner: Some("p1"),
                    ..RecordOptions::default()
                },
            ));
        }
        for i in 1..=100 {
            records.push(make_game_record(
                &format!("dev-{i}"),
                RecordOptions {
                    source: Some("dev"),
                    p1_deck: Some(vec!["core-001", "core-002"]),
                    winner: Some("p2"),
                    ..RecordOptions::default()
                },
            ));
        }
        insert_records(&app, records).await;

        let (status, _, data) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(status, 200);

        assert_eq!(data["gate"]["cleared"], json!(true));
        assert_eq!(data["gate"]["liveGames"], json!(1000));
        assert_eq!(data["source"], "live");
        assert_eq!(data["sourceLabel"], "Live games");

        // At/above 1000 live games, AI dev games are strictly ignored (totalGames is exactly 1000 live)
        assert_eq!(data["totalGames"], json!(1000));
        let card001 = find(&data["cards"], "id", &json!("core-001")).expect("core-001 is listed");
        assert_eq!(card001["games"], json!(1000));
        assert_eq!(card001["winRate"].as_f64(), Some(1.0)); // all 1000 live games won by p1
    }

    #[tokio::test]
    async fn r654_respects_sample_floor_card_stats_min_sample_20_for_win_rate_and_best_worst_summary() {
        let app = test_app().await;
        // 19 games with core-001, 25 games with core-002
        let mut records = Vec::new();
        for i in 1..=19 {
            records.push(make_game_record(
                &format!("rec-19-{i}"),
                RecordOptions {
                    source: Some("live"),
                    p1_deck: Some(vec!["core-001"]),
                    winner: Some("p1"),
                    ..RecordOptions::default()
                },
            ));
        }
        for i in 1..=25 {
            records.push(make_game_record(
                &format!("rec-25-{i}"),
                RecordOptions {
                    source: Some("live"),
                    p1_deck: Some(vec!["core-002"]),
                    winner: Some(if i <= 15 { "p1" } else { "p2" }),
                    ..RecordOptions::default()
                },
            ));
        }
        insert_records(&app, records).await;

        let (_, _, data) = get_public(&app, "/api/stats/cards").await;

        let c1 = find(&data["cards"], "id", &json!("core-001")).expect("core-001 is listed");
        let c2 = find(&data["cards"], "id", &json!("core-002")).expect("core-002 is listed");

        assert_eq!(c1["games"], json!(19));
        assert_eq!(c1["hasEnoughGames"], json!(false)); // below 20

        assert_eq!(c2["games"], json!(25));
        assert_eq!(c2["hasEnoughGames"], json!(true)); // >= 20
        assert_eq!(c2["winRate"].as_f64(), Some(15.0 / 25.0));

        // Summary best/worst card only considers cards with hasEnoughGames === true
        // core-001 has 100% win rate (19/19) but is excluded because games < 20
        assert_eq!(data["summary"]["bestCard"]["id"], "core-002");
        assert_eq!(data["summary"]["bestCard"]["winRate"].as_f64(), Some(15.0 / 25.0));
        assert_eq!(
            data["summary"]["worstCard"]["winRate"].as_f64(),
            Some(10.0 / 44.0)
        );
    }

    #[tokio::test]
    async fn r654_filters_cards_by_set_rarity_cost_source_and_card_id() {
        let app = test_app().await;
        let (_, _, all) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(all["cards"].as_array().map_or(0, Vec::len), deckable().len());

        let (_, _, set_cards) = get_public(&app, "/api/stats/cards?set=Core").await;
        assert_eq!(ids_of(&set_cards), deckable_ids(|def| def["set"] == "Core"));

        let (_, _, rarity_cards) = get_public(&app, "/api/stats/cards?rarity=Legendary").await;
        assert_eq!(
            ids_of(&rarity_cards),
            deckable_ids(|def| def["rarity"] == "Legendary")
        );

        let (_, _, cost_cards) = get_public(&app, "/api/stats/cards?cost=3").await;
        assert_eq!(ids_of(&cost_cards), deckable_ids(|def| numeric_cost(def) == 3));

        // Cost 6+ bucket includes every card of cost 6 and above
        let six_and_up = deckable_ids(|def| numeric_cost(def) >= 6);
        let (_, _, cost6_cards) = get_public(&app, "/api/stats/cards?cost=6").await;
        assert_eq!(ids_of(&cost6_cards), six_and_up);

        let (_, _, cost6_plus_cards) = get_public(&app, "/api/stats/cards?cost=6%2B").await;
        assert_eq!(ids_of(&cost6_plus_cards), six_and_up);

        let (_, _, single_card) = get_public(&app, "/api/stats/cards?card=core-001").await;
        assert_eq!(ids_of(&single_card), ["core-001"]);

        // Source filtering
        let (_, _, live_stats) = get_public(&app, "/api/stats/cards?source=live").await;
        assert_eq!(live_stats["source"], "live");
        assert_eq!(live_stats["sourceLabel"], "Live games");

        let (_, _, dev_stats) = get_public(&app, "/api/stats/cards?source=dev").await;
        assert_eq!(dev_stats["source"], "dev");
        assert_eq!(dev_stats["sourceLabel"], "AI development games");

        let (_, _, provisional_stats) = get_public(&app, "/api/stats/cards?source=provisional").await;
        assert_eq!(provisional_stats["source"], "provisional");
        assert_eq!(
            provisional_stats["sourceLabel"],
            "AI games + live games (provisional)"
        );
    }

    #[tokio::test]
    async fn r654_filters_cards_by_each_shipped_set_alone_so_classic_keeps_classic_plus_out() {
        let app = test_app().await;
        for set in jackioh_engine::SHIPPED_SETS {
            let name = set.as_str();
            // `+` is a space in a query unless it is encoded.
            let param = name.to_lowercase().replace('+', "%2B");
            let prefix = format!("{}-", name.to_lowercase().replace('+', "plus"));
            let (status, _, data) = get_public(&app, &format!("/api/stats/cards?set={param}")).await;
            let ids = ids_of(&data);
            assert_eq!(status, 200, "set={param}");
            assert!(!ids.is_empty(), "set={param}");
            assert_eq!(ids, deckable_ids(|def| def["set"] == name), "set={param}");
            assert!(ids.iter().all(|id| id.starts_with(&prefix)), "set={param}");
        }
    }

    #[tokio::test]
    async fn r654_get_api_stats_cards_id_returns_drill_down_with_turn_played_and_co_played_cards() {
        let app = test_app().await;
        let records = (1..=30)
            .map(|i| {
                make_game_record(
                    &format!("rec-drill-{i}"),
                    RecordOptions {
                        source: Some("live"),
                        p1_deck: Some(vec!["core-001", "core-002"]),
                        p1_played: Some(if i % 2 == 0 {
                            vec!["core-002", "core-001"]
                        } else {
                            vec!["core-001"]
                        }),
                        p1_played_turns: Some(if i % 2 == 0 { vec![2, 3] } else { vec![1] }),
                        turns: Some(4),
                        winner: Some(if i <= 20 { "p1" } else { "p2" }),
                        ..RecordOptions::default()
                    },
                )
            })
            .collect();
        insert_records(&app, records).await;

        let (status, headers, drill) = get_public(&app, "/api/stats/cards/core-001").await;
        assert_eq!(status, 200);
        assert_eq!(
            cache_control(&headers),
            Some(format!("public, max-age={CARD_STATS_CACHE_TTL_SECONDS}").as_str())
        );

        assert_eq!(drill["card"]["id"], "core-001");
        assert_eq!(drill["card"]["name"], "Big D-fender");

        // Co-played synergy cards
        let co_played = find(&drill["coPlayed"], "id", &json!("core-002")).expect("core-002 is co-played");
        assert_eq!(co_played["games"], json!(30));

        // Turn played distribution: core-001 was played on turn 1 in 15 games, and turn 3 in 15 games
        let turn1 = find(&drill["byTurn"], "turn", &json!(1)).expect("turn 1 is tallied");
        let turn3 = find(&drill["byTurn"], "turn", &json!(3)).expect("turn 3 is tallied");
        assert_eq!(turn1["games"], json!(15));
        assert_eq!(turn3["games"], json!(15));

        // 404 on missing card
        let (not_found, _, _) = get_public(&app, "/api/stats/cards/nonexistent-card").await;
        assert_eq!(not_found, 404);
    }

    #[tokio::test]
    async fn r377_get_api_stats_cards_publishes_drawn_not_played_counts_and_server_calculated_played_delta() {
        let app = test_app().await;
        // 50 games where p1 holds core-001 and core-002:
        // core-001 is played in 30 games (21 wins, 9 losses -> 70% played win rate).
        // core-001 is drawn but not played in 20 games (10 wins, 10 losses -> 50% baseline).
        // Played delta for core-001 = 0.70 - 0.50 = +0.20 (+20 points).
        // core-002 is never played in any of the 50 games (played cohort is empty -> delta is null).
        let mut records = Vec::new();
        // 21 played games won by p1
        for i in 1..=21 {
            records.push(make_game_record(
                &format!("rec-p-win-{i}"),
                RecordOptions {
                    source: Some("live"),
                    p1_deck: Some(vec!["core-001", "core-002"]),
                    p1_played: Some(vec!["core-001"]),
                    winner: Some("p1"),
                    ..RecordOptions::default()
                },
            ));
        }
        // 9 played games lost by p1
        for i in 1..=9 {
            records.push(make_game_record(
                &format!("rec-p-loss-{i}"),
                RecordOptions {
                    source: Some("live"),
                    p1_deck: Some(vec!["core-001", "core-002"]),
                    p1_played: Some(vec!["core-001"]),
                    winner: Some("p2"),
                    ..RecordOptions::default()
                },
            ));
        }
        // 10 unplayed games won by p1
        for i in 1..=10 {
            records.push(make_game_record(
                &format!("rec-u-win-{i}"),
                RecordOptions {
                    source: Some("live"),
                    p1_deck: Some(vec!["core-001", "core-002"]),
                    p1_played: Some(vec![]),
                    winner: Some("p1"),
                    ..RecordOptions::default()
                },
            ));
        }
        // 10 unplayed games lost by p1
        for i in 1..=10 {
            records.push(make_game_record(
                &format!("rec-u-loss-{i}"),
                RecordOptions {
                    source: Some("live"),
                    p1_deck: Some(vec!["core-001", "core-002"]),
                    p1_played: Some(vec![]),
                    winner: Some("p2"),
                    ..RecordOptions::default()
                },
            ));
        }
        insert_records(&app, records).await;

        let (status, _, data) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(status, 200);

        let card001 = find(&data["cards"], "id", &json!("core-001")).expect("core-001 is listed");
        assert_eq!(card001["games"], json!(50));
        assert_eq!(card001["drawnGames"], json!(50));
        assert_eq!(card001["playedGames"], json!(30));
        assert_eq!(card001["playedWinRate"].as_f64(), Some(0.7));
        assert_eq!(card001["drawnNotPlayedGames"], json!(20));
        assert_eq!(card001["drawnNotPlayedWinRate"].as_f64(), Some(0.5));
        let delta001 = card001["playedDelta"].as_f64().expect("core-001 has playedDelta");
        assert!((delta001 - 0.2).abs() < 1e-9);

        let card002 = find(&data["cards"], "id", &json!("core-002")).expect("core-002 is listed");
        assert_eq!(card002["games"], json!(50));
        assert_eq!(card002["playedGames"], json!(0));
        assert!(card002["playedWinRate"].is_null());
        assert_eq!(card002["drawnNotPlayedGames"], json!(50));
        assert_eq!(card002["drawnNotPlayedWinRate"].as_f64(), Some(31.0 / 50.0));
        assert!(card002["playedDelta"].is_null());
    }

    #[tokio::test]
    async fn r1147000_get_api_stats_cards_splits_the_played_games_by_the_face_the_card_was_played_on() {
        let app = test_app().await;
        let records = vec![
            make_game_record(
                "live-1",
                RecordOptions {
                    source: Some("live"),
                    p1_deck: Some(vec!["core-001", "core-002"]),
                    p1_played: Some(vec!["core-001", "core-002"]),
                    p1_played_radiant: Some(vec![true, false]),
                    winner: Some("p1"),
                    ..RecordOptions::default()
                },
            ),
            make_game_record(
                "live-2",
                RecordOptions {
                    source: Some("live"),
                    p1_deck: Some(vec!["core-001", "core-002"]),
                    p1_played: Some(vec!["core-001", "core-002"]),
                    p1_played_radiant: Some(vec![false, false]),
                    winner: Some("p2"),
                    ..RecordOptions::default()
                },
            ),
        ];
        insert_records(&app, records).await;

        let (status, _, data) = get_public(&app, "/api/stats/cards?source=live").await;
        assert_eq!(status, 200);

        let card001 = find(&data["cards"], "id", &json!("core-001")).expect("core-001 is listed");
        assert_eq!(card001["playedGames"], json!(2));
        assert_eq!(card001["playedRadiantGames"], json!(1));
        assert_eq!(card001["playedRadiantWinRate"].as_f64(), Some(1.0));
        assert_eq!(card001["playedNormalGames"], json!(1));
        assert_eq!(card001["playedNormalWinRate"].as_f64(), Some(0.0));

        let card002 = find(&data["cards"], "id", &json!("core-002")).expect("core-002 is listed");
        assert_eq!(card002["playedGames"], json!(2));
        assert_eq!(card002["playedRadiantGames"], json!(0));
        assert!(card002["playedRadiantWinRate"].is_null());
        assert_eq!(card002["playedNormalGames"], json!(2));
        assert_eq!(card002["playedNormalWinRate"].as_f64(), Some(0.5));
    }

    #[tokio::test]
    async fn r654_get_api_stats_cards_withholds_the_played_delta_below_the_floor_on_either_side() {
        let app = test_app().await;
        // 40 games where p1 holds core-001, core-002 and core-005:
        //  - 20 won, core-001 and core-002 played;
        //  - 19 lost, only core-005 played;
        //  - 1 lost, only core-002 played.
        // core-001: 20 played, 20 drawn but not played: both at the floor, so its delta shows (+100).
        // core-002: 21 played, 19 not: its baseline is one game short, so no delta.
        // core-005: 19 played, 21 not: its played side is one game short, so no delta.
        let deck = vec!["core-001", "core-002", "core-005"];
        let mut records = Vec::new();
        let groups: [(&str, usize, Vec<&str>, &str); 3] = [
            ("both", 20, vec!["core-001", "core-002"], "p1"),
            ("third", 19, vec!["core-005"], "p2"),
            ("second", 1, vec!["core-002"], "p2"),
        ];
        for (name, count, played, winner) in groups {
            for i in 1..=count {
                records.push(make_game_record(
                    &format!("rec-{name}-{i}"),
                    RecordOptions {
                        source: Some("live"),
                        p1_deck: Some(deck.clone()),
                        p1_played: Some(played.clone()),
                        winner: Some(winner),
                        ..RecordOptions::default()
                    },
                ));
            }
        }
        insert_records(&app, records).await;

        let (status, _, data) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(status, 200);

        let card = |id: &str| find(&data["cards"], "id", &json!(id)).expect("the card is listed");
        let card001 = card("core-001");
        assert_eq!(card001["playedGames"], json!(20));
        assert_eq!(card001["drawnNotPlayedGames"], json!(20));
        let delta001 = card001["playedDelta"].as_f64().expect("core-001 has playedDelta");
        assert!((delta001 - 1.0).abs() < 1e-9);

        let card002 = card("core-002");
        assert_eq!(card002["playedGames"], json!(21));
        assert_eq!(card002["drawnNotPlayedGames"], json!(19));
        assert!(card002["drawnNotPlayedWinRate"].as_f64().is_some());
        assert!(card002["playedDelta"].is_null());

        let card005 = card("core-005");
        assert_eq!(card005["playedGames"], json!(19));
        assert_eq!(card005["drawnNotPlayedGames"], json!(21));
        assert!(card005["playedWinRate"].as_f64().is_some());
        assert!(card005["playedDelta"].is_null());
    }

    #[tokio::test]
    async fn r654_get_and_put_api_stats_player_requires_active_auth_stores_stats_and_privacy_opt_out() {
        let app = test_app().await;
        // Unauthenticated requests
        let (unauth_get, _, _) = request(&app, "GET", "/api/stats/player", None, None).await;
        assert_eq!(unauth_get, 401);
        let (unauth_put, _, _) = request(
            &app,
            "PUT",
            "/api/stats/player",
            None,
            Some(json!({ "stats": {} })),
        )
        .await;
        assert_eq!(unauth_put, 401);

        // Pending account
        let (pending_get, _, _) = request(&app, "GET", "/api/stats/player", Some(PENDING_TOKEN), None).await;
        assert_eq!(pending_get, 403);

        // Active account initially empty
        let (init_get, _, init_data) =
            request(&app, "GET", "/api/stats/player", Some(USER_TOKEN), None).await;
        assert_eq!(init_get, 200);
        assert_eq!(init_data["stats"], json!({}));
        assert_eq!(init_data["isPrivate"], json!(false));

        // Save stats with privacy opt-out
        let sample_stats = json!({
            "games": 15,
            "wins": 10,
            "losses": 5,
            "cards": {
                "core-001": { "played": 12, "defeated": 4, "destroyed": 1 },
            },
        });
        let (put_status, _, put_data) = request(
            &app,
            "PUT",
            "/api/stats/player",
            Some(USER_TOKEN),
            Some(json!({ "stats": sample_stats, "isPrivate": true })),
        )
        .await;
        assert_eq!(put_status, 200);
        assert_eq!(put_data["stats"], sample_stats);
        assert_eq!(put_data["isPrivate"], json!(true));

        // Verify GET reads it back
        let (_, _, after_data) = request(&app, "GET", "/api/stats/player", Some(USER_TOKEN), None).await;
        assert_eq!(after_data["stats"], sample_stats);
        assert_eq!(after_data["isPrivate"], json!(true));

        // Rejects body exceeding PLAYER_STATS_BYTES_MAX
        let huge_stats = json!({ "data": "x".repeat(count(PLAYER_STATS_BYTES_MAX) + 10) });
        let (huge_status, _, _) = request(
            &app,
            "PUT",
            "/api/stats/player",
            Some(USER_TOKEN),
            Some(json!({ "stats": huge_stats })),
        )
        .await;
        assert_eq!(huge_status, 400);
    }

    /// R1142000: PUT /api/stats/player accepts a body under PLAYER_STATS_BYTES_MAX (65536)
    /// and summarises it on GET /api/stats/players; a body exceeding it returns 400.
    #[tokio::test]
    async fn r1142000_put_player_stats_accepts_body_under_new_cap_and_refuses_past_it() {
        let app = test_app().await;

        let base_prefix = r#"{"stats":{"games":20,"wins":15,"losses":5,"draws":0,"cards":{"core-001":{"played":10}},"pad":""#;
        let base_suffix = r#""},"isPrivate":false}"#;
        let overhead = base_prefix.len() + base_suffix.len();
        assert!(overhead < PLAYER_STATS_BYTES_MAX);

        // 1. Body just under cap (exactly PLAYER_STATS_BYTES_MAX bytes)
        let pad_len = PLAYER_STATS_BYTES_MAX - overhead;
        let body_str = format!("{base_prefix}{}{base_suffix}", "a".repeat(pad_len));
        let body: Value = serde_json::from_str(&body_str).expect("valid JSON");
        assert_eq!(body.to_string().encode_utf16().count(), PLAYER_STATS_BYTES_MAX);

        let (status, _, put_res) =
            request(&app, "PUT", "/api/stats/player", Some(USER_TOKEN), Some(body)).await;
        assert_eq!(status, 200);
        assert_eq!(put_res["stats"]["games"], 20);

        // Verified summarized in public players list
        let (list_status, _, list_body) = get_public(&app, "/api/stats/players").await;
        assert_eq!(list_status, 200);
        let players = list_body["players"].as_array().expect("players list");
        let found = players.iter().find(|p| p["games"] == 20);
        assert!(found.is_some(), "player summary found in public players list");
        let player = found.unwrap();
        assert_eq!(player["games"], 20);
        assert_eq!(player["wins"], 15);
        assert_eq!(player["losses"], 5);
        assert_eq!(player["favouriteCards"][0]["id"], "core-001");
        assert_eq!(player["favouriteCards"][0]["count"], 10);

        // 2. Body exceeding cap by 1 byte returns 400
        let over_body_str = format!("{base_prefix}{}{base_suffix}", "a".repeat(pad_len + 1));
        let over_body: Value = serde_json::from_str(&over_body_str).expect("valid JSON");
        assert_eq!(
            over_body.to_string().encode_utf16().count(),
            PLAYER_STATS_BYTES_MAX + 1
        );

        let (over_status, _, _) = request(
            &app,
            "PUT",
            "/api/stats/player",
            Some(USER_TOKEN),
            Some(over_body),
        )
        .await;
        assert_eq!(over_status, 400);
    }

    #[tokio::test]
    async fn r654_get_api_stats_players_lists_public_player_summaries_respects_privacy_and_search_omits_elo()
    {
        let app = test_app().await;
        // Setup profiles
        let alice_id = seed_profile(&app, "u-alice", "Alice_Wonder").await;
        let bob_id = seed_profile(&app, "u-bob", "Bob_Builder").await;
        let charlie_id = seed_profile(&app, "u-charlie", "Charlie_Secret").await;

        // Alice: public, 50 games
        put_player_stats(
            &app,
            &alice_id,
            json!({
                "games": 50,
                "wins": 30,
                "losses": 20,
                "cards": {
                    "core-001": { "played": 25, "defeated": 3, "destroyed": 2 },
                    "core-002": { "played": 15, "defeated": 1, "destroyed": 0 },
                    "core-nemesis": { "playedAgainst": 12 },
                },
            }),
            false,
            1000,
        )
        .await;

        // Bob: public, 100 games
        put_player_stats(
            &app,
            &bob_id,
            json!({ "games": 100, "wins": 70, "losses": 30 }),
            false,
            2000,
        )
        .await;

        // Charlie: opted out (isPrivate: true), 200 games
        put_player_stats(
            &app,
            &charlie_id,
            json!({ "games": 200, "wins": 150 }),
            true,
            3000,
        )
        .await;

        let (status, headers, data) = get_public(&app, "/api/stats/players").await;
        assert_eq!(status, 200);
        assert_eq!(
            cache_control(&headers),
            Some(format!("public, max-age={PLAYER_STATS_CACHE_TTL_SECONDS}").as_str())
        );

        // Charlie is private, so only Bob and Alice appear
        let players = data["players"].as_array().expect("players is a list");
        let ids: Vec<&str> = players
            .iter()
            .map(|player| player["profileId"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(ids, [bob_id.as_str(), alice_id.as_str()]);
        let (bob, alice) = (&players[0], &players[1]);
        assert_eq!(bob["username"], "Bob_Builder");
        assert_eq!(alice["username"], "Alice_Wonder");
        assert!(bob.get("displayName").is_none());

        // Elo/rating must NOT be exposed
        assert!(bob.get("rating").is_none());
        assert!(alice.get("rating").is_none());

        // Alice's favourite cards and fun stats
        assert_eq!(
            alice["favouriteCards"],
            json!([{ "id": "core-001", "count": 25 }, { "id": "core-002", "count": 15 }])
        );
        assert_eq!(
            alice["funStats"],
            json!({ "nemesisCardId": "core-nemesis", "totalDestroyed": 2, "totalDefeated": 4 })
        );

        // Search by username, by its case fold (R1436)
        let (_, _, search_data) = get_public(&app, "/api/stats/players?search=ALICE").await;
        let found: Vec<&str> = search_data["players"]
            .as_array()
            .expect("players is a list")
            .iter()
            .map(|player| player["profileId"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(found, [alice_id.as_str()]);

        let (_, _, search_none_data) = get_public(&app, "/api/stats/players?search=Charlie").await;
        assert_eq!(search_none_data["players"], json!([])); // Charlie opted out
    }

    /// R1436: the stats rows name nobody; each row's `username` is joined in from `profiles`, so a
    /// rename shows on the next read, with the row it labels left as it was.
    #[tokio::test]
    async fn r1436_get_api_stats_players_shows_a_rename_at_once_and_rewrites_no_stats_row() {
        let app = test_app().await;
        let alice_id = seed_profile(&app, "u-alice", "Alice").await;
        let bob_id = seed_profile(&app, "u-bob", "Bob").await;
        put_player_stats(
            &app,
            &alice_id,
            json!({ "games": 5, "wins": 3, "losses": 2 }),
            false,
            1000,
        )
        .await;
        put_player_stats(
            &app,
            &bob_id,
            json!({ "games": 9, "wins": 4, "losses": 5 }),
            false,
            2000,
        )
        .await;
        let rows = |data: &Value| -> Vec<Value> {
            data["players"]
                .as_array()
                .expect("players is a list")
                .iter()
                .map(|player| json!([player["profileId"], player["username"], player["updatedAt"]]))
                .collect()
        };
        let (_, _, before) = get_public(&app, "/api/stats/players").await;
        assert_eq!(
            rows(&before),
            vec![json!([bob_id, "Bob", 2000]), json!([alice_id, "Alice", 1000])]
        );

        claim_username(&app, &alice_id, "Alicia", USERNAME_CHANGE_COOLDOWN_MS).await;

        let (_, _, after) = get_public(&app, "/api/stats/players").await;
        assert_eq!(
            rows(&after),
            vec![json!([bob_id, "Bob", 2000]), json!([alice_id, "Alicia", 1000])]
        );
        let (_, _, found) = get_public(&app, "/api/stats/players?search=alicia").await;
        assert_eq!(rows(&found), vec![json!([alice_id, "Alicia", 1000])]);
    }
}

mod r978000_kept_card_stats {
    use super::*;
    use std::time::Duration;

    /// The cache's time to live, in milliseconds.
    fn ttl_ms() -> u64 {
        u64::try_from(CARD_STATS_CACHE_TTL_SECONDS).expect("the TTL is not negative") * 1000
    }

    /// A live game on `patch` (the current one when `None`) whose first seat plays `p1_deck`.
    fn live_game(id: &str, p1_deck: Vec<&'static str>, patch: Option<&str>) -> Value {
        make_game_record(
            id,
            RecordOptions {
                source: Some("live"),
                patch: patch.map(str::to_string),
                p1_deck: Some(p1_deck),
                ..RecordOptions::default()
            },
        )
    }

    /// The games of the card `card` in a cards response, 0 when it is not listed.
    fn games_of(data: &Value, card: &str) -> Value {
        find(&data["cards"], "id", &json!(card))
            .map(|stat| stat["games"].clone())
            .unwrap_or(json!(0))
    }

    #[tokio::test(start_paused = true)]
    async fn r978000_card_stats_are_kept_for_the_ttl_then_computed_again() {
        let app = test_app().await;
        insert_records(
            &app,
            vec![live_game("kept-1", vec!["core-001", "core-002"], None)],
        )
        .await;

        let (status, headers, first) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(status, 200);
        assert_eq!(
            cache_control(&headers),
            Some(format!("public, max-age={CARD_STATS_CACHE_TTL_SECONDS}").as_str())
        );
        assert_eq!(first["totalGames"], json!(1));
        assert_eq!(first["gate"]["liveGames"], json!(1));
        assert_eq!(games_of(&first, "core-001"), json!(1));

        // A record inserted within the TTL is not in what the server kept.
        insert_records(
            &app,
            vec![live_game("kept-2", vec!["core-001", "core-002"], None)],
        )
        .await;
        let (_, _, again) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(again, first);
        // The filters are applied to the kept report, not kept apart.
        let (_, _, core_only) = get_public(&app, "/api/stats/cards?set=Core").await;
        assert_eq!(core_only["totalGames"], json!(1));
        // Another pilot is another report: it is read off the records as they stand.
        let (_, _, humans) = get_public(&app, "/api/stats/cards?pilot=human").await;
        assert_eq!(humans["totalGames"], json!(2));

        tokio::time::advance(Duration::from_millis(ttl_ms() - 1)).await;
        let (_, _, within) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(within, first);

        tokio::time::advance(Duration::from_millis(1)).await;
        let (_, _, after) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(after["totalGames"], json!(2));
        assert_eq!(after["gate"]["liveGames"], json!(2));
        assert_eq!(games_of(&after, "core-001"), json!(2));
    }

    #[tokio::test(start_paused = true)]
    async fn r978000_card_drill_downs_share_the_kept_reports_across_cards() {
        let app = test_app().await;
        let gate = count(PUBLIC_STATS_MIN_LIVE_GAMES);
        let records: Vec<Value> = (0..gate)
            .map(|i| live_game(&format!("kept-drill-{i}"), vec!["core-001", "core-002"], None))
            .collect();
        insert_records(&app, records).await;
        let patch = json!(current_patch());

        let (status, _, first) = get_public(&app, "/api/stats/cards/core-001").await;
        assert_eq!(status, 200);
        assert_eq!(
            find(&first["patches"], "patch", &patch).expect("the current patch cleared the gate")["games"],
            json!(gate)
        );

        // A game inserted within the TTL is in neither card's page: the second card reads what the
        // first one's page kept.
        insert_records(
            &app,
            vec![live_game("kept-drill-late", vec!["core-002", "core-005"], None)],
        )
        .await;
        let (_, _, kept) = get_public(&app, "/api/stats/cards/core-002").await;
        assert_eq!(
            find(&kept["patches"], "patch", &patch).expect("the current patch is listed")["games"],
            json!(gate)
        );
        assert_eq!(
            find(&kept["coPlayed"], "id", &json!("core-001")).expect("core-001 shared its deck")["games"],
            json!(gate)
        );
        assert!(find(&kept["coPlayed"], "id", &json!("core-005")).is_none());
        assert!(find(&kept["byTurn"], "turn", &json!(1)).is_none());

        tokio::time::advance(Duration::from_millis(ttl_ms())).await;
        let (_, _, after) = get_public(&app, "/api/stats/cards/core-002").await;
        assert_eq!(
            find(&after["patches"], "patch", &patch).expect("the current patch is listed")["games"],
            json!(gate + 1)
        );
        assert_eq!(
            find(&after["coPlayed"], "id", &json!("core-005")).expect("core-005 shared its deck")["games"],
            json!(1)
        );
        assert_eq!(
            find(&after["byTurn"], "turn", &json!(1)).expect("core-002 was played on turn 1")["games"],
            json!(1)
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r978000_a_patch_the_server_does_not_ship_is_never_kept() {
        let app = test_app().await;
        let patch = "v0.0.0-not-shipped";
        insert_records(
            &app,
            vec![live_game("kept-odd-1", vec!["core-001", "core-002"], Some(patch))],
        )
        .await;

        let path = format!("/api/stats/cards?patch={patch}");
        let (status, _, first) = get_public(&app, &path).await;
        assert_eq!(status, 200);
        assert_eq!(first["patch"], json!(patch));
        assert_eq!(first["totalGames"], json!(1));

        insert_records(
            &app,
            vec![live_game("kept-odd-2", vec!["core-001", "core-002"], Some(patch))],
        )
        .await;
        let (_, _, second) = get_public(&app, &path).await;
        assert_eq!(second["totalGames"], json!(2));
    }
}

mod r1044000_condensed_card_stats {
    use super::*;
    use std::time::Duration;

    use jackioh_server::api::retention::purge_expired;
    use jackioh_server::config::GAME_RECORD_RAW_RETENTION_DAYS;
    use jackioh_server::db::store::{Db, GameRecordQuery};

    const DAY_MS: i64 = 86_400_000;

    /// Every public card figure the records give: the cards for each source and pilot, and two
    /// cards' drill-downs, which read the turns and the co-played cards.
    async fn figures(app: &Arc<App>) -> Vec<Value> {
        let mut bodies = Vec::new();
        for path in [
            "/api/stats/cards",
            "/api/stats/cards?source=live",
            "/api/stats/cards?source=dev",
            "/api/stats/cards?pilot=human",
            "/api/stats/cards?pilot=ai",
            "/api/stats/cards/core-001",
            "/api/stats/cards/core-002?pilot=ai",
        ] {
            let (status, _, body) = get_public(app, path).await;
            assert_eq!(status, 200, "{path}");
            bodies.push(body);
        }
        bodies
    }

    #[tokio::test(start_paused = true)]
    async fn r1044000_r1044001_the_card_figures_stand_when_each_games_detail_is_purged() {
        let app = test_app().await;
        let decks = [
            vec!["core-001", "core-002", "core-003"],
            vec!["core-003", "core-002", "core-004"],
            vec!["core-004", "core-001", "core-005"],
        ];
        let mut records: Vec<Value> = Vec::new();
        for i in 0..24 {
            let mut record = make_game_record(
                &format!("cond-{i:02}"),
                RecordOptions {
                    source: Some(if i % 4 == 3 { "dev" } else { "live" }),
                    p1_deck: Some(decks[i % 3].clone()),
                    p2_deck: Some(decks[(i + 1) % 3].clone()),
                    winner: Some(["p1", "p2", "draw"][i % 3]),
                    p1_played_turns: Some(vec![1 + i % 4, 2 + i % 3]),
                    ..RecordOptions::default()
                },
            );
            // Some live games against a ladder bot, whose side is the AI's.
            if i % 4 == 1 {
                record["pilots"]["p2"] = json!("ai");
            }
            records.push(record);
        }
        insert_records(&app, records).await;
        let before = figures(&app).await;
        assert_eq!(before[0]["totalGames"], json!(24));

        // A week and a day on, the purge empties every record's detail.
        match &app.db {
            Db::Fake(data) => {
                for row in data.lock().await.tables.game_record_ledger.values_mut() {
                    row.recorded_at -= (GAME_RECORD_RAW_RETENTION_DAYS + 1) * DAY_MS;
                }
            }
            Db::Pg(_) => panic!("the API tests run on the fake store"),
        }
        let purged = purge_expired(&app).await.expect("purgeExpired");
        assert_eq!(purged.game_records, 24);
        let mut tx = app.db.begin(None).await.expect("a store transaction");
        let held = tx
            .game_records_list(&from::<GameRecordQuery>(
                json!({ "source": "all", "mode": null, "patch": null }),
            ))
            .await
            .expect("gameRecords.list");
        tx.commit().await.expect("commit");
        assert!(held.is_empty(), "no record's detail is held");

        // Past the kept reports' TTL, every figure is computed again, off the totals alone.
        tokio::time::advance(Duration::from_secs(
            u64::try_from(CARD_STATS_CACHE_TTL_SECONDS).expect("a TTL") + 1,
        ))
        .await;
        assert_eq!(figures(&app).await, before);
    }
}
