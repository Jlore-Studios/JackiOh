//! The ranked ladder through the server (SPEC §9.12, R603–R612): opening seasons, rating ranked
//! games of players and bots, Jlorious, and the three reads the client has.
//!
//! The pure rules have their own suites (`tests/api/{glicko2,ladder,season}.rs`); these drive
//! `crates/server/src/api/ranked.rs` against the fake store, so what is proved here is the reading
//! and writing around them.
//!
//! Ported from `apps/server/test/api/ranked.test.ts` (part 18). TS's test deps ran as patch
//! `v0.1.1`, so its season was the literal `"v0.1"`; the Rust test app runs as the build's own
//! patch (`jackioh_cards::catalog_version()`, SURFACE §11.3), so the season here is
//! `season_id_of` of that, and the "next minor version" is computed from it.

use std::sync::Arc;

use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use jackioh_server::actor::contracts::RecordResultInput;
use jackioh_server::api::crypto::player_tag;
use jackioh_server::api::ranked::{
    RankedGameInput, SeasonDeps, leaderboard, open_season, open_season_in_tx, own_rank, rate_ranked_game,
};
use jackioh_server::api::results::record_result;
use jackioh_server::app::App;
use jackioh_server::config::{JLORIOUS_SIZE, RANK_PLACEMENT_GAMES, RATING_DEVIATION_START, SEASON_RESET_STRENGTH};
use jackioh_server::db::fake::FakeData;
use jackioh_server::db::store::{BotRating, Db, MatchRow, SeriesRow};
use jackioh_server::ranked::glicko2::START_GLICKO;
use jackioh_server::ranked::ladder::{SeasonRank, fresh_rank, tier_bottom};
use jackioh_server::ranked::season::season_id_of;

use crate::support::deps::{add_user, call, test_app};

const A: &str = "profile-a";
const B: &str = "profile-b";

// ---------------------------------------------------------------------------
// Harness (a private copy per file, SURFACE rule 5)
// ---------------------------------------------------------------------------

/// A port value built from TS's own object literal, so the test depends on the JSON shape only.
fn from<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn json_of<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

/// `{ ...base, ...extra }` on JSON objects.
fn merged(mut base: Value, extra: Value) -> Value {
    if let (Some(into), Value::Object(extra)) = (base.as_object_mut(), extra) {
        for (key, value) in extra {
            into.insert(key, value);
        }
    }
    base
}

/// Jest's `toMatchObject`: every key `expected` names is in `actual` with a matching value, arrays
/// element by element.
fn assert_matches(actual: &Value, expected: &Value, at: &str) {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => {
            for (key, value) in expected {
                let found = actual.get(key).unwrap_or_else(|| panic!("{at}.{key} is missing"));
                assert_matches(found, value, &format!("{at}.{key}"));
            }
        }
        (Value::Array(actual), Value::Array(expected)) => {
            assert_eq!(actual.len(), expected.len(), "{at}: length");
            for (index, (found, value)) in actual.iter().zip(expected).enumerate() {
                assert_matches(found, value, &format!("{at}[{index}]"));
            }
        }
        (Value::Number(found), Value::Number(value)) => {
            assert_eq!(found.as_f64(), value.as_f64(), "{at}");
        }
        _ => assert_eq!(actual, expected, "{at}"),
    }
}

/// Jest's `toBeCloseTo(expected, 9)`.
fn assert_close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 0.5e-9, "{actual} is not close to {expected}");
}

/// The fake store behind the test app (TS `deps.store`).
async fn fake(app: &App) -> tokio::sync::MutexGuard<'_, FakeData> {
    match &app.db {
        Db::Fake(data) => data.lock().await,
        Db::Pg(_) => panic!("the API tests run on the fake store"),
    }
}

/// One store call in a transaction of its own (TS called the store's methods bare).
macro_rules! store {
    ($app:expr, $method:ident($($arg:expr),* $(,)?)) => {{
        let mut t = $app.db.begin(None).await.expect("begin");
        let out = t.$method($($arg),*).await.unwrap_or_else(|error| panic!("{}: {error:?}", stringify!($method)));
        t.commit().await.expect("commit");
        out
    }};
}

/// The patch the test app runs as, and so its season (R609).
fn patch() -> String {
    jackioh_cards::catalog_version().to_string()
}

fn season() -> String {
    season_id_of(&patch())
}

/// The first patch of the next minor version: `v0.2.11` → `v0.3.0` (TS: `v0.1.1` → `v0.2.0`).
fn next_minor(patch: &str) -> String {
    let digits = patch.trim_start_matches('v');
    let mut parts = digits.split(['.', '-']);
    let major: u32 = parts.next().and_then(|part| part.parse().ok()).expect("a major version");
    let minor: u32 = parts.next().and_then(|part| part.parse().ok()).expect("a minor version");
    format!("v{major}.{}.0", minor + 1)
}

/// TS `deps.store.seedProfile(...)`.
async fn seed_profile(app: &App, row: Value) {
    fake(app).await.seed_profile(row);
}

async fn profile(app: &App, id: &str) -> Value {
    json_of(&store!(app, profiles_get_by_id(id)))
}

fn rating_of(profile: &Value) -> f64 {
    profile["rating"].as_f64().expect("a rating")
}

/// A ranked match game between two sides, `winner_side` winning (`None`: a draw).
fn game(id: &str, sides: [Value; 2], winner_side: Option<usize>, at: i64) -> Value {
    json!({
        "id": id,
        "kind": "match",
        "catalogVersion": "test-1",
        "sides": sides,
        "winnerSide": winner_side,
        "reason": if winner_side.is_none() { "draw-accepted" } else { "hero-death" },
        "at": at,
    })
}

fn player(profile_id: &str) -> Value {
    json!({ "kind": "player", "profileId": profile_id })
}

fn bot(bot_id: &str) -> Value {
    json!({ "kind": "bot", "botId": bot_id })
}

/// TS `deps.store.tx((t) => rateRankedGame(t, deps, input))`.
async fn rate(app: &App, input: Value) -> Value {
    let input: RankedGameInput = from(input);
    let mut t = app.db.begin(None).await.expect("begin");
    let row = rate_ranked_game(&mut t, app, &input).await.expect("rateRankedGame");
    t.commit().await.expect("commit");
    json_of(&row)
}

async fn own(app: &App, profile_id: &str) -> Value {
    json_of(&own_rank(app, profile_id).await.expect("ownRank"))
}

async fn board(app: &App, viewer_id: &str) -> Value {
    json_of(&leaderboard(app, viewer_id).await.expect("leaderboard"))
}

async fn put_rank(app: &App, rank: Value) {
    let rank: SeasonRank = from(rank);
    store!(app, ranked_put_rank(&rank));
}

async fn ladder_of(app: &App, profile_id: &str) -> i64 {
    let rank = json_of(&store!(app, ranked_rank(&season(), profile_id)));
    rank["ladder"].as_i64().unwrap_or(-1)
}

/// A placed player at `ladder` with `rating`, written straight into the season.
async fn seed_placed(app: &App, profile_id: &str, rating: f64, ladder: i32, extra: Value) {
    seed_profile(app, json!({ "id": profile_id, "rating": rating })).await;
    let rank = merged(
        json_of(&fresh_rank(&season(), profile_id, 0)),
        json!({
            "games": RANK_PLACEMENT_GAMES,
            "wins": RANK_PLACEMENT_GAMES,
            "ladder": ladder,
            "floor": ladder / tier_bottom(1),
            "peakLadder": ladder,
        }),
    );
    put_rank(app, merged(rank, extra)).await;
}

fn raisin(played: i64) -> Value {
    json!({ "tier": "raisin", "placementsPlayed": played, "placementGames": RANK_PLACEMENT_GAMES })
}

/// A live match row with the clocks a row carries before its actor has synced (TS
/// `initialClocks(0, deps.config)`; the ceiling plays no part here).
fn live_match(id: &str, seed: &str, ranked: bool) -> MatchRow {
    from(json!({
        "id": id,
        "seed": seed,
        "players": [A, B],
        "decks": [[], []],
        "catalogVersion": "test-1",
        "ranked": ranked,
        "status": "live",
        "createdAt": 0,
        "finishedAt": null,
        "clocks": { "turnDeadline": null, "promptDeadline": null, "graceDeadline": { "p1": null, "p2": null }, "ceilingAt": 0 },
    }))
}

// ---------------------------------------------------------------------------
// R609 seasons on the server
// ---------------------------------------------------------------------------

mod r609_seasons_on_the_server {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r609_the_build_s_season_opens_once_and_the_first_season_of_all_resets_nothing() {
        let app = test_app().await;
        seed_profile(&app, json!({ "id": A, "rating": 1300, "ratingDeviation": 80 })).await;
        let first = open_season(&app).await.expect("openSeason");
        assert_matches(
            &json_of(&first),
            &json!({ "season": { "id": season(), "patchVersion": patch() }, "opened": true, "reset": null }),
            "first",
        );
        let again = open_season(&app).await.expect("openSeason");
        assert_matches(&json_of(&again), &json!({ "opened": false, "reset": null }), "again");
        assert_eq!(fake(&app).await.tables.seasons.len(), 1);
        assert_eq!(rating_of(&profile(&app, A).await), 1300.0);
    }

    #[tokio::test(start_paused = true)]
    async fn r609_a_new_minor_version_opens_the_next_season_with_a_soft_reset_of_every_rated_player_and_nobody_else() {
        let app = test_app().await;
        seed_profile(&app, json!({ "id": A, "rating": 1000 })).await;
        seed_profile(&app, json!({ "id": B, "rating": 1000 })).await;
        seed_profile(&app, json!({ "id": "never-played", "rating": 1000 })).await;
        for n in 0..4 {
            rate(&app, game(&format!("m-{n}"), [player(A), player(B)], Some(0), 1)).await;
        }
        rate(&app, game("bot-game", [player(A), bot("ai-hard")], Some(1), 1)).await;
        let a = profile(&app, A).await;
        let b = profile(&app, B).await;
        let bot_before = json_of(&store!(app, ranked_bot("ai-hard")));
        assert!(!a.is_null() && !b.is_null() && !bot_before.is_null(), "premise: all three were rated");
        let mean = (rating_of(&a) + rating_of(&b)) / 2.0;

        let next_patch = next_minor(&patch());
        let deps = SeasonDeps { patch_version: next_patch.clone() };
        let mut t = app.db.begin(None).await.expect("begin");
        let next = open_season_in_tx(&mut t, &deps).await.expect("openSeason");
        t.commit().await.expect("commit");
        assert_matches(
            &json_of(&next),
            &json!({
                "season": { "id": season_id_of(&next_patch), "patchVersion": next_patch },
                "opened": true,
                "reset": { "players": 2 },
            }),
            "next",
        );
        let after = [profile(&app, A).await, profile(&app, B).await];
        assert_close(rating_of(&after[0]), mean + (rating_of(&a) - mean) * (1.0 - SEASON_RESET_STRENGTH));
        assert_close(rating_of(&after[1]), mean + (rating_of(&b) - mean) * (1.0 - SEASON_RESET_STRENGTH));
        assert!(
            after[0]["ratingDeviation"].as_f64() > a["ratingDeviation"].as_f64(),
            "the reset widens the deviation"
        );
        // A player who never played a rated game, and the bot, are left as they were.
        assert_matches(
            &profile(&app, "never-played").await,
            &json!({ "rating": 1000, "ratingDeviation": RATING_DEVIATION_START }),
            "never-played",
        );
        assert_eq!(json_of(&store!(app, ranked_bot("ai-hard"))), bot_before);
        // Everyone starts the new season back in placements; the old season's rows are kept. TS read
        // this through `ownRank({ ...deps, patchVersion: "v0.2.0" }, A)`; the Rust test app runs as
        // one patch, so the same fact is read off the store: no row in the new season, which
        // `own_rank` answers as a Raisin with no placements played (R605).
        assert!(json_of(&store!(app, ranked_rank(&season_id_of(&next_patch), A))).is_null());
        let seasons: Vec<Value> = json_of(&store!(app, ranked_ranks_of(A)))
            .as_array()
            .expect("a list")
            .iter()
            .map(|rank| rank["seasonId"].clone())
            .collect();
        assert_eq!(seasons, vec![json!(season())]);
    }

    #[tokio::test(start_paused = true)]
    async fn r609_the_first_rated_game_of_a_build_opens_its_season_itself() {
        let app = test_app().await;
        seed_profile(&app, json!({ "id": A })).await;
        seed_profile(&app, json!({ "id": B })).await;
        let row = rate(&app, game("m-1", [player(A), player(B)], None, 1)).await;
        assert_eq!(row["seasonId"], json!(season()));
        let ids: Vec<String> = fake(&app).await.tables.seasons.iter().map(|season| season.id.clone()).collect();
        assert_eq!(ids, vec![season()]);
    }
}

// ---------------------------------------------------------------------------
// R605 placements through rated games
// ---------------------------------------------------------------------------

mod r605_placements_through_rated_games {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r605_rank_placement_games_rated_games_place_a_raisin_where_the_rating_calls_for() {
        let app = test_app().await;
        // A placed field to be read against, rated 700 to 1300.
        for n in 0..7 {
            seed_placed(&app, &format!("field-{n}"), 700.0 + 100.0 * f64::from(n), tier_bottom(1), json!({})).await;
        }
        seed_profile(&app, json!({ "id": A })).await;
        seed_profile(&app, json!({ "id": B })).await;
        let placements = RANK_PLACEMENT_GAMES as i64;
        for n in 1..=placements {
            rate(&app, game(&format!("m-{n}"), [player(A), player(B)], Some(0), n)).await;
            let mine = own(&app, A).await;
            if n < placements {
                assert_eq!(mine["rank"], raisin(n), "after {n} placements");
            }
        }
        let winner = own(&app, A).await;
        let loser = own(&app, B).await;
        // Both placed by the same game, each read against the field: the winner well above the loser.
        assert_ne!(winner["rank"]["tier"], json!("raisin"));
        assert_ne!(loser["rank"]["tier"], json!("raisin"));
        assert_eq!(
            winner["record"],
            json!({ "games": RANK_PLACEMENT_GAMES, "wins": RANK_PLACEMENT_GAMES, "losses": 0, "draws": 0 })
        );
        assert_eq!(winner["streak"], json!(RANK_PLACEMENT_GAMES));
        assert!(ladder_of(&app, A).await > ladder_of(&app, B).await);
    }

    #[tokio::test(start_paused = true)]
    async fn r603_rating_the_same_game_twice_changes_nothing() {
        let app = test_app().await;
        seed_profile(&app, json!({ "id": A })).await;
        seed_profile(&app, json!({ "id": B })).await;
        let first = rate(&app, game("m-1", [player(A), player(B)], Some(0), 1)).await;
        let before = profile(&app, A).await;
        assert_eq!(rate(&app, game("m-1", [player(A), player(B)], Some(1), 1)).await, first);
        assert_eq!(profile(&app, A).await, before);
        assert_eq!(fake(&app).await.tables.rated_games.len(), 1);
    }
}

// ---------------------------------------------------------------------------
// R610 bots
// ---------------------------------------------------------------------------

mod r610_bots {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r610_a_bot_is_rated_like_a_player_from_its_own_rating_and_has_no_rank_no_season_row_and_no_place_on_the_leaderboard() {
        let app = test_app().await;
        seed_profile(&app, json!({ "id": A })).await;
        let first = rate(&app, game("b-1", [player(A), bot("ai-easy")], Some(0), 1)).await;
        assert_matches(
            &first["sides"][1],
            &json!({
                "profileId": null,
                "botId": "ai-easy",
                "pilot": "ai",
                "before": json_of(&START_GLICKO),
                "rankBefore": null,
                "rankAfter": null,
            }),
            "sides[1]",
        );
        let easy = json_of(&store!(app, ranked_bot("ai-easy")));
        assert_eq!(easy["games"], json!(1));
        assert!(easy["glicko"]["rating"].as_f64().expect("a rating") < START_GLICKO.rating);
        assert_eq!(easy["glicko"], first["sides"][1]["after"]);

        // The bot's next game starts from the rating its last one left it.
        let second = rate(&app, game("b-2", [bot("ai-easy"), player(A)], Some(0), 1)).await;
        assert_eq!(second["sides"][0]["before"], first["sides"][1]["after"]);
        assert_eq!(json_of(&store!(app, ranked_bot("ai-easy")))["games"], json!(2));

        let ranked: Vec<String> =
            fake(&app).await.tables.season_ranks.iter().map(|rank| rank.profile_id.clone()).collect();
        assert_eq!(ranked, vec![A.to_string()]);
        let listed = board(&app, A).await;
        assert!(!listed.to_string().contains("ai-easy"));
    }

    #[tokio::test(start_paused = true)]
    async fn r610_a_bot_s_rating_is_not_among_the_players_a_rank_s_percentile_is_read_from() {
        let app = test_app().await;
        // One placed player, rated far above a fresh one: the fresh player's target reads only them.
        seed_placed(&app, "placed", 2000.0, tier_bottom(2), json!({})).await;
        seed_profile(&app, json!({ "id": A })).await;
        let hard: BotRating = from(json!({
            "botId": "ai-hard",
            "glicko": merged(json_of(&START_GLICKO), json!({ "rating": 400 })),
            "games": 50,
            "updatedAt": 0,
        }));
        store!(app, ranked_put_bot(&hard));
        for n in 1..=(RANK_PLACEMENT_GAMES as i64) {
            rate(&app, game(&format!("g-{n}"), [player(A), bot("ai-hard")], Some(0), n)).await;
        }
        // Rated below the one placed player, alone with them: the 25th percentile, in Normal Grape.
        assert_matches(&own(&app, A).await["rank"], &json!({ "tier": "normal" }), "rank");
    }
}

// ---------------------------------------------------------------------------
// R608 Jlorious through the server
// ---------------------------------------------------------------------------

mod r608_jlorious_through_the_server {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r608_ranks_mythic_grape_players_by_rating_on_the_leaderboard_and_records_the_position_a_game_gives_as_the_season_s_peak()
     {
        let app = test_app().await;
        let mythic = tier_bottom(4);
        seed_placed(&app, "m-high", 1900.0, mythic, json!({})).await;
        seed_placed(&app, "m-low", 1500.0, mythic + 2, json!({})).await;
        seed_placed(&app, "golden", 2500.0, tier_bottom(3) + 8, json!({})).await;
        seed_placed(&app, A, 1400.0, mythic + 1, json!({})).await;
        seed_placed(&app, B, 1300.0, tier_bottom(1), json!({})).await;
        // A beats B and climbs past m-low on rating: Jlorious #2.
        rate(&app, game("j-1", [player(A), player(B)], Some(0), 1)).await;
        let listed = board(&app, A).await;
        let jlorious: Vec<Value> = listed["jlorious"]
            .as_array()
            .expect("a list")
            .iter()
            .map(|row| json!([row["position"], row["tag"], row["you"]]))
            .collect();
        assert_eq!(
            jlorious,
            vec![
                json!([1, player_tag("m-high"), false]),
                json!([2, player_tag(A), true]),
                json!([3, player_tag("m-low"), false]),
            ]
        );
        let peak = |rank: Value| rank["peakJlorious"].clone();
        assert_eq!(peak(json_of(&store!(app, ranked_rank(&season(), A)))), json!(2));
        assert_eq!(own(&app, A).await["rank"], json!({ "tier": "jlorious", "position": 2 }));
        assert_eq!(own(&app, A).await["badges"], json!([{ "seasonId": season(), "tier": "jlorious", "position": 2 }]));
        // m-low fell to #3 and keeps the #2 it never held: no peak is invented for it.
        assert_eq!(peak(json_of(&store!(app, ranked_rank(&season(), "m-low")))), json!(3));
        // Golden is rated highest of all and is not Jlorious: Jlorious is drawn from Mythic Grape.
        let golden: Vec<Value> = listed["tiers"]
            .as_array()
            .expect("tiers")
            .iter()
            .find(|tier| tier["tier"] == json!("golden"))
            .expect("a golden tier")["players"]
            .as_array()
            .expect("players")
            .iter()
            .map(|row| row["tag"].clone())
            .collect();
        assert_eq!(golden, vec![json!(player_tag("golden"))]);
        assert_eq!(JLORIOUS_SIZE, 100);
    }
}

// ---------------------------------------------------------------------------
// R612 what the client reads
// ---------------------------------------------------------------------------

mod r612_what_the_client_reads {
    use super::*;

    struct Routed {
        app: Arc<App>,
        token_a: String,
        stranger: String,
    }

    struct Got {
        status: u16,
        body: Value,
        text: String,
    }

    async fn routed() -> Routed {
        let app = test_app().await;
        let token_a = add_user(&app, &format!("user-{A}"), "a@example.test", true);
        let _token_b = add_user(&app, &format!("user-{B}"), "b@example.test", true);
        let stranger = add_user(&app, "user-stranger", "s@example.test", true);
        seed_profile(&app, json!({ "id": A, "userId": format!("user-{A}"), "rating": 1234.5678 })).await;
        seed_profile(&app, json!({ "id": B, "userId": format!("user-{B}"), "rating": 987.654 })).await;
        seed_profile(&app, json!({ "id": "stranger", "userId": "user-stranger" })).await;
        Routed { app, token_a, stranger }
    }

    async fn get(app: &Arc<App>, path: &str, token: &str) -> Got {
        let (status, _headers, body) = call(app, "GET", path, Some(token), Value::Null).await;
        let text = body.to_string();
        Got { status, body, text }
    }

    fn mentions_none(text: &str, words: &[&str]) {
        for word in words {
            assert!(!text.contains(word), "{text} mentions {word}");
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r612_get_api_ranked_answers_the_caller_s_tag_rank_streak_record_and_badges_and_never_a_rating() {
        let Routed { app, token_a, .. } = routed().await;
        let got = get(&app, "/api/ranked", &token_a).await;
        assert_eq!(got.status, 200);
        assert_eq!(
            got.body,
            json!({
                "season": season(),
                "tag": player_tag(A),
                "rank": raisin(0),
                "streak": 0,
                "record": { "games": 0, "wins": 0, "losses": 0, "draws": 0 },
                "badges": [],
            })
        );
        mentions_none(&got.text, &["1234", "rating", "deviation", "volatility"]);
    }

    #[tokio::test(start_paused = true)]
    async fn r612_get_api_leaderboard_lists_tags_and_ranks_marks_the_caller_and_counts_the_raisins() {
        let Routed { app, token_a, .. } = routed().await;
        put_rank(&app, merged(json_of(&fresh_rank(&season(), B, 0)), json!({ "games": 2, "wins": 2 }))).await;
        put_rank(
            &app,
            merged(
                json_of(&fresh_rank(&season(), A, 0)),
                json!({
                    "games": RANK_PLACEMENT_GAMES,
                    "ladder": tier_bottom(1) + 4,
                    "floor": 1,
                    "peakLadder": tier_bottom(1) + 4,
                }),
            ),
        )
        .await;
        let got = get(&app, "/api/leaderboard", &token_a).await;
        assert_eq!(got.status, 200);
        assert_eq!(got.body["jlorious"], json!([]));
        assert_eq!(got.body["raisins"], json!(1));
        let tiers: Vec<Value> =
            got.body["tiers"].as_array().expect("tiers").iter().map(|tier| tier["tier"].clone()).collect();
        assert_eq!(tiers, vec![json!("mythic"), json!("golden"), json!("large"), json!("normal"), json!("rotten")]);
        let normal = got.body["tiers"]
            .as_array()
            .expect("tiers")
            .iter()
            .find(|tier| tier["tier"] == json!("normal"))
            .cloned()
            .expect("a normal tier");
        assert_eq!(
            normal,
            json!({
                "tier": "normal",
                "count": 1,
                "players": [{ "tag": player_tag(A), "division": 2, "pips": 1, "you": true }],
            })
        );
        assert_matches(&got.body["you"], &json!({ "tier": "normal", "division": 2, "pips": 1 }), "you");
        mentions_none(&got.text, &["1234", "987", "profile-", "rating"]);
    }

    #[tokio::test(start_paused = true)]
    async fn r612_get_api_matches_id_ranks_shows_both_seats_to_a_player_of_the_match_and_404s_for_anyone_else() {
        let Routed { app, token_a, stranger } = routed().await;
        let row = live_match("match-1", "s", true);
        store!(app, matches_create(&row));
        let mine = get(&app, "/api/matches/match-1/ranks", &token_a).await;
        assert_eq!(mine.status, 200);
        assert_eq!(
            mine.body,
            json!({
                "ranked": true,
                "seats": {
                    "p1": { "tag": player_tag(A), "rank": raisin(0), "you": true },
                    "p2": { "tag": player_tag(B), "rank": raisin(0), "you": false },
                },
            })
        );
        mentions_none(&mine.text, &["rating", "profile-"]);
        assert_eq!(get(&app, "/api/matches/match-1/ranks", &stranger).await.status, 404);
        assert_eq!(get(&app, "/api/matches/no-such-match/ranks", &token_a).await.status, 404);
    }
}

// ---------------------------------------------------------------------------
// R604 a ranked series through the results writer
// ---------------------------------------------------------------------------

mod r604_a_ranked_series_through_the_results_writer {
    use super::*;

    fn deck(slot: usize) -> Value {
        json!({ "name": format!("d{slot}"), "cards": [] })
    }

    #[tokio::test(start_paused = true)]
    async fn r604_a_room_s_series_moves_no_rating_when_it_ends_and_a_queue_s_moves_it_once() {
        for ranked in [false, true] {
            let app = test_app().await;
            seed_profile(&app, json!({ "id": A, "inMatchId": "game-1" })).await;
            seed_profile(&app, json!({ "id": B, "inMatchId": "game-1" })).await;
            let trio = json!({ "name": "t", "decks": [deck(0), deck(1), deck(2)] });
            let series: SeriesRow = from(json!({
                "id": "series-1",
                "sides": [
                    { "profileId": A, "trio": trio, "wins": 2, "pick": null },
                    { "profileId": B, "trio": trio, "wins": 0, "pick": null },
                ],
                "catalogVersion": "test-1",
                "ranked": ranked,
                "seedBase": "s",
                "status": "playing",
                "games": [
                    { "gameNo": 1, "matchId": "g-a", "slots": [0, 0], "first": "p1", "winner": "p1", "reason": "concede" },
                    { "gameNo": 2, "matchId": "g-b", "slots": [1, 0], "first": "p2", "winner": "p1", "reason": "concede" },
                    { "gameNo": 3, "matchId": "game-1", "slots": [2, 0], "first": "p1", "winner": null, "reason": null },
                ],
                "nextMatchId": "game-1",
                "pickDeadline": null,
                "winner": null,
                "endReason": null,
                "ratingBefore": null,
                "ratingAfter": null,
                "createdAt": 0,
                "updatedAt": 0,
                "endedAt": null,
                "version": 1,
            }));
            store!(app, series_create(&series));
            let game_1 = live_match("game-1", "s:3", ranked);
            store!(app, matches_create(&game_1));
            let input: RecordResultInput = from(json!({
                "matchId": "game-1",
                "seats": [
                    { "profileId": A, "player": "p1", "deck": [] },
                    { "profileId": B, "player": "p2", "deck": [] },
                ],
                "outcome": { "winner": "p1", "reason": "hero-death" },
                "turns": 4,
                "at": 5,
            }));
            record_result(&app, input).await.expect("recordResult");

            let ended = json_of(&store!(app, series_get("series-1")));
            assert_eq!(ended["status"], json!("over"), "ranked: {ranked}");
            let rating = profile(&app, A).await["rating"].as_f64().unwrap_or(0.0);
            let rated: Vec<Value> = fake(&app)
                .await
                .tables
                .rated_games
                .iter()
                .map(|row| {
                    let row = json_of(row);
                    json!([row["id"], row["kind"], row["reason"], row["winnerSide"]])
                })
                .collect();
            if ranked {
                assert!(rating > 1000.0, "the series' winner gains rating");
                assert_eq!(rated, vec![json!(["series-1", "series", "decided", 0])]);
                assert_eq!(ended["ratingAfter"][0].as_f64(), Some(rating));
            } else {
                assert_eq!(rating, 1000.0);
                assert!(rated.is_empty());
                assert!(ended["ratingAfter"].is_null());
            }
        }
    }
}
