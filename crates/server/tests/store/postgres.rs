//! What only a real Postgres can prove about the Postgres store (`Db::Pg`, `src/db/pg.rs`) — the
//! half of the store that is not behaviour the in-memory fake could ever have
//! (`tests/store/contract.rs` covers that half against both). Everything here is about the
//! DATABASE: which role the store runs as, that `SET LOCAL` really is local, that the `app.*`
//! functions are the ones doing the work, and the schema invariants the port leans on (§9.4, §9.5).
//!
//! The port of `apps/server/test/db/postgres.spec.ts`. Run with `pnpm test:db`
//! (`tests/db/run.sh`), which stands a Postgres up and sets `DATABASE_URL`; without it every case
//! here returns at once, so `cargo test` stays hermetic. The cases share one database and truncate
//! it, so `run.sh` runs them one at a time (`--test-threads=1`).
//!
//! Each store call that TS made outside `store.tx` is its own transaction here too: `once!` begins
//! one (`Db::begin`), makes the call and commits, as TS's `session.run` did. Rows go in as the JSON
//! TS wrote and come out as JSON, so these cases read the wire shapes, not Rust field names.

use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;
use std::task::Poll;

use jackioh_server::db::store::{Db, MatchActionRow};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sqlx::Row;
use sqlx::postgres::{PgPool, PgPoolOptions};

/// One transaction around one store call, as TS's `session.run` gave every method called outside
/// `store.tx`: begin (the role switch and the subject), the call, commit. Answers the call's value,
/// or the first error's text; an error drops the transaction, which rolls it back.
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
// The fixture catalog and the database setup (`test/db/harness.ts`, the parts this file used)
// ---------------------------------------------------------------------------

/// SPEC §9.4 L2: `DECK_SIZE` is 20, and L1 says three decks, so 60 ids is the floor.
const CATALOG_VERSION: &str = "core-1";
const PLAYABLE_COUNT: usize = 64;
const DECK_SIZE: usize = 20;

fn playable_ids() -> Vec<String> {
    (1..=PLAYABLE_COUNT).map(|i| format!("core-{i:03}")).collect()
}

/// §9.4 L3: "no Token-tagged cards" — here so the R111 launch grant can be seen skipping them.
fn token_ids() -> Vec<String> {
    vec!["core-001.1".to_string(), "core-002.1".to_string()]
}

const TRUNCATE: &str = "truncate
  public.series, public.results, public.match_actions, public.tickets, public.matches,
  public.trios, public.decks,
  public.loadout_deck_cards, public.loadout_decks, public.loadouts,
  public.collection_grants, public.collection,
  public.code_attempts, public.invite_codes, public.profiles, auth.users
  restart identity cascade";

/// `DATABASE_URL`, or `None` when it is unset: then there is no Postgres to prove anything against.
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

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// An id as the `uuid` Postgres holds: sqlx binds a `String` as `text`, which `uuid = text` refuses.
fn uid(id: &str) -> uuid::Uuid {
    uuid::Uuid::parse_str(id).unwrap_or_else(|error| panic!("{id:?} is not a uuid: {error}"))
}

fn deck_of(slot: usize) -> Vec<String> {
    playable_ids()[slot * DECK_SIZE..slot * DECK_SIZE + DECK_SIZE].to_vec()
}

fn must<T>(value: Option<T>, what: &str) -> T {
    value.unwrap_or_else(|| panic!("expected {what}"))
}

/// A row from the JSON TS wrote it as; the store method's parameter names the type.
fn de<T: DeserializeOwned>(value: Value) -> T {
    serde_json::from_value(value.clone()).unwrap_or_else(|error| panic!("{error}: {value}"))
}

/// A row as the JSON it serialises to.
fn js<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("a store row serialises")
}

/// `Date.now()`.
fn now_ms() -> i64 {
    (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
}

fn assert_refused<T>(result: Result<T, String>, wanted: &str) {
    match result {
        Ok(_) => panic!("expected a refusal matching {wanted:?}, but the call succeeded"),
        Err(error) => assert!(error.contains(wanted), "expected {wanted:?} in {error:?}"),
    }
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

/// The probe that answers "what was the session, inside the store's own transaction?". A trigger on
/// `public.code_attempts` — a table `codes.logAttempt` writes directly — records `current_user` and
/// `auth.uid()` as the store's statement sees them. Nothing here is faked or asserted from the
/// outside: the row is written by Postgres, during the store's call, in the store's transaction.
const PROBE_SETUP: &str = "
create schema if not exists probe;
create table if not exists probe.session_log (
  who text not null, role_setting text, uid uuid, at timestamptz not null default now());
create or replace function probe.record_session() returns trigger
language plpgsql as $$
begin
  insert into probe.session_log (who, role_setting, uid)
  values (current_user, current_setting('role', true), auth.uid());
  return new;
end $$;
drop trigger if exists code_attempts_session_probe on public.code_attempts;
create trigger code_attempts_session_probe after insert on public.code_attempts
  for each row execute function probe.record_session();
grant usage on schema probe to service_role;
grant insert on probe.session_log to service_role;
";

const PROBE_TEARDOWN: &str = "
drop trigger if exists code_attempts_session_probe on public.code_attempts;
drop schema if exists probe cascade;
";

#[derive(Debug)]
struct ProbeRow {
    who: String,
    role_setting: Option<String>,
    uid: Option<String>,
}

/// TS's `beforeAll` and `beforeEach` together: each case here stands alone.
struct Ctx {
    url: String,
    admin: PgPool,
    pool: PgPool,
    db: Db,
}

async fn setup() -> Option<Ctx> {
    let url = database_url()?;
    // A raw superuser connection, for the setup and teardown the store deliberately cannot do.
    let admin = PgPool::connect(&url).await.expect("the admin connection");
    sqlx::raw_sql(TRUNCATE).execute(&admin).await.expect("truncate");
    seed_cards(&admin).await;
    sqlx::raw_sql(PROBE_SETUP).execute(&admin).await.expect("the session probe");
    sqlx::raw_sql("truncate probe.session_log").execute(&admin).await.expect("truncate the probe");
    // One pooled connection, so "the role and the claim do not leak" is measured on the SAME
    // physical connection rather than on a lucky second one.
    let pool = PgPoolOptions::new().max_connections(1).connect(&url).await.expect("the store's pool");
    let db = Db::Pg(pool.clone());
    Some(Ctx { url, admin, pool, db })
}

impl Ctx {
    /// TS's `afterAll`.
    async fn finish(self) {
        self.pool.close().await;
        sqlx::raw_sql(PROBE_TEARDOWN).execute(&self.admin).await.expect("drop the probe");
        self.admin.close().await;
    }

    /// A second store over its own pool of `max` connections, for calls that really run at once.
    async fn racing(&self, max: u32) -> PgPool {
        PgPoolOptions::new().max_connections(max).connect(&self.url).await.expect("the racing pool")
    }

    async fn probe_rows(&self) -> Vec<ProbeRow> {
        let rows = sqlx::query("select who, role_setting, uid from probe.session_log order by at, who")
            .fetch_all(&self.admin)
            .await
            .expect("read the probe");
        rows.iter()
            .map(|row| ProbeRow {
                who: row.try_get("who").expect("who"),
                role_setting: row.try_get("role_setting").expect("role_setting"),
                uid: row
                    .try_get::<Option<uuid::Uuid>, _>("uid")
                    .expect("uid")
                    .map(|id| id.to_string()),
            })
            .collect()
    }

    /// A signed-up, email-verified managed-auth identity, with the profile row 0001's trigger makes.
    async fn sign_up(&self) -> String {
        self.sign_up_as(&format!("{}@example.test", uuid())).await
    }

    async fn sign_up_as(&self, email: &str) -> String {
        let rows = sqlx::query("insert into auth.users (email, email_confirmed_at) values ($1, now()) returning id")
            .bind(email)
            .fetch_all(&self.admin)
            .await
            .expect("insert the auth user");
        let id = must(rows.first(), "the new auth user id")
            .try_get::<uuid::Uuid, _>("id")
            .expect("the new auth user id");
        id.to_string()
    }

    async fn active_profile(&self) -> String {
        let user_id = self.sign_up().await;
        once!(self.db, Some(user_id.as_str()), |tx| tx.profiles_set_status(&user_id, de(json!("active"))))
            .expect("profiles.setStatus");
        user_id
    }
}

fn attempt(profile_id: Option<&str>, ip_hash: &str, result: &str, reason: &str, at: i64) -> Value {
    json!({ "profileId": profile_id, "ipHash": ip_hash, "result": result, "reason": reason, "at": at })
}

async fn log_attempt(db: &Db, attempt: Value) {
    let profile = attempt["profileId"].as_str().map(str::to_string);
    once!(db, profile.as_deref(), |tx| tx.codes_log_attempt(&de(attempt.clone()))).expect("codes.logAttempt");
}

/// `store.redeemInviteCode` (`Store.redeem`): one of `app.redeem_invite_code`'s seven strings.
async fn redeem(db: &Db, profile_id: &str, code_hash: Option<&str>, ip_hash: &str) -> Value {
    let input = json!({ "profileId": profile_id, "codeHash": code_hash, "ipHash": ip_hash });
    js(&once!(db, Some(profile_id), |tx| tx.redeem(&de(input.clone()))).expect("redeem"))
}

async fn profile(db: &Db, profile_id: &str) -> Value {
    js(&must(
        once!(db, Some(profile_id), |tx| tx.profiles_get_by_id(profile_id)).expect("profiles.getById"),
        "the profile",
    ))
}

async fn code_by_hash(db: &Db, code_hash: &str) -> Value {
    js(&must(
        once!(db, None, |tx| tx.codes_find_by_hash(code_hash)).expect("codes.findByHash"),
        "the code",
    ))
}

fn clocks(now: i64) -> Value {
    json!({ "turnDeadline": null, "promptDeadline": null, "graceDeadline": { "p1": null, "p2": null }, "ceilingAt": now + 1000 })
}

fn match_row(id: &str, seed: &str, p1: &str, p2: &str, now: i64) -> Value {
    json!({
        "id": id,
        "seed": seed,
        "players": [p1, p2],
        "decks": [deck_of(0), deck_of(1)],
        "catalogVersion": CATALOG_VERSION,
        "status": "live",
        "createdAt": now,
        "finishedAt": null,
        "clocks": clocks(now),
    })
}

async fn create_match(db: &Db, row: Value) {
    once!(db, None, |tx| tx.matches_create(&de(row.clone()))).expect("matches.create");
}

// ---------------------------------------------------------------------------
// The thing that bit this project before: SET LOCAL outside a transaction
// ---------------------------------------------------------------------------

mod the_acting_role {
    use super::*;

    #[tokio::test]
    async fn runs_its_statements_as_service_role_with_auth_uid_set_to_the_profile_in_hand() {
        let Some(ctx) = setup().await else { return };
        let user_id = ctx.sign_up().await;
        log_attempt(&ctx.db, attempt(Some(user_id.as_str()), "ip-1", "rejected", "missing", now_ms())).await;

        let rows = ctx.probe_rows().await;
        assert_eq!(rows.len(), 1);
        // The connection is made as the migration owner; not one statement runs as it.
        assert_eq!(rows[0].who, "service_role");
        assert_eq!(rows[0].role_setting.as_deref(), Some("service_role"));
        // `app.current_profile_id()` is `auth.uid()`, which every RLS policy in 0002-0004 reads.
        assert_eq!(rows[0].uid.as_deref(), Some(user_id.as_str()));
        ctx.finish().await;
    }

    #[tokio::test]
    async fn does_not_leak_the_role_or_the_claim_into_the_next_transaction_on_the_same_connection() {
        let Some(ctx) = setup().await else { return };
        let user_id = ctx.sign_up().await;
        let at = now_ms();
        log_attempt(&ctx.db, attempt(Some(user_id.as_str()), "ip-1", "ok", "redeemed", at)).await;
        // Pool size is 1, so this is the same physical connection. `SET LOCAL` is undone at commit,
        // so the second transaction must start from nothing and set its own (null) subject — if the
        // GUC leaked, `uid` below would still be the first profile.
        log_attempt(&ctx.db, attempt(None, "ip-2", "rejected", "missing", at)).await;

        let rows = ctx.probe_rows().await;
        assert_eq!(rows.len(), 2);
        let uids: Vec<Option<String>> = rows.iter().map(|row| row.uid.clone()).collect();
        assert_eq!(uids, vec![Some(user_id.clone()), None]);
        assert!(rows.iter().all(|row| row.who == "service_role"));
        ctx.finish().await;
    }

    #[tokio::test]
    async fn stamps_each_statements_own_subject_inside_one_transaction() {
        let Some(ctx) = setup().await else { return };
        let (a, b) = (ctx.sign_up().await, ctx.sign_up().await);
        let at = now_ms();
        {
            let mut tx = ctx.db.begin(None).await.expect("store.tx");
            tx.codes_log_attempt(&de(attempt(Some(a.as_str()), "ip", "rejected", "x", at))).await.expect("logAttempt a");
            tx.codes_log_attempt(&de(attempt(Some(b.as_str()), "ip", "rejected", "x", at))).await.expect("logAttempt b");
            tx.commit().await.expect("commit");
        }
        let rows = ctx.probe_rows().await;
        assert_eq!(rows.len(), 2);
        // One `begin`, one role switch, two subjects: `auth.uid()` follows the profile each statement
        // is about rather than being stuck on whoever opened the transaction.
        let uids: BTreeSet<Option<String>> = rows.iter().map(|row| row.uid.clone()).collect();
        assert_eq!(uids, BTreeSet::from([Some(a.clone()), Some(b.clone())]));
        assert!(rows.iter().all(|row| row.who == "service_role"));
        ctx.finish().await;
    }

    #[tokio::test]
    async fn acts_inside_one_transaction_per_call_a_failed_write_leaves_nothing_behind() {
        let Some(ctx) = setup().await else { return };
        let user_id = ctx.sign_up().await;
        // `profiles.current_match_id` is a foreign key into `public.matches`, so this raises.
        let missing = uuid();
        let refused = once!(ctx.db, Some(user_id.as_str()), |tx| tx
            .profiles_set_in_match(&user_id, Some(missing.as_str())));
        assert!(refused.is_err(), "setInMatch to a match that does not exist must fail");
        let rows = sqlx::query("select current_match_id from public.profiles where id = $1")
            .bind(uid(&user_id))
            .fetch_all(&ctx.admin)
            .await
            .expect("read the profile");
        let current: Option<uuid::Uuid> = must(rows.first(), "the profile row")
            .try_get("current_match_id")
            .expect("current_match_id");
        assert_eq!(current, None);
        // And the connection is still usable: the rollback happened, rather than the client being
        // left in a failed transaction.
        let again = once!(ctx.db, Some(user_id.as_str()), |tx| tx.profiles_get_by_id(&user_id)).expect("getById");
        assert!(again.is_some());
        ctx.finish().await;
    }
}

// ---------------------------------------------------------------------------
// Migration 0001's signup trigger and R111's launch-grant trigger
// ---------------------------------------------------------------------------

mod triggers_the_application_never_sees {
    use super::*;

    #[tokio::test]
    async fn provisions_a_pending_profile_for_a_new_auth_user_0001_on_auth_user_created() {
        let Some(ctx) = setup().await else { return };
        let user_id = ctx.sign_up().await;
        let found = once!(ctx.db, Some(user_id.as_str()), |tx| tx.profiles_get_by_user_id(&user_id))
            .expect("profiles.getByUserId");
        let profile = js(&must(found, "the auto-provisioned profile"));
        assert_eq!(profile["status"], "pending");
        assert_eq!(profile["id"], user_id.as_str());
        assert_eq!(profile["rating"].as_f64(), Some(1000.0));
        ctx.finish().await;
    }

    #[tokio::test]
    async fn r111_the_launch_grant_writes_both_ledger_tables_through_app_grant_cards() {
        let Some(ctx) = setup().await else { return };
        let user_id = ctx.sign_up().await;
        once!(ctx.db, Some(user_id.as_str()), |tx| tx.profiles_set_status(&user_id, de(json!("active"))))
            .expect("profiles.setStatus");

        let owned = once!(ctx.db, Some(user_id.as_str()), |tx| tx.collection_get(&user_id)).expect("collection.get");
        assert_eq!(owned.len(), PLAYABLE_COUNT);

        let rows = sqlx::query(
            "select count(*)::text as n, min(reason) as reason
           from public.collection_grants where profile_id = $1 group by reason",
        )
        .bind(uid(&user_id))
        .fetch_all(&ctx.admin)
        .await
        .expect("read the grants");
        assert_eq!(rows.len(), 1);
        let reason: String = rows[0].try_get("reason").expect("reason");
        let n: String = rows[0].try_get("n").expect("n");
        assert_eq!(reason, "launch");
        assert_eq!(n.parse::<usize>().expect("a count"), PLAYABLE_COUNT);
        ctx.finish().await;
    }
}

// ---------------------------------------------------------------------------
// SPEC §9.4's redemption, as the one database transaction the spec describes
// ---------------------------------------------------------------------------

mod app_redeem_invite_code {
    use super::*;

    /// A code inserted straight through `codes.insert`, answered as the row's JSON.
    async fn mint(db: &Db, max_uses: i64, expires_at: Option<i64>, revoked: bool) -> Value {
        let code = json!({
            "id": uuid(),
            "codeHash": format!("hash-{}", uuid()),
            "maxUses": max_uses,
            "uses": 0,
            "revoked": revoked,
            "expiresAt": expires_at,
            "createdAt": now_ms(),
        });
        once!(db, None, |tx| tx.codes_insert(&de(code.clone()))).expect("codes.insert");
        code
    }

    fn hash_of(code: &Value) -> String {
        code["codeHash"].as_str().expect("a code hash").to_string()
    }

    #[tokio::test]
    async fn flips_pending_to_active_and_consumes_one_use_in_one_call() {
        let Some(ctx) = setup().await else { return };
        let user_id = ctx.sign_up().await;
        let code = mint(&ctx.db, 1, None, false).await;

        assert_eq!(redeem(&ctx.db, &user_id, Some(hash_of(&code).as_str()), "ip").await, "ok");

        assert_eq!(profile(&ctx.db, &user_id).await["status"], "active");
        assert_eq!(code_by_hash(&ctx.db, &hash_of(&code)).await["uses"].as_i64(), Some(1));
        // §9.4 step 4, and R111's trigger on the way past: both ledgers were written too.
        let owned = once!(ctx.db, Some(user_id.as_str()), |tx| tx.collection_get(&user_id)).expect("collection.get");
        assert_eq!(owned.len(), PLAYABLE_COUNT);
        ctx.finish().await;
    }

    #[tokio::test]
    async fn gives_missing_expired_revoked_and_exhausted_codes_the_same_answer() {
        let Some(ctx) = setup().await else { return };
        let expired = mint(&ctx.db, 1, Some(now_ms() - 60_000), false).await;
        let revoked = mint(&ctx.db, 1, None, true).await;
        let exhausted = mint(&ctx.db, 1, None, false).await;
        let exhausted_id = exhausted["id"].as_str().expect("an id").to_string();
        once!(ctx.db, None, |tx| tx.codes_claim(&exhausted_id, now_ms())).expect("codes.claim");

        for code_hash in ["no-such-hash".to_string(), hash_of(&expired), hash_of(&revoked), hash_of(&exhausted)] {
            let user_id = ctx.sign_up().await;
            assert_eq!(redeem(&ctx.db, &user_id, Some(code_hash.as_str()), "ip").await, "invalid_code", "{code_hash}");
            assert_eq!(profile(&ctx.db, &user_id).await["status"], "pending");
        }
        ctx.finish().await;
    }

    #[tokio::test]
    async fn refuses_an_account_that_is_not_pending_and_one_with_an_unverified_email() {
        let Some(ctx) = setup().await else { return };
        let code = mint(&ctx.db, 5, None, false).await;

        let active = ctx.active_profile().await;
        assert_eq!(redeem(&ctx.db, &active, Some(hash_of(&code).as_str()), "ip").await, "not_pending");

        let unverified = ctx.sign_up().await;
        sqlx::query("update auth.users set email_confirmed_at = null where id = $1")
            .bind(uid(&unverified))
            .execute(&ctx.admin)
            .await
            .expect("unverify the email");
        assert_eq!(redeem(&ctx.db, &unverified, Some(hash_of(&code).as_str()), "ip").await, "email_unverified");
        ctx.finish().await;
    }

    /// §9.4 step 2.
    #[tokio::test]
    async fn rate_limits_a_profile_past_5_attempts_in_the_hour() {
        let Some(ctx) = setup().await else { return };
        let user_id = ctx.sign_up().await;
        let code = mint(&ctx.db, 99, None, false).await;
        for _ in 0..6 {
            log_attempt(&ctx.db, attempt(Some(user_id.as_str()), "ip", "rejected", "missing", now_ms())).await;
        }
        assert_eq!(redeem(&ctx.db, &user_id, Some(hash_of(&code).as_str()), "ip").await, "rate_limited_profile");
        ctx.finish().await;
    }

    /// §9.4 step 4.
    #[tokio::test]
    async fn logs_the_attempt_either_way() {
        let Some(ctx) = setup().await else { return };
        let user_id = ctx.sign_up().await;
        redeem(&ctx.db, &user_id, Some("no-such-hash"), "ip-1").await;
        let since = now_ms() - 60_000;
        let by_profile = once!(ctx.db, Some(user_id.as_str()), |tx| tx.codes_count_attempts_by_profile(&user_id, since))
            .expect("codes.countAttemptsByProfile");
        assert_eq!(by_profile, 1);
        let failures = once!(ctx.db, None, |tx| tx.codes_count_failures(since)).expect("codes.countFailures");
        assert_eq!(failures, 1);
        ctx.finish().await;
    }
}

// ---------------------------------------------------------------------------
// app.upsert_deck / app.upsert_trio: the SQL is stricter than the port, on purpose
// ---------------------------------------------------------------------------

mod r250_r252_app_upsert_deck_and_app_upsert_trio {
    use super::*;

    /// A saved deck of five fixture cards; `over` replaces any of its fields.
    fn deck(profile_id: &str, over: Value) -> Value {
        let at = now_ms();
        let mut deck = json!({
            "id": uuid(),
            "profileId": profile_id,
            "name": "Aggro",
            "cards": deck_of(0)[..5].to_vec(),
            "portrait": null,
            "catalogVersion": CATALOG_VERSION,
            "createdAt": at,
            "updatedAt": at,
        });
        if let (Some(fields), Some(base)) = (over.as_object(), deck.as_object_mut()) {
            for (key, value) in fields {
                base.insert(key.clone(), value.clone());
            }
        }
        deck
    }

    /// `decks.upsert`, in its own transaction: the outcome's JSON, or the refusal's text.
    async fn upsert(db: &Db, deck: Value, cap: i64) -> Result<Value, String> {
        let profile_id = deck["profileId"].as_str().unwrap_or_default().to_string();
        once!(db, Some(profile_id.as_str()), |tx| tx.decks_upsert(&de(deck.clone()), cap)).map(|outcome| js(&outcome))
    }

    async fn listed(db: &Db, profile_id: &str) -> Vec<Value> {
        let decks = once!(db, Some(profile_id), |tx| tx.decks_list(profile_id)).expect("decks.list");
        decks.iter().map(js).collect()
    }

    /// ports.ts: "The cap is checked under a lock on the profile, so two concurrent creates cannot
    /// both pass it." The in-memory store is single-threaded and cannot show this; here the creates
    /// really do run at once, on separate connections, each in its own transaction.
    #[tokio::test]
    async fn r250_lets_exactly_as_many_concurrent_creates_through_as_the_cap_has_room_for() {
        let Some(ctx) = setup().await else { return };
        let profile_id = ctx.active_profile().await;
        let racing_pool = ctx.racing(6).await;
        let racing = Db::Pg(racing_pool.clone());
        upsert(&racing, deck(&profile_id, json!({})), 3).await.expect("the first create");
        upsert(&racing, deck(&profile_id, json!({})), 3).await.expect("the second create");
        let outcomes = join_all((0..6).map(|_| upsert(&racing, deck(&profile_id, json!({})), 3)).collect()).await;
        racing_pool.close().await;

        let outcomes: Vec<Value> = outcomes.into_iter().map(|outcome| outcome.expect("decks.upsert")).collect();
        assert_eq!(outcomes.iter().filter(|outcome| **outcome == "created").count(), 1);
        assert_eq!(outcomes.iter().filter(|outcome| **outcome == "limit").count(), 5);
        assert_eq!(listed(&ctx.db, &profile_id).await.len(), 3);
        ctx.finish().await;
    }

    /// R256: a retried save of one new deck, sent twice at once, makes one deck.
    #[tokio::test]
    async fn r256_turns_two_simultaneous_saves_of_one_new_id_into_one_create_and_one_update() {
        let Some(ctx) = setup().await else { return };
        let profile_id = ctx.active_profile().await;
        let racing_pool = ctx.racing(2).await;
        let racing = Db::Pg(racing_pool.clone());
        let draft = deck(&profile_id, json!({}));
        let (first, second) = tokio::join!(upsert(&racing, draft.clone(), 10), upsert(&racing, draft.clone(), 10));
        racing_pool.close().await;

        let mut outcomes = vec![
            first.expect("decks.upsert").as_str().unwrap_or_default().to_string(),
            second.expect("decks.upsert").as_str().unwrap_or_default().to_string(),
        ];
        outcomes.sort();
        assert_eq!(outcomes, vec!["created", "updated"]);
        assert_eq!(listed(&ctx.db, &profile_id).await.len(), 1);
        ctx.finish().await;
    }

    /// The database's own cap (0007's `max_saved_decks`) holds even for a caller that asks for more.
    #[tokio::test]
    async fn r250_applies_app_settings_cap_when_the_callers_is_larger() {
        let Some(ctx) = setup().await else { return };
        let profile_id = ctx.active_profile().await;
        sqlx::raw_sql("update app.settings set value = to_jsonb(2) where key = 'max_saved_decks'")
            .execute(&ctx.admin)
            .await
            .expect("lower the cap");
        let mut outcomes = Vec::new();
        for _ in 0..3 {
            outcomes.push(upsert(&ctx.db, deck(&profile_id, json!({})), 10).await);
        }
        // TS's `finally`: the cap goes back before anything is asserted.
        sqlx::raw_sql("update app.settings set value = to_jsonb(10) where key = 'max_saved_decks'")
            .execute(&ctx.admin)
            .await
            .expect("restore the cap");

        let outcomes: Vec<Value> = outcomes.into_iter().map(|outcome| outcome.expect("decks.upsert")).collect();
        assert_eq!(outcomes, vec![json!("created"), json!("created"), json!("limit")]);
        ctx.finish().await;
    }

    /// KNOWN DIVERGENCES (deck and trio strictness): refused here, accepted by the fake.
    #[tokio::test]
    async fn r250_refuses_a_deck_the_servers_d1_d2_and_d4_would_have_refused() {
        let Some(ctx) = setup().await else { return };
        let profile_id = ctx.active_profile().await;
        let mut oversized = deck_of(0);
        oversized.push("core-061".to_string());
        assert_refused(upsert(&ctx.db, deck(&profile_id, json!({ "cards": oversized })), 10).await, "deck: D2");
        assert_refused(
            upsert(&ctx.db, deck(&profile_id, json!({ "cards": ["core-001", "core-001"] })), 10).await,
            "deck: D4",
        );
        assert_refused(upsert(&ctx.db, deck(&profile_id, json!({ "name": "   " })), 10).await, "needs a name");
        assert_refused(
            upsert(&ctx.db, deck(&profile_id, json!({ "name": "x".repeat(41) })), 10).await,
            "at most 40",
        );
        assert_eq!(listed(&ctx.db, &profile_id).await, Vec::<Value>::new());
        ctx.finish().await;
    }

    /// §9.4's gate: a pending account has no collection, deck, queue or match.
    #[tokio::test]
    async fn r250_refuses_a_deck_or_a_trio_for_a_profile_that_is_not_active() {
        let Some(ctx) = setup().await else { return };
        let pending = ctx.sign_up().await;
        assert_refused(upsert(&ctx.db, deck(&pending, json!({})), 10).await, "not active");
        let at = now_ms();
        let trio = json!({
            "id": uuid(), "profileId": pending, "name": "Ladder", "deckIds": [null, null, null],
            "createdAt": at, "updatedAt": at,
        });
        assert_refused(
            once!(ctx.db, Some(pending.as_str()), |tx| tx.trios_upsert(&de(trio.clone()), 5)).map(|outcome| js(&outcome)),
            "not active",
        );
        ctx.finish().await;
    }

    /// The composite foreign key, not only the function's check, keeps a slot inside its profile.
    #[tokio::test]
    async fn r252_refuses_a_raw_trio_row_naming_another_profiles_deck() {
        let Some(ctx) = setup().await else { return };
        let (owner, other) = (ctx.active_profile().await, ctx.active_profile().await);
        let theirs = deck(&other, json!({}));
        upsert(&ctx.db, theirs.clone(), 10).await.expect("their deck");
        let raw = sqlx::query("insert into public.trios (id, profile_id, name, deck1_id) values ($1, $2, 'Stolen', $3)")
            .bind(uid(&uuid()))
            .bind(uid(&owner))
            .bind(uid(theirs["id"].as_str().expect("a deck id")))
            .execute(&ctx.admin)
            .await
            .map_err(|error| error.to_string());
        assert_refused(raw, "trios_deck1_fk");
        ctx.finish().await;
    }

    #[tokio::test]
    async fn r252_answers_unknown_deck_for_a_slot_that_is_not_a_deck_id_at_all() {
        let Some(ctx) = setup().await else { return };
        let profile_id = ctx.active_profile().await;
        let at = now_ms();
        let trio = json!({
            "id": uuid(), "profileId": profile_id, "name": "Ladder", "deckIds": ["not-a-uuid", null, null],
            "createdAt": at, "updatedAt": at,
        });
        let outcome = once!(ctx.db, Some(profile_id.as_str()), |tx| tx.trios_upsert(&de(trio.clone()), 5)).expect("trios.upsert");
        assert_eq!(js(&outcome), "unknown_deck");
        let got = once!(ctx.db, None, |tx| tx.decks_get("not-a-uuid")).expect("decks.get");
        assert!(got.is_none());
        let removed = once!(ctx.db, Some(profile_id.as_str()), |tx| tx.decks_remove(&profile_id, "not-a-uuid"))
            .expect("decks.remove");
        assert!(!removed);
        ctx.finish().await;
    }
}

// ---------------------------------------------------------------------------
// R263: a reserved match id is a row here, and discarding it releases what it held
// ---------------------------------------------------------------------------

mod r263_matches_discard_open {
    use super::*;

    fn trio() -> Value {
        json!({
            "name": "Ladder",
            "decks": [
                { "name": "A", "cards": deck_of(0) },
                { "name": "B", "cards": deck_of(1) },
                { "name": "C", "cards": deck_of(2) },
            ],
        })
    }

    fn ticket(profile_id: &str) -> Value {
        json!({
            "id": uuid(),
            "profileId": profile_id,
            "rating": 1000,
            "mode": "bo3",
            "deck": [],
            "trio": trio(),
            "catalogVersion": CATALOG_VERSION,
            "enqueuedAt": now_ms(),
            "status": "open",
            "matchId": null,
        })
    }

    #[tokio::test]
    async fn r263_deletes_a_claimed_pairs_reservation_and_unlinks_both_tickets() {
        let Some(ctx) = setup().await else { return };
        let (a, b) = (ctx.active_profile().await, ctx.active_profile().await);
        let (ta, tb) = (ticket(&a), ticket(&b));
        let (ta_id, tb_id) = (ta["id"].as_str().unwrap_or_default().to_string(), tb["id"].as_str().unwrap_or_default().to_string());
        once!(ctx.db, Some(a.as_str()), |tx| tx.tickets_insert(&de(ta.clone()))).expect("tickets.insert a");
        once!(ctx.db, Some(b.as_str()), |tx| tx.tickets_insert(&de(tb.clone()))).expect("tickets.insert b");
        let reserved = uuid();
        let claimed = once!(ctx.db, None, |tx| tx.tickets_claim_pair(&ta_id, &tb_id, &reserved, now_ms())).expect("claimPair");
        assert!(claimed);
        let before = once!(ctx.db, None, |tx| tx.tickets_get(&ta_id)).expect("tickets.get");
        assert_eq!(js(&must(before, "ticket a"))["matchId"], reserved.as_str());

        once!(ctx.db, None, |tx| tx.matches_discard_open(&reserved)).expect("matches.discardOpen");

        let rows = sqlx::query("select 1 from public.matches where id = $1")
            .bind(uid(&reserved))
            .fetch_all(&ctx.admin)
            .await
            .expect("read the matches");
        assert_eq!(rows.len(), 0);
        // `tickets.match_id` is `on delete set null` (0004): the tickets stay matched and forget the id.
        let after = js(&must(once!(ctx.db, None, |tx| tx.tickets_get(&ta_id)).expect("tickets.get"), "ticket a"));
        assert_eq!(after["status"], "matched");
        assert_eq!(after["matchId"], Value::Null);
        ctx.finish().await;
    }

    /// R110: a code is free again once its room is gone.
    #[tokio::test]
    async fn r263_frees_a_claimed_rooms_code_for_the_next_room() {
        let Some(ctx) = setup().await else { return };
        let (host, guest, next) = (ctx.active_profile().await, ctx.active_profile().await, ctx.active_profile().await);
        let now = now_ms();
        let room = json!({
            "code": "BCD345",
            "hostProfileId": host,
            "mode": "bo3",
            "hostDeck": [],
            "hostTrio": trio(),
            "catalogVersion": CATALOG_VERSION,
            "createdAt": now,
            "expiresAt": now + 600_000,
            "guestProfileId": null,
            "matchId": null,
        });
        let mut next_room = room.clone();
        next_room["hostProfileId"] = json!(next);

        assert!(once!(ctx.db, Some(host.as_str()), |tx| tx.rooms_create(&de(room.clone()))).expect("rooms.create"));
        let reserved = uuid();
        let claimed = once!(ctx.db, Some(guest.as_str()), |tx| tx.rooms_claim("BCD345", &guest, &reserved, now)).expect("rooms.claim");
        must(claimed, "the claim");
        assert!(!once!(ctx.db, Some(next.as_str()), |tx| tx.rooms_create(&de(next_room.clone()))).expect("rooms.create"));

        once!(ctx.db, None, |tx| tx.matches_discard_open(&reserved)).expect("matches.discardOpen");

        assert!(once!(ctx.db, None, |tx| tx.rooms_get("BCD345")).expect("rooms.get").is_none());
        assert!(once!(ctx.db, Some(next.as_str()), |tx| tx.rooms_create(&de(next_room.clone()))).expect("rooms.create"));
        ctx.finish().await;
    }

    #[tokio::test]
    async fn r263_never_touches_a_finished_match() {
        let Some(ctx) = setup().await else { return };
        let (a, b) = (ctx.active_profile().await, ctx.active_profile().await);
        let match_id = uuid();
        let now = now_ms();
        create_match(&ctx.db, match_row(&match_id, "seed-1", &a, &b, now)).await;
        once!(ctx.db, None, |tx| tx.matches_finish(&match_id, now + 1)).expect("matches.finish");
        once!(ctx.db, None, |tx| tx.matches_discard_open(&match_id)).expect("matches.discardOpen");
        let row = once!(ctx.db, None, |tx| tx.matches_get(&match_id)).expect("matches.get");
        assert_eq!(js(&must(row, "the match"))["status"], "finished");
        ctx.finish().await;
    }
}

mod r672_profiles_current_match_id {
    use super::*;

    #[tokio::test]
    async fn r672_refuses_the_flags_ahead_of_the_row_and_takes_them_once_the_row_exists() {
        let Some(ctx) = setup().await else { return };
        let (a, b) = (ctx.active_profile().await, ctx.active_profile().await);
        let match_id = uuid();
        let now = now_ms();
        // Flagging before the row exists violates the foreign key (0004): this is why a rematch
        // starts its game before it flags the seats (`src/api/rematch.rs`).
        assert_refused(
            once!(ctx.db, Some(a.as_str()), |tx| tx.profiles_set_in_match(&a, Some(match_id.as_str()))),
            "current_match_id",
        );
        create_match(&ctx.db, match_row(&match_id, "seed-rematch", &a, &b, now)).await;
        once!(ctx.db, Some(a.as_str()), |tx| tx.profiles_set_in_match(&a, Some(match_id.as_str()))).expect("setInMatch a");
        once!(ctx.db, Some(b.as_str()), |tx| tx.profiles_set_in_match(&b, Some(match_id.as_str()))).expect("setInMatch b");
        let both = once!(ctx.db, None, |tx| tx.profiles_get_many(&[a.clone(), b.clone()])).expect("profiles.getMany");
        let both: Vec<Value> = both.iter().map(js).collect();
        let in_match = |id: &str| {
            both.iter().find(|profile| profile["id"] == id).map(|profile| profile["inMatchId"].clone())
        };
        assert_eq!(in_match(a.as_str()), Some(json!(match_id)));
        assert_eq!(in_match(b.as_str()), Some(json!(match_id)));
        ctx.finish().await;
    }
}

// ---------------------------------------------------------------------------
// app.append_match_action: the nonce dedupe the port cannot express
// ---------------------------------------------------------------------------

mod app_append_match_action {
    use super::*;

    struct Live {
        id: String,
        p2: String,
    }

    async fn live_match(ctx: &Ctx) -> Live {
        let (p1, p2) = (ctx.active_profile().await, ctx.active_profile().await);
        let match_id = uuid();
        create_match(&ctx.db, match_row(&match_id, "seed-1", &p1, &p2, now_ms())).await;
        Live { id: match_id, p2 }
    }

    fn action_row(match_id: &str, seq: i64, player_id: &str, nonce: &str) -> MatchActionRow {
        de(json!({
            "matchId": match_id,
            "seq": seq,
            "action": { "type": "endTurn", "playerId": player_id, "nonce": nonce },
            "at": now_ms(),
        }))
    }

    async fn append(db: &Db, row: MatchActionRow) -> Result<(), String> {
        once!(db, None, |tx| tx.matches_append_actions(std::slice::from_ref(&row)))
    }

    #[tokio::test]
    async fn keeps_matches_last_seq_in_step_with_the_seq_the_actor_assigns() {
        let Some(ctx) = setup().await else { return };
        let live = live_match(&ctx).await;
        for seq in 1..=3 {
            append(&ctx.db, action_row(&live.id, seq, "p1", &format!("n{seq}"))).await.expect("appendActions");
        }
        let rows = sqlx::query("select last_seq::text from public.matches where id = $1")
            .bind(uid(&live.id))
            .fetch_all(&ctx.admin)
            .await
            .expect("read the match");
        let last: String = must(rows.first(), "the match row").try_get("last_seq").expect("last_seq");
        assert_eq!(last.parse::<i64>().expect("a seq"), 3);
        ctx.finish().await;
    }

    #[tokio::test]
    async fn records_the_seats_profile_id_alongside_the_action() {
        let Some(ctx) = setup().await else { return };
        let live = live_match(&ctx).await;
        append(&ctx.db, action_row(&live.id, 1, "p2", "n1")).await.expect("appendActions");
        let rows = sqlx::query("select player_id, player_seat from public.match_actions where match_id = $1")
            .bind(uid(&live.id))
            .fetch_all(&ctx.admin)
            .await
            .expect("read the actions");
        let row = must(rows.first(), "the action row");
        let seat: String = row.try_get("player_seat").expect("player_seat");
        let player: uuid::Uuid = row.try_get("player_id").expect("player_id");
        assert_eq!(seat, "p2");
        assert_eq!(player.to_string(), live.p2);
        ctx.finish().await;
    }

    #[tokio::test]
    async fn raises_rather_than_double_writing_when_a_nonce_is_replayed() {
        let Some(ctx) = setup().await else { return };
        let live = live_match(&ctx).await;
        append(&ctx.db, action_row(&live.id, 1, "p1", "same")).await.expect("appendActions");
        assert_refused(append(&ctx.db, action_row(&live.id, 2, "p1", "same")).await, "append-only");
        let logged = once!(ctx.db, None, |tx| tx.matches_actions(&live.id)).expect("matches.actions");
        assert_eq!(logged.len(), 1);
        ctx.finish().await;
    }

    #[tokio::test]
    async fn cannot_edit_or_delete_a_logged_action_app_deny_row_mutation() {
        let Some(ctx) = setup().await else { return };
        let live = live_match(&ctx).await;
        append(&ctx.db, action_row(&live.id, 1, "p1", "n1")).await.expect("appendActions");
        let edited = sqlx::query("update public.match_actions set nonce = 'edited' where match_id = $1")
            .bind(uid(&live.id))
            .execute(&ctx.admin)
            .await
            .map_err(|error| error.to_string());
        assert_refused(edited, "append-only");
        ctx.finish().await;
    }
}

// ---------------------------------------------------------------------------
// The two atomic claims (§9.5), under genuine concurrency
// ---------------------------------------------------------------------------

mod concurrency {
    use super::*;

    async fn claim_room(db: &Db, code: &str, guest: &str, match_id: String, at: i64) -> bool {
        once!(db, Some(guest), |tx| tx.rooms_claim(code, guest, &match_id, at)).expect("rooms.claim").is_some()
    }

    async fn claim_pair(db: &Db, a: &str, b: &str, match_id: String) -> bool {
        once!(db, None, |tx| tx.tickets_claim_pair(a, b, &match_id, now_ms())).expect("tickets.claimPair")
    }

    async fn claim_code(db: &Db, code_id: &str) -> bool {
        once!(db, None, |tx| tx.codes_claim(code_id, now_ms())).expect("codes.claim")
    }

    #[tokio::test]
    async fn lets_only_one_of_two_simultaneous_joiners_claim_a_room() {
        let Some(ctx) = setup().await else { return };
        let host = ctx.active_profile().await;
        let (a, b) = (ctx.active_profile().await, ctx.active_profile().await);
        let now = now_ms();
        let room = json!({
            "code": "ABC234",
            "hostProfileId": host,
            "mode": "bo1",
            "hostDeck": deck_of(0),
            "hostTrio": null,
            "catalogVersion": CATALOG_VERSION,
            "createdAt": now,
            "expiresAt": now + 600_000,
            "guestProfileId": null,
            "matchId": null,
        });
        once!(ctx.db, Some(host.as_str()), |tx| tx.rooms_create(&de(room.clone()))).expect("rooms.create");

        // Two connections, so the two claims really overlap.
        let racing_pool = ctx.racing(2).await;
        let racing = Db::Pg(racing_pool.clone());
        let results = join_all(vec![
            claim_room(&racing, "ABC234", &a, uuid(), now),
            claim_room(&racing, "ABC234", &b, uuid(), now),
        ])
        .await;
        racing_pool.close().await;
        assert_eq!(results.iter().filter(|claimed| **claimed).count(), 1);
        ctx.finish().await;
    }

    #[tokio::test]
    async fn lets_only_one_of_two_simultaneous_matchers_claim_the_same_ticket_pair() {
        let Some(ctx) = setup().await else { return };
        let (a, b) = (ctx.active_profile().await, ctx.active_profile().await);
        let mut ids = Vec::new();
        for profile_id in [&a, &b] {
            let ticket = json!({
                "id": uuid(),
                "profileId": profile_id,
                "rating": 1000,
                "mode": "bo1",
                "deck": deck_of(0),
                "trio": null,
                "catalogVersion": CATALOG_VERSION,
                "enqueuedAt": now_ms(),
                "status": "open",
                "matchId": null,
            });
            once!(ctx.db, Some(profile_id.as_str()), |tx| tx.tickets_insert(&de(ticket.clone()))).expect("tickets.insert");
            ids.push(ticket["id"].as_str().unwrap_or_default().to_string());
        }
        let (ta, tb) = (must(ids.first(), "ticket a"), must(ids.get(1), "ticket b"));

        let racing_pool = ctx.racing(2).await;
        let racing = Db::Pg(racing_pool.clone());
        let won = join_all(vec![claim_pair(&racing, ta, tb, uuid()), claim_pair(&racing, ta, tb, uuid())]).await;
        racing_pool.close().await;
        assert_eq!(won.iter().filter(|claimed| **claimed).count(), 1);
        ctx.finish().await;
    }

    #[tokio::test]
    async fn lets_only_one_of_two_simultaneous_redemptions_consume_the_last_use() {
        let Some(ctx) = setup().await else { return };
        let code = json!({
            "id": uuid(), "codeHash": format!("hash-{}", uuid()), "maxUses": 1, "uses": 0, "revoked": false,
            "expiresAt": null, "createdAt": now_ms(),
        });
        once!(ctx.db, None, |tx| tx.codes_insert(&de(code.clone()))).expect("codes.insert");
        let code_id = code["id"].as_str().unwrap_or_default().to_string();

        let racing_pool = ctx.racing(2).await;
        let racing = Db::Pg(racing_pool.clone());
        let claims = join_all(vec![claim_code(&racing, &code_id), claim_code(&racing, &code_id)]).await;
        racing_pool.close().await;
        assert_eq!(claims.iter().filter(|claimed| **claimed).count(), 1);
        let hash = code["codeHash"].as_str().unwrap_or_default().to_string();
        assert_eq!(code_by_hash(&ctx.db, &hash).await["uses"].as_i64(), Some(1));
        ctx.finish().await;
    }
}

// ---------------------------------------------------------------------------
// The reaper's input (§9.5)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn app_live_matches_backs_matches_live_and_an_open_room_is_not_one() {
    let Some(ctx) = setup().await else { return };
    let host = ctx.active_profile().await;
    let guest = ctx.active_profile().await;
    let now = now_ms();
    let room = json!({
        "code": "ABC234",
        "hostProfileId": host,
        "mode": "bo1",
        "hostDeck": deck_of(0),
        "hostTrio": null,
        "catalogVersion": CATALOG_VERSION,
        "createdAt": now,
        "expiresAt": now + 600_000,
        "guestProfileId": null,
        "matchId": null,
    });
    once!(ctx.db, Some(host.as_str()), |tx| tx.rooms_create(&de(room.clone()))).expect("rooms.create");
    assert!(once!(ctx.db, None, |tx| tx.matches_live()).expect("matches.live").is_empty());

    let match_id = uuid();
    once!(ctx.db, Some(guest.as_str()), |tx| tx.rooms_claim("ABC234", &guest, &match_id, now)).expect("rooms.claim");
    // Claimed but not yet created by the registry: still not a match anyone can fold.
    assert!(once!(ctx.db, None, |tx| tx.matches_live()).expect("matches.live").is_empty());
    assert!(once!(ctx.db, None, |tx| tx.matches_get(&match_id)).expect("matches.get").is_none());

    create_match(&ctx.db, match_row(&match_id, "seed-1", &host, &guest, now)).await;
    let live = once!(ctx.db, None, |tx| tx.matches_live()).expect("matches.live");
    let ids: Vec<Value> = live.iter().map(|row| js(row)["id"].clone()).collect();
    assert_eq!(ids, vec![json!(match_id)]);
    ctx.finish().await;
}
