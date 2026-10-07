//! Public card and player statistics page API (SPEC §9.11, R654). Port of
//! `apps/server/src/api/stats.ts`.
//!
//! Exposes:
//!  - GET /api/stats/cards: public card win rates, provisional gate at 1000 live games (R654).
//!  - GET /api/stats/cards/:id: card drill-down (patches, turn played, co-played cards).
//!  - GET /api/stats/player: caller's full stats (auth: `Active`).
//!  - PUT /api/stats/player: caller's stats upsert (auth: `Active`).
//!  - GET /api/stats/players: public player summaries (auth: `None`).

use std::cmp::Ordering;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use indexmap::{IndexMap, IndexSet};
use serde::Serialize;
use serde_json::{Map, Value, json};

use jackioh_engine::wire::stats::{
    CardStatsFilter, GameMode, GameRecord, GameSource, PilotFilter, SourceFilter, Tally, card_stats, win_rate,
};
use jackioh_engine::{CardCost, CardDef, PLAYER_IDS, Winner};

use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, now_ms, ok_of, to_json};
use crate::app::App;
use crate::config::{
    CARD_STATS_CACHE_TTL_SECONDS, CARD_STATS_CURVE_TOP, CARD_STATS_MIN_SAMPLE, PLAYER_STATS_BYTES_MAX,
    PLAYER_STATS_CACHE_TTL_SECONDS, PLAYER_STATS_PAGE_LIMIT, PUBLIC_STATS_MIN_LIVE_GAMES,
};
use crate::db::store::{GameRecordQuery, PlayerStatsListOptions, Profile};

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicCardStat {
    pub id: String,
    pub name: String,
    pub cost: i32,
    pub rarity: String,
    pub set: String,
    pub games: i64,
    pub win_rate: Option<f64>,
    pub drawn_games: i64,
    pub drawn_win_rate: Option<f64>,
    pub played_games: i64,
    pub played_win_rate: Option<f64>,
    pub play_rate: f64,
    pub has_enough_games: bool,
}

/// Which records the public figures were counted from.
#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase")]
pub enum PublicStatsSource {
    Provisional,
    Live,
    Dev,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicStatsGate {
    pub cleared: bool,
    pub live_games: i64,
    pub min_live_games: i64,
}

/// The best or the worst card above the sample threshold.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicStatsExtreme {
    pub id: String,
    pub name: String,
    pub win_rate: f64,
    pub games: i64,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicStatsSummary {
    pub total_games: i64,
    pub live_games: i64,
    pub active_patch: String,
    pub source: PublicStatsSource,
    pub best_card: Option<PublicStatsExtreme>,
    pub worst_card: Option<PublicStatsExtreme>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PublicStatsCardsResponse {
    pub patch: String,
    pub previous_patch: Option<String>,
    pub gate: PublicStatsGate,
    pub source: PublicStatsSource,
    pub source_label: String,
    pub min_sample: i64,
    pub total_games: i64,
    pub cards: Vec<PublicCardStat>,
    pub summary: PublicStatsSummary,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DrillDownCard {
    pub id: String,
    pub name: String,
    pub cost: i32,
    pub rarity: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PatchRate {
    pub patch: String,
    pub games: i64,
    pub win_rate: Option<f64>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TurnRate {
    pub turn: i32,
    pub games: i64,
    pub win_rate: Option<f64>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CoPlayedRate {
    pub id: String,
    pub name: String,
    pub games: i64,
    pub win_rate: Option<f64>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CardDrillDownResponse {
    pub card: DrillDownCard,
    pub patches: Vec<PatchRate>,
    pub by_turn: Vec<TurnRate>,
    pub co_played: Vec<CoPlayedRate>,
}

fn cached_ok(body: Value, max_age_seconds: i64) -> Response {
    let mut response = Response::new(Body::from(body.to_string()));
    *response.status_mut() = StatusCode::OK;
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("application/json; charset=utf-8"));
    if let Ok(value) = HeaderValue::from_str(&format!("public, max-age={max_age_seconds}")) {
        headers.insert(header::CACHE_CONTROL, value);
    }
    response
}

/// All patch versions from `patches.json` in release order (TS `loadPatchVersions`, a private copy
/// over the file compiled in from `crates/cards/patches/`).
fn load_patch_versions() -> Vec<String> {
    let Ok(parsed) = serde_json::from_str::<Value>(include_str!("../../../cards/patches/patches.json")) else {
        return Vec::new();
    };
    let Some(entries) = parsed.as_array() else {
        return Vec::new();
    };
    entries
        .iter()
        .map(|entry| entry.get("version").and_then(Value::as_str).unwrap_or("").to_string())
        .filter(|version| !version.is_empty())
        .collect()
}

/// R654: tutorial games are never counted. No game mode is `tutorial` today; the guard is TS's.
fn is_tutorial(record: &GameRecord) -> bool {
    serde_json::to_value(&record.mode).ok().as_ref().and_then(Value::as_str) == Some("tutorial")
}

/// A card's cost as a number: its printed cost, the base of an embiggen cost, 0 for X.
fn numeric_cost(def: &CardDef) -> i32 {
    match def.cost {
        CardCost::Fixed(cost) => cost,
        CardCost::Embiggen { base, .. } => base,
        CardCost::X => 0,
    }
}

/// TS `Number(text)` on a query parameter: NaN as none.
fn js_number(text: &str) -> Option<f64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Some(0.0);
    }
    match trimmed {
        "Infinity" | "+Infinity" => return Some(f64::INFINITY),
        "-Infinity" => return Some(f64::NEG_INFINITY),
        _ => {}
    }
    for (prefix, radix) in [("0x", 16), ("0X", 16), ("0o", 8), ("0O", 8), ("0b", 2), ("0B", 2)] {
        if let Some(digits) = trimmed.strip_prefix(prefix) {
            return u64::from_str_radix(digits, radix).ok().map(|value| value as f64);
        }
    }
    if !trimmed.chars().all(|ch| ch.is_ascii_digit() || matches!(ch, '.' | 'e' | 'E' | '+' | '-')) {
        return None;
    }
    trimmed.parse::<f64>().ok()
}

/// TS `a.name.localeCompare(b.name)`, approximated without a collator: case-insensitive first,
/// then as written.
fn locale_compare(a: &str, b: &str) -> Ordering {
    a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b))
}

/// A tally's games and wins as `i64`s.
fn tally_of(tally: &Tally) -> (i64, i64) {
    (tally.games as i64, tally.wins as i64)
}

/// The caller behind a route that declares `AuthLevel::Active` (a private copy of
/// `collection.rs`'s `callerProfile`).
fn caller_profile(req: &Req) -> Result<&Profile, ApiError> {
    req.caller.as_ref().map(|caller| &caller.profile).ok_or_else(|| bad_request("this endpoint needs a signed-in profile"))
}

/// A query parameter, trimmed, when it is present and not empty after trimming.
fn query_trimmed(req: &Req, name: &str) -> Option<String> {
    req.query.get(name).map(|value| value.trim().to_string()).filter(|value| !value.is_empty())
}

/// The game records of one patch that `source` reads (`SourceFilter::All` for both sources).
async fn records_for(app: &App, source: SourceFilter, patch: &str) -> Result<Vec<GameRecord>, ApiError> {
    let mut tx = app.db.begin(None).await?;
    let records = tx
        .game_records_list(&GameRecordQuery { source, mode: None::<GameMode>, patch: Some(patch.to_string()) })
        .await?;
    tx.commit().await?;
    Ok(records)
}

// TS's `createStatsRoutes()`, in its order, is five rows of `app.rs`'s `ROUTES`:
//   GET /api/stats/cards → get_cards (None);      GET /api/stats/cards/:id → get_card (None);
//   GET /api/stats/player → get_player (Active);  PUT /api/stats/player → put_player (Active);
//   GET /api/stats/players → get_players (None).

/// GET /api/stats/cards
/// Public card win-rate data. Applies R654 publication gate:
/// - AI games pad the stats until the current patch has logged 1000 live games.
/// - At and above 1000 live games, live games only, AI games ignored.
/// - Minimum sample threshold: below 20 games, hasEnoughGames is false.
pub async fn get_cards(app: &Arc<App>, req: Req) -> ApiResult {
    let versions = load_patch_versions();
    let current_patch = app.catalog.version.clone();
    let requested_patch = query_trimmed(&req, "patch").unwrap_or(current_patch);
    let source_param = req.query.get("source").map(|value| value.trim().to_lowercase());

    let patch_index = versions.iter().position(|version| *version == requested_patch);
    let previous_patch = match patch_index {
        Some(index) if index > 0 => versions.get(index - 1).cloned(),
        _ => None,
    };

    // Query all records for the requested patch
    let patch_records = records_for(app, SourceFilter::All, &requested_patch).await?;

    // Filter live games (tutorial games excluded per R654)
    let live_records: Vec<GameRecord> = patch_records
        .iter()
        .filter(|record| record.source == GameSource::Live && !is_tutorial(record))
        .cloned()
        .collect();
    let live_games_count = live_records.len() as i64;

    // R654 publication gate: exactly 1000 live games required to clear the gate
    let cleared = live_games_count >= PUBLIC_STATS_MIN_LIVE_GAMES as i64;

    let (source, source_label, records_to_count): (PublicStatsSource, &str, Vec<GameRecord>) = if cleared {
        // When cleared, strictly ignore AI development games (no toggle to bring them back)
        (PublicStatsSource::Live, "Live games", live_records)
    } else if source_param.as_deref() == Some("live") {
        (PublicStatsSource::Live, "Live games", live_records)
    } else if matches!(source_param.as_deref(), Some("dev") | Some("ai")) {
        let dev: Vec<GameRecord> = patch_records
            .iter()
            .filter(|record| record.source == GameSource::Dev && !is_tutorial(record))
            .cloned()
            .collect();
        (PublicStatsSource::Dev, "AI development games", dev)
    } else {
        let all: Vec<GameRecord> = patch_records.iter().filter(|record| !is_tutorial(record)).cloned().collect();
        (PublicStatsSource::Provisional, "AI games + live games (provisional)", all)
    };

    let report = card_stats(
        &records_to_count,
        &CardStatsFilter {
            source: match source {
                PublicStatsSource::Live => SourceFilter::Live,
                PublicStatsSource::Dev => SourceFilter::Dev,
                PublicStatsSource::Provisional => SourceFilter::All,
            },
            mode: None,
            patch: Some(requested_patch.clone()),
            pilot: PilotFilter::Unified,
        },
    );

    let stats_map: IndexMap<&str, _> = report.cards.iter().map(|stat| (stat.card.as_str(), stat)).collect();
    let total_decks = report.decks as i64;

    let set_param = req.query.get("set").map(|value| value.trim().to_lowercase()).filter(|value| !value.is_empty());
    let rarity_param = req.query.get("rarity").map(|value| value.trim().to_lowercase()).filter(|value| !value.is_empty());
    let cost_param = req.query.get("cost").cloned();
    let card_param = query_trimmed(&req, "card");

    let mut cards: Vec<PublicCardStat> = Vec::new();
    for (id, def) in &app.catalog.defs {
        if def.token {
            continue; // Only non-token deckable cards
        }
        if card_param.as_ref().is_some_and(|card| id != card) {
            continue;
        }
        if let Some(set) = &set_param {
            if def.set.as_str().to_lowercase() != *set && !id.to_lowercase().starts_with(set.as_str()) {
                continue;
            }
        }
        if let Some(rarity) = &rarity_param {
            if def.rarity.as_str().to_lowercase() != *rarity {
                continue;
            }
        }
        let cost = numeric_cost(def);
        if let Some(cost_text) = cost_param.as_deref().filter(|text| !text.is_empty()) {
            if let Some(parsed_cost) = js_number(&cost_text.replacen('+', "", 1)) {
                if parsed_cost >= CARD_STATS_CURVE_TOP as f64 {
                    if (cost as f64) < CARD_STATS_CURVE_TOP as f64 {
                        continue;
                    }
                } else if cost as f64 != parsed_cost {
                    continue;
                }
            }
        }

        let stat = stats_map.get(id.as_str()).copied();
        let (in_deck_games, _) = stat.map(|stat| tally_of(&stat.in_deck)).unwrap_or((0, 0));
        let rate = stat.and_then(|stat| win_rate(&stat.in_deck));
        let (played_games, played_wins) = stat.map(|stat| tally_of(&stat.played)).unwrap_or((0, 0));
        let (unplayed_games, unplayed_wins) = stat.map(|stat| tally_of(&stat.drawn_not_played)).unwrap_or((0, 0));
        let drawn_games = played_games + unplayed_games;
        let drawn_wins = played_wins + unplayed_wins;
        let drawn_rate = if drawn_games > 0 { Some(drawn_wins as f64 / drawn_games as f64) } else { None };
        let played_rate = stat.and_then(|stat| win_rate(&stat.played));
        let play_rate = if total_decks > 0 { in_deck_games as f64 / total_decks as f64 } else { 0.0 };
        let has_enough_games = in_deck_games >= CARD_STATS_MIN_SAMPLE as i64;

        cards.push(PublicCardStat {
            id: id.clone(),
            name: def.name.clone(),
            cost,
            rarity: def.rarity.as_str().to_string(),
            set: def.set.as_str().to_string(),
            games: in_deck_games,
            win_rate: rate,
            drawn_games,
            drawn_win_rate: drawn_rate,
            played_games,
            played_win_rate: played_rate,
            play_rate,
            has_enough_games,
        });
    }

    // Sort cards by win rate desc, then games desc, then name
    cards.sort_by(|a, b| {
        if a.has_enough_games && !b.has_enough_games {
            return Ordering::Less;
        }
        if !a.has_enough_games && b.has_enough_games {
            return Ordering::Greater;
        }
        if let (Some(a_rate), Some(b_rate)) = (a.win_rate, b.win_rate) {
            if a_rate != b_rate {
                return b_rate.partial_cmp(&a_rate).unwrap_or(Ordering::Equal);
            }
        }
        b.games.cmp(&a.games).then_with(|| locale_compare(&a.name, &b.name))
    });

    // Best and worst card above sample threshold
    let qualifying: Vec<&PublicCardStat> =
        cards.iter().filter(|card| card.has_enough_games && card.win_rate.is_some()).collect();
    let extreme = |card: &PublicCardStat| PublicStatsExtreme {
        id: card.id.clone(),
        name: card.name.clone(),
        win_rate: card.win_rate.unwrap_or(0.0),
        games: card.games,
    };
    let best_card = qualifying.first().map(|card| extreme(*card));
    let worst_card = qualifying.last().map(|card| extreme(*card));

    let response = PublicStatsCardsResponse {
        patch: requested_patch.clone(),
        previous_patch,
        gate: PublicStatsGate {
            cleared,
            live_games: live_games_count,
            min_live_games: PUBLIC_STATS_MIN_LIVE_GAMES as i64,
        },
        source,
        source_label: source_label.to_string(),
        min_sample: CARD_STATS_MIN_SAMPLE as i64,
        total_games: report.games as i64,
        cards,
        summary: PublicStatsSummary {
            total_games: report.games as i64,
            live_games: live_games_count,
            active_patch: requested_patch,
            source,
            best_card,
            worst_card,
        },
    };

    Ok(cached_ok(to_json(&response)?, CARD_STATS_CACHE_TTL_SECONDS as i64))
}

/// A turn's or a co-played card's games and wins.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Count {
    games: i64,
    wins: i64,
}

/// GET /api/stats/cards/:id
/// Card drill-down:
/// - Win rate by cleared patch over time.
/// - Win rate by turn played.
/// - Co-played synergy cards.
pub async fn get_card(app: &Arc<App>, req: Req) -> ApiResult {
    let card_id = req.params.get("id").cloned().unwrap_or_default();
    let def = if card_id.is_empty() { None } else { app.catalog.defs.get(&card_id) };
    let Some(def) = def else {
        return Err(ApiError::new(ApiErrorCode::NotFound, format!("Card {card_id} not found")));
    };

    let versions = load_patch_versions();
    let mut patch_history: Vec<PatchRate> = Vec::new();

    for version in &versions {
        let records: Vec<GameRecord> =
            records_for(app, SourceFilter::Live, version).await?.into_iter().filter(|record| !is_tutorial(record)).collect();
        if records.len() as i64 >= PUBLIC_STATS_MIN_LIVE_GAMES as i64 {
            let report = card_stats(
                &records,
                &CardStatsFilter {
                    source: SourceFilter::Live,
                    mode: None,
                    patch: Some(version.clone()),
                    pilot: PilotFilter::Unified,
                },
            );
            let stat = report.cards.iter().find(|stat| stat.card == card_id);
            patch_history.push(PatchRate {
                patch: version.clone(),
                games: stat.map(|stat| tally_of(&stat.in_deck).0).unwrap_or(0),
                win_rate: stat.and_then(|stat| win_rate(&stat.in_deck)),
            });
        }
    }

    // Use active patch or all records for turn and co-play metrics
    let current_patch = app.catalog.version.clone();
    let all_records = records_for(app, SourceFilter::All, &current_patch).await?;
    let live_records: Vec<&GameRecord> =
        all_records.iter().filter(|record| record.source == GameSource::Live && !is_tutorial(record)).collect();
    let records_to_count: Vec<&GameRecord> = if live_records.len() as i64 >= PUBLIC_STATS_MIN_LIVE_GAMES as i64 {
        live_records
    } else {
        all_records.iter().filter(|record| !is_tutorial(record)).collect()
    };

    // By turn played
    let mut turn_tallies: IndexMap<i32, Count> = IndexMap::new();
    // Co-played cards
    let mut co_played_tallies: IndexMap<String, Count> = IndexMap::new();

    for record in &records_to_count {
        for seat in PLAYER_IDS {
            let summary = &record.game.seats[seat];
            if !summary.deck.iter().any(|card| *card == card_id) {
                continue;
            }

            let won = record.game.winner == Winner::from(seat);

            // Check turns on which this card was played (from recorded playedTurns)
            let mut turns_played_in_game: IndexSet<i32> = IndexSet::new();
            if let Some(played_turns) = summary.played_turns.as_ref().filter(|turns| turns.len() == summary.played.len()) {
                for (index, played) in summary.played.iter().enumerate() {
                    if *played == card_id {
                        if let Some(turn) = played_turns.get(index).map(|turn| *turn as i32) {
                            if turn > 0 {
                                turns_played_in_game.insert(turn);
                            }
                        }
                    }
                }
            }
            for turn in turns_played_in_game {
                let tally = turn_tallies.entry(turn).or_default();
                tally.games += 1;
                if won {
                    tally.wins += 1;
                }
            }

            // Co-played cards in the same deck
            let distinct: IndexSet<&String> = summary.deck.iter().collect();
            for other_id in distinct {
                if *other_id == card_id {
                    continue;
                }
                let tally = co_played_tallies.entry(other_id.clone()).or_default();
                tally.games += 1;
                if won {
                    tally.wins += 1;
                }
            }
        }
    }

    let mut by_turn: Vec<TurnRate> = turn_tallies
        .iter()
        .map(|(turn, tally)| TurnRate {
            turn: *turn,
            games: tally.games,
            win_rate: if tally.games > 0 { Some(tally.wins as f64 / tally.games as f64) } else { None },
        })
        .collect();
    by_turn.sort_by(|a, b| a.turn.cmp(&b.turn));

    let mut co_played: Vec<CoPlayedRate> = co_played_tallies
        .iter()
        .map(|(id, tally)| CoPlayedRate {
            id: id.clone(),
            name: app.catalog.defs.get(id).map(|def| def.name.clone()).unwrap_or_else(|| id.clone()),
            games: tally.games,
            win_rate: if tally.games > 0 { Some(tally.wins as f64 / tally.games as f64) } else { None },
        })
        .collect();
    co_played.sort_by(|a, b| b.games.cmp(&a.games));
    co_played.truncate(10);

    let body = CardDrillDownResponse {
        card: DrillDownCard {
            id: def.id.clone(),
            name: def.name.clone(),
            cost: numeric_cost(def),
            rarity: def.rarity.as_str().to_string(),
        },
        patches: patch_history,
        by_turn,
        co_played,
    };
    Ok(cached_ok(to_json(&body)?, CARD_STATS_CACHE_TTL_SECONDS as i64))
}

/// GET /api/stats/player
/// Signed-in player reads their own tracked statistics and privacy setting.
pub async fn get_player(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let row = tx.player_stats_get(&profile.id).await?;
    tx.commit().await?;
    let body = match row {
        Some(row) => json!({ "stats": to_json(&row.stats)?, "isPrivate": row.is_private, "updatedAt": row.updated_at }),
        None => json!({ "stats": {}, "isPrivate": false, "updatedAt": null }),
    };
    ok_of(&body)
}

/// GET /api/stats/player's body length as TS measured it: `JSON.stringify(body).length`, in UTF-16
/// code units.
fn stringified_length(body: &Value) -> usize {
    body.to_string().encode_utf16().count()
}

/// PUT /api/stats/player
/// Signed-in player updates their tracked statistics and privacy setting.
pub async fn put_player(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    if stringified_length(&req.body) > PLAYER_STATS_BYTES_MAX as usize {
        return Err(bad_request(format!(
            "player stats exceeds maximum payload size of {PLAYER_STATS_BYTES_MAX} bytes"
        )));
    }

    let mut tx = app.db.begin(Some(&profile.id)).await?;
    let existing = tx.player_stats_get(&profile.id).await?;
    let is_private = match req.body.get("isPrivate") {
        Some(Value::Bool(value)) => *value,
        _ => existing.as_ref().map(|row| row.is_private).unwrap_or(false),
    };

    let stats: IndexMap<String, Value> = match req.body.get("stats") {
        Some(Value::Object(map)) => map.iter().map(|(key, value)| (key.clone(), value.clone())).collect(),
        _ => existing.as_ref().map(|row| row.stats.clone()).unwrap_or_default(),
    };

    let now = now_ms();
    tx.player_stats_put(&profile.id, &stats, is_private, now).await?;
    tx.commit().await?;
    let stats_json: Map<String, Value> = stats.into_iter().collect();
    ok_of(&json!({ "stats": Value::Object(stats_json), "isPrivate": is_private, "updatedAt": now }))
}

/// GET /api/stats/players
/// Public player summaries (games, win rate, favourite cards, fun stats).
/// Excludes private players. Keeps Elo and rankings separate.
pub async fn get_players(app: &Arc<App>, req: Req) -> ApiResult {
    let search = query_trimmed(&req, "search");
    let page: u64 = match req.query.get("page") {
        Some(text) if !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit()) => {
            text.parse::<u64>().unwrap_or(u64::MAX).max(1)
        }
        _ => 1,
    };
    let limit = PLAYER_STATS_PAGE_LIMIT as u64;
    let offset = (page - 1).saturating_mul(limit);

    let mut tx = app.db.begin(None).await?;
    let players = tx
        .player_stats_list_public(&PlayerStatsListOptions {
            search,
            limit: limit as i64,
            offset: i64::try_from(offset).unwrap_or(i64::MAX),
        })
        .await?;
    tx.commit().await?;
    Ok(cached_ok(
        json!({ "players": to_json(&players)?, "page": page, "limit": limit }),
        PLAYER_STATS_CACHE_TTL_SECONDS as i64,
    ))
}
