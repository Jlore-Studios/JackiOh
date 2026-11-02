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

use serde_json::{Value, json};

use jackioh_server::actor::contracts::RecordResultInput;
use jackioh_server::api::ranked::{
    RankedGameInput, SeasonDeps, ladder_tiers, leaderboard, match_ranks, open_season, open_season_in_tx,
    own_rank, rate_ranked_game,
};
use jackioh_server::api::results::record_result;
use jackioh_server::app::App;
use jackioh_server::config::{
    BOT_PROMOTION_DEVIATION_BOOST, JLORIOUS_SIZE, RANK_PLACEMENT_GAMES, RATING_DEVIATION_START,
    SEASON_RESET_STRENGTH,
};
use jackioh_server::db::store::{BotRating, MatchRow, SeriesRow, StoreError};
use jackioh_server::ranked::glicko2::START_GLICKO;
use jackioh_server::ranked::ladder::{RankTier, SeasonRank, fresh_rank, tier_bottom};
use jackioh_server::ranked::season::season_id_of;

use crate::support::deps::{add_user, call, test_app};
use crate::support::store::{fake_lock, from, q, to_json};

const A: &str = "profile-a";
const B: &str = "profile-b";

// ---------------------------------------------------------------------------
// Harness (a private copy per file, SURFACE rule 5)
// ---------------------------------------------------------------------------

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
    assert!(
        (actual - expected).abs() < 0.5e-9,
        "{actual} is not close to {expected}"
    );
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
    let major: u32 = parts
        .next()
        .and_then(|part| part.parse().ok())
        .expect("a major version");
    let minor: u32 = parts
        .next()
        .and_then(|part| part.parse().ok())
        .expect("a minor version");
    format!("v{major}.{}.0", minor + 1)
}

/// TS `deps.store.seedProfile(...)`.
async fn seed_profile(app: &App, row: Value) {
    fake_lock(app).await.seed_profile(row);
}

async fn profile(app: &App, id: &str) -> Value {
    to_json(&q!(app, profiles_get_by_id(id)))
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
    let row = rate_ranked_game(&mut t, app, &input)
        .await
        .expect("rateRankedGame");
    t.commit().await.expect("commit");
    to_json(&row)
}

async fn own(app: &App, profile_id: &str) -> Value {
    to_json(&own_rank(app, profile_id).await.expect("ownRank"))
}

async fn board(app: &App, viewer_id: &str) -> Value {
    to_json(&leaderboard(app, viewer_id).await.expect("leaderboard"))
}

async fn put_rank(app: &App, rank: Value) {
    let rank: SeasonRank = from(rank);
    q!(app, ranked_put_rank(&rank));
}

async fn ladder_of(app: &App, profile_id: &str) -> i64 {
    let rank = to_json(&q!(app, ranked_rank(&season(), profile_id)));
    rank["ladder"].as_i64().unwrap_or(-1)
}

/// The username `seed_placed` gives a profile: its id with `_` for `-` (`m-high` is `m_high`).
fn username_of(profile_id: &str) -> String {
    profile_id.replace('-', "_")
}

/// A placed player at `ladder` with `rating`, written straight into the season, named
/// `username_of` its id.
async fn seed_placed(app: &App, profile_id: &str, rating: f64, ladder: i32, extra: Value) {
    seed_profile(
        app,
        json!({ "id": profile_id, "rating": rating, "username": username_of(profile_id) }),
    )
    .await;
    let rank = merged(
        to_json(&fresh_rank(&season(), profile_id, 0)),
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
            &to_json(&first),
            &json!({ "season": { "id": season(), "patchVersion": patch() }, "opened": true, "reset": null }),
            "first",
        );
        let again = open_season(&app).await.expect("openSeason");
        assert_matches(
            &to_json(&again),
            &json!({ "opened": false, "reset": null }),
            "again",
        );
        assert_eq!(fake_lock(&app).await.tables.seasons.len(), 1);
        assert_eq!(rating_of(&profile(&app, A).await), 1300.0);
    }

    #[tokio::test(start_paused = true)]
    async fn r609_a_new_minor_version_opens_the_next_season_with_a_soft_reset_of_every_rated_player_and_nobody_else()
     {
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
        let bot_before = to_json(&q!(app, ranked_bot("ai-hard")));
        assert!(
            !a.is_null() && !b.is_null() && !bot_before.is_null(),
            "premise: all three were rated"
        );
        let mean = (rating_of(&a) + rating_of(&b)) / 2.0;

        let next_patch = next_minor(&patch());
        let deps = SeasonDeps {
            patch_version: next_patch.clone(),
        };
        let mut t = app.db.begin(None).await.expect("begin");
        let next = open_season_in_tx(&mut t, &deps).await.expect("openSeason");
        t.commit().await.expect("commit");
        assert_matches(
            &to_json(&next),
            &json!({
                "season": { "id": season_id_of(&next_patch), "patchVersion": next_patch },
                "opened": true,
                "reset": { "players": 2 },
            }),
            "next",
        );
        let after = [profile(&app, A).await, profile(&app, B).await];
        assert_close(
            rating_of(&after[0]),
            mean + (rating_of(&a) - mean) * (1.0 - SEASON_RESET_STRENGTH),
        );
        assert_close(
            rating_of(&after[1]),
            mean + (rating_of(&b) - mean) * (1.0 - SEASON_RESET_STRENGTH),
        );
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
        assert_eq!(to_json(&q!(app, ranked_bot("ai-hard"))), bot_before);
        // Everyone starts the new season back in placements; the old season's rows are kept. TS read
        // this through `ownRank({ ...deps, patchVersion: "v0.2.0" }, A)`; the Rust test app runs as
        // one patch, so the same fact is read off the store: no row in the new season, which
        // `own_rank` answers as a Raisin with no placements played (R605).
        assert!(to_json(&q!(app, ranked_rank(&season_id_of(&next_patch), A))).is_null());
        let seasons: Vec<Value> = to_json(&q!(app, ranked_ranks_of(A)))
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
        let ids: Vec<String> = fake_lock(&app)
            .await
            .tables
            .seasons
            .iter()
            .map(|season| season.id.clone())
            .collect();
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
            seed_placed(
                &app,
                &format!("field-{n}"),
                700.0 + 100.0 * f64::from(n),
                tier_bottom(1),
                json!({}),
            )
            .await;
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
        assert_eq!(
            rate(&app, game("m-1", [player(A), player(B)], Some(1), 1)).await,
            first
        );
        assert_eq!(profile(&app, A).await, before);
        assert_eq!(fake_lock(&app).await.tables.rated_games.len(), 1);
    }
}

// ---------------------------------------------------------------------------
// R610 bots
// ---------------------------------------------------------------------------

mod r610_bots {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn r610_a_bot_is_rated_like_a_player_from_its_own_rating_and_an_id_with_no_profile_holds_no_season_row()
     {
        let app = test_app().await;
        seed_profile(&app, json!({ "id": A })).await;
        let first = rate(&app, game("b-1", [player(A), bot("ai-easy")], Some(0), 1)).await;
        assert_matches(
            &first["sides"][1],
            &json!({
                "profileId": null,
                "botId": "ai-easy",
                "pilot": "ai",
                "before": to_json(&START_GLICKO),
                "rankBefore": null,
                "rankAfter": null,
            }),
            "sides[1]",
        );
        let easy = to_json(&q!(app, ranked_bot("ai-easy")));
        assert_eq!(easy["games"], json!(1));
        assert!(easy["glicko"]["rating"].as_f64().expect("a rating") < START_GLICKO.rating);
        assert_eq!(easy["glicko"], first["sides"][1]["after"]);

        // The bot's next game starts from the rating its last one left it.
        let second = rate(&app, game("b-2", [bot("ai-easy"), player(A)], Some(0), 1)).await;
        assert_eq!(second["sides"][0]["before"], first["sides"][1]["after"]);
        assert_eq!(to_json(&q!(app, ranked_bot("ai-easy")))["games"], json!(2));

        let ranked: Vec<String> = fake_lock(&app)
            .await
            .tables
            .season_ranks
            .iter()
            .map(|rank| rank.profile_id.clone())
            .collect();
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
            "glicko": merged(to_json(&START_GLICKO), json!({ "rating": 400 })),
            "games": 50,
            "updatedAt": 0,
        }));
        q!(app, ranked_put_bot(&hard));
        for n in 1..=(RANK_PLACEMENT_GAMES as i64) {
            rate(
                &app,
                game(&format!("g-{n}"), [player(A), bot("ai-hard")], Some(0), n),
            )
            .await;
        }
        // Rated below the one placed player, alone with them: the 25th percentile, in Normal Grape.
        assert_matches(&own(&app, A).await["rank"], &json!({ "tier": "normal" }), "rank");
    }
}

// ---------------------------------------------------------------------------
// R2755000 the Jlorious card back
// ---------------------------------------------------------------------------

mod r2755000_the_jlorious_card_back {
    use super::*;
    use jackioh_server::cosmetics::JLORIOUS_CARD_BACK;
    use jackioh_server::economy::store::EconomyReason;

    use super::r640000_bots_on_the_ladder::seed_placed_bot;

    /// The Jlorious card back's ledger rows of `profile_id`: each one's reason and reference.
    async fn backs(app: &App, profile_id: &str) -> Vec<(EconomyReason, Option<String>)> {
        let mut tx = app.db.begin(Some(profile_id)).await.expect("begin");
        let ledger = tx.economy_ledger_get(profile_id).await.expect("the ledger");
        tx.commit().await.expect("commit");
        ledger
            .cosmetics
            .into_iter()
            .filter(|row| row.cosmetic_id == JLORIOUS_CARD_BACK)
            .map(|row| (row.reason, row.reference))
            .collect()
    }

    /// R608's season: m-high, A and m-low in Mythic Grape, golden rated above them all but Golden
    /// Grape, and B below; no peak yet.
    async fn r608_season(app: &App) {
        let mythic = tier_bottom(4);
        seed_placed(app, "m-high", 1900.0, mythic, json!({})).await;
        seed_placed(app, "m-low", 1500.0, mythic + 2, json!({})).await;
        seed_placed(app, "golden", 2500.0, tier_bottom(3) + 8, json!({})).await;
        seed_placed(app, A, 1400.0, mythic + 1, json!({})).await;
        seed_placed(app, B, 1300.0, tier_bottom(1), json!({})).await;
    }

    #[tokio::test(start_paused = true)]
    async fn r2755000_the_game_that_leaves_people_jlorious_grants_each_the_card_back_once() {
        let app = test_app().await;
        r608_season(&app).await;
        // A beats B into Jlorious #2; m-high (#1) and m-low (#3) are Jlorious beside them, bystanders
        // whose first peak this game writes.
        rate(&app, game("j-1", [player(A), player(B)], Some(0), 1)).await;
        let granted = vec![(EconomyReason::Jlorious, Some("j-1".to_string()))];
        for jlorious in [A, "m-high", "m-low"] {
            assert_eq!(backs(&app, jlorious).await, granted, "{jlorious}");
        }
        // The loser, and golden, rated highest but not in Mythic Grape: nothing.
        assert!(backs(&app, B).await.is_empty());
        assert!(backs(&app, "golden").await.is_empty());

        // Jlorious again after the next game: owned already, so nothing more is written.
        rate(&app, game("j-2", [player(A), player(B)], Some(0), 2)).await;
        assert_eq!(backs(&app, A).await, granted);
    }

    #[tokio::test(start_paused = true)]
    async fn r2755000_a_ladder_bot_never_gets_it_and_the_economy_off_grants_nothing() {
        // R640000: a Mythic bot rated above everyone stays Mythic Grape, so it never owns it.
        let app = test_app().await;
        r608_season(&app).await;
        seed_placed_bot(&app, "bot-mythic", 3000.0, tier_bottom(4) + 3).await;
        rate(&app, game("j-1", [player(A), player(B)], Some(0), 1)).await;
        rate(&app, game("j-2", [bot("bot-mythic"), player(B)], Some(0), 2)).await;
        assert!(backs(&app, "bot-mythic").await.is_empty());
        assert!(!backs(&app, A).await.is_empty());

        // §14.12: while the economy is off, A still reaches Jlorious and its peak, and owns nothing.
        let mut off = test_app().await;
        Arc::get_mut(&mut off)
            .expect("the test app has no second handle yet")
            .economy_live = false;
        r608_season(&off).await;
        rate(&off, game("j-1", [player(A), player(B)], Some(0), 1)).await;
        assert_eq!(
            to_json(&q!(off, ranked_rank(&season(), A)))["peakJlorious"],
            json!(2)
        );
        assert!(backs(&off, A).await.is_empty());
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
            .map(|row| json!([row["position"], row["profileId"], row["username"], row["you"]]))
            .collect();
        assert_eq!(
            jlorious,
            vec![
                json!([1, "m-high", "m_high", false]),
                json!([2, A, username_of(A), true]),
                json!([3, "m-low", "m_low", false]),
            ]
        );
        let peak = |rank: Value| rank["peakJlorious"].clone();
        assert_eq!(peak(to_json(&q!(app, ranked_rank(&season(), A)))), json!(2));
        assert_eq!(
            own(&app, A).await["rank"],
            json!({ "tier": "jlorious", "position": 2 })
        );
        assert_eq!(
            own(&app, A).await["badges"],
            json!([{ "seasonId": season(), "tier": "jlorious", "position": 2 }])
        );
        // m-low fell to #3 and keeps the #2 it never held: no peak is invented for it.
        assert_eq!(peak(to_json(&q!(app, ranked_rank(&season(), "m-low")))), json!(3));
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
            .map(|row| row["username"].clone())
            .collect();
        assert_eq!(golden, vec![json!("golden")]);
        assert_eq!(JLORIOUS_SIZE, 100);
    }

    /// #2022: GET /api/ranked, the match screen's ranks and the telemetry's tiers read only the rows
    /// they show and Mythic Grape's, never the whole season, and rank each player as the leaderboard,
    /// which still reads it whole, does: A in Jlorious mid-list, B the first Mythic Grape player past
    /// its cut.
    #[tokio::test(start_paused = true)]
    async fn r608_the_rank_reads_skip_the_whole_season_and_rank_as_the_leaderboard_does() {
        let app = test_app().await;
        let mythic = tier_bottom(4);
        for n in 0..i32::try_from(JLORIOUS_SIZE).expect("a size") - 1 {
            seed_placed(
                &app,
                &format!("m-{n:03}"),
                2000.0 - f64::from(n),
                mythic,
                json!({}),
            )
            .await;
        }
        seed_placed(&app, A, 1950.5, mythic + 1, json!({})).await;
        seed_placed(&app, B, 900.0, mythic + 2, json!({})).await;
        seed_placed(&app, "golden", 2500.0, tier_bottom(3) + 8, json!({})).await;
        q!(app, matches_create(&live_match("match-1", "s", true)));
        fake_lock(&app).await.on_call = Some(Arc::new(|called: &str| {
            if called == "ranked.standings" {
                Err(StoreError::Other("a rank read read the whole season".to_string()))
            } else {
                Ok(())
            }
        }));

        let a_rank = own(&app, A).await["rank"].clone();
        let b_rank = own(&app, B).await["rank"].clone();
        assert_eq!(a_rank, json!({ "tier": "jlorious", "position": 51 }));
        assert_matches(&b_rank, &json!({ "tier": "mythic" }), "B, past Jlorious's cut");
        let seats = to_json(
            &match_ranks(&app, "match-1", A)
                .await
                .expect("matchRanks")
                .expect("A's match"),
        )["seats"]
            .clone();
        assert_eq!(seats["p1"]["rank"], a_rank);
        assert_eq!(seats["p2"]["rank"], b_rank);
        assert_eq!(
            ladder_tiers(&app, &(A.to_string(), B.to_string()))
                .await
                .expect("ladderTiers"),
            (RankTier::Jlorious, RankTier::Mythic)
        );

        fake_lock(&app).await.on_call = None;
        let listed = board(&app, A).await;
        assert_eq!(listed["jlorious"][50]["profileId"], A);
        assert_eq!(listed["you"], a_rank);
        assert_eq!(board(&app, B).await["you"], b_rank);
    }
}

// ---------------------------------------------------------------------------
// R640000 ladder bots on the ladder
// ---------------------------------------------------------------------------

mod r640000_bots_on_the_ladder {
    use super::*;

    /// A ladder bot as the pool writes one (R1454): a bot profile at the starting rating, named
    /// `username_of` its id, and its own R610 rating at `rating`.
    async fn seed_bot(app: &App, bot_id: &str, rating: f64) {
        seed_profile(
            app,
            json!({ "id": bot_id, "kind": "bot", "username": username_of(bot_id) }),
        )
        .await;
        let own: BotRating = from(json!({
            "botId": bot_id,
            "glicko": merged(to_json(&START_GLICKO), json!({ "rating": rating })),
            "games": 50,
            "updatedAt": 0,
        }));
        q!(app, ranked_put_bot(&own));
    }

    /// `seed_placed` for a ladder bot.
    pub(super) async fn seed_placed_bot(app: &App, bot_id: &str, rating: f64, ladder: i32) {
        seed_bot(app, bot_id, rating).await;
        put_rank(
            app,
            merged(
                to_json(&fresh_rank(&season(), bot_id, 0)),
                json!({
                    "games": RANK_PLACEMENT_GAMES,
                    "wins": RANK_PLACEMENT_GAMES,
                    "ladder": ladder,
                    "floor": ladder / tier_bottom(1),
                    "peakLadder": ladder,
                }),
            ),
        )
        .await;
    }

    #[tokio::test(start_paused = true)]
    async fn r640000_a_bot_profile_climbs_its_own_season_row_and_its_rated_side_records_its_ranks() {
        let app = test_app().await;
        seed_profile(&app, json!({ "id": A })).await;
        seed_bot(&app, "bot-1", 1000.0).await;
        let placements = RANK_PLACEMENT_GAMES as i64;
        let mut last = Value::Null;
        for n in 1..=placements {
            last = rate(
                &app,
                game(&format!("b-{n}"), [player(A), bot("bot-1")], Some(1), n),
            )
            .await;
        }
        let row = to_json(&q!(app, ranked_rank(&season(), "bot-1")));
        assert_matches(
            &row,
            &json!({ "games": RANK_PLACEMENT_GAMES, "wins": RANK_PLACEMENT_GAMES, "losses": 0 }),
            "the bot's row",
        );
        assert!(row["ladder"].is_number(), "placed: {row}");
        // The record still names the side a bot's; it carries the bot's ranks as a player's would.
        // Placed with no placed person to be read against, the median: Normal Grape.
        assert_matches(
            &last["sides"][1],
            &json!({
                "profileId": null,
                "botId": "bot-1",
                "pilot": "ai",
                "rankBefore": raisin(placements - 1),
                "rankAfter": { "tier": "normal" },
            }),
            "sides[1]",
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r640000_a_bot_on_the_ladder_moves_no_person_s_percentile() {
        let app = test_app().await;
        seed_placed(&app, "placed", 2000.0, tier_bottom(2), json!({})).await;
        for n in 0..5 {
            seed_placed_bot(&app, &format!("bot-{n}"), 3000.0, tier_bottom(4)).await;
        }
        seed_profile(&app, json!({ "id": A })).await;
        for n in 1..=(RANK_PLACEMENT_GAMES as i64) {
            rate(
                &app,
                game(&format!("g-{n}"), [player(A), bot("bot-0")], Some(1), n),
            )
            .await;
        }
        // Read against the one placed person alone: the 25th percentile, in Normal Grape. Counting
        // the five bots above them would put A at 1 in 14, in Rotten Grape.
        assert_matches(&own(&app, A).await["rank"], &json!({ "tier": "normal" }), "rank");
    }

    #[tokio::test(start_paused = true)]
    async fn r640000_jlorious_skips_a_bot_that_would_qualify_and_it_stays_mythic_grape() {
        let app = test_app().await;
        // R608's season, and a placed Mythic bot rated above everyone in it.
        let mythic = tier_bottom(4);
        seed_placed(&app, "m-high", 1900.0, mythic, json!({})).await;
        seed_placed(&app, "m-low", 1500.0, mythic + 2, json!({})).await;
        seed_placed(&app, "golden", 2500.0, tier_bottom(3) + 8, json!({})).await;
        seed_placed(&app, A, 1400.0, mythic + 1, json!({})).await;
        seed_placed(&app, B, 1300.0, tier_bottom(1), json!({})).await;
        seed_placed_bot(&app, "bot-mythic", 3000.0, mythic + 3).await;
        rate(&app, game("j-1", [player(A), player(B)], Some(0), 1)).await;
        let played = rate(&app, game("j-2", [bot("bot-mythic"), player(B)], Some(0), 2)).await;

        let listed = board(&app, A).await;
        let jlorious: Vec<Value> = listed["jlorious"]
            .as_array()
            .expect("a list")
            .iter()
            .map(|row| row["profileId"].clone())
            .collect();
        assert_eq!(jlorious, vec![json!("m-high"), json!(A), json!("m-low")]);
        let in_mythic: Vec<Value> = listed["tiers"]
            .as_array()
            .expect("tiers")
            .iter()
            .find(|tier| tier["tier"] == json!("mythic"))
            .expect("a mythic tier")["players"]
            .as_array()
            .expect("players")
            .iter()
            .map(|row| row["profileId"].clone())
            .collect();
        assert_eq!(in_mythic, vec![json!("bot-mythic")]);
        assert_matches(
            &played["sides"][0],
            &json!({ "rankAfter": { "tier": "mythic" } }),
            "sides[0]",
        );
        assert_eq!(
            to_json(&q!(app, ranked_rank(&season(), "bot-mythic")))["peakJlorious"],
            Value::Null
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r640000_get_api_leaderboard_and_match_ranks_show_a_bot_like_a_player() {
        let app = test_app().await;
        let token = add_user(&app, &format!("user-{A}"), "a@example.test", true);
        seed_profile(
            &app,
            json!({ "id": A, "userId": format!("user-{A}"), "username": "Alice" }),
        )
        .await;
        seed_bot(&app, "bot-1", 1000.0).await;
        for id in ["bot-1", A] {
            put_rank(
                &app,
                merged(
                    to_json(&fresh_rank(&season(), id, 0)),
                    json!({
                        "games": RANK_PLACEMENT_GAMES,
                        "ladder": tier_bottom(1) + 4,
                        "floor": 1,
                        "peakLadder": tier_bottom(1) + 4,
                    }),
                ),
            )
            .await;
        }

        let (status, _headers, listed) =
            call(&app, "GET", "/api/leaderboard", Some(&token), Value::Null).await;
        assert_eq!(status, 200, "{listed}");
        let normal = listed["tiers"]
            .as_array()
            .expect("tiers")
            .iter()
            .find(|tier| tier["tier"] == json!("normal"))
            .cloned()
            .expect("a normal tier");
        // A tie on the ladder is broken by profile id, as between two people.
        assert_eq!(
            normal["players"],
            json!([
                { "profileId": "bot-1", "username": "bot_1", "division": 2, "pips": 1, "you": false },
                { "profileId": A, "username": "Alice", "division": 2, "pips": 1, "you": true },
            ])
        );

        let mut row = live_match("match-1", "s", true);
        row.players.1 = String::from("bot-1");
        q!(app, matches_create(&row));
        let (status, _headers, ranks) = call(
            &app,
            "GET",
            "/api/matches/match-1/ranks",
            Some(&token),
            Value::Null,
        )
        .await;
        assert_eq!(status, 200, "{ranks}");
        assert_eq!(ranks["seats"]["p2"]["rank"], ranks["seats"]["p1"]["rank"]);
        assert_matches(
            &ranks["seats"]["p2"]["rank"],
            &json!({ "tier": "normal", "division": 2, "pips": 1 }),
            "the bot's seat",
        );
        for body in [&listed, &ranks] {
            let text = body.to_string();
            for tell in ["\"kind\"", "\"pilot", "\"bot\""] {
                assert!(!text.contains(tell), "{tell} in {text}");
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn r2388000_a_bot_s_seat_carries_its_recorded_move_like_a_player_s() {
        let app = test_app().await;
        seed_profile(&app, json!({ "id": A })).await;
        seed_bot(&app, "bot-1", 1000.0).await;
        let mut row = live_match("match-1", "s", true);
        row.players.1 = String::from("bot-1");
        q!(app, matches_create(&row));
        let input: RecordResultInput = from(json!({
            "matchId": "match-1",
            "seats": [
                { "profileId": A, "player": "p1", "deck": [] },
                { "profileId": "bot-1", "player": "p2", "deck": [] },
            ],
            "outcome": { "winner": "p1", "reason": "hero-death" },
            "turns": 4,
            "at": 5,
        }));
        record_result(&app, input).await.expect("recordResult");
        let recorded = to_json(&q!(app, ranked_game("match-1")).expect("the rated record"));
        assert_eq!(recorded["sides"][1]["botId"], json!("bot-1"));
        let body = to_json(
            &match_ranks(&app, "match-1", A)
                .await
                .expect("matchRanks")
                .expect("a body"),
        );
        assert_eq!(
            body["seats"]["p2"]["rankBefore"],
            recorded["sides"][1]["rankBefore"]
        );
        assert_eq!(
            body["seats"]["p2"]["rankAfter"],
            recorded["sides"][1]["rankAfter"]
        );
        assert_eq!(body["seats"]["p2"]["rankAfter"], raisin(1));
        let text = body.to_string();
        for tell in ["\"kind\"", "\"pilot", "\"bot\"", "botId", "rating"] {
            assert!(!text.contains(tell), "{tell} in {text}");
        }
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
        seed_profile(
            &app,
            json!({ "id": A, "userId": format!("user-{A}"), "rating": 1234.5678, "username": "Alice" }),
        )
        .await;
        seed_profile(
            &app,
            json!({ "id": B, "userId": format!("user-{B}"), "rating": 987.654, "username": "Bob", "usernameTag": 3 }),
        )
        .await;
        seed_profile(&app, json!({ "id": "stranger", "userId": "user-stranger" })).await;
        Routed {
            app,
            token_a,
            stranger,
        }
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
    async fn r612_r1436_get_api_ranked_answers_the_caller_s_username_rank_streak_record_and_badges_and_never_a_rating()
     {
        let Routed { app, token_a, .. } = routed().await;
        let got = get(&app, "/api/ranked", &token_a).await;
        assert_eq!(got.status, 200);
        assert_eq!(
            got.body,
            json!({
                "season": season(),
                "profileId": A,
                "username": "Alice",
                "rank": raisin(0),
                "streak": 0,
                "record": { "games": 0, "wins": 0, "losses": 0, "draws": 0 },
                "badges": [],
            })
        );
        mentions_none(&got.text, &["1234", "rating", "deviation", "volatility"]);
    }

    #[tokio::test(start_paused = true)]
    async fn r612_r1436_get_api_leaderboard_lists_usernames_and_ranks_marks_the_caller_and_counts_the_raisins()
     {
        let Routed { app, token_a, .. } = routed().await;
        put_rank(
            &app,
            merged(
                to_json(&fresh_rank(&season(), B, 0)),
                json!({ "games": 2, "wins": 2 }),
            ),
        )
        .await;
        put_rank(
            &app,
            merged(
                to_json(&fresh_rank(&season(), A, 0)),
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
        let tiers: Vec<Value> = got.body["tiers"]
            .as_array()
            .expect("tiers")
            .iter()
            .map(|tier| tier["tier"].clone())
            .collect();
        assert_eq!(
            tiers,
            vec![
                json!("mythic"),
                json!("golden"),
                json!("large"),
                json!("normal"),
                json!("rotten")
            ]
        );
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
                "players": [{ "profileId": A, "username": "Alice", "division": 2, "pips": 1, "you": true }],
            })
        );
        assert_matches(
            &got.body["you"],
            &json!({ "tier": "normal", "division": 2, "pips": 1 }),
            "you",
        );
        mentions_none(&got.text, &["1234", "987", "rating", "\"tag\""]);
    }

    #[tokio::test(start_paused = true)]
    async fn r612_r1436_get_api_matches_id_ranks_shows_both_seats_to_a_player_of_the_match_and_404s_for_anyone_else()
     {
        let Routed {
            app,
            token_a,
            stranger,
        } = routed().await;
        let row = live_match("match-1", "s", true);
        q!(app, matches_create(&row));
        let mine = get(&app, "/api/matches/match-1/ranks", &token_a).await;
        assert_eq!(mine.status, 200);
        assert_eq!(
            mine.body,
            json!({
                "ranked": true,
                "seats": {
                    "p1": { "profileId": A, "username": "Alice", "rank": raisin(0), "streak": 0, "you": true },
                    "p2": { "profileId": B, "username": "Bob#3", "rank": raisin(0), "streak": 0, "you": false },
                },
            })
        );
        mentions_none(&mine.text, &["rating", "\"tag\""]);
        assert_eq!(
            get(&app, "/api/matches/match-1/ranks", &stranger).await.status,
            404
        );
        assert_eq!(
            get(&app, "/api/matches/no-such-match/ranks", &token_a)
                .await
                .status,
            404
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r2388000_get_api_matches_id_ranks_carries_each_seat_s_recorded_move_once_the_result_is_written()
    {
        let Routed { app, token_a, .. } = routed().await;
        q!(app, matches_create(&live_match("match-1", "s", true)));
        q!(app, matches_create(&live_match("match-2", "t", false)));
        for id in ["match-1", "match-2"] {
            let got = get(&app, &format!("/api/matches/{id}/ranks"), &token_a).await;
            assert_eq!(got.status, 200, "{id}");
            for side in ["p1", "p2"] {
                assert!(got.body["seats"][side].get("rankBefore").is_none(), "{id} {side}");
                assert!(got.body["seats"][side].get("rankAfter").is_none(), "{id} {side}");
            }
        }
        for id in ["match-1", "match-2"] {
            let input: RecordResultInput = from(json!({
                "matchId": id,
                "seats": [
                    { "profileId": A, "player": "p1", "deck": [] },
                    { "profileId": B, "player": "p2", "deck": [] },
                ],
                "outcome": { "winner": "p1", "reason": "hero-death" },
                "turns": 4,
                "at": 5,
            }));
            record_result(&app, input).await.expect("recordResult");
        }
        let recorded = to_json(&q!(app, ranked_game("match-1")).expect("the rated record"));
        let got = get(&app, "/api/matches/match-1/ranks", &token_a).await;
        assert_eq!(got.status, 200, "{}", got.text);
        for (side, index) in [("p1", 0), ("p2", 1)] {
            assert_eq!(
                got.body["seats"][side]["rankBefore"], recorded["sides"][index]["rankBefore"],
                "{side}"
            );
            assert_eq!(
                got.body["seats"][side]["rankAfter"], recorded["sides"][index]["rankAfter"],
                "{side}"
            );
        }
        assert_eq!(got.body["seats"]["p1"]["rankBefore"], raisin(0));
        assert_eq!(got.body["seats"]["p1"]["rankAfter"], raisin(1));
        assert_eq!(got.body["seats"]["p1"]["rank"], raisin(1));
        mentions_none(&got.text, &["rating", "deviation", "volatility", "\"tag\""]);
        let unranked = get(&app, "/api/matches/match-2/ranks", &token_a).await;
        assert_eq!(unranked.status, 200);
        for side in ["p1", "p2"] {
            assert!(unranked.body["seats"][side].get("rankBefore").is_none(), "{side}");
            assert!(unranked.body["seats"][side].get("rankAfter").is_none(), "{side}");
        }
        assert!(q!(app, ranked_game("match-2")).is_none());
    }

    /// R2101000: both seats carry their season row's win streak, the opponent's included.
    #[tokio::test(start_paused = true)]
    async fn r2101000_get_api_matches_id_ranks_carries_each_seat_s_win_streak() {
        let Routed { app, token_a, .. } = routed().await;
        put_rank(
            &app,
            merged(
                to_json(&fresh_rank(&season(), A, 0)),
                json!({ "games": 4, "wins": 4, "streak": 4 }),
            ),
        )
        .await;
        put_rank(
            &app,
            merged(
                to_json(&fresh_rank(&season(), B, 0)),
                json!({ "games": 3, "wins": 2, "losses": 1, "streak": 0 }),
            ),
        )
        .await;
        q!(app, matches_create(&live_match("match-1", "s", true)));
        let got = get(&app, "/api/matches/match-1/ranks", &token_a).await;
        assert_eq!(got.status, 200);
        assert_eq!(got.body["seats"]["p1"]["streak"], json!(4));
        assert_eq!(got.body["seats"]["p2"]["streak"], json!(0));
        mentions_none(&got.text, &["rating", "\"tag\""]);
    }

    /// R1436: the ladder names nobody; it joins each username in from `profiles`, so a rename shows
    /// on the next read with no rank row rewritten.
    #[tokio::test(start_paused = true)]
    async fn r1436_a_rename_shows_on_the_leaderboard_at_once_and_rewrites_no_rank_row() {
        let Routed { app, token_a, .. } = routed().await;
        let placed = merged(
            to_json(&fresh_rank(&season(), A, 0)),
            json!({
                "games": RANK_PLACEMENT_GAMES,
                "ladder": tier_bottom(1) + 4,
                "floor": 1,
                "peakLadder": tier_bottom(1) + 4,
            }),
        );
        put_rank(&app, placed).await;
        let row_before = to_json(&q!(app, ranked_rank(&season(), A)));
        let names = |body: &Value| -> Vec<Value> {
            body["tiers"]
                .as_array()
                .expect("tiers")
                .iter()
                .flat_map(|tier| tier["players"].as_array().cloned().unwrap_or_default())
                .map(|row| json!([row["profileId"], row["username"]]))
                .collect()
        };
        let before = get(&app, "/api/leaderboard", &token_a).await;
        assert_eq!(names(&before.body), vec![json!([A, "Alice"])]);

        let (status, _headers, renamed) = call(
            &app,
            "PUT",
            "/api/username",
            Some(&token_a),
            json!({ "username": "Alicia" }),
        )
        .await;
        assert_eq!(status, 200, "{renamed}");

        let after = get(&app, "/api/leaderboard", &token_a).await;
        assert_eq!(names(&after.body), vec![json!([A, "Alicia"])]);
        assert_eq!(
            get(&app, "/api/ranked", &token_a).await.body["username"],
            json!("Alicia")
        );
        assert_eq!(to_json(&q!(app, ranked_rank(&season(), A))), row_before);
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
            q!(app, series_create(&series));
            let game_1 = live_match("game-1", "s:3", ranked);
            q!(app, matches_create(&game_1));
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

            let ended = to_json(&q!(app, series_get("series-1")));
            assert_eq!(ended["status"], json!("over"), "ranked: {ranked}");
            let rating = profile(&app, A).await["rating"].as_f64().unwrap_or(0.0);
            let rated: Vec<Value> = fake_lock(&app)
                .await
                .tables
                .rated_games
                .iter()
                .map(|row| {
                    let row = to_json(row);
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

    #[tokio::test(start_paused = true)]
    async fn r2388000_a_ranked_series_last_game_carries_the_series_move_and_an_earlier_game_none() {
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
            "ranked": true,
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
        q!(app, series_create(&series));
        q!(app, matches_create(&live_match("game-1", "s:3", true)));
        q!(app, matches_create(&live_match("g-b", "s:2", true)));
        let before = match_ranks(&app, "game-1", A)
            .await
            .expect("matchRanks")
            .expect("a body");
        assert!(to_json(&before)["seats"]["p1"].get("rankAfter").is_none());
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
        let recorded = to_json(&q!(app, ranked_game("series-1")).expect("the series' record"));
        let after = to_json(
            &match_ranks(&app, "game-1", A)
                .await
                .expect("matchRanks")
                .expect("a body"),
        );
        for (side, index) in [("p1", 0), ("p2", 1)] {
            assert_eq!(
                after["seats"][side]["rankBefore"], recorded["sides"][index]["rankBefore"],
                "{side}"
            );
            assert_eq!(
                after["seats"][side]["rankAfter"], recorded["sides"][index]["rankAfter"],
                "{side}"
            );
        }
        assert!(after["seats"]["p1"]["rankAfter"].is_object());
        let earlier = to_json(
            &match_ranks(&app, "g-b", A)
                .await
                .expect("matchRanks")
                .expect("a body"),
        );
        for side in ["p1", "p2"] {
            assert!(earlier["seats"][side].get("rankBefore").is_none(), "{side}");
            assert!(earlier["seats"][side].get("rankAfter").is_none(), "{side}");
        }
        assert!(q!(app, ranked_game("game-1")).is_none());
    }
}

// ---------------------------------------------------------------------------
// R1560000: a bot's rating re-settles after an AI promotion
// ---------------------------------------------------------------------------

mod r1560000_a_bot_after_an_ai_promotion {
    use super::*;

    /// The AI generation the test build plays its bots on.
    fn generation() -> i64 {
        i64::from(jackioh_ai::ai_generation().expect("crates/ai/generation.json names one"))
    }

    /// `ai-hard`, settled at 1200 with `deviation` after 40 games, last rated on `ai_generation`
    /// (`None`: a row from before R1560000).
    async fn put_bot(app: &App, deviation: f64, ai_generation: Option<i64>) {
        let mut row = json!({
            "botId": "ai-hard",
            "glicko": { "rating": 1200.0, "deviation": deviation, "volatility": 0.06 },
            "games": 40,
            "updatedAt": 0,
        });
        if let Some(generation) = ai_generation {
            row["aiGeneration"] = json!(generation);
        }
        let row: BotRating = from(row);
        q!(app, ranked_put_bot(&row));
    }

    /// The bot's side of a rated game `id` that player A wins, as the record holds it.
    async fn bot_side(app: &App, id: &str, at: i64) -> Value {
        rate(app, game(id, [player(A), bot("ai-hard")], Some(0), at)).await["sides"][1].clone()
    }

    fn widened(deviation: f64, promotions: f64) -> f64 {
        (deviation * deviation + promotions * BOT_PROMOTION_DEVIATION_BOOST * BOT_PROMOTION_DEVIATION_BOOST)
            .sqrt()
    }

    #[tokio::test(start_paused = true)]
    async fn r1560000_a_bot_last_rated_on_an_older_ai_is_rated_from_a_widened_deviation_and_stamped() {
        let app = test_app().await;
        seed_profile(&app, json!({ "id": A })).await;
        put_bot(&app, 60.0, Some(generation() - 2)).await;
        let side = bot_side(&app, "g-1", 1).await;
        // Two promotions: the deviation widened twice in quadrature, the rating and volatility kept.
        assert_matches(
            &side["before"],
            &json!({ "rating": 1200.0, "volatility": 0.06 }),
            "before",
        );
        assert_close(
            side["before"]["deviation"].as_f64().expect("a deviation"),
            widened(60.0, 2.0),
        );
        let row = q!(app, ranked_bot("ai-hard")).expect("the bot's row");
        assert_eq!(row.ai_generation, Some(generation()));
        assert_eq!(row.games, 41);
        assert_eq!(to_json(&row.glicko), side["after"]);
        // Stamped, it widens once: the next game is rated from exactly where this one left it.
        let next = bot_side(&app, "g-2", 2).await;
        assert_eq!(next["before"], side["after"]);
    }

    #[tokio::test(start_paused = true)]
    async fn r1560000_a_row_naming_no_generation_counts_as_one_promotion_behind() {
        let app = test_app().await;
        seed_profile(&app, json!({ "id": A })).await;
        put_bot(&app, 60.0, None).await;
        let side = bot_side(&app, "g-1", 1).await;
        assert_close(
            side["before"]["deviation"].as_f64().expect("a deviation"),
            widened(60.0, 1.0),
        );
        assert_eq!(
            q!(app, ranked_bot("ai-hard"))
                .expect("the bot's row")
                .ai_generation,
            Some(generation())
        );
    }

    #[tokio::test(start_paused = true)]
    async fn r1560000_a_bot_rated_on_this_ai_is_not_widened_and_a_widened_one_stops_at_a_new_player_s() {
        let app = test_app().await;
        seed_profile(&app, json!({ "id": A })).await;
        put_bot(&app, 60.0, Some(generation())).await;
        assert_eq!(
            bot_side(&app, "g-1", 1).await["before"]["deviation"].as_f64(),
            Some(60.0)
        );

        let app = test_app().await;
        seed_profile(&app, json!({ "id": A })).await;
        put_bot(&app, 300.0, Some(generation() - 2)).await;
        assert_eq!(
            bot_side(&app, "g-1", 1).await["before"]["deviation"].as_f64(),
            Some(RATING_DEVIATION_START)
        );
    }
}
