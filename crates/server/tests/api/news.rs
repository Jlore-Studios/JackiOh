//! News (SPEC §14.13, #2875): `GET /api/news` lists the live entries newest first, each marked
//! claimed or not, and `POST /api/news/:newsId/claim` pays one entry's reward once, as `news` ledger
//! rows (R2875000–R2875003); `POST /api/news/claim-all` claims every unclaimed one in one spend
//! (R2982001). Each test posts its own entries, timed from the server's clock, so none depends on
//! the date `news.json` was written.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};

use jackioh_engine::SetName;
use jackioh_server::admin::store::NewsPost;
use jackioh_server::app::{App, now_ms};
use jackioh_server::config::{
    MS_PER_DAY, NEWS_REWARD_CARD_PACKS, NEWS_REWARD_CARD_TOKENS, NEWS_REWARD_COSMETIC_PACKS,
    NEWS_REWARD_COSMETIC_TOKENS, NEWS_REWARD_JLORE, NEWS_TTL_DAYS,
};
use jackioh_server::cosmetics::CosmeticRarity;
use jackioh_server::economy::news::{NewsEntry, NewsKind, NewsReward};
use jackioh_server::economy::roll::PackKind;
use jackioh_server::economy::store::{EconomyReason, EconomyTables, JloreReason};

use crate::support::deps::{add_user, call, call_with_key, empty_test_app};
use crate::support::store::{fake, uuid};

const PROFILE: &str = "p1";
const USER: &str = "u1";

/// An alert posted at `posted_at`, paying `reward`.
fn entry(id: &str, posted_at: i64, reward: NewsReward) -> NewsEntry {
    NewsEntry {
        id: id.to_string(),
        kind: NewsKind::Alert,
        version: None,
        title: format!("The {id} entry"),
        summary: "Something happened.".to_string(),
        posted_at,
        reward,
    }
}

/// The entries most tests post, oldest first as `news.json` is: one expired a day ago, two live
/// (`first`, then `second`), and one posted tomorrow.
fn entries(now: i64) -> Vec<NewsEntry> {
    vec![
        entry("old", now - (NEWS_TTL_DAYS + 1) * MS_PER_DAY, NewsReward::Jlore),
        entry("first", now - 2 * MS_PER_DAY, NewsReward::Jlore),
        entry("second", now - MS_PER_DAY, NewsReward::CardPack(PackKind::Global)),
        entry("future", now + MS_PER_DAY, NewsReward::Jlore),
    ]
}

/// An empty app posting `news`, its economy `live`, with the active `p1` and its Bearer token.
async fn setup_with(news: Vec<NewsEntry>, live: bool) -> (Arc<App>, String) {
    let mut app = empty_test_app().await;
    {
        let app = Arc::get_mut(&mut app).expect("the test app has no second handle yet");
        app.news = news;
        app.economy_live = live;
    }
    fake(&app)
        .lock()
        .await
        .seed_profile(json!({ "id": PROFILE, "userId": USER, "status": "active" }));
    let token = add_user(&app, USER, "p1@example.test", true);
    (app, token)
}

async fn setup() -> (Arc<App>, String) {
    setup_with(entries(now_ms()), true).await
}

fn claim_path(id: &str) -> String {
    format!("/api/news/{id}/claim")
}

async fn claim(app: &Arc<App>, token: &str, id: &str) -> (u16, Value) {
    let (status, _, body) = call(app, "POST", &claim_path(id), Some(token), json!({})).await;
    (status, body)
}

async fn read(app: &Arc<App>, token: &str, path: &str) -> Value {
    let (status, _, body) = call(app, "GET", path, Some(token), Value::Null).await;
    assert_eq!(status, 200, "GET {path}: {body}");
    body
}

/// Every economy row the fake holds, the news claims among them.
async fn economy(app: &App) -> EconomyTables {
    fake(app).lock().await.tables.economy.clone()
}

/// The board's entries as `(id, claimed)`, in the order it lists them.
fn rows(board: &Value) -> Vec<(String, bool)> {
    board["news"]
        .as_array()
        .expect("a news list")
        .iter()
        .map(|entry| {
            (
                entry["id"].as_str().expect("an id").to_string(),
                entry["claimed"].as_bool().expect("claimed"),
            )
        })
        .collect()
}

fn owned(rows: &[(&str, bool)]) -> Vec<(String, bool)> {
    rows.iter()
        .map(|(id, claimed)| ((*id).to_string(), *claimed))
        .collect()
}

#[tokio::test]
async fn r2875001_get_lists_the_live_entries_newest_first_each_marked_claimed() {
    let (app, token) = setup().await;
    let board = read(&app, &token, "/api/news").await;
    // `old` expired a day ago and `future` is not posted yet: neither is listed.
    assert_eq!(rows(&board), owned(&[("second", false), ("first", false)]));
    assert_eq!(
        board["news"][1],
        json!({
            "id": "first",
            "kind": "alert",
            "version": null,
            "title": "The first entry",
            "summary": "Something happened.",
            "postedAt": app.news[1].posted_at,
            "reward": { "kind": "jlore", "amount": NEWS_REWARD_JLORE },
            "claimed": false,
        })
    );

    let (status, answer) = claim(&app, &token, "first").await;
    assert_eq!(status, 200, "{answer}");
    // R2875002: the claim answers the board, the entry claimed; a read says the same.
    assert_eq!(rows(&answer), owned(&[("second", false), ("first", true)]));
    assert_eq!(rows(&read(&app, &token, "/api/news").await), rows(&answer));
}

#[tokio::test(start_paused = true)]
async fn r2875001_an_entry_past_news_ttl_days_leaves_the_list_and_pays_nothing() {
    let (app, token) = setup_with(entries(now_ms())[1..3].to_vec(), true).await;
    let (status, answer) = claim(&app, &token, "first").await;
    assert_eq!(status, 200, "{answer}");

    // `first` was posted two days ago: past its days, claimed or not, it is gone, and `second`,
    // posted a day ago, with it once its own day passes.
    tokio::time::advance(Duration::from_millis(
        u64::try_from((NEWS_TTL_DAYS - 2) * MS_PER_DAY).expect("a wait"),
    ))
    .await;
    assert_eq!(
        rows(&read(&app, &token, "/api/news").await),
        owned(&[("second", false)])
    );
    let before = economy(&app).await;
    let (status, answer) = claim(&app, &token, "first").await;
    assert_eq!(status, 409, "{answer}");
    assert_eq!(answer["error"]["message"], json!("That news has expired."));

    tokio::time::advance(Duration::from_millis(u64::try_from(MS_PER_DAY).expect("a wait"))).await;
    assert_eq!(rows(&read(&app, &token, "/api/news").await), owned(&[]));
    let (status, answer) = claim(&app, &token, "second").await;
    assert_eq!(status, 409, "{answer}");
    assert_eq!(economy(&app).await, before, "an expired entry pays nothing");
}

#[tokio::test]
async fn r2875000_a_claim_pays_news_rows_once() {
    let (app, token) = setup().await;
    let wallet = read(&app, &token, "/api/wallet").await["jlore"]
        .as_i64()
        .expect("jlore");

    let (status, answer) = claim(&app, &token, "first").await;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(
        read(&app, &token, "/api/wallet").await["jlore"],
        json!(wallet + NEWS_REWARD_JLORE)
    );
    {
        let tables = economy(&app).await;
        let news: Vec<_> = tables
            .jlore_grants
            .iter()
            .filter(|grant| grant.reason == JloreReason::News)
            .collect();
        assert_eq!(news.len(), 1, "{news:?}");
        assert_eq!(news[0].profile_id, PROFILE);
        assert_eq!(news[0].delta, NEWS_REWARD_JLORE);
        assert_eq!(news[0].reference.as_deref(), Some("first"));
        assert!(!news[0].dev);
        assert_eq!(tables.news_claims.len(), 1);
        assert_eq!(tables.news_claims[0].news_id, "first");
    }

    let before = economy(&app).await;
    let (status, again) = claim(&app, &token, "first").await;
    assert_eq!(status, 409, "{again}");
    assert_eq!(
        again["error"]["message"],
        json!("That reward is already claimed.")
    );
    assert_eq!(economy(&app).await, before, "a second claim pays nothing");
}

#[tokio::test]
async fn r2875000_the_same_key_replays_and_another_entry_with_it_is_422() {
    let (app, token) = setup().await;
    let key = uuid(2875001);
    let send = |id: &'static str| {
        let (app, token, key) = (Arc::clone(&app), token.clone(), key.clone());
        async move { call_with_key(&app, "POST", &claim_path(id), Some(&token), json!({}), &key).await }
    };

    let (status, _, first) = send("first").await;
    assert_eq!(status, 200, "{first}");
    let after = economy(&app).await;
    // Without its key the retry would be 409 "already claimed": with it, it answers as it did.
    let (status, _, again) = send("first").await;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again, first);
    assert_eq!(economy(&app).await, after, "the replay pays nothing more");

    // The hash covers the entry: the key sent to claim another one is another request.
    let (status, _, other) = send("second").await;
    assert_eq!(status, 422, "{other}");
    assert_eq!(other["error"]["code"], json!("idempotency_key_reused"));
    assert_eq!(economy(&app).await, after, "nor does the key reused");
}

#[tokio::test]
async fn r2875001_an_expired_unposted_or_unknown_entry_pays_nothing() {
    let (app, token) = setup().await;
    let before = economy(&app).await;
    for (id, status, message) in [
        ("old", 409, "That news has expired."),
        ("future", 404, "That news isn't posted."),
        ("no-such-entry", 404, "That news isn't posted."),
    ] {
        let (answered, body) = claim(&app, &token, id).await;
        assert_eq!(answered, status, "{id}: {body}");
        assert_eq!(body["error"]["message"], json!(message), "{id}");
    }
    assert_eq!(economy(&app).await, before);
}

#[tokio::test]
async fn r2875000_with_the_economy_off_nothing_is_claimable() {
    let (app, token) = setup_with(entries(now_ms()), false).await;
    let before = economy(&app).await;
    let (status, body) = claim(&app, &token, "first").await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(
        body["error"]["message"],
        json!("News rewards can't be claimed yet.")
    );
    assert_eq!(economy(&app).await, before);
}

#[tokio::test]
async fn r2875003_each_reward_kind_lands_where_its_section_keeps_it() {
    let at = now_ms() - MS_PER_DAY;
    let rewards = [
        ("jlore", NewsReward::Jlore),
        ("global-pack", NewsReward::CardPack(PackKind::Global)),
        (
            "set-pack",
            NewsReward::CardPack(PackKind::Set(SetName::Stinkyscape)),
        ),
        ("card-token", NewsReward::CardToken),
        ("epic-token", NewsReward::CosmeticToken(CosmeticRarity::Epic)),
        ("cosmetic-pack", NewsReward::CosmeticPack),
    ];
    let news = rewards
        .iter()
        .map(|(id, reward)| entry(id, at, *reward))
        .collect();
    let (app, token) = setup_with(news, true).await;
    let wallet = read(&app, &token, "/api/wallet").await["jlore"]
        .as_i64()
        .expect("jlore");
    for (id, _) in rewards {
        let (status, body) = claim(&app, &token, id).await;
        assert_eq!(status, 200, "{id}: {body}");
    }

    // §14.1: Jlore in the wallet.
    assert_eq!(
        read(&app, &token, "/api/wallet").await["jlore"],
        json!(wallet + NEWS_REWARD_JLORE)
    );
    // §14.3: unopened card packs, the global one and a set's.
    let unopened = read(&app, &token, "/api/packs").await["unopened"].clone();
    for pack in ["global", "Stinkyscape"] {
        assert!(
            unopened
                .as_array()
                .expect("the unopened packs")
                .contains(&json!({ "pack": pack, "count": NEWS_REWARD_CARD_PACKS })),
            "{pack} in {unopened}"
        );
    }
    // §14.5, §14.6: the tokens, an unopened cosmetic pack among them.
    let tokens = read(&app, &token, "/api/tokens").await["tokens"].clone();
    assert_eq!(tokens["card"], json!(NEWS_REWARD_CARD_TOKENS), "{tokens}");
    assert_eq!(
        tokens["cosmeticEpic"],
        json!(NEWS_REWARD_COSMETIC_TOKENS),
        "{tokens}"
    );
    assert_eq!(tokens["cosmeticRare"], json!(0), "{tokens}");
    assert_eq!(
        tokens["cosmeticPack"],
        json!(NEWS_REWARD_COSMETIC_PACKS),
        "{tokens}"
    );
    assert_eq!(
        read(&app, &token, "/api/cosmetic-packs").await["unopened"],
        json!(NEWS_REWARD_COSMETIC_PACKS)
    );

    // §14.11: each paid with the reason `news`, its `ref` the entry.
    let tables = economy(&app).await;
    let paid: Vec<(&str, bool)> = tables
        .jlore_grants
        .iter()
        .map(|grant| {
            (
                grant.reference.as_deref().unwrap_or(""),
                grant.reason == JloreReason::News,
            )
        })
        .chain(tables.inventory_grants.iter().map(|grant| {
            (
                grant.reference.as_deref().unwrap_or(""),
                grant.reason == EconomyReason::News,
            )
        }))
        .collect();
    assert_eq!(
        paid,
        [
            ("jlore", true),
            ("global-pack", true),
            ("set-pack", true),
            ("card-token", true),
            ("epic-token", true),
            ("cosmetic-pack", true),
        ]
    );
}

#[tokio::test]
async fn r2875002_a_pending_account_is_refused() {
    let (app, _) = setup().await;
    fake(&app)
        .lock()
        .await
        .seed_profile(json!({ "id": "p2", "userId": "u2", "status": "pending" }));
    let pending = add_user(&app, "u2", "p2@example.test", true);
    let before = economy(&app).await;
    for (method, path) in [("GET", "/api/news".to_string()), ("POST", claim_path("first"))] {
        let (status, _, body) = call(&app, method, &path, Some(&pending), json!({})).await;
        assert_eq!(status, 403, "{method} {path}: {body}");
    }
    assert_eq!(economy(&app).await, before);
}

/// R2982001: Claim all's route.
const CLAIM_ALL: &str = "/api/news/claim-all";

async fn claim_all(app: &Arc<App>, token: &str) -> (u16, Value) {
    let (status, _, body) = call(app, "POST", CLAIM_ALL, Some(token), json!({})).await;
    (status, body)
}

/// The economy store's calls `run` makes, in order: one per statement or so in Postgres.
async fn economy_calls<F: std::future::Future<Output = (u16, Value)>>(
    app: &App,
    run: impl FnOnce() -> F,
) -> (u16, Value, Vec<String>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let seen = Arc::clone(&calls);
    fake(app).lock().await.on_call = Some(Arc::new(move |method: &str| {
        if method.starts_with("economy.") {
            seen.lock().expect("the calls").push(method.to_string());
        }
        Ok(())
    }));
    let (status, body) = run().await;
    fake(app).lock().await.on_call = None;
    let calls = calls.lock().expect("the calls").clone();
    (status, body, calls)
}

/// `count` live entries posted a day ago, of every reward kind in turn.
fn many(count: usize) -> Vec<NewsEntry> {
    let at = now_ms() - MS_PER_DAY;
    let kinds = [
        NewsReward::Jlore,
        NewsReward::CardPack(PackKind::Global),
        NewsReward::CardToken,
        NewsReward::CosmeticToken(CosmeticRarity::Epic),
        NewsReward::CosmeticPack,
    ];
    (0..count)
        .map(|index| entry(&format!("entry-{index}"), at, kinds[index % kinds.len()]))
        .collect()
}

#[tokio::test]
async fn r2982001_claim_all_claims_every_live_unclaimed_entry_once_each_with_its_own_rows() {
    let now = now_ms();
    let mut news = entries(now);
    news.insert(
        3,
        entry(
            "third",
            now - MS_PER_DAY / 2,
            NewsReward::CosmeticToken(CosmeticRarity::Epic),
        ),
    );
    let (app, token) = setup_with(news, true).await;
    let wallet = read(&app, &token, "/api/wallet").await["jlore"]
        .as_i64()
        .expect("jlore");
    // One claimed already, through its own route: Claim all leaves it as it is.
    let (status, body) = claim(&app, &token, "first").await;
    assert_eq!(status, 200, "{body}");

    let (status, answer) = claim_all(&app, &token).await;
    assert_eq!(status, 200, "{answer}");
    // The board, every live entry claimed; `old` expired and `future` not posted, so neither is.
    assert_eq!(
        rows(&answer),
        owned(&[("third", true), ("second", true), ("first", true)])
    );
    assert_eq!(rows(&read(&app, &token, "/api/news").await), rows(&answer));
    // `first`'s Jlore was paid by its own claim and not again.
    assert_eq!(
        read(&app, &token, "/api/wallet").await["jlore"],
        json!(wallet + NEWS_REWARD_JLORE)
    );
    let tokens = read(&app, &token, "/api/tokens").await["tokens"].clone();
    assert_eq!(
        tokens["cosmeticEpic"],
        json!(NEWS_REWARD_COSMETIC_TOKENS),
        "{tokens}"
    );
    let tables = economy(&app).await;
    let paid: Vec<(&str, EconomyReason)> = tables
        .inventory_grants
        .iter()
        .map(|grant| (grant.reference.as_deref().unwrap_or(""), grant.reason))
        .collect();
    // Each entry its own ledger row, `news`, its `ref` the entry (§14.11).
    assert_eq!(
        paid,
        [("second", EconomyReason::News), ("third", EconomyReason::News)]
    );
    assert_eq!(tables.jlore_grants.len(), 1, "{:?}", tables.jlore_grants);
    let mut ids: Vec<&str> = tables
        .news_claims
        .iter()
        .map(|claim| claim.news_id.as_str())
        .collect();
    ids.sort_unstable();
    assert_eq!(ids, ["first", "second", "third"]);

    // Nothing is left to claim: a second press answers the board and writes nothing.
    let (status, again) = claim_all(&app, &token).await;
    assert_eq!(status, 200, "{again}");
    assert_eq!(rows(&again), rows(&answer));
    assert_eq!(economy(&app).await, tables, "nothing left pays nothing");
}

#[tokio::test]
async fn r2982001_claim_all_costs_the_same_store_calls_for_two_entries_as_for_twenty() {
    let mut seen = Vec::new();
    for count in [2, 20] {
        let (app, token) = setup_with(many(count), true).await;
        let (status, answer, calls) = economy_calls(&app, || claim_all(&app, &token)).await;
        assert_eq!(status, 200, "{answer}");
        assert!(rows(&answer).iter().all(|(_, claimed)| *claimed), "{answer}");
        assert_eq!(economy(&app).await.news_claims.len(), count);
        // One write of each kind for the lot, and no claim or grant of one entry.
        for batch in [
            "economy.newsClaims.putAll",
            "economy.wallets.grantAll",
            "economy.inventory.grantAll",
        ] {
            assert_eq!(
                calls.iter().filter(|call| *call == batch).count(),
                1,
                "{batch} in {calls:?}"
            );
        }
        for single in [
            "economy.newsClaims.put",
            "economy.wallets.grant",
            "economy.inventory.grant",
        ] {
            assert!(!calls.iter().any(|call| call == single), "{single} in {calls:?}");
        }
        seen.push(calls);
    }
    assert_eq!(seen[0], seen[1], "twenty entries cost what two do");
}

#[tokio::test]
async fn r2982001_claim_all_with_its_key_replays_and_the_key_is_its_own_route() {
    let (app, token) = setup().await;
    let key = uuid(2982001);
    let (status, _, first) = call_with_key(&app, "POST", CLAIM_ALL, Some(&token), json!({}), &key).await;
    assert_eq!(status, 200, "{first}");
    let after = economy(&app).await;
    assert_eq!(after.news_claims.len(), 2);

    let (status, _, again) = call_with_key(&app, "POST", CLAIM_ALL, Some(&token), json!({}), &key).await;
    assert_eq!(status, 200, "{again}");
    assert_eq!(again, first);
    assert_eq!(economy(&app).await, after, "the replay pays nothing more");

    // The key belongs to Claim all: sent to one entry's claim it is another request, 422.
    let (status, _, other) =
        call_with_key(&app, "POST", &claim_path("first"), Some(&token), json!({}), &key).await;
    assert_eq!(status, 422, "{other}");
    assert_eq!(other["error"]["code"], json!("idempotency_key_reused"));
    assert_eq!(economy(&app).await, after);
}

#[tokio::test]
async fn r2982001_claim_all_marks_a_dev_post_read_and_pays_it_nothing() {
    let now = now_ms();
    let (app, token) = setup_with(entries(now)[1..2].to_vec(), true).await;
    let mut tx = app.db.begin(None).await.expect("a store transaction");
    for post in [
        NewsPost {
            id: "post-standing".to_string(),
            title: "Maintenance".to_string(),
            summary: "The server restarts at noon.".to_string(),
            posted_at: now - MS_PER_DAY,
            posted_by: None,
            taken_down_at: None,
        },
        NewsPost {
            id: "post-down".to_string(),
            title: "Taken down".to_string(),
            summary: "Gone.".to_string(),
            posted_at: now - MS_PER_DAY,
            posted_by: None,
            taken_down_at: Some(now - 1),
        },
    ] {
        tx.admin_posts_create(&post).await.expect("a post");
    }
    tx.commit().await.expect("commit");

    let (status, answer) = claim_all(&app, &token).await;
    assert_eq!(status, 200, "{answer}");
    assert_eq!(rows(&answer), owned(&[("post-standing", true), ("first", true)]));
    let tables = economy(&app).await;
    let mut ids: Vec<&str> = tables
        .news_claims
        .iter()
        .map(|claim| claim.news_id.as_str())
        .collect();
    ids.sort_unstable();
    assert_eq!(
        ids,
        ["first", "post-standing"],
        "a post taken down is not claimed"
    );
    let refs: Vec<Option<&str>> = tables
        .jlore_grants
        .iter()
        .map(|grant| grant.reference.as_deref())
        .collect();
    assert_eq!(refs, [Some("first")], "the post pays nothing");
    assert!(tables.inventory_grants.is_empty());
}

#[tokio::test]
async fn r2982001_claim_all_is_refused_with_the_economy_off_and_to_a_pending_account() {
    let (app, token) = setup_with(entries(now_ms()), false).await;
    fake(&app)
        .lock()
        .await
        .seed_profile(json!({ "id": "p2", "userId": "u2", "status": "pending" }));
    let pending = add_user(&app, "u2", "p2@example.test", true);
    let before = economy(&app).await;
    let (status, body) = claim_all(&app, &token).await;
    assert_eq!(status, 409, "{body}");
    assert_eq!(
        body["error"]["message"],
        json!("News rewards can't be claimed yet.")
    );
    let (status, body) = claim_all(&app, &pending).await;
    assert_eq!(status, 403, "{body}");
    assert_eq!(economy(&app).await, before, "a refusal writes nothing");
}
