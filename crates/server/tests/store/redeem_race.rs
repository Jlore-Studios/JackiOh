//! docs/polish/5-sign-in.md B14: concurrent redemptions, against both stores.
//!
//! §9.4 step 6 says "increment uses and set the account active, atomically", and `Tx::redeem` is
//! the whole six-step transaction. `contract.rs` proves each step one call at a time; this proves
//! the transaction holds when calls overlap:
//!
//!  - one pending profile redeeming several good codes at once is activated once and spends exactly
//!    one code (the in-memory store once let both calls pass step 1 across an `await`, so one account
//!    used up two codes);
//!  - several profiles racing for a code's last use get exactly one `ok`;
//!  - at §9.4 step 3's per-IP boundary, concurrent redemptions from DIFFERENT profiles at one
//!    address get exactly one more lookup (Postgres once let 7 to 9 through: the count was not held
//!    across profiles until migration 0006's advisory lock);
//!  - a redemption that overlaps another request's transaction stays committed when that one rolls
//!    back (the in-memory stores once shared one snapshot between them).
//!
//! ONE suite, TWO stores, like `contract.rs` (the port of `apps/server/test/db/redeem-race.ts` and
//! its two runners): every case runs against `Db::Fake` always, and against `Db::Pg` too when
//! `DATABASE_URL` is set (`pnpm test:db`, `tests/db/run.sh`, which runs the cases one at a time over
//! one database).
//!
//! Every call is started before any is awaited (`join_all`, TS's `Promise.all`), so the stores see
//! them overlap. The fake serialises them on its one lock (`FakeTx` holds it from `begin` to
//! commit or drop); Postgres runs them on separate connections under `app.redeem_invite_code`'s
//! row and advisory locks.

use std::future::Future;
use std::pin::Pin;
use std::task::Poll;
use std::time::Duration;

use jackioh_server::config::CODE_ATTEMPTS_PER_IP_PER_HOUR;
use jackioh_server::db::store::Db;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sqlx::Row;
use sqlx::postgres::PgPool;

use crate::support::deps::test_app;

/// One transaction around one store call, as TS's store gave every method called outside
/// `store.tx`: begin, the call, commit. Answers the call's value, or the first error's text; an
/// error drops the transaction, which rolls it back.
macro_rules! once {
    ($db:expr, $sub:expr, |$tx:ident| $call:expr) => {
        async {
            let mut $tx = $db.begin($sub).await.map_err(|error| error.to_string())?;
            let value = $call.await.map_err(|error| error.to_string())?;
            $tx.commit().await.map_err(|error| error.to_string())?;
            Ok::<_, String>(value)
        }
        .await
    };
}

// ---------------------------------------------------------------------------
// The two ends of the contract (`test/db/harness.ts`, the parts this suite uses)
// ---------------------------------------------------------------------------

/// The fixture catalog the Postgres end seeds: 64 playable ids and two tokens (§9.4 L1–L3).
const CATALOG_VERSION: &str = "core-1";
const PLAYABLE_COUNT: usize = 64;

fn playable_ids() -> Vec<String> {
    (1..=PLAYABLE_COUNT).map(|i| format!("core-{i:03}")).collect()
}

fn token_ids() -> Vec<String> {
    vec!["core-001.1".to_string(), "core-002.1".to_string()]
}

/// Every table the migrations create, children first. `cards` is seeded once and kept. The loadout
/// tables are no longer written by anything (R254) but are emptied all the same, so a test that
/// wrote one by hand leaves nothing behind.
const TRUNCATE: &str = "truncate
  public.game_records, public.tutorial_progress, public.player_settings,
  public.rated_games, public.bot_ratings, public.season_ranks, public.seasons,
  public.series, public.results, public.match_actions, public.tickets, public.matches,
  public.trios, public.decks,
  public.loadout_deck_cards, public.loadout_decks, public.loadouts,
  public.collection_grants, public.collection,
  public.code_attempts, public.invite_codes, public.profiles, auth.users
  restart identity cascade";

/// `app.settings.redemption_enabled` — migration 0001 seeds it `true` and
/// `app.redeem_invite_code` reads it before the lookup, answering `circuit_open` when it is false.
const REDEMPTION_ENABLED_SQL: &str =
    "update app.settings set value = to_jsonb($1::boolean) where key = 'redemption_enabled'";

fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|url| !url.is_empty())
}

async fn seed_cards(admin: &PgPool) {
    sqlx::query(
        "insert into public.cards (id, card_index, name, set_id, type, tags, rarity, token, cost, catalog_version)
     select c.id, c.ord::text, 'Fixture ' || c.id, 'Core', 'Unit', '{}'::text[],
            case when c.token then 'Token' else 'Common' end, c.token, '1'::jsonb, $3::text
       from (
         select t.id, false as token, t.ord from unnest($1::text[]) with ordinality as t(id, ord)
         union all
         select t.id, true, 1000 + t.ord from unnest($2::text[]) with ordinality as t(id, ord)
       ) as c
     on conflict (id) do nothing",
    )
    .bind(playable_ids())
    .bind(token_ids())
    .bind(CATALOG_VERSION)
    .execute(admin)
    .await
    .expect("seed the fixture cards");
}

struct Harness {
    /// Appears in the assertion messages, so a failure says which store broke.
    name: &'static str,
    db: Db,
    /// The Postgres end's raw superuser connection, for the setup the store deliberately cannot do.
    admin: Option<PgPool>,
}

/// The same `Db` value again: both variants hold shared handles (a pool, an `Arc`).
fn clone_db(db: &Db) -> Db {
    match db {
        Db::Pg(pool) => Db::Pg(pool.clone()),
        Db::Fake(data) => Db::Fake(data.clone()),
    }
}

/// The in-memory end: the fake the server's own tests run on. Each case builds a fresh one, which
/// is the reset.
async fn memory_harness() -> Harness {
    let app = test_app().await;
    Harness { name: "fake store (in memory)", db: clone_db(&app.db), admin: None }
}

/// The Postgres end, emptied: TS's `postgresHarness()` and its `reset()` before the case.
async fn postgres_harness(url: &str) -> Harness {
    let admin = PgPool::connect(url).await.expect("the admin connection");
    sqlx::raw_sql(TRUNCATE).execute(&admin).await.expect("truncate");
    seed_cards(&admin).await;
    // `app.settings` is not truncated (migration 0001 seeds it once), so the redemption switch
    // is put back by hand rather than left flipped for whatever test runs next.
    sqlx::query(REDEMPTION_ENABLED_SQL).bind(true).execute(&admin).await.expect("redemption on");
    let pool = PgPool::connect(url).await.expect("the store's pool");
    Harness { name: "postgres store (src/db/pg.rs)", db: Db::Pg(pool), admin: Some(admin) }
}

/// The fake always; Postgres as well when there is one.
async fn harnesses() -> Vec<Harness> {
    let mut all = vec![memory_harness().await];
    if let Some(url) = database_url() {
        all.push(postgres_harness(&url).await);
    }
    all
}

impl Harness {
    /// Provisions the managed-auth identity a profile needs and returns its user id (§9.4:
    /// "managed auth provider"). In Postgres that is an `auth.users` row, which `profiles.id`
    /// references.
    async fn new_user_id(&self, email: &str) -> String {
        let Some(admin) = &self.admin else { return uuid() };
        let rows = sqlx::query("insert into auth.users (email, email_confirmed_at) values ($1, now()) returning id")
            .bind(email)
            .fetch_all(admin)
            .await
            .expect("insert the auth user");
        let id: uuid::Uuid = rows.first().expect("auth.users insert returned no id").try_get("id").expect("id");
        // Migration 0001's `on_auth_user_created` trigger has just made the pending profile row.
        // The contract exercises `profiles.create` itself — the path `resolve_caller` takes for a
        // user whose row is missing — so the trigger's row is removed here and asserted separately
        // in `postgres.rs`, where it belongs.
        sqlx::query("delete from public.profiles where id = $1")
            .bind(id)
            .execute(admin)
            .await
            .expect("remove the trigger's profile");
        id.to_string()
    }

    async fn close(self) {
        if let Db::Pg(pool) = &self.db {
            pool.close().await;
        }
        if let Some(admin) = &self.admin {
            admin.close().await;
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// uuids, because the Postgres store types every id column as `uuid`.
fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

fn must<T>(value: Option<T>, what: &str) -> T {
    value.unwrap_or_else(|| panic!("expected {what}"))
}

/// A row from the JSON TS wrote it as; the store method's parameter names the type.
fn de<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

fn js<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("a store row serialises")
}

/// `harness.now()`: the wall clock in epoch ms.
fn now_ms() -> i64 {
    (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
}

/// Every future started before any is awaited, as `Promise.all` starts them; the answers in order.
async fn join_all<F: Future>(futures: Vec<F>) -> Vec<F::Output> {
    let mut futures: Vec<Pin<Box<F>>> = futures.into_iter().map(Box::pin).collect();
    let mut outputs: Vec<Option<F::Output>> = futures.iter().map(|_| None).collect();
    std::future::poll_fn(|cx| {
        let mut pending = false;
        for (future, output) in futures.iter_mut().zip(outputs.iter_mut()) {
            if output.is_none() {
                match future.as_mut().poll(cx) {
                    Poll::Ready(value) => *output = Some(value),
                    Poll::Pending => pending = true,
                }
            }
        }
        if pending { Poll::Pending } else { Poll::Ready(()) }
    })
    .await;
    outputs.into_iter().map(|output| output.expect("every future finished")).collect()
}

/// §9.4: pending until a code is redeemed, with a verified email (the harness default). Answers
/// the profile's id.
async fn pending_profile(h: &Harness) -> String {
    let email = format!("{}@example.test", uuid());
    let user_id = h.new_user_id(&email).await;
    let existing = once!(h.db, Some(user_id.as_str()), |tx| tx.profiles_get_by_user_id(&user_id))
        .expect("profiles.getByUserId");
    if let Some(profile) = existing {
        return js(&profile)["id"].as_str().expect("a profile id").to_string();
    }
    let input = json!({ "userId": user_id, "email": email, "rating": 1000, "at": now_ms() });
    let created = once!(h.db, Some(user_id.as_str()), |tx| tx.profiles_create(&de(input.clone())))
        .expect("profiles.create");
    js(&created)["id"].as_str().expect("a profile id").to_string()
}

/// A code with `max_uses` uses; answers its id and its hash.
async fn mint(h: &Harness, max_uses: i64) -> (String, String) {
    let row = json!({
        "id": uuid(),
        "codeHash": format!("hash-{}", uuid()),
        "maxUses": max_uses,
        "uses": 0,
        "revoked": false,
        "expiresAt": null,
        "createdAt": now_ms(),
    });
    once!(h.db, None, |tx| tx.codes_insert(&de(row.clone()))).expect("codes.insert");
    (
        row["id"].as_str().unwrap_or_default().to_string(),
        row["codeHash"].as_str().unwrap_or_default().to_string(),
    )
}

async fn uses_of(h: &Harness, code_hash: &str) -> i64 {
    let code = once!(h.db, None, |tx| tx.codes_find_by_hash(code_hash)).expect("codes.findByHash");
    js(&must(code, "the code"))["uses"].as_i64().expect("uses")
}

async fn status_of(h: &Harness, profile_id: &str) -> String {
    let profile = once!(h.db, Some(profile_id), |tx| tx.profiles_get_by_id(profile_id)).expect("profiles.getById");
    js(&must(profile, "the profile"))["status"].as_str().unwrap_or_default().to_string()
}

/// `Tx::redeem` in its own transaction: what it answers (`RedeemResult`), as its string.
async fn redeem(h: &Harness, profile_id: &str, code_hash: &str, ip_hash: String) -> String {
    let input = json!({ "profileId": profile_id, "codeHash": code_hash, "ipHash": ip_hash });
    let result = once!(h.db, Some(profile_id), |tx| tx.redeem(&de(input.clone()))).expect("redeem");
    js(&result).as_str().unwrap_or_default().to_string()
}

fn count(results: &[String], wanted: &str) -> usize {
    results.iter().filter(|result| *result == wanted).count()
}

fn sorted(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values
}

// ---------------------------------------------------------------------------
// B14 concurrent redemptions (SPEC §9.4 step 6)
// ---------------------------------------------------------------------------

mod b14_concurrent_redemptions {
    use super::*;

    /// One pending profile redeems `codes` good codes at once.
    async fn one_profile_racing_its_own_codes(h: &Harness, codes: usize, ip: &str) {
        let profile = pending_profile(h).await;
        let mut minted = Vec::new();
        for _ in 0..codes {
            minted.push(mint(h, 1).await);
        }

        let results = join_all(
            minted
                .iter()
                .enumerate()
                .map(|(index, (_, code_hash))| redeem(h, &profile, code_hash, format!("{ip}-{index}")))
                .collect(),
        )
        .await;

        assert_eq!(count(&results, "ok"), 1, "{}: {results:?}", h.name);
        assert_eq!(status_of(h, &profile).await, "active", "{}", h.name);
        let mut spent = 0;
        for (_, code_hash) in &minted {
            spent += uses_of(h, code_hash).await;
        }
        assert_eq!(spent, 1, "{}", h.name);
    }

    #[tokio::test]
    async fn b14_one_pending_profile_redeeming_two_good_codes_at_once_is_activated_once_and_spends_one_code() {
        for h in harnesses().await {
            one_profile_racing_its_own_codes(&h, 2, "ip-race").await;
            h.close().await;
        }
    }

    #[tokio::test]
    async fn b14_one_pending_profile_redeeming_four_good_codes_at_once_still_spends_exactly_one() {
        for h in harnesses().await {
            one_profile_racing_its_own_codes(&h, 4, "ip-many").await;
            h.close().await;
        }
    }

    #[tokio::test]
    async fn b14_two_profiles_racing_for_a_single_use_code_get_exactly_one_ok() {
        for h in harnesses().await {
            let profiles = vec![pending_profile(&h).await, pending_profile(&h).await];
            let (_, code_hash) = mint(&h, 1).await;

            let results = join_all(
                profiles
                    .iter()
                    .enumerate()
                    .map(|(index, profile)| redeem(&h, profile, &code_hash, format!("ip-pair-{index}")))
                    .collect(),
            )
            .await;

            assert_eq!(sorted(results.clone()), vec!["invalid_code", "ok"], "{}", h.name);
            assert_eq!(uses_of(&h, &code_hash).await, 1, "{}", h.name);
            let mut statuses = Vec::new();
            for profile in &profiles {
                statuses.push(status_of(&h, profile).await);
            }
            assert_eq!(sorted(statuses.clone()), vec!["active", "pending"], "{}", h.name);
            // The winner is the one the store said won.
            let winner = results.iter().position(|result| result == "ok").expect("one ok");
            assert_eq!(statuses[winner], "active", "{}", h.name);
            h.close().await;
        }
    }

    #[tokio::test]
    async fn b14_two_profiles_racing_for_the_last_use_of_a_multi_use_code_get_exactly_one_ok() {
        for h in harnesses().await {
            let (code_id, code_hash) = mint(&h, 2).await;
            let claimed = once!(h.db, None, |tx| tx.codes_claim(&code_id, now_ms())).expect("codes.claim");
            assert!(claimed, "{}", h.name);
            let profiles = vec![pending_profile(&h).await, pending_profile(&h).await];

            let results = join_all(
                profiles
                    .iter()
                    .enumerate()
                    .map(|(index, profile)| redeem(&h, profile, &code_hash, format!("ip-last-{index}")))
                    .collect(),
            )
            .await;

            assert_eq!(sorted(results), vec!["invalid_code", "ok"], "{}", h.name);
            assert_eq!(uses_of(&h, &code_hash).await, 2, "{}", h.name);
            let mut statuses = Vec::new();
            for profile in &profiles {
                statuses.push(status_of(&h, profile).await);
            }
            assert_eq!(sorted(statuses), vec!["active", "pending"], "{}", h.name);
            h.close().await;
        }
    }

    #[tokio::test]
    async fn b14_five_profiles_racing_for_a_single_use_code_activate_exactly_one_account() {
        for h in harnesses().await {
            let mut profiles = Vec::new();
            for _ in 0..5 {
                profiles.push(pending_profile(&h).await);
            }
            let (_, code_hash) = mint(&h, 1).await;

            let results = join_all(
                profiles
                    .iter()
                    .enumerate()
                    .map(|(index, profile)| redeem(&h, profile, &code_hash, format!("ip-crowd-{index}")))
                    .collect(),
            )
            .await;

            assert_eq!(count(&results, "ok"), 1, "{}: {results:?}", h.name);
            assert_eq!(count(&results, "invalid_code"), profiles.len() - 1, "{}: {results:?}", h.name);
            assert_eq!(uses_of(&h, &code_hash).await, 1, "{}", h.name);
            let mut active = 0;
            for profile in &profiles {
                if status_of(&h, profile).await == "active" {
                    active += 1;
                }
            }
            assert_eq!(active, 1, "{}", h.name);
            h.close().await;
        }
    }

    #[tokio::test]
    async fn b14_at_the_per_ip_boundary_concurrent_redemptions_from_different_profiles_get_exactly_one_more_lookup() {
        for h in harnesses().await {
            let ip_hash = format!("ip-boundary-{}", uuid());
            // Exactly at the limit, one account at a time: §9.4 step 3 refuses "more than" it.
            for _ in 0..CODE_ATTEMPTS_PER_IP_PER_HOUR {
                let profile = pending_profile(&h).await;
                let answer = redeem(&h, &profile, &format!("missing-{}", uuid()), ip_hash.clone()).await;
                assert_eq!(answer, "invalid_code", "{}", h.name);
            }
            let mut burst = Vec::new();
            for _ in 0..10 {
                burst.push(pending_profile(&h).await);
            }
            let missing: Vec<String> = burst.iter().map(|_| format!("missing-{}", uuid())).collect();

            let results = join_all(
                burst
                    .iter()
                    .zip(missing.iter())
                    .map(|(profile, code_hash)| redeem(&h, profile, code_hash, ip_hash.clone()))
                    .collect(),
            )
            .await;

            // The first to get through sees the limit (not more than it) and reaches the lookup; every
            // other one then sees one more than the limit.
            assert_eq!(count(&results, "invalid_code"), 1, "{}: {}", h.name, results.join(","));
            assert_eq!(count(&results, "rate_limited_ip"), burst.len() - 1, "{}: {}", h.name, results.join(","));
            h.close().await;
        }
    }

    #[tokio::test]
    async fn b14_a_redemption_that_overlaps_another_transactions_rollback_stays_committed() {
        for h in harnesses().await {
            let profile = pending_profile(&h).await;
            let (_, code_hash) = mint(&h, 1).await;

            // Another request's transaction (a loadout save, a result) is still running, and then fails.
            let other = async {
                let tx = h.db.begin(None).await.map_err(|error| error.to_string())?;
                tokio::time::sleep(Duration::from_millis(20)).await;
                // The other request's write failed: its transaction is dropped uncommitted, which
                // rolls it back.
                drop(tx);
                Err::<&str, String>("the other request's write failed".to_string())
            };
            let redeemed = redeem(&h, &profile, &code_hash, "ip-overlap".to_string());
            let (other, redeemed) = tokio::join!(other, redeemed);
            let other = match other {
                Ok(_) => "committed".to_string(),
                Err(error) => error,
            };
            assert_eq!(other, "the other request's write failed", "{}", h.name);

            assert_eq!(redeemed, "ok", "{}", h.name);
            assert_eq!(status_of(&h, &profile).await, "active", "{}", h.name);
            assert_eq!(uses_of(&h, &code_hash).await, 1, "{}", h.name);
            h.close().await;
        }
    }

    #[tokio::test]
    async fn b14_serves_the_next_redemption_normally_once_a_race_has_settled() {
        for h in harnesses().await {
            let racer = pending_profile(&h).await;
            let raced = [mint(&h, 1).await, mint(&h, 1).await];
            join_all(
                raced
                    .iter()
                    .enumerate()
                    .map(|(index, (_, code_hash))| redeem(&h, &racer, code_hash, format!("ip-settle-{index}")))
                    .collect(),
            )
            .await;

            let later = pending_profile(&h).await;
            let (_, fresh) = mint(&h, 1).await;
            assert_eq!(redeem(&h, &later, &fresh, "ip-later".to_string()).await, "ok", "{}", h.name);
            assert_eq!(status_of(&h, &later).await, "active", "{}", h.name);
            assert_eq!(uses_of(&h, &fresh).await, 1, "{}", h.name);
            h.close().await;
        }
    }
}
