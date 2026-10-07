//! ONE suite, TWO stores. Every assertion here runs against both `crates/server/src/db/fake.rs` (the
//! in-memory fixture the rest of the server suite already trusts) and `crates/server/src/db/pg.rs`
//! (the real Postgres implementation), so "the fake and the real one agree" is a test result rather
//! than a hope. Port of `apps/server/test/db/contract.ts`, with its harness (`test/db/harness.ts`,
//! `contract.memory.test.ts`, `contract.postgres.spec.ts`) folded in below.
//!
//! It exists because of what went wrong without it: `src/db/store.ts` did not exist at all, the
//! server booted only in `E2E=1` mode, and nothing in `pnpm test` could notice — every server test
//! passed against a fake that nothing was ever compared to.
//!
//! What is asserted is the list in `e2e-store.ts`'s own header: the invariants SPEC §9.4 and §9.5
//! lean on, the ones "a laxer fixture would let a real bug pass the suite" past. Where the schema
//! and the port genuinely cannot hold the same information, the divergence is named in
//! `db/pg.rs`'s KNOWN DIVERGENCES block and the assertion here is written to the weaker of the two
//! (deck order, action timestamps) rather than deleted.
//!
//!   cargo test -p jackioh-server --test server store::contract   -> `<case>::memory::*` always;
//!                                                                    `<case>::postgres::*` skip
//!   test:db (crates/server/tests/db/run.sh, DATABASE_URL set)     -> `<case>::postgres::*` run too
//!
//! How the TS reads in Rust:
//!
//! - A TS call on `store` outside `store.tx` is one transaction of its own: `q!(h, t => t.<method>(…))`
//!   begins it, makes the call, commits it when the call succeeded and answers the value (it panics,
//!   naming the store and the call, on an error). `call!` is the same and answers the `Result`, for
//!   the cases TS wrote as `rejects.toThrow()`.
//! - Every port value is built from TS's own object literal with `from(json!({ … }))` and read back
//!   through `j(&value)` (its JSON), so the cases lean on the wire shape SURFACE §5.1 fixes (TS's
//!   keys, camelCase) and not on how the Rust structs spell their fields.
//! - `beforeEach(harness.reset)` is the first line of every generated test (`both_stores!`).

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use indexmap::{IndexMap, IndexSet};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use jackioh_engine::{Action, GameRecord, LastBoardEntry};
use jackioh_server::db::fake::{self, E2eStoreOptions, FakeCatalog, FakeData, RedemptionSettings};
use jackioh_server::db::store::{
    BotRating, CodeAttempt, CollectionEntry, CollectionGrant, Db, FrozenTrio, GameRecordQuery, InviteCode,
    LastBoardKind, MatchActionRow, MatchClocks, MatchRow, PlayerSettingsLimits, PlayerSettingsMergeInput,
    PlayerSettingsRow, PlayerStatsListOptions, Profile, ProfileCreateInput, ProfileStatus, RatedGameRow,
    RedeemInviteCodeInput, ResultRow, RetentionPurgeInput, Room, SavedDeck, SavedTrio, Season, SeriesRow,
    StoreError, Ticket, TutorialMergeInput, TutorialProgressRow, Tx,
};
use jackioh_server::ranked::glicko2::Glicko;
use jackioh_server::ranked::ladder::{SeasonRank, fresh_rank};
use jackioh_server::ranked::season::ResetChange;

// ---------------------------------------------------------------------------
// One store call, one transaction
// ---------------------------------------------------------------------------

/// One store call in a transaction of its own, committed when it succeeds and rolled back (the
/// transaction dropped) when it fails: what a TS call on `store` outside `store.tx` is. Answers
/// the call's `Result`.
macro_rules! call {
    ($h:expr, $t:ident => $body:expr) => {{
        let mut $t = $h
            .db
            .begin(None)
            .await
            .unwrap_or_else(|error| panic!("{}: begin a transaction: {error}", $h.name));
        let outcome = $body.await;
        if outcome.is_ok() {
            $t.commit()
                .await
                .unwrap_or_else(|error| panic!("{}: commit {}: {error}", $h.name, stringify!($body)));
        }
        outcome
    }};
}

/// `call!`, answering the value; an error fails the test, naming the store and the call.
macro_rules! q {
    ($h:expr, $t:ident => $body:expr) => {
        call!($h, $t => $body).unwrap_or_else(|error| panic!("{}: {}: {error}", $h.name, stringify!($body)))
    };
}

/// The cases listed, once against each store: `memory::<case>` always (`cargo test`'s half,
/// `contract.memory.test.ts`), `postgres::<case>` when `DATABASE_URL` is set (`tests/db/run.sh`'s
/// half, `contract.postgres.spec.ts`). Each starts with the harness's `reset`, TS's `beforeEach`.
macro_rules! both_stores {
    ($($case:ident),+ $(,)?) => {
        /// Against the in-memory fake (`db/fake.rs`). A failure here means the FIXTURE broke the
        /// contract.
        mod memory {
            $(
                #[tokio::test]
                async fn $case() {
                    let harness = $crate::store::contract::StoreHarness::memory();
                    harness.reset().await;
                    super::$case(&harness).await;
                    harness.close().await;
                }
            )+
        }

        /// Against a real Postgres (`db/pg.rs`), one test at a time: one database, shared tables,
        /// `truncate` between tests. Skipped when `DATABASE_URL` is unset, so `cargo test` stays
        /// hermetic and never needs Docker. A failure only here means the real store broke it.
        mod postgres {
            $(
                #[tokio::test]
                async fn $case() {
                    let _one_at_a_time = $crate::store::contract::PG_SERIAL.lock().await;
                    let Some(harness) = $crate::store::contract::StoreHarness::postgres().await else {
                        return;
                    };
                    harness.reset().await;
                    super::$case(&harness).await;
                    harness.close().await;
                }
            )+
        }
    };
}

// ---------------------------------------------------------------------------
// A fixture catalog big enough for a legal loadout (← test/db/harness.ts)
// ---------------------------------------------------------------------------

/// SPEC §9.4 L2: `DECK_SIZE` is 20, and L1 says three decks, so 60 ids is the floor.
pub const CATALOG_VERSION: &str = "core-1";

/// `core-001` … `core-064`.
pub fn playable_ids() -> Vec<String> {
    (1..=64).map(|i| format!("core-{i:03}")).collect()
}

/// §9.4 L3: "no Token-tagged cards" — here so the R111 launch grant can be seen skipping them.
pub fn token_ids() -> Vec<String> {
    vec!["core-001.1".to_string(), "core-002.1".to_string()]
}

/// TS's `fixtureCatalog(): CatalogInfo`. The contract never renders a card, so it had no defs:
/// `CatalogInfo` was consulted here only for `cardIds`, `isToken` and `isBanned` (the three R111
/// reads), which are all the fake's `FakeCatalog` holds. The version is `CATALOG_VERSION`.
pub fn fixture_catalog() -> FakeCatalog {
    let all: Vec<String> = playable_ids().into_iter().chain(token_ids()).collect();
    let tokens: IndexSet<String> = token_ids().into_iter().collect();
    FakeCatalog {
        card_ids: all,
        is_token: Arc::new(move |card_id: &str| tokens.contains(card_id)),
        is_banned: Arc::new(|_card_id: &str| false),
    }
}

// ---------------------------------------------------------------------------
// The harness the contract drives (← test/db/harness.ts)
// ---------------------------------------------------------------------------

/// The Postgres runs of every suite in this binary share one database; they take this first.
pub static PG_SERIAL: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub struct StoreHarness {
    /// Appears in every failure message, so a failure says which store broke.
    pub name: &'static str,
    pub db: Db,
    pub playable_ids: Vec<String>,
    pub token_ids: Vec<String>,
    pub catalog_version: String,
    ends: Ends,
}

/// What each store's harness holds besides the store itself.
enum Ends {
    Memory {
        /// The fake's tables, which `reset` empties (`store.reset()`).
        data: Arc<tokio::sync::Mutex<FakeData>>,
        /// The two pieces of database state `app.redeem_invite_code` reads and the port cannot:
        /// the managed-auth verification flag and the redemption switch. Held here so the
        /// contract drives both stores through one pair of harness methods.
        unverified: Arc<std::sync::Mutex<IndexSet<String>>>,
        redemption_enabled: Arc<AtomicBool>,
        next_user: AtomicU32,
    },
    Postgres {
        /// A raw superuser connection, for the setup and teardown the store deliberately cannot do.
        admin: PgPool,
    },
}

impl StoreHarness {
    // -----------------------------------------------------------------------
    // In-memory
    // -----------------------------------------------------------------------

    pub fn memory() -> StoreHarness {
        let unverified = Arc::new(std::sync::Mutex::new(IndexSet::<String>::new()));
        let redemption_enabled = Arc::new(AtomicBool::new(true));
        let redemption = RedemptionSettings {
            email_verified: {
                let unverified = Arc::clone(&unverified);
                Arc::new(move |profile_id: &str| {
                    !unverified
                        .lock()
                        .expect("the unverified profiles")
                        .contains(profile_id)
                })
            },
            enabled: {
                let enabled = Arc::clone(&redemption_enabled);
                Arc::new(move || enabled.load(Ordering::SeqCst))
            },
            ..fake::default_redemption_settings()
        };
        let db = fake::create_e2e_store(E2eStoreOptions {
            catalog: fixture_catalog(),
            now: Arc::new(now_ms),
            redemption: Some(redemption),
        });
        let data = match &db {
            Db::Fake(data) => Arc::clone(data),
            Db::Pg(_) => panic!("create_e2e_store answers the in-memory store"),
        };
        StoreHarness {
            name: "fake store (in memory, db/fake.rs)",
            db,
            playable_ids: playable_ids(),
            token_ids: token_ids(),
            catalog_version: CATALOG_VERSION.to_string(),
            ends: Ends::Memory {
                data,
                unverified,
                redemption_enabled,
                next_user: AtomicU32::new(1),
            },
        }
    }

    // -----------------------------------------------------------------------
    // Postgres
    // -----------------------------------------------------------------------

    /// None when `DATABASE_URL` is unset: the Postgres half of the suite is skipped.
    pub async fn postgres() -> Option<StoreHarness> {
        let url = database_url()?;
        let admin = admin_client(&url).await;
        sqlx::raw_sql(TRUNCATE)
            .execute(&admin)
            .await
            .expect("truncate every table");
        seed_cards(&admin).await;
        let pool = PgPoolOptions::new()
            .connect(&url)
            .await
            .expect("connect the store to DATABASE_URL");
        Some(StoreHarness {
            name: "postgres store (db/pg.rs)",
            db: Db::Pg(pool),
            playable_ids: playable_ids(),
            token_ids: token_ids(),
            catalog_version: CATALOG_VERSION.to_string(),
            ends: Ends::Postgres { admin },
        })
    }

    /// Empties every table. Called before each test.
    pub async fn reset(&self) {
        match &self.ends {
            Ends::Memory {
                data,
                unverified,
                redemption_enabled,
                next_user,
            } => {
                data.lock().await.reset();
                unverified.lock().expect("the unverified profiles").clear();
                redemption_enabled.store(true, Ordering::SeqCst);
                next_user.store(1, Ordering::SeqCst);
            }
            Ends::Postgres { admin } => {
                sqlx::raw_sql(TRUNCATE)
                    .execute(admin)
                    .await
                    .expect("truncate every table");
                // `app.settings` is not truncated (migration 0001 seeds it once), so the redemption
                // switch is put back by hand rather than left flipped for whatever test runs next.
                sqlx::query(REDEMPTION_ENABLED_SQL)
                    .bind(true)
                    .execute(admin)
                    .await
                    .expect("turn redemption back on");
            }
        }
    }

    /// Provisions the managed-auth identity a profile needs and returns its user id (§9.4: "managed
    /// auth provider"). In Postgres that is an `auth.users` row, which `profiles.id` references.
    pub async fn new_user_id(&self, email: &str) -> String {
        match &self.ends {
            Ends::Memory { next_user, .. } => {
                let n = next_user.fetch_add(1, Ordering::SeqCst);
                format!("user-{n}")
            }
            Ends::Postgres { admin } => {
                let id: String = sqlx::query_scalar(
                    "insert into auth.users (email, email_confirmed_at) values ($1, now()) returning id::text",
                )
                .bind(email)
                .fetch_one(admin)
                .await
                .expect("auth.users insert returned no id");
                // Migration 0001's `on_auth_user_created` trigger has just made the pending profile
                // row. The contract exercises `profiles.create` itself — the path `resolveCaller`
                // takes for a user whose row is missing — so the trigger's row is removed here and
                // asserted separately in `postgres.rs`, where it belongs.
                sqlx::query("delete from public.profiles where id = $1::uuid")
                    .bind(&id)
                    .execute(admin)
                    .await
                    .expect("remove the trigger's profile row");
                id
            }
        }
    }

    /// §9.4 step 1's "verified email", which `Store.redeem` reads and no port method exposes: in
    /// Postgres `auth.users.email_confirmed_at`, in memory `RedemptionSettings.email_verified`.
    /// Every profile starts verified; this is how the contract reaches the other answer.
    pub async fn set_email_verified(&self, profile_id: &str, verified: bool) {
        match &self.ends {
            Ends::Memory { unverified, .. } => {
                let mut unverified = unverified.lock().expect("the unverified profiles");
                if verified {
                    unverified.shift_remove(profile_id);
                } else {
                    unverified.insert(profile_id.to_string());
                }
            }
            // §9.4 step 1's "verified email" is Supabase Auth's own `email_confirmed_at`, which is
            // what `app.redeem_invite_code` joins `auth.users` for.
            Ends::Postgres { admin } => {
                sqlx::query(
                    "update auth.users set email_confirmed_at = case when $2::boolean then now() else null end
                      where id = $1::uuid",
                )
                .bind(profile_id)
                .bind(verified)
                .execute(admin)
                .await
                .expect("set email_confirmed_at");
            }
        }
    }

    /// §9.4's database-side redemption switch — `app.settings.redemption_enabled` in Postgres, the
    /// `RedemptionSettings.enabled` hook in memory. `reset()` puts it back to true.
    pub async fn set_redemption_enabled(&self, enabled: bool) {
        match &self.ends {
            Ends::Memory {
                redemption_enabled, ..
            } => redemption_enabled.store(enabled, Ordering::SeqCst),
            Ends::Postgres { admin } => {
                sqlx::query(REDEMPTION_ENABLED_SQL)
                    .bind(enabled)
                    .execute(admin)
                    .await
                    .expect("set app.settings.redemption_enabled");
            }
        }
    }

    pub async fn close(self) {
        if let Db::Pg(pool) = &self.db {
            pool.close().await;
        }
        if let Ends::Postgres { admin } = &self.ends {
            admin.close().await;
        }
    }

    pub fn now(&self) -> i64 {
        now_ms()
    }
}

/// Every table the migrations create, children first. `cards` is seeded once and kept. The loadout
/// tables are no longer written by anything (R254) but are emptied all the same, so a test that
/// wrote one by hand leaves nothing behind.
pub const TRUNCATE: &str = "truncate
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
pub const REDEMPTION_ENABLED_SQL: &str =
    "update app.settings set value = to_jsonb($1::boolean) where key = 'redemption_enabled'";

/// The database these specs need: a real Postgres, which `test:db` (`crates/server/tests/db/run.sh`)
/// stands up in Docker, applies `tests/db/bootstrap.sql`, the migrations and `tests/db/grants.sql`
/// to, and then points `DATABASE_URL` at. Unset or empty, the Postgres half is skipped.
pub fn database_url() -> Option<String> {
    std::env::var("DATABASE_URL").ok().filter(|url| !url.is_empty())
}

/// A raw superuser connection, for the setup and teardown the store deliberately cannot do.
pub async fn admin_client(url: &str) -> PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .connect(url)
        .await
        .expect("connect the admin client to DATABASE_URL")
}

pub async fn seed_cards(admin: &PgPool) {
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

/// `Date.now()`.
fn now_ms() -> i64 {
    let since = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("the clock is after 1970");
    i64::try_from(since.as_millis()).expect("epoch milliseconds fit an i64")
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn must<T>(value: Option<T>, what: &str) -> T {
    value.unwrap_or_else(|| panic!("expected {what}"))
}

fn sorted(ids: &[String]) -> Vec<String> {
    let mut ids = ids.to_vec();
    ids.sort();
    ids
}

/// A port value from TS's object literal (its JSON shape, SURFACE §5.1).
fn from<T: DeserializeOwned>(value: Value) -> T {
    let shown = value.to_string();
    serde_json::from_value(value)
        .unwrap_or_else(|error| panic!("{} from {shown}: {error}", std::any::type_name::<T>()))
}

/// A port value's JSON, which is what the cases read fields off.
fn j<T: Serialize>(value: &T) -> Value {
    serde_json::to_value(value).expect("a port value serialises")
}

/// A string-union value's literal (`"ok"`, `"created"`, …).
fn lit<T: Serialize>(value: &T) -> String {
    must(j(value).as_str().map(str::to_string), "a string literal")
}

/// `{ ...base, ...over }`: `over`'s keys replace `base`'s, one level deep.
fn spread(base: Value, over: Value) -> Value {
    let mut base = base;
    if let (Some(fields), Value::Object(over)) = (base.as_object_mut(), over) {
        for (key, value) in over {
            fields.insert(key, value);
        }
    }
    base
}

/// `{ ...base, key: undefined }`.
fn without(base: Value, key: &str) -> Value {
    let mut base = base;
    if let Some(fields) = base.as_object_mut() {
        fields.remove(key);
    }
    base
}

/// TS's `toBeUndefined()` on an optional field: absent from the JSON (or, where a Rust type keeps
/// the key, null).
fn absent(value: &Value, key: &str) -> bool {
    matches!(value.get(key), None | Some(Value::Null))
}

fn ids_of<T: Serialize>(rows: &[T]) -> Vec<String> {
    rows.iter()
        .map(|row| must(j(row)["id"].as_str().map(str::to_string), "an id"))
        .collect()
}

fn status(literal: &str) -> ProfileStatus {
    from(json!(literal))
}

fn board_kind(literal: &str) -> LastBoardKind {
    from(json!(literal))
}

/// The deck at `slot` of a legal three-deck loadout over the fixture catalog (§9.4 L1, L2, L4).
fn deck_of(harness: &StoreHarness, slot: usize) -> Vec<String> {
    harness.playable_ids[slot * 20..slot * 20 + 20].to_vec()
}

fn clocks(now: i64) -> MatchClocks {
    from(json!({
        "turnDeadline": now + 75_000,
        "promptDeadline": null,
        "graceDeadline": { "p1": null, "p2": null },
        "ceilingAt": now + 3_600_000,
    }))
}

fn match_row(id: &str, p1: &str, p2: &str, harness: &StoreHarness, now: i64) -> MatchRow {
    from(json!({
        "id": id,
        "seed": "seed-1",
        "players": [p1, p2],
        "decks": [deck_of(harness, 0), deck_of(harness, 1)],
        "catalogVersion": harness.catalog_version,
        "status": "live",
        "createdAt": now,
        "finishedAt": null,
        "clocks": j(&clocks(now)),
    }))
}

/// A frozen trio over three disjoint fixture decks (R259).
fn frozen_trio(harness: &StoreHarness, name: Option<&str>) -> FrozenTrio {
    from(json!({
        "name": name.unwrap_or("Ladder"),
        "decks": [
            { "name": "Aggro", "cards": deck_of(harness, 0) },
            { "name": "Control", "cards": deck_of(harness, 1) },
            { "name": "Tempo", "cards": deck_of(harness, 2) },
        ],
    }))
}

fn action(player_id: &str, nonce: &str) -> Action {
    from(json!({ "type": "endTurn", "playerId": player_id, "nonce": nonce }))
}

fn action_row(match_id: &str, seq: i64, player_id: &str, nonce: &str, at: i64) -> MatchActionRow {
    from(json!({ "matchId": match_id, "seq": seq, "action": j(&action(player_id, nonce)), "at": at }))
}

/// uuids for the Postgres store, which types every id column as `uuid`.
fn id() -> String {
    uuid::Uuid::new_v4().to_string()
}

/// §9.4: the account exists the moment auth says so, and stays pending until a code is redeemed.
async fn pending_profile(harness: &StoreHarness, email: Option<&str>) -> Profile {
    let email = email
        .map(str::to_string)
        .unwrap_or_else(|| format!("{}@example.test", id()));
    let user_id = harness.new_user_id(&email).await;
    // Exactly what `resolveCaller` (src/api/http.ts) does.
    if let Some(existing) = q!(harness, t => t.profiles_get_by_user_id(&user_id)) {
        return existing;
    }
    let input: ProfileCreateInput =
        from(json!({ "userId": user_id, "email": email, "rating": 1000, "at": harness.now() }));
    q!(harness, t => t.profiles_create(&input))
}

async fn active_profile(harness: &StoreHarness, email: Option<&str>) -> Profile {
    let profile = pending_profile(harness, email).await;
    q!(harness, t => t.profiles_set_status(&profile.id, status("active")));
    must(
        q!(harness, t => t.profiles_get_by_id(&profile.id)),
        "the profile after activation",
    )
}

fn saved_deck(harness: &StoreHarness, profile_id: &str, over: Value) -> SavedDeck {
    let at = harness.now();
    from(spread(
        json!({
            "id": id(),
            "profileId": profile_id,
            "name": "Aggro",
            "cards": deck_of(harness, 0)[..7].to_vec(),
            "portrait": null,
            "catalogVersion": harness.catalog_version,
            "createdAt": at,
            "updatedAt": at,
        }),
        over,
    ))
}

fn saved_trio(harness: &StoreHarness, profile_id: &str, deck_ids: Value, over: Value) -> SavedTrio {
    let at = harness.now();
    from(spread(
        json!({
            "id": id(),
            "profileId": profile_id,
            "name": "Ladder",
            "deckIds": deck_ids,
            "createdAt": at,
            "updatedAt": at,
        }),
        over,
    ))
}

// ---------------------------------------------------------------------------
// Profiles (SPEC §9.4)
// ---------------------------------------------------------------------------

mod profiles {
    use super::*;

    async fn creates_a_pending_profile_and_reads_it_back_by_id_and_by_user_id(harness: &StoreHarness) {
        let email = "pending@example.test";
        let user_id = harness.new_user_id(email).await;
        let input: ProfileCreateInput =
            from(json!({ "userId": user_id, "email": email, "rating": 1000, "at": harness.now() }));
        let created = q!(harness, t => t.profiles_create(&input));

        let shown = j(&created);
        assert_eq!(shown["status"], "pending");
        assert_eq!(shown["rating"].as_f64(), Some(1000.0));
        assert!(shown["inMatchId"].is_null());
        assert_eq!(shown["email"], email);
        assert_eq!(shown["userId"], user_id.as_str());

        assert_eq!(
            q!(harness, t => t.profiles_get_by_id(&created.id)),
            Some(created.clone())
        );
        assert_eq!(
            q!(harness, t => t.profiles_get_by_user_id(&user_id)),
            Some(created)
        );
    }

    async fn returns_null_for_an_unknown_profile(harness: &StoreHarness) {
        assert!(q!(harness, t => t.profiles_get_by_id(&id())).is_none());
    }

    async fn reads_many_profiles_at_once(harness: &StoreHarness) {
        let a = pending_profile(harness, None).await;
        let b = pending_profile(harness, None).await;
        let many = q!(harness, t => t.profiles_get_many(&[a.id.clone(), b.id.clone(), id()]));
        assert_eq!(sorted(&ids_of(&many)), sorted(&[a.id.clone(), b.id.clone()]));
    }

    async fn moves_the_rating_and_the_in_match_pointer(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let match_id = id();
        let other = active_profile(harness, None).await;
        q!(harness, t => t.matches_create(&match_row(&match_id, &profile.id, &other.id, harness, harness.now())));

        // TS wrote the rating alone with `setRating`, which has no caller and is not ported
        // (SURFACE §11.2): the rating moves through `setGlicko`, at a new profile's deviation and
        // volatility (R603), which is the same row.
        let rating: Glicko = from(json!({ "rating": 1032, "deviation": 350, "volatility": 0.06 }));
        q!(harness, t => t.profiles_set_glicko(&profile.id, &rating));
        q!(harness, t => t.profiles_set_in_match(&profile.id, Some(match_id.as_str())));
        let rated = j(&must(
            q!(harness, t => t.profiles_get_by_id(&profile.id)),
            "the rated profile",
        ));
        assert_eq!(rated["rating"].as_f64(), Some(1032.0));
        assert_eq!(rated["inMatchId"], match_id.as_str());

        // §9.5: "Every ending ... clears both players' in-match state."
        q!(harness, t => t.profiles_set_in_match(&profile.id, None));
        assert!(
            j(&must(
                q!(harness, t => t.profiles_get_by_id(&profile.id)),
                "profile"
            ))["inMatchId"]
                .is_null()
        );
    }

    /// R603: a new profile starts at Glickman's deviation and volatility, and both move with it.
    async fn r603_round_trips_the_whole_glicko_triple(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let created = j(&must(
            q!(harness, t => t.profiles_get_by_id(&profile.id)),
            "the new profile",
        ));
        assert_eq!(created["rating"].as_f64(), Some(1000.0));
        assert_eq!(created["ratingDeviation"].as_f64(), Some(350.0));
        assert_eq!(created["ratingVolatility"].as_f64(), Some(0.06));

        let after: Glicko = from(json!({ "rating": 1016.25, "deviation": 330.5, "volatility": 0.059995 }));
        q!(harness, t => t.profiles_set_glicko(&profile.id, &after));
        let moved = j(&must(
            q!(harness, t => t.profiles_get_by_id(&profile.id)),
            "the rated profile",
        ));
        assert_eq!(moved["rating"].as_f64(), Some(1016.25));
        assert_eq!(moved["ratingDeviation"].as_f64(), Some(330.5));
        assert_eq!(moved["ratingVolatility"].as_f64(), Some(0.059995));

        // TS went on to check `setRating`, the rating-only write the old SQL paths used, leaving the
        // other two alone; it has no caller and is not ported (SURFACE §11.2), so neither is that
        // check.

        assert!(call!(harness, t => t.profiles_set_glicko(&id(), &after)).is_err());
    }

    /// R111, and the reason `e2e-store.ts` carries a trigger at all: "Becoming `active` grants one
    /// copy of every non-token card, written by a trigger on the `pending → active` transition and
    /// idempotent, so a repeated redemption cannot double a collection."
    async fn r111_grants_one_copy_of_every_non_token_card_on_pending_active_idempotently(
        harness: &StoreHarness,
    ) {
        let profile = pending_profile(harness, None).await;
        assert!(q!(harness, t => t.collection_get(&profile.id)).is_empty());

        q!(harness, t => t.profiles_set_status(&profile.id, status("active")));
        let granted = q!(harness, t => t.collection_get(&profile.id));
        let card_ids: Vec<String> = granted.iter().map(|entry| entry.card_id.clone()).collect();
        assert_eq!(sorted(&card_ids), sorted(&harness.playable_ids));
        assert!(granted.iter().all(|entry| entry.quantity == 1));
        for token_id in &harness.token_ids {
            assert!(!granted.iter().any(|entry| entry.card_id == *token_id));
        }

        q!(harness, t => t.profiles_set_status(&profile.id, status("active")));
        assert_eq!(q!(harness, t => t.collection_get(&profile.id)), granted);
    }

    both_stores!(
        creates_a_pending_profile_and_reads_it_back_by_id_and_by_user_id,
        returns_null_for_an_unknown_profile,
        reads_many_profiles_at_once,
        moves_the_rating_and_the_in_match_pointer,
        r603_round_trips_the_whole_glicko_triple,
        r111_grants_one_copy_of_every_non_token_card_on_pending_active_idempotently,
    );
}

// ---------------------------------------------------------------------------
// Invite codes (SPEC §9.4)
// ---------------------------------------------------------------------------

mod codes {
    use super::*;

    fn code(harness: &StoreHarness, over: Value) -> InviteCode {
        from(spread(
            json!({
                "id": id(),
                "codeHash": format!("hash-{}", id()),
                "maxUses": 1,
                "uses": 0,
                "revoked": false,
                "expiresAt": null,
                "createdAt": harness.now(),
            }),
            over,
        ))
    }

    async fn round_trips_a_code_by_its_hash(harness: &StoreHarness) {
        let row = code(harness, json!({}));
        q!(harness, t => t.codes_insert(&row));
        assert_eq!(
            q!(harness, t => t.codes_find_by_hash(&row.code_hash)),
            Some(row.clone())
        );
        assert!(q!(harness, t => t.codes_find_by_hash("no-such-hash")).is_none());
    }

    /// §9.4 step 6: "Two concurrent callers cannot both win the last use."
    async fn claims_a_single_use_code_exactly_once(harness: &StoreHarness) {
        let row = code(harness, json!({}));
        q!(harness, t => t.codes_insert(&row));
        assert!(q!(harness, t => t.codes_claim(&row.id, harness.now())));
        assert!(!q!(harness, t => t.codes_claim(&row.id, harness.now())));
        assert_eq!(
            must(q!(harness, t => t.codes_find_by_hash(&row.code_hash)), "the code").uses,
            1
        );
    }

    async fn refuses_a_revoked_or_expired_code(harness: &StoreHarness) {
        let revoked = code(harness, json!({ "revoked": true }));
        let expired = code(harness, json!({ "expiresAt": harness.now() - 1 }));
        q!(harness, t => t.codes_insert(&revoked));
        q!(harness, t => t.codes_insert(&expired));
        assert!(!q!(harness, t => t.codes_claim(&revoked.id, harness.now())));
        assert!(!q!(harness, t => t.codes_claim(&expired.id, harness.now())));
    }

    /// §9.4 steps 2-4 and the circuit breaker read exactly these three counters.
    async fn counts_attempts_by_profile_by_ip_hash_and_by_failure(harness: &StoreHarness) {
        let profile = pending_profile(harness, None).await;
        let now = harness.now();
        let ip_hash = format!("ip-{}", id());
        let attempts: [CodeAttempt; 3] = [
            from(
                json!({ "profileId": profile.id, "ipHash": ip_hash, "result": "rejected", "reason": "missing", "at": now - 10 }),
            ),
            from(
                json!({ "profileId": profile.id, "ipHash": ip_hash, "result": "ok", "reason": "redeemed", "at": now }),
            ),
            from(
                json!({ "profileId": null, "ipHash": "other", "result": "rejected", "reason": "missing", "at": now }),
            ),
        ];
        for attempt in &attempts {
            q!(harness, t => t.codes_log_attempt(attempt));
        }

        assert_eq!(
            q!(harness, t => t.codes_count_attempts_by_profile(&profile.id, now - 60_000)),
            2
        );
        assert_eq!(
            q!(harness, t => t.codes_count_attempts_by_profile(&profile.id, now + 1)),
            0
        );
        // R192: when the oldest counted attempt was made, so the status can say when it lapses.
        assert_eq!(
            q!(harness, t => t.codes_oldest_attempt_at_by_profile(&profile.id, now - 60_000)),
            Some(now - 10)
        );
        assert_eq!(
            q!(harness, t => t.codes_oldest_attempt_at_by_profile(&profile.id, now - 5)),
            Some(now)
        );
        assert_eq!(
            q!(harness, t => t.codes_oldest_attempt_at_by_profile(&profile.id, now + 1)),
            None
        );
        assert_eq!(
            q!(harness, t => t.codes_count_attempts_by_ip(&ip_hash, now - 60_000)),
            2
        );
        assert_eq!(q!(harness, t => t.codes_count_failures(now - 60_000)), 2);
    }

    both_stores!(
        round_trips_a_code_by_its_hash,
        claims_a_single_use_code_exactly_once,
        refuses_a_revoked_or_expired_code,
        counts_attempts_by_profile_by_ip_hash_and_by_failure,
    );
}

// ---------------------------------------------------------------------------
// Redemption (SPEC §9.4's one server-side transaction)
// ---------------------------------------------------------------------------

/// `Store.redeem` is the whole of §9.4's six steps, and the two implementations of it are as far
/// apart as this port gets: one `select app.redeem_invite_code(...)` against migration 0001's
/// plpgsql, and the fake's `createInMemoryRedeem` walking the same six steps over its tables.
/// Nothing above this block would notice if they disagreed — `api/codes.rs` calls the port and the
/// whole server suite runs on the fake — so every result code the port declares is exercised here,
/// against both.
mod redeem {
    use super::*;

    /// The hash is opaque to the store; what matters is that it is the same one both times.
    fn code_hash() -> String {
        format!("hash-{}", id())
    }

    /// A code's id and hash, as `mint` answers them.
    struct Minted {
        id: String,
        code_hash: String,
    }

    async fn mint(harness: &StoreHarness, over: Value) -> Minted {
        let row: InviteCode = from(json!({
            "id": id(),
            "codeHash": code_hash(),
            "maxUses": over.get("maxUses").cloned().unwrap_or(json!(1)),
            "uses": 0,
            "revoked": over.get("revoked").cloned().unwrap_or(json!(false)),
            "expiresAt": over.get("expiresAt").cloned().unwrap_or(Value::Null),
            "createdAt": harness.now(),
        }));
        q!(harness, t => t.codes_insert(&row));
        Minted {
            id: row.id.clone(),
            code_hash: row.code_hash.clone(),
        }
    }

    /// Attempts logged for this profile inside §9.4's window — step 4's evidence.
    async fn attempts(harness: &StoreHarness, profile_id: &str) -> i64 {
        let count = q!(harness, t => t.codes_count_attempts_by_profile(profile_id, harness.now() - 60_000));
        must(j(&count).as_i64(), "a count")
    }

    async fn uses_of(harness: &StoreHarness, hash: &str) -> i64 {
        let code = must(q!(harness, t => t.codes_find_by_hash(hash)), "the code");
        must(j(&code)["uses"].as_i64(), "the code's uses")
    }

    async fn status_of(harness: &StoreHarness, profile_id: &str) -> String {
        lit(&must(q!(harness, t => t.profiles_get_by_id(profile_id)), "the profile").status)
    }

    async fn redeem_with(
        harness: &StoreHarness,
        profile_id: &str,
        code_hash: Option<&str>,
        ip_hash: &str,
    ) -> String {
        let input: RedeemInviteCodeInput =
            from(json!({ "profileId": profile_id, "codeHash": code_hash, "ipHash": ip_hash }));
        lit(&q!(harness, t => t.redeem(&input)))
    }

    /// §9.4 step 6: "increment uses and set the account active, atomically."
    async fn activates_a_pending_account_consumes_one_use_and_logs_the_attempt(harness: &StoreHarness) {
        let profile = pending_profile(harness, None).await;
        let code = mint(harness, json!({})).await;

        assert_eq!(
            redeem_with(harness, &profile.id, Some(code.code_hash.as_str()), "ip-ok").await,
            "ok"
        );

        assert_eq!(status_of(harness, &profile.id).await, "active");
        assert_eq!(uses_of(harness, &code.code_hash).await, 1);
        assert_eq!(attempts(harness, &profile.id).await, 1);
        // R111's launch grant rides the pending → active transition, wherever it is made.
        assert_eq!(
            q!(harness, t => t.collection_get(&profile.id)).len(),
            harness.playable_ids.len()
        );
    }

    /// §9.4: "Missing, expired and exhausted codes return an identical error." Revoked joins them
    /// (step 5 names it) and so does a hash the caller has already established cannot be a code
    /// (`codeHash: null`) — the one result code means no caller can tell the five apart.
    async fn answers_missing_revoked_expired_exhausted_and_malformed_identically(harness: &StoreHarness) {
        let revoked = mint(harness, json!({ "revoked": true })).await;
        let expired = mint(harness, json!({ "expiresAt": harness.now() - 60_000 })).await;
        let exhausted = mint(harness, json!({})).await;
        assert!(q!(harness, t => t.codes_claim(&exhausted.id, harness.now())));

        let cases: [(&str, Option<String>); 5] = [
            ("missing", Some(code_hash())),
            ("revoked", Some(revoked.code_hash.clone())),
            ("expired", Some(expired.code_hash.clone())),
            ("exhausted", Some(exhausted.code_hash.clone())),
            ("malformed", None),
        ];

        for (name, hash) in &cases {
            let profile = pending_profile(harness, None).await;
            assert_eq!(
                redeem_with(harness, &profile.id, hash.as_deref(), &format!("ip-{name}")).await,
                "invalid_code",
                "{name}"
            );
            // Refused, but not for free: §9.4 logs the attempt (step 4) before it looks anything up.
            assert_eq!(status_of(harness, &profile.id).await, "pending", "{name}");
            assert_eq!(attempts(harness, &profile.id).await, 1, "{name}");
        }

        assert_eq!(uses_of(harness, &revoked.code_hash).await, 0);
        assert_eq!(uses_of(harness, &expired.code_hash).await, 0);
        assert_eq!(uses_of(harness, &exhausted.code_hash).await, 1);
    }

    /// §9.4 step 1, "reject unless the account is pending": one result for every account that is
    /// not. R145 makes `api/codes.rs` tell banned from already-active from unknown, which it does
    /// from the caller's own profile — the store cannot and does not.
    async fn refuses_an_account_that_is_not_pending_without_logging_an_attempt(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let code = mint(harness, json!({ "maxUses": 5 })).await;

        assert_eq!(
            redeem_with(harness, &profile.id, Some(code.code_hash.as_str()), "ip").await,
            "not_pending"
        );
        assert_eq!(uses_of(harness, &code.code_hash).await, 0);
        assert_eq!(attempts(harness, &profile.id).await, 0);
    }

    /// §9.4 step 1's other half: "with a verified email".
    async fn refuses_a_pending_account_whose_email_is_not_verified(harness: &StoreHarness) {
        let profile = pending_profile(harness, None).await;
        let code = mint(harness, json!({ "maxUses": 5 })).await;
        harness.set_email_verified(&profile.id, false).await;

        assert_eq!(
            redeem_with(harness, &profile.id, Some(code.code_hash.as_str()), "ip").await,
            "email_unverified"
        );
        assert_eq!(status_of(harness, &profile.id).await, "pending");
        assert_eq!(uses_of(harness, &code.code_hash).await, 0);
        assert_eq!(attempts(harness, &profile.id).await, 0);

        // And verifying it is all that stood in the way.
        harness.set_email_verified(&profile.id, true).await;
        assert_eq!(
            redeem_with(harness, &profile.id, Some(code.code_hash.as_str()), "ip").await,
            "ok"
        );
    }

    /// §9.4 step 2: "reject if this profile made more than 5 attempts in the last hour" — and step 2
    /// runs before step 4, so a caller already over the limit cannot pin their own counter by
    /// retrying.
    async fn rate_limits_a_profile_past_its_hourly_attempts_and_logs_nothing_more(harness: &StoreHarness) {
        let profile = pending_profile(harness, None).await;
        let code = mint(harness, json!({ "maxUses": 9 })).await;
        let at = harness.now();
        for _ in 0..6 {
            let attempt: CodeAttempt = from(json!({
                "profileId": profile.id,
                "ipHash": "ip-flood",
                "result": "rejected",
                "reason": "missing",
                "at": at,
            }));
            q!(harness, t => t.codes_log_attempt(&attempt));
        }

        assert_eq!(
            redeem_with(harness, &profile.id, Some(code.code_hash.as_str()), "ip-flood").await,
            "rate_limited_profile"
        );
        assert_eq!(attempts(harness, &profile.id).await, 6);
        assert_eq!(status_of(harness, &profile.id).await, "pending");
        assert_eq!(uses_of(harness, &code.code_hash).await, 0);
    }

    /// §9.4 step 3: "reject if this IP hash made more than 20" — a different window, per address.
    async fn rate_limits_an_ip_hash_past_its_hourly_attempts(harness: &StoreHarness) {
        let profile = pending_profile(harness, None).await;
        let code = mint(harness, json!({ "maxUses": 9 })).await;
        let ip_hash = format!("ip-{}", id());
        let at = harness.now();
        // Nobody's profile in particular: the neighbours behind one NAT, so step 2 stays at zero.
        for _ in 0..21 {
            let attempt: CodeAttempt = from(json!({
                "profileId": null,
                "ipHash": ip_hash,
                "result": "rejected",
                "reason": "missing",
                "at": at,
            }));
            q!(harness, t => t.codes_log_attempt(&attempt));
        }

        assert_eq!(attempts(harness, &profile.id).await, 0);
        assert_eq!(
            redeem_with(harness, &profile.id, Some(code.code_hash.as_str()), &ip_hash).await,
            "rate_limited_ip"
        );
        assert_eq!(status_of(harness, &profile.id).await, "pending");
    }

    /// §9.4: "A global circuit breaker disables redemption." This is the database's own switch,
    /// which the store answers `circuit_open` from; the server's R106 breaker (`api/codes.rs`) is a
    /// separate object that never lets a request reach here while it is open.
    async fn refuses_every_redemption_while_the_database_s_redemption_switch_is_off(harness: &StoreHarness) {
        let profile = pending_profile(harness, None).await;
        let code = mint(harness, json!({})).await;
        harness.set_redemption_enabled(false).await;

        assert_eq!(
            redeem_with(harness, &profile.id, Some(code.code_hash.as_str()), "ip").await,
            "circuit_open"
        );
        // A good code is refused too, unspent, and the attempt still costs the caller a row.
        assert_eq!(status_of(harness, &profile.id).await, "pending");
        assert_eq!(uses_of(harness, &code.code_hash).await, 0);
        assert_eq!(attempts(harness, &profile.id).await, 1);

        harness.set_redemption_enabled(true).await;
        assert_eq!(
            redeem_with(harness, &profile.id, Some(code.code_hash.as_str()), "ip").await,
            "ok"
        );
    }

    /// §9.4 step 6 again, from the other side: "Two concurrent callers cannot both win the last
    /// use" (ports.ts). One code, two accounts, one activation.
    async fn lets_a_single_use_code_activate_exactly_one_account(harness: &StoreHarness) {
        let first = pending_profile(harness, None).await;
        let second = pending_profile(harness, None).await;
        let code = mint(harness, json!({ "maxUses": 1 })).await;

        assert_eq!(
            redeem_with(harness, &first.id, Some(code.code_hash.as_str()), "ip-a").await,
            "ok"
        );
        assert_eq!(
            redeem_with(harness, &second.id, Some(code.code_hash.as_str()), "ip-b").await,
            "invalid_code"
        );

        assert_eq!(status_of(harness, &first.id).await, "active");
        assert_eq!(status_of(harness, &second.id).await, "pending");
        assert_eq!(uses_of(harness, &code.code_hash).await, 1);
    }

    /// R161: "a larger maximum stays available to whoever mints deliberately."
    async fn spends_a_multi_use_code_once_per_account_until_it_is_exhausted(harness: &StoreHarness) {
        let code = mint(harness, json!({ "maxUses": 2 })).await;
        let profiles = [
            pending_profile(harness, None).await,
            pending_profile(harness, None).await,
            pending_profile(harness, None).await,
        ];
        let mut results = Vec::new();
        for profile in &profiles {
            results.push(redeem_with(harness, &profile.id, Some(code.code_hash.as_str()), "ip").await);
        }

        assert_eq!(results, ["ok", "ok", "invalid_code"]);
        assert_eq!(uses_of(harness, &code.code_hash).await, 2);
    }

    both_stores!(
        activates_a_pending_account_consumes_one_use_and_logs_the_attempt,
        answers_missing_revoked_expired_exhausted_and_malformed_identically,
        refuses_an_account_that_is_not_pending_without_logging_an_attempt,
        refuses_a_pending_account_whose_email_is_not_verified,
        rate_limits_a_profile_past_its_hourly_attempts_and_logs_nothing_more,
        rate_limits_an_ip_hash_past_its_hourly_attempts,
        refuses_every_redemption_while_the_database_s_redemption_switch_is_off,
        lets_a_single_use_code_activate_exactly_one_account,
        spends_a_multi_use_code_once_per_account_until_it_is_exhausted,
    );
}

// ---------------------------------------------------------------------------
// Collection (SPEC §9.4's entitlement ledger)
// ---------------------------------------------------------------------------

mod collection {
    use super::*;

    fn entry(card_id: &str, quantity: i64) -> CollectionEntry {
        from(json!({ "cardId": card_id, "quantity": quantity }))
    }

    fn grant(profile_id: &str, card_id: &str, delta: i64, at: i64) -> CollectionGrant {
        from(
            json!({ "profileId": profile_id, "cardId": card_id, "delta": delta, "reason": "admin", "at": at }),
        )
    }

    async fn sets_absolute_quantities_and_appends_grants(harness: &StoreHarness) {
        let profile = pending_profile(harness, None).await;
        let first = must(harness.playable_ids.first().cloned(), "a card");
        let second = must(harness.playable_ids.get(1).cloned(), "a card");

        q!(harness, t => t.collection_upsert_quantities(&profile.id, &[entry(&first, 2), entry(&second, 1)]));
        q!(harness, t => t.collection_append_grants(&[
            grant(&profile.id, &first, 2, harness.now()),
            grant(&profile.id, &second, 1, harness.now()),
        ]));

        let owned = q!(harness, t => t.collection_get(&profile.id));
        let card_ids: Vec<String> = owned.iter().map(|entry| entry.card_id.clone()).collect();
        assert_eq!(sorted(&card_ids), sorted(&[first.clone(), second.clone()]));
        assert_eq!(
            must(
                owned.iter().find(|entry| entry.card_id == first),
                "the first card"
            )
            .quantity,
            2
        );

        // "SETS each card's quantity to the absolute value given; it does not add to it" (ports.ts).
        q!(harness, t => t.collection_upsert_quantities(&profile.id, &[entry(&first, 5)]));
        let after = q!(harness, t => t.collection_get(&profile.id));
        assert_eq!(
            must(
                after.iter().find(|entry| entry.card_id == first),
                "the first card"
            )
            .quantity,
            5
        );
    }

    /// §9.4: "Every collection change writes `collection` and `collection_grants` in one transaction."
    async fn rolls_both_ledger_writes_back_when_the_transaction_throws(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let card_id = must(harness.playable_ids.first().cloned(), "a card");
        let before = q!(harness, t => t.collection_get(&profile.id));

        let failed: Result<(), String> = async {
            let mut t = harness.db.begin(None).await.map_err(|error| error.to_string())?;
            t.collection_upsert_quantities(&profile.id, &[entry(&card_id, 9)])
                .await
                .map_err(|error| error.to_string())?;
            t.collection_append_grants(&[grant(&profile.id, &card_id, 8, harness.now())])
                .await
                .map_err(|error| error.to_string())?;
            // The fault: `t` is dropped here without a commit, which is the rollback.
            Err::<(), String>("fault injected after both writes".to_string())
        }
        .await;
        assert!(must(failed.err(), "the injected fault").contains("fault injected"));

        assert_eq!(q!(harness, t => t.collection_get(&profile.id)), before);
    }

    async fn commits_what_a_transaction_that_returns_wrote(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let card_id = must(harness.playable_ids.first().cloned(), "a card");
        let mut t = harness.db.begin(None).await.expect("begin");
        t.collection_upsert_quantities(&profile.id, &[entry(&card_id, 3)])
            .await
            .expect("upsert");
        t.commit().await.expect("commit");
        let owned = q!(harness, t => t.collection_get(&profile.id));
        assert_eq!(
            must(owned.iter().find(|entry| entry.card_id == card_id), "the card").quantity,
            3
        );
    }

    both_stores!(
        sets_absolute_quantities_and_appends_grants,
        rolls_both_ledger_writes_back_when_the_transaction_throws,
        commits_what_a_transaction_that_returns_wrote,
    );
}

// ---------------------------------------------------------------------------
// Saved decks and trios (SPEC §9.4, R250, R252, R256)
// ---------------------------------------------------------------------------

mod decks {
    use super::*;

    async fn r250_lists_nothing_before_anything_is_saved(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        assert!(q!(harness, t => t.decks_list(&profile.id)).is_empty());
        assert!(q!(harness, t => t.decks_get(&id())).is_none());
    }

    async fn r250_creates_a_draft_deck_and_reads_it_back_exactly_card_order_included(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let mut cards = deck_of(harness, 0)[..5].to_vec();
        cards.reverse();
        let deck = saved_deck(harness, &profile.id, json!({ "cards": cards }));
        assert_eq!(lit(&q!(harness, t => t.decks_upsert(&deck, 10))), "created");
        assert_eq!(q!(harness, t => t.decks_get(&deck.id)), Some(deck.clone()));
        assert_eq!(q!(harness, t => t.decks_list(&profile.id)), vec![deck]);
    }

    async fn r641_round_trips_the_portrait_null_and_a_known_id_and_re_saves_it_in_place(
        harness: &StoreHarness,
    ) {
        let profile = active_profile(harness, None).await;
        let portraitless = saved_deck(harness, &profile.id, json!({}));
        // One tick later, so "oldest first" below asks a real ordering question, not a tie.
        let pictured = saved_deck(
            harness,
            &profile.id,
            json!({
                "portrait": "gary",
                "createdAt": harness.now() + 1_000,
                "updatedAt": harness.now() + 1_000,
            }),
        );
        // PREMISE: `saved_deck` really does hold `null` — the pre-portrait default — not undefined.
        assert!(j(&portraitless)["portrait"].is_null());
        assert_ne!(portraitless, pictured);

        assert_eq!(
            lit(&q!(harness, t => t.decks_upsert(&portraitless, 10))),
            "created"
        );
        assert_eq!(lit(&q!(harness, t => t.decks_upsert(&pictured, 10))), "created");
        assert_eq!(
            q!(harness, t => t.decks_get(&portraitless.id)),
            Some(portraitless.clone())
        );
        assert_eq!(
            q!(harness, t => t.decks_get(&pictured.id)),
            Some(pictured.clone())
        );
        let portraits: Vec<Value> = q!(harness, t => t.decks_list(&profile.id))
            .iter()
            .map(|deck| j(deck)["portrait"].clone())
            .collect();
        assert_eq!(portraits, vec![Value::Null, json!("gary")]);

        // An update swaps the field like any other: `null` back to a choice and back again.
        let changed: SavedDeck = from(spread(
            j(&pictured),
            json!({ "portrait": null, "updatedAt": harness.now() + 1_000 }),
        ));
        assert_eq!(lit(&q!(harness, t => t.decks_upsert(&changed, 10))), "updated");
        let expected: SavedDeck = from(spread(
            j(&changed),
            json!({ "createdAt": j(&pictured)["createdAt"] }),
        ));
        assert_eq!(q!(harness, t => t.decks_get(&pictured.id)), Some(expected));
    }

    async fn r256_updates_the_name_cards_and_version_in_place_and_keeps_created_at(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let deck = saved_deck(harness, &profile.id, json!({}));
        q!(harness, t => t.decks_upsert(&deck, 10));
        let shown = j(&deck);
        let created_at = must(shown["createdAt"].as_i64(), "createdAt");
        let updated_at = must(shown["updatedAt"].as_i64(), "updatedAt");
        let edited: SavedDeck = from(spread(
            shown.clone(),
            json!({
                "name": "Aggro v2",
                "cards": deck_of(harness, 1),
                "createdAt": created_at + 99_000,
                "updatedAt": updated_at + 5_000,
            }),
        ));
        assert_eq!(lit(&q!(harness, t => t.decks_upsert(&edited, 10))), "updated");
        let expected: SavedDeck = from(spread(j(&edited), json!({ "createdAt": created_at })));
        assert_eq!(q!(harness, t => t.decks_get(&deck.id)), Some(expected));
    }

    async fn r250_lists_a_profile_s_decks_oldest_first_and_only_its_own(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let now = harness.now();
        let second = saved_deck(
            harness,
            &a.id,
            json!({ "name": "Second", "createdAt": now + 2, "updatedAt": now + 2 }),
        );
        let first = saved_deck(
            harness,
            &a.id,
            json!({ "name": "First", "createdAt": now + 1, "updatedAt": now + 1 }),
        );
        q!(harness, t => t.decks_upsert(&second, 10));
        q!(harness, t => t.decks_upsert(&first, 10));
        q!(harness, t => t.decks_upsert(&saved_deck(harness, &b.id, json!({})), 10));
        let names: Vec<Value> = q!(harness, t => t.decks_list(&a.id))
            .iter()
            .map(|deck| j(deck)["name"].clone())
            .collect();
        assert_eq!(names, vec![json!("First"), json!("Second")]);
    }

    async fn r250_refuses_a_create_past_the_cap_and_still_updates_at_the_cap(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let decks = [
            saved_deck(harness, &profile.id, json!({})),
            saved_deck(harness, &profile.id, json!({})),
        ];
        for deck in &decks {
            assert_eq!(lit(&q!(harness, t => t.decks_upsert(deck, 2))), "created");
        }
        assert_eq!(
            lit(&q!(harness, t => t.decks_upsert(&saved_deck(harness, &profile.id, json!({})), 2))),
            "limit"
        );
        assert_eq!(q!(harness, t => t.decks_list(&profile.id)).len(), 2);
        let renamed: SavedDeck = from(spread(j(&decks[0]), json!({ "name": "Renamed" })));
        assert_eq!(lit(&q!(harness, t => t.decks_upsert(&renamed, 2))), "updated");
    }

    async fn r256_refuses_to_overwrite_another_profile_s_deck_whatever_the_id(harness: &StoreHarness) {
        let owner = active_profile(harness, None).await;
        let other = active_profile(harness, None).await;
        let deck = saved_deck(harness, &owner.id, json!({}));
        q!(harness, t => t.decks_upsert(&deck, 10));
        let theirs: SavedDeck = from(spread(
            j(&deck),
            json!({ "profileId": other.id, "name": "Mine now" }),
        ));
        assert_eq!(lit(&q!(harness, t => t.decks_upsert(&theirs, 10))), "not_owner");
        assert_eq!(
            j(&must(q!(harness, t => t.decks_get(&deck.id)), "the deck"))["name"],
            "Aggro"
        );
        assert!(!q!(harness, t => t.decks_remove(&other.id, &deck.id)));
    }

    async fn r252_removes_a_deck_and_empties_every_trio_slot_that_named_it(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let x = saved_deck(harness, &profile.id, json!({}));
        let y = saved_deck(harness, &profile.id, json!({}));
        q!(harness, t => t.decks_upsert(&x, 10));
        q!(harness, t => t.decks_upsert(&y, 10));
        let trio = saved_trio(harness, &profile.id, json!([x.id, null, y.id]), json!({}));
        assert_eq!(lit(&q!(harness, t => t.trios_upsert(&trio, 5))), "created");

        assert!(q!(harness, t => t.decks_remove(&profile.id, &x.id)));
        assert!(q!(harness, t => t.decks_get(&x.id)).is_none());
        assert_eq!(
            j(&must(q!(harness, t => t.trios_get(&trio.id)), "the trio"))["deckIds"],
            json!([null, null, y.id])
        );
        assert!(!q!(harness, t => t.decks_remove(&profile.id, &x.id)));
    }

    both_stores!(
        r250_lists_nothing_before_anything_is_saved,
        r250_creates_a_draft_deck_and_reads_it_back_exactly_card_order_included,
        r641_round_trips_the_portrait_null_and_a_known_id_and_re_saves_it_in_place,
        r256_updates_the_name_cards_and_version_in_place_and_keeps_created_at,
        r250_lists_a_profile_s_decks_oldest_first_and_only_its_own,
        r250_refuses_a_create_past_the_cap_and_still_updates_at_the_cap,
        r256_refuses_to_overwrite_another_profile_s_deck_whatever_the_id,
        r252_removes_a_deck_and_empties_every_trio_slot_that_named_it,
    );
}

mod trios {
    use super::*;

    async fn three_decks(harness: &StoreHarness, profile_id: &str) -> [String; 3] {
        let decks = [
            saved_deck(harness, profile_id, json!({})),
            saved_deck(harness, profile_id, json!({})),
            saved_deck(harness, profile_id, json!({})),
        ];
        for deck in &decks {
            q!(harness, t => t.decks_upsert(deck, 10));
        }
        [decks[0].id.clone(), decks[1].id.clone(), decks[2].id.clone()]
    }

    async fn r252_creates_a_trio_with_empty_slots_and_reads_it_back(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let trio = saved_trio(harness, &profile.id, json!([null, null, null]), json!({}));
        assert_eq!(lit(&q!(harness, t => t.trios_upsert(&trio, 5))), "created");
        assert_eq!(q!(harness, t => t.trios_get(&trio.id)), Some(trio.clone()));
        assert_eq!(q!(harness, t => t.trios_list(&profile.id)), vec![trio]);
    }

    async fn r252_updates_slots_and_name_in_place(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let ids = three_decks(harness, &profile.id).await;
        let trio = saved_trio(harness, &profile.id, json!([ids[0], null, null]), json!({}));
        q!(harness, t => t.trios_upsert(&trio, 5));
        let updated_at = must(j(&trio)["updatedAt"].as_i64(), "updatedAt");
        let edited: SavedTrio = from(spread(
            j(&trio),
            json!({ "name": "Full", "deckIds": ids, "updatedAt": updated_at + 1 }),
        ));
        assert_eq!(lit(&q!(harness, t => t.trios_upsert(&edited, 5))), "updated");
        assert_eq!(q!(harness, t => t.trios_get(&trio.id)), Some(edited));
    }

    async fn r252_refuses_a_slot_naming_a_deck_that_is_not_this_profile_s(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let [theirs, _, _] = three_decks(harness, &b.id).await;
        let borrowed = saved_trio(harness, &a.id, json!([theirs, null, null]), json!({}));
        assert_eq!(
            lit(&q!(harness, t => t.trios_upsert(&borrowed, 5))),
            "unknown_deck"
        );
        let invented = saved_trio(harness, &a.id, json!([id(), null, null]), json!({}));
        assert_eq!(
            lit(&q!(harness, t => t.trios_upsert(&invented, 5))),
            "unknown_deck"
        );
        assert!(q!(harness, t => t.trios_list(&a.id)).is_empty());
    }

    async fn r252_refuses_one_deck_in_two_slots(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let [x, _, _] = three_decks(harness, &profile.id).await;
        let twice = saved_trio(harness, &profile.id, json!([x, x, null]), json!({}));
        assert!(call!(harness, t => t.trios_upsert(&twice, 5)).is_err());
        assert!(q!(harness, t => t.trios_list(&profile.id)).is_empty());
    }

    async fn r252_refuses_a_create_past_the_cap_and_another_profile_s_trio(harness: &StoreHarness) {
        let owner = active_profile(harness, None).await;
        let other = active_profile(harness, None).await;
        let trio = saved_trio(harness, &owner.id, json!([null, null, null]), json!({}));
        assert_eq!(lit(&q!(harness, t => t.trios_upsert(&trio, 1))), "created");
        let second = saved_trio(harness, &owner.id, json!([null, null, null]), json!({}));
        assert_eq!(lit(&q!(harness, t => t.trios_upsert(&second, 1))), "limit");
        let theirs: SavedTrio = from(spread(j(&trio), json!({ "profileId": other.id })));
        assert_eq!(lit(&q!(harness, t => t.trios_upsert(&theirs, 5))), "not_owner");
        assert!(!q!(harness, t => t.trios_remove(&other.id, &trio.id)));
        assert!(q!(harness, t => t.trios_remove(&owner.id, &trio.id)));
        assert!(q!(harness, t => t.trios_get(&trio.id)).is_none());
    }

    both_stores!(
        r252_creates_a_trio_with_empty_slots_and_reads_it_back,
        r252_updates_slots_and_name_in_place,
        r252_refuses_a_slot_naming_a_deck_that_is_not_this_profile_s,
        r252_refuses_one_deck_in_two_slots,
        r252_refuses_a_create_past_the_cap_and_another_profile_s_trio,
    );
}

// ---------------------------------------------------------------------------
// Tutorial progress on the account (SPEC §9.10, R320)
// ---------------------------------------------------------------------------

mod tutorial {
    use super::*;

    /// A whole-millisecond instant, as the handler hands the store (and as Postgres keeps it).
    fn instant(harness: &StoreHarness, offset: i64) -> i64 {
        harness.now() + offset
    }

    fn merged(outcome: Value) -> TutorialProgressRow {
        if outcome["kind"] != "merged" {
            panic!("expected merged, got {}", outcome["kind"]);
        }
        from(outcome["progress"].clone())
    }

    async fn merge(harness: &StoreHarness, input: Value, max_lessons: usize) -> Value {
        let input: TutorialMergeInput = from(input);
        j(&q!(harness, t => t.tutorial_merge(&input, max_lessons as i64)))
    }

    async fn r320_holds_no_row_before_the_first_write_and_the_first_write_makes_one(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        assert!(q!(harness, t => t.tutorial_get(&profile.id)).is_none());

        let progress = merged(
            merge(
                harness,
                json!({
                    "profileId": profile.id,
                    "completed": ["spells", "basics"],
                    "hiddenChoice": null,
                    "at": instant(harness, 0),
                }),
                32,
            )
            .await,
        );
        // Each id once, in code-point order, whatever order they were sent in.
        assert_eq!(
            j(&progress),
            json!({ "profileId": profile.id, "completed": ["basics", "spells"], "hiddenChoice": null })
        );
        assert_eq!(q!(harness, t => t.tutorial_get(&profile.id)), Some(progress));
    }

    async fn r320_unions_the_lessons_a_write_never_removes_one_and_the_same_write_twice_changes_nothing(
        harness: &StoreHarness,
    ) {
        let profile = active_profile(harness, None).await;
        let write = |completed: Value| {
            merge(
                harness,
                json!({ "profileId": profile.id, "completed": completed, "hiddenChoice": null, "at": instant(harness, 0) }),
                32,
            )
        };
        let completed = |row: TutorialProgressRow| j(&row)["completed"].clone();

        merged(write(json!(["basics", "spells"])).await);
        // A stale device that has won only lesson 1, or nothing at all, takes nothing away.
        assert_eq!(
            completed(merged(write(json!(["basics"])).await)),
            json!(["basics", "spells"])
        );
        assert_eq!(
            completed(merged(write(json!([])).await)),
            json!(["basics", "spells"])
        );
        // Another device's lesson joins them; a repeat of it is the same row.
        assert_eq!(
            completed(merged(write(json!(["traps"])).await)),
            json!(["basics", "spells", "traps"])
        );
        assert_eq!(
            completed(merged(write(json!(["traps", "traps"])).await)),
            json!(["basics", "spells", "traps"])
        );
        // An id no lesson of this client has is kept all the same: the server does not know the lessons.
        assert_eq!(
            completed(merged(write(json!(["lesson-from-a-newer-client"])).await)),
            json!(["basics", "lesson-from-a-newer-client", "spells", "traps"])
        );
        let row = j(&must(q!(harness, t => t.tutorial_get(&profile.id)), "the row"));
        assert_eq!(row["completed"].as_array().map(Vec::len), Some(4));
    }

    async fn r320_keeps_the_newest_hide_show_choice_an_older_one_never_replaces_it_and_a_tie_keeps_the_stored(
        harness: &StoreHarness,
    ) {
        let profile = active_profile(harness, None).await;
        let t0 = instant(harness, 0);
        let choose = |hidden: bool, at: i64| {
            merge(
                harness,
                json!({ "profileId": profile.id, "completed": [], "hiddenChoice": { "hidden": hidden, "at": at }, "at": t0 }),
                32,
            )
        };
        let choice = |row: TutorialProgressRow| j(&row)["hiddenChoice"].clone();

        assert_eq!(
            choice(merged(choose(true, t0).await)),
            json!({ "hidden": true, "at": t0 })
        );
        // "Show" made later, on another device, wins.
        assert_eq!(
            choice(merged(choose(false, t0 + 5_000).await)),
            json!({ "hidden": false, "at": t0 + 5_000 })
        );
        // An older "Hide" arriving afterwards does not undo it.
        assert_eq!(
            choice(merged(choose(true, t0 + 1_000).await)),
            json!({ "hidden": false, "at": t0 + 5_000 })
        );
        // The same instant is not newer: the stored choice stays.
        assert_eq!(
            choice(merged(choose(true, t0 + 5_000).await)),
            json!({ "hidden": false, "at": t0 + 5_000 })
        );
        // A write with no choice leaves the choice alone, and lessons alone move nothing else.
        let lessons_only = merged(
            merge(
                harness,
                json!({ "profileId": profile.id, "completed": ["basics"], "hiddenChoice": null, "at": t0 + 9_000 }),
                32,
            )
            .await,
        );
        assert_eq!(
            j(&lessons_only),
            json!({
                "profileId": profile.id,
                "completed": ["basics"],
                "hiddenChoice": { "hidden": false, "at": t0 + 5_000 },
            })
        );
    }

    async fn r320_refuses_a_union_past_the_cap_and_writes_nothing_and_keeps_each_profile_s_row_apart(
        harness: &StoreHarness,
    ) {
        let owner = active_profile(harness, None).await;
        let other = active_profile(harness, None).await;
        let write = |profile_id: String, completed: Value| {
            merge(
                harness,
                json!({ "profileId": profile_id, "completed": completed, "hiddenChoice": null, "at": instant(harness, 0) }),
                3,
            )
        };
        let completed_of =
            |row: Option<TutorialProgressRow>| j(&must(row, "the owner's row"))["completed"].clone();

        merged(write(owner.id.clone(), json!(["basics", "spells"])).await);
        assert_eq!(
            write(owner.id.clone(), json!(["traps", "advanced"])).await,
            json!({ "kind": "limit" })
        );
        assert_eq!(
            completed_of(q!(harness, t => t.tutorial_get(&owner.id))),
            json!(["basics", "spells"])
        );
        // At the cap exactly is fine.
        assert_eq!(
            j(&merged(write(owner.id.clone(), json!(["traps"])).await))["completed"],
            json!(["basics", "spells", "traps"])
        );

        assert!(q!(harness, t => t.tutorial_get(&other.id)).is_none());
        assert_eq!(
            j(&merged(write(other.id.clone(), json!(["advanced"])).await))["completed"],
            json!(["advanced"])
        );
        assert_eq!(
            completed_of(q!(harness, t => t.tutorial_get(&owner.id))),
            json!(["basics", "spells", "traps"])
        );
    }

    both_stores!(
        r320_holds_no_row_before_the_first_write_and_the_first_write_makes_one,
        r320_unions_the_lessons_a_write_never_removes_one_and_the_same_write_twice_changes_nothing,
        r320_keeps_the_newest_hide_show_choice_an_older_one_never_replaces_it_and_a_tie_keeps_the_stored,
        r320_refuses_a_union_past_the_cap_and_writes_nothing_and_keeps_each_profile_s_row_apart,
    );
}

mod player_settings {
    use super::*;

    /// A whole-millisecond instant, as the handler hands the store (and as Postgres keeps it).
    fn instant(harness: &StoreHarness, offset: i64) -> i64 {
        harness.now() + offset
    }

    /// `{ maxGroups: 8, maxBytes: 4096 }`.
    fn limits() -> Value {
        json!({ "maxGroups": 8, "maxBytes": 4096 })
    }

    fn merged(outcome: Value) -> PlayerSettingsRow {
        if outcome["kind"] != "merged" {
            panic!("expected merged, got {}", outcome["kind"]);
        }
        from(outcome["settings"].clone())
    }

    async fn write(harness: &StoreHarness, profile_id: &str, groups: Value, limits: Value) -> Value {
        let input: PlayerSettingsMergeInput =
            from(json!({ "profileId": profile_id, "groups": groups, "at": instant(harness, 0) }));
        let limits: PlayerSettingsLimits = from(limits);
        j(&q!(harness, t => t.player_settings_merge(&input, &limits)))
    }

    fn group_of(row: &PlayerSettingsRow, group: &str) -> Value {
        j(row)["groups"][group].clone()
    }

    fn group_ids(row: &PlayerSettingsRow) -> Vec<String> {
        must(
            j(row)["groups"]
                .as_object()
                .map(|groups| groups.keys().cloned().collect()),
            "the groups",
        )
    }

    async fn r633_holds_no_row_before_the_first_write_and_the_first_write_makes_one(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        assert!(q!(harness, t => t.player_settings_get(&profile.id)).is_none());

        let row = merged(
            write(
                harness,
                &profile.id,
                json!({
                    "audio": { "at": 1_000, "values": { "master": 0.5, "muted": false, "station": "tavern" } },
                    "gameplay": { "at": 1_000, "values": { "dragToPlay": true } },
                }),
                limits(),
            )
            .await,
        );
        assert_eq!(
            j(&row),
            json!({
                "profileId": profile.id,
                "groups": {
                    "audio": { "at": 1_000, "values": { "master": 0.5, "muted": false, "station": "tavern" } },
                    "gameplay": { "at": 1_000, "values": { "dragToPlay": true } },
                },
            })
        );
        assert_eq!(q!(harness, t => t.player_settings_get(&profile.id)), Some(row));
    }

    async fn r634_a_strictly_later_group_replaces_the_stored_one_whole_an_older_or_tied_one_changes_nothing(
        harness: &StoreHarness,
    ) {
        let profile = active_profile(harness, None).await;
        merged(
            write(
                harness,
                &profile.id,
                json!({ "audio": { "at": 2_000, "values": { "master": 0.5, "muted": true } } }),
                limits(),
            )
            .await,
        );

        // Later: replaced whole, so a key the new group lacks is gone.
        let later = merged(
            write(
                harness,
                &profile.id,
                json!({ "audio": { "at": 3_000, "values": { "master": 0.9 } } }),
                limits(),
            )
            .await,
        );
        assert_eq!(
            group_of(&later, "audio"),
            json!({ "at": 3_000, "values": { "master": 0.9 } })
        );
        // Older, and tied: the stored group stays.
        let older = merged(
            write(
                harness,
                &profile.id,
                json!({ "audio": { "at": 2_500, "values": { "master": 0.1 } } }),
                limits(),
            )
            .await,
        );
        assert_eq!(
            group_of(&older, "audio"),
            json!({ "at": 3_000, "values": { "master": 0.9 } })
        );
        let tied = merged(
            write(
                harness,
                &profile.id,
                json!({ "audio": { "at": 3_000, "values": { "master": 0.2 } } }),
                limits(),
            )
            .await,
        );
        assert_eq!(
            group_of(&tied, "audio"),
            json!({ "at": 3_000, "values": { "master": 0.9 } })
        );
        // Zero is a time like any other.
        let zero = merged(
            write(
                harness,
                &profile.id,
                json!({ "fx": { "at": 0, "values": { "speed": 1 } } }),
                limits(),
            )
            .await,
        );
        assert_eq!(
            group_of(&zero, "fx"),
            json!({ "at": 0, "values": { "speed": 1 } })
        );
    }

    async fn r634_a_group_a_write_does_not_name_stays_and_one_write_can_win_one_group_and_lose_another(
        harness: &StoreHarness,
    ) {
        let profile = active_profile(harness, None).await;
        merged(
            write(
                harness,
                &profile.id,
                json!({
                    "gameplay": { "at": 5_000, "values": { "dragToPlay": false } },
                    "audio": { "at": 1_000, "values": { "master": 0.4 } },
                }),
                limits(),
            )
            .await,
        );

        let row = merged(
            write(
                harness,
                &profile.id,
                json!({
                    "audio": { "at": 2_000, "values": { "master": 0.7 } },
                    "gameplay": { "at": 4_000, "values": { "dragToPlay": true } },
                    "cards": { "at": 1, "values": { "animatedFoil": false } },
                }),
                limits(),
            )
            .await,
        );
        assert_eq!(
            j(&row)["groups"],
            json!({
                "gameplay": { "at": 5_000, "values": { "dragToPlay": false } },
                "audio": { "at": 2_000, "values": { "master": 0.7 } },
                "cards": { "at": 1, "values": { "animatedFoil": false } },
            })
        );
        // A write naming nothing is the row as it stands.
        assert_eq!(
            merged(write(harness, &profile.id, json!({}), limits()).await),
            row
        );
    }

    async fn r633_refuses_a_result_past_either_cap_and_writes_nothing_and_keeps_each_profile_s_row_apart(
        harness: &StoreHarness,
    ) {
        let owner = active_profile(harness, None).await;
        let other = active_profile(harness, None).await;
        let small = json!({ "maxGroups": 2, "maxBytes": 4096 });

        merged(
            write(
                harness,
                &owner.id,
                json!({ "a": { "at": 1, "values": { "x": true } }, "b": { "at": 1, "values": { "x": true } } }),
                small.clone(),
            )
            .await,
        );
        assert_eq!(
            write(
                harness,
                &owner.id,
                json!({ "c": { "at": 9, "values": { "x": true } } }),
                small.clone()
            )
            .await,
            json!({ "kind": "limit" })
        );
        let owners = must(
            q!(harness, t => t.player_settings_get(&owner.id)),
            "the owner's row",
        );
        assert_eq!(group_ids(&owners), ["a", "b"]);
        // A group already held may still be replaced at the cap.
        let replaced = merged(
            write(
                harness,
                &owner.id,
                json!({ "a": { "at": 9, "values": { "x": false } } }),
                small,
            )
            .await,
        );
        assert_eq!(
            group_of(&replaced, "a"),
            json!({ "at": 9, "values": { "x": false } })
        );

        // The byte cap counts the text the stored groups come to; keep well clear of the boundary.
        let text = "x".repeat(40);
        let wide: serde_json::Map<String, Value> =
            (0..12).map(|at| (format!("key{at}"), json!(text))).collect();
        assert_eq!(
            write(
                harness,
                &other.id,
                json!({ "big": { "at": 1, "values": wide } }),
                json!({ "maxGroups": 8, "maxBytes": 300 })
            )
            .await,
            json!({ "kind": "limit" })
        );
        assert!(q!(harness, t => t.player_settings_get(&other.id)).is_none());
        let big = merged(
            write(
                harness,
                &other.id,
                json!({ "big": { "at": 1, "values": wide } }),
                json!({ "maxGroups": 8, "maxBytes": 4096 }),
            )
            .await,
        );
        assert_eq!(group_of(&big, "big"), json!({ "at": 1, "values": wide }));
        let owners = must(
            q!(harness, t => t.player_settings_get(&owner.id)),
            "the owner's row",
        );
        assert_eq!(group_ids(&owners), ["a", "b"]);
    }

    both_stores!(
        r633_holds_no_row_before_the_first_write_and_the_first_write_makes_one,
        r634_a_strictly_later_group_replaces_the_stored_one_whole_an_older_or_tied_one_changes_nothing,
        r634_a_group_a_write_does_not_name_stays_and_one_write_can_win_one_group_and_lose_another,
        r633_refuses_a_result_past_either_cap_and_writes_nothing_and_keeps_each_profile_s_row_apart,
    );
}

// ---------------------------------------------------------------------------
// The Best-of-3 series (SPEC §9.5, R259–R263)
// ---------------------------------------------------------------------------

mod last_boards {
    use super::*;

    fn board() -> Vec<LastBoardEntry> {
        from(json!([
            { "defId": "core-012", "radiant": false },
            { "defId": "t-1:core-012+core-025", "radiant": true },
        ]))
    }

    async fn r565_holds_no_board_before_a_profile_s_first_finished_game_then_the_one_written(
        harness: &StoreHarness,
    ) {
        let profile = active_profile(harness, None).await;
        assert!(q!(harness, t => t.last_boards_get(&profile.id, board_kind("server"))).is_none());
        q!(harness, t => t.last_boards_put(&profile.id, board_kind("server"), &board(), harness.now()));
        assert_eq!(
            q!(harness, t => t.last_boards_get(&profile.id, board_kind("server"))),
            Some(board())
        );
    }

    async fn r565_a_later_game_s_board_replaces_it_and_each_kind_is_its_own(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        q!(harness, t => t.last_boards_put(&profile.id, board_kind("server"), &board(), harness.now()));
        q!(harness, t => t.last_boards_put(&profile.id, board_kind("server"), &[], harness.now()));
        assert_eq!(
            q!(harness, t => t.last_boards_get(&profile.id, board_kind("server"))),
            Some(Vec::new())
        );
        q!(harness, t => t.last_boards_put(&profile.id, board_kind("practice"), &board(), harness.now()));
        assert_eq!(
            q!(harness, t => t.last_boards_get(&profile.id, board_kind("server"))),
            Some(Vec::new())
        );
        assert_eq!(
            q!(harness, t => t.last_boards_get(&profile.id, board_kind("practice"))),
            Some(board())
        );
        let other = active_profile(harness, None).await;
        assert!(q!(harness, t => t.last_boards_get(&other.id, board_kind("practice"))).is_none());
    }

    async fn r565_goes_with_the_profile(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        q!(harness, t => t.last_boards_put(&profile.id, board_kind("server"), &board(), harness.now()));
        assert!(q!(harness, t => t.profiles_remove(&profile.id)));
        assert!(q!(harness, t => t.last_boards_get(&profile.id, board_kind("server"))).is_none());
    }

    async fn r417_a_match_row_keeps_the_boards_it_started_with(harness: &StoreHarness) {
        let p1 = active_profile(harness, None).await;
        let p2 = active_profile(harness, None).await;
        let row: MatchRow = from(spread(
            j(&match_row(&id(), &p1.id, &p2.id, harness, harness.now())),
            json!({ "lastBoards": [j(&board()), []] }),
        ));
        q!(harness, t => t.matches_create(&row));
        assert_eq!(q!(harness, t => t.matches_get(&row.id)), Some(row.clone()));
        let live = q!(harness, t => t.matches_live());
        let found = must(
            live.iter().find(|candidate| candidate.id == row.id),
            "the live match",
        );
        assert_eq!(j(found)["lastBoards"], json!([j(&board()), []]));
    }

    async fn r678_samples_other_profiles_non_empty_server_boards_never_an_excluded_one(
        harness: &StoreHarness,
    ) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let c = active_profile(harness, None).await;
        let d = active_profile(harness, None).await;
        let e = active_profile(harness, None).await;
        let f = active_profile(harness, None).await;
        let board_c: Vec<LastBoardEntry> = from(json!([{ "defId": "core-025", "radiant": false }]));
        let board_f: Vec<LastBoardEntry> = from(json!([{ "defId": "core-012", "radiant": true }]));
        q!(harness, t => t.last_boards_put(&a.id, board_kind("server"), &board(), harness.now()));
        q!(harness, t => t.last_boards_put(&b.id, board_kind("server"), &board(), harness.now()));
        q!(harness, t => t.last_boards_put(&c.id, board_kind("server"), &board_c, harness.now()));
        q!(harness, t => t.last_boards_put(&d.id, board_kind("server"), &[], harness.now()));
        q!(harness, t => t.last_boards_put(&e.id, board_kind("practice"), &board(), harness.now()));
        q!(harness, t => t.last_boards_put(&f.id, board_kind("server"), &board_f, harness.now()));

        let excluded = [a.id.clone(), b.id.clone()];
        let two = q!(harness, t => t.last_boards_sample_others(&excluded, 2));
        assert_eq!(two.len(), 2);
        assert!(two.contains(&board_c) && two.contains(&board_f));
        assert_eq!(
            q!(harness, t => t.last_boards_sample_others(&excluded, 5)).len(),
            2
        );
        assert_eq!(
            q!(harness, t => t.last_boards_sample_others(&excluded, 1)).len(),
            1
        );
        let all_four = [a.id.clone(), b.id.clone(), c.id.clone(), f.id.clone()];
        assert!(q!(harness, t => t.last_boards_sample_others(&all_four, 2)).is_empty());
        assert!(q!(harness, t => t.last_boards_sample_others(&excluded, 0)).is_empty());
    }

    async fn r678_a_match_row_keeps_the_glitch_boards_it_started_with(harness: &StoreHarness) {
        let p1 = active_profile(harness, None).await;
        let p2 = active_profile(harness, None).await;
        let row: MatchRow = from(spread(
            j(&match_row(&id(), &p1.id, &p2.id, harness, harness.now())),
            json!({ "glitchBoards": [j(&board()), []] }),
        ));
        q!(harness, t => t.matches_create(&row));
        assert_eq!(q!(harness, t => t.matches_get(&row.id)), Some(row.clone()));
        let live = q!(harness, t => t.matches_live());
        let found = must(
            live.iter().find(|candidate| candidate.id == row.id),
            "the live match",
        );
        assert_eq!(j(found)["glitchBoards"], json!([j(&board()), []]));
        // A row written without them reads none.
        let plain = match_row(&id(), &p1.id, &p2.id, harness, harness.now());
        q!(harness, t => t.matches_create(&plain));
        assert!(absent(
            &j(&must(
                q!(harness, t => t.matches_get(&plain.id)),
                "the plain match"
            )),
            "glitchBoards"
        ));
    }

    both_stores!(
        r565_holds_no_board_before_a_profile_s_first_finished_game_then_the_one_written,
        r565_a_later_game_s_board_replaces_it_and_each_kind_is_its_own,
        r565_goes_with_the_profile,
        r417_a_match_row_keeps_the_boards_it_started_with,
        r678_samples_other_profiles_non_empty_server_boards_never_an_excluded_one,
        r678_a_match_row_keeps_the_glitch_boards_it_started_with,
    );
}

mod player_stats {
    use super::*;

    /// A `Record<string, unknown>` bag (SURFACE §4.3).
    fn bag(value: Value) -> IndexMap<String, Value> {
        from(value)
    }

    fn list_options(value: Value) -> PlayerStatsListOptions {
        from(value)
    }

    async fn r654_holds_no_row_before_the_first_write_and_put_creates_or_updates_a_row(
        harness: &StoreHarness,
    ) {
        let profile = active_profile(harness, None).await;
        assert!(q!(harness, t => t.player_stats_get(&profile.id)).is_none());

        let now = harness.now();
        let initial_stats = json!({
            "games": 10,
            "wins": 6,
            "losses": 4,
            "draws": 0,
            "cards": {
                "core:c01": { "played": 8, "defeated": 2, "destroyed": 1 },
            },
        });

        q!(harness, t => t.player_stats_put(&profile.id, &bag(initial_stats.clone()), false, now));
        let stored = q!(harness, t => t.player_stats_get(&profile.id));
        assert_eq!(
            j(&must(stored, "the stats row")),
            json!({ "profileId": profile.id, "stats": initial_stats, "isPrivate": false, "updatedAt": now })
        );

        let updated_stats = spread(initial_stats, json!({ "games": 11, "wins": 7 }));
        q!(harness, t => t.player_stats_put(&profile.id, &bag(updated_stats.clone()), true, now + 1000));
        let updated = q!(harness, t => t.player_stats_get(&profile.id));
        assert_eq!(
            j(&must(updated, "the stats row")),
            json!({ "profileId": profile.id, "stats": updated_stats, "isPrivate": true, "updatedAt": now + 1000 })
        );
    }

    async fn r654_list_public_excludes_private_players_respects_search_and_sorts_by_games_descending(
        harness: &StoreHarness,
    ) {
        let p1 = active_profile(harness, None).await;
        let p2 = active_profile(harness, None).await;
        let p3 = active_profile(harness, None).await;

        q!(harness, t => t.profiles_set_display_name(&p1.id, Some("Alice Wonderland")));
        q!(harness, t => t.profiles_set_display_name(&p2.id, Some("Bob Builder")));
        q!(harness, t => t.profiles_set_display_name(&p3.id, Some("Alice Secret")));

        let now = harness.now();
        let stats_1 = bag(json!({
            "games": 50,
            "wins": 30,
            "losses": 20,
            "draws": 0,
            "cards": {
                "c-strike": { "played": 25, "defeated": 5, "destroyed": 2 },
                "c-shield": { "played": 15, "defeated": 1, "destroyed": 0 },
                "c-nemesis": { "playedAgainst": 10 },
            },
        }));
        q!(harness, t => t.player_stats_put(&p1.id, &stats_1, false, now));

        let stats_2 = bag(json!({ "games": 100, "wins": 60, "losses": 40, "draws": 0 }));
        q!(harness, t => t.player_stats_put(&p2.id, &stats_2, false, now + 500));

        let stats_3 = bag(json!({ "games": 200, "wins": 150, "losses": 50, "draws": 0 }));
        q!(harness, t => t.player_stats_put(&p3.id, &stats_3, true, now + 1000));

        let profile_ids =
            |rows: &[Value]| -> Vec<Value> { rows.iter().map(|row| row["profileId"].clone()).collect() };

        let public_all: Vec<Value> =
            q!(harness, t => t.player_stats_list_public(&list_options(json!({ "limit": 10, "offset": 0 }))))
                .iter()
                .map(j)
                .collect();
        assert_eq!(profile_ids(&public_all), vec![json!(p2.id), json!(p1.id)]);
        assert_eq!(public_all[0]["displayName"], "Bob Builder");
        assert_eq!(public_all[1]["displayName"], "Alice Wonderland");
        assert_eq!(
            public_all[1]["favouriteCards"],
            json!([{ "id": "c-strike", "count": 25 }, { "id": "c-shield", "count": 15 }])
        );
        assert_eq!(
            public_all[1]["funStats"],
            json!({ "nemesisCardId": "c-nemesis", "totalDestroyed": 2, "totalDefeated": 6 })
        );

        let search_alice: Vec<Value> =
            q!(harness, t => t.player_stats_list_public(&list_options(json!({ "search": "Alice", "limit": 10, "offset": 0 }))))
                .iter()
                .map(j)
                .collect();
        assert_eq!(profile_ids(&search_alice), vec![json!(p1.id)]);

        let page_1: Vec<Value> =
            q!(harness, t => t.player_stats_list_public(&list_options(json!({ "limit": 1, "offset": 0 }))))
                .iter()
                .map(j)
                .collect();
        assert_eq!(profile_ids(&page_1), vec![json!(p2.id)]);
        let page_2: Vec<Value> =
            q!(harness, t => t.player_stats_list_public(&list_options(json!({ "limit": 1, "offset": 1 }))))
                .iter()
                .map(j)
                .collect();
        assert_eq!(profile_ids(&page_2), vec![json!(p1.id)]);
    }

    both_stores!(
        r654_holds_no_row_before_the_first_write_and_put_creates_or_updates_a_row,
        r654_list_public_excludes_private_players_respects_search_and_sorts_by_games_descending,
    );
}

mod series {
    use super::*;

    fn series_row(harness: &StoreHarness, p1: &str, p2: &str, over: Value) -> SeriesRow {
        let now = harness.now();
        from(spread(
            json!({
                "id": id(),
                "sides": [
                    { "profileId": p1, "trio": j(&frozen_trio(harness, Some("Mine"))), "wins": 0, "pick": null },
                    { "profileId": p2, "trio": j(&frozen_trio(harness, Some("Theirs"))), "wins": 0, "pick": null },
                ],
                "catalogVersion": harness.catalog_version,
                "seedBase": "series-seed",
                "status": "picking",
                "games": [],
                "nextMatchId": id(),
                "pickDeadline": now + 60_000,
                "winner": null,
                "endReason": null,
                "ratingBefore": null,
                "ratingAfter": null,
                "createdAt": now,
                "updatedAt": now,
                "endedAt": null,
                "version": 1,
            }),
            over,
        ))
    }

    fn version_of(row: &SeriesRow) -> i64 {
        must(j(row)["version"].as_i64(), "the version")
    }

    async fn r263_round_trips_a_series_row_exactly(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let row = series_row(harness, &a.id, &b.id, json!({}));
        q!(harness, t => t.series_create(&row));
        assert_eq!(q!(harness, t => t.series_get(&row.id)), Some(row.clone()));
        assert!(q!(harness, t => t.series_get(&id())).is_none());
        assert!(call!(harness, t => t.series_create(&row)).is_err());
    }

    async fn r263_writes_a_transition_only_over_the_version_it_was_made_from(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let row = series_row(harness, &a.id, &b.id, json!({}));
        q!(harness, t => t.series_create(&row));
        let mut picked = j(&row);
        picked["sides"][0]["pick"] = json!(2);
        picked["version"] = json!(version_of(&row) + 1);
        let picked: SeriesRow = from(picked);
        assert!(q!(harness, t => t.series_update(&picked)));
        // A second writer that read the same version loses.
        let stale: SeriesRow = from(spread(
            j(&row),
            json!({ "status": "over", "version": version_of(&row) + 1 }),
        ));
        assert!(!q!(harness, t => t.series_update(&stale)));
        assert_eq!(q!(harness, t => t.series_get(&row.id)), Some(picked));
    }

    /// R604: the ranked flag rides along on the version-checked update, same as every field.
    async fn r604_round_trips_the_ranked_flag(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let row = series_row(harness, &a.id, &b.id, json!({ "ranked": true }));
        q!(harness, t => t.series_create(&row));
        assert_eq!(q!(harness, t => t.series_get(&row.id)), Some(row.clone()));
        let moved: SeriesRow = from(without(
            spread(j(&row), json!({ "version": version_of(&row) + 1 })),
            "ranked",
        ));
        assert!(q!(harness, t => t.series_update(&moved)));
        assert!(absent(
            &j(&must(q!(harness, t => t.series_get(&row.id)), "the series")),
            "ranked"
        ));
    }

    async fn r263_finds_a_series_by_the_match_it_is_playing_and_only_while_it_is_playing_it(
        harness: &StoreHarness,
    ) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let row = series_row(harness, &a.id, &b.id, json!({}));
        q!(harness, t => t.series_create(&row));
        let next_match_id = must(
            j(&row)["nextMatchId"].as_str().map(str::to_string),
            "the next match id",
        );
        assert!(q!(harness, t => t.series_by_match(&next_match_id)).is_none());

        let playing: SeriesRow = from(spread(
            j(&row),
            json!({
                "status": "playing",
                "pickDeadline": null,
                "games": [
                    { "gameNo": 1, "matchId": next_match_id, "slots": [0, 1], "first": "p1", "winner": null, "reason": null },
                ],
                "version": version_of(&row) + 1,
            }),
        ));
        assert!(q!(harness, t => t.series_update(&playing)));
        assert_eq!(
            q!(harness, t => t.series_by_match(&next_match_id)),
            Some(playing.clone())
        );
        assert!(q!(harness, t => t.series_by_match(&id())).is_none());
        assert_eq!(
            q!(harness, t => t.series_with_game(&next_match_id)),
            Some(playing.clone())
        );

        // Once the game is over and the series has moved on, only `withGame` still finds it.
        let next_id = id();
        let shown = j(&playing);
        let mut game = shown["games"][0].clone();
        game["winner"] = json!("p2");
        game["reason"] = json!("concede");
        let mut second_side = shown["sides"][1].clone();
        second_side["wins"] = json!(1);
        let picking: SeriesRow = from(spread(
            shown.clone(),
            json!({
                "status": "picking",
                "nextMatchId": next_id,
                "pickDeadline": harness.now() + 60_000,
                "games": [game],
                "sides": [shown["sides"][0].clone(), second_side],
                "version": version_of(&playing) + 1,
            }),
        ));
        assert!(q!(harness, t => t.series_update(&picking)));
        assert!(q!(harness, t => t.series_by_match(&next_match_id)).is_none());
        assert_eq!(
            q!(harness, t => t.series_with_game(&next_match_id)),
            Some(picking)
        );
        assert!(q!(harness, t => t.series_with_game(&next_id)).is_none());
    }

    async fn r263_lists_the_series_that_are_not_over_and_each_player_s(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let c = active_profile(harness, None).await;
        let live = series_row(harness, &a.id, &b.id, json!({}));
        let done = series_row(
            harness,
            &c.id,
            &b.id,
            json!({ "status": "over", "winner": "p1", "endReason": "forfeit", "endedAt": harness.now() }),
        );
        q!(harness, t => t.series_create(&live));
        q!(harness, t => t.series_create(&done));
        assert_eq!(
            ids_of(&q!(harness, t => t.series_active())),
            vec![live.id.clone()]
        );
        assert_eq!(
            q!(harness, t => t.series_active_for(&b.id)).map(|row| row.id),
            Some(live.id.clone())
        );
        assert!(q!(harness, t => t.series_active_for(&c.id)).is_none());
    }

    both_stores!(
        r263_round_trips_a_series_row_exactly,
        r263_writes_a_transition_only_over_the_version_it_was_made_from,
        r604_round_trips_the_ranked_flag,
        r263_finds_a_series_by_the_match_it_is_playing_and_only_while_it_is_playing_it,
        r263_lists_the_series_that_are_not_over_and_each_player_s,
    );
}

// ---------------------------------------------------------------------------
// Matches (SPEC §9.3's log, §9.5's lifecycle)
// ---------------------------------------------------------------------------

mod matches {
    use super::*;

    async fn live_match(harness: &StoreHarness) -> MatchRow {
        let p1 = active_profile(harness, None).await;
        let p2 = active_profile(harness, None).await;
        let row = match_row(&id(), &p1.id, &p2.id, harness, harness.now());
        q!(harness, t => t.matches_create(&row));
        row
    }

    fn live_ids(rows: &[MatchRow]) -> Vec<String> {
        rows.iter().map(|row| row.id.clone()).collect()
    }

    async fn round_trips_a_match_row_clocks_included(harness: &StoreHarness) {
        let row = live_match(harness).await;
        assert_eq!(q!(harness, t => t.matches_get(&row.id)), Some(row.clone()));
        assert!(q!(harness, t => t.matches_get(&id())).is_none());
    }

    async fn refuses_a_second_match_with_the_same_id(harness: &StoreHarness) {
        let row = live_match(harness).await;
        let again: MatchRow = from(spread(j(&row), json!({ "seed": "other" })));
        assert!(call!(harness, t => t.matches_create(&again)).is_err());
    }

    /// §9.3: "Append-only action log per match."
    async fn appends_actions_in_order_and_reads_them_back(harness: &StoreHarness) {
        let row = live_match(harness).await;
        q!(harness, t => t.matches_append_actions(&[action_row(&row.id, 1, "p1", "n1", harness.now())]));
        q!(harness, t => t.matches_append_actions(&[
            action_row(&row.id, 2, "p2", "n2", harness.now()),
            action_row(&row.id, 3, "p1", "n3", harness.now()),
        ]));

        let log: Vec<Value> = q!(harness, t => t.matches_actions(&row.id))
            .iter()
            .map(j)
            .collect();
        let seqs: Vec<Value> = log.iter().map(|entry| entry["seq"].clone()).collect();
        assert_eq!(seqs, vec![json!(1), json!(2), json!(3)]);
        let actions: Vec<Value> = log.iter().map(|entry| entry["action"].clone()).collect();
        assert_eq!(
            actions,
            vec![
                j(&action("p1", "n1")),
                j(&action("p2", "n2")),
                j(&action("p1", "n3"))
            ]
        );
        // `at` is the database's clock in Postgres (`match_actions` is append-only, so the caller
        // cannot stamp it) — see KNOWN DIVERGENCES (action timestamps).
        assert!(log.iter().all(|entry| entry["at"].is_number()));
    }

    async fn refuses_a_seq_that_already_exists(harness: &StoreHarness) {
        let row = live_match(harness).await;
        q!(harness, t => t.matches_append_actions(&[action_row(&row.id, 1, "p1", "n1", harness.now())]));
        assert!(
            call!(harness, t => t.matches_append_actions(&[action_row(&row.id, 1, "p2", "clash", harness.now())]))
                .is_err()
        );
        assert_eq!(q!(harness, t => t.matches_actions(&row.id)).len(), 1);
    }

    async fn stores_the_clocks_the_clients_render(harness: &StoreHarness) {
        let row = live_match(harness).await;
        let now = harness.now();
        let next: MatchClocks = from(json!({
            "turnDeadline": now + 1_000,
            "promptDeadline": now + 2_000,
            "graceDeadline": { "p1": now + 3_000, "p2": null },
            "ceilingAt": now + 4_000,
        }));
        q!(harness, t => t.matches_set_clocks(&row.id, &next));
        assert_eq!(
            j(&must(q!(harness, t => t.matches_get(&row.id)), "the match"))["clocks"],
            j(&next)
        );
    }

    /// R604: the queue's ranked flag survives the round trip; a room's absence reads unranked.
    async fn r604_round_trips_the_ranked_flag(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let ranked: MatchRow = from(spread(
            j(&match_row(&id(), &a.id, &b.id, harness, harness.now())),
            json!({ "ranked": true }),
        ));
        let unranked = match_row(&id(), &a.id, &b.id, harness, harness.now());
        q!(harness, t => t.matches_create(&ranked));
        q!(harness, t => t.matches_create(&unranked));
        assert_eq!(
            j(&must(
                q!(harness, t => t.matches_get(&ranked.id)),
                "the ranked match"
            ))["ranked"],
            true
        );
        assert!(absent(
            &j(&must(
                q!(harness, t => t.matches_get(&unranked.id)),
                "the unranked match"
            )),
            "ranked"
        ));
    }

    /// §9.5: a rematch's mode and stakes survive the round trip; their absence reads as "derive it"
    /// and 1.
    async fn round_trips_the_rematch_mode_and_stake(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let rematch: MatchRow = from(spread(
            j(&match_row(&id(), &a.id, &b.id, harness, harness.now())),
            json!({ "mode": "random", "stake": 2 }),
        ));
        let plain = match_row(&id(), &a.id, &b.id, harness, harness.now());
        q!(harness, t => t.matches_create(&rematch));
        q!(harness, t => t.matches_create(&plain));
        let got = j(&must(q!(harness, t => t.matches_get(&rematch.id)), "the rematch"));
        assert_eq!(got["mode"], "random");
        assert_eq!(got["stake"], 2);
        let got_plain = j(&must(
            q!(harness, t => t.matches_get(&plain.id)),
            "the plain match",
        ));
        assert!(absent(&got_plain, "mode"));
        assert!(absent(&got_plain, "stake"));
        assert_eq!(
            j(&q!(harness, t => t.matches_mode_of(&rematch.id))),
            json!("random")
        );
    }

    async fn r263_discards_a_reserved_match_id_without_touching_a_live_match(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let row = match_row(&id(), &a.id, &b.id, harness, harness.now());
        q!(harness, t => t.matches_create(&row));
        q!(harness, t => t.matches_discard_open(&row.id));
        q!(harness, t => t.matches_discard_open(&id()));
        assert_eq!(q!(harness, t => t.matches_get(&row.id)), Some(row));
    }

    async fn r679_forgets_a_voided_live_match_its_row_its_log_and_both_players_in_match_flags(
        harness: &StoreHarness,
    ) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let row = match_row(&id(), &a.id, &b.id, harness, harness.now());
        q!(harness, t => t.matches_create(&row));
        q!(harness, t => t.profiles_set_in_match(&a.id, Some(row.id.as_str())));
        q!(harness, t => t.profiles_set_in_match(&b.id, Some(row.id.as_str())));
        q!(harness, t => t.matches_append_actions(&[
            action_row(&row.id, 1, "p1", "n1", harness.now()),
            action_row(&row.id, 2, "p2", "n2", harness.now()),
        ]));

        q!(harness, t => t.matches_forget_voided(&row.id));
        assert!(q!(harness, t => t.matches_get(&row.id)).is_none());
        assert!(q!(harness, t => t.matches_actions(&row.id)).is_empty());
        assert!(!live_ids(&q!(harness, t => t.matches_live())).contains(&row.id));
        assert!(j(&must(q!(harness, t => t.profiles_get_by_id(&a.id)), "a"))["inMatchId"].is_null());
        assert!(j(&must(q!(harness, t => t.profiles_get_by_id(&b.id)), "b"))["inMatchId"].is_null());
        // The id is free again: a Conquest game voided is started again under it.
        q!(harness, t => t.matches_create(&row));
        assert_eq!(q!(harness, t => t.matches_get(&row.id)), Some(row.clone()));
        // An unknown id is a no-op.
        q!(harness, t => t.matches_forget_voided(&id()));
    }

    async fn r679_never_forgets_a_finished_match_or_one_with_a_result(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let finished = match_row(&id(), &a.id, &b.id, harness, harness.now());
        q!(harness, t => t.matches_create(&finished));
        q!(harness, t => t.matches_finish(&finished.id, harness.now()));
        q!(harness, t => t.matches_forget_voided(&finished.id));
        assert_eq!(
            j(&must(
                q!(harness, t => t.matches_get(&finished.id)),
                "the finished match"
            ))["status"],
            "finished"
        );

        let resulted = match_row(&id(), &a.id, &b.id, harness, harness.now());
        q!(harness, t => t.matches_create(&resulted));
        let result: ResultRow = from(json!({
            "matchId": resulted.id,
            "players": [a.id, b.id],
            "winnerProfileId": null,
            "reason": "draw-accepted",
            "turns": 3,
            "endedAt": harness.now(),
            "ratingBefore": [1000, 1000],
            "ratingAfter": [1000, 1000],
        }));
        q!(harness, t => t.results_insert(&result));
        q!(harness, t => t.matches_forget_voided(&resulted.id));
        assert!(q!(harness, t => t.matches_get(&resulted.id)).is_some());
    }

    async fn finishes_a_match_and_drops_it_out_of_the_live_set(harness: &StoreHarness) {
        let row = live_match(harness).await;
        assert!(live_ids(&q!(harness, t => t.matches_live())).contains(&row.id));

        let at = harness.now();
        q!(harness, t => t.matches_finish(&row.id, at));
        let finished = j(&must(
            q!(harness, t => t.matches_get(&row.id)),
            "the finished match",
        ));
        assert_eq!(finished["status"], "finished");
        assert_eq!(finished["finishedAt"].as_i64(), Some(at));
        assert!(!live_ids(&q!(harness, t => t.matches_live())).contains(&row.id));
    }

    both_stores!(
        round_trips_a_match_row_clocks_included,
        refuses_a_second_match_with_the_same_id,
        appends_actions_in_order_and_reads_them_back,
        refuses_a_seq_that_already_exists,
        stores_the_clocks_the_clients_render,
        r604_round_trips_the_ranked_flag,
        round_trips_the_rematch_mode_and_stake,
        r263_discards_a_reserved_match_id_without_touching_a_live_match,
        r679_forgets_a_voided_live_match_its_row_its_log_and_both_players_in_match_flags,
        r679_never_forgets_a_finished_match_or_one_with_a_result,
        finishes_a_match_and_drops_it_out_of_the_live_set,
    );
}

// ---------------------------------------------------------------------------
// Rooms (SPEC §9.5's direct challenge)
// ---------------------------------------------------------------------------

mod rooms {
    use super::*;

    fn room(harness: &StoreHarness, code: &str, host: &str, over: Value) -> Room {
        let now = harness.now();
        from(spread(
            json!({
                "code": code,
                "hostProfileId": host,
                "mode": "bo1",
                "hostDeck": deck_of(harness, 0),
                "hostTrio": null,
                "catalogVersion": harness.catalog_version,
                "createdAt": now,
                "expiresAt": now + 600_000,
                "guestProfileId": null,
                "matchId": null,
            }),
            over,
        ))
    }

    async fn creates_a_room_and_reads_it_back_by_code(harness: &StoreHarness) {
        let host = active_profile(harness, None).await;
        let created = room(harness, "ABC234", &host.id, json!({}));
        assert!(q!(harness, t => t.rooms_create(&created)));

        let found = j(&must(q!(harness, t => t.rooms_get("ABC234")), "the room"));
        let created = j(&created);
        assert_eq!(found["hostProfileId"], host.id.as_str());
        assert_eq!(found["hostDeck"], created["hostDeck"]);
        assert_eq!(found["catalogVersion"], harness.catalog_version.as_str());
        assert_eq!(found["expiresAt"], created["expiresAt"]);
        assert!(found["guestProfileId"].is_null());
        assert!(found["matchId"].is_null());
        assert!(q!(harness, t => t.rooms_get("ZZZ999")).is_none());
    }

    async fn r264_keeps_a_room_s_mode_and_a_best_of_3_host_s_frozen_trio(harness: &StoreHarness) {
        let host = active_profile(harness, None).await;
        let guest = active_profile(harness, None).await;
        let bo3 = room(
            harness,
            "BCD345",
            &host.id,
            json!({ "mode": "bo3", "hostDeck": [], "hostTrio": j(&frozen_trio(harness, None)) }),
        );
        assert!(q!(harness, t => t.rooms_create(&bo3)));
        let found = j(&must(q!(harness, t => t.rooms_get("BCD345")), "the room"));
        assert_eq!(found["mode"], "bo3");
        assert_eq!(found["hostDeck"], json!([]));
        assert_eq!(found["hostTrio"], j(&frozen_trio(harness, None)));

        let random = room(
            harness,
            "CDE456",
            &host.id,
            json!({ "mode": "random", "hostDeck": [] }),
        );
        q!(harness, t => t.rooms_create(&random));
        let claimed = j(&must(
            q!(harness, t => t.rooms_claim("CDE456", &guest.id, &id(), harness.now())),
            "the claim",
        ));
        assert_eq!(claimed["mode"], "random");
        assert!(claimed["hostTrio"].is_null());
    }

    async fn refuses_a_code_that_is_already_taken(harness: &StoreHarness) {
        let host = active_profile(harness, None).await;
        let other = active_profile(harness, None).await;
        assert!(q!(harness, t => t.rooms_create(&room(harness, "ABC234", &host.id, json!({})))));
        assert!(!q!(harness, t => t.rooms_create(&room(harness, "ABC234", &other.id, json!({})))));
    }

    /// §9.5: the atomic single-claim — the loser of a join race never gets a second match.
    async fn lets_exactly_one_guest_claim_a_room(harness: &StoreHarness) {
        let host = active_profile(harness, None).await;
        let guest = active_profile(harness, None).await;
        let loser = active_profile(harness, None).await;
        q!(harness, t => t.rooms_create(&room(harness, "ABC234", &host.id, json!({}))));

        let match_id = id();
        let claimed = j(&must(
            q!(harness, t => t.rooms_claim("ABC234", &guest.id, &match_id, harness.now())),
            "the claim",
        ));
        assert_eq!(claimed["guestProfileId"], guest.id.as_str());
        assert_eq!(claimed["matchId"], match_id.as_str());
        assert_eq!(claimed["hostDeck"], json!(deck_of(harness, 0)));

        assert!(q!(harness, t => t.rooms_claim("ABC234", &loser.id, &id(), harness.now())).is_none());
        assert_eq!(
            j(&must(q!(harness, t => t.rooms_get("ABC234")), "the room"))["guestProfileId"],
            guest.id.as_str()
        );
    }

    async fn refuses_a_claim_after_the_room_has_expired(harness: &StoreHarness) {
        let host = active_profile(harness, None).await;
        let guest = active_profile(harness, None).await;
        let now = harness.now();
        q!(harness, t => t.rooms_create(&room(harness, "ABC234", &host.id, json!({ "expiresAt": now + 1_000 }))));
        assert!(q!(harness, t => t.rooms_claim("ABC234", &guest.id, &id(), now + 60_000)).is_none());
    }

    /// The whole room path: create -> claim -> the match the registry then writes (§9.5).
    async fn becomes_a_live_match_once_the_registry_creates_it(harness: &StoreHarness) {
        let host = active_profile(harness, None).await;
        let guest = active_profile(harness, None).await;
        q!(harness, t => t.rooms_create(&room(harness, "ABC234", &host.id, json!({}))));

        let match_id = id();
        must(
            q!(harness, t => t.rooms_claim("ABC234", &guest.id, &match_id, harness.now())),
            "the claim",
        );
        let row = match_row(&match_id, &host.id, &guest.id, harness, harness.now());
        q!(harness, t => t.matches_create(&row));

        assert_eq!(q!(harness, t => t.matches_get(&match_id)), Some(row));
        assert!(
            q!(harness, t => t.matches_live())
                .iter()
                .any(|live| live.id == match_id)
        );
    }

    both_stores!(
        creates_a_room_and_reads_it_back_by_code,
        r264_keeps_a_room_s_mode_and_a_best_of_3_host_s_frozen_trio,
        refuses_a_code_that_is_already_taken,
        lets_exactly_one_guest_claim_a_room,
        refuses_a_claim_after_the_room_has_expired,
        becomes_a_live_match_once_the_registry_creates_it,
    );
}

// ---------------------------------------------------------------------------
// Tickets (SPEC §9.5's ranked queue)
// ---------------------------------------------------------------------------

mod tickets {
    use super::*;

    fn ticket(harness: &StoreHarness, profile_id: &str, over: Value) -> Ticket {
        from(spread(
            json!({
                "id": id(),
                "profileId": profile_id,
                "rating": 1000,
                "mode": "bo1",
                "deck": deck_of(harness, 0),
                // R642: the Bo1 deck's frozen portrait; `null` reads as `vanilla`.
                "portrait": null,
                "trio": null,
                "catalogVersion": harness.catalog_version,
                "enqueuedAt": harness.now(),
                "status": "open",
                "matchId": null,
            }),
            over,
        ))
    }

    async fn inserts_a_ticket_and_finds_the_profile_s_open_one(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let row = ticket(harness, &profile.id, json!({}));
        q!(harness, t => t.tickets_insert(&row));

        assert_eq!(q!(harness, t => t.tickets_get(&row.id)), Some(row.clone()));
        assert_eq!(
            q!(harness, t => t.tickets_open_for_profile(&profile.id)),
            Some(row.clone())
        );
        assert_eq!(q!(harness, t => t.tickets_count_open()), 1);
        assert_eq!(
            ids_of(&q!(harness, t => t.tickets_list_open())),
            vec![row.id.clone()]
        );
    }

    async fn r257_keeps_a_ticket_s_mode_and_a_best_of_3_ticket_s_trio_and_counts_open_tickets_per_mode(
        harness: &StoreHarness,
    ) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let c = active_profile(harness, None).await;
        let bo3 = ticket(
            harness,
            &a.id,
            json!({ "mode": "bo3", "deck": [], "trio": j(&frozen_trio(harness, None)) }),
        );
        let random = ticket(harness, &b.id, json!({ "mode": "random", "deck": [] }));
        q!(harness, t => t.tickets_insert(&bo3));
        q!(harness, t => t.tickets_insert(&random));
        q!(harness, t => t.tickets_insert(&ticket(harness, &c.id, json!({}))));

        assert_eq!(q!(harness, t => t.tickets_get(&bo3.id)), Some(bo3.clone()));
        assert_eq!(q!(harness, t => t.tickets_get(&random.id)), Some(random.clone()));
        assert_eq!(
            j(&q!(harness, t => t.tickets_count_open_by_mode())),
            json!({ "bo1": 1, "bo3": 1, "random": 1 })
        );
        q!(harness, t => t.tickets_cancel(&random.id, harness.now()));
        assert_eq!(
            j(&q!(harness, t => t.tickets_count_open_by_mode())),
            json!({ "bo1": 1, "bo3": 1, "random": 0 })
        );
    }

    async fn r642_keeps_a_best_of_3_ticket_s_per_deck_portraits_and_absence_stays_absent(
        harness: &StoreHarness,
    ) {
        let profile = active_profile(harness, None).await;
        let mut trio = j(&frozen_trio(harness, None));
        trio["decks"][0]["portrait"] = json!("gary");
        trio["decks"][1]["portrait"] = Value::Null;
        trio["decks"][2]["portrait"] = json!("timmy");
        let bo3 = ticket(
            harness,
            &profile.id,
            json!({ "mode": "bo3", "deck": [], "trio": trio }),
        );
        q!(harness, t => t.tickets_insert(&bo3));
        assert_eq!(q!(harness, t => t.tickets_get(&bo3.id)), Some(bo3.clone()));

        let mut legacy = j(&frozen_trio(harness, None));
        if let Some(deck) = legacy["decks"][0].as_object_mut() {
            deck.remove("portrait");
        }
        let owner = active_profile(harness, None).await;
        let old = ticket(
            harness,
            &owner.id,
            json!({ "mode": "bo3", "deck": [], "trio": legacy }),
        );
        q!(harness, t => t.tickets_insert(&old));
        assert_eq!(q!(harness, t => t.tickets_get(&old.id)), Some(old.clone()));
    }

    /// `tickets_profile_queued_key`: the race-proof half of §9.5's "not already queued".
    async fn refuses_a_second_open_ticket_for_one_profile(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        q!(harness, t => t.tickets_insert(&ticket(harness, &profile.id, json!({}))));
        assert!(call!(harness, t => t.tickets_insert(&ticket(harness, &profile.id, json!({})))).is_err());
        assert_eq!(q!(harness, t => t.tickets_count_open()), 1);
    }

    async fn cancels_an_open_ticket_and_cancelling_twice_is_harmless(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let row = ticket(harness, &profile.id, json!({}));
        q!(harness, t => t.tickets_insert(&row));

        q!(harness, t => t.tickets_cancel(&row.id, harness.now()));
        assert_eq!(
            j(&must(q!(harness, t => t.tickets_get(&row.id)), "the ticket"))["status"],
            "cancelled"
        );
        assert!(q!(harness, t => t.tickets_open_for_profile(&profile.id)).is_none());
        q!(harness, t => t.tickets_cancel(&row.id, harness.now()));
        assert_eq!(q!(harness, t => t.tickets_count_open()), 0);
    }

    /// §9.5: "both tickets are claimed in one atomic statement".
    async fn claims_a_pair_exactly_once(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let ta = ticket(harness, &a.id, json!({}));
        let tb = ticket(harness, &b.id, json!({}));
        q!(harness, t => t.tickets_insert(&ta));
        q!(harness, t => t.tickets_insert(&tb));

        let match_id = id();
        assert!(q!(harness, t => t.tickets_claim_pair(&ta.id, &tb.id, &match_id, harness.now())));
        assert!(!q!(harness, t => t.tickets_claim_pair(&ta.id, &tb.id, &id(), harness.now())));

        let claimed = j(&must(q!(harness, t => t.tickets_get(&ta.id)), "ticket a"));
        assert_eq!(claimed["status"], "matched");
        assert_eq!(claimed["matchId"], match_id.as_str());
        assert_eq!(q!(harness, t => t.tickets_count_open()), 0);
    }

    async fn refuses_to_pair_a_ticket_with_itself(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let row = ticket(harness, &profile.id, json!({}));
        q!(harness, t => t.tickets_insert(&row));
        assert!(!q!(harness, t => t.tickets_claim_pair(&row.id, &row.id, &id(), harness.now())));
    }

    /// The whole queue path: claim the pair, then write the match the pair produced (§9.5).
    async fn becomes_a_live_match_once_the_pair_is_claimed_and_the_match_created(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let ta = ticket(harness, &a.id, json!({}));
        let tb = ticket(harness, &b.id, json!({}));
        q!(harness, t => t.tickets_insert(&ta));
        q!(harness, t => t.tickets_insert(&tb));

        let match_id = id();
        assert!(q!(harness, t => t.tickets_claim_pair(&ta.id, &tb.id, &match_id, harness.now())));
        let row = match_row(&match_id, &a.id, &b.id, harness, harness.now());
        q!(harness, t => t.matches_create(&row));
        assert_eq!(q!(harness, t => t.matches_get(&match_id)), Some(row));
    }

    both_stores!(
        inserts_a_ticket_and_finds_the_profile_s_open_one,
        r257_keeps_a_ticket_s_mode_and_a_best_of_3_ticket_s_trio_and_counts_open_tickets_per_mode,
        r642_keeps_a_best_of_3_ticket_s_per_deck_portraits_and_absence_stays_absent,
        refuses_a_second_open_ticket_for_one_profile,
        cancels_an_open_ticket_and_cancelling_twice_is_harmless,
        claims_a_pair_exactly_once,
        refuses_to_pair_a_ticket_with_itself,
        becomes_a_live_match_once_the_pair_is_claimed_and_the_match_created,
    );
}

// ---------------------------------------------------------------------------
// Game records for the card statistics (SPEC §9.11, R376–R378)
// ---------------------------------------------------------------------------

mod game_records {
    use super::*;

    fn game_record(record_id: &str, partial: Value) -> GameRecord {
        from(spread(
            json!({
                "id": record_id,
                "source": "live",
                "mode": "bo1",
                "patch": "v0.1.1",
                "pilots": { "p1": "human", "p2": "human" },
                "game": {
                    "first": "p1",
                    "winner": "p2",
                    "reason": "concede",
                    "turns": 6,
                    "seats": {
                        "p1": { "deck": ["core-001", "core-002"], "opening": ["core-001"], "drawn": ["core-002"], "played": ["core-001"] },
                        "p2": { "deck": ["core-003"], "opening": ["core-003", "core-t-coin"], "drawn": [], "played": [] },
                    },
                },
            }),
            partial,
        ))
    }

    fn query(value: Value) -> GameRecordQuery {
        from(value)
    }

    /// R376: the mode a record is filed under is read off what made the match.
    async fn r376_reads_a_match_s_mode_off_its_room_its_queue_tickets_or_its_series(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let now = harness.now();

        // A room's match: the room's mode.
        for (code, mode) in [("ABC234", "random"), ("BCD345", "bo1")] {
            let room: Room = from(json!({
                "code": code,
                "hostProfileId": a.id,
                "mode": mode,
                "hostDeck": (if mode == "bo1" { deck_of(harness, 0) } else { Vec::new() }),
                "hostTrio": null,
                "catalogVersion": harness.catalog_version,
                "createdAt": now,
                "expiresAt": now + 600_000,
                "guestProfileId": null,
                "matchId": null,
            }));
            q!(harness, t => t.rooms_create(&room));
            let room_match = id();
            must(
                q!(harness, t => t.rooms_claim(code, &b.id, &room_match, now)),
                "the claim",
            );
            q!(harness, t => t.matches_create(&match_row(&room_match, &a.id, &b.id, harness, now)));
            assert_eq!(j(&q!(harness, t => t.matches_mode_of(&room_match))), json!(mode));
            q!(harness, t => t.matches_finish(&room_match, now));
        }

        // A queue match: its tickets' mode.
        let tickets: Vec<Ticket> = [&a.id, &b.id]
            .into_iter()
            .map(|profile_id| {
                from(json!({
                    "id": id(),
                    "profileId": profile_id,
                    "rating": 1000,
                    "mode": "random",
                    "deck": [],
                    "trio": null,
                    "catalogVersion": harness.catalog_version,
                    "enqueuedAt": now,
                    "status": "open",
                    "matchId": null,
                }))
            })
            .collect();
        for row in &tickets {
            q!(harness, t => t.tickets_insert(row));
        }
        let queue_match = id();
        assert!(q!(harness, t => t.tickets_claim_pair(&tickets[0].id, &tickets[1].id, &queue_match, now)));
        q!(harness, t => t.matches_create(&match_row(&queue_match, &a.id, &b.id, harness, now)));
        assert_eq!(
            j(&q!(harness, t => t.matches_mode_of(&queue_match))),
            json!("random")
        );

        // A game of a Conquest series: bo3, whatever made the series.
        let series_game = id();
        let series: SeriesRow = from(json!({
            "id": id(),
            "sides": [
                { "profileId": a.id, "trio": j(&frozen_trio(harness, Some("Mine"))), "wins": 0, "pick": null },
                { "profileId": b.id, "trio": j(&frozen_trio(harness, Some("Theirs"))), "wins": 0, "pick": null },
            ],
            "catalogVersion": harness.catalog_version,
            "seedBase": "series-seed",
            "status": "playing",
            "games": [{ "gameNo": 2, "matchId": series_game, "slots": [0, 1], "first": "p2", "winner": null, "reason": null }],
            "nextMatchId": series_game,
            "pickDeadline": null,
            "winner": null,
            "endReason": null,
            "ratingBefore": null,
            "ratingAfter": null,
            "createdAt": now,
            "updatedAt": now,
            "endedAt": null,
            "version": 1,
        }));
        q!(harness, t => t.series_create(&series));
        q!(harness, t => t.matches_create(&match_row(&series_game, &b.id, &a.id, harness, now)));
        assert_eq!(
            j(&q!(harness, t => t.matches_mode_of(&series_game))),
            json!("bo3")
        );

        // Nothing made this one.
        assert!(q!(harness, t => t.matches_mode_of(&id())).is_none());
    }

    async fn r376_writes_one_record_per_game_and_refuses_a_second_with_the_same_id(harness: &StoreHarness) {
        let live = game_record(&id(), json!({}));
        assert!(q!(harness, t => t.game_records_insert(&live)));
        let again: GameRecord = from(spread(j(&live), json!({ "patch": "v0.2.0" })));
        assert!(!q!(harness, t => t.game_records_insert(&again)));
        assert_eq!(
            q!(harness, t => t.game_records_list(&query(json!({ "source": "live", "mode": null, "patch": null })))),
            vec![live]
        );
    }

    async fn r378_reads_development_records_only_when_asked_and_filters_by_mode_and_patch(
        harness: &StoreHarness,
    ) {
        let records = [
            game_record("a-live", json!({ "mode": "bo1", "patch": "v0.1.1" })),
            game_record("b-live", json!({ "mode": "random", "patch": "v0.2.5" })),
            game_record(
                "dev:c",
                json!({ "source": "dev", "mode": "random", "patch": "v0.2.5", "pilots": { "p1": "ai", "p2": "ai" } }),
            ),
            game_record(
                "dev:d",
                json!({ "source": "dev", "mode": "random", "patch": "v0.1.1", "pilots": { "p1": "ai", "p2": "ai" } }),
            ),
        ];
        // Written out of order: a read comes back in id order.
        for record in records.iter().rev() {
            assert!(q!(harness, t => t.game_records_insert(record)));
        }
        async fn ids(harness: &StoreHarness, filter: Value) -> Vec<String> {
            let filter = query(filter);
            ids_of(&q!(harness, t => t.game_records_list(&filter)))
        }

        assert_eq!(
            ids(harness, json!({ "source": "live", "mode": null, "patch": null })).await,
            ["a-live", "b-live"]
        );
        assert_eq!(
            ids(harness, json!({ "source": "dev", "mode": null, "patch": null })).await,
            ["dev:c", "dev:d"]
        );
        assert_eq!(
            ids(harness, json!({ "source": "all", "mode": null, "patch": null })).await,
            ["a-live", "b-live", "dev:c", "dev:d"]
        );
        assert_eq!(
            ids(
                harness,
                json!({ "source": "all", "mode": "random", "patch": null })
            )
            .await,
            ["b-live", "dev:c", "dev:d"]
        );
        assert_eq!(
            ids(
                harness,
                json!({ "source": "all", "mode": null, "patch": "v0.2.5" })
            )
            .await,
            ["b-live", "dev:c"]
        );
        assert_eq!(
            ids(
                harness,
                json!({ "source": "dev", "mode": "random", "patch": "v0.1.1" })
            )
            .await,
            ["dev:d"]
        );
        assert!(
            ids(harness, json!({ "source": "live", "mode": "bo3", "patch": null }))
                .await
                .is_empty()
        );
        assert_eq!(
            q!(harness, t => t.game_records_list(&query(json!({ "source": "dev", "mode": null, "patch": "v0.2.5" })))),
            vec![records[2].clone()]
        );
    }

    /// R378: neither kind of record can take the other's place under the one id.
    async fn r378_refuses_a_development_record_under_a_match_id_and_a_live_one_under_a_development_id(
        harness: &StoreHarness,
    ) {
        let dev = json!({ "source": "dev", "pilots": { "p1": "ai", "p2": "ai" } });
        assert!(call!(harness, t => t.game_records_insert(&game_record(&id(), dev.clone()))).is_err());
        assert!(call!(harness, t => t.game_records_insert(&game_record("dev:forged", json!({})))).is_err());
        assert!(
            q!(harness, t => t.game_records_list(&query(json!({ "source": "all", "mode": null, "patch": null })))).is_empty()
        );
    }

    both_stores!(
        r376_reads_a_match_s_mode_off_its_room_its_queue_tickets_or_its_series,
        r376_writes_one_record_per_game_and_refuses_a_second_with_the_same_id,
        r378_reads_development_records_only_when_asked_and_filters_by_mode_and_patch,
        r378_refuses_a_development_record_under_a_match_id_and_a_live_one_under_a_development_id,
    );
}

// ---------------------------------------------------------------------------
// Results (SPEC §2.5, §9.5)
// ---------------------------------------------------------------------------

mod results {
    use super::*;

    async fn writes_one_result_per_match_and_refuses_a_second(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let match_id = id();
        q!(harness, t => t.matches_create(&match_row(&match_id, &a.id, &b.id, harness, harness.now())));

        let row: ResultRow = from(json!({
            "matchId": match_id,
            "players": [a.id, b.id],
            "winnerProfileId": a.id,
            "reason": "concede",
            "turns": 7,
            "endedAt": harness.now(),
            "ratingBefore": [1000, 1000],
            "ratingAfter": [1016, 984],
        }));
        q!(harness, t => t.results_insert(&row));
        assert_eq!(
            q!(harness, t => t.results_get_by_match(&match_id)),
            Some(row.clone())
        );
        // The port's own refusal — `api/results.rs` retries on it, so it must not arrive untyped.
        let refused = must(
            call!(harness, t => t.results_insert(&row)).err(),
            "the second result's refusal",
        );
        assert!(matches!(refused, StoreError::Duplicate { .. }), "{refused}");
    }

    async fn records_a_draw_as_a_null_winner(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let match_id = id();
        q!(harness, t => t.matches_create(&match_row(&match_id, &a.id, &b.id, harness, harness.now())));
        let row: ResultRow = from(json!({
            "matchId": match_id,
            "players": [a.id, b.id],
            "winnerProfileId": null,
            "reason": "match-ceiling",
            "turns": 0,
            "endedAt": harness.now(),
            "ratingBefore": [1000, 1000],
            "ratingAfter": [1000, 1000],
        }));
        q!(harness, t => t.results_insert(&row));
        assert!(
            j(&must(
                q!(harness, t => t.results_get_by_match(&match_id)),
                "the result"
            ))["winnerProfileId"]
                .is_null()
        );
    }

    async fn returns_null_for_a_match_that_has_not_ended(harness: &StoreHarness) {
        assert!(q!(harness, t => t.results_get_by_match(&id())).is_none());
    }

    both_stores!(
        writes_one_result_per_match_and_refuses_a_second,
        records_a_draw_as_a_null_winner,
        returns_null_for_a_match_that_has_not_ended,
    );
}

// ---------------------------------------------------------------------------
// The ranked ladder (SPEC §9.12, R603–R612)
// ---------------------------------------------------------------------------

mod ranked {
    use super::*;

    fn glicko(rating: f64) -> Glicko {
        from(json!({ "rating": rating, "deviation": 350, "volatility": 0.06 }))
    }

    fn season(id: &str, at: i64) -> Season {
        from(json!({ "id": id, "patchVersion": format!("{id}.1"), "startedAt": at }))
    }

    fn rank_row(harness: &StoreHarness, season_id: &str, profile_id: &str, over: Value) -> SeasonRank {
        from(spread(j(&fresh_rank(season_id, profile_id, harness.now())), over))
    }

    fn rated_game(
        harness: &StoreHarness,
        game_id: &str,
        season_id: &str,
        p1: &Profile,
        p2: &Profile,
        over: Value,
    ) -> RatedGameRow {
        from(spread(
            json!({
                "id": game_id,
                "kind": "match",
                "seasonId": season_id,
                "patchVersion": "v0.1.1",
                "catalogVersion": harness.catalog_version,
                "sides": [
                    {
                        "profileId": p1.id,
                        "botId": null,
                        "pilot": "human",
                        "before": j(&glicko(1000.0)),
                        "after": j(&glicko(1016.0)),
                        "rankBefore": null,
                        "rankAfter": { "tier": "rotten", "division": 3, "pips": 2, "pipsPerDivision": 5, "floor": "rotten" },
                    },
                    {
                        "profileId": p2.id,
                        "botId": null,
                        "pilot": "human",
                        "before": j(&glicko(1000.0)),
                        "after": j(&glicko(984.0)),
                        "rankBefore": null,
                        "rankAfter": { "tier": "rotten", "division": 3, "pips": 1, "pipsPerDivision": 5, "floor": "rotten" },
                    },
                ],
                "winnerSide": 0,
                "reason": "hero-death",
                "endedAt": harness.now(),
            }),
            over,
        ))
    }

    fn peak_jlorious(rank: Option<SeasonRank>) -> Option<i64> {
        j(&must(rank, "the rank"))["peakJlorious"].as_i64()
    }

    async fn r609_opens_a_season_once_and_lists_every_season_oldest_first(harness: &StoreHarness) {
        let now = harness.now();
        let older = season("v0.1", now);
        let newer = season("v0.2", now + 1_000);
        // The open serializer: a no-op where one process owns the store, a real advisory lock under
        // Postgres — callable either way.
        q!(harness, t => t.ranked_lock_seasons());
        assert!(q!(harness, t => t.ranked_create_season(&newer)));
        assert!(q!(harness, t => t.ranked_create_season(&older)));
        // A second opener of the same id writes nothing and answers false.
        let reopened: Season = from(spread(j(&newer), json!({ "patchVersion": "v0.2.9" })));
        assert!(!q!(harness, t => t.ranked_create_season(&reopened)));
        assert_eq!(q!(harness, t => t.ranked_seasons()), vec![older, newer]);
    }

    async fn r605_keeps_one_rank_row_per_player_per_season_put_rank_replacing_it(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let now = harness.now();
        q!(harness, t => t.ranked_create_season(&season("v0.1", now)));

        let first = rank_row(
            harness,
            "v0.1",
            &a.id,
            json!({ "games": 7, "wins": 5, "losses": 2, "ladder": 40, "floor": 1, "streak": 3, "peakLadder": 40 }),
        );
        let other = rank_row(harness, "v0.1", &b.id, json!({ "games": 1, "wins": 1 }));
        q!(harness, t => t.ranked_put_rank(&first));
        q!(harness, t => t.ranked_put_rank(&other));

        assert_eq!(
            q!(harness, t => t.ranked_rank("v0.1", &a.id)),
            Some(first.clone())
        );
        assert_eq!(q!(harness, t => t.ranked_rank("v0.1", &b.id)), Some(other));
        assert!(q!(harness, t => t.ranked_rank("v0.2", &a.id)).is_none());
        assert!(q!(harness, t => t.ranked_rank("v0.1", &id())).is_none());

        // The second write for the same (season, profile) replaces the first.
        let moved: SeasonRank = from(spread(
            j(&first),
            json!({ "games": 8, "wins": 6, "ladder": 42, "streak": 4, "updatedAt": now + 1 }),
        ));
        q!(harness, t => t.ranked_put_rank(&moved));
        assert_eq!(q!(harness, t => t.ranked_rank("v0.1", &a.id)), Some(moved));
    }

    async fn r605_answers_a_season_s_standings_with_each_player_s_current_rating_in_profile_id_order(
        harness: &StoreHarness,
    ) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let now = harness.now();
        q!(harness, t => t.ranked_create_season(&season("v0.1", now)));
        let rank_a = rank_row(
            harness,
            "v0.1",
            &a.id,
            json!({ "games": 3, "wins": 3, "ladder": 45 }),
        );
        let rank_b = rank_row(
            harness,
            "v0.1",
            &b.id,
            json!({ "games": 3, "losses": 3, "ladder": 30 }),
        );
        q!(harness, t => t.ranked_put_rank(&rank_a));
        q!(harness, t => t.ranked_put_rank(&rank_b));
        q!(harness, t => t.profiles_set_glicko(&a.id, &glicko(1123.5)));
        q!(harness, t => t.profiles_set_glicko(&b.id, &glicko(877.25)));

        let standings: Vec<Value> = q!(harness, t => t.ranked_standings("v0.1"))
            .iter()
            .map(j)
            .collect();
        let profile_ids: Vec<String> = standings
            .iter()
            .map(|standing| must(standing["profileId"].as_str().map(str::to_string), "a profile id"))
            .collect();
        assert_eq!(profile_ids, sorted(&[a.id.clone(), b.id.clone()]));
        let of = |profile_id: &str| {
            standings
                .iter()
                .find(|standing| standing["profileId"] == profile_id)
                .cloned()
        };
        assert_eq!(of(&a.id), Some(spread(j(&rank_a), json!({ "rating": 1123.5 }))));
        assert_eq!(of(&b.id), Some(spread(j(&rank_b), json!({ "rating": 877.25 }))));
        assert!(q!(harness, t => t.ranked_standings("v0.2")).is_empty());
    }

    async fn r607_lists_a_profile_s_badges_oldest_season_first(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let now = harness.now();
        q!(harness, t => t.ranked_create_season(&season("v0.1", now)));
        q!(harness, t => t.ranked_create_season(&season("v0.2", now + 1_000)));
        q!(harness, t => t.ranked_put_rank(&rank_row(harness, "v0.2", &a.id, json!({ "games": 2 }))));
        q!(harness, t => t.ranked_put_rank(&rank_row(harness, "v0.1", &a.id, json!({ "games": 9, "peakLadder": 55 }))));
        let seasons: Vec<Value> = q!(harness, t => t.ranked_ranks_of(&a.id))
            .iter()
            .map(|row| j(row)["seasonId"].clone())
            .collect();
        assert_eq!(seasons, vec![json!("v0.1"), json!("v0.2")]);
        assert!(q!(harness, t => t.ranked_ranks_of(&id())).is_empty());
    }

    async fn r608_keeps_the_best_jlorious_position_the_season_has_held(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let now = harness.now();
        q!(harness, t => t.ranked_create_season(&season("v0.1", now)));
        q!(harness, t => t.ranked_put_rank(&rank_row(harness, "v0.1", &a.id, json!({}))));
        q!(harness, t => t.ranked_note_peak_jlorious("v0.1", &a.id, 17));
        q!(harness, t => t.ranked_note_peak_jlorious("v0.1", &a.id, 80));
        q!(harness, t => t.ranked_note_peak_jlorious("v0.1", &a.id, 4));
        assert_eq!(
            peak_jlorious(q!(harness, t => t.ranked_rank("v0.1", &a.id))),
            Some(4)
        );
        // A player with no season row is noted nowhere — the write is a no-op.
        q!(harness, t => t.ranked_note_peak_jlorious("v0.1", &id(), 1));
        q!(harness, t => t.ranked_note_peak_jlorious("v0.2", &a.id, 1));

        // putRank merges the peak rather than replacing it: a rank row the player's own game wrote
        // cannot undo a better position a bystander's game already recorded, and a null never
        // erases — but a better position the writer computed still lands.
        let row = j(&must(q!(harness, t => t.ranked_rank("v0.1", &a.id)), "the rank"));
        let worse: SeasonRank = from(spread(row.clone(), json!({ "peakJlorious": 90 })));
        q!(harness, t => t.ranked_put_rank(&worse));
        assert_eq!(
            peak_jlorious(q!(harness, t => t.ranked_rank("v0.1", &a.id))),
            Some(4)
        );
        let unknown: SeasonRank = from(spread(row.clone(), json!({ "peakJlorious": null })));
        q!(harness, t => t.ranked_put_rank(&unknown));
        assert_eq!(
            peak_jlorious(q!(harness, t => t.ranked_rank("v0.1", &a.id))),
            Some(4)
        );
        let better: SeasonRank = from(spread(row, json!({ "peakJlorious": 2 })));
        q!(harness, t => t.ranked_put_rank(&better));
        assert_eq!(
            peak_jlorious(q!(harness, t => t.ranked_rank("v0.1", &a.id))),
            Some(2)
        );
    }

    async fn r610_keeps_each_bot_s_own_rating_upserted(harness: &StoreHarness) {
        let now = harness.now();
        assert!(q!(harness, t => t.ranked_bot("ai-easy")).is_none());
        let rating: BotRating = from(json!({
            "botId": "ai-easy",
            "glicko": { "rating": 1050.5, "deviation": 300.25, "volatility": 0.06 },
            "games": 3,
            "updatedAt": now,
        }));
        q!(harness, t => t.ranked_put_bot(&rating));
        assert_eq!(q!(harness, t => t.ranked_bot("ai-easy")), Some(rating.clone()));
        let mut moved = j(&rating);
        moved["glicko"]["rating"] = json!(1036.25);
        moved["games"] = json!(4);
        moved["updatedAt"] = json!(now + 1);
        let moved: BotRating = from(moved);
        q!(harness, t => t.ranked_put_bot(&moved));
        assert_eq!(q!(harness, t => t.ranked_bot("ai-easy")), Some(moved));
        assert!(q!(harness, t => t.ranked_bot("ai-hard")).is_none());
    }

    async fn r611_records_a_rated_game_once_and_answers_it_back_whole(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let now = harness.now();
        q!(harness, t => t.ranked_create_season(&season("v0.1", now)));
        let row = rated_game(harness, &id(), "v0.1", &a, &b, json!({}));
        q!(harness, t => t.ranked_record_game(&row));
        assert_eq!(q!(harness, t => t.ranked_game(&row.id)), Some(row.clone()));
        assert!(q!(harness, t => t.ranked_game(&id())).is_none());
        // R262's rate-once guard is the row's own id.
        let refused = must(
            call!(harness, t => t.ranked_record_game(&row)).err(),
            "the second record's refusal",
        );
        assert!(
            refused
                .to_string()
                .contains(&format!("rated_games already holds a row for {}", row.id)),
            "{refused}"
        );
    }

    async fn r611_records_a_bot_side_with_no_profile_and_no_rank(harness: &StoreHarness) {
        let a = active_profile(harness, None).await;
        let now = harness.now();
        q!(harness, t => t.ranked_create_season(&season("v0.1", now)));
        let row = rated_game(
            harness,
            &id(),
            "v0.1",
            &a,
            &a,
            json!({
                "kind": "series",
                "reason": "decided",
                "winnerSide": 1,
                "sides": [
                    {
                        "profileId": a.id,
                        "botId": null,
                        "pilot": "human",
                        "before": j(&glicko(1000.0)),
                        "after": j(&glicko(984.0)),
                        "rankBefore": null,
                        "rankAfter": null,
                    },
                    {
                        "profileId": null,
                        "botId": "ai-easy",
                        "pilot": "ai",
                        "before": j(&glicko(1050.0)),
                        "after": j(&glicko(1062.0)),
                        "rankBefore": null,
                        "rankAfter": null,
                    },
                ],
            }),
        );
        q!(harness, t => t.ranked_record_game(&row));
        assert_eq!(q!(harness, t => t.ranked_game(&row.id)), Some(row));
    }

    async fn r609_reset_input_is_everyone_a_rated_game_touched_and_reset_ratings_writes_it(
        harness: &StoreHarness,
    ) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let c = active_profile(harness, None).await;
        let now = harness.now();
        q!(harness, t => t.ranked_create_season(&season("v0.1", now)));
        q!(harness, t => t.ranked_record_game(&rated_game(harness, &id(), "v0.1", &a, &b, json!({}))));

        let players: Vec<Value> = q!(harness, t => t.ranked_rated_players()).iter().map(j).collect();
        let profile_ids: Vec<String> = players
            .iter()
            .map(|player| must(player["profileId"].as_str().map(str::to_string), "a profile id"))
            .collect();
        assert_eq!(profile_ids, sorted(&[a.id.clone(), b.id.clone()]));
        let ratings: Vec<Value> = players.iter().map(|player| player["glicko"].clone()).collect();
        assert_eq!(ratings, vec![j(&glicko(1000.0)), j(&glicko(1000.0))]);

        // The soft reset's output lands on exactly the same profiles.
        let soft_a = glicko(1075.5);
        let soft_b = glicko(1037.75);
        let changes: [ResetChange; 2] = [
            from(json!({ "profileId": a.id, "before": j(&glicko(1000.0)), "after": j(&soft_a) })),
            from(json!({ "profileId": b.id, "before": j(&glicko(1000.0)), "after": j(&soft_b) })),
        ];
        q!(harness, t => t.ranked_reset_ratings(&changes));
        let rated_a = j(&must(q!(harness, t => t.profiles_get_by_id(&a.id)), "a"));
        assert_eq!(rated_a["rating"].as_f64(), Some(1075.5));
        assert_eq!(rated_a["ratingDeviation"].as_f64(), Some(350.0));
        assert_eq!(
            j(&must(q!(harness, t => t.profiles_get_by_id(&b.id)), "b"))["rating"].as_f64(),
            Some(1037.75)
        );
        // A player no rated game touched is untouched.
        assert_eq!(
            j(&must(q!(harness, t => t.profiles_get_by_id(&c.id)), "c"))["rating"].as_f64(),
            Some(1000.0)
        );
        // And an empty reset is legal (the first season's input can be empty).
        q!(harness, t => t.ranked_reset_ratings(&[]));
    }

    both_stores!(
        r609_opens_a_season_once_and_lists_every_season_oldest_first,
        r605_keeps_one_rank_row_per_player_per_season_put_rank_replacing_it,
        r605_answers_a_season_s_standings_with_each_player_s_current_rating_in_profile_id_order,
        r607_lists_a_profile_s_badges_oldest_season_first,
        r608_keeps_the_best_jlorious_position_the_season_has_held,
        r610_keeps_each_bot_s_own_rating_upserted,
        r611_records_a_rated_game_once_and_answers_it_back_whole,
        r611_records_a_bot_side_with_no_profile_and_no_rank,
        r609_reset_input_is_everyone_a_rated_game_touched_and_reset_ratings_writes_it,
    );
}

mod profiles_remove {
    use super::*;

    /// Account deletion.
    async fn removes_the_profile_and_its_own_rows_and_keeps_the_other_player_s_finished_match(
        harness: &StoreHarness,
    ) {
        let gone = active_profile(harness, None).await;
        let other = active_profile(harness, None).await;
        let now = harness.now();
        q!(harness, t => t.decks_upsert(&saved_deck(harness, &gone.id, json!({})), 10));
        let lessons: TutorialMergeInput =
            from(json!({ "profileId": gone.id, "completed": ["basics"], "hiddenChoice": null, "at": now }));
        q!(harness, t => t.tutorial_merge(&lessons, 32));
        let settings: PlayerSettingsMergeInput = from(json!({
            "profileId": gone.id,
            "groups": { "audio": { "at": 1, "values": { "master": 0.5 } } },
            "at": now,
        }));
        let limits: PlayerSettingsLimits = from(json!({ "maxGroups": 8, "maxBytes": 4096 }));
        q!(harness, t => t.player_settings_merge(&settings, &limits));
        let ticket: Ticket = from(json!({
            "id": id(),
            "profileId": gone.id,
            "rating": 1000,
            "mode": "bo1",
            "deck": deck_of(harness, 0),
            "trio": null,
            "catalogVersion": harness.catalog_version,
            "enqueuedAt": now,
            "status": "open",
            "matchId": null,
        }));
        q!(harness, t => t.tickets_insert(&ticket));
        let stats: IndexMap<String, Value> = from(json!({ "games": 1 }));
        q!(harness, t => t.player_stats_put(&gone.id, &stats, false, now));
        let room: Room = from(json!({
            "code": "QWERTZ",
            "hostProfileId": gone.id,
            "mode": "bo1",
            "hostDeck": deck_of(harness, 0),
            "hostTrio": null,
            "catalogVersion": harness.catalog_version,
            "createdAt": now,
            "expiresAt": now + 600_000,
            "guestProfileId": null,
            "matchId": null,
        }));
        q!(harness, t => t.rooms_create(&room));
        let attempt: CodeAttempt = from(json!({
            "profileId": gone.id,
            "ipHash": "ip-gone",
            "result": "rejected",
            "reason": "missing",
            "at": now,
        }));
        q!(harness, t => t.codes_log_attempt(&attempt));

        let match_id = id();
        q!(harness, t => t.matches_create(&match_row(&match_id, &gone.id, &other.id, harness, now)));
        q!(harness, t => t.matches_append_actions(&[action_row(&match_id, 1, "p1", "n1", now)]));
        let result: ResultRow = from(json!({
            "matchId": match_id,
            "players": [gone.id, other.id],
            "winnerProfileId": gone.id,
            "reason": "concede",
            "turns": 3,
            "endedAt": now,
            "ratingBefore": [1000, 1000],
            "ratingAfter": [1016, 984],
        }));
        q!(harness, t => t.results_insert(&result));
        q!(harness, t => t.matches_finish(&match_id, now));

        // R611's record of the same match (R611 keeps it for good, like `results`).
        let first_season: Season =
            from(json!({ "id": "v0.1", "patchVersion": "v0.1.1", "startedAt": now - 1 }));
        q!(harness, t => t.ranked_create_season(&first_season));
        let rated: RatedGameRow = from(json!({
            "id": match_id,
            "kind": "match",
            "seasonId": "v0.1",
            "patchVersion": "v0.1.1",
            "catalogVersion": harness.catalog_version,
            "sides": [
                { "profileId": gone.id, "botId": null, "pilot": "human", "before": { "rating": 1000, "deviation": 350, "volatility": 0.06 }, "after": { "rating": 1016, "deviation": 340, "volatility": 0.06 }, "rankBefore": null, "rankAfter": null },
                { "profileId": other.id, "botId": null, "pilot": "human", "before": { "rating": 1000, "deviation": 350, "volatility": 0.06 }, "after": { "rating": 984, "deviation": 340, "volatility": 0.06 }, "rankBefore": null, "rankAfter": null },
            ],
            "winnerSide": 0,
            "reason": "concede",
            "endedAt": now,
        }));
        q!(harness, t => t.ranked_record_game(&rated));

        assert!(q!(harness, t => t.profiles_remove(&gone.id)));
        assert!(!q!(harness, t => t.profiles_remove(&gone.id)));

        assert!(q!(harness, t => t.profiles_get_by_id(&gone.id)).is_none());
        assert!(q!(harness, t => t.decks_list(&gone.id)).is_empty());
        assert!(q!(harness, t => t.tutorial_get(&gone.id)).is_none());
        assert!(q!(harness, t => t.player_settings_get(&gone.id)).is_none());
        assert!(q!(harness, t => t.player_stats_get(&gone.id)).is_none());
        assert!(q!(harness, t => t.collection_get(&gone.id)).is_empty());
        assert!(q!(harness, t => t.tickets_open_for_profile(&gone.id)).is_none());
        assert!(q!(harness, t => t.rooms_get("QWERTZ")).is_none());
        assert_eq!(
            q!(harness, t => t.codes_count_attempts_by_profile(&gone.id, now - 60_000)),
            0
        );
        // The attempt itself stays for the per-IP limit.
        assert_eq!(
            q!(harness, t => t.codes_count_attempts_by_ip("ip-gone", now - 60_000)),
            1
        );

        // The other player keeps the log and the result, and the match they lost is still a loss.
        assert_eq!(q!(harness, t => t.matches_actions(&match_id)).len(), 1);
        assert!(q!(harness, t => t.results_get_by_match(&match_id)).is_some());
        assert_eq!(
            j(&q!(harness, t => t.results_record_for(&other.id))),
            json!({ "wins": 0, "losses": 1, "draws": 0 })
        );
        assert!(q!(harness, t => t.profiles_get_by_id(&other.id)).is_some());

        // The rated-game record stays whole too (R611): Postgres empties the deleted side's seat
        // (`on delete set null`) while the memory store keeps the id — KNOWN DIVERGENCES — so only
        // the record's survival and the other side are asserted across both.
        let kept = j(&must(
            q!(harness, t => t.ranked_game(&match_id)),
            "the rated-game record",
        ));
        assert_eq!(kept["sides"][1]["profileId"], other.id.as_str());
    }

    both_stores!(removes_the_profile_and_its_own_rows_and_keeps_the_other_player_s_finished_match);
}

mod purge_expired {
    use super::*;

    /// Retention.
    async fn deletes_attempts_and_finished_logs_older_than_the_cutoffs_and_nothing_newer(
        harness: &StoreHarness,
    ) {
        let a = active_profile(harness, None).await;
        let b = active_profile(harness, None).await;
        let now = harness.now();
        let day: i64 = 86_400_000;
        let old: CodeAttempt = from(json!({
            "profileId": a.id, "ipHash": "ip-old", "result": "rejected", "reason": "missing", "at": now - 31 * day,
        }));
        let new: CodeAttempt = from(json!({
            "profileId": a.id, "ipHash": "ip-new", "result": "rejected", "reason": "missing", "at": now - 29 * day,
        }));
        q!(harness, t => t.codes_log_attempt(&old));
        q!(harness, t => t.codes_log_attempt(&new));

        let (old_match, recent_match, live_match) = (id(), id(), id());
        for match_id in [&old_match, &recent_match, &live_match] {
            q!(harness, t => t.matches_create(&match_row(match_id, &a.id, &b.id, harness, now)));
            q!(harness, t => t.matches_append_actions(&[action_row(match_id, 1, "p1", "n1", now)]));
        }
        q!(harness, t => t.matches_finish(&old_match, now - 91 * day));
        q!(harness, t => t.matches_finish(&recent_match, now - 89 * day));

        let cutoffs: RetentionPurgeInput = from(json!({
            "codeAttemptsBefore": now - 30 * day,
            "matchActionsEndedBefore": now - 90 * day,
        }));
        let purged = q!(harness, t => t.purge_expired(&cutoffs));
        assert_eq!(j(&purged), json!({ "codeAttempts": 1, "matchActions": 1 }));
        assert_eq!(q!(harness, t => t.codes_count_attempts_by_ip("ip-old", 0)), 0);
        assert_eq!(q!(harness, t => t.codes_count_attempts_by_ip("ip-new", 0)), 1);
        assert!(q!(harness, t => t.matches_actions(&old_match)).is_empty());
        assert_eq!(q!(harness, t => t.matches_actions(&recent_match)).len(), 1);
        assert_eq!(q!(harness, t => t.matches_actions(&live_match)).len(), 1);
        assert!(q!(harness, t => t.matches_get(&old_match)).is_some());
    }

    both_stores!(deletes_attempts_and_finished_logs_older_than_the_cutoffs_and_nothing_newer);
}

mod tx {
    use super::*;

    /// The inner `store.tx` of the TS case. A Rust transaction is one `Tx` handle, so a nested
    /// transaction is the outer handle passed down: it can only join, never open a second one.
    async fn inner(t: &mut Tx<'_>, profile_id: &str) -> Result<(), String> {
        // TS wrote `setRating(profile.id, 1234)`; `setRating` is not ported (SURFACE §11.2).
        let rating: Glicko = from(json!({ "rating": 1234, "deviation": 350, "volatility": 0.06 }));
        t.profiles_set_glicko(profile_id, &rating)
            .await
            .map_err(|error| error.to_string())
    }

    async fn joins_a_nested_transaction_rather_than_opening_a_second_one(harness: &StoreHarness) {
        let profile = active_profile(harness, None).await;
        let failed: Result<(), String> = async {
            let mut t = harness.db.begin(None).await.map_err(|error| error.to_string())?;
            inner(&mut t, &profile.id).await?;
            // `t` is dropped here without a commit: the outer transaction rolls back.
            Err::<(), String>("outer fails after the inner one returned".to_string())
        }
        .await;
        assert!(must(failed.err(), "the outer failure").contains("outer fails"));

        // The inner `tx` must not have committed on its own.
        let after = j(&must(
            q!(harness, t => t.profiles_get_by_id(&profile.id)),
            "the profile",
        ));
        assert_eq!(after["rating"].as_f64(), Some(1000.0));
    }

    async fn returns_the_callback_s_value(harness: &StoreHarness) {
        let t = harness.db.begin(None).await.expect("begin");
        let value = 42;
        t.commit().await.expect("commit");
        assert_eq!(value, 42);
    }

    both_stores!(
        joins_a_nested_transaction_rather_than_opening_a_second_one,
        returns_the_callback_s_value,
    );
}
