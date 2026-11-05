//! The dev dashboard's routes (`src/api/admin.rs`, SPEC §14.10, §14.13; issue #3074): who may reach
//! them (R3074000), the hand-set rating (R3074001), the whole ladder (R3074002), news posts
//! (R3074003) and the audit row every dev and admin write leaves (R3074000).

use std::sync::Arc;

use indexmap::IndexMap;
use serde_json::{Value, json};

use jackioh_server::admin::store::{NewsPost, RatingChange};
use jackioh_server::app::{self, App, now_ms};
use jackioh_server::config::{
    DEV_JLORE_PER_REQUEST_MAX, DEV_RATING_MAX, DEV_RATING_MIN, DEV_XP_PER_REQUEST_MAX, MS_PER_DAY,
    NEWS_TTL_DAYS, RANK_PLACEMENT_GAMES, RATING_START,
};
use jackioh_server::db::store::NewBot;
use jackioh_server::env::{SERVER_ONLY_ENV_VARS, load_env};
use jackioh_server::ranked::ladder::{GrapeTier, Ladder, fresh_rank, tier_bottom};
use jackioh_server::ranked::season::season_id_of;

use crate::support::deps::{TestAppOptions, add_user, call, fake_auth, test_app_with};
use crate::support::store::{fake_lock, from, q};

/// The dev account `DEV_PROFILE_IDS` lists.
const DEV: &str = "d0000000-0000-4000-8000-000000307400";
/// An active account it does not.
const OTHER: &str = "d0000000-0000-4000-8000-000000307401";

/// The tokens of one test app: the dev's without and with the two-step code, the other account's
/// without and with it.
struct Tokens {
    dev: String,
    dev_aal2: String,
    other: String,
    other_aal2: String,
}

/// The app with `DEV` listed, on a strict server or not, and both accounts seeded active.
async fn admin_app(strict: bool) -> (Arc<App>, Tokens) {
    let app = test_app_with(TestAppOptions {
        skip_fixtures: true,
        env: vec![
            ("DEV_PROFILE_IDS".to_string(), DEV.to_string()),
            (
                "DEV_STRICT".to_string(),
                if strict { "on" } else { "off" }.to_string(),
            ),
        ],
        ..TestAppOptions::default()
    })
    .await;
    let dev = account(&app, DEV, 1).await;
    let other = account(&app, OTHER, 2).await;
    let auth = fake_auth(&app);
    let tokens = Tokens {
        dev,
        dev_aal2: auth.add_second_factor_token("user-1").expect("the dev's user"),
        other,
        other_aal2: auth.add_second_factor_token("user-2").expect("the other's user"),
    };
    (app, tokens)
}

/// `id`, seeded active with a username, and its token.
async fn account(app: &Arc<App>, id: &str, n: u32) -> String {
    let user_id = format!("user-{n}");
    fake_lock(app).await.seed_profile(json!({
        "id": id,
        "userId": user_id,
        "status": "active",
        "username": format!("Admin{n}"),
        "usernamePrompted": true,
    }));
    add_user(app, &user_id, &format!("{user_id}@example.test"), true)
}

async fn api(app: &Arc<App>, method: &str, path: &str, token: &str, body: Value) -> (u16, Value) {
    let (status, _headers, body) = call(app, method, path, Some(token), body).await;
    (status, body)
}

/// `api`, which must answer `status`.
async fn expect(app: &Arc<App>, method: &str, path: &str, token: &str, body: Value, status: u16) -> Value {
    let (got, answer) = api(app, method, path, token, body).await;
    assert_eq!(got, status, "{method} {path}: {answer}");
    answer
}

/// Every route of the dashboard, its parameters filled in.
fn admin_routes() -> Vec<(&'static str, String)> {
    app::ROUTES
        .iter()
        .filter(|(_, path, _, _)| path.starts_with("/api/admin/"))
        .map(|(method, path, _, _)| (*method, path.replace(":postId", "post-abc")))
        .collect()
}

/// A write's body that every route would take, so a 403 is the gate's and never the body's.
fn any_body() -> Value {
    json!({ "ladder": "constructed", "rating": 1500, "title": "A post", "summary": "Plain text." })
}

/// The audit rows the store holds, oldest first.
async fn audit_rows(app: &App) -> Vec<jackioh_server::admin::store::AdminAction> {
    fake_lock(app).await.tables.admin.actions.clone()
}

// Who may reach the dashboard (R3074000)

#[tokio::test]
async fn r3074000_every_admin_route_answers_403_to_a_non_dev_and_401_unsigned_writing_nothing() {
    let (app, tokens) = admin_app(true).await;
    let routes = admin_routes();
    assert_eq!(
        routes,
        vec![
            ("GET", "/api/admin/me".to_string()),
            ("GET", "/api/admin/rating".to_string()),
            ("PUT", "/api/admin/rating".to_string()),
            ("POST", "/api/admin/rating/reset".to_string()),
            ("GET", "/api/admin/leaderboard".to_string()),
            ("GET", "/api/admin/news".to_string()),
            ("POST", "/api/admin/news".to_string()),
            ("DELETE", "/api/admin/news/post-abc".to_string()),
            ("GET", "/api/admin/audit".to_string()),
        ]
    );
    for (method, path) in &routes {
        // A non-dev is refused whether or not its session proved a second factor.
        for token in [&tokens.other, &tokens.other_aal2] {
            let answer = expect(&app, method, path, token, any_body(), 403).await;
            assert_eq!(answer["error"]["code"], "forbidden", "{method} {path}: {answer}");
        }
        let (status, _, answer) = call(&app, method, path, None, any_body()).await;
        assert_eq!(status, 401, "{method} {path}: {answer}");
    }
    let tables = fake_lock(&app).await.tables.admin.clone();
    assert_eq!(tables, Default::default(), "a refusal writes nothing");
}

#[tokio::test]
async fn r3074000_a_strict_server_refuses_a_dev_without_the_two_step_code() {
    let (app, tokens) = admin_app(true).await;
    for (method, path) in admin_routes() {
        let answer = expect(&app, method, &path, &tokens.dev, any_body(), 403).await;
        assert_eq!(answer["error"]["code"], "forbidden", "{method} {path}: {answer}");
        assert!(
            answer["error"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("two-step code")),
            "{method} {path}: {answer}"
        );
    }
    assert!(audit_rows(&app).await.is_empty());
    let me = expect(&app, "GET", "/api/admin/me", &tokens.dev_aal2, Value::Null, 200).await;
    assert_eq!(
        me,
        json!({ "profileId": DEV, "username": "Admin1", "strict": true, "secondFactor": true })
    );
    // The dev tools /account uses keep their own gate: only the list (R1300000).
    expect(
        &app,
        "POST",
        "/api/dev/jlore",
        &tokens.dev,
        json!({ "jlore": 5 }),
        200,
    )
    .await;
}

#[tokio::test]
async fn r3074000_a_lax_server_lets_a_dev_in_without_the_two_step_code() {
    let (app, tokens) = admin_app(false).await;
    let me = expect(&app, "GET", "/api/admin/me", &tokens.dev, Value::Null, 200).await;
    assert_eq!(me["strict"], false, "{me}");
    assert_eq!(me["secondFactor"], false, "{me}");
    // Still a dev's alone.
    expect(&app, "GET", "/api/admin/me", &tokens.other, Value::Null, 403).await;
}

#[test]
fn r3074000_dev_strict_is_on_unless_it_says_off() {
    let base = |extra: &[(&str, &str)]| -> IndexMap<String, String> {
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
            ("NODE_ENV", "production"),
        ]
        .into_iter()
        .chain(extra.iter().copied())
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
    };
    let strict = |extra: &[(&str, &str)]| load_env(&base(extra)).map(|env| env.dev_strict);
    assert_eq!(strict(&[]), Ok(true), "unset is on");
    assert_eq!(strict(&[("DEV_STRICT", " ")]), Ok(true), "blank is on");
    assert_eq!(strict(&[("DEV_STRICT", "on")]), Ok(true));
    assert_eq!(strict(&[("DEV_STRICT", "off")]), Ok(false));
    let problems = load_env(&base(&[("DEV_STRICT", "maybe")]))
        .expect_err("a value that is neither")
        .problems;
    assert!(
        problems.iter().any(|problem| problem.starts_with("DEV_STRICT:")),
        "{problems:?}"
    );
    assert!(SERVER_ONLY_ENV_VARS.contains(&"DEV_STRICT"));
}

#[tokio::test]
async fn r3074000_each_dev_grant_is_capped_by_its_constant() {
    let (app, tokens) = admin_app(false).await;
    for (path, body) in [
        (
            "/api/dev/jlore",
            json!({ "jlore": DEV_JLORE_PER_REQUEST_MAX + 1 }),
        ),
        (
            "/api/dev/xp",
            json!({ "setId": "Core", "xp": DEV_XP_PER_REQUEST_MAX + 1 }),
        ),
    ] {
        expect(&app, "POST", path, &tokens.dev, body, 400).await;
    }
    assert!(
        audit_rows(&app).await.is_empty(),
        "a refused grant writes no audit row"
    );
    let answer = expect(
        &app,
        "POST",
        "/api/dev/jlore",
        &tokens.dev,
        json!({ "jlore": DEV_JLORE_PER_REQUEST_MAX }),
        200,
    )
    .await;
    assert_eq!(answer["jlore"], DEV_JLORE_PER_REQUEST_MAX);
}

#[tokio::test]
async fn r3074000_every_dev_and_admin_write_leaves_its_audit_row_and_a_read_none() {
    let (app, tokens) = admin_app(true).await;
    let dev = &tokens.dev_aal2;
    expect(&app, "POST", "/api/dev/jlore", dev, json!({ "jlore": 40 }), 200).await;
    expect(&app, "POST", "/api/dev/jlore", dev, json!({ "jlore": 2 }), 200).await;
    expect(
        &app,
        "POST",
        "/api/dev/xp",
        dev,
        json!({ "setId": "Core", "xp": 100 }),
        200,
    )
    .await;
    expect(&app, "POST", "/api/dev/fill", dev, json!({}), 200).await;
    expect(&app, "POST", "/api/dev/bar/card", dev, json!({}), 200).await;
    let quest = expect(&app, "POST", "/api/dev/quests", dev, json!({}), 200).await;
    let quest_id = quest["quest"]["questId"]
        .as_str()
        .or_else(|| quest["quest"]["id"].as_str())
        .unwrap_or_else(|| panic!("a quest id in {quest}"))
        .to_string();
    expect(
        &app,
        "POST",
        &format!("/api/dev/quests/{quest_id}/complete"),
        dev,
        json!({}),
        200,
    )
    .await;
    expect(&app, "POST", "/api/dev/clear/cards", dev, json!({}), 200).await;
    expect(
        &app,
        "PUT",
        "/api/admin/rating",
        dev,
        json!({ "ladder": "random", "rating": 1700 }),
        200,
    )
    .await;
    expect(
        &app,
        "POST",
        "/api/admin/rating/reset",
        dev,
        json!({ "ladder": "random" }),
        200,
    )
    .await;
    let posted = expect(
        &app,
        "POST",
        "/api/admin/news",
        dev,
        json!({ "title": "Hello", "summary": "A word from the team." }),
        201,
    )
    .await;
    let post_id = posted["post"]["id"].as_str().expect("an id").to_string();
    expect(
        &app,
        "DELETE",
        &format!("/api/admin/news/{post_id}"),
        dev,
        Value::Null,
        200,
    )
    .await;
    // Reads leave nothing.
    let written = audit_rows(&app).await.len();
    for path in [
        "/api/admin/me",
        "/api/admin/rating",
        "/api/admin/leaderboard",
        "/api/admin/news",
        "/api/admin/audit",
    ] {
        expect(&app, "GET", path, dev, Value::Null, 200).await;
    }
    assert_eq!(audit_rows(&app).await.len(), written);

    let audit = expect(&app, "GET", "/api/admin/audit", dev, Value::Null, 200).await;
    let actions = audit["actions"].as_array().expect("actions");
    let routes: Vec<&str> = actions
        .iter()
        .rev()
        .map(|row| row["route"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        routes,
        vec![
            "POST /api/dev/jlore".to_string(),
            "POST /api/dev/jlore".to_string(),
            "POST /api/dev/xp".to_string(),
            "POST /api/dev/fill".to_string(),
            "POST /api/dev/bar/card".to_string(),
            "POST /api/dev/quests".to_string(),
            format!("POST /api/dev/quests/{quest_id}/complete"),
            "POST /api/dev/clear/cards".to_string(),
            "PUT /api/admin/rating".to_string(),
            "POST /api/admin/rating/reset".to_string(),
            "POST /api/admin/news".to_string(),
            format!("DELETE /api/admin/news/{post_id}"),
        ]
    );
    // Who, the request, the values before and after, and when, newest first.
    for row in actions {
        assert_eq!(row["profileId"], DEV, "{row}");
        assert_eq!(row["username"], "Admin1", "{row}");
        assert!(row["at"].as_i64().is_some_and(|at| at > 0), "{row}");
    }
    let second_jlore = &actions[actions.len() - 2];
    assert_eq!(second_jlore["request"], json!({ "jlore": 2 }));
    assert_eq!(second_jlore["before"], json!({ "jlore": 40 }));
    assert_eq!(second_jlore["after"], json!({ "jlore": 42 }));
    let set = actions
        .iter()
        .find(|row| row["route"] == "PUT /api/admin/rating")
        .expect("the rating's row");
    assert_eq!(set["request"], json!({ "ladder": "random", "rating": 1700.0 }));
    assert_eq!(set["before"]["glicko"]["rating"], json!(RATING_START));
    assert_eq!(set["after"]["glicko"]["rating"], json!(1700.0));
    assert_eq!(set["after"]["handSet"], true);
    let ids: Vec<i64> = actions.iter().filter_map(|row| row["id"].as_i64()).collect();
    assert!(
        ids.windows(2).all(|pair| pair[0] > pair[1]),
        "newest first: {ids:?}"
    );
}

// The hand-set rating (R3074001)

/// The caller's rating body's row for `ladder`.
fn ladder_row<'a>(body: &'a Value, ladder: &str) -> &'a Value {
    body["ladders"]
        .as_array()
        .expect("ladders")
        .iter()
        .find(|row| row["ladder"] == ladder)
        .unwrap_or_else(|| panic!("{ladder} in {body}"))
}

#[tokio::test]
async fn r3074001_a_hand_set_rating_keeps_the_earned_one_and_a_reset_puts_it_back() {
    let (app, tokens) = admin_app(true).await;
    let dev = &tokens.dev_aal2;
    let start = expect(&app, "GET", "/api/admin/rating", dev, Value::Null, 200).await;
    let earned = ladder_row(&start, "constructed").clone();
    assert_eq!(earned["handSet"], false);
    assert_eq!(earned["earned"], Value::Null);
    assert_eq!(earned["rating"], json!(RATING_START));
    let order: Vec<&Value> = start["ladders"]
        .as_array()
        .expect("ladders")
        .iter()
        .map(|row| &row["ladder"])
        .collect();
    assert_eq!(order, [&json!("constructed"), &json!("random")]);

    let set = expect(
        &app,
        "PUT",
        "/api/admin/rating",
        dev,
        json!({ "ladder": "constructed", "rating": 2000 }),
        200,
    )
    .await;
    let row = ladder_row(&set, "constructed");
    assert_eq!(row["rating"], json!(2000.0));
    assert_eq!(row["deviation"], earned["deviation"], "the deviation is kept");
    assert_eq!(row["handSet"], true);
    assert_eq!(row["earned"]["rating"], earned["rating"]);
    assert_eq!(
        ladder_row(&set, "random")["handSet"],
        false,
        "the other ladder is its own"
    );

    // A second hand-set rating keeps the earned one from before the first.
    let again = expect(
        &app,
        "PUT",
        "/api/admin/rating",
        dev,
        json!({ "ladder": "constructed", "rating": DEV_RATING_MIN }),
        200,
    )
    .await;
    assert_eq!(ladder_row(&again, "constructed")["rating"], json!(DEV_RATING_MIN));
    assert_eq!(
        ladder_row(&again, "constructed")["earned"]["rating"],
        earned["rating"]
    );
    let profile = fake_lock(&app)
        .await
        .tables
        .profiles
        .iter()
        .find(|profile| profile.id == DEV)
        .cloned()
        .expect("the dev's profile");
    assert_eq!(
        profile.glicko(Ladder::Constructed).rating,
        DEV_RATING_MIN,
        "the rating itself moved"
    );

    // What the server refuses writes nothing.
    for body in [
        json!({ "ladder": "constructed", "rating": DEV_RATING_MIN - 1.0 }),
        json!({ "ladder": "constructed", "rating": DEV_RATING_MAX + 1.0 }),
        json!({ "ladder": "constructed", "rating": "1500" }),
        json!({ "ladder": "constructed" }),
        json!({ "ladder": "Constructed", "rating": 1500 }),
        json!({ "ladder": "conquest", "rating": 1500 }),
        json!({ "rating": 1500 }),
    ] {
        expect(&app, "PUT", "/api/admin/rating", dev, body, 400).await;
    }
    let grants = q!(app, admin_rating_grants_of(DEV));
    let kinds: Vec<(RatingChange, f64, f64, bool)> = grants
        .iter()
        .map(|grant| (grant.change, grant.before.rating, grant.after.rating, grant.dev))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (RatingChange::Set, RATING_START, 2000.0, true),
            (RatingChange::Set, 2000.0, DEV_RATING_MIN, true),
        ]
    );

    let reset = expect(
        &app,
        "POST",
        "/api/admin/rating/reset",
        dev,
        json!({ "ladder": "constructed" }),
        200,
    )
    .await;
    assert_eq!(ladder_row(&reset, "constructed"), &earned, "{reset}");
    let grants = q!(app, admin_rating_grants_of(DEV));
    let last = grants.last().expect("the reset's row");
    assert_eq!(
        (last.change, last.before.rating, last.after.rating, last.dev),
        (RatingChange::Reset, DEV_RATING_MIN, RATING_START, true)
    );
    // Nothing is hand-set now: a second reset is refused.
    let answer = expect(
        &app,
        "POST",
        "/api/admin/rating/reset",
        dev,
        json!({ "ladder": "constructed" }),
        409,
    )
    .await;
    assert_eq!(answer["error"]["code"], "conflict");
    assert_eq!(q!(app, admin_rating_grants_of(DEV)).len(), 3);
}

/// `id` on `ladder` this season, placed in Mythic Grape, rated `rating`.
async fn mythic(app: &App, id: &str, rating: f64) {
    let season = season_id_of(jackioh_cards::catalog_version());
    let mythic = GrapeTier::Mythic as i32;
    let mut rank = fresh_rank(&season, id, 0);
    rank.games = RANK_PLACEMENT_GAMES;
    rank.ladder = Some(tier_bottom(mythic));
    rank.floor = mythic;
    rank.peak_ladder = rank.ladder;
    {
        let mut data = fake_lock(app).await;
        if !data.tables.profiles.iter().any(|profile| profile.id == id) {
            data.seed_profile(json!({ "id": id, "status": "active" }));
        }
        let profile = data
            .tables
            .profiles
            .iter_mut()
            .find(|profile| profile.id == id)
            .expect("the profile");
        profile.rating = rating;
    }
    q!(app, ranked_put_rank(&rank));
}

/// The profile ids the public leaderboard lists, Jlorious first, as `token` reads it.
async fn public_ids(app: &Arc<App>, token: &str) -> (Vec<String>, Value) {
    let board = expect(app, "GET", "/api/leaderboard", token, Value::Null, 200).await;
    let mut ids: Vec<String> = board["jlorious"]
        .as_array()
        .expect("jlorious")
        .iter()
        .map(|row| row["profileId"].as_str().unwrap_or("").to_string())
        .collect();
    for tier in board["tiers"].as_array().expect("tiers") {
        for row in tier["players"].as_array().expect("players") {
            ids.push(row["profileId"].as_str().unwrap_or("").to_string());
        }
    }
    (ids, board)
}

#[tokio::test]
async fn r3074001_a_strict_server_keeps_a_hand_set_rating_off_the_public_ladder_until_reset() {
    let (app, tokens) = admin_app(true).await;
    mythic(&app, DEV, 1000.0).await;
    mythic(&app, OTHER, 1000.0).await;
    let (ids, _) = public_ids(&app, &tokens.other).await;
    assert!(
        ids.contains(&DEV.to_string()) && ids.contains(&OTHER.to_string()),
        "{ids:?}"
    );

    expect(
        &app,
        "PUT",
        "/api/admin/rating",
        &tokens.dev_aal2,
        json!({ "ladder": "constructed", "rating": DEV_RATING_MAX }),
        200,
    )
    .await;
    let (ids, board) = public_ids(&app, &tokens.other).await;
    assert_eq!(
        ids,
        vec![OTHER.to_string()],
        "off the ladder and out of Jlorious: {board}"
    );
    assert_eq!(
        board["jlorious"][0]["position"], 1,
        "the person keeps the first place: {board}"
    );
    // The other ladder is not hand-set: the dev stands on it as before (it holds no row there yet).
    let (status, _, random) = call(
        &app,
        "GET",
        "/api/leaderboard?ladder=random",
        Some(&tokens.other),
        Value::Null,
    )
    .await;
    assert_eq!(status, 200, "{random}");

    // The dev's own dashboard still shows the row, marked.
    let whole = expect(
        &app,
        "GET",
        "/api/admin/leaderboard",
        &tokens.dev_aal2,
        Value::Null,
        200,
    )
    .await;
    let dev_row = whole["rows"]
        .as_array()
        .expect("rows")
        .iter()
        .find(|row| row["profileId"] == DEV)
        .expect("the dev's row");
    assert_eq!(dev_row["handSet"], true);
    assert_eq!(dev_row["you"], true);
    assert_eq!(dev_row["rating"], json!(DEV_RATING_MAX));
    assert_ne!(dev_row["rank"]["tier"], "jlorious", "out of Jlorious: {dev_row}");

    expect(
        &app,
        "POST",
        "/api/admin/rating/reset",
        &tokens.dev_aal2,
        json!({ "ladder": "constructed" }),
        200,
    )
    .await;
    let (ids, _) = public_ids(&app, &tokens.other).await;
    let mut sorted = ids.clone();
    sorted.sort();
    let mut both = vec![DEV.to_string(), OTHER.to_string()];
    both.sort();
    assert_eq!(sorted, both, "back on the ladder once reset");
}

#[tokio::test]
async fn r3074001_a_lax_server_leaves_a_hand_set_rating_on_the_ladder() {
    let (app, tokens) = admin_app(false).await;
    mythic(&app, DEV, 1000.0).await;
    mythic(&app, OTHER, 1000.0).await;
    expect(
        &app,
        "PUT",
        "/api/admin/rating",
        &tokens.dev,
        json!({ "ladder": "constructed", "rating": DEV_RATING_MAX }),
        200,
    )
    .await;
    let (ids, _) = public_ids(&app, &tokens.other).await;
    assert!(ids.contains(&DEV.to_string()), "{ids:?}");
}

// The whole ladder (R3074002)

/// A ladder bot as the pool writes one, placed in Mythic Grape; answers its profile id.
async fn bot(app: &App, id: &str, rating: f64) -> String {
    let new: NewBot = from(json!({
        "profileId": id,
        "strength": 3,
        "speed": 1.0,
        "persona": "polite",
        "activeFromHour": 0,
        "activeHours": 24,
        "decks": [],
        "decksCatalogVersion": null,
        "createdAt": now_ms(),
        "seedRating": rating,
    }));
    q!(app, bots_create(&new));
    let season = season_id_of(jackioh_cards::catalog_version());
    let mut rank = fresh_rank(&season, id, 0);
    rank.games = RANK_PLACEMENT_GAMES;
    rank.ladder = Some(tier_bottom(GrapeTier::Golden as i32));
    q!(app, ranked_put_rank(&rank));
    id.to_string()
}

#[tokio::test]
async fn r3074002_the_whole_ladder_marks_each_row_person_or_bot_for_a_dev_alone() {
    let (app, tokens) = admin_app(false).await;
    mythic(&app, OTHER, 1200.0).await;
    let bot_id = bot(&app, "bot-3074", 1500.0).await;
    // A placing person is a row too: every row, not the public page's top.
    let season = season_id_of(jackioh_cards::catalog_version());
    fake_lock(&app)
        .await
        .seed_profile(json!({ "id": "person-placing", "status": "active", "rating": 900 }));
    q!(app, ranked_put_rank(&fresh_rank(&season, "person-placing", 0)));

    let whole = expect(
        &app,
        "GET",
        "/api/admin/leaderboard",
        &tokens.dev,
        Value::Null,
        200,
    )
    .await;
    assert_eq!(whole["ladder"], "constructed");
    assert_eq!(whole["season"], json!(season));
    let rows = whole["rows"].as_array().expect("rows");
    let summary: Vec<(&str, &str, f64)> = rows
        .iter()
        .map(|row| {
            (
                row["profileId"].as_str().unwrap_or(""),
                row["kind"].as_str().unwrap_or(""),
                row["rating"].as_f64().unwrap_or(f64::NAN),
            )
        })
        .collect();
    assert_eq!(
        summary,
        vec![
            (bot_id.as_str(), "bot", 1500.0),
            (OTHER, "person", 1200.0),
            ("person-placing", "person", 900.0),
        ],
        "highest rating first: {whole}"
    );
    assert_eq!(
        rows[0]["bot"],
        json!({ "strength": 3, "persona": "polite", "retired": false })
    );
    assert_eq!(rows[1]["bot"], Value::Null);
    assert_eq!(rows[1]["username"], "Admin2");
    assert_eq!(
        rows[1]["rank"]["tier"], "jlorious",
        "the person's own place: {whole}"
    );
    assert_eq!(rows[0]["rank"]["tier"], "golden", "{whole}");
    assert_eq!(rows[2]["rank"]["tier"], "raisin", "{whole}");
    assert_eq!(rows[1]["games"], json!(RANK_PLACEMENT_GAMES));

    // The All Random ladder is its own: no row there.
    let random = expect(
        &app,
        "GET",
        "/api/admin/leaderboard?ladder=random",
        &tokens.dev,
        Value::Null,
        200,
    )
    .await;
    assert_eq!(random["rows"], json!([]));

    // A non-dev never reads it.
    expect(
        &app,
        "GET",
        "/api/admin/leaderboard",
        &tokens.other,
        Value::Null,
        403,
    )
    .await;
}

#[tokio::test]
async fn r3074002_r1452_public_routes_carry_no_kind_and_no_rating_even_for_a_dev() {
    let (app, tokens) = admin_app(false).await;
    mythic(&app, OTHER, 1200.0).await;
    mythic(&app, DEV, 1100.0).await;
    bot(&app, "bot-3074", 1500.0).await;
    expect(
        &app,
        "PUT",
        "/api/admin/rating",
        &tokens.dev,
        json!({ "ladder": "constructed", "rating": 1300 }),
        200,
    )
    .await;
    for token in [&tokens.dev, &tokens.other] {
        for path in [
            "/api/leaderboard",
            "/api/leaderboard?ladder=random",
            "/api/ranked",
            "/api/ranked?ladder=random",
            "/api/auth/me",
        ] {
            let body = expect(&app, "GET", path, token, Value::Null, 200).await;
            let text = body.to_string();
            for tell in [
                "\"kind\"",
                "\"bot\"",
                "\"handSet\"",
                "\"rating\"",
                "\"deviation\"",
                "\"persona\"",
                "\"strength\"",
            ] {
                assert!(!text.contains(tell), "{tell} in {path}: {text}");
            }
        }
    }
}

// News posts (R3074003)

#[tokio::test]
async fn r3074003_a_post_stands_on_the_news_with_no_reward_and_its_claim_pays_nothing() {
    let (app, tokens) = admin_app(false).await;
    let posted = expect(
        &app,
        "POST",
        "/api/admin/news",
        &tokens.dev,
        json!({ "title": "  Maintenance tonight ", "summary": "The server restarts at midnight UTC." }),
        201,
    )
    .await;
    let post = &posted["post"];
    let id = post["id"].as_str().expect("an id").to_string();
    assert!(id.starts_with("post-"), "{post}");
    assert_eq!(post["title"], "Maintenance tonight", "trimmed: {post}");
    assert_eq!(post["postedBy"], "Admin1");
    assert_eq!(post["takenDownAt"], Value::Null);

    let news = expect(&app, "GET", "/api/news", &tokens.other, Value::Null, 200).await;
    let entry = news["news"]
        .as_array()
        .expect("news")
        .iter()
        .find(|entry| entry["id"] == id.as_str())
        .unwrap_or_else(|| panic!("the post in {news}"))
        .clone();
    assert_eq!(entry["kind"], "alert");
    assert_eq!(entry["reward"], Value::Null);
    assert_eq!(entry["claimed"], false);
    assert_eq!(entry["title"], "Maintenance tonight");

    // Its claim marks it read and pays nothing; a second is refused.
    let ledger_before = {
        let mut tx = app.db.begin(Some(OTHER)).await.expect("begin");
        let ledger = tx.economy_ledger_get(OTHER).await.expect("the ledger");
        tx.commit().await.expect("commit");
        ledger
    };
    let claimed = expect(
        &app,
        "POST",
        &format!("/api/news/{id}/claim"),
        &tokens.other,
        json!({}),
        200,
    )
    .await;
    let row = claimed["news"]
        .as_array()
        .expect("news")
        .iter()
        .find(|entry| entry["id"] == id.as_str())
        .expect("the post")
        .clone();
    assert_eq!(row["claimed"], true);
    let ledger_after = {
        let mut tx = app.db.begin(Some(OTHER)).await.expect("begin");
        let ledger = tx.economy_ledger_get(OTHER).await.expect("the ledger");
        tx.commit().await.expect("commit");
        ledger
    };
    assert_eq!(ledger_after, ledger_before, "a post pays nothing");
    expect(
        &app,
        "POST",
        &format!("/api/news/{id}/claim"),
        &tokens.other,
        json!({}),
        409,
    )
    .await;

    // Taken down, it leaves the news, and its claim is refused as for no post at all.
    let down = expect(
        &app,
        "DELETE",
        &format!("/api/admin/news/{id}"),
        &tokens.dev,
        Value::Null,
        200,
    )
    .await;
    assert!(down["post"]["takenDownAt"].as_i64().is_some(), "{down}");
    let news = expect(&app, "GET", "/api/news", &tokens.dev, Value::Null, 200).await;
    assert!(
        !news.to_string().contains(&id),
        "a post taken down is gone: {news}"
    );
    expect(
        &app,
        "POST",
        &format!("/api/news/{id}/claim"),
        &tokens.dev,
        json!({}),
        404,
    )
    .await;
    expect(
        &app,
        "DELETE",
        &format!("/api/admin/news/{id}"),
        &tokens.dev,
        Value::Null,
        409,
    )
    .await;
    expect(
        &app,
        "DELETE",
        "/api/admin/news/post-nothing",
        &tokens.dev,
        Value::Null,
        404,
    )
    .await;

    // The dashboard lists it, taken down.
    let listed = expect(&app, "GET", "/api/admin/news", &tokens.dev, Value::Null, 200).await;
    assert_eq!(listed["posts"][0]["id"], id.as_str());
    assert!(listed["posts"][0]["takenDownAt"].as_i64().is_some());
}

#[tokio::test]
async fn r3074003_a_post_is_checked_as_news_json_is_and_a_refusal_writes_nothing() {
    let (app, tokens) = admin_app(false).await;
    for body in [
        json!({ "title": "", "summary": "Fine." }),
        json!({ "title": "   ", "summary": "Fine." }),
        json!({ "title": "T", "summary": "No end" }),
        json!({ "title": "T", "summary": "One. Two. Three." }),
        json!({ "title": "T", "summary": "Line one.\nLine two." }),
        json!({ "title": "T" }),
        json!({ "summary": "Fine." }),
        json!({ "title": 5, "summary": "Fine." }),
    ] {
        let answer = expect(&app, "POST", "/api/admin/news", &tokens.dev, body.clone(), 400).await;
        assert_eq!(answer["error"]["code"], "bad_request", "{body}: {answer}");
    }
    let tables = fake_lock(&app).await.tables.admin.clone();
    assert!(tables.posts.is_empty() && tables.actions.is_empty(), "{tables:?}");
}

#[tokio::test]
async fn r3074003_a_post_expires_with_the_news_days() {
    let (app, tokens) = admin_app(false).await;
    let old = NewsPost {
        id: "post-old".to_string(),
        title: "Old".to_string(),
        summary: "Long ago.".to_string(),
        posted_at: now_ms() - NEWS_TTL_DAYS * MS_PER_DAY - 1,
        posted_by: Some(DEV.to_string()),
        taken_down_at: None,
    };
    q!(app, admin_posts_create(&old));
    let news = expect(&app, "GET", "/api/news", &tokens.other, Value::Null, 200).await;
    assert!(!news.to_string().contains("post-old"), "{news}");
    let answer = expect(
        &app,
        "POST",
        "/api/news/post-old/claim",
        &tokens.other,
        json!({}),
        409,
    )
    .await;
    assert_eq!(answer["error"]["message"], "That news has expired.");
}
