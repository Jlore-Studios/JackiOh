//! Public card and player statistics page tests (SPEC §9.11, R654).

use std::sync::Arc;

use axum::http::HeaderMap;
use jackioh_server::app::App;
use jackioh_server::config::{
    CARD_STATS_CACHE_TTL_SECONDS, CARD_STATS_MIN_SAMPLE, PLAYER_STATS_BYTES_MAX,
    PLAYER_STATS_CACHE_TTL_SECONDS, PUBLIC_STATS_MIN_LIVE_GAMES, USERNAME_CHANGE_COOLDOWN_MS,
};
use jackioh_server::db::store::{UsernameClaim, UsernameClaimOutcome};
use jackioh_server::username::username_key;
use serde_json::{Map, Value, json};

use crate::support::deps::{call, test_app};
use crate::support::store::from;

/// The active account's bearer token (`e2e-p1`).
const USER_TOKEN: &str = "e2e-token-p1";
/// The pending account's bearer token (`e2e-pending`).
const PENDING_TOKEN: &str = "e2e-token-pending";

/// The patch the server counts as current.
fn current_patch() -> String {
    jackioh_cards::catalog_version().to_string()
}

/// A config count as a `usize`, whichever integer type `config.rs` gives it.
fn count<T>(value: T) -> usize
where
    usize: TryFrom<T>,
    <usize as TryFrom<T>>::Error: std::fmt::Debug,
{
    usize::try_from(value).expect("a count fits a usize")
}

/// Options for constructing game records.
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
    /// p1's pilot and p2's: by default a live game is two people's, a development one two AIs'.
    pilots: Option<[&'static str; 2]>,
    /// R2089000: p1's Created cards used (`createdUsed`) and made but never used (`createdUnused`).
    p1_created_used: Option<Value>,
    p1_created_unused: Option<Vec<&'static str>>,
    /// R2777002: the cards of p1's deck that reached its hand (the first three its opening hand, the
    /// rest drawn); by default the whole deck, so no card goes unseen.
    p1_hand: Option<Vec<&'static str>>,
}

/// One seat's summary: the first three cards of `hand` are the opening hand, the rest drawn.
fn seat(
    deck: &[&str],
    hand: &[&str],
    played: &[&str],
    played_turns: Option<Vec<usize>>,
    played_radiant: Option<Vec<bool>>,
) -> Value {
    let mut val = json!({
        "deck": deck,
        "opening": hand.iter().take(3).collect::<Vec<_>>(),
        "drawn": hand.iter().skip(3).collect::<Vec<_>>(),
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
    let [p1_pilot, p2_pilot] = options.pilots.unwrap_or([pilot, pilot]);
    let p1_hand = options.p1_hand.unwrap_or_else(|| p1_deck.clone());
    let mut p1 = seat(
        &p1_deck,
        &p1_hand,
        &p1_played,
        options.p1_played_turns,
        options.p1_played_radiant,
    );
    if let Some(used) = options.p1_created_used {
        p1["createdUsed"] = used;
    }
    if let Some(unused) = options.p1_created_unused {
        p1["createdUnused"] = json!(unused);
    }

    json!({
        "id": record_id,
        "source": source,
        "mode": options.mode.unwrap_or("bo1"),
        "patch": patch,
        "pilots": { "p1": p1_pilot, "p2": p2_pilot },
        "game": {
            "first": "p1",
            "winner": options.winner.unwrap_or("p1"),
            "reason": "hero-death",
            "turns": options.turns.unwrap_or(5),
            "seats": {
                "p1": p1,
                "p2": seat(&p2_deck, &p2_deck, &p2_played, options.p2_played_turns, options.p2_played_radiant),
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

/// New active profile picking username at time 0; answers id.
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

/// One request through the whole app.
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
            // `+` is a space in a query unless it is encoded, and a space is no URI character
            // ("Training Grounds", #2204).
            let param = name.to_lowercase().replace('+', "%2B").replace(' ', "%20");
            let prefix = format!("{}-", name.to_lowercase().replace('+', "plus").replace(' ', ""));
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
        // 50 games: core-001 played in 30 (70% win), drawn in 20 (50% baseline), delta +0.20; core-002 never played.
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
        // 40 games testing floor on either side: core-001 meets floor, core-002 and core-005 each short one game.
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
    async fn r2427000_get_api_stats_cards_compares_the_drawn_games_with_the_never_drawn_and_withholds_the_delta_below_the_floor()
     {
        let app = test_app().await;
        // 40 live games with p1 holding core-001, core-002, core-005, core-006 and core-007 and
        // playing nothing: 20 won drawing core-006, 19 lost drawing core-007, 1 lost drawing neither.
        let deck = vec!["core-001", "core-002", "core-005", "core-006", "core-007"];
        let mut records = Vec::new();
        let groups: [(&str, usize, Vec<&str>, &str); 3] = [
            ("drawn-six", 20, vec!["core-006"], "p1"),
            ("drawn-seven", 19, vec!["core-007"], "p2"),
            ("drawn-neither", 1, vec![], "p2"),
        ];
        for (name, count, drawn, winner) in groups {
            for i in 1..=count {
                let mut record = make_game_record(
                    &format!("rec-{name}-{i}"),
                    RecordOptions {
                        source: Some("live"),
                        p1_deck: Some(deck.clone()),
                        p1_played: Some(vec![]),
                        winner: Some(winner),
                        ..RecordOptions::default()
                    },
                );
                record["game"]["seats"]["p1"]["drawn"] = json!(&drawn);
                records.push(record);
            }
        }
        insert_records(&app, records).await;

        let (status, _, data) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(status, 200);

        let card = |id: &str| find(&data["cards"], "id", &json!(id)).expect("the card is listed");
        // core-006 is drawn in the 20 games p1 won and never in the 20 it lost: both sides at the floor.
        let six = card("core-006");
        assert_eq!(six["drawnGames"], json!(20));
        assert_eq!(six["notDrawnGames"], json!(20));
        assert_eq!(six["notDrawnWinRate"].as_f64(), Some(0.0));
        let gap = six["drawnDelta"].as_f64().expect("core-006 has drawnDelta");
        assert!((gap - 1.0).abs() < 1e-9);
        // core-007 is drawn in 19, one short of the floor, so no delta, whatever the baseline.
        let seven = card("core-007");
        assert_eq!(seven["drawnGames"], json!(19));
        assert_eq!(seven["notDrawnGames"], json!(21));
        assert_eq!(seven["notDrawnWinRate"].as_f64(), Some(20.0 / 21.0));
        assert!(seven["drawnDelta"].is_null());
        // core-005 never leaves the opening hand: never drawn is empty, so no rate and no delta.
        let five = card("core-005");
        assert_eq!(five["notDrawnGames"], json!(0));
        assert!(five["notDrawnWinRate"].is_null());
        assert!(five["drawnDelta"].is_null());
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

    /// R1142000: PUT /api/stats/player accepts a body under PLAYER_STATS_BYTES_MAX (131072)
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

        // Alice's favorite cards and fun stats
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

/// R1771000: the server keeps the account copy in R1142000's shape, whatever the PUT holds.
mod r1771000_player_stats_account_copy {
    use super::*;

    /// PUT `stats` as the caller (`e2e-p1`); answers the echoed and the read-back copy.
    async fn put_and_read(app: &Arc<App>, stats: Value) -> (Value, Value) {
        let (status, _, put) = request(
            app,
            "PUT",
            "/api/stats/player",
            Some(USER_TOKEN),
            Some(json!({ "stats": stats, "isPrivate": false })),
        )
        .await;
        assert_eq!(status, 200);
        let (_, _, read) = request(app, "GET", "/api/stats/player", Some(USER_TOKEN), None).await;
        (put["stats"].clone(), read["stats"].clone())
    }

    #[tokio::test]
    async fn r1771000_put_player_stats_keeps_only_catalog_cards_and_whole_counts() {
        let app = test_app().await;
        let (put, read) = put_and_read(
            &app,
            json!({
                "games": 12, "wins": "7", "losses": -1, "draws": 2.0,
                "motto": "Visit example.test for free packs",
                "cards": {
                    "core-001": { "played": 9, "playedAgainst": 2.5, "destroyed": "3", "defeated": -4, "seen": 6 },
                    "Visit example.test for free packs": { "played": 1000 },
                    "core-002": { "played": 0, "seen": 5 },
                    "core-003": { "playedAgainst": 4, "destroyed": 1 },
                },
            }),
        )
        .await;
        let kept = json!({
            "games": 12,
            "draws": 2,
            "cards": { "core-001": { "played": 9 }, "core-003": { "playedAgainst": 4, "destroyed": 1 } },
        });
        assert_eq!(put, kept);
        assert_eq!(read, kept);

        let (_, _, list) = get_public(&app, "/api/stats/players").await;
        let player = find(&list["players"], "games", &json!(12)).expect("the caller's summary");
        assert_eq!(player["wins"], json!(0));
        assert_eq!(player["losses"], json!(0));
        assert_eq!(player["draws"], json!(2));
        assert_eq!(
            player["favouriteCards"],
            json!([{ "id": "core-001", "count": 9 }])
        );
        assert_eq!(
            player["funStats"],
            json!({ "nemesisCardId": "core-003", "totalDestroyed": 1, "totalDefeated": 0 })
        );
    }

    #[tokio::test]
    async fn r1771000_put_player_stats_stores_what_sync_payload_sends_unchanged() {
        let app = test_app().await;

        let first = json!({ "games": 0, "wins": 0, "losses": 0, "draws": 0, "cards": {} });
        let (put, read) = put_and_read(&app, first.clone()).await;
        assert_eq!(put, first);
        assert_eq!(read, first);

        let synced = json!({
            "games": 3, "wins": 2, "losses": 1, "draws": 0,
            "cards": {
                "core-001": { "played": 4, "playedAgainst": 1, "destroyed": 2, "defeated": 3 },
                "core-002": { "playedAgainst": 5 },
            },
        });
        let (put, read) = put_and_read(&app, synced.clone()).await;
        assert_eq!(put, synced);
        assert_eq!(read, synced);
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

mod r2491000_kept_card_history {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use jackioh_server::api::catalog::load_patch_versions;
    use jackioh_server::db::store::{Db, StoreError};

    /// Counts the fake store's `statsTotals.load` calls from here on, through its `on_call` hook.
    async fn count_loads(app: &App) -> Arc<AtomicUsize> {
        let loads = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&loads);
        match &app.db {
            Db::Fake(data) => {
                data.lock().await.on_call = Some(Arc::new(move |method: &str| -> Result<(), StoreError> {
                    if method == "statsTotals.load" {
                        counter.fetch_add(1, Ordering::SeqCst);
                    }
                    Ok(())
                }));
            }
            Db::Pg(_) => panic!("the API tests run on the fake store"),
        }
        loads
    }

    /// The loads one request to `path` made, which answers 200; the count starts again after it.
    async fn loads_of(app: &Arc<App>, loads: &AtomicUsize, path: &str) -> usize {
        let (status, _, _) = get_public(app, path).await;
        assert_eq!(status, 200, "{path}");
        loads.swap(0, Ordering::SeqCst)
    }

    /// A live game of `mode` on `patch` whose first seat, on `core-001` and `core-002`, `winner` wins.
    fn game(id: String, patch: &str, mode: &'static str, winner: &'static str) -> Value {
        make_game_record(
            &id,
            RecordOptions {
                source: Some("live"),
                mode: Some(mode),
                patch: Some(patch.to_string()),
                p1_deck: Some(vec!["core-001", "core-002"]),
                winner: Some(winner),
                ..RecordOptions::default()
            },
        )
    }

    #[tokio::test(start_paused = true)]
    async fn r2491000_a_drill_down_reads_its_patch_history_in_one_load_kept_per_match_type() {
        let app = test_app().await;
        let loads = count_loads(&app).await;
        assert!(load_patch_versions().await.len() > 2);

        // Each view misses once: the history's one load and the current patch's drills, however
        // many patches ship.
        for path in [
            "/api/stats/cards/core-001",
            "/api/stats/cards/core-001?mode=bo1",
            "/api/stats/cards/core-001?mode=bo3",
            "/api/stats/cards/core-001?mode=random",
        ] {
            assert_eq!(loads_of(&app, &loads, path).await, 2, "{path}");
        }

        // Every view is kept now, for every card's page.
        for path in ["/api/stats/cards/core-001", "/api/stats/cards/core-002?mode=bo3"] {
            assert_eq!(loads_of(&app, &loads, path).await, 0, "{path}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r2491000_the_kept_history_lists_each_cleared_patch_as_its_own_load_did() {
        let app = test_app().await;
        let gate = count(PUBLIC_STATS_MIN_LIVE_GAMES);
        let current = current_patch();
        let versions = load_patch_versions().await;
        let mut past = versions.iter().filter(|version| **version != current);
        let old = past.next().expect("a past patch").clone();
        let short = past.next().expect("a second past patch").clone();

        let mut records: Vec<Value> = (0..gate)
            .map(|i| game(format!("hist-old-{i}"), &old, "bo1", "p1"))
            .collect();
        records.push(game("hist-short-0".to_string(), &short, "bo1", "p1"));
        records.extend((0..gate).map(|i| game(format!("hist-now-{i}"), &current, "bo3", "p2")));
        insert_records(&app, records).await;

        let row = |patch: &str, games: usize, rate: Value| json!({ "patch": patch, "games": games, "winRate": rate });

        // `short` is below the gate: it is not listed.
        let (_, _, all) = get_public(&app, "/api/stats/cards/core-001").await;
        assert_eq!(
            all["patches"],
            json!([row(&old, gate, json!(1.0)), row(&current, gate, json!(0.0))])
        );

        // The gate counts every mode, so a patch whose games are all of another mode is listed
        // with no games.
        let (_, _, bo1) = get_public(&app, "/api/stats/cards/core-001?mode=bo1").await;
        assert_eq!(
            bo1["patches"],
            json!([row(&old, gate, json!(1.0)), row(&current, 0, Value::Null)])
        );

        // A card no counted deck held is listed on each cleared patch with no games.
        let (_, _, unheld) = get_public(&app, "/api/stats/cards/core-005").await;
        assert_eq!(
            unheld["patches"],
            json!([row(&old, 0, Value::Null), row(&current, 0, Value::Null)])
        );
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

// R1367000: `GET /api/stats/cards` reads each card over its patch-history window.
// Query `?patch=` tests micro version stability across releases (#1832).
mod r1367000_card_stats_by_window {
    use super::*;
    use jackioh_engine::wire::stats::{PatchTier, WindowRequest, card_window};

    /// Newest micro version and card whose previous window opens at X.
    fn x_and_micro() -> (String, String, &'static str) {
        let history = jackioh_cards::patch_history();
        for micro in (1..history.len()).rev() {
            if history[micro].tier != PatchTier::Micro {
                continue;
            }
            let Some(x) = history[..micro]
                .iter()
                .rposition(|step| step.tier != PatchTier::Micro)
            else {
                continue;
            };
            for card in &history[micro].plays {
                let previous = card_window(
                    &history,
                    &history[micro - 1].version,
                    Some(card),
                    WindowRequest::Gated,
                    |_| 0,
                );
                if previous.versions.first() == Some(&history[x].version) {
                    let card: &'static str = card.clone().leak();
                    return (history[x].version.clone(), history[micro].version.clone(), card);
                }
            }
        }
        panic!("the history holds a micro that plays a card after its X")
    }

    fn live(id: String, patch: &str, card: &'static str) -> Value {
        make_game_record(
            &id,
            RecordOptions {
                source: Some("live"),
                patch: Some(patch.to_string()),
                p1_deck: Some(vec![card, "core-001"]),
                ..RecordOptions::default()
            },
        )
    }

    fn dev(id: String, patch: &str, card: &'static str) -> Value {
        make_game_record(
            &id,
            RecordOptions {
                source: Some("dev"),
                patch: Some(patch.to_string()),
                p1_deck: Some(vec![card, "core-001"]),
                ..RecordOptions::default()
            },
        )
    }

    /// Live and development game figures across X and micro versions.
    async fn cards(x_games: i64, micro_games: i64) -> (Value, &'static str, String, String) {
        let (x, micro, card) = x_and_micro();
        let app = test_app().await;
        let mut records: Vec<Value> = (0..x_games).map(|n| live(format!("x-{n}"), &x, card)).collect();
        records.extend((0..micro_games).map(|n| live(format!("micro-{n}"), &micro, card)));
        records.extend((0..10).map(|n| dev(format!("pad-{n}"), &micro, card)));
        insert_records(&app, records).await;
        let (status, _, data) = get_public(&app, &format!("/api/stats/cards?patch={micro}")).await;
        assert_eq!(status, 200);
        (data, card, x, micro)
    }

    #[tokio::test]
    async fn r1367000_the_provisional_label_follows_the_card_window_not_the_patch() {
        let short = PUBLIC_STATS_MIN_LIVE_GAMES - 300;
        let (data, card, x, micro) = cards(300, short).await;
        // The window every card shares holds both versions' live games, and clears.
        assert_eq!(data["gate"]["liveGames"], json!(PUBLIC_STATS_MIN_LIVE_GAMES));
        assert_eq!(data["gate"]["cleared"], json!(true));
        assert_eq!(data["source"], "live");
        let core = find(&data["cards"], "id", &json!("core-001")).expect("core-001 is listed");
        assert_eq!(core["window"]["from"], json!(x));
        assert_eq!(core["window"]["source"], "live");
        assert_eq!(core["games"], json!(PUBLIC_STATS_MIN_LIVE_GAMES));
        // The micro reopened the card's window: its 700 live games are short, and so is the previous
        // window's 300, so AI games pad it.
        let stat = find(&data["cards"], "id", &json!(card)).expect("the card is listed");
        assert_eq!(stat["window"]["from"], json!(micro));
        assert_eq!(stat["window"]["liveGames"], json!(short));
        assert_eq!(stat["window"]["source"], "provisional");
        assert_eq!(stat["games"], json!(short + 10));
    }

    #[tokio::test]
    async fn r1367000_a_short_card_window_shows_its_previous_window_once_that_one_clears() {
        let (data, card, x, _) = cards(PUBLIC_STATS_MIN_LIVE_GAMES, PUBLIC_STATS_MIN_LIVE_GAMES - 1).await;
        let stat = find(&data["cards"], "id", &json!(card)).expect("the card is listed");
        assert_eq!(stat["window"]["source"], "previous");
        assert_eq!(stat["window"]["counted"][0], json!(x));
        assert_eq!(stat["games"], json!(PUBLIC_STATS_MIN_LIVE_GAMES));
        // At 1,000 the card's own window clears and counts its own live games alone.
        let (data, card, _, micro) = cards(0, PUBLIC_STATS_MIN_LIVE_GAMES).await;
        let stat = find(&data["cards"], "id", &json!(card)).expect("the card is listed");
        assert_eq!(stat["window"]["source"], "live");
        assert_eq!(stat["window"]["counted"][0], json!(micro));
        assert_eq!(stat["games"], json!(PUBLIC_STATS_MIN_LIVE_GAMES));
    }

    #[tokio::test]
    async fn r1973001_r1973002_the_route_lists_every_patch_newest_first_with_a_gameless_patch_folded_in() {
        let (x, micro, card) = x_and_micro();
        let app = test_app().await;
        let mut records: Vec<Value> = (0..5).map(|n| live(format!("x-{n}"), &x, card)).collect();
        records.extend((0..3).map(|n| dev(format!("pad-{n}"), &micro, card)));
        insert_records(&app, records).await;
        let (status, _, data) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(status, 200);
        let patches = data["patches"].as_array().expect("the patches are a list");
        let history = jackioh_cards::patch_history();
        let newest = &history.last().expect("a version has shipped").version;
        let versions = |window: &Value| -> Vec<String> {
            window["versions"]
                .as_array()
                .expect("a window lists its versions")
                .iter()
                .map(|version| version.as_str().expect("a version").to_string())
                .collect()
        };
        // Newest first: the first window ends at the newest version.
        assert_eq!(versions(&patches[0]).last(), Some(newest));
        // X's window holds its five live games and the micro's three development games.
        let held = patches
            .iter()
            .find(|window| versions(window)[0] == x)
            .expect("X opens a window");
        assert_eq!((&held["games"], &held["liveGames"]), (&json!(8), &json!(5)));
        assert!(versions(held).contains(&micro));
        // R1973001: no other patch had a game, so each folds into the window before it, and only a
        // major, X or the newest version opens one.
        for window in patches {
            let first = &versions(window)[0];
            let step = history
                .iter()
                .find(|step| &step.version == first)
                .expect("a shipped version");
            assert!(
                step.tier == PatchTier::Major || *first == x || first == newest,
                "{first} opens a window with no game played on it"
            );
        }
        // Every version is listed once, in the history's order.
        let listed: Vec<String> = patches.iter().rev().flat_map(versions).collect();
        let shipped: Vec<String> = history.iter().map(|step| step.version.clone()).collect();
        assert_eq!(listed, shipped);
    }
}

mod r377_public_stats_match_type {
    use super::*;

    /// 25 live games of `mode` with p1 on `core-001`/`core-002`, won by `winner`.
    fn mode_games(mode: &'static str, winner: &'static str, from: usize, count: usize) -> Vec<Value> {
        (from..from + count)
            .map(|i| {
                make_game_record(
                    &format!("mode-{mode}-{i}"),
                    RecordOptions {
                        source: Some("live"),
                        mode: Some(mode),
                        p1_deck: Some(vec!["core-001", "core-002"]),
                        winner: Some(winner),
                        ..RecordOptions::default()
                    },
                )
            })
            .collect()
    }

    /// 25 live games of each of R257's three modes: `bo1` won by p1, `bo3` by p2, `random` by p1.
    async fn seed_three_modes(app: &App) {
        let mut records = Vec::new();
        records.extend(mode_games("bo1", "p1", 1, 25));
        records.extend(mode_games("bo3", "p2", 101, 25));
        records.extend(mode_games("random", "p1", 201, 25));
        insert_records(app, records).await;
    }

    #[tokio::test]
    async fn r377_get_api_stats_cards_filters_cards_by_match_type() {
        let app = test_app().await;
        seed_three_modes(&app).await;

        let (_, _, bo1) = get_public(&app, "/api/stats/cards?source=live&mode=bo1").await;
        assert_eq!(bo1["totalGames"], json!(25));
        assert_eq!(bo1["mode"], json!("bo1"));
        assert_eq!(bo1["modeLabel"], json!("Best of 1"));
        assert_eq!(bo1["summary"]["mode"], json!("bo1"));
        let bo1_card = find(&bo1["cards"], "id", &json!("core-001")).expect("core-001 is listed");
        assert_eq!(bo1_card["games"], json!(25));
        assert_eq!(bo1_card["winRate"].as_f64(), Some(1.0));

        let (_, _, bo3) = get_public(&app, "/api/stats/cards?source=live&mode=bo3").await;
        assert_eq!(bo3["totalGames"], json!(25));
        assert_eq!(bo3["mode"], json!("bo3"));
        assert_eq!(bo3["modeLabel"], json!("Conquest"));
        let bo3_card = find(&bo3["cards"], "id", &json!("core-001")).expect("core-001 is listed");
        assert_eq!(bo3_card["games"], json!(25));
        assert_eq!(bo3_card["winRate"].as_f64(), Some(0.0));

        let (_, _, random) = get_public(&app, "/api/stats/cards?source=live&mode=random").await;
        assert_eq!(random["totalGames"], json!(25));
        assert_eq!(random["mode"], json!("random"));
        assert_eq!(random["modeLabel"], json!("All Random"));
        let random_card = find(&random["cards"], "id", &json!("core-001")).expect("core-001 is listed");
        assert_eq!(random_card["games"], json!(25));
    }

    #[tokio::test]
    async fn r377_get_api_stats_cards_defaults_to_every_mode() {
        let app = test_app().await;
        seed_three_modes(&app).await;

        let (_, _, missing) = get_public(&app, "/api/stats/cards?source=live").await;
        let (_, _, all) = get_public(&app, "/api/stats/cards?source=live&mode=all").await;
        let (_, _, bogus) = get_public(&app, "/api/stats/cards?source=live&mode=bogus").await;
        assert_eq!(missing, all);
        assert_eq!(bogus, all);
        assert_eq!(all["totalGames"], json!(75));
        assert_eq!(all["mode"], json!("all"));
        assert_eq!(all["modeLabel"], json!("All modes"));
        assert_eq!(all["summary"]["mode"], json!("all"));
    }

    #[tokio::test]
    async fn r978000_kept_reports_are_keyed_by_match_type() {
        let app = test_app().await;
        seed_three_modes(&app).await;

        let (_, _, bo1_first) = get_public(&app, "/api/stats/cards?source=live&mode=bo1").await;
        let (_, _, bo3) = get_public(&app, "/api/stats/cards?source=live&mode=bo3").await;
        assert_ne!(bo1_first, bo3);
        let (_, _, bo1_again) = get_public(&app, "/api/stats/cards?source=live&mode=bo1").await;
        assert_eq!(bo1_again, bo1_first);
    }

    #[tokio::test]
    async fn r377_get_api_stats_cards_answers_empty_figures_when_a_mode_has_no_games() {
        let app = test_app().await;
        insert_records(&app, mode_games("bo1", "p1", 1, 25)).await;

        let (status, _, random) = get_public(&app, "/api/stats/cards?source=live&mode=random").await;
        assert_eq!(status, 200);
        assert_eq!(random["totalGames"], json!(0));
        assert_eq!(random["mode"], json!("random"));
        for card in random["cards"].as_array().expect("cards is a list") {
            assert_eq!(card["games"], json!(0));
            assert_eq!(card["hasEnoughGames"], json!(false));
        }
        assert_eq!(random["summary"]["bestCard"], Value::Null);
        assert_eq!(random["summary"]["worstCard"], Value::Null);
    }

    #[tokio::test]
    async fn r377_get_api_stats_cards_id_scopes_the_drill_down_by_match_type() {
        let app = test_app().await;
        let mut records = Vec::new();
        for i in 1..=5 {
            records.push(make_game_record(
                &format!("drill-bo1-{i}"),
                RecordOptions {
                    source: Some("live"),
                    mode: Some("bo1"),
                    p1_deck: Some(vec!["core-001", "core-002"]),
                    p1_played: Some(vec!["core-001"]),
                    p1_played_turns: Some(vec![1]),
                    winner: Some("p1"),
                    ..RecordOptions::default()
                },
            ));
        }
        for i in 1..=5 {
            records.push(make_game_record(
                &format!("drill-bo3-{i}"),
                RecordOptions {
                    source: Some("live"),
                    mode: Some("bo3"),
                    p1_deck: Some(vec!["core-001", "core-002"]),
                    p1_played: Some(vec!["core-001"]),
                    p1_played_turns: Some(vec![3]),
                    winner: Some("p1"),
                    ..RecordOptions::default()
                },
            ));
        }
        insert_records(&app, records).await;

        let (status, _, bo1) = get_public(&app, "/api/stats/cards/core-001?mode=bo1").await;
        assert_eq!(status, 200);
        let bo1_turn1 = find(&bo1["byTurn"], "turn", &json!(1)).expect("turn 1 is tallied");
        assert_eq!(bo1_turn1["games"], json!(5));
        assert!(find(&bo1["byTurn"], "turn", &json!(3)).is_none());

        let (_, _, bo3) = get_public(&app, "/api/stats/cards/core-001?mode=bo3").await;
        let bo3_turn3 = find(&bo3["byTurn"], "turn", &json!(3)).expect("turn 3 is tallied");
        assert_eq!(bo3_turn3["games"], json!(5));
        assert!(find(&bo3["byTurn"], "turn", &json!(1)).is_none());
    }
}

mod r3070000_every_card {
    use super::*;

    fn card<'a>(data: &'a Value, id: &str) -> &'a Value {
        find(&data["cards"], "id", &json!(id)).expect("the card is listed")
    }

    /// `count` live Best of 1 games p1 wins when `won`: p1 holds core-001 to core-040 and core-041, sees
    /// `hand` and plays none, p2 holds core-100, and p1 made `made` Dormant Starships, each used at once.
    fn games(tag: &str, hand: &[&'static str], won: bool, made: usize, count: usize) -> Vec<Value> {
        let mut deck: Vec<&'static str> = FIELD.to_vec();
        deck.push("core-041");
        (0..count)
            .map(|i| {
                let starships: Vec<Value> = (0..made)
                    .map(|_| json!({ "card": "stinkyscape-t-dormant-starship", "turn": 5, "radiant": false }))
                    .collect();
                make_game_record(
                    &format!("every-{tag}-{i}"),
                    RecordOptions {
                        source: Some("live"),
                        mode: Some("bo1"),
                        p1_deck: Some(deck.clone()),
                        p1_hand: Some(hand.to_vec()),
                        p1_played: Some(Vec::new()),
                        p2_deck: Some(vec!["core-100"]),
                        p2_played: Some(Vec::new()),
                        p1_created_used: (made > 0).then(|| json!(starships)),
                        winner: Some(if won { "p1" } else { "p2" }),
                        ..RecordOptions::default()
                    },
                )
            })
            .collect()
    }

    /// Forty Core cards, half seen in some games and half in others, as R3022000's test reads them.
    const FIELD: [&str; 40] = [
        "core-001", "core-002", "core-003", "core-004", "core-005", "core-006", "core-007", "core-008",
        "core-009", "core-010", "core-011", "core-012", "core-013", "core-014", "core-015", "core-016",
        "core-017", "core-018", "core-019", "core-020", "core-021", "core-022", "core-023", "core-024",
        "core-025", "core-026", "core-027", "core-028", "core-029", "core-030", "core-031", "core-032",
        "core-033", "core-034", "core-035", "core-036", "core-037", "core-038", "core-039", "core-040",
    ];

    #[tokio::test]
    async fn r3070000_a_card_always_in_hand_and_the_dormant_starship_each_answer_an_adjusted_rate_and_a_tier()
    {
        let app = test_app().await;
        let (top, bottom) = FIELD.split_at(20);
        // core-041 is in every hand, as a Quickdraw card is. A Dormant Starship is made in the 48 games the
        // top half was seen and won, and in none of the rest.
        let with = |half: &[&'static str]| {
            let mut hand = half.to_vec();
            hand.push("core-041");
            hand
        };
        let mut records = Vec::new();
        records.extend(games("top-won", &with(top), true, 1, 48));
        records.extend(games("top-lost", &with(top), false, 0, 12));
        records.extend(games("bottom-won", &with(bottom), true, 0, 24));
        records.extend(games("bottom-lost", &with(bottom), false, 0, 36));
        insert_records(&app, records).await;

        let (status, _, page) = get_public(&app, "/api/stats/cards?source=live&mode=bo1").await;
        assert_eq!(status, 200);
        let tiers = &page["tiers"];
        let spread = tiers["spread"].as_f64().expect("the field's spread");
        let baseline = tiers["baseline"].as_f64().expect("the average deck's win rate");
        // The average deck: p1's 41 cards won 72 of 120 decks each, and p2's core-100 48 of 120.
        assert!(
            (baseline - (41.0 * 72.0 + 48.0) / (42.0 * 120.0)).abs() < 1e-9,
            "{tiers}"
        );
        let held = |rate: f64, games: f64| {
            let noise = 0.25 / games;
            0.5 + spread * spread / (spread * spread + noise) * (rate - baseline)
        };

        // core-041 was never left undrawn, yet answers a rate read against the average deck, and a tier.
        let always = card(&page, "core-041");
        let rate = always["adjustedWinRate"]
            .as_f64()
            .expect("read against the average deck");
        assert!((rate - held(0.6, 120.0)).abs() < 1e-9, "{always}");
        assert!(always["tier"].is_string(), "{always}");

        // The Dormant Starship's row reads the 48 seats that made one, all of which won: S.
        let starship = card(&page, "stinkyscape-t-dormant-starship");
        assert_eq!(starship["games"], json!(48));
        assert_eq!(starship["winRate"].as_f64(), Some(1.0));
        let made = starship["adjustedWinRate"]
            .as_f64()
            .expect("read against the average deck");
        assert!((made - held(1.0, 48.0)).abs() < 1e-9, "{starship}");
        assert!(starship["adjustedLow"].as_f64().is_some());
        assert_eq!(starship["hasEnoughGames"], json!(true));
        assert_eq!(starship["tier"], json!("S"));
    }
}

mod r3022000_tiers {
    use super::*;

    /// Forty Core cards, the field the test reads.
    const FIELD: [&str; 40] = [
        "core-001", "core-002", "core-003", "core-004", "core-005", "core-006", "core-007", "core-008",
        "core-009", "core-010", "core-011", "core-012", "core-013", "core-014", "core-015", "core-016",
        "core-017", "core-018", "core-019", "core-020", "core-021", "core-022", "core-023", "core-024",
        "core-025", "core-026", "core-027", "core-028", "core-029", "core-030", "core-031", "core-032",
        "core-033", "core-034", "core-035", "core-036", "core-037", "core-038", "core-039", "core-040",
    ];

    /// `count` live Best of 1 games: p1 holds the forty, sees `hand` of them and plays none, and wins
    /// when `won`.
    fn games(tag: &str, hand: &[&'static str], won: bool, count: usize) -> Vec<Value> {
        (0..count)
            .map(|i| {
                make_game_record(
                    &format!("tiers-{tag}-{i}"),
                    RecordOptions {
                        source: Some("live"),
                        mode: Some("bo1"),
                        p1_deck: Some(FIELD.to_vec()),
                        p1_hand: Some(hand.to_vec()),
                        p1_played: Some(Vec::new()),
                        p2_deck: Some(vec!["core-100"]),
                        p2_played: Some(Vec::new()),
                        winner: Some(if won { "p1" } else { "p2" }),
                        ..RecordOptions::default()
                    },
                )
            })
            .collect()
    }

    fn card<'a>(data: &'a Value, id: &str) -> &'a Value {
        find(&data["cards"], "id", &json!(id)).expect("the card is listed")
    }

    #[tokio::test]
    async fn r3022000_the_cards_read_against_the_whole_field_and_each_answers_its_tier() {
        let app = test_app().await;
        let (top, bottom) = FIELD.split_at(20);
        let mut records = Vec::new();
        // The first twenty seen in 60 games (48 won), the last twenty in 60 others (24 won), all forty
        // in 60 more (all won): the first twenty's drawn delta is 108/120 − 24/60 = +50 points, the last
        // twenty's 84/120 − 48/60 = −10. A deck that sees more of its cards wins more here, so the
        // average card's delta is +20, not 0.
        records.extend(games("top-won", top, true, 48));
        records.extend(games("top-lost", top, false, 12));
        records.extend(games("bottom-won", bottom, true, 24));
        records.extend(games("bottom-lost", bottom, false, 36));
        records.extend(games("all", &FIELD, true, 60));
        insert_records(&app, records).await;

        let (status, _, page) = get_public(&app, "/api/stats/cards?source=live&mode=bo1").await;
        assert_eq!(status, 200);
        // Equal games, so equal weights: the centre is the mean delta, and τ² the deltas' sample variance
        // less their sampling variance.
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
        let noise: f64 = 0.25 * (1.0 / 120.0 + 1.0 / 60.0);
        let spread_squared = 40.0 * 0.3 * 0.3 / 39.0 - noise;
        let tiers = &page["tiers"];
        assert_eq!(tiers["cards"], json!(40));
        assert!(close(tiers["center"].as_f64().unwrap(), 0.2), "{tiers}");
        assert!(
            close(tiers["spread"].as_f64().unwrap(), spread_squared.sqrt()),
            "{tiers}"
        );
        assert_eq!(
            (tiers["bandLow"].as_f64(), tiers["bandHigh"].as_f64()),
            (Some(0.45), Some(0.55))
        );

        // Each half keeps its 30 points from the centre as far as its games back it: the top is S, the
        // bottom F, though a raw −10 points would have read C under R2777000's centre of 0.
        let weight = spread_squared / (spread_squared + noise);
        let first = card(&page, "core-001");
        let last = card(&page, "core-040");
        assert!(
            close(first["adjustedWinRate"].as_f64().unwrap(), 0.5 + weight * 0.3),
            "{first}"
        );
        assert!(
            close(last["adjustedWinRate"].as_f64().unwrap(), 0.5 - weight * 0.3),
            "{last}"
        );
        assert_eq!(
            (first["tier"].clone(), last["tier"].clone()),
            (json!("S"), json!("F"))
        );
        // R3070000: p2's card, seen in every game, is read against the average deck: its decks won 48 of 180
        // against the average card's 40 · 132 + 48 of 41 · 180, so F.
        assert!(
            (tiers["baseline"].as_f64().unwrap() - 5328.0 / 7380.0).abs() < 1e-9,
            "{tiers}"
        );
        assert_eq!(card(&page, "core-100")["tier"], json!("F"));

        // A filtered page reads its cards against the same field.
        let (_, _, one) = get_public(&app, "/api/stats/cards?source=live&mode=bo1&card=core-040").await;
        assert_eq!(ids_of(&one), vec!["core-040".to_string()]);
        assert_eq!(one["tiers"], page["tiers"]);
        assert_eq!(card(&one, "core-040")["adjustedWinRate"], last["adjustedWinRate"]);
        assert_eq!(card(&one, "core-040")["tier"], json!("F"));
    }

    #[tokio::test]
    async fn r3022000_below_the_floor_a_card_has_no_tier_and_a_small_field_keeps_r2777000_s_prior() {
        let app = test_app().await;
        let mut records = Vec::new();
        // 15 games, below R654's floor of 20: core-001 seen in 8 (all won) and never drawn in 7 (none won).
        records.extend(games("seen", &["core-001"], true, 8));
        records.extend(games("unseen", &["core-002"], false, 7));
        insert_records(&app, records).await;

        let (_, _, page) = get_public(&app, "/api/stats/cards?source=live&mode=bo1").await;
        // No card has the floor's games on both sides: R2777000's centre and prior.
        assert_eq!(page["tiers"]["cards"], json!(0));
        assert_eq!(page["tiers"]["center"].as_f64(), Some(0.0));
        assert_eq!(page["tiers"]["spread"].as_f64(), Some(0.05));
        let a = card(&page, "core-001");
        assert_eq!(a["hasEnoughGames"], json!(false));
        assert!(a["adjustedWinRate"].as_f64().is_some());
        assert_eq!(a["tier"], Value::Null);
    }
}

mod r2777000_adjusted_and_split_card_stats {
    use super::*;

    /// `count` live games of `mode`: p1 holds `core-001` to `core-004`, sees only `hand` of them and
    /// plays none, and wins when `won`.
    fn games(tag: &str, mode: &'static str, hand: Vec<&'static str>, won: bool, count: usize) -> Vec<Value> {
        (0..count)
            .map(|i| {
                make_game_record(
                    &format!("adjusted-{tag}-{i}"),
                    RecordOptions {
                        source: Some("live"),
                        mode: Some(mode),
                        p1_deck: Some(vec!["core-001", "core-002", "core-003", "core-004"]),
                        p1_hand: Some(hand.clone()),
                        p1_played: Some(Vec::new()),
                        p2_deck: Some(vec!["core-005"]),
                        p2_played: Some(Vec::new()),
                        winner: Some(if won { "p1" } else { "p2" }),
                        ..RecordOptions::default()
                    },
                )
            })
            .collect()
    }

    fn card<'a>(data: &'a Value, id: &str) -> &'a Value {
        find(&data["cards"], "id", &json!(id)).expect("the card is listed")
    }

    #[tokio::test]
    async fn r2777000_r2777001_r2777002_constructed_pools_bo1_and_conquest_random_counts_a_card_once_seen_and_cards_list_by_adjusted_rate()
     {
        let app = test_app().await;
        let mut records = Vec::new();
        // Constructed: core-001 seen in 30 Best of 1 games (24 won) and 30 Conquest games (24 won),
        // never drawn in 60 more (30 won); core-002 seen in all 120 (78 won).
        let all = vec!["core-001", "core-002", "core-003", "core-004"];
        let without = vec!["core-002", "core-003", "core-004"];
        records.extend(games("bo1-seen-won", "bo1", all.clone(), true, 24));
        records.extend(games("bo1-seen-lost", "bo1", all.clone(), false, 6));
        records.extend(games("bo3-seen-won", "bo3", all.clone(), true, 24));
        records.extend(games("bo3-seen-lost", "bo3", all.clone(), false, 6));
        records.extend(games("bo1-unseen-won", "bo1", without.clone(), true, 30));
        records.extend(games("bo1-unseen-lost", "bo1", without.clone(), false, 30));
        // All Random: core-001 dealt in 40 games, seen in 10 of them (all won), never drawn in 30
        // (10 won).
        records.extend(games("random-seen", "random", all.clone(), true, 10));
        records.extend(games("random-unseen-won", "random", without.clone(), true, 10));
        records.extend(games("random-unseen-lost", "random", without.clone(), false, 20));
        insert_records(&app, records).await;

        // R2777001: Constructed is Best of 1 and Conquest together, and no All Random game.
        let (status, _, constructed) =
            get_public(&app, "/api/stats/cards?source=live&mode=constructed").await;
        assert_eq!(status, 200);
        assert_eq!(constructed["mode"], json!("constructed"));
        assert_eq!(constructed["modeLabel"], json!("Constructed"));
        assert_eq!(constructed["totalGames"], json!(120));
        let a = card(&constructed, "core-001");
        assert_eq!(a["games"], json!(120));
        assert_eq!(
            (a["drawnGames"].clone(), a["notDrawnGames"].clone()),
            (json!(60), json!(60))
        );
        // R2777000: 50% plus the 30-point drawn delta, kept as far as 60 games a side back it.
        let prior = 0.05_f64 * 0.05;
        let kept = prior / (prior + 0.25 * (1.0 / 60.0 + 1.0 / 60.0));
        let adjusted = a["adjustedWinRate"].as_f64().expect("both sides have games");
        assert!((adjusted - (0.5 + kept * 0.3)).abs() < 1e-9, "{adjusted}");
        // R3070000 amends this: core-002 was always seen, so it is read against the average deck. The four
        // cards p1 held won 78 of their 120 decks each, and p2's core-005 42 of 120: a baseline of 354/600.
        let held = prior / (prior + 0.25 / 120.0);
        let always = card(&constructed, "core-002")["adjustedWinRate"]
            .as_f64()
            .expect("read against the average deck");
        assert!(
            (always - (0.5 + held * (0.65 - 354.0 / 600.0))).abs() < 1e-9,
            "{always}"
        );
        // It still lists after core-001, whose rate is higher.
        let order = ids_of(&constructed);
        let at = |id: &str| order.iter().position(|listed| listed == id);
        assert!(at("core-001") < at("core-002"), "{order:?}");

        // R2777002: All Random counts a card once seen: 10 games, all won, below the floor.
        let (_, _, random) = get_public(&app, "/api/stats/cards?source=live&mode=random").await;
        let a = card(&random, "core-001");
        assert_eq!(a["games"], json!(10));
        assert_eq!(a["winRate"].as_f64(), Some(1.0));
        assert_eq!(a["hasEnoughGames"], json!(false));
        assert_eq!(a["notDrawnGames"], json!(30));
        assert!((a["playRate"].as_f64().unwrap() - 10.0 / 80.0).abs() < 1e-9);
        // Best of 1 alone still counts every deck that held it.
        let (_, _, bo1) = get_public(&app, "/api/stats/cards?source=live&mode=bo1").await;
        assert_eq!(card(&bo1, "core-001")["games"], json!(90));

        // The game figures and the drill-down read the same two modes.
        let (_, _, games) = get_public(&app, "/api/stats/games?source=live&mode=constructed").await;
        assert_eq!(games["totalGames"], json!(120));
        let (status, _, drill) = get_public(&app, "/api/stats/cards/core-001?mode=constructed").await;
        assert_eq!(status, 200);
        assert_eq!(drill["card"]["id"], json!("core-001"));
    }

    #[tokio::test]
    async fn r2895000_each_card_answers_the_95_percent_ranges_of_its_win_rate_and_adjusted_rate() {
        use jackioh_engine::wire::stats::wilson_interval;

        let app = test_app().await;
        let mut records = Vec::new();
        // Constructed: core-001 seen in 60 games (48 won) and never drawn in 60 (30 won); core-002 seen in all.
        let all = vec!["core-001", "core-002", "core-003", "core-004"];
        let without = vec!["core-002", "core-003", "core-004"];
        records.extend(games("seen-won", "bo1", all.clone(), true, 48));
        records.extend(games("seen-lost", "bo1", all.clone(), false, 12));
        records.extend(games("unseen-won", "bo1", without.clone(), true, 30));
        records.extend(games("unseen-lost", "bo1", without.clone(), false, 30));
        // All Random: core-001 seen in 10 games (all won), never drawn in 30.
        records.extend(games("random-seen", "random", all.clone(), true, 10));
        records.extend(games("random-unseen", "random", without.clone(), false, 30));
        insert_records(&app, records).await;
        let range = |card: &Value, low: &str, high: &str| (card[low].as_f64(), card[high].as_f64());

        let (status, _, constructed) =
            get_public(&app, "/api/stats/cards?source=live&mode=constructed").await;
        assert_eq!(status, 200);
        let a = card(&constructed, "core-001");
        // The win rate's is Wilson's, over every deck that held it: 78 wins in 120 games.
        let (low, high) = wilson_interval(78, 120);
        assert_eq!(range(a, "winRateLow", "winRateHigh"), (Some(low), Some(high)));
        // The adjusted rate's is its posterior: ± z · √(weight · SE²) about it, for 60 games a side.
        let adjusted = a["adjustedWinRate"].as_f64().expect("both sides have games");
        let prior = 0.05_f64 * 0.05;
        let noise = 0.25 * (1.0 / 60.0 + 1.0 / 60.0);
        let half = 1.96 * (prior / (prior + noise) * noise).sqrt();
        let (from, to) = range(a, "adjustedLow", "adjustedHigh");
        assert!((adjusted - from.unwrap() - half).abs() < 1e-9, "{from:?}");
        assert!((to.unwrap() - adjusted - half).abs() < 1e-9, "{to:?}");
        // R3070000: always seen, it is read against the average deck, its range its seen games' alone; no
        // game at all, neither range.
        let b = card(&constructed, "core-002");
        assert!(b["winRateLow"].as_f64().is_some());
        let always = b["adjustedWinRate"]
            .as_f64()
            .expect("read against the average deck");
        let held_noise = 0.25 / 120.0;
        let held_half = 1.96 * (prior / (prior + held_noise) * held_noise).sqrt();
        let (from, to) = range(b, "adjustedLow", "adjustedHigh");
        assert!((always - from.unwrap() - held_half).abs() < 1e-9, "{from:?}");
        assert!((to.unwrap() - always - held_half).abs() < 1e-9, "{to:?}");
        let unheld = card(&constructed, "core-006");
        assert_eq!(unheld["winRate"], Value::Null);
        assert_eq!(range(unheld, "winRateLow", "winRateHigh"), (None, None));

        // R2777002: under All Random the win rate's range counts only the seen games, as the rate does.
        let (_, _, random) = get_public(&app, "/api/stats/cards?source=live&mode=random").await;
        let (low, high) = wilson_interval(10, 10);
        assert_eq!(
            range(card(&random, "core-001"), "winRateLow", "winRateHigh"),
            (Some(low), Some(high))
        );
    }
}

mod r1583000_play_rates {
    use super::*;
    use std::time::Duration;

    use jackioh_server::config::{PLAY_RATES_MIN_GAMES, PLAY_RATES_REFRESH_SECONDS};

    const PATH: &str = "/api/stats/play-rates";

    /// How long the server keeps the play rates, in milliseconds.
    fn refresh_ms() -> u64 {
        u64::try_from(PLAY_RATES_REFRESH_SECONDS).expect("the refresh is not negative") * 1000
    }

    /// A live game on the current patch, p1 holding core-001 and core-002, p2 core-003 and core-004,
    /// each seat playing the cards given.
    fn game(id: &str, p1_played: Vec<&'static str>, p2_played: Vec<&'static str>) -> Value {
        make_game_record(
            id,
            RecordOptions {
                source: Some("live"),
                p1_played: Some(p1_played),
                p2_played: Some(p2_played),
                ..RecordOptions::default()
            },
        )
    }

    /// The rate the response lists for `card`, if it lists it.
    fn rate_of(data: &Value, card: &str) -> Option<f64> {
        find(&data["cards"], "id", &json!(card)).and_then(|stat| stat["rate"].as_f64())
    }

    /// `PLAY_RATES_MIN_GAMES` games, so each of the four cards is held by just enough seats: p1 plays
    /// core-001 in every other one, and p2 plays both its cards in each.
    async fn base(app: &App) -> usize {
        let n = count(PLAY_RATES_MIN_GAMES);
        let records = (0..n)
            .map(|i| {
                let p1_played = if i % 2 == 0 { vec!["core-001"] } else { vec![] };
                game(&format!("pr-{i}"), p1_played, vec!["core-003", "core-004"])
            })
            .collect();
        insert_records(app, records).await;
        n
    }

    #[tokio::test]
    async fn r1583000_a_cards_rate_is_the_share_of_human_live_seats_that_played_it() {
        let app = test_app().await;
        let n = base(&app).await;

        let (status, headers, data) = get_public(&app, PATH).await;
        assert_eq!(status, 200);
        assert_eq!(
            cache_control(&headers),
            Some(format!("public, max-age={PLAY_RATES_REFRESH_SECONDS}").as_str())
        );
        assert_eq!(data["decks"], json!(2 * n));
        let listed: Vec<(&str, f64)> = data["cards"]
            .as_array()
            .expect("cards is a list")
            .iter()
            .map(|card| {
                (
                    card["id"].as_str().expect("a card id"),
                    card["rate"].as_f64().expect("a rate"),
                )
            })
            .collect();
        // In catalog-id order; a card held but never played is listed at 0.
        assert_eq!(
            listed,
            vec![
                ("core-001", 0.25),
                ("core-002", 0.0),
                ("core-003", 0.5),
                ("core-004", 0.5),
            ]
        );

        // Counts and rates only: nothing names a game or a player.
        let keys = |value: &Value| {
            let mut keys: Vec<String> = value.as_object().expect("an object").keys().cloned().collect();
            keys.sort();
            keys
        };
        assert_eq!(keys(&data), vec!["cards", "decks"]);
        for card in data["cards"].as_array().expect("cards is a list") {
            assert_eq!(keys(card), vec!["id", "rate"]);
        }
    }

    #[tokio::test]
    async fn r1583000_only_human_seats_of_live_games_count_on_every_patch_and_mode() {
        let app = test_app().await;
        base(&app).await;
        let dev = make_game_record(
            "pr-dev",
            RecordOptions {
                source: Some("dev"),
                p1_played: Some(vec!["core-001"]),
                p2_played: Some(vec![]),
                ..RecordOptions::default()
            },
        );
        let mut bot = game("pr-bot", vec!["core-001"], vec!["core-003"]);
        bot["pilots"]["p2"] = json!("ai");
        let older = make_game_record(
            "pr-older",
            RecordOptions {
                source: Some("live"),
                mode: Some("bo3"),
                patch: Some("v0.0.0-older".to_string()),
                p1_played: Some(vec!["core-001"]),
                p2_played: Some(vec![]),
                ..RecordOptions::default()
            },
        );
        insert_records(&app, vec![dev, bot, older]).await;

        let (_, _, data) = get_public(&app, PATH).await;
        // The base's 40 seats, the bot game's human seat and both seats of the older Conquest game;
        // neither development seat, nor the bot's.
        assert_eq!(data["decks"], json!(43));
        assert_eq!(rate_of(&data, "core-001"), Some(12.0 / 43.0));
        assert_eq!(rate_of(&data, "core-003"), Some(20.0 / 43.0));
    }

    #[tokio::test(start_paused = true)]
    async fn r1583000_a_card_is_listed_once_play_rates_min_games_seats_held_it() {
        let app = test_app().await;
        let n = count(PLAY_RATES_MIN_GAMES);

        let (status, _, empty) = get_public(&app, PATH).await;
        assert_eq!(status, 200);
        assert_eq!(empty, json!({ "decks": 0, "cards": [] }));

        tokio::time::advance(Duration::from_millis(refresh_ms())).await;
        let records = (0..n - 1)
            .map(|i| game(&format!("pr-short-{i}"), vec!["core-001"], vec![]))
            .collect();
        insert_records(&app, records).await;
        let (_, _, short) = get_public(&app, PATH).await;
        assert_eq!(short["decks"], json!(2 * (n - 1)));
        assert_eq!(short["cards"], json!([]));

        tokio::time::advance(Duration::from_millis(refresh_ms())).await;
        insert_records(&app, vec![game("pr-short-last", vec!["core-001"], vec![])]).await;
        let (_, _, held) = get_public(&app, PATH).await;
        assert_eq!(
            ids_of(&held),
            vec!["core-001", "core-002", "core-003", "core-004"]
        );
        assert_eq!(rate_of(&held, "core-001"), Some(0.5));
    }

    #[tokio::test(start_paused = true)]
    async fn r1583000_rates_are_kept_for_the_refresh_interval_then_computed_again() {
        let app = test_app().await;
        let n = base(&app).await;

        let (_, _, first) = get_public(&app, PATH).await;
        insert_records(&app, vec![game("pr-late", vec!["core-001"], vec![])]).await;
        let (_, _, again) = get_public(&app, PATH).await;
        assert_eq!(again, first);

        tokio::time::advance(Duration::from_millis(refresh_ms() - 1)).await;
        let (_, _, within) = get_public(&app, PATH).await;
        assert_eq!(within, first);

        tokio::time::advance(Duration::from_millis(1)).await;
        let (_, _, after) = get_public(&app, PATH).await;
        assert_eq!(after["decks"], json!(2 * n + 2));
    }
}

mod r1994000_game_figures {
    use super::*;

    /// `count` records of `source`, piloted `pilots`, p1 winning those `p1_wins` picks.
    fn games(
        prefix: &str,
        count: usize,
        source: &'static str,
        pilots: [&'static str; 2],
        p1_wins: impl Fn(usize) -> bool,
    ) -> Vec<Value> {
        (1..=count)
            .map(|i| {
                make_game_record(
                    &format!("{prefix}-{i}"),
                    RecordOptions {
                        source: Some(source),
                        winner: Some(if p1_wins(i) { "p1" } else { "p2" }),
                        pilots: Some(pilots),
                        ..RecordOptions::default()
                    },
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn r1994000_get_api_stats_games_is_provisional_below_1000_live_games_and_live_at_1000() {
        let short = count(PUBLIC_STATS_MIN_LIVE_GAMES) - 1;
        let app = test_app().await;
        let mut records = games("live", short, "live", ["human", "human"], |i| i % 2 == 0);
        records.extend(games("dev", 50, "dev", ["ai", "ai"], |_| true));
        insert_records(&app, records).await;

        let (status, headers, data) = get_public(&app, "/api/stats/games").await;
        assert_eq!(status, 200);
        assert_eq!(
            cache_control(&headers),
            Some(format!("public, max-age={CARD_STATS_CACHE_TTL_SECONDS}").as_str())
        );
        assert_eq!(data["patch"], json!(current_patch()));
        assert_eq!(data["source"], "provisional");
        assert_eq!(data["sourceLabel"], "AI games + live games (provisional)");
        assert_eq!(data["minSample"], json!(CARD_STATS_MIN_SAMPLE));
        // The AI games pad the window below the gate, as they pad the cards' (1049 games).
        assert_eq!(data["totalGames"], json!(short + 50));
        assert_eq!(data["figures"]["games"], json!(short + 50));
        assert_eq!(data["figures"]["firstWins"], json!(short / 2 + 50));
        assert_eq!(data["figures"]["turnQuartiles"], json!([5, 5, 5]));
        assert_eq!(data["figures"]["reasons"]["hero-death"], json!(short + 50));
        // These records carry none of R1256000's counts: unknown, not zero.
        assert_eq!(data["figures"]["counted"], json!(0));
        assert_eq!(data["figures"]["fatigueShare"], Value::Null);

        let app = test_app().await;
        let gate = count(PUBLIC_STATS_MIN_LIVE_GAMES);
        let mut records = games("live", gate, "live", ["human", "human"], |i| i % 2 == 0);
        records.extend(games("dev", 100, "dev", ["ai", "ai"], |_| true));
        insert_records(&app, records).await;
        let (_, _, data) = get_public(&app, "/api/stats/games").await;
        assert_eq!(data["source"], "live");
        assert_eq!(data["sourceLabel"], "Live games");
        assert_eq!(data["totalGames"], json!(gate));
        assert_eq!(data["figures"]["games"], json!(gate));
        assert_eq!(data["figures"]["firstWins"], json!(gate / 2));
    }

    #[tokio::test]
    async fn r1994000_get_api_stats_games_splits_games_against_a_ladder_bot_by_pilot_pair() {
        let app = test_app().await;
        let mut records = games("people", 30, "live", ["human", "human"], |i| i % 3 == 0);
        records.extend(games("bot-second", 20, "live", ["human", "ai"], |_| false));
        records.extend(games("bot-first", 10, "live", ["ai", "human"], |_| true));
        insert_records(&app, records).await;

        let (status, _, data) = get_public(&app, "/api/stats/games").await;
        assert_eq!(status, 200);
        // A game is read whole, whoever piloted it: all sixty count.
        assert_eq!(data["totalGames"], json!(60));
        assert_eq!(data["figures"]["games"], json!(60));
        assert_eq!(data["figures"]["firstWins"], json!(20));
        let pairs: Vec<(Value, Value, Value)> = data["pairs"]
            .as_array()
            .expect("pairs is a list")
            .iter()
            .map(|pair| {
                (
                    pair["pilots"].clone(),
                    pair["figures"]["games"].clone(),
                    pair["figures"]["firstWins"].clone(),
                )
            })
            .collect();
        assert_eq!(
            pairs,
            [
                (json!(["human", "human"]), json!(30), json!(10)),
                (json!(["human", "ai"]), json!(20), json!(0)),
                (json!(["ai", "human"]), json!(10), json!(10)),
            ]
        );

        // A mode nobody played has no figures.
        let (_, _, conquest) = get_public(&app, "/api/stats/games?mode=bo3").await;
        assert_eq!(conquest["mode"], "bo3");
        assert_eq!(conquest["figures"], Value::Null);
        assert_eq!(conquest["pairs"], json!([]));
        assert_eq!(conquest["totalGames"], json!(0));
    }
}

// R2089000: `GET /api/stats/cards` lists the Created cards, tokens among them, apart from the deck
// cards and under the same filters.
mod r2089000_created_card_stats {
    use super::*;

    #[tokio::test]
    async fn r2089000_get_api_stats_cards_lists_created_cards_apart_from_the_deck_table() {
        let app = test_app().await;
        // p1 plays dealt core-001 in both games with Rush tokens and Created card.
        let records = vec![
            make_game_record(
                "live-1",
                RecordOptions {
                    p1_played: Some(vec!["core-001"]),
                    p1_played_radiant: Some(vec![false]),
                    p1_created_used: Some(json!([
                        { "card": "core-t-rush", "turn": 2, "radiant": false },
                        { "card": "core-001", "turn": 3, "radiant": true },
                    ])),
                    winner: Some("p1"),
                    ..RecordOptions::default()
                },
            ),
            make_game_record(
                "live-2",
                RecordOptions {
                    p1_played: Some(vec!["core-001"]),
                    p1_played_radiant: Some(vec![false]),
                    p1_created_unused: Some(vec!["core-t-rush"]),
                    winner: Some("p2"),
                    ..RecordOptions::default()
                },
            ),
        ];
        insert_records(&app, records).await;

        let (status, _, data) = get_public(&app, "/api/stats/cards?source=live").await;
        assert_eq!(status, 200);
        // The deck table counts the dealt copies alone and holds no token.
        let card001 = find(&data["cards"], "id", &json!("core-001")).expect("core-001 is listed");
        assert_eq!(card001["playedGames"], json!(2));
        assert_eq!(card001["playedRadiantGames"], json!(0));
        assert!(find(&data["cards"], "id", &json!("core-t-rush")).is_none());

        let created = data["created"].as_array().expect("a Created table");
        let ids: Vec<&str> = created.iter().filter_map(|row| row["id"].as_str()).collect();
        // Both are short of the floor, so the one with more games comes first.
        assert_eq!(ids, ["core-t-rush", "core-001"]);
        let rush = &created[0];
        assert_eq!(rush["name"], json!(jackioh_cards::CATALOG["core-t-rush"].name));
        assert_eq!(rush["games"], json!(2));
        assert_eq!(rush["createdPlayedGames"], json!(1));
        assert_eq!(rush["createdPlayedWinRate"].as_f64(), Some(1.0));
        assert_eq!(rush["createdNotPlayedGames"], json!(1));
        assert_eq!(rush["createdNotPlayedWinRate"].as_f64(), Some(0.0));
        assert_eq!(rush["createdPlayedNormalGames"], json!(1));
        assert_eq!(rush["createdPlayedRadiantGames"], json!(0));
        assert!(rush["createdPlayedRadiantWinRate"].is_null());
        // R654: the delta waits for the floor on both sides.
        assert!(rush["createdPlayedDelta"].is_null());
        assert_eq!(rush["hasEnoughGames"], json!(false));
        assert!(rush["window"]["from"].is_string());
        let made001 = &created[1];
        assert_eq!(made001["createdPlayedGames"], json!(1));
        assert_eq!(made001["createdPlayedRadiantGames"], json!(1));
        assert_eq!(made001["createdNotPlayedGames"], json!(0));

        // `?card=` filters both tables alike.
        let (status, _, only) = get_public(&app, "/api/stats/cards?source=live&card=core-t-rush").await;
        assert_eq!(status, 200);
        assert_eq!(only["cards"], json!([]));
        let ids: Vec<&str> = only["created"]
            .as_array()
            .expect("a Created table")
            .iter()
            .filter_map(|row| row["id"].as_str())
            .collect();
        assert_eq!(ids, ["core-t-rush"]);
    }
}
