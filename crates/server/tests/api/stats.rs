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
/// R1310004: the end-to-end dev account's bearer token (`e2e-econ`), which reads every patch (R4677000).
const DEV_TOKEN: &str = "e2e-token-econ";

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
    /// R4552000: p1's turn rows (`swingTurns`), absent by default as from a record written before them.
    p1_swing_turns: Option<Value>,
    /// R4672000: p1's plays' variants (`playedVariants`), beside `played`.
    p1_played_variants: Option<Value>,
    /// R4672002: p1's picks (`picks`), each Discover-like choice a card of its asked.
    p1_picks: Option<Value>,
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
        // R5054000: a record written since also lists the cards its seat held Radiant, each once; a play from the
        // hand on its Radiant face was held so.
        let mut held: Vec<&str> = Vec::new();
        for (card, _) in played.iter().zip(&flags).filter(|(_, radiant)| **radiant) {
            if !held.contains(card) {
                held.push(card);
            }
        }
        val["heldRadiant"] = json!(held);
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
    if let Some(rows) = options.p1_swing_turns {
        p1["swingTurns"] = rows;
    }
    if let Some(variants) = options.p1_played_variants {
        p1["playedVariants"] = variants;
    }
    if let Some(picks) = options.p1_picks {
        p1["picks"] = picks;
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
    async fn r3185000_get_api_stats_cards_sends_only_the_figures_the_client_reads() {
        let app = test_app().await;
        // 50 games: core-001 played in 30 (21 won) and drawn but not played in 20 (10 won); core-002 never played.
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

        // Every row carries these keys and no others: no drawn, played, face, drawn-not-played or never-drawn
        // breakdown and no played or drawn delta, which no client reads.
        for row in data["cards"].as_array().expect("the cards") {
            let mut keys: Vec<&str> = row
                .as_object()
                .expect("a row")
                .keys()
                .map(String::as_str)
                .collect();
            keys.sort_unstable();
            assert_eq!(
                keys,
                [
                    "adjustedHigh",
                    "adjustedLow",
                    "adjustedWinRate",
                    "cost",
                    "decisiveness",
                    "games",
                    "hasEnoughGames",
                    "id",
                    "name",
                    "playRate",
                    "rarity",
                    "set",
                    "tier",
                    "winRate",
                    "winRateHigh",
                    "winRateLow",
                    "window",
                ],
                "{}",
                row["id"]
            );
        }
        let card001 = find(&data["cards"], "id", &json!("core-001")).expect("core-001 is listed");
        assert_eq!(card001["games"], json!(50));
        assert_eq!(card001["winRate"].as_f64(), Some(31.0 / 50.0));
        assert_eq!(card001["hasEnoughGames"], json!(true));
        // Its adjusted rate and tier are still read off the tallies the row no longer carries.
        assert!(card001["adjustedWinRate"].as_f64().is_some());
        assert!(card001["tier"].is_string());
    }

    #[tokio::test]
    async fn r3185000_r2427000_the_adjusted_rate_and_tier_still_read_the_drawn_games_against_the_never_drawn_unsent()
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
        // R2777000 at R3022000's prior field (one card has the floor on both sides, too few to measure): 50%
        // plus the drawn delta, kept as far as the games on each side back it.
        let adjusted = |seen: f64, never: f64, delta: f64| {
            let prior = 0.05_f64 * 0.05;
            0.5 + prior / (prior + 0.25 * (1.0 / seen + 1.0 / never)) * delta
        };
        // core-006 is drawn in the 20 games p1 won and never in the 20 it lost: a drawn delta of 100 points.
        let six = card("core-006");
        let rate = six["adjustedWinRate"].as_f64().expect("core-006 has both sides");
        assert!((rate - adjusted(20.0, 20.0, 1.0)).abs() < 1e-9, "{rate}");
        // Above the band, but its range still reaches 50%: A, not S.
        assert_eq!(six["tier"], json!("A"));
        // core-007 is drawn in 19 games, all lost, and never in 21, 20 of them won.
        let seven = card("core-007");
        let rate = seven["adjustedWinRate"]
            .as_f64()
            .expect("core-007 has both sides");
        assert!((rate - adjusted(19.0, 21.0, -20.0 / 21.0)).abs() < 1e-9, "{rate}");
        // Below the band, but its range still reaches 50%: D, not F.
        assert_eq!(seven["tier"], json!("D"));
        // R3185000: none of the tallies they were read from is sent.
        for row in [six, seven] {
            for gone in ["drawnGames", "notDrawnGames", "notDrawnWinRate", "drawnDelta"] {
                assert!(row.get(gone).is_none(), "{gone} is sent");
            }
        }
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

    /// R3911000: a banned account drops off GET /api/stats/players and its search,
    /// and is restored when unbanned without modifying stored stats.
    #[tokio::test]
    async fn r3911000_get_api_stats_players_excludes_banned_profile_and_restores_when_unbanned() {
        let app = test_app().await;
        let alice_id = seed_profile(&app, "u-alice-banned", "Alice_Banned").await;
        let bob_id = seed_profile(&app, "u-bob-active", "Bob_Active").await;

        put_player_stats(&app, &alice_id, json!({ "games": 200, "wins": 150 }), false, 1000).await;
        put_player_stats(&app, &bob_id, json!({ "games": 100, "wins": 70 }), false, 2000).await;

        let (status, _, data) = get_public(&app, "/api/stats/players").await;
        assert_eq!(status, 200);
        let ids: Vec<&str> = data["players"]
            .as_array()
            .expect("players list")
            .iter()
            .map(|p| p["profileId"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(ids, [alice_id.as_str(), bob_id.as_str()]);

        // Ban Alice
        {
            let mut tx = app.db.begin(Some(&alice_id)).await.expect("tx");
            tx.profiles_set_status(&alice_id, from(json!("banned")))
                .await
                .expect("set banned");
            tx.commit().await.expect("commit");
        }

        // Alice is excluded from /api/stats/players
        let (_, _, banned_data) = get_public(&app, "/api/stats/players").await;
        let banned_ids: Vec<&str> = banned_data["players"]
            .as_array()
            .expect("players list")
            .iter()
            .map(|p| p["profileId"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(banned_ids, [bob_id.as_str()]);

        // Alice is excluded from search
        let (_, _, search_data) = get_public(&app, "/api/stats/players?search=Alice").await;
        assert_eq!(search_data["players"], json!([]));

        // Unban Alice
        {
            let mut tx = app.db.begin(Some(&alice_id)).await.expect("tx");
            tx.profiles_set_status(&alice_id, from(json!("active")))
                .await
                .expect("set active");
            tx.commit().await.expect("commit");
        }

        // Alice is restored
        let (_, _, restored_data) = get_public(&app, "/api/stats/players").await;
        let restored_ids: Vec<&str> = restored_data["players"]
            .as_array()
            .expect("players list")
            .iter()
            .map(|p| p["profileId"].as_str().unwrap_or_default())
            .collect();
        assert_eq!(restored_ids, [alice_id.as_str(), bob_id.as_str()]);
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
        // R3067000: another pilot's report is cut from the snapshot the first read loaded.
        let (_, _, humans) = get_public(&app, "/api/stats/cards?pilot=human").await;
        assert_eq!(humans["totalGames"], json!(1));

        tokio::time::advance(Duration::from_millis(ttl_ms() - 1)).await;
        let (_, _, within) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(within, first);

        tokio::time::advance(Duration::from_millis(1)).await;
        let (_, _, after) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(after["totalGames"], json!(2));
        assert_eq!(after["gate"]["liveGames"], json!(2));
        assert_eq!(games_of(&after, "core-001"), json!(2));
        // R3067000: the snapshot is read again with it, so another pilot's report counts both records.
        let (_, _, humans) = get_public(&app, "/api/stats/cards?pilot=human").await;
        assert_eq!(humans["totalGames"], json!(2));
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
    use std::time::Duration;

    use jackioh_server::api::catalog::load_patch_versions;
    use jackioh_server::db::store::{Db, StoreError};

    /// What the fake store was asked to read: every `statsTotals.load`, and among them the loads of
    /// every patch's totals (R3067000), which it charges as `statsTotals.loadAllPatches` too. R5174000:
    /// and among those the reads no list of patches bounds, `statsTotals.loadWholeTable`.
    #[derive(Default)]
    struct Loads {
        all: AtomicUsize,
        full: AtomicUsize,
        whole: AtomicUsize,
    }

    /// Counts the fake store's totals loads from here on, through its `on_call` hook.
    async fn count_loads(app: &App) -> Arc<Loads> {
        let loads = Arc::new(Loads::default());
        let counter = Arc::clone(&loads);
        match &app.db {
            Db::Fake(data) => {
                data.lock().await.on_call = Some(Arc::new(move |method: &str| -> Result<(), StoreError> {
                    match method {
                        "statsTotals.load" => counter.all.fetch_add(1, Ordering::SeqCst),
                        "statsTotals.loadAllPatches" => counter.full.fetch_add(1, Ordering::SeqCst),
                        "statsTotals.loadWholeTable" => counter.whole.fetch_add(1, Ordering::SeqCst),
                        _ => 0,
                    };
                    Ok(())
                }));
            }
            Db::Pg(_) => panic!("the API tests run on the fake store"),
        }
        loads
    }

    /// The loads one request to `path` made, which answers 200, as (every load, those of every
    /// patch's totals); the counts start again after it.
    async fn reads_of(app: &Arc<App>, loads: &Loads, path: &str) -> (usize, usize) {
        let (status, _, _) = get_public(app, path).await;
        assert_eq!(status, 200, "{path}");
        (
            loads.all.swap(0, Ordering::SeqCst),
            loads.full.swap(0, Ordering::SeqCst),
        )
    }

    /// The loads one request to `path` made.
    async fn loads_of(app: &Arc<App>, loads: &Loads, path: &str) -> usize {
        reads_of(app, loads, path).await.0
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

        // R3067000: the first view loads the snapshot of every patch's totals, however many patches
        // ship, and the current patch's drills. Each view after it misses its history alone, cut
        // from that snapshot, so only its drills load.
        for (path, made) in [
            ("/api/stats/cards/core-001", 2),
            ("/api/stats/cards/core-001?mode=bo1", 1),
            ("/api/stats/cards/core-001?mode=bo3", 1),
            ("/api/stats/cards/core-001?mode=random", 1),
        ] {
            assert_eq!(loads_of(&app, &loads, path).await, made, "{path}");
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
        // R4677000: the newest past patches, which anyone may read, listed oldest first.
        let mut past = versions.iter().rev().filter(|version| **version != current);
        let short = past.next().expect("a past patch").clone();
        let old = past.next().expect("a second past patch").clone();

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

    /// The cache's time to live, in milliseconds.
    fn ttl_ms() -> u64 {
        u64::try_from(CARD_STATS_CACHE_TTL_SECONDS).expect("the TTL is not negative") * 1000
    }

    /// A past shipped patch, which is not the current one: the newest, which anyone may read (R4677000).
    async fn past_patch() -> String {
        let current = current_patch();
        load_patch_versions()
            .await
            .into_iter()
            .rev()
            .find(|version| *version != current)
            .expect("a past patch")
    }

    /// Live games of three modes, a development game and a past patch's games: the records the first
    /// R3067000 test reads under every key.
    fn mixed_records(past: &str) -> Vec<Value> {
        let current = current_patch();
        let mut records: Vec<Value> = (0..3)
            .map(|i| game(format!("mix-now-{i}"), &current, "bo1", "p1"))
            .collect();
        records.push(game("mix-now-bo3".to_string(), &current, "bo3", "p2"));
        records.push(game("mix-now-random".to_string(), &current, "random", "p1"));
        records.push(make_game_record(
            "mix-now-dev",
            RecordOptions {
                source: Some("dev"),
                patch: Some(current),
                ..RecordOptions::default()
            },
        ));
        records.extend((0..2).map(|i| game(format!("mix-old-{i}"), past, "bo1", "p2")));
        records
    }

    #[tokio::test(start_paused = true)]
    async fn r3067000_distinct_known_keys_within_one_ttl_cost_one_load_of_every_patch() {
        let past = past_patch().await;
        let app = test_app().await;
        insert_records(&app, mixed_records(&past)).await;
        let loads = count_loads(&app).await;

        let mut paths: Vec<String> = Vec::new();
        for source in ["all", "live", "dev"] {
            for pilot in ["merged", "human", "ai"] {
                for mode in ["all", "bo1", "bo3", "random", "constructed"] {
                    paths.push(format!(
                        "/api/stats/cards?source={source}&pilot={pilot}&mode={mode}"
                    ));
                }
            }
        }
        assert!(paths.len() >= 30, "{} keys", paths.len());
        for mode in ["all", "bo1"] {
            paths.push(format!("/api/stats/cards?patch={past}&mode={mode}"));
        }
        for source in ["all", "live", "dev"] {
            for mode in ["all", "bo3"] {
                paths.push(format!("/api/stats/games?source={source}&mode={mode}"));
            }
        }
        // One key is held back to be asked for after the snapshot has aged.
        let late = paths.pop().expect("a key");

        // The first request loads every patch's totals once; no other key loads them again.
        assert_eq!(reads_of(&app, &loads, &paths[0]).await, (1, 1), "{}", paths[0]);
        let mut bodies: Vec<Value> = Vec::new();
        for path in &paths {
            let (status, _, body) = get_public(&app, path).await;
            assert_eq!(status, 200, "{path}");
            bodies.push(body);
        }
        assert_eq!(
            (
                loads.all.load(Ordering::SeqCst),
                loads.full.load(Ordering::SeqCst)
            ),
            (0, 0),
            "{} keys",
            paths.len()
        );

        // A card's drill-down reads its history off the snapshot too: only its current patch's drills load.
        for path in [
            "/api/stats/cards/core-001",
            "/api/stats/cards/core-001?mode=bo1&pilot=ai",
        ] {
            assert_eq!(reads_of(&app, &loads, path).await, (1, 0), "{path}");
        }

        // R654: whatever key it is read under, a body is what a load of its own gives from the same records.
        let sampled = [
            "/api/stats/cards?source=all&pilot=merged&mode=all".to_string(),
            "/api/stats/cards?source=live&pilot=human&mode=bo3".to_string(),
            format!("/api/stats/cards?patch={past}&mode=bo1"),
            "/api/stats/games?source=dev&mode=all".to_string(),
        ];
        for path in &sampled {
            let index = paths.iter().position(|held| held == path).expect("a sampled key");
            let reference = test_app().await;
            insert_records(&reference, mixed_records(&past)).await;
            let (_, _, own) = get_public(&reference, path).await;
            assert_eq!(bodies[index], own, "{path}");
        }

        // Within the TTL a new key is cut from the snapshot already held; at the TTL it is loaded again.
        tokio::time::advance(Duration::from_millis(ttl_ms() - 1)).await;
        assert_eq!(reads_of(&app, &loads, &late).await, (0, 0), "{late}");
        tokio::time::advance(Duration::from_millis(1)).await;
        assert_eq!(reads_of(&app, &loads, &late).await, (1, 1), "{late}");
        assert_eq!(reads_of(&app, &loads, &paths[0]).await, (0, 0), "{}", paths[0]);
    }

    // R5174000: the cards, the game figures, a drill-down and the win-rate snapshot every match starts with
    // read only the versions the snapshot keeps as filed, so their load never goes over every version's card
    // totals. The play versions' table reads the older ones folded, once, beside the cards' snapshot and with
    // its time; the Hall of Fame and the cards then share it, and every figure is what an app that read the
    // older versions first answers.
    #[tokio::test(start_paused = true)]
    async fn r5174000_only_the_play_versions_read_every_versions_card_totals() {
        let past = past_patch().await;
        let oldest = load_patch_versions()
            .await
            .into_iter()
            .next()
            .expect("a first patch");
        let mut records = mixed_records(&past);
        records.extend((0..2).map(|i| game(format!("mix-oldest-{i}"), &oldest, "bo1", "p1")));
        let app = test_app().await;
        insert_records(&app, records.clone()).await;
        let loads = count_loads(&app).await;

        for path in [
            "/api/stats/cards?mode=all",
            "/api/stats/games?mode=all",
            "/api/stats/cards/core-001",
        ] {
            let (status, _, _) = get_public(&app, path).await;
            assert_eq!(status, 200, "{path}");
        }
        let _table = jackioh_server::api::stats::win_rate_snapshot(&app).await;
        assert_eq!(
            loads.whole.swap(0, Ordering::SeqCst),
            0,
            "a read of the versions kept as filed went over every version's card totals"
        );
        assert_eq!(loads.full.swap(0, Ordering::SeqCst), 1);
        loads.all.store(0, Ordering::SeqCst);

        assert_eq!(reads_of(&app, &loads, "/api/stats/versions").await, (1, 1));
        assert_eq!(loads.whole.swap(0, Ordering::SeqCst), 1);
        for path in [
            "/api/stats/hall-of-fame",
            "/api/stats/versions?mode=bo1",
            "/api/stats/cards?mode=bo1",
            "/api/stats/games?mode=bo1",
        ] {
            assert_eq!(reads_of(&app, &loads, path).await, (0, 0), "{path}");
        }
        assert_eq!(loads.whole.load(Ordering::SeqCst), 0);

        let reference = test_app().await;
        insert_records(&reference, records).await;
        for path in [
            "/api/stats/versions",
            "/api/stats/hall-of-fame",
            "/api/stats/cards?mode=all",
            "/api/stats/games?mode=all",
        ] {
            let (_, _, theirs) = get_public(&reference, path).await;
            let (_, _, ours) = get_public(&app, path).await;
            assert_eq!(ours, theirs, "{path}");
        }

        // At the TTL both are dropped together: the cards load the kept versions again, alone.
        tokio::time::advance(Duration::from_millis(ttl_ms())).await;
        assert_eq!(reads_of(&app, &loads, "/api/stats/cards?mode=all").await, (1, 1));
        assert_eq!(loads.whole.load(Ordering::SeqCst), 0);
    }

    // R4675001: the play versions' table is cut from the snapshot the cards read, once per `?pilot=` and
    // `?mode=`, so the whole history costs one load of every patch's totals however many keys ask.
    #[tokio::test(start_paused = true)]
    async fn r4675001_the_route_reads_every_patch_once_and_shares_the_snapshot_with_the_cards() {
        let past = past_patch().await;
        let app = test_app().await;
        insert_records(&app, mixed_records(&past)).await;
        let loads = count_loads(&app).await;

        assert_eq!(reads_of(&app, &loads, "/api/stats/versions").await, (1, 1));
        for path in [
            "/api/stats/versions",
            "/api/stats/versions?mode=bo1&pilot=human",
            "/api/stats/versions?mode=constructed&pilot=ai",
            "/api/stats/cards",
        ] {
            assert_eq!(reads_of(&app, &loads, path).await, (0, 0), "{path}");
        }
        // A key's table is what a load of its own gives from the same records.
        let (_, _, kept) = get_public(&app, "/api/stats/versions?mode=bo1&pilot=human").await;
        let reference = test_app().await;
        insert_records(&reference, mixed_records(&past)).await;
        let (_, _, own) = get_public(&reference, "/api/stats/versions?mode=bo1&pilot=human").await;
        assert_eq!(kept, own);

        // At the TTL the snapshot is loaded again.
        tokio::time::advance(Duration::from_millis(ttl_ms())).await;
        assert_eq!(reads_of(&app, &loads, "/api/stats/versions").await, (1, 1));
    }

    // R5040000: Major patch running's table is cut from the same snapshot, kept once per `?scope=` beside
    // `?pilot=` and `?mode=`.
    #[tokio::test(start_paused = true)]
    async fn r5040000_each_scope_is_kept_once_and_cut_from_the_same_snapshot() {
        let past = past_patch().await;
        let app = test_app().await;
        insert_records(&app, mixed_records(&past)).await;
        let loads = count_loads(&app).await;

        assert_eq!(
            reads_of(&app, &loads, "/api/stats/versions?scope=major").await,
            (1, 1)
        );
        for path in [
            "/api/stats/versions?scope=major",
            "/api/stats/versions",
            "/api/stats/versions?scope=major&mode=bo1&pilot=human",
            "/api/stats/cards",
        ] {
            assert_eq!(reads_of(&app, &loads, path).await, (0, 0), "{path}");
        }
        // A scope's table is what a load of its own gives from the same records.
        let (_, _, kept) = get_public(&app, "/api/stats/versions?scope=major&mode=bo1").await;
        let reference = test_app().await;
        insert_records(&reference, mixed_records(&past)).await;
        let (_, _, own) = get_public(&reference, "/api/stats/versions?scope=major&mode=bo1").await;
        assert_eq!(kept, own);
        // Each scope keeps its own table.
        let (_, _, bare) = get_public(&app, "/api/stats/versions").await;
        let (_, _, major) = get_public(&app, "/api/stats/versions?scope=major").await;
        assert_ne!(bare, major);
    }

    #[tokio::test(start_paused = true)]
    async fn r3067000_distinct_unshipped_patches_cost_no_load_of_every_patch() {
        let app = test_app().await;
        let loads = count_loads(&app).await;
        let made_up = |i: usize| format!("v0.0.0-made-up-{i}");
        let mut records = vec![game("odd-now".to_string(), &current_patch(), "bo1", "p1")];
        records.extend((0..10).map(|i| game(format!("odd-{i}"), &made_up(i), "bo1", "p1")));
        insert_records(&app, records).await;

        // The first one loads what the page's own report is cut from, once, and its patch's totals; the
        // nine after it load their patch's totals alone.
        for i in 0..10 {
            let patch = made_up(i);
            let path = format!("/api/stats/cards?patch={patch}");
            let (status, _, body) = get_public(&app, &path).await;
            assert_eq!(status, 200, "{path}");
            assert_eq!(body["patch"], json!(patch));
            assert_eq!(body["totalGames"], json!(1), "{path}");
            assert_eq!(body["summary"]["activePatch"], json!(patch));
            let expected = if i == 0 { (2, 1) } else { (1, 0) };
            let made = (
                loads.all.swap(0, Ordering::SeqCst),
                loads.full.swap(0, Ordering::SeqCst),
            );
            assert_eq!(made, expected, "{path}");
        }
        // The games' page reads an unshipped patch the same way.
        let (_, _, games) = get_public(&app, &format!("/api/stats/games?patch={}", made_up(3))).await;
        assert_eq!(games["totalGames"], json!(1));
        assert_eq!(
            (
                loads.all.swap(0, Ordering::SeqCst),
                loads.full.swap(0, Ordering::SeqCst)
            ),
            (1, 0)
        );

        // The page's own report was kept by the first of them.
        assert_eq!(reads_of(&app, &loads, "/api/stats/cards").await, (0, 0));

        // R978000: an unshipped patch is still not kept.
        insert_records(&app, vec![game("odd-late".to_string(), &made_up(0), "bo1", "p1")]).await;
        let (_, _, again) = get_public(&app, &format!("/api/stats/cards?patch={}", made_up(0))).await;
        assert_eq!(again["totalGames"], json!(2));
    }

    #[tokio::test(start_paused = true)]
    async fn r3067000_an_unshipped_patch_answers_what_its_own_records_give() {
        let past = past_patch().await;
        let patch = "v0.0.0-not-shipped";
        let own: Vec<Value> = (0..3)
            .map(|i| game(format!("own-{i}"), patch, "bo1", if i == 0 { "p2" } else { "p1" }))
            .collect();

        // One app holds the shipped patches' games too; the other holds only the unshipped patch's.
        let shipped = test_app().await;
        let mut records = mixed_records(&past);
        records.extend(own.clone());
        insert_records(&shipped, records).await;
        let alone = test_app().await;
        insert_records(&alone, own).await;

        // The page's figures are the patch's own, whatever else the store holds; only its list of
        // patches counts every shipped patch's games, as the page's own report lists them.
        let path = format!("/api/stats/cards?patch={patch}");
        let (_, _, mut with) = get_public(&shipped, &path).await;
        let (_, _, mut without) = get_public(&alone, &path).await;
        assert_eq!(with["totalGames"], json!(3));
        let listed = with["patches"].take();
        without["patches"].take();
        assert_eq!(with, without);
        let (_, _, page) = get_public(&shipped, "/api/stats/cards").await;
        assert_eq!(listed, page["patches"]);
        assert_ne!(listed, json!([]));

        let path = format!("/api/stats/games?patch={patch}");
        let (_, _, with) = get_public(&shipped, &path).await;
        let (_, _, without) = get_public(&alone, &path).await;
        assert_eq!(with["totalGames"], json!(3));
        assert_eq!(with, without);
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
        // R4677000: a dev account is listed every patch.
        let (status, _, data) = request(&app, "GET", "/api/stats/cards", Some(DEV_TOKEN), None).await;
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
        // major, X or (R3526000) the newest patch, whose window holds the newest version, opens one.
        let newest_patch = &history
            .iter()
            .rfind(|step| step.tier != PatchTier::Micro)
            .expect("a major has shipped")
            .version;
        for window in patches {
            let first = &versions(window)[0];
            let step = history
                .iter()
                .find(|step| &step.version == first)
                .expect("a shipped version");
            assert!(
                step.tier == PatchTier::Major || *first == x || first == newest_patch,
                "{first} opens a window with no game played on it"
            );
        }
        // Every version is listed once, in the history's order.
        let listed: Vec<String> = patches.iter().rev().flat_map(versions).collect();
        let shipped: Vec<String> = history.iter().map(|step| step.version.clone()).collect();
        assert_eq!(listed, shipped);
    }

    #[tokio::test]
    async fn r3526000_a_patch_whose_games_are_its_micros_opens_its_window_as_does_the_newest_patch() {
        let history = jackioh_cards::patch_history();
        // A patch a later patch replaced, with a micro right after it: as v0.5.11 shipped in one
        // commit with v0.5.11b, every game was filed under the micro and none under the patch.
        let at = (0..history.len().saturating_sub(1))
            .rev()
            .find(|&at| {
                history[at].tier == PatchTier::Patch
                    && history[at + 1].tier == PatchTier::Micro
                    && history[at + 1..].iter().any(|step| step.tier != PatchTier::Micro)
            })
            .expect("a replaced patch has a micro after it");
        let (patch, micro) = (&history[at].version, &history[at + 1].version);
        let app = test_app().await;
        let records: Vec<Value> = (0..4)
            .map(|n| live(format!("micro-{n}"), micro, "core-002"))
            .collect();
        insert_records(&app, records).await;
        let (status, _, data) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(status, 200);
        let patches = data["patches"].as_array().expect("the patches are a list");
        let first = |window: &Value| window["versions"][0].as_str().expect("a version").to_string();
        let held = patches
            .iter()
            .find(|window| first(window) == *patch)
            .expect("the patch opens a window");
        assert_eq!((&held["games"], &held["liveGames"]), (&json!(4), &json!(4)));
        assert_eq!(held["versions"][1], json!(micro));
        // The newest patch opens the newest window, with no game on any of its versions yet.
        let newest_patch = &history
            .iter()
            .rfind(|step| step.tier != PatchTier::Micro)
            .expect("a major has shipped")
            .version;
        assert_eq!(first(&patches[0]), *newest_patch);
    }
}

// R4677000: anyone but a dev account reads the newest patches alone: every patch of the newest line and two
// of the line before, or the five newest, whichever run is longer. A dev account reads every patch.
mod r4677000_patches_by_reader {
    use super::*;
    use std::time::Duration;

    use jackioh_engine::wire::stats::{PatchTier, PatchWindow};
    use jackioh_server::api::stats::public_patch_count;
    use jackioh_server::config::{PUBLIC_STATS_PATCHES_LINE_BEFORE, PUBLIC_STATS_PATCHES_NEWEST};

    use crate::support::deps::{TestAppOptions, add_user, test_app_with};
    use crate::support::store::fake;

    /// `GET path` with `token`, when there is one.
    async fn get_as(app: &Arc<App>, path: &str, token: Option<&str>) -> (u16, HeaderMap, Value) {
        request(app, "GET", path, token, None).await
    }

    /// Each listed patch's name, the version that opened it, newest first.
    fn names(data: &Value) -> Vec<String> {
        data["patches"]
            .as_array()
            .expect("the patches are a list")
            .iter()
            .map(|window| window["versions"][0].as_str().expect("a version").to_string())
            .collect()
    }

    /// A version's line, its `major.minor`.
    fn line(version: &str) -> String {
        version.split('.').take(2).collect::<Vec<_>>().join(".")
    }

    /// One live game filed under `patch`.
    fn live_on(id: String, patch: &str) -> Value {
        make_game_record(
            &id,
            RecordOptions {
                source: Some("live"),
                patch: Some(patch.to_string()),
                ..RecordOptions::default()
            },
        )
    }

    /// A window of the fixture histories: its name and nothing else read.
    fn window(name: &str) -> PatchWindow {
        PatchWindow {
            versions: vec![name.to_string()],
            games: 0,
            live_games: 0,
        }
    }

    fn windows(names: &[&str]) -> Vec<PatchWindow> {
        names.iter().map(|name| window(name)).collect()
    }

    /// The shipped versions that open a window once a game is played on each, newest first, of the
    /// newest line and of the line before it.
    fn the_two_newest_lines() -> (Vec<String>, Vec<String>) {
        let history = jackioh_cards::patch_history();
        let opening: Vec<String> = history
            .iter()
            .rev()
            .filter(|step| step.tier != PatchTier::Micro)
            .map(|step| step.version.clone())
            .collect();
        let newest = line(&opening[0]);
        let current: Vec<String> = opening
            .iter()
            .take_while(|version| line(version) == newest)
            .cloned()
            .collect();
        let before_line = line(&opening[current.len()]);
        let before: Vec<String> = opening[current.len()..]
            .iter()
            .take_while(|version| line(version) == before_line)
            .cloned()
            .collect();
        (current, before)
    }

    #[test]
    fn r4677000_the_longer_run_of_the_newest_line_with_two_of_the_line_before_or_the_five_newest() {
        // Just after a major, the newest line holds one patch: the five newest are the longer run.
        let after_a_major = windows(&[
            "v0.7.0", "v0.6.3", "v0.6.2", "v0.6.1", "v0.6.0", "v0.5.1", "v0.5.0",
        ]);
        assert_eq!(1 + PUBLIC_STATS_PATCHES_LINE_BEFORE, 3);
        assert_eq!(public_patch_count(&after_a_major), PUBLIC_STATS_PATCHES_NEWEST);
        // A long newest line: all of it and the two newest of the line before.
        let long_line = windows(&[
            "v0.6.3", "v0.6.2", "v0.6.1", "v0.6.0", "v0.5.9", "v0.5.8", "v0.5.7", "v0.4.0",
        ]);
        assert_eq!(
            public_patch_count(&long_line),
            4 + PUBLIC_STATS_PATCHES_LINE_BEFORE
        );
        // A line before of one patch gives one; and nothing past the list.
        let thin_before = windows(&[
            "v0.6.5", "v0.6.4", "v0.6.3", "v0.6.2", "v0.6.1", "v0.6.0", "v0.5.0", "v0.4.0",
        ]);
        assert_eq!(public_patch_count(&thin_before), 7);
        assert_eq!(public_patch_count(&windows(&["v0.1.1", "v0.1.0"])), 2);
        assert_eq!(public_patch_count(&[]), 0);
        // A line is its major and minor: v0.10 is not v0.1.
        let tenth = windows(&[
            "v0.10.2", "v0.10.1", "v0.10.0", "v0.1.9", "v0.1.8", "v0.1.7", "v0.1.6",
        ]);
        assert_eq!(public_patch_count(&tenth), 3 + PUBLIC_STATS_PATCHES_LINE_BEFORE);
    }

    #[tokio::test]
    async fn r4677000_anyone_reads_the_newest_line_and_two_of_the_line_before_when_that_run_is_longer() {
        let (current, before) = the_two_newest_lines();
        assert!(current.len() + PUBLIC_STATS_PATCHES_LINE_BEFORE > PUBLIC_STATS_PATCHES_NEWEST);
        let app = test_app().await;
        // A game on each patch of the two lines opens its window (R1973001).
        let records: Vec<Value> = current
            .iter()
            .chain(&before)
            .enumerate()
            .map(|(n, version)| live_on(format!("line-{n}"), version))
            .collect();
        insert_records(&app, records).await;

        let mut expected = current.clone();
        expected.extend(before.iter().take(PUBLIC_STATS_PATCHES_LINE_BEFORE).cloned());
        // Signed out, and signed in as an account that is no dev account, alike.
        for token in [None, Some(USER_TOKEN)] {
            let (status, _, data) = get_as(&app, "/api/stats/cards", token).await;
            assert_eq!(status, 200);
            assert_eq!(names(&data), expected, "{token:?}");
            assert_eq!(data["olderPatchesForDevs"], json!(true));
        }
        // The line before's third patch is past it.
        let third = &before[PUBLIC_STATS_PATCHES_LINE_BEFORE];
        let (status, _, refused) = get_public(&app, &format!("/api/stats/cards?patch={third}")).await;
        assert_eq!(
            (status, refused["error"]["code"].clone()),
            (403, json!("patch_dev_only"))
        );
    }

    #[tokio::test]
    async fn r4677000_anyone_reads_the_five_newest_patches_when_that_run_is_longer() {
        // No game anywhere: the newest line is its major and its newest patch, and each line before is its
        // major alone, so the newest line and one of the line before are three, short of five.
        let app = test_app().await;
        let (_, _, all) = get_as(&app, "/api/stats/cards", Some(DEV_TOKEN)).await;
        let every = names(&all);
        let newest_line = every
            .iter()
            .take_while(|name| line(name) == line(&every[0]))
            .count();
        assert_eq!(newest_line, 2);
        assert!(newest_line + 1 < PUBLIC_STATS_PATCHES_NEWEST);
        assert!(every.len() > PUBLIC_STATS_PATCHES_NEWEST);
        let (status, _, data) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(status, 200);
        assert_eq!(names(&data), every[..PUBLIC_STATS_PATCHES_NEWEST].to_vec());
    }

    #[tokio::test]
    async fn r4677000_a_dev_account_reads_every_patch_back_to_v0_1_0() {
        let history = jackioh_cards::patch_history();
        let first = &history[0].version;
        let app = test_app().await;
        insert_records(&app, vec![live_on("old-1".to_string(), first)]).await;

        // R1310004's end-to-end dev fixture: every window, the oldest opened by the first version shipped.
        let (status, headers, data) = get_as(&app, "/api/stats/cards", Some(DEV_TOKEN)).await;
        assert_eq!(status, 200);
        let listed = names(&data);
        assert_eq!(listed.last(), Some(first));
        let versions: Vec<String> = data["patches"]
            .as_array()
            .expect("the patches are a list")
            .iter()
            .rev()
            .flat_map(|window| window["versions"].as_array().expect("versions").clone())
            .map(|version| version.as_str().expect("a version").to_string())
            .collect();
        let shipped: Vec<String> = history.iter().map(|step| step.version.clone()).collect();
        assert_eq!(versions, shipped);
        assert_eq!(data["olderPatchesForDevs"], json!(false));
        assert_eq!(cache_control(&headers), Some("private, no-store"));
        // Its oldest patch reads on both routes, a micro of it too.
        for path in [
            format!("/api/stats/cards?patch={first}"),
            format!("/api/stats/games?patch={first}"),
        ] {
            let (status, _, body) = get_as(&app, &path, Some(DEV_TOKEN)).await;
            assert_eq!(status, 200, "{path}");
            assert_eq!(body["totalGames"], json!(1), "{path}");
        }

        // An account `DEV_PROFILE_IDS` lists, whose profile id is not its user id, reads them too.
        let listed_id = "d0000000-0000-4000-8000-000004677000";
        let listed_app = test_app_with(TestAppOptions {
            env: vec![("DEV_PROFILE_IDS".to_string(), listed_id.to_string())],
            ..TestAppOptions::default()
        })
        .await;
        fake(&listed_app).lock().await.seed_profile(json!({
            "id": listed_id,
            "userId": "user-4677",
            "status": "active",
            "username": "Dev4677",
            "usernamePrompted": true,
        }));
        let token = add_user(&listed_app, "user-4677", "user-4677@example.test", true);
        let (status, _, body) = get_as(
            &listed_app,
            &format!("/api/stats/cards?patch={first}"),
            Some(&token),
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(body["olderPatchesForDevs"], json!(false));
    }

    #[tokio::test]
    async fn r4677000_a_patch_past_the_callers_is_403_patch_dev_only_on_both_routes() {
        let history = jackioh_cards::patch_history();
        let first = &history[0].version;
        // A micro of the first patch, read as the patch that holds it.
        let micro = &history
            .iter()
            .find(|step| step.tier == PatchTier::Micro)
            .expect("a micro has shipped")
            .version;
        let app = test_app().await;
        for token in [None, Some(USER_TOKEN)] {
            for patch in [first, micro] {
                for route in ["cards", "games"] {
                    let path = format!("/api/stats/{route}?patch={patch}");
                    let (status, _, body) = get_as(&app, &path, token).await;
                    assert_eq!(status, 403, "{path} {token:?}");
                    assert_eq!(body["error"]["code"], json!("patch_dev_only"), "{path}");
                }
            }
        }
        // A token that does not verify is read as no token: the route stays open, and the limit holds.
        let (status, _, _) = get_as(&app, "/api/stats/cards", Some("not-a-token")).await;
        assert_eq!(status, 200);
        let (status, _, _) = get_as(
            &app,
            &format!("/api/stats/cards?patch={first}"),
            Some("not-a-token"),
        )
        .await;
        assert_eq!(status, 403);
        // No patch, the current one, is open to everyone.
        let (status, _, _) = get_public(&app, &format!("/api/stats/cards?patch={}", current_patch())).await;
        assert_eq!(status, 200);

        // The patch before the oldest readable patch is past it: anyone is told none, a dev account the version.
        let (_, _, page) = get_public(&app, "/api/stats/cards").await;
        let oldest = names(&page).last().expect("a patch").clone();
        let path = format!("/api/stats/cards?patch={oldest}");
        let (_, _, anyone) = get_public(&app, &path).await;
        assert_eq!(anyone["previousPatch"], Value::Null);
        let (_, _, dev) = get_as(&app, &path, Some(DEV_TOKEN)).await;
        assert!(dev["previousPatch"].is_string(), "{}", dev["previousPatch"]);
    }

    #[tokio::test]
    async fn r4677000_the_drill_down_lists_only_the_callers_patches() {
        let history = jackioh_cards::patch_history();
        let first = history[0].version.clone();
        let current = current_patch();
        let gate = count(PUBLIC_STATS_MIN_LIVE_GAMES);
        // R4965000: the window just past anyone's patches, whose totals the kept snapshot still holds as
        // filed, where the first version's are folded by play version.
        let windows_with = |played: &[&str]| {
            jackioh_engine::wire::stats::patch_windows(
                &history,
                |version| i64::from(played.contains(&version)),
                |version| i64::from(played.contains(&version)),
            )
        };
        let before = windows_with(&[first.as_str(), current.as_str()]);
        let beyond = before[public_patch_count(&before)].versions[0].clone();
        let after = windows_with(&[first.as_str(), beyond.as_str(), current.as_str()]);
        assert_eq!(after[public_patch_count(&after)].versions[0], beyond);
        assert_ne!(beyond, first);
        let app = test_app().await;
        let mut records: Vec<Value> = (0..gate).map(|n| live_on(format!("first-{n}"), &first)).collect();
        records.extend((0..gate).map(|n| live_on(format!("beyond-{n}"), &beyond)));
        records.extend((0..gate).map(|n| live_on(format!("now-{n}"), &current)));
        insert_records(&app, records).await;
        let patches = |data: &Value| -> Vec<Value> {
            data["patches"]
                .as_array()
                .expect("the patches are a list")
                .iter()
                .map(|row| row["patch"].clone())
                .collect()
        };
        // R4965000: a dev account's list stops at the patches the snapshot holds as filed, so the first
        // version's patch is a `?patch=` away.
        let (_, _, dev) = get_as(&app, "/api/stats/cards/core-001", Some(DEV_TOKEN)).await;
        assert_eq!(patches(&dev), vec![json!(beyond), json!(current)]);
        for token in [None, Some(USER_TOKEN)] {
            let (status, _, anyone) = get_as(&app, "/api/stats/cards/core-001", token).await;
            assert_eq!(status, 200);
            assert_eq!(patches(&anyone), vec![json!(current)], "{token:?}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r4677000_the_kept_figures_never_cross_from_one_reader_to_another() {
        let history = jackioh_cards::patch_history();
        let first = history[0].version.clone();
        let app = test_app().await;
        insert_records(&app, vec![live_on("old-1".to_string(), &first)]).await;
        let old_cards = format!("/api/stats/cards?patch={first}");
        let old_games = format!("/api/stats/games?patch={first}");

        // A dev account reads first, filling every kept figure; anyone after it, inside the same TTL, is
        // still refused and listed the newest patches alone.
        let (_, _, dev_page) = get_as(&app, "/api/stats/cards", Some(DEV_TOKEN)).await;
        for path in [&old_cards, &old_games] {
            assert_eq!(get_as(&app, path, Some(DEV_TOKEN)).await.0, 200, "{path}");
        }
        tokio::time::advance(Duration::from_secs(1)).await;
        for path in [&old_cards, &old_games] {
            assert_eq!(get_public(&app, path).await.0, 403, "{path}");
        }
        let (_, anyone_headers, anyone_page) = get_public(&app, "/api/stats/cards").await;
        assert!(names(&anyone_page).len() < names(&dev_page).len());
        // And the other way: what anyone was answered never stands in for the dev account's.
        let (_, dev_headers, dev_again) = get_as(&app, "/api/stats/cards", Some(DEV_TOKEN)).await;
        assert_eq!(names(&dev_again), names(&dev_page));

        // No cache in front keeps the dev account's answer, and every answer varies by who asks.
        assert_eq!(cache_control(&dev_headers), Some("private, no-store"));
        assert_eq!(
            cache_control(&anyone_headers),
            Some(format!("public, max-age={CARD_STATS_CACHE_TTL_SECONDS}").as_str())
        );
        for path in [
            "/api/stats/cards",
            "/api/stats/games",
            "/api/stats/cards/core-001",
        ] {
            for token in [None, Some(DEV_TOKEN)] {
                let (status, headers, _) = get_as(&app, path, token).await;
                assert_eq!(status, 200, "{path}");
                let vary = headers
                    .get("vary")
                    .and_then(|value| value.to_str().ok())
                    .unwrap_or("");
                assert!(vary.contains("Authorization"), "{path} {token:?}: {vary}");
                let expected = if token.is_some() {
                    "private, no-store".to_string()
                } else {
                    format!("public, max-age={CARD_STATS_CACHE_TTL_SECONDS}")
                };
                assert_eq!(
                    cache_control(&headers),
                    Some(expected.as_str()),
                    "{path} {token:?}"
                );
            }
        }
    }
}

mod r4675001_play_versions {
    use super::*;
    use jackioh_engine::wire::stats::{PatchTier, major_line_versions};

    /// The keys of a JSON object, sorted.
    fn keys(value: &Value) -> Vec<String> {
        let mut keys: Vec<String> = value.as_object().expect("an object").keys().cloned().collect();
        keys.sort();
        keys
    }

    #[tokio::test]
    async fn r4675001_the_route_lists_every_play_version_and_an_old_one_with_no_totals() {
        let app = test_app().await;
        let floor = count(CARD_STATS_MIN_SAMPLE);
        let records: Vec<Value> = (0..floor)
            .map(|i| make_game_record(&format!("pv-{i}"), RecordOptions::default()))
            .collect();
        insert_records(&app, records).await;

        let (status, headers, data) = get_public(&app, "/api/stats/versions").await;
        assert_eq!(status, 200);
        let ttl = format!("public, max-age={CARD_STATS_CACHE_TTL_SECONDS}");
        assert_eq!(cache_control(&headers), Some(ttl.as_str()));
        assert_eq!(
            keys(&data),
            [
                "bandHigh",
                "bandLow",
                "decisiveness",
                "field",
                "minSample",
                "mode",
                "pilot",
                "versions"
            ]
        );
        assert_eq!((&data["pilot"], &data["mode"]), (&json!("merged"), &json!("all")));
        assert_eq!(data["minSample"], json!(CARD_STATS_MIN_SAMPLE));
        let rows = data["versions"].as_array().expect("versions is a list");
        let versions = jackioh_cards::deck_play_versions();
        assert_eq!(rows.len(), versions.len());
        // Figures only, and no source (R661): a client reads each definition from its catalog at `last`.
        assert_eq!(
            keys(&rows[0]),
            [
                "adjustedHigh",
                "adjustedLow",
                "adjustedWinRate",
                "card",
                "decisiveness",
                "first",
                "firstShipped",
                "games",
                "hasEnoughGames",
                "id",
                "iteration",
                "last",
                "live",
                "name",
                "patches",
                "playRate",
                "tier",
                "winRate"
            ]
        );
        for (row, version) in rows.iter().zip(&versions) {
            assert_eq!(row["id"], json!(version.id));
            // R5035000: its play version's iteration, as a label.
            assert_eq!(row["iteration"], json!(version.iteration_label()));
        }

        let core: Vec<&Value> = rows
            .iter()
            .filter(|row| row["card"] == json!("core-001"))
            .collect();
        let live = core.last().expect("core-001 has play versions");
        assert_eq!(live["live"], json!(true));
        assert_eq!(live["last"], json!(current_patch()));
        assert_eq!(live["games"], json!(floor));
        assert_eq!(live["hasEnoughGames"], json!(true));
        assert!(live["tier"].is_string(), "{live}");
        assert!(live["adjustedWinRate"].is_number(), "{live}");
        // R4678001: the patches its versions reach, the newest among them.
        assert!(
            live["patches"].as_u64().is_some_and(|patches| patches >= 1),
            "{live}"
        );
        // Its first version's position in the shipped history, which the client never orders names for.
        let shipped = jackioh_cards::shipped_versions();
        let first = live["first"].as_str().expect("first is a version");
        assert_eq!(
            live["firstShipped"],
            json!(shipped.iter().position(|version| *version == first))
        );

        // Its oldest play version began before the records: no games and no figure.
        let oldest = core[0];
        assert_eq!(oldest["first"], json!(jackioh_cards::shipped_versions()[0]));
        assert_eq!(oldest["firstShipped"], json!(0));
        // R5035000: counted though it began before the records.
        assert_eq!(oldest["iteration"], json!("v1"));
        assert_eq!(oldest["live"], json!(false));
        assert_eq!(oldest["games"], json!(0));
        assert_eq!(oldest["hasEnoughGames"], json!(false));
        for figure in [
            "adjustedWinRate",
            "adjustedLow",
            "adjustedHigh",
            "tier",
            "winRate",
            "playRate",
        ] {
            assert_eq!(oldest[figure], Value::Null, "{figure}");
        }
        // R5036000: nor a Decisiveness, sent as null.
        assert_eq!(oldest["decisiveness"], Value::Null);

        // `?mode=` and `?pilot=` name what the figures count; every game here is a Best of 1.
        let (_, _, conquest) = get_public(&app, "/api/stats/versions?mode=bo3&pilot=human").await;
        assert_eq!(
            (&conquest["pilot"], &conquest["mode"]),
            (&json!("human"), &json!("bo3"))
        );
        let row = find(&conquest["versions"], "id", &live["id"]).expect("the live row");
        assert_eq!(row["games"], json!(0));
        // R5035000: a match type moves no number.
        assert_eq!(row["iteration"], live["iteration"]);
    }

    // R5040000: `?scope=major` sends the live play versions alone, each cut to the newest major's line,
    // the last `major` of the shipped history through the newest; no scope, or one the route does not
    // know, answers the whole table as before.
    #[tokio::test]
    async fn r5040000_scope_major_answers_the_live_play_versions_cut_to_the_newest_major_and_no_scope_answers_as_before()
     {
        let history = jackioh_cards::patch_history();
        let major = history
            .iter()
            .rposition(|step| step.tier == PatchTier::Major)
            .expect("a major has shipped");
        assert!(major > 0, "a version shipped before the newest major");
        let before = history[major - 1].version.clone();
        let plays = jackioh_cards::deck_play_versions();
        let spanning = plays
            .iter()
            .find(|play| play.live && play.versions.contains(&before))
            .expect("a live play version crosses the newest major");
        let card: &'static str = jackioh_cards::CATALOG_IDS
            .iter()
            .find(|id| **id == spanning.card)
            .map(String::as_str)
            .expect("a catalog card");
        let other: &'static str = jackioh_cards::CATALOG_IDS
            .iter()
            .find(|id| **id != spanning.card)
            .map(String::as_str)
            .expect("another catalog card");
        let floor = count(CARD_STATS_MIN_SAMPLE);
        let on = |patch: String, from: usize| {
            (from..from + floor).map(move |i| {
                make_game_record(
                    &format!("scope-{i}"),
                    RecordOptions {
                        patch: Some(patch.clone()),
                        p1_deck: Some(vec![card]),
                        p2_deck: Some(vec![other]),
                        ..RecordOptions::default()
                    },
                )
            })
        };
        let app = test_app().await;
        insert_records(
            &app,
            on(before.clone(), 0).chain(on(current_patch(), floor)).collect(),
        )
        .await;

        let (_, _, whole) = get_public(&app, "/api/stats/versions").await;
        let (status, headers, cut) = get_public(&app, "/api/stats/versions?scope=major").await;
        assert_eq!(status, 200);
        let ttl = format!("public, max-age={CARD_STATS_CACHE_TTL_SECONDS}");
        assert_eq!(cache_control(&headers), Some(ttl.as_str()));
        // The body never names the scope.
        assert_eq!(keys(&cut), keys(&whole));
        let rows = cut["versions"].as_array().expect("versions is a list");
        let expected: Vec<Value> = major_line_versions(&plays, &history)
            .iter()
            .map(|play| json!(play.id))
            .collect();
        assert_eq!(
            rows.iter().map(|row| row["id"].clone()).collect::<Vec<_>>(),
            expected
        );
        assert!(!rows.is_empty());
        assert!(rows.len() < plays.len(), "the closed play versions have no row");
        let line: Vec<Value> = history[major..].iter().map(|step| json!(step.version)).collect();
        for row in rows {
            assert_eq!(row["live"], json!(true), "{row}");
            assert!(line.contains(&row["first"]), "{row}");
            assert_eq!(row["last"], json!(current_patch()), "{row}");
        }
        let id = json!(spanning.id);
        let cut_row = find(&cut["versions"], "id", &id).expect("the spanning row, cut");
        assert_eq!(cut_row["first"], json!(history[major].version));
        // R5035000: a cut row keeps its play version's iteration.
        assert_eq!(cut_row["iteration"], json!(spanning.iteration_label()));
        assert_eq!(cut_row["games"], json!(floor));

        // No scope: every play version over its whole span.
        let all = whole["versions"].as_array().expect("versions is a list");
        assert_eq!(all.len(), plays.len());
        let whole_row = find(&whole["versions"], "id", &id).expect("the spanning row");
        assert_eq!(whole_row["first"], json!(spanning.first()));
        assert_eq!(whole_row["games"], json!(2 * floor));
        let patches = |row: &Value| row["patches"].as_u64().expect("a patch count");
        assert!(patches(whole_row) > patches(cut_row), "{whole_row} {cut_row}");

        // A scope the route does not know answers as none does.
        let (_, _, unknown) = get_public(&app, "/api/stats/versions?scope=nonsense").await;
        assert_eq!(unknown, whole);
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

mod r4082000_created_tiers {
    use super::*;

    /// Thirty-two Core cards, each made as a Created copy in some games: the first sixteen in the games of
    /// one group, the rest in the other's.
    const MADE: [&str; 32] = [
        "core-050", "core-051", "core-052", "core-053", "core-054", "core-055", "core-056", "core-057",
        "core-058", "core-059", "core-060", "core-061", "core-062", "core-063", "core-064", "core-065",
        "core-066", "core-067", "core-068", "core-069", "core-070", "core-071", "core-072", "core-073",
        "core-074", "core-075", "core-076", "core-077", "core-078", "core-079", "core-080", "core-081",
    ];

    /// `count` live Best of 1 games p1 wins `won` of: p1 holds core-001 and core-002, plays both, uses a
    /// Created copy of each of `made` and makes The Coin, never used.
    fn games(tag: &str, made: &[&str], won: usize, count: usize) -> Vec<Value> {
        (0..count)
            .map(|i| {
                let used: Vec<Value> = made
                    .iter()
                    .map(|card| json!({ "card": card, "turn": 5, "radiant": false }))
                    .collect();
                make_game_record(
                    &format!("created-{tag}-{i}"),
                    RecordOptions {
                        source: Some("live"),
                        mode: Some("bo1"),
                        p1_created_used: Some(json!(used)),
                        p1_created_unused: Some(vec!["core-t-coin"]),
                        winner: Some(if i < won { "p1" } else { "p2" }),
                        ..RecordOptions::default()
                    },
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn r4082000_a_created_card_answers_an_adjusted_rate_and_a_tier_read_against_the_created_cards() {
        let app = test_app().await;
        let (top, bottom) = MADE.split_at(16);
        let mut records = Vec::new();
        records.extend(games("top", top, 35, 50));
        records.extend(games("bottom", bottom, 25, 50));
        insert_records(&app, records).await;

        let (status, _, page) = get_public(&app, "/api/stats/cards?source=live&mode=bo1").await;
        assert_eq!(status, 200);
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
        // The average deck: p1's two cards won 60 of 100 decks each, and p2's two 40.
        let deck = page["tiers"]["baseline"]
            .as_f64()
            .expect("the average deck's win rate");
        assert!(close(deck, 0.5), "{}", page["tiers"]);
        // The Created cards' seats won 70% and 50%, 50 games each: their centre is the mean lift over the
        // average deck, and their spread their lifts' variance less the noise.
        let field = &page["tiers"]["created"];
        assert_eq!(field["cards"], json!(32), "{field}");
        let noise: f64 = 0.25 / 50.0;
        let spread = (32.0 * 0.1 * 0.1 / 31.0 - noise).sqrt();
        assert!(close(field["center"].as_f64().unwrap(), 0.6 - deck), "{field}");
        assert!(close(field["spread"].as_f64().unwrap(), spread), "{field}");
        assert!(close(field["baseline"].as_f64().unwrap(), 0.6), "{field}");

        let weight = spread * spread / (spread * spread + noise);
        let row = |id: &str| {
            find(&page["created"], "id", &json!(id))
                .expect("the Created card is listed")
                .clone()
        };
        // 70% over the 60% an average Created card's seats win: kept as far as the spread lets it, A, as its
        // range still reaches an even game; 50%, as far below, D.
        let high = row("core-050");
        assert_eq!(high["games"], json!(50));
        let rate = high["adjustedWinRate"].as_f64().expect("an adjusted rate");
        assert!(close(rate, 0.5 + weight * 0.1), "{high}");
        let half = jackioh_engine::config::STATS_INTERVAL_Z * (weight * noise).sqrt();
        assert!(
            close(high["adjustedLow"].as_f64().unwrap(), rate - half),
            "{high}"
        );
        assert!(
            close(high["adjustedHigh"].as_f64().unwrap(), rate + half),
            "{high}"
        );
        assert_eq!(high["tier"], json!("A"));
        let low = row("core-081");
        assert!(
            close(low["adjustedWinRate"].as_f64().unwrap(), 0.5 - weight * 0.1),
            "{low}"
        );
        assert_eq!(low["tier"], json!("D"));

        // The Coin, made in every game before any play, is read against the average deck: 60% of 100.
        let coin = row("core-t-coin");
        let deck_spread = page["tiers"]["spread"].as_f64().unwrap();
        let kept = deck_spread * deck_spread / (deck_spread * deck_spread + 0.25 / 100.0);
        assert!(
            close(
                coin["adjustedWinRate"].as_f64().unwrap(),
                0.5 + kept * (0.6 - deck)
            ),
            "{coin}"
        );
        assert!(coin["tier"].is_string(), "{coin}");
    }

    #[tokio::test]
    async fn r4082000_below_the_floor_a_created_card_answers_its_adjusted_rate_and_no_tier() {
        let app = test_app().await;
        insert_records(&app, games("few", &["core-050"], 10, 10)).await;
        let (status, _, page) = get_public(&app, "/api/stats/cards?source=live&mode=bo1").await;
        assert_eq!(status, 200);
        // Too few Created cards for a field: read against the average deck, as the Dormant Starship is.
        assert_eq!(page["tiers"]["created"]["cards"], json!(0));
        assert_eq!(page["tiers"]["created"]["center"], json!(0.0));
        let row = find(&page["created"], "id", &json!("core-050")).expect("listed");
        assert_eq!(row["hasEnoughGames"], json!(false));
        assert!(row["adjustedWinRate"].as_f64().is_some(), "{row}");
        assert_eq!(row["tier"], Value::Null);
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

mod r3950000_play_rate {
    use super::*;

    const STARSHIP: &str = "stinkyscape-t-dormant-starship";

    fn card<'a>(data: &'a Value, id: &str) -> &'a Value {
        find(&data["cards"], "id", &json!(id)).expect("the card is listed")
    }

    /// `count` live All Random games p1 wins: p1 holds core-001 to core-004, sees `hand`, plays `played`,
    /// and makes a Dormant Starship it uses (`Some(true)`), one it never uses (`Some(false)`) or none.
    fn games(
        tag: &str,
        hand: &[&'static str],
        played: &[&'static str],
        starship: Option<bool>,
        count: usize,
    ) -> Vec<Value> {
        games_won_by("p1", tag, hand, played, starship, count)
    }

    /// `games`, won by `winner`.
    fn games_won_by(
        winner: &'static str,
        tag: &str,
        hand: &[&'static str],
        played: &[&'static str],
        starship: Option<bool>,
        count: usize,
    ) -> Vec<Value> {
        (0..count)
            .map(|i| {
                make_game_record(
                    &format!("play-rate-{tag}-{i}"),
                    RecordOptions {
                        source: Some("live"),
                        mode: Some("random"),
                        p1_deck: Some(vec!["core-001", "core-002", "core-003", "core-004"]),
                        p1_hand: Some(hand.to_vec()),
                        p1_played: Some(played.to_vec()),
                        p2_deck: Some(vec!["core-005"]),
                        p2_played: Some(Vec::new()),
                        p1_created_used: (starship == Some(true))
                            .then(|| json!([{ "card": STARSHIP, "turn": 5, "radiant": false }])),
                        p1_created_unused: (starship == Some(false)).then(|| vec![STARSHIP]),
                        winner: Some(winner),
                        ..RecordOptions::default()
                    },
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn r3950000_a_cards_play_rate_is_its_played_games_over_its_seen_games_in_every_mode() {
        let app = test_app().await;
        let mut records = Vec::new();
        // core-001 is held in 80 games, seen in 40 of them and played in 10 of those. core-002 is seen in
        // all 80 and never played, and core-004 is never seen. A Dormant Starship is made in 40 games and
        // used in 30 of them.
        let seen = ["core-001", "core-002", "core-003"];
        records.extend(games("played", &seen, &["core-001"], Some(true), 10));
        records.extend(games("held-used", &seen, &[], Some(true), 20));
        records.extend(games("held-unused", &seen, &[], Some(false), 10));
        records.extend(games("unseen", &["core-002", "core-003"], &[], None, 40));
        insert_records(&app, records).await;

        // All Random counts the 40 seen games (R2777002), and 10 of them played it.
        let (status, _, random) = get_public(&app, "/api/stats/cards?source=live&mode=random").await;
        assert_eq!(status, 200);
        let a = card(&random, "core-001");
        assert_eq!(a["games"], json!(40));
        assert_eq!(a["playRate"].as_f64(), Some(0.25));
        // Every mode counts all 80 decks that held it, and its play rate is still 10 of 40 seen, not 80
        // held of the 160 seats counted.
        let (_, _, all) = get_public(&app, "/api/stats/cards?source=live").await;
        let a = card(&all, "core-001");
        assert_eq!(a["games"], json!(80));
        assert_eq!(a["playRate"].as_f64(), Some(0.25));
        // Seen every game and never played: 0. Never seen: 0, not a share of no games.
        assert_eq!(card(&all, "core-002")["playRate"].as_f64(), Some(0.0));
        assert_eq!(card(&all, "core-004")["playRate"].as_f64(), Some(0.0));
        // A card only a rule makes (R3070000): used in 30 of the 40 games a seat made one.
        assert_eq!(card(&all, STARSHIP)["playRate"].as_f64(), Some(0.75));
    }

    // R5037000: a play version's Win rate and Play rate are the Cards tab's over the same records.
    #[tokio::test]
    async fn r5037000_a_play_versions_win_rate_and_play_rate_are_the_cards_tabs_over_the_same_records() {
        let app = test_app().await;
        let mut records = Vec::new();
        // The Cards test's four groups, but p1 loses the 30 games in which it makes a Starship and plays
        // nothing: core-001 is seen in 40 games, wins 10 and plays in the same 10.
        let seen = ["core-001", "core-002", "core-003"];
        records.extend(games("played", &seen, &["core-001"], Some(true), 10));
        records.extend(games_won_by("p2", "held-used", &seen, &[], Some(true), 20));
        records.extend(games_won_by("p2", "held-unused", &seen, &[], Some(false), 10));
        records.extend(games("unseen", &["core-002", "core-003"], &[], None, 40));
        insert_records(&app, records).await;

        let (_, _, cards) = get_public(&app, "/api/stats/cards?source=live&mode=random").await;
        let (status, _, versions) = get_public(&app, "/api/stats/versions?mode=random").await;
        assert_eq!(status, 200);
        let live = |id: &str| {
            versions["versions"]
                .as_array()
                .expect("versions is a list")
                .iter()
                .find(|row| row["card"] == json!(id) && row["live"] == json!(true))
                .expect("the card has a live play version")
        };

        let a = card(&cards, "core-001");
        let row = live("core-001");
        assert_eq!(row["games"], a["games"]);
        assert_eq!(a["winRate"].as_f64(), Some(0.25));
        assert_eq!(row["winRate"], a["winRate"]);
        assert_eq!(a["playRate"].as_f64(), Some(0.25));
        assert_eq!(row["playRate"], a["playRate"]);

        // core-002 was seen every game and never played: both read 0, as on the Cards tab.
        let b = card(&cards, "core-002");
        let row = live("core-002");
        assert_eq!(row["winRate"], b["winRate"]);
        assert_eq!(row["playRate"].as_f64(), Some(0.0));
        assert_eq!(row["playRate"], b["playRate"]);

        // core-004 was held and never seen: the Cards tab reads 0, the play version none rather than 0%.
        let never = live("core-004");
        assert_eq!(card(&cards, "core-004")["playRate"].as_f64(), Some(0.0));
        assert_eq!(never["playRate"], Value::Null);
        assert_eq!(never["winRate"], Value::Null);

        // The Dormant Starship (R3070000): used in 30 of the 40 games a seat made one, and won in 10 of them.
        assert_eq!(live(STARSHIP)["playRate"].as_f64(), Some(0.75));
        assert_eq!(live(STARSHIP)["playRate"], card(&cards, STARSHIP)["playRate"]);
        assert_eq!(live(STARSHIP)["winRate"], card(&cards, STARSHIP)["winRate"]);
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
        // R2777000: 50% plus the 30-point drawn delta, kept as far as 60 games a side (seen and never drawn)
        // back it.
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
        // R3950000 amends R2777002's play rate: played over seen, and none of its 10 seen games played it.
        assert_eq!(a["playRate"].as_f64(), Some(0.0));
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
        assert_eq!(card001["games"], json!(2));
        assert!(find(&data["cards"], "id", &json!("core-t-rush")).is_none());

        let created = data["created"].as_array().expect("a Created table");
        let ids: Vec<&str> = created.iter().filter_map(|row| row["id"].as_str()).collect();
        // Both are short of the floor, so the one with more games comes first.
        assert_eq!(ids, ["core-t-rush", "core-001"]);
        let rush = &created[0];
        assert_eq!(rush["name"], json!(jackioh_cards::CATALOG["core-t-rush"].name));
        assert_eq!(rush["games"], json!(2));
        assert_eq!(rush["hasEnoughGames"], json!(false));
        assert!(rush["window"]["from"].is_string());
        let made001 = &created[1];
        assert_eq!(made001["games"], json!(1));

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

    #[tokio::test]
    async fn r4083000_a_created_row_answers_only_the_fields_the_client_reads() {
        let app = test_app().await;
        // R2089000's two games: p1 makes a Rush token in both, using it in the first, and a Created core-001
        // in the first.
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
        // Every Created row carries these keys and no others: no used, unused, Radiant or normal split and no
        // played delta, which no client reads.
        let created = data["created"].as_array().expect("a Created table");
        assert_eq!(created.len(), 2);
        for row in created {
            let mut keys: Vec<&str> = row
                .as_object()
                .expect("a row")
                .keys()
                .map(String::as_str)
                .collect();
            keys.sort_unstable();
            assert_eq!(
                keys,
                [
                    "adjustedHigh",
                    "adjustedLow",
                    "adjustedWinRate",
                    "cost",
                    "decisiveness",
                    "games",
                    "hasEnoughGames",
                    "id",
                    "name",
                    "rarity",
                    "set",
                    "tier",
                    "window",
                ],
                "{}",
                row["id"]
            );
        }
        // The games still count every seat that made one, used or not.
        let rush = find(&data["created"], "id", &json!("core-t-rush")).expect("the Rush token is listed");
        assert_eq!(rush["games"], json!(2));
        // The Created cards' field stays, as the tiers are read against it.
        let mut field: Vec<&str> = data["tiers"]["created"]
            .as_object()
            .expect("the Created field")
            .keys()
            .map(String::as_str)
            .collect();
        field.sort_unstable();
        // R4552003: and the typical Created card's Decisiveness.
        assert_eq!(field, ["baseline", "cards", "center", "decisiveness", "spread"]);
    }
}

// R3049000: the win-rate snapshot a match starts with is the cached report `/api/stats/cards` shows.
mod r3049000_the_win_rate_snapshot {
    use super::*;
    use jackioh_engine::WinRateSource;
    use jackioh_server::api::stats::win_rate_snapshot;

    /// `count` records of `source` with core-001 and core-002 in p1's deck, p1 winning every even one,
    /// their ids `prefix` and a number.
    fn records(prefix: &str, source: &'static str, count: usize) -> Vec<Value> {
        (1..=count)
            .map(|i| {
                make_game_record(
                    &format!("{prefix}-{i}"),
                    RecordOptions {
                        source: Some(source),
                        p1_deck: Some(vec!["core-001", "core-002"]),
                        winner: Some(if i % 2 == 0 { "p1" } else { "p2" }),
                        ..RecordOptions::default()
                    },
                )
            })
            .collect()
    }

    #[tokio::test]
    async fn r3049000_the_snapshot_is_the_cached_reports_in_deck_figures() {
        let app = test_app().await;
        insert_records(&app, records("live", "live", 30)).await;
        let snapshot = win_rate_snapshot(&app).await;
        assert_eq!(snapshot.patch, current_patch());
        let row = snapshot
            .cards
            .iter()
            .find(|row| row.id == "core-001")
            .expect("core-001's row");
        assert_eq!((row.wins, row.games), (15, 30));
        assert!(
            snapshot.cards.windows(2).all(|pair| pair[0].id < pair[1].id),
            "sorted by id"
        );
        assert!(snapshot.cards.iter().all(|row| row.games > 0));
        let (status, _, data) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(status, 200);
        let card = find(&data["cards"], "id", &json!("core-001")).expect("core-001 is listed");
        assert_eq!(card["games"], json!(row.games));
        assert_eq!(card["winRate"], json!(0.5));
        // The kept report: records written since leave it as it was until the cache lets it go
        // (`CARD_STATS_CACHE_TTL_SECONDS`).
        insert_records(&app, records("late", "live", 10)).await;
        assert_eq!(win_rate_snapshot(&app).await, snapshot);
    }

    #[tokio::test]
    async fn r2386000_r3049000_a_snapshot_taken_while_the_window_is_short_counts_development_games() {
        let app = test_app().await;
        let mut both = records("live", "live", 30);
        both.extend(records("dev", "dev", 20));
        insert_records(&app, both).await;
        let snapshot = win_rate_snapshot(&app).await;
        let row = snapshot
            .cards
            .iter()
            .find(|row| row.id == "core-001")
            .expect("core-001's row");
        assert_eq!(
            row.games, 50,
            "the live games and the development games that pad them"
        );
        assert_eq!(snapshot.source, WinRateSource::Provisional);
        let (_, _, data) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(data["source"], "provisional");
        let card = find(&data["cards"], "id", &json!("core-001")).expect("core-001 is listed");
        assert_eq!(card["games"], json!(row.games));
    }
}

/// #4011 (cache poisoning): every route answered through `cached_ok` is `public`, so shared
/// caches key on the URL alone. The same path and query must answer byte-identical bodies
/// whatever else the request carries.
mod cache_poisoning {
    use super::*;
    use axum::http::{HeaderName, HeaderValue};
    use jackioh_server::config::PLAY_RATES_REFRESH_SECONDS;

    use crate::support::deps::{json_request, send};

    /// `GET path` carrying `headers`, signed in when `token` is `Some`.
    async fn get_with(
        app: &Arc<App>,
        path: &str,
        token: Option<&str>,
        headers: &[(&str, &str)],
    ) -> (u16, HeaderMap, Value) {
        let mut request = json_request("GET", path, &Value::Null, token, None);
        for (name, value) in headers {
            request.headers_mut().insert(
                name.parse::<HeaderName>().expect("a header name"),
                HeaderValue::from_str(value).expect("a header value"),
            );
        }
        send(app, request).await
    }

    /// The same `path` seven ways: bare, signed in, another language, a cookie, forged
    /// forward/rewrite headers, an unknown query parameter, and all of those at once.
    async fn seven_ways(app: &Arc<App>, path: &str) -> Vec<(u16, HeaderMap, Value)> {
        let poisoned = if path.contains('?') {
            format!("{path}&poison=1")
        } else {
            format!("{path}?poison=1")
        };
        let forwarded = [
            ("x-forwarded-host", "evil.example"),
            ("x-forwarded-for", "203.0.113.9"),
            ("x-original-url", "/api/admin"),
            ("x-rewrite-url", "/api/admin"),
        ];
        let mut all = vec![("accept-language", "fr-FR"), ("cookie", "sid=abc")];
        all.extend(forwarded);
        vec![
            get_with(app, path, None, &[]).await,
            get_with(app, path, Some(USER_TOKEN), &[]).await,
            get_with(app, path, None, &[("accept-language", "fr-FR")]).await,
            get_with(app, path, None, &[("cookie", "sid=abc")]).await,
            get_with(app, path, None, &forwarded).await,
            get_with(app, &poisoned, None, &[]).await,
            get_with(app, &poisoned, Some(USER_TOKEN), &all).await,
        ]
    }

    /// Every variant answers 200 with `public, max-age=<ttl>` and the same bytes (`cached_ok`
    /// sends `body.to_string()`, so equal serializations are byte-identity).
    fn assert_same_bytes(responses: &[(u16, HeaderMap, Value)], ttl: i64) {
        let expected = format!("public, max-age={ttl}");
        let mut bodies = Vec::with_capacity(responses.len());
        for (status, headers, body) in responses {
            assert_eq!(*status, 200);
            assert_eq!(cache_control(headers), Some(expected.as_str()));
            bodies.push(serde_json::to_string(body).expect("a body serializes"));
        }
        for body in &bodies[1..] {
            assert_eq!(body, &bodies[0]);
        }
    }

    #[tokio::test]
    async fn cache_poisoning_cards_same_bytes_whatever_the_caller_carries() {
        let app = test_app().await;
        insert_records(&app, vec![make_game_record("poison-1", RecordOptions::default())]).await;
        let responses = seven_ways(&app, "/api/stats/cards").await;
        assert_same_bytes(&responses, CARD_STATS_CACHE_TTL_SECONDS);
    }

    #[tokio::test]
    async fn cache_poisoning_card_same_bytes_whatever_the_caller_carries() {
        let app = test_app().await;
        insert_records(&app, vec![make_game_record("poison-1", RecordOptions::default())]).await;
        let responses = seven_ways(&app, "/api/stats/cards/core-001").await;
        assert_same_bytes(&responses, CARD_STATS_CACHE_TTL_SECONDS);
    }

    #[tokio::test]
    async fn cache_poisoning_play_rates_same_bytes_whatever_the_caller_carries() {
        let app = test_app().await;
        insert_records(&app, vec![make_game_record("poison-1", RecordOptions::default())]).await;
        let responses = seven_ways(&app, "/api/stats/play-rates").await;
        assert_same_bytes(&responses, PLAY_RATES_REFRESH_SECONDS);
    }

    #[tokio::test]
    async fn cache_poisoning_games_same_bytes_whatever_the_caller_carries() {
        let app = test_app().await;
        insert_records(&app, vec![make_game_record("poison-1", RecordOptions::default())]).await;
        let responses = seven_ways(&app, "/api/stats/games").await;
        assert_same_bytes(&responses, CARD_STATS_CACHE_TTL_SECONDS);
    }

    #[tokio::test]
    async fn cache_poisoning_versions_same_bytes_whatever_the_caller_carries() {
        let app = test_app().await;
        insert_records(&app, vec![make_game_record("poison-1", RecordOptions::default())]).await;
        let responses = seven_ways(&app, "/api/stats/versions").await;
        assert_same_bytes(&responses, CARD_STATS_CACHE_TTL_SECONDS);
    }

    #[tokio::test]
    async fn cache_poisoning_players_same_bytes_whatever_the_caller_carries() {
        let app = test_app().await;
        let alice_id = seed_profile(&app, "u-alice", "Alice_Wonder").await;
        put_player_stats(
            &app,
            &alice_id,
            json!({ "games": 50, "wins": 30, "losses": 20 }),
            false,
            1000,
        )
        .await;
        let responses = seven_ways(&app, "/api/stats/players").await;
        assert_same_bytes(&responses, PLAYER_STATS_CACHE_TTL_SECONDS);
    }
}

// R4673000, R4673001: `?face=` on both card routes: each face read against its own field and band, the
// drill-down's history and drills following it, and its face line.
mod r4673000_card_faces {
    use super::*;
    use jackioh_engine::config::{RADIANT_TIER_BAND_HIGH, RADIANT_TIER_BAND_LOW, STATS_INTERVAL_Z};

    /// Forty Core cards, the field the test reads.
    const FIELD: [&str; 40] = [
        "core-001", "core-002", "core-003", "core-004", "core-005", "core-006", "core-007", "core-008",
        "core-009", "core-010", "core-011", "core-012", "core-013", "core-014", "core-015", "core-016",
        "core-017", "core-018", "core-019", "core-020", "core-021", "core-022", "core-023", "core-024",
        "core-025", "core-026", "core-027", "core-028", "core-029", "core-030", "core-031", "core-032",
        "core-033", "core-034", "core-035", "core-036", "core-037", "core-038", "core-039", "core-040",
    ];

    /// How p1 met the forty in a game.
    #[derive(Clone, Copy)]
    enum Met {
        /// Played every one, Radiant, and used a Created core-050, Radiant.
        Radiant,
        /// Played every one, normal, and used a Created core-050, normal.
        Normal,
        /// Never drew one.
        Unseen,
    }

    /// `count` live Best of 1 games of p1's forty met as `met`, p1 winning the first `won`; p2 holds core-100
    /// and plays nothing.
    fn games(tag: &str, met: Met, won: usize, count: usize) -> Vec<Value> {
        (0..count)
            .map(|i| {
                let (hand, played, radiant): (Vec<&'static str>, Vec<&'static str>, Option<bool>) = match met
                {
                    Met::Radiant => (FIELD.to_vec(), FIELD.to_vec(), Some(true)),
                    Met::Normal => (FIELD.to_vec(), FIELD.to_vec(), Some(false)),
                    Met::Unseen => (Vec::new(), Vec::new(), None),
                };
                make_game_record(
                    &format!("faces-{tag}-{i}"),
                    RecordOptions {
                        source: Some("live"),
                        mode: Some("bo1"),
                        p1_deck: Some(FIELD.to_vec()),
                        p1_hand: Some(hand),
                        p1_played: Some(played),
                        p1_played_radiant: radiant.map(|flag| vec![flag; FIELD.len()]),
                        p1_created_used: radiant
                            .map(|flag| json!([{ "card": "core-050", "turn": 3, "radiant": flag }])),
                        p2_deck: Some(vec!["core-100"]),
                        p2_played: Some(Vec::new()),
                        winner: Some(if i < won { "p1" } else { "p2" }),
                        ..RecordOptions::default()
                    },
                )
            })
            .collect()
    }

    fn card<'a>(data: &'a Value, id: &str) -> &'a Value {
        find(&data["cards"], "id", &json!(id)).expect("the card is listed")
    }

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 1e-9
    }

    #[tokio::test]
    async fn r4673000_each_face_reads_its_own_field_and_band() {
        let app = test_app().await;
        // Every card: played Radiant in 30 games (24 won), normal in 30 (18 won), never drawn in 30 (12 won).
        // Its Radiant face is 40 points above its never-drawn games, its normal face 20, as every card's is.
        let mut records = games("radiant", Met::Radiant, 24, 30);
        records.extend(games("normal", Met::Normal, 18, 30));
        records.extend(games("unseen", Met::Unseen, 12, 30));
        insert_records(&app, records).await;

        let (status, _, all) = get_public(&app, "/api/stats/cards?source=live&mode=bo1").await;
        assert_eq!(status, 200);
        let (status, _, radiant) =
            get_public(&app, "/api/stats/cards?source=live&mode=bo1&face=radiant").await;
        assert_eq!(status, 200);
        let (_, _, normal) = get_public(&app, "/api/stats/cards?source=live&mode=bo1&face=normal").await;

        // Each face's field: its own centre, every card alike so its spread is the floor; the bands are its own.
        let floor = jackioh_engine::config::ADJUSTED_FIELD_SPREAD_FLOOR;
        for (page, center) in [(&radiant, 0.4), (&normal, 0.2)] {
            let tiers = &page["tiers"];
            assert_eq!(tiers["cards"], json!(40), "{tiers}");
            assert!(close(tiers["center"].as_f64().unwrap(), center), "{tiers}");
            assert!(close(tiers["spread"].as_f64().unwrap(), floor), "{tiers}");
            assert_eq!(tiers["baseline"], all["tiers"]["baseline"]);
        }
        assert_eq!(
            (
                radiant["tiers"]["bandLow"].as_f64(),
                radiant["tiers"]["bandHigh"].as_f64()
            ),
            (Some(RADIANT_TIER_BAND_LOW), Some(RADIANT_TIER_BAND_HIGH))
        );
        // All's field reads every card's seen games, both faces together: 30 points over the never-drawn.
        assert!(
            close(all["tiers"]["center"].as_f64().unwrap(), 0.3),
            "{}",
            all["tiers"]
        );

        // Each card reads an even game on each face, over that face's 30 games, its win rate and range that
        // face's; its play rate is All's.
        let noise = 0.25 * (1.0 / 30.0 + 1.0 / 30.0);
        let weight = floor * floor / (floor * floor + noise);
        let half = STATS_INTERVAL_Z * (weight * noise).sqrt();
        for (page, rate) in [(&radiant, 0.8), (&normal, 0.6)] {
            let row = card(page, "core-001");
            assert_eq!(row["games"], json!(30), "{row}");
            assert_eq!(row["hasEnoughGames"], json!(true));
            assert!(close(row["winRate"].as_f64().unwrap(), rate), "{row}");
            let adjusted = row["adjustedWinRate"].as_f64().unwrap();
            assert!(close(adjusted, 0.5), "{row}");
            assert!(
                close(row["adjustedHigh"].as_f64().unwrap(), adjusted + half),
                "{row}"
            );
            assert_eq!(row["tier"], json!(if adjusted >= 0.5 { "B" } else { "C" }));
            assert_eq!(row["playRate"], card(&all, "core-001")["playRate"]);
        }
        assert_eq!(card(&all, "core-001")["games"], json!(90));
        // p2's card, never played, has no games and no tier on a face.
        for page in [&radiant, &normal] {
            let unplayed = card(page, "core-100");
            assert_eq!(unplayed["games"], json!(0), "{unplayed}");
            assert_eq!(unplayed["adjustedWinRate"], Value::Null);
            assert_eq!(unplayed["tier"], Value::Null);
            assert_eq!(unplayed["hasEnoughGames"], json!(false));
        }

        // A Created card's faces are the seats that used a copy on each.
        let created = |page: &Value| {
            find(&page["created"], "id", &json!("core-050"))
                .expect("listed")
                .clone()
        };
        assert_eq!(created(&all)["games"], json!(60));
        assert_eq!(created(&radiant)["games"], json!(30));
        assert_eq!(created(&normal)["games"], json!(30));
        assert_ne!(
            created(&radiant)["adjustedWinRate"],
            created(&normal)["adjustedWinRate"]
        );
    }

    #[tokio::test]
    async fn r5054000_a_face_reads_the_games_its_card_was_seen_on_that_face_not_only_its_plays() {
        let app = test_app().await;
        let mut records = games("radiant", Met::Radiant, 24, 30);
        records.extend(games("normal", Met::Normal, 18, 30));
        records.extend(games("unseen", Met::Unseen, 12, 30));
        // A win-more card: core-001 also sat Radiant in p1's hand through 30 lost games and was never played.
        for i in 0..30 {
            let mut record = make_game_record(
                &format!("faces-held-{i}"),
                RecordOptions {
                    source: Some("live"),
                    mode: Some("bo1"),
                    p1_deck: Some(vec!["core-001"]),
                    p1_hand: Some(vec!["core-001"]),
                    p1_played: Some(Vec::new()),
                    p2_deck: Some(vec!["core-100"]),
                    p2_played: Some(Vec::new()),
                    winner: Some("p2"),
                    ..RecordOptions::default()
                },
            );
            record["game"]["seats"]["p1"]["heldRadiant"] = json!(["core-001"]);
            records.push(record);
        }
        insert_records(&app, records).await;

        let (status, _, radiant) =
            get_public(&app, "/api/stats/cards?source=live&mode=bo1&face=radiant").await;
        assert_eq!(status, 200);
        let held = card(&radiant, "core-001");
        let typical = card(&radiant, "core-002");
        // Its Radiant face is its 60 seen games at 40%, not its 30 plays at 80%, as every other card's plays are.
        assert_eq!(
            (held["games"].clone(), typical["games"].clone()),
            (json!(60), json!(30))
        );
        assert!(close(held["winRate"].as_f64().unwrap(), 0.4), "{held}");
        assert!(close(typical["winRate"].as_f64().unwrap(), 0.8), "{typical}");
        assert!(
            held["adjustedWinRate"].as_f64().unwrap() < typical["adjustedWinRate"].as_f64().unwrap(),
            "{held} {typical}"
        );
        // Its normal face is untouched: it was never seen normal in those games.
        let (_, _, normal) = get_public(&app, "/api/stats/cards?source=live&mode=bo1&face=normal").await;
        assert_eq!(card(&normal, "core-001")["games"], json!(30));
    }

    #[tokio::test]
    async fn r4673001_face_all_and_no_face_answer_the_same_bytes_and_an_unknown_face_reads_all() {
        let app = test_app().await;
        let mut records = games("radiant", Met::Radiant, 24, 30);
        records.extend(games("unseen", Met::Unseen, 12, 30));
        insert_records(&app, records).await;

        for base in [
            "/api/stats/cards?source=live&mode=bo1",
            "/api/stats/cards/core-001?mode=bo1",
        ] {
            let (_, _, plain) = get_public(&app, base).await;
            for face in ["all", "ALL", "sideways", ""] {
                let (status, _, asked) = get_public(&app, &format!("{base}&face={face}")).await;
                assert_eq!(status, 200);
                assert_eq!(asked, plain, "{base} face={face}");
            }
            // A face is read whatever its case and spaces.
            let (_, _, radiant) = get_public(&app, &format!("{base}&face=Radiant")).await;
            let (_, _, lower) = get_public(&app, &format!("{base}&face=radiant")).await;
            assert_eq!(radiant, lower, "{base}");
            assert_ne!(radiant, plain, "{base}");
        }
        // The default answer's rows carry no new field.
        let (_, _, page) = get_public(&app, "/api/stats/cards?source=live&mode=bo1").await;
        let keys: Vec<&String> = card(&page, "core-001").as_object().unwrap().keys().collect();
        assert!(
            !keys.iter().any(|key| key.to_lowercase().contains("face")),
            "{keys:?}"
        );
        assert!(page.get("face").is_none());
        let (_, _, drill) = get_public(&app, "/api/stats/cards/core-001").await;
        assert_eq!(drill["face"], json!("all"));
    }

    #[tokio::test]
    async fn r4673001_the_drill_down_follows_the_face_and_a_drill_with_no_face_split_reads_empty() {
        let app = test_app().await;
        let radiant_games = count(PUBLIC_STATS_MIN_LIVE_GAMES) / 2;
        let normal_games = count(PUBLIC_STATS_MIN_LIVE_GAMES) - radiant_games;
        let played = |id: String, turn: usize, radiant: Option<bool>, winner: &'static str| {
            make_game_record(
                &id,
                RecordOptions {
                    source: Some("live"),
                    mode: Some("bo1"),
                    p1_played_turns: Some(vec![turn, turn]),
                    p1_played_radiant: radiant.map(|flag| vec![flag, flag]),
                    winner: Some(winner),
                    ..RecordOptions::default()
                },
            )
        };
        // p1 plays core-001 and core-002: Radiant on turn 2 and wins, normal on turn 3 and loses, or with no
        // face flags on turn 4 and wins. p2's plays of core-003 and core-004 carry no flags.
        let mut records: Vec<Value> = (0..radiant_games)
            .map(|i| played(format!("drill-radiant-{i}"), 2, Some(true), "p1"))
            .collect();
        records.extend((0..normal_games).map(|i| played(format!("drill-normal-{i}"), 3, Some(false), "p2")));
        records.extend((0..10).map(|i| played(format!("drill-flagless-{i}"), 4, None, "p1")));
        insert_records(&app, records).await;
        let current = current_patch();

        let turn =
            |turn: i32, games: usize, rate: f64| json!({ "turn": turn, "games": games, "winRate": rate });
        let (_, _, all) = get_public(&app, "/api/stats/cards/core-001").await;
        let name = all["coPlayed"][0]["name"].clone();
        let co = |games: usize, rate: f64| json!([{ "id": "core-002", "name": name, "games": games, "winRate": rate }]);
        let patch =
            |games: usize, rate: Value| json!([{ "patch": current, "games": games, "winRate": rate }]);

        let (status, _, radiant) = get_public(&app, "/api/stats/cards/core-001?face=radiant").await;
        assert_eq!(status, 200);
        assert_eq!(radiant["face"], json!("radiant"));
        assert_eq!(radiant["patches"], patch(radiant_games, json!(1.0)));
        assert_eq!(radiant["byTurn"], json!([turn(2, radiant_games, 1.0)]));
        assert_eq!(radiant["coPlayed"], co(radiant_games, 1.0));

        let (_, _, normal) = get_public(&app, "/api/stats/cards/core-001?face=normal").await;
        assert_eq!(normal["face"], json!("normal"));
        assert_eq!(normal["patches"], patch(normal_games, json!(0.0)));
        assert_eq!(normal["byTurn"], json!([turn(3, normal_games, 0.0)]));
        assert_eq!(normal["coPlayed"], co(normal_games, 0.0));

        // All holds every seat, the flagless ones among them.
        let every = radiant_games + normal_games + 10;
        let won = radiant_games + 10;
        assert_eq!(all["patches"][0]["games"], json!(every));
        assert_eq!(
            all["byTurn"],
            json!([
                turn(2, radiant_games, 1.0),
                turn(3, normal_games, 0.0),
                turn(4, 10, 1.0)
            ])
        );
        assert_eq!(all["coPlayed"][0]["games"], json!(every));
        assert!(close(
            all["coPlayed"][0]["winRate"].as_f64().unwrap(),
            won as f64 / every as f64
        ));

        // A card whose plays never carried a face reads empty on each face, never All's figures.
        let (_, _, unsplit) = get_public(&app, "/api/stats/cards/core-003?face=radiant").await;
        assert_eq!(unsplit["byTurn"], json!([]));
        assert_eq!(unsplit["coPlayed"], json!([]));
        assert_eq!(unsplit["patches"], patch(0, Value::Null));
        let (_, _, unsplit_all) = get_public(&app, "/api/stats/cards/core-003").await;
        assert_ne!(unsplit_all["byTurn"], json!([]));

        // The face line, on every face: each face's games; no never-drawn games, so no rate and no gap.
        for page in [&all, &radiant, &normal] {
            let faces = &page["faces"];
            assert_eq!(faces["radiant"]["games"], json!(radiant_games), "{faces}");
            assert_eq!(faces["normal"]["games"], json!(normal_games), "{faces}");
            assert_eq!(faces["radiant"]["adjustedWinRate"], Value::Null, "{faces}");
            assert_eq!(faces["normal"]["tier"], Value::Null, "{faces}");
            assert_eq!(faces["gap"], Value::Null, "{faces}");
        }
    }

    #[tokio::test]
    async fn r4673001_the_face_line_reads_each_faces_rate_and_the_gap_at_the_page_s_patch() {
        let app = test_app().await;
        let mut records = games("radiant", Met::Radiant, 24, 30);
        records.extend(games("normal", Met::Normal, 18, 30));
        records.extend(games("unseen", Met::Unseen, 12, 30));
        insert_records(&app, records).await;

        // Every card is the field's on each face: both rates an even game, their gap about 0, its range the two
        // ranges' half-widths summed in quadrature.
        let (_, _, drill) = get_public(&app, "/api/stats/cards/core-001?mode=bo1").await;
        let (_, _, radiant) = get_public(&app, "/api/stats/cards?source=live&mode=bo1&face=radiant").await;
        let (_, _, normal) = get_public(&app, "/api/stats/cards?source=live&mode=bo1&face=normal").await;
        let faces = &drill["faces"];
        for (face, page) in [("radiant", &radiant), ("normal", &normal)] {
            let row = card(page, "core-001");
            for key in [
                "games",
                "adjustedWinRate",
                "adjustedLow",
                "adjustedHigh",
                "tier",
                "hasEnoughGames",
            ] {
                assert_eq!(faces[face][key], row[key], "{face} {key}");
            }
        }
        let gap = &faces["gap"];
        let half = |face: &str| {
            (faces[face]["adjustedHigh"].as_f64().unwrap() - faces[face]["adjustedLow"].as_f64().unwrap())
                / 2.0
        };
        let spread = half("normal").hypot(half("radiant"));
        let value = gap["gap"].as_f64().expect("a gap");
        assert!(close(value, 0.0), "{gap}");
        assert!(close(gap["low"].as_f64().unwrap(), value - spread), "{gap}");
        assert!(close(gap["high"].as_f64().unwrap(), value + spread), "{gap}");

        // At a patch with none of these games, each face is below the floor: no rates and no gap. R4677000: the
        // patch before the current one that anyone may read, asked at its newest version.
        let (_, _, page) = get_public(&app, "/api/stats/cards").await;
        let past = page["patches"][1]["versions"]
            .as_array()
            .and_then(|versions| versions.last())
            .and_then(Value::as_str)
            .expect("a past patch anyone reads")
            .to_string();
        assert_ne!(past, current_patch());
        let (status, _, before) =
            get_public(&app, &format!("/api/stats/cards/core-001?mode=bo1&patch={past}")).await;
        assert_eq!(status, 200);
        assert_eq!(before["faces"]["radiant"]["games"], json!(0));
        assert_eq!(before["faces"]["gap"], Value::Null);
        assert_eq!(before["byTurn"], drill["byTurn"]);
    }

    #[tokio::test]
    async fn r4673001_the_face_lines_patch_past_the_callers_is_403_patch_dev_only() {
        // R4677000: the oldest patch is a dev account's alone, so the face line cannot read it around the table.
        let app = test_app().await;
        let first = jackioh_cards::patch_history()[0].version.clone();
        let path = format!("/api/stats/cards/core-001?patch={first}");
        for token in [None, Some(USER_TOKEN)] {
            let (status, _, body) = request(&app, "GET", &path, token, None).await;
            assert_eq!(
                (status, body["error"]["code"].clone()),
                (403, json!("patch_dev_only")),
                "{token:?}"
            );
        }
        // A dev account reads it; and no patch, the current one, stays open to everyone.
        let (status, _, dev) = request(&app, "GET", &path, Some(DEV_TOKEN), None).await;
        assert_eq!(status, 200);
        assert_eq!(dev["faces"]["radiant"]["games"], json!(0));
        assert_eq!(get_public(&app, "/api/stats/cards/core-001").await.0, 200);
    }

    #[tokio::test]
    async fn r4673000_decisiveness_reads_all_s_on_every_face() {
        // R4552001's turn rows name no face, so a face reads All's Decisiveness, as it does the play rate:
        // half the games play core-001 and use the Rush Token Radiant, half normal, and each swings far.
        let app = test_app().await;
        let records: Vec<Value> = (0..40)
            .map(|i| {
                let won = i % 2 == 0;
                let radiant = i % 4 < 2;
                make_game_record(
                    &format!("faces-swing-{i}"),
                    RecordOptions {
                        source: Some("live"),
                        p1_deck: Some(vec!["core-001", "core-002", "core-015"]),
                        p1_played: Some(vec!["core-001"]),
                        p1_played_radiant: Some(vec![radiant]),
                        winner: Some(if won { "p1" } else { "p2" }),
                        p1_swing_turns: Some(json!([
                            { "turn": 1, "eval": 0 },
                            {
                                "turn": 3,
                                "eval": if won { 5_000_000 } else { -5_000_000 },
                                "acting": ["core-001", { "card": "core-t-rush", "maker": "core-015" }],
                            },
                        ])),
                        p1_created_used: Some(
                            json!([{ "card": "core-t-rush", "turn": 1, "radiant": radiant }]),
                        ),
                        ..RecordOptions::default()
                    },
                )
            })
            .collect();
        insert_records(&app, records).await;

        let (_, _, all) = get_public(&app, "/api/stats/cards").await;
        let decisive = card(&all, "core-001")["decisiveness"].clone();
        assert!(decisive.is_f64(), "{decisive}");
        let token = |data: &Value| {
            find(&data["created"], "id", &json!("core-t-rush")).expect("the Rush Token is listed")["decisiveness"]
                .clone()
        };
        assert!(token(&all).is_f64());
        for face in ["normal", "radiant"] {
            let (status, _, data) = get_public(&app, &format!("/api/stats/cards?face={face}")).await;
            assert_eq!(status, 200);
            // The face's own games, but All's Decisiveness, the card's, the token's and each field's typical.
            assert_eq!(card(&data, "core-001")["games"], json!(20), "{face}");
            assert_eq!(card(&data, "core-001")["decisiveness"], decisive, "{face}");
            assert_eq!(token(&data), token(&all), "{face}");
            assert_eq!(
                data["tiers"]["decisiveness"], all["tiers"]["decisiveness"],
                "{face}"
            );
            assert_eq!(
                data["tiers"]["created"]["decisiveness"], all["tiers"]["created"]["decisiveness"],
                "{face}"
            );
        }
    }
}

// R4552003: `GET /api/stats/cards` sends each card's Decisiveness and each field's typical value, and
// nothing else of R4552001's: no Swing, no Board swing, no play term, no raw RMS.
mod r4552003_decisiveness {
    use super::*;

    /// Every key of a JSON value, at any depth.
    fn keys_of(value: &Value, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (key, inner) in map {
                    out.push(key.clone());
                    keys_of(inner, out);
                }
            }
            Value::Array(items) => items.iter().for_each(|item| keys_of(item, out)),
            _ => {}
        }
    }

    /// 40 games, their ids `<prefix>-<i>`: p1 plays core-001 on its first turn and its chance then swings
    /// far one way or the other; a Rush Token core-015 made acts on its second turn. core-002 is never
    /// played.
    fn swing_records(prefix: &str) -> Vec<Value> {
        let mut records = Vec::new();
        for i in 0..40 {
            let won = i % 2 == 0;
            let after = if won { 5_000_000 } else { -5_000_000 };
            records.push(make_game_record(
                &format!("{prefix}-{i}"),
                RecordOptions {
                    source: Some("live"),
                    p1_deck: Some(vec!["core-001", "core-002", "core-015"]),
                    p1_played: Some(vec!["core-001"]),
                    winner: Some(if won { "p1" } else { "p2" }),
                    p1_swing_turns: Some(json!([
                        { "turn": 1, "eval": 0 },
                        {
                            "turn": 3,
                            "eval": after,
                            "acting": ["core-001", { "card": "core-t-rush", "maker": "core-015" }],
                        },
                    ])),
                    p1_created_used: Some(json!([{ "card": "core-t-rush", "turn": 1, "radiant": false }])),
                    ..RecordOptions::default()
                },
            ));
        }
        records
    }

    #[tokio::test]
    async fn r4552003_cards_answer_decisiveness_and_the_typical_value_and_no_swing() {
        let app = test_app().await;
        insert_records(&app, swing_records("rec-swing")).await;

        let (status, _, data) = get_public(&app, "/api/stats/cards").await;
        assert_eq!(status, 200);
        let played = find(&data["cards"], "id", &json!("core-001")).expect("core-001 is listed");
        let decisive = played["decisiveness"].as_f64().expect("a Decisiveness");
        assert!(decisive > 0.0 && decisive <= 1.0, "{decisive}");
        // Never played and never acting: no turn of its own to read.
        let quiet = find(&data["cards"], "id", &json!("core-002")).expect("core-002 is listed");
        assert_eq!(quiet["decisiveness"], Value::Null);
        // The typical card's value of each field, beside the tiers' centres.
        assert!(
            data["tiers"]["decisiveness"]
                .as_f64()
                .is_some_and(|typical| typical > 0.0)
        );
        assert!(
            data["tiers"]["created"]["decisiveness"]
                .as_f64()
                .is_some_and(|typical| typical > 0.0)
        );
        // The token its maker put onto the field reads its acting turns.
        let token = find(&data["created"], "id", &json!("core-t-rush")).expect("the Rush Token is listed");
        assert!(token["decisiveness"].as_f64().is_some(), "{token}");
        // No key of the response names a swing.
        let mut keys = Vec::new();
        keys_of(&data, &mut keys);
        assert!(
            keys.iter().all(|key| !key.to_lowercase().contains("swing")),
            "{keys:?}"
        );
    }

    // R5036000: every record is on one patch, so a live play version's span and the Cards tab's window
    // count the same turns, and its Decisiveness, in both scopes, is its card's there.
    #[tokio::test]
    async fn r5036000_the_versions_route_sends_a_rows_decisiveness_as_the_cards_tab_reads_it() {
        let app = test_app().await;
        insert_records(&app, swing_records("rec-pv-swing")).await;

        let (_, _, cards) = get_public(&app, "/api/stats/cards").await;
        let card = |id: &str| find(&cards["cards"], "id", &json!(id)).expect("the card is listed");
        let decisive = card("core-001")["decisiveness"].clone();
        assert!(decisive.is_number(), "{decisive}");
        for path in ["/api/stats/versions", "/api/stats/versions?scope=major"] {
            let (status, _, data) = get_public(&app, path).await;
            assert_eq!(status, 200, "{path}");
            let live = |id: &str| {
                data["versions"]
                    .as_array()
                    .expect("versions is a list")
                    .iter()
                    .find(|row| row["card"] == json!(id) && row["live"] == json!(true))
                    .cloned()
                    .expect("the card has a live play version")
            };
            assert_eq!(live("core-001")["decisiveness"], decisive, "{path}");
            assert_eq!(data["decisiveness"], cards["tiers"]["decisiveness"], "{path}");
            // Never played and never acting: no turn of its own to read.
            assert_eq!(live("core-002")["decisiveness"], Value::Null, "{path}");
        }
    }

    #[tokio::test]
    async fn stats_queries_refuse_nul_characters_with_bad_request() {
        let app = test_app().await;

        let (status, _, body) = get_public(&app, "/api/stats/players?search=%00").await;
        assert_eq!(status, 400);
        assert_eq!(
            body["error"]["message"],
            "a query parameter may not contain a NUL character"
        );

        let (status, _, body) = get_public(&app, "/api/stats/cards?patch=%00").await;
        assert_eq!(status, 400);
        assert_eq!(
            body["error"]["message"],
            "a query parameter may not contain a NUL character"
        );

        let (status, _, body) = get_public(&app, "/api/stats/cards/core-001?patch=%00").await;
        assert_eq!(status, 400);
        assert_eq!(
            body["error"]["message"],
            "a query parameter may not contain a NUL character"
        );

        let (status, _, body) = get_public(&app, "/api/stats/games?patch=%00").await;
        assert_eq!(status, 400);
        assert_eq!(
            body["error"]["message"],
            "a query parameter may not contain a NUL character"
        );
    }
}

mod r4681001_hall_of_fame {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use jackioh_server::api::catalog::load_patch_versions;
    use jackioh_server::config::HALL_OF_FAME_LIST_LEN;

    /// The cache's time to live, in milliseconds.
    fn ttl_ms() -> u64 {
        u64::try_from(CARD_STATS_CACHE_TTL_SECONDS).expect("the TTL is not negative") * 1000
    }

    /// What the fake store was asked to read: every `statsTotals.load`, and among them the loads of
    /// every patch's totals (R3067000), which it charges as `statsTotals.loadAllPatches` too.
    #[derive(Default)]
    struct Loads {
        all: AtomicUsize,
        full: AtomicUsize,
    }

    /// Counts the fake store's totals loads from here on, through its `on_call` hook.
    async fn count_loads(app: &App) -> Arc<Loads> {
        use jackioh_server::db::store::{Db, StoreError};
        let loads = Arc::new(Loads::default());
        let counter = Arc::clone(&loads);
        match &app.db {
            Db::Fake(data) => {
                data.lock().await.on_call = Some(Arc::new(move |method: &str| -> Result<(), StoreError> {
                    match method {
                        "statsTotals.load" => counter.all.fetch_add(1, Ordering::SeqCst),
                        "statsTotals.loadAllPatches" => counter.full.fetch_add(1, Ordering::SeqCst),
                        _ => 0,
                    };
                    Ok(())
                }));
            }
            Db::Pg(_) => panic!("the API tests run on the fake store"),
        }
        loads
    }

    /// The loads one request to `path` made, which answers 200, as (every load, those of every
    /// patch's totals); the counts start again after it.
    async fn reads_of(app: &Arc<App>, loads: &Loads, path: &str) -> (usize, usize) {
        let (status, _, _) = get_public(app, path).await;
        assert_eq!(status, 200, "{path}");
        (
            loads.all.swap(0, Ordering::SeqCst),
            loads.full.swap(0, Ordering::SeqCst),
        )
    }

    /// A live Best of 1 game on `patch` whose first seat, on `core-001` and `core-002`, `winner` wins.
    fn game(id: String, patch: &str, winner: &'static str) -> Value {
        make_game_record(
            &id,
            RecordOptions {
                source: Some("live"),
                mode: Some("bo1"),
                patch: Some(patch.to_string()),
                p1_deck: Some(vec!["core-001", "core-002"]),
                winner: Some(winner),
                ..RecordOptions::default()
            },
        )
    }

    /// A past shipped patch, which is not the current one: the newest, which anyone may read (R4677000).
    async fn past_patch() -> String {
        let current = current_patch();
        load_patch_versions()
            .await
            .into_iter()
            .rev()
            .find(|version| *version != current)
            .expect("a past patch")
    }

    /// One entry's two rows, their dates, and its balance line: the shape every list shares.
    fn assert_entry(entry: &Value) {
        for side in ["before", "after"] {
            let row = &entry[side];
            for key in [
                "id",
                "card",
                "name",
                "first",
                "last",
                "live",
                "patches",
                "games",
                "adjustedWinRate",
                "adjustedLow",
                "adjustedHigh",
                "tier",
                "hasEnoughGames",
            ] {
                assert!(row.get(key).is_some(), "{side}.{key} of {entry}");
            }
        }
        assert_eq!(entry["before"]["card"], entry["after"]["card"]);
        for key in ["beforeDate", "afterDate"] {
            assert!(entry[key].is_null() || entry[key].is_string(), "{key} of {entry}");
        }
        assert!(
            entry["balance"].is_null() || entry["balance"].is_string(),
            "balance of {entry}"
        );
        if entry["before"]["id"] == entry["after"]["id"] {
            assert_eq!(entry["balance"], Value::Null, "one face needs no line: {entry}");
        }
    }

    fn rate(row: &Value) -> Option<f64> {
        row["adjustedWinRate"].as_f64()
    }

    #[tokio::test(start_paused = true)]
    async fn r4681001_hall_of_fame_is_public_cached_and_shaped() {
        let past = past_patch().await;
        let current = current_patch();
        let app = test_app().await;
        // The floor's games on the past patch, all won, and on the current one, all lost: the live
        // versions read both spans, and versions closed before today read the past one's.
        let floor = count(CARD_STATS_MIN_SAMPLE);
        let mut records: Vec<Value> = (0..floor)
            .map(|i| game(format!("hall-old-{i}"), &past, "p1"))
            .collect();
        records.extend((0..floor).map(|i| game(format!("hall-now-{i}"), &current, "p2")));
        insert_records(&app, records).await;
        let loads = count_loads(&app).await;

        // The lists are cut from the snapshot the cards read: one load of every patch's totals.
        assert_eq!(reads_of(&app, &loads, "/api/stats/hall-of-fame").await, (1, 1));
        let (status, headers, data) = get_public(&app, "/api/stats/hall-of-fame").await;
        assert_eq!(status, 200);
        let ttl = format!("public, max-age={CARD_STATS_CACHE_TTL_SECONDS}");
        assert_eq!(cache_control(&headers), Some(ttl.as_str()));
        assert_eq!((&data["pilot"], &data["mode"]), (&json!("merged"), &json!("all")));
        assert_eq!(data["minSample"], json!(CARD_STATS_MIN_SAMPLE));
        assert!(data["bandLow"].as_f64().is_some());
        assert!(data["bandHigh"].as_f64().is_some());
        let top = count(HALL_OF_FAME_LIST_LEN);
        for list in ["broken", "nerfed", "buffed", "changed"] {
            let entries = data[list].as_array().expect("{list} is a list");
            assert!(entries.len() <= top, "{list}");
            for entry in entries {
                assert_entry(entry);
            }
        }
        // The definition cases live in the engine's tests on fabricated versions; here the shape of
        // what ranks: broken and changed are never empty on a history that changed cards, and every
        // ranked pair runs the ranked way.
        assert!(!data["broken"].as_array().expect("a list").is_empty());
        assert!(!data["changed"].as_array().expect("a list").is_empty());
        let broken = data["broken"].as_array().expect("a list");
        assert_eq!(broken[0]["before"]["hasEnoughGames"], json!(true));
        let lows: Vec<f64> = broken
            .iter()
            .map(|entry| entry["before"]["adjustedLow"].as_f64().expect("a bound"))
            .collect();
        let mut ordered = lows.clone();
        ordered.sort_by(|a, b| b.partial_cmp(a).expect("no NaN"));
        assert_eq!(lows, ordered, "Most broken ranks by the lower bound");
        for entry in data["nerfed"].as_array().expect("a list") {
            let (before, after) = (&entry["before"], &entry["after"]);
            assert!(
                rate(before).expect("a rate") > rate(after).expect("a rate"),
                "{entry}"
            );
            assert_eq!(entry["changesBetween"], Value::Null);
        }
        for entry in data["buffed"].as_array().expect("a list") {
            let (before, after) = (&entry["before"], &entry["after"]);
            assert!(
                rate(after).expect("a rate") > rate(before).expect("a rate"),
                "{entry}"
            );
            assert_eq!(entry["changesBetween"], Value::Null);
        }
        for entry in data["changed"].as_array().expect("a list") {
            assert!(entry["changesBetween"].as_u64().is_some(), "{entry}");
        }

        // `?mode=` names what the figures count; every game here is a Best of 1.
        let (_, _, bo1) = get_public(&app, "/api/stats/hall-of-fame?mode=bo1").await;
        assert_eq!((&bo1["pilot"], &bo1["mode"]), (&json!("merged"), &json!("bo1")));

        // Within the TTL a repeat read folds nothing new, whatever its key, and the versions read
        // off the same snapshot: the Hall of Fame's table is the versions' table.
        assert_eq!(reads_of(&app, &loads, "/api/stats/hall-of-fame").await, (0, 0));
        assert_eq!(
            reads_of(&app, &loads, "/api/stats/hall-of-fame?mode=bo1").await,
            (0, 0)
        );
        assert_eq!(reads_of(&app, &loads, "/api/stats/versions").await, (0, 0));
        // At the TTL the snapshot is loaded again.
        tokio::time::advance(Duration::from_millis(ttl_ms())).await;
        assert_eq!(reads_of(&app, &loads, "/api/stats/hall-of-fame").await, (1, 1));
    }
}

// R4676000: `GET /api/stats/cards/:id`'s `variants`: each choice of a Choose One, an X and an Embiggen card,
// at the page's patch and on its face.
mod r4676000_extended_stats {
    use super::*;

    const FLOOD: &str = "core-017"; // Flood: Radiant, Choose one of three.
    const DIVIDEND: &str = "core-024"; // Efficiency Dividend: X, and Choose one.
    const PREP: &str = "classic-035"; // Prep: an Embiggen Spell.
    const PLAIN: &str = "core-001";
    const DECK: [&str; 4] = [FLOOD, DIVIDEND, PREP, PLAIN];

    /// `count` live Best of 1 games, p1 winning the first `won`. With `variants`, p1 plays the four cards with
    /// them (Flood's, Dividend's, Prep's), Flood on its Radiant face when `radiant`; without, it never draws one.
    fn games(tag: &str, variants: Option<[Value; 3]>, radiant: bool, won: usize, count: usize) -> Vec<Value> {
        (0..count)
            .map(|i| {
                let (hand, played, flags, chosen) = match &variants {
                    Some([flood, dividend, prep]) => (
                        DECK.to_vec(),
                        DECK.to_vec(),
                        Some(vec![radiant, false, false, false]),
                        Some(json!([flood, dividend, prep, null])),
                    ),
                    None => (Vec::new(), Vec::new(), None, None),
                };
                make_game_record(
                    &format!("variants-{tag}-{i}"),
                    RecordOptions {
                        source: Some("live"),
                        mode: Some("bo1"),
                        p1_deck: Some(DECK.to_vec()),
                        p1_hand: Some(hand),
                        p1_played: Some(played),
                        p1_played_radiant: flags,
                        p1_played_variants: chosen,
                        p2_deck: Some(vec!["core-100"]),
                        p2_played: Some(Vec::new()),
                        winner: Some(if i < won { "p1" } else { "p2" }),
                        ..RecordOptions::default()
                    },
                )
            })
            .collect()
    }

    async fn seeded() -> Arc<App> {
        let app = test_app().await;
        let mut records = games(
            "a",
            Some([
                json!({ "modes": ["bounce all units"] }),
                json!({ "modes": ["deal"], "x": 2 }),
                json!({ "embiggened": true }),
            ]),
            true,
            24,
            30,
        );
        records.extend(games(
            "b",
            Some([
                json!({ "modes": ["destroy all enemy units"] }),
                json!({ "modes": ["heal"], "x": 6 }),
                json!({ "embiggened": false }),
            ]),
            false,
            12,
            30,
        ));
        records.extend(games(
            "c",
            Some([
                json!({ "modes": ["bounce all enemy units"] }),
                json!({ "modes": ["heal"], "x": 7 }),
                json!({ "embiggened": false }),
            ]),
            false,
            5,
            10,
        ));
        records.extend(games("unseen", None, false, 15, 30));
        insert_records(&app, records).await;
        app
    }

    fn kinds(drill: &Value) -> Vec<&str> {
        drill["variants"]
            .as_array()
            .expect("variants")
            .iter()
            .map(|group| group["kind"].as_str().expect("a kind"))
            .collect()
    }

    fn rows<'a>(drill: &'a Value, kind: &str) -> &'a Vec<Value> {
        drill["variants"]
            .as_array()
            .and_then(|groups| groups.iter().find(|group| group["kind"] == json!(kind)))
            .and_then(|group| group["rows"].as_array())
            .expect("the kind's rows")
    }

    fn keys(rows: &[Value]) -> Vec<&str> {
        rows.iter()
            .map(|row| row["key"].as_str().expect("a key"))
            .collect()
    }

    #[tokio::test]
    async fn r4676000_a_choose_one_card_answers_each_option_by_share_with_its_figures() {
        let app = seeded().await;
        let (status, _, drill) = get_public(&app, &format!("/api/stats/cards/{FLOOD}?mode=bo1")).await;
        assert_eq!(status, 200);
        assert_eq!(kinds(&drill), ["mode"]);
        let modes = rows(&drill, "mode");
        // By share, most chosen first; the 10-game option below the floor reads no tier.
        assert_eq!(
            keys(modes),
            [
                "bounce all units",
                "destroy all enemy units",
                "bounce all enemy units"
            ]
        );
        assert_eq!(modes[0]["label"], json!("bounce all units"));
        // Each option apart, for the page to name in the card's own words.
        assert_eq!(modes[0]["options"], json!(["bounce all units"]));
        assert_eq!(modes[0]["games"], json!(30));
        let share = modes[0]["share"].as_f64().expect("a share");
        assert!((share - 30.0 / 70.0).abs() < 1e-3, "{share}");
        assert_eq!(modes[0]["hasEnoughGames"], json!(true));
        assert!(modes[0]["tier"].is_string());
        // The option that won more reads above its sibling.
        let rate = |row: &Value| row["adjustedWinRate"].as_f64().expect("a rate");
        assert!(rate(&modes[0]) > rate(&modes[1]));
        assert!(modes[0]["adjustedLow"].as_f64().unwrap() < rate(&modes[0]));
        assert!(modes[0]["adjustedHigh"].as_f64().unwrap() > rate(&modes[0]));
        assert_eq!(modes[2]["hasEnoughGames"], json!(false));
        assert_eq!(modes[2]["tier"], Value::Null);
        // A card with no choice answers an empty list.
        let (_, _, plain) = get_public(&app, &format!("/api/stats/cards/{PLAIN}?mode=bo1")).await;
        assert_eq!(plain["variants"], json!([]));
    }

    #[tokio::test]
    async fn r4676000_an_x_card_groups_its_top_x_and_an_embiggen_card_splits_both_ways() {
        let app = seeded().await;
        let (_, _, dividend) = get_public(&app, &format!("/api/stats/cards/{DIVIDEND}?mode=bo1")).await;
        assert_eq!(kinds(&dividend), ["mode", "x"]);
        let x = rows(&dividend, "x");
        // X=6 and X=7 are one row, X ≥ 5, its 40 seats ahead of X=2's 30.
        assert_eq!(keys(x), ["top", "2"]);
        assert_eq!(x[0]["label"], json!("X ≥ 5"));
        assert_eq!(x[0]["games"], json!(40));
        assert_eq!(x[1]["label"], json!("X = 2"));
        // Only a mode row names options.
        assert!(x[0].get("options").is_none());
        assert_eq!(keys(rows(&dividend, "mode")), ["heal", "deal"]);

        let (_, _, prep) = get_public(&app, &format!("/api/stats/cards/{PREP}?mode=bo1")).await;
        assert_eq!(kinds(&prep), ["embiggen"]);
        let embiggen = rows(&prep, "embiggen");
        assert_eq!(keys(embiggen), ["notEmbiggened", "embiggened"]);
        assert_eq!(embiggen[1]["label"], json!("Embiggened"));
        assert_eq!(embiggen[1]["games"], json!(30));
    }

    #[tokio::test]
    async fn r4676000_the_face_and_the_patch_scope_the_variants() {
        let app = seeded().await;
        // Flood was played Radiant only with its first option: on Radiant that is its one row, all its plays.
        let (_, _, radiant) =
            get_public(&app, &format!("/api/stats/cards/{FLOOD}?mode=bo1&face=radiant")).await;
        let modes = rows(&radiant, "mode");
        assert_eq!(keys(modes), ["bounce all units"]);
        assert_eq!(modes[0]["share"], json!(1.0));
        let (_, _, normal) =
            get_public(&app, &format!("/api/stats/cards/{FLOOD}?mode=bo1&face=normal")).await;
        assert_eq!(
            keys(rows(&normal, "mode")),
            ["destroy all enemy units", "bounce all enemy units"]
        );
        // At a patch with none of these games, there is nothing to split.
        let (_, _, page) = get_public(&app, "/api/stats/cards").await;
        let past = page["patches"][1]["versions"]
            .as_array()
            .and_then(|versions| versions.last())
            .and_then(Value::as_str)
            .expect("a past patch anyone reads")
            .to_string();
        let (status, _, before) =
            get_public(&app, &format!("/api/stats/cards/{FLOOD}?mode=bo1&patch={past}")).await;
        assert_eq!(status, 200);
        assert_eq!(before["variants"], json!([]));
    }
}

// R4679000: `GET /api/stats/cards/:id`'s `outputs`: each card a random-output card and a Discover card made,
// read against the card's other outputs, at the page's patch and on its face.
mod r4679000_outputs {
    use super::*;

    const PANCAKE: &str = "classicplus-012"; // The Mother Pancake: a random Pancake each end of turn.
    const BLUEPRINTS: &str = "meditative-097"; // Jlockeed's Evil Blueprints: Discover a building.
    const PLAIN: &str = "core-001";

    /// A `createdUsed` entry: `card`, made by `source` on its Radiant face or not.
    fn made(card: &str, source: &str, radiant: bool) -> Value {
        json!({ "card": card, "turn": 3, "radiant": false, "source": { "card": source, "radiant": radiant } })
    }

    /// `count` live Best of 1 games of p1 playing `source`, p1 winning the first `won`, its seat making `used`
    /// and asking `picks`.
    fn games(
        tag: &str,
        source: &'static str,
        used: Value,
        picks: Option<Value>,
        won: usize,
        count: usize,
    ) -> Vec<Value> {
        (0..count)
            .map(|i| {
                make_game_record(
                    &format!("outputs-{tag}-{i}"),
                    RecordOptions {
                        source: Some("live"),
                        mode: Some("bo1"),
                        p1_deck: Some(vec![source, PLAIN]),
                        p1_created_used: Some(used.clone()),
                        p1_picks: picks.clone(),
                        p2_deck: Some(vec!["core-100"]),
                        p2_played: Some(Vec::new()),
                        winner: Some(if i < won { "p1" } else { "p2" }),
                        ..RecordOptions::default()
                    },
                )
            })
            .collect()
    }

    fn outputs(drill: &Value) -> &Vec<Value> {
        drill["outputs"].as_array().expect("outputs")
    }

    fn keys(rows: &[Value]) -> Vec<&str> {
        rows.iter()
            .map(|row| row["key"].as_str().expect("a key"))
            .collect()
    }

    fn rate(row: &Value) -> f64 {
        row["adjustedWinRate"].as_f64().expect("a rate")
    }

    #[tokio::test]
    async fn r4679000_a_random_output_card_answers_each_output_against_its_siblings() {
        let app = test_app().await;
        let pancake = |n: u8| format!("{PANCAKE}-{n}");
        let mut records = games(
            "a",
            PANCAKE,
            json!([made(&pancake(1), PANCAKE, true)]),
            None,
            24,
            30,
        );
        records.extend(games(
            "b",
            PANCAKE,
            json!([made(&pancake(2), PANCAKE, false)]),
            None,
            12,
            30,
        ));
        records.extend(games(
            "c",
            PANCAKE,
            json!([made(&pancake(3), PANCAKE, false)]),
            None,
            5,
            10,
        ));
        insert_records(&app, records).await;

        let (status, _, drill) = get_public(&app, &format!("/api/stats/cards/{PANCAKE}?mode=bo1")).await;
        assert_eq!(status, 200);
        let rows = outputs(&drill);
        // Best adjusted first; the 10-game Pancake below the floor after both.
        assert_eq!(keys(rows), [pancake(1), pancake(2), pancake(3)]);
        let devour = &rows[0];
        assert_eq!(devour["label"], json!("Devour"));
        assert_eq!(devour["rarity"], json!("Token"));
        assert_eq!(devour["cost"], json!(0));
        assert_eq!(devour["games"], json!(30));
        let share = devour["share"].as_f64().expect("a share");
        assert!((share - 30.0 / 70.0).abs() < 1e-3, "{share}");
        assert_eq!(devour["hasEnoughGames"], json!(true));
        assert!(devour["tier"].is_string());
        assert!(devour["adjustedLow"].as_f64().unwrap() < rate(devour));
        assert!(devour["adjustedHigh"].as_f64().unwrap() > rate(devour));
        // Devour's seats won more than its siblings': it reads above the Pancake that won less.
        assert!(rate(devour) > rate(&rows[1]));
        assert_eq!(rows[2]["hasEnoughGames"], json!(false));
        assert_eq!(rows[2]["tier"], Value::Null);
        // No Pancake was offered: no row names offers or a pick rate.
        assert!(
            rows.iter()
                .all(|row| row.get("offered").is_none() && row.get("pickRate").is_none())
        );

        // Devour was made only by a Radiant Mother Pancake: on Radiant it is every output, with no sibling.
        let (_, _, radiant) =
            get_public(&app, &format!("/api/stats/cards/{PANCAKE}?mode=bo1&face=radiant")).await;
        let rows = outputs(&radiant);
        assert_eq!(keys(rows), [pancake(1)]);
        assert_eq!(rows[0]["share"], json!(1.0));
        assert_eq!(rows[0]["adjustedWinRate"], Value::Null);
        let (_, _, normal) =
            get_public(&app, &format!("/api/stats/cards/{PANCAKE}?mode=bo1&face=normal")).await;
        assert_eq!(keys(outputs(&normal)), [pancake(2), pancake(3)]);
        // At a patch with none of these games, it made nothing.
        let (_, _, page) = get_public(&app, "/api/stats/cards").await;
        let past = page["patches"][1]["versions"]
            .as_array()
            .and_then(|versions| versions.last())
            .and_then(Value::as_str)
            .expect("a past patch anyone reads")
            .to_string();
        let (status, _, before) =
            get_public(&app, &format!("/api/stats/cards/{PANCAKE}?mode=bo1&patch={past}")).await;
        assert_eq!(status, 200);
        assert_eq!(before["outputs"], json!([]));

        // A card that made nothing answers an empty list.
        let (_, _, plain) = get_public(&app, &format!("/api/stats/cards/{PLAIN}?mode=bo1")).await;
        assert_eq!(plain["outputs"], json!([]));
    }

    #[tokio::test]
    async fn r4679000_a_discover_card_answers_each_offer_with_its_pick_rate() {
        let app = test_app().await;
        let building = |n: u8| format!("{BLUEPRINTS}-{n}");
        let pick = |offered: [u8; 3], picked: u8| {
            json!([{
                "source": { "card": BLUEPRINTS, "radiant": false },
                "offered": offered.map(building),
                "picked": [building(picked)],
            }])
        };
        let mut records = games(
            "a",
            BLUEPRINTS,
            json!([made(&building(1), BLUEPRINTS, false)]),
            Some(pick([1, 2, 3], 1)),
            20,
            30,
        );
        records.extend(games(
            "b",
            BLUEPRINTS,
            json!([made(&building(2), BLUEPRINTS, false)]),
            Some(pick([1, 2, 4], 2)),
            8,
            20,
        ));
        records.extend(games(
            "c",
            BLUEPRINTS,
            json!([made(&building(1), BLUEPRINTS, false)]),
            Some(pick([1, 2, 3], 1)),
            5,
            10,
        ));
        insert_records(&app, records).await;

        let (status, _, drill) = get_public(&app, &format!("/api/stats/cards/{BLUEPRINTS}?mode=bo1")).await;
        assert_eq!(status, 200);
        let rows = outputs(&drill);
        // The two built read against each other; the two never picked come after, by id.
        assert_eq!(keys(rows), [building(1), building(2), building(3), building(4)]);
        assert!(rate(&rows[0]) > rate(&rows[1]));
        let pick_rate = |row: &Value| row["pickRate"].as_f64().expect("a pick rate");
        assert_eq!(rows[0]["offered"], json!(60));
        assert!((pick_rate(&rows[0]) - 40.0 / 60.0).abs() < 1e-3);
        assert_eq!(rows[1]["offered"], json!(60));
        assert!((pick_rate(&rows[1]) - 20.0 / 60.0).abs() < 1e-3);
        // School was offered in 40 seat-games and never picked: no seat built it, so it has no rate.
        let school = &rows[2];
        assert_eq!(school["label"], json!("School"));
        assert_eq!(school["offered"], json!(40));
        assert_eq!(school["pickRate"], json!(0.0));
        assert_eq!(school["games"], json!(0));
        assert_eq!(school["share"], json!(0.0));
        assert_eq!(school["adjustedWinRate"], Value::Null);
        assert_eq!(school["tier"], Value::Null);
        assert_eq!(rows[3]["offered"], json!(20));
    }
}

// R4707001: `GET /api/stats/cards/:id`'s `rolled`: each outcome a Call to Chaos and a dice card rolled, read
// against the card's other outcomes, with its expected share beside its share, at the page's patch and face.
mod r4707001_rolled {
    use super::*;

    const CHAOS: &str = "core-095"; // Call to Chaos (Core Edition): one random effect of ten.
    const CLASSIC_CHAOS: &str = "classicplus-073"; // Call to Chaos (Classic+ Edition), a Recursion's cast.
    const DOOM: &str = "stinkyscape-154"; // Dice of Doom: a d100, which does something on a 67.
    const PLAIN: &str = "core-001";

    /// `count` live Best of 1 games of p1 playing `card` on its normal face once, rolling `rolled`, p1
    /// winning the first `won`.
    fn games(tag: &str, card: &'static str, rolled: Value, won: usize, count: usize) -> Vec<Value> {
        (0..count)
            .map(|i| {
                make_game_record(
                    &format!("rolled-{tag}-{i}"),
                    RecordOptions {
                        source: Some("live"),
                        mode: Some("bo1"),
                        p1_deck: Some(vec![card, PLAIN]),
                        p1_played: Some(vec![card]),
                        p1_played_radiant: Some(vec![false]),
                        p1_played_variants: Some(json!([{ "rolled": rolled }])),
                        p2_deck: Some(vec!["core-100"]),
                        p2_played: Some(Vec::new()),
                        winner: Some(if i < won { "p1" } else { "p2" }),
                        ..RecordOptions::default()
                    },
                )
            })
            .collect()
    }

    fn effect(card: &str, at: i32, label: &str) -> Value {
        json!({ "kind": "effect", "card": card, "picked": [at], "labels": [label], "of": 10 })
    }

    fn die(kept: i32) -> Value {
        json!({ "kind": "die", "card": DOOM, "sides": 100, "lucky": 0, "hit": { "from": 67, "to": 67 }, "kept": [kept] })
    }

    fn group<'a>(drill: &'a Value, kind: &str) -> &'a Vec<Value> {
        drill["rolled"]
            .as_array()
            .and_then(|groups| groups.iter().find(|group| group["kind"] == json!(kind)))
            .and_then(|group| group["rows"].as_array())
            .expect("the kind's rows")
    }

    fn keys(rows: &[Value]) -> Vec<&str> {
        rows.iter()
            .map(|row| row["key"].as_str().expect("a key"))
            .collect()
    }

    fn number(row: &Value, field: &str) -> f64 {
        row[field].as_f64().unwrap_or_else(|| panic!("{field} of {row}"))
    }

    #[tokio::test]
    async fn r4707001_a_call_to_chaos_answers_each_rolled_effect_with_its_expected_share() {
        let app = test_app().await;
        let mut records = games(
            "heal",
            CHAOS,
            json!([effect(CHAOS, 1, "Heal your hero 30")]),
            24,
            30,
        );
        records.extend(games(
            "add",
            CHAOS,
            json!([effect(
                CHAOS,
                3,
                "Add 3 random cards to your hand, which cost (0)"
            )]),
            32,
            60,
        ));
        // Ten plays rolled the Recursion, whose Classic+ cast rolled one of its own table's (R28).
        records.extend(games(
            "chain",
            CHAOS,
            json!([
                effect(CHAOS, 3, "Add 3 random cards to your hand, which cost (0)"),
                effect(CLASSIC_CHAOS, 2, "Gain 5 mana"),
            ]),
            0,
            10,
        ));
        insert_records(&app, records).await;

        let (status, _, drill) = get_public(&app, &format!("/api/stats/cards/{CHAOS}?mode=bo1")).await;
        assert_eq!(status, 200);
        let kinds: Vec<&str> = drill["rolled"]
            .as_array()
            .expect("rolled")
            .iter()
            .map(|group| group["kind"].as_str().expect("a kind"))
            .collect();
        assert_eq!(kinds, ["effect"]);
        let rows = group(&drill, "effect");
        // Best adjusted first; the Classic+ edition's entry, which has no sibling to read against, last.
        assert_eq!(keys(rows), ["core-095:1", "core-095:3", "classicplus-073:2"]);
        let heal = &rows[0];
        assert_eq!(heal["label"], json!("Heal your hero 30"));
        assert_eq!(heal["games"], json!(30));
        assert!((number(heal, "share") - 30.0 / 100.0).abs() < 1e-3, "{heal}");
        assert!((number(&rows[1], "share") - 70.0 / 100.0).abs() < 1e-3);
        // One in ten of the Core table's, whatever came up.
        assert!((number(heal, "expected") - 0.1).abs() < 1e-9, "{heal}");
        assert!((number(&rows[1], "expected") - 0.1).abs() < 1e-9);
        assert_eq!(heal["hasEnoughGames"], json!(true));
        assert!(heal["tier"].is_string());
        assert!(number(heal, "adjustedWinRate") > number(&rows[1], "adjustedWinRate"));
        assert!(number(heal, "adjustedLow") < number(heal, "adjustedWinRate"));
        assert!(number(heal, "adjustedHigh") > number(heal, "adjustedWinRate"));
        // A Recursion's cast's entry names its edition, as the catalog does.
        let chain = &rows[2];
        assert_eq!(
            chain["label"],
            json!("Call to Chaos (Classic+ Edition): Gain 5 mana")
        );
        assert_eq!(chain["share"], json!(1.0));
        assert_eq!(chain["adjustedWinRate"], Value::Null);
        assert_eq!(chain["hasEnoughGames"], json!(false));

        // None of it was rolled on the Radiant face, and a card that rolls nothing answers an empty list.
        let (_, _, radiant) =
            get_public(&app, &format!("/api/stats/cards/{CHAOS}?mode=bo1&face=radiant")).await;
        assert_eq!(radiant["rolled"], json!([]));
        let (_, _, plain) = get_public(&app, &format!("/api/stats/cards/{PLAIN}?mode=bo1")).await;
        assert_eq!(plain["rolled"], json!([]));
    }

    #[tokio::test]
    async fn r4707001_dice_of_doom_answers_67_and_not_67() {
        let app = test_app().await;
        let mut records = games("hit", DOOM, json!([die(67)]), 25, 30);
        records.extend(games("miss", DOOM, json!([die(12)]), 30, 70));
        insert_records(&app, records).await;

        let (status, _, drill) = get_public(&app, &format!("/api/stats/cards/{DOOM}?mode=bo1")).await;
        assert_eq!(status, 200);
        let rows = group(&drill, "die");
        assert_eq!(keys(rows), ["stinkyscape-154:hit", "stinkyscape-154:miss"]);
        assert_eq!(rows[0]["label"], json!("Lands on 67"));
        assert_eq!(rows[1]["label"], json!("Misses 67"));
        // Expected 1 in 100 against 30 in 100 observed: the gap a test fixture makes, not a balance figure.
        assert!((number(&rows[0], "expected") - 0.01).abs() < 1e-9);
        assert!((number(&rows[1], "expected") - 0.99).abs() < 1e-9);
        assert!((number(&rows[0], "share") - 0.3).abs() < 1e-9);
        assert!(number(&rows[0], "adjustedWinRate") > number(&rows[1], "adjustedWinRate"));
        assert!(drill["rolled"].as_array().is_some_and(|groups| groups.len() == 1));
    }
}

mod r4965000_bounded_snapshot {
    use super::*;
    use indexmap::{IndexMap, IndexSet};
    use jackioh_cards::{deck_play_versions, patch_history};
    use jackioh_engine::GameRecord;
    use jackioh_engine::wire::stats::{
        GameMode, ModeFilter, PatchStep, PatchWindow, PilotFilter, PlayVersion, PlayVersionQuery,
        StatsTotals, WindowQuery, WindowRequest, patch_windows, play_version_figures, windowed_card_stats,
    };
    use jackioh_server::api::stats::{public_patch_count, reach_of, snapshot_filing};
    use jackioh_server::config::CARD_STATS_MIN_SAMPLE;
    use jackioh_server::db::store::{CardFiling, CardRows, file_card_totals};

    const MODES: [&str; 3] = ["bo1", "bo3", "random"];

    /// A game on `patch` whose seats hold `p1` and `p2`, of the source, mode and pilots `n` turns through.
    fn game_on(n: usize, patch: &str, p1: &[&str], p2: &[&str]) -> GameRecord {
        let (source, pilots): (&'static str, [&'static str; 2]) = match n % 3 {
            0 => ("dev", ["ai", "ai"]),
            1 => ("live", ["human", "human"]),
            _ => ("live", ["human", "ai"]),
        };
        let mut record = make_game_record(
            &format!("r4965000-{n}"),
            RecordOptions {
                source: Some(source),
                mode: Some(MODES[n % MODES.len()]),
                patch: Some(patch.to_string()),
                pilots: Some(pilots),
                winner: Some(if n.is_multiple_of(2) { "p1" } else { "p2" }),
                ..RecordOptions::default()
            },
        );
        record["game"]["seats"]["p1"] = seat(p1, p1, &p1[..2], None, None);
        record["game"]["seats"]["p2"] = seat(p2, p2, &p2[..2], None, None);
        from(record)
    }

    /// Every shipped version's totals: two games on each, of every source, mode and pilot by turns, on
    /// decks of the deck cards the first version shipped (a card a version did not hold counts too, and
    /// is held by no play version there), with a card no deck can hold among them.
    fn every_version_totals(history: &[PatchStep], plays: &[PlayVersion]) -> StatsTotals {
        let first = history[0].version.as_str();
        let mut cards: Vec<&str> = plays
            .iter()
            .filter(|play| play.first() == first)
            .map(|play| play.card.as_str())
            .collect();
        cards.dedup();
        assert!(
            cards.len() >= 8,
            "{} deck cards from the first version",
            cards.len()
        );
        let mut totals = StatsTotals::default();
        for (at, step) in history.iter().enumerate() {
            for turn in 0..2 {
                let n = at * 2 + turn;
                let p1 = [cards[n % 4], cards[4], "r4965000-no-deck-card", cards[5]];
                let p2 = [cards[6], cards[7], cards[(n + 1) % 4], cards[5]];
                totals.fold(&game_on(n, &step.version, &p1, &p2), false);
            }
        }
        totals
    }

    fn windows_of(history: &[PatchStep], totals: &StatsTotals) -> Vec<PatchWindow> {
        patch_windows(
            history,
            |version| totals.games(version),
            |version| totals.live_games(version),
        )
    }

    /// Every figure a play version's table sends is what the whole totals give, under every pilot and
    /// match type.
    fn same_play_versions(
        filed: &StatsTotals,
        whole: &StatsTotals,
        history: &[PatchStep],
        plays: &[PlayVersion],
    ) {
        for live_pilot in [PilotFilter::Unified, PilotFilter::Human, PilotFilter::Ai] {
            for mode in [
                ModeFilter::All,
                ModeFilter::Only(GameMode::Bo1),
                ModeFilter::Only(GameMode::Bo3),
                ModeFilter::Only(GameMode::Random),
                ModeFilter::Constructed,
            ] {
                let query = PlayVersionQuery {
                    versions: plays,
                    history,
                    live_pilot,
                    mode,
                    min_sample: CARD_STATS_MIN_SAMPLE,
                };
                assert_eq!(
                    play_version_figures(filed, &query),
                    play_version_figures(whole, &query),
                    "{live_pilot:?} {mode:?}"
                );
            }
        }
    }

    /// Every card's window at `at` is what the whole totals give, under every pilot and match type.
    fn same_windows(filed: &StatsTotals, whole: &StatsTotals, history: &[PatchStep], at: &str) {
        for live_pilot in [PilotFilter::Unified, PilotFilter::Human] {
            for mode in [ModeFilter::All, ModeFilter::Only(GameMode::Bo3)] {
                let query = WindowQuery {
                    history,
                    at,
                    live_pilot,
                    mode,
                    request: WindowRequest::Gated,
                };
                assert_eq!(
                    windowed_card_stats(filed, &query),
                    windowed_card_stats(whole, &query),
                    "{at} {live_pilot:?} {mode:?}"
                );
            }
        }
    }

    #[test]
    fn r4965000_the_snapshot_keeps_its_reach_as_filed_and_one_total_per_play_version_before_it() {
        let history = patch_history();
        let plays = deck_play_versions();
        let whole = every_version_totals(&history, &plays);
        let filing = snapshot_filing(&history, &whole, &plays, &current_patch());
        let filed = file_card_totals(whole.clone(), &CardRows::Filed(filing.clone()));

        // Its reach: the patches anyone may read and the window before the oldest of them.
        let windows = windows_of(&history, &whole);
        let public = public_patch_count(&windows);
        assert!(public + 1 < windows.len(), "patches older than the reach");
        assert_eq!(
            filing.versions.get_index_of(&windows[public].versions[0]),
            Some(filing.kept_from)
        );
        assert_eq!(filing.kept_to, history.len() - 1);
        let place = |patch: &str| filing.versions.get_index_of(patch).unwrap_or(usize::MAX);

        // From there on every total is kept as filed.
        for (key, cards) in &whole.cards {
            if place(&key.patch) >= filing.kept_from {
                assert_eq!(filed.cards.get(key), Some(cards), "{key:?}");
            }
        }

        // Before it, a deck card's totals are one per play version and key, under the play version's
        // first version, and no other card's are kept.
        let firsts: IndexSet<(&str, &str)> = plays
            .iter()
            .map(|play| (play.card.as_str(), play.first()))
            .collect();
        let rows_before = |totals: &StatsTotals| -> usize {
            totals
                .cards
                .iter()
                .filter(|(key, _)| place(&key.patch) < filing.kept_from)
                .map(|(_, cards)| cards.len())
                .sum()
        };
        for (key, cards) in &filed.cards {
            if place(&key.patch) < filing.kept_from {
                for card in cards.keys() {
                    assert!(
                        firsts.contains(&(card.as_str(), key.patch.as_str())),
                        "{card} under {key:?}"
                    );
                }
            }
        }
        let keys: IndexSet<_> = whole
            .cards
            .keys()
            .map(|key| (key.source, key.mode, key.pilot))
            .collect();
        let play_versions_before = plays
            .iter()
            .filter(|play| place(play.first()) < filing.kept_from)
            .count();
        assert!(rows_before(&filed) <= play_versions_before * keys.len());
        assert!(
            rows_before(&filed) * 4 < rows_before(&whole),
            "{} totals before the reach folded into {}",
            rows_before(&whole),
            rows_before(&filed)
        );

        // Every figure anyone is sent from it is what the whole totals give: the play versions' table,
        // and every card's window at each patch anyone may read.
        same_play_versions(&filed, &whole, &history, &plays);
        for window in &windows[..public] {
            for at in &window.versions {
                same_windows(&filed, &whole, &history, at);
            }
        }
    }

    #[test]
    fn r4965000_a_patch_past_the_snapshot_reads_its_reach_alone_as_the_whole_totals_give() {
        let history = patch_history();
        let plays = deck_play_versions();
        let whole = every_version_totals(&history, &plays);
        let filing = snapshot_filing(&history, &whole, &plays, &current_patch());
        let windows = windows_of(&history, &whole);
        let public = public_patch_count(&windows);

        // The newest patch past the snapshot's, which only a dev account reads (R4677000), and the oldest.
        for window in [&windows[public + 1], windows.last().expect("a window")] {
            let at = window.versions.last().expect("a version");
            let (from, to) = reach_of(&windows, &filing.versions, at).expect("a shipped version");
            assert!(from < filing.kept_from, "{at} is past the snapshot");
            let rows = CardRows::Filed(CardFiling {
                versions: filing.versions.clone(),
                kept_from: from,
                kept_to: to,
                also_kept: Vec::new(),
                folded: IndexMap::new(),
            });
            let own = file_card_totals(whole.clone(), &rows);
            for key in own.cards.keys() {
                let place = filing
                    .versions
                    .get_index_of(&key.patch)
                    .expect("a shipped version");
                assert!((from..=to).contains(&place), "{key:?} outside {at}'s reach");
            }
            assert_eq!(own.records, whole.records);
            same_windows(&own, &whole, &history, at);
        }
    }
}
