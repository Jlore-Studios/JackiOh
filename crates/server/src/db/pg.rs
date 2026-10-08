//! The production store (SPEC §9.2's `API functions -> Postgres` edge), implemented over the
//! migrations in `crates/server/migrations` (0001-0027) with sqlx (← `apps/server/src/db/store.ts`,
//! SURFACE §11.1, §11.2).
//!
//! One free async fn per store method, named `<substore>_<method>` (the root `redeem` and
//! `purge_expired` keep their names), each taking the open transaction `Tx::Pg` holds;
//! `db::store` dispatches to them. `begin` and `commit` are `Db::begin` and `Tx::commit`'s Postgres
//! halves, and `create_postgres_store` builds the pool `Db::Pg` holds.
//!
//! ---------------------------------------------------------------------------
//! THREE RULES THIS FILE LIVES BY
//! ---------------------------------------------------------------------------
//!
//! 1. CALL THE `app.*` FUNCTIONS, DO NOT RE-DERIVE THEM. The migrations put the rules SPEC §9.4/§9.5
//!    call "one transaction", "one atomic statement" and "append-only" inside SECURITY DEFINER
//!    functions. Wherever the port's shape admits it, a method here is one call to one of them —
//!    `app.upsert_deck`, `app.upsert_trio`, `app.append_match_action`, `app.claim_ticket_pair`,
//!    `app.live_matches` — and the SQL stays the authority. Each method that does NOT reach an
//!    `app.*` function says why in its own comment; the report that came with this file lists them
//!    together.
//!
//! 2. THE SERVER IS `service_role`, AND NEVER THE OWNER. Every migration ends with the same
//!    sentence: "service_role: BYPASSRLS covers reads/writes to these tables; the EXECUTE grants
//!    below are what the API server and the match actor actually call." So the connection may be
//!    made as the migration owner (Supabase hands out `postgres` in `DATABASE_URL`), but no
//!    statement in this file ever RUNS as it: every transaction begins by switching to
//!    `service_role` and stamping `request.jwt.claim.sub` — the GUC `auth.uid()` reads — with the
//!    profile the call is about.
//!
//!    `SET LOCAL` is scoped to a transaction and is a silent no-op with a warning outside one
//!    (`SET LOCAL can only be used in transaction blocks`), which is exactly how a driver ends up
//!    quietly running as a superuser with RLS bypassed. This one therefore has no path that
//!    executes SQL outside a transaction: every method takes the `sqlx::Transaction` that `begin`
//!    opened (sqlx issues `BEGIN` when it hands one out), `begin` issues the role switch right
//!    after, every method re-stamps its own subject first (`run_as`), and `SESSION_SQL` is the only
//!    statement that sets either GUC. `tests/store/postgres.rs` asserts both from inside the store's
//!    own transaction, with a trigger that records `current_user` and `auth.uid()` as the store's
//!    statements see them.
//!
//! 3. `src/db/fake.rs` (TS's `src/api/e2e-store.ts` and `memory-stores.ts`) IS THE BEHAVIOURAL
//!    SPECIFICATION. It implements the same methods, the server suite runs against it, and its
//!    header lists the invariants §9.4 and §9.5 lean on. `tests/store/contract.rs` is those
//!    invariants as one suite, run against BOTH stores — the fake in `cargo test` and this one in
//!    `test:db`. Where the two genuinely cannot agree (Postgres is stricter, or the schema cannot
//!    hold something the port carries) the divergence is written down in `KNOWN DIVERGENCES` at
//!    the bottom of this file and asserted, not hidden.
//!
//! The SQL is `store.ts`'s, statement for statement, sent through `sqlx::query` with binds (never
//! the compile-time `query!` macros, which need a live database at build time). `store.ts` built
//! its statements by interpolating `ts("$n")`, `nullableTs("$n")` and the `*_COLUMNS` constants;
//! here those are the macros `ts!`, `nullable_ts!` and `*_columns!`, expanded by `concat!` at
//! compile time, so every statement is a `&'static str` carrying exactly the text TS sent.

use std::time::Duration;

use indexmap::IndexMap;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::types::Json;
use sqlx::{Postgres, Row, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use jackioh_engine::wire::{GameRecord, parse_game_record, portrait_or_default, sources_of};
use jackioh_engine::{Action, LastBoardEntry};

use crate::auth::is_uuid;
use crate::db::fake::to_public_player_summary;
use crate::db::store::{
    BotRating, CodeAttempt, CollectionEntry, CollectionGrant, FrozenDeck, FrozenTrio, GameRecordQuery,
    InviteCode, LastBoardKind, MatchActionRow, MatchClocks, MatchRow, MatchSeats, PerMode, PlayerSettingsGroup,
    PlayerSettingsLimits, PlayerSettingsMergeInput, PlayerSettingsMergeOutcome, PlayerSettingsRow,
    PlayerStatsListOptions, PlayerStatsRow, Profile, ProfileCreateInput, ProfileRecord, ProfileStatus,
    PublicPlayerSummary, QueueMode, RatedGameRow, RatedSide, RedeemInviteCodeInput, RedeemResult, ReplayRow, ResultRow,
    RetentionPurgeInput, RetentionPurgeResult, Room, SavedDeck, SavedTrio, Season, SeasonStanding, SeriesRow,
    SeriesSide, StoreError, Ticket, TicketStatus, TrioUpsertOutcome, TutorialHiddenChoice,
    TutorialMergeInput, TutorialMergeOutcome, TutorialProgressRow, UpsertOutcome,
};
use crate::ranked::glicko2::Glicko;
use crate::ranked::ladder::SeasonRank;
use crate::ranked::season::{ResetChange, ResetPlayer};

/// The open transaction every method here runs in: what `Tx::Pg` holds.
pub type PgTx<'a> = Transaction<'a, Postgres>;

/// The endings that always have a winner (`GameOverReason`, `crates/engine/src/wire/events.rs`): one
/// hero died, a player conceded, or a player's disconnect grace ran out. The other four
/// (`both-heroes-dead`, `draw-accepted`, `turn-cap`, `match-ceiling`) are draws. `results_record_for`
/// reads a winnerless decisive row as the deleted winner's (migration 0012).
const DECISIVE_REASONS: &[&str] = &["hero-death", "concede", "disconnect"];

// ---------------------------------------------------------------------------
// The role the server acts as
// ---------------------------------------------------------------------------

/// Migration 0001 §8, and the same closing note in 0002-0009: `service_role` is the role the API
/// server and the match actor hold. It is the only role granted EXECUTE on `app.redeem_invite_code`,
/// `app.upsert_deck`, `app.append_match_action`, `app.claim_ticket_pair` and the rest, so running
/// as it is not a formality — a call this file gets wrong fails with `insufficient_privilege`
/// instead of succeeding because the connection happened to own the table.
const ACTING_ROLE: &str = "service_role";

/// Postgres's `unique_violation`, the SQLSTATE `results_insert` translates on `results_pkey`.
const UNIQUE_VIOLATION: &str = "23505";

/// The primary key whose violation is the port's `StoreError::Duplicate`.
const RESULTS_PKEY: &str = "results_pkey";

/// `unique_violation` (23505) on `results_pkey` — the shape `results_insert` translates into the
/// port's `StoreError::Duplicate` (TS's `DuplicateResultError`). Checked structurally, on the
/// database error's code and constraint, so it holds however the driver wrapped the error.
fn is_results_key_conflict(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(held) => {
            held.code().as_deref() == Some(UNIQUE_VIOLATION) && held.constraint() == Some(RESULTS_PKEY)
        }
        _ => false,
    }
}

/// One statement, two `SET LOCAL`s. `set_config(name, value, true)` is `SET LOCAL name = value`,
/// and unlike `SET LOCAL` it takes parameters, so the profile id is bound rather than interpolated.
///
/// `request.jwt.claim.sub` is the GUC Supabase's `auth.uid()` reads (see
/// `tests/sql/00_supabase_stub.sql`, which stands the same function up for a plain Postgres). Every
/// RLS policy in 0002-0007 is `profile_id = app.current_profile_id()`, and `app.profile_is_active()`
/// reads `auth.uid()` directly, so a transaction that leaves it unset is a transaction where those
/// expressions silently see NULL. `service_role` carries BYPASSRLS, which means this cannot change
/// the result of anything here today; it is set anyway so that the day a policy, a trigger or a
/// SECURITY INVOKER helper does consult the caller, it sees the profile the call is about rather
/// than nobody.
const SESSION_SQL: &str =
    "select set_config('role', $1, true), set_config('request.jwt.claim.sub', $2, true)";

// ---------------------------------------------------------------------------
// Errors. Every failure leaves this file through these two, so the `StoreError` variants they
// name are the only thing about `db::store`'s error type this file depends on.
// ---------------------------------------------------------------------------

/// A driver or database failure, as the port's error.
fn db_error(error: sqlx::Error) -> StoreError {
    StoreError::Db(error)
}

// ---------------------------------------------------------------------------
// Value conversions. The port speaks epoch milliseconds and plain JSON; Postgres speaks
// timestamptz, jsonb, uuid and int8.
// ---------------------------------------------------------------------------

/// Nanoseconds in a millisecond: `OffsetDateTime` counts the one, the port the other.
const NANOS_PER_MS: i128 = 1_000_000;

/// `timestamptz` -> epoch ms. sqlx decodes it into an `OffsetDateTime` (TS: node-pg's `Date`, whose
/// `getTime()` drops the microseconds the same way).
fn ms_of(value: OffsetDateTime) -> i64 {
    (value.unix_timestamp_nanos() / NANOS_PER_MS) as i64
}

fn ms_or_null(value: Option<OffsetDateTime>) -> Option<i64> {
    value.map(ms_of)
}

/// Written as `to_timestamp($n / 1000.0)`; the SQL half of an epoch-ms parameter. TS's `ts(param)`,
/// expanded at compile time.
macro_rules! ts {
    ($param:literal) => {
        concat!("to_timestamp(", $param, "::double precision / 1000.0)")
    };
}

/// `to_timestamp` of a nullable epoch-ms parameter, which SQL cannot express inline. TS's
/// `nullableTs(param)` (one of its small helpers at the bottom), expanded at compile time; a macro
/// is defined before its first use, hence here.
macro_rules! nullable_ts {
    ($param:literal) => {
        concat!(
            "case when ",
            $param,
            "::double precision is null then null else ",
            ts!($param),
            " end"
        )
    };
}

/// An int8 that arrives as text (`seq::text`, the `::text` of `app.append_match_action`), because
/// TS read it so: it does not fit a JS number in general.
fn int_of(value: &str) -> Result<i64, StoreError> {
    value
        .trim()
        .parse::<i64>()
        .map_err(|_| StoreError::from(format!("expected an integer, got {}", stringify(&value))))
}

/// JS's `typeof`, for the messages `text_of` and friends write: what a JSON value would have been.
fn type_of(value: Option<&Value>) -> &'static str {
    match value {
        None => "undefined",
        Some(Value::Bool(_)) => "boolean",
        Some(Value::Number(_)) => "number",
        Some(Value::String(_)) => "string",
        Some(Value::Null | Value::Array(_) | Value::Object(_)) => "object",
    }
}

fn text_of(value: Option<&Value>) -> Result<String, StoreError> {
    match value {
        Some(Value::String(text)) => Ok(text.clone()),
        other => Err(StoreError::from(format!("expected text, got {}", type_of(other)))),
    }
}

fn card_list_of(value: &Value) -> Result<Vec<String>, StoreError> {
    let Some(entries) = value.as_array() else {
        return Err(StoreError::from("expected a jsonb array of card ids"));
    };
    entries.iter().map(|entry| text_of(Some(entry))).collect()
}

/// A frozen trio (R259) as `tickets.frozen_trio`, `matches.room_trio` and the series state hold it.
/// Migration 0008's shape checks guarantee three decks on the two columns; the rest is read back as
/// this file wrote it, and checked just far enough that a row from some other writer fails here, by
/// name, rather than as a missing field deep inside a series transition.
fn frozen_trio_of(value: &Value) -> Result<FrozenTrio, StoreError> {
    // JS's `typeof value !== "object" || value === null`: an array is an object there, and fails
    // one check later, at its missing `decks`.
    if !(value.is_object() || value.is_array()) {
        return Err(StoreError::from("expected a frozen trio object"));
    }
    let Some(decks) = value.get("decks").and_then(Value::as_array) else {
        return Err(StoreError::from("expected a frozen trio's decks array"));
    };
    let empty = Value::Object(serde_json::Map::new());
    let mut frozen: Vec<FrozenDeck> = Vec::with_capacity(decks.len());
    for deck in decks {
        // `deck ?? {}`.
        let entry = if deck.is_null() { &empty } else { deck };
        let name = text_of(entry.get("name"))?;
        let cards = card_list_of(entry.get("cards").unwrap_or(&Value::Null))?;
        // R642: the portrait freezes with the deck (ports.ts). Absent on rows frozen before
        // portraits existed, so absence stays absent and reads as `vanilla` downstream.
        // R642: absent stays absent, null stays null (ports.ts's `portrait?: string | null`).
        let portrait = match entry.get("portrait") {
            None => None,
            Some(Value::Null) => Some(None),
            Some(other) => Some(Some(text_of(Some(other))?)),
        };
        frozen.push(FrozenDeck {
            name,
            cards,
            portrait,
        });
    }
    let mut decks = frozen.into_iter();
    match (decks.next(), decks.next(), decks.next(), decks.next()) {
        (Some(first), Some(second), Some(third), None) => Ok(FrozenTrio {
            name: text_of(value.get("name"))?,
            decks: (first, second, third),
        }),
        _ => Err(StoreError::from("expected a frozen trio of exactly three decks")),
    }
}

fn trio_or_null(value: Option<&Value>) -> Result<Option<FrozenTrio>, StoreError> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(trio) => frozen_trio_of(trio).map(Some),
    }
}

/// `tickets.mode` and `matches.room_mode` both carry `check (... in ('bo1', 'bo3', 'random'))`.
const QUEUE_MODES: &[&str] = &["bo1", "bo3", "random"];

fn queue_mode_of(value: &str) -> Result<QueueMode, StoreError> {
    if QUEUE_MODES.contains(&value) {
        return from_literal(value);
    }
    Err(StoreError::from(format!(
        "expected a queue mode, got {}",
        stringify(&value)
    )))
}

/// The two stakes a rematch may carry: a normal game and double-or-nothing.
const STAKES: &[i16] = &[1, 2];

/// `matches.stake` carries `check (stake is null or stake in (1, 2))` (migration 0023, R672). Read
/// into whatever `MatchRow.stake` holds, through its serde form (TS's `1 | 2`).
fn stake_of<T: DeserializeOwned>(value: i16) -> Result<T, StoreError> {
    if STAKES.contains(&value) {
        return from_json(json!(value));
    }
    Err(StoreError::from(format!(
        "expected rematch stakes of 1 or 2, got {}",
        stringify(&value)
    )))
}

/// TS's `json(value)`: `JSON.stringify`, the text a `$n::jsonb` parameter is bound as.
fn json<T: Serialize + ?Sized>(value: &T) -> Result<String, StoreError> {
    serde_json::to_string(value).map_err(|error| StoreError::from(error.to_string()))
}

/// `value === null ? null : json(value)`.
fn json_or_null<T: Serialize>(value: &Option<T>) -> Result<Option<String>, StoreError> {
    value.as_ref().map(json).transpose()
}

/// `JSON.stringify` for an error message, which never fails.
fn stringify<T: Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| String::from("undefined"))
}

/// A string union's literal, as its serde form writes it (SURFACE §4.3: every union serialises as
/// TS's literal), so the column holds exactly the text TS bound.
fn literal<T: Serialize + ?Sized>(value: &T) -> Result<String, StoreError> {
    match serde_json::to_value(value) {
        Ok(Value::String(text)) => Ok(text),
        Ok(other) => Err(StoreError::from(format!(
            "expected a string literal, got {other}"
        ))),
        Err(error) => Err(StoreError::from(error.to_string())),
    }
}

/// A nullable union (`SeriesSeat | "draw" | null`, …): its literal, or null.
fn literal_or_null<T: Serialize + ?Sized>(value: &T) -> Result<Option<String>, StoreError> {
    match serde_json::to_value(value) {
        Ok(Value::Null) => Ok(None),
        Ok(Value::String(text)) => Ok(Some(text)),
        Ok(other) => Err(StoreError::from(format!(
            "expected a string literal or null, got {other}"
        ))),
        Err(error) => Err(StoreError::from(error.to_string())),
    }
}

/// A column's text read back as the string union it holds (TS's `value as T` after its check).
fn from_literal<T: DeserializeOwned>(text: &str) -> Result<T, StoreError> {
    from_json(Value::String(text.to_string()))
}

/// A jsonb value read back as the port type TS cast it to (`row.action as Action`, `state.games`,
/// a `VisibleRank`).
fn from_json<T: DeserializeOwned>(value: Value) -> Result<T, StoreError> {
    serde_json::from_value(value).map_err(|error| StoreError::from(error.to_string()))
}

/// A numeric literal union (`0 | 1 | null`) as the integer it serialises as, whatever Rust type
/// holds it.
fn int_or_null<T: Serialize + ?Sized>(value: &T) -> Result<Option<i64>, StoreError> {
    match serde_json::to_value(value) {
        Ok(Value::Null) => Ok(None),
        Ok(Value::Number(number)) => number
            .as_i64()
            .map(Some)
            .ok_or_else(|| StoreError::from(format!("expected an integer, got {number}"))),
        Ok(other) => Err(StoreError::from(format!(
            "expected an integer or null, got {other}"
        ))),
        Err(error) => Err(StoreError::from(error.to_string())),
    }
}

// ---------------------------------------------------------------------------
// Reading a row. sqlx decodes by the column's own type, strictly: a uuid column is a `Uuid`, a
// timestamptz an `OffsetDateTime`, a jsonb a `serde_json::Value`, an int4 an `i32`, an int8 an `i64`.
// ---------------------------------------------------------------------------

fn get<'r, T>(row: &'r PgRow, column: &str) -> Result<T, StoreError>
where
    T: sqlx::Decode<'r, Postgres> + sqlx::Type<Postgres>,
{
    row.try_get(column).map_err(db_error)
}

/// A `uuid` column as the text the port carries (lower case, hyphenated: Postgres's own text form).
fn uuid_text(row: &PgRow, column: &str) -> Result<String, StoreError> {
    Ok(get::<Uuid>(row, column)?.to_string())
}

fn uuid_text_or_null(row: &PgRow, column: &str) -> Result<Option<String>, StoreError> {
    Ok(get::<Option<Uuid>>(row, column)?.map(|id| id.to_string()))
}

/// A jsonb column read straight into a port type, so an object keeps the key order Postgres printed
/// it in (TS: node-pg's `JSON.parse` of the same text); `serde_json::Value` would sort the keys.
fn json_col<T: DeserializeOwned>(row: &PgRow, column: &str) -> Result<T, StoreError> {
    Ok(get::<Json<T>>(row, column)?.0)
}

// ---------------------------------------------------------------------------
// Sessions: one transaction per call, the role switched inside it
// ---------------------------------------------------------------------------

/// TS's `beginSession` and `Store.tx`, in one: `Db::begin`'s Postgres half.
///
/// ports.ts: "`tx` runs `fn` against a handle scoped to one database transaction and rolls back if
/// `fn` throws." This is what makes SPEC §9.4's "Redemption is one server-side transaction" and its
/// "writes `collection` and `collection_grants` in one transaction", and R263's "a game's result,
/// the series' record of it and, when the game ends the series, the rating move commit in one
/// transaction", true of the calls `src/api/**` makes: every statement between `begin` and
/// `commit` lands on the one connection the transaction holds. A transaction dropped without
/// `commit` rolls back (sqlx's `Transaction` does it on drop), which is TS's `rollback` in its
/// `catch`. `claim_sub` is the profile the transaction is about, or `None` for the calls that are
/// about nobody (`tickets_list_open`, `matches_live`, the breaker's `codes_count_failures`); each
/// method re-stamps its own subject anyway (`run_as`).
///
/// TS's `checkout`, `poolSession`, `joinedSession` and `withQuery` are not ported: they were the
/// `pg` driver's plumbing. A connection that breaks mid-transaction fails its own statement and
/// sqlx drops it rather than handing it to the next caller, which is what `checkout`'s `release(err)`
/// existed to do; nothing in sqlx re-throws an unhandled `error` event, which is what its listener
/// existed to stop. And there is no top-level, one-statement session: in Rust every call runs inside
/// a `Tx` its caller opened, so TS's "nested `tx` joins the enclosing transaction" is the only
/// path. A failed statement aborts that transaction in Postgres, so a caller that catches a store
/// error and goes on (queue.ts's duplicate-ticket insert, results.ts's retry after a duplicate
/// result) begins a fresh `Tx` for what follows, as each of TS's top-level calls ran in its own.
pub async fn begin(pool: &PgPool, claim_sub: Option<&str>) -> Result<PgTx<'static>, StoreError> {
    let mut t = pool.begin().await.map_err(db_error)?;
    run_as(&mut t, claim_sub).await?;
    Ok(t)
}

/// `Tx::commit`'s Postgres half: everything the transaction wrote lands together.
pub async fn commit(t: PgTx<'_>) -> Result<(), StoreError> {
    t.commit().await.map_err(db_error)
}

/// TS's `session.run(subject, …)` inside an open transaction: re-stamps the subject (the role is
/// already `service_role` and stays that way until commit). `subject` is the profile the call is
/// about — or `None` for the calls that are about nobody. Every statement the method issues after
/// it is on the same connection and commits or rolls back together — which is what a method like
/// `matches_create` (read-lock, then write) or `tickets_claim_pair` (claim, then link) needs to be
/// safe at all, and what makes a mid-method error leave nothing behind.
async fn run_as(t: &mut PgTx<'_>, subject: Option<&str>) -> Result<(), StoreError> {
    sqlx::query(SESSION_SQL)
        .bind(ACTING_ROLE)
        .bind(subject.unwrap_or(""))
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Row shapes
// ---------------------------------------------------------------------------

struct ProfileRow {
    id: String,
    status: String,
    rating: f64,
    rating_deviation: f64,
    rating_volatility: f64,
    current_match_id: Option<String>,
    created_at: OffsetDateTime,
    email: Option<String>,
    display_name: Option<String>,
}

impl ProfileRow {
    fn read(row: &PgRow) -> Result<ProfileRow, StoreError> {
        Ok(ProfileRow {
            id: uuid_text(row, "id")?,
            status: get(row, "status")?,
            rating: get(row, "rating")?,
            rating_deviation: get(row, "rating_deviation")?,
            rating_volatility: get(row, "rating_volatility")?,
            current_match_id: uuid_text_or_null(row, "current_match_id")?,
            created_at: get(row, "created_at")?,
            email: get(row, "email")?,
            display_name: get(row, "display_name")?,
        })
    }
}

/// TS's `PROFILE_COLUMNS`.
macro_rules! profile_columns {
    () => {
        "p.id, p.status, p.rating, p.rating_deviation, p.rating_volatility,
  p.current_match_id, p.created_at, u.email, p.display_name"
    };
}

/// TS's `PROFILE_FROM`.
macro_rules! profile_from {
    () => {
        "from public.profiles p left join auth.users u on u.id = p.id"
    };
}

/// The three `profiles.status` values.
const PROFILE_STATUSES: &[&str] = &["pending", "active", "banned"];

fn to_profile(row: ProfileRow) -> Result<Profile, StoreError> {
    let status = row.status;
    if !PROFILE_STATUSES.contains(&status.as_str()) {
        return Err(StoreError::from(format!(
            "profiles.status holds an unknown value: {status}"
        )));
    }
    let status: ProfileStatus = from_literal(&status)?;
    Ok(Profile {
        // SPEC §9.4's managed auth provider owns identity, and migration 0001 keys `profiles.id`
        // 1:1 to `auth.users(id)`. There is no second column: in this schema the profile id IS the
        // managed-auth user id, so `profiles_get_by_id` and `profiles_get_by_user_id` are the same
        // lookup.
        user_id: row.id.clone(),
        id: row.id,
        email: row.email.unwrap_or_default(),
        display_name: Some(row.display_name),
        status,
        rating: row.rating,
        // R603's Glicko triple, carried on the row since migration 0022.
        rating_deviation: row.rating_deviation,
        rating_volatility: row.rating_volatility,
        in_match_id: row.current_match_id,
        created_at: ms_of(row.created_at),
    })
}

struct MatchDbRow {
    id: String,
    status: String,
    seed: String,
    /// Null only for a seat whose account was deleted since (migration 0012).
    p1_profile_id: Option<String>,
    p2_profile_id: Option<String>,
    p1_deck: Value,
    p2_deck: Option<Value>,
    catalog_version: String,
    turn_deadline_at: Option<OffsetDateTime>,
    prompt_deadline_at: Option<OffsetDateTime>,
    p1_disconnected_at: Option<OffsetDateTime>,
    p2_disconnected_at: Option<OffsetDateTime>,
    ceiling_at: OffsetDateTime,
    created_at: OffsetDateTime,
    ended_at: Option<OffsetDateTime>,
    p1_last_board: Value,
    p2_last_board: Value,
    p1_glitch_board: Value,
    p2_glitch_board: Value,
    ranked: bool,
    p1_portrait: Option<String>,
    p2_portrait: Option<String>,
    /// R672 (migration 0023): the mode a rematch stated; null on every older row.
    mode: Option<String>,
    /// R672 (migration 0023): 2 on a double-or-nothing rematch; null is a normal game.
    stake: Option<i16>,
    /// R768 (migration 0027): the final state's hash, written with the result.
    final_hash: Option<String>,
}

impl MatchDbRow {
    fn read(row: &PgRow) -> Result<MatchDbRow, StoreError> {
        Ok(MatchDbRow {
            id: uuid_text(row, "id")?,
            status: get(row, "status")?,
            seed: get(row, "seed")?,
            p1_profile_id: uuid_text_or_null(row, "p1_profile_id")?,
            p2_profile_id: uuid_text_or_null(row, "p2_profile_id")?,
            p1_deck: get(row, "p1_deck")?,
            p2_deck: get(row, "p2_deck")?,
            catalog_version: get(row, "catalog_version")?,
            turn_deadline_at: get(row, "turn_deadline_at")?,
            prompt_deadline_at: get(row, "prompt_deadline_at")?,
            p1_disconnected_at: get(row, "p1_disconnected_at")?,
            p2_disconnected_at: get(row, "p2_disconnected_at")?,
            ceiling_at: get(row, "ceiling_at")?,
            created_at: get(row, "created_at")?,
            ended_at: get(row, "ended_at")?,
            p1_last_board: get(row, "p1_last_board")?,
            p2_last_board: get(row, "p2_last_board")?,
            p1_glitch_board: get(row, "p1_glitch_board")?,
            p2_glitch_board: get(row, "p2_glitch_board")?,
            ranked: get(row, "ranked")?,
            p1_portrait: get(row, "p1_portrait")?,
            p2_portrait: get(row, "p2_portrait")?,
            mode: get(row, "mode")?,
            stake: get(row, "stake")?,
            final_hash: get(row, "final_hash")?,
        })
    }
}

/// TS's `MATCH_COLUMNS`.
macro_rules! match_columns {
    () => {
        "id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
  catalog_version, turn_deadline_at, prompt_deadline_at, p1_disconnected_at, p2_disconnected_at,
  ceiling_at, created_at, ended_at, p1_last_board, p2_last_board, ranked, p1_portrait, p2_portrait,
  mode, stake, p1_glitch_board, p2_glitch_board, final_hash"
    };
}

/// R417: a stored board (`last_boards.board`, `matches.p*_last_board`), as migration 0017's CHECK
/// admits it. Each entry is taken as TS took it, field by field and unchecked.
fn last_board_of(value: &Value) -> Result<Vec<LastBoardEntry>, StoreError> {
    let Some(entries) = value.as_array() else {
        return Err(StoreError::from(format!(
            "a last board is not an array: {}",
            stringify(value)
        )));
    };
    Ok(entries
        .iter()
        .map(|entry| LastBoardEntry {
            def_id: entry
                .get("defId")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            radiant: entry.get("radiant").and_then(Value::as_bool).unwrap_or(false),
        })
        .collect())
}

fn to_match(row: MatchDbRow) -> Result<MatchRow, StoreError> {
    let Some(p2) = row.p2_profile_id else {
        // Only an `open` room reaches this, and `matches_get`/`matches_live` filter those out
        // before here.
        return Err(StoreError::from(format!(
            "match {} has no second player; it is still an open room",
            row.id
        )));
    };
    let status = from_literal(if row.status == "over" { "finished" } else { "live" })?;
    let boards = (
        last_board_of(&row.p1_last_board)?,
        last_board_of(&row.p2_last_board)?,
    );
    let glitch = (
        last_board_of(&row.p1_glitch_board)?,
        last_board_of(&row.p2_glitch_board)?,
    );
    let p2_deck = row.p2_deck.unwrap_or(Value::Null);
    // R642: absent when neither seat carried a portrait (a match from before 0019); both then
    // read as `vanilla` wherever `MatchRow.portraits` is consumed.
    let portraits = if row.p1_portrait.is_some() || row.p2_portrait.is_some() {
        Some((
            portrait_or_default(row.p1_portrait.as_deref()),
            portrait_or_default(row.p2_portrait.as_deref()),
        ))
    } else {
        None
    };
    Ok(MatchRow {
        id: row.id,
        seed: row.seed,
        // See KNOWN DIVERGENCES (deleted accounts): a deleted seat reads back as an empty id.
        players: (row.p1_profile_id.unwrap_or_default(), p2),
        decks: (card_list_of(&row.p1_deck)?, card_list_of(&p2_deck)?),
        catalog_version: row.catalog_version,
        status,
        created_at: ms_of(row.created_at),
        finished_at: ms_or_null(row.ended_at),
        clocks: MatchClocks {
            turn_deadline: ms_or_null(row.turn_deadline_at),
            prompt_deadline: ms_or_null(row.prompt_deadline_at),
            // See KNOWN DIVERGENCES (clocks): these two columns carry the per-player grace DEADLINE.
            grace_deadline: from_json(json!({
                "p1": ms_or_null(row.p1_disconnected_at),
                "p2": ms_or_null(row.p2_disconnected_at),
            }))?,
            ceiling_at: ms_of(row.ceiling_at),
        },
        // R417: absent when both are empty, as the registry writes it.
        last_boards: if boards.0.len() + boards.1.len() > 0 {
            Some(boards)
        } else {
            None
        },
        // R678: absent when both are empty, as the registry writes it (migration 0024).
        glitch_boards: if glitch.0.len() + glitch.1.len() > 0 {
            Some(glitch)
        } else {
            None
        },
        // R604: the flag migration 0022 adds. Absent when false, exactly as `MatchRow` types it —
        // `results.rs` reads a missing flag the same way (unranked).
        ranked: if row.ranked { Some(true) } else { None },
        // R672: migration 0023's columns. A null mode is "derive it" (`matches_mode_of` reads the
        // row first), and a null stake is a normal game — both absent exactly as `MatchRow` types
        // them.
        mode: row.mode.as_deref().map(queue_mode_of).transpose()?,
        stake: row.stake.map(stake_of).transpose()?,
        portraits,
        // R768 (migration 0027): absent until the result writes it.
        final_hash: row.final_hash,
    })
}

/// `public.matches` holds one `grace_deadline_at` plus `p1_disconnected_at` / `p2_disconnected_at`,
/// while `MatchClocks` (ports.ts) holds a deadline per player and no disconnect instant. The two
/// per-player columns therefore carry the deadline, and `grace_deadline_at` — the column migration
/// 0004 documents as "the grace countdown ... so both clients show it" — carries the nearer of the
/// two, so a SQL reader still finds the grace deadline where the migration says it is.
fn nearest_grace(clocks: &MatchClocks) -> Option<i64> {
    [clocks.grace_deadline.p1, clocks.grace_deadline.p2]
        .into_iter()
        .flatten()
        .min()
}

struct TicketRow {
    id: String,
    profile_id: String,
    rating: f64,
    mode: String,
    frozen_deck: Value,
    portrait: Option<String>,
    frozen_trio: Option<Value>,
    catalog_version: String,
    status: String,
    enqueued_at: OffsetDateTime,
    match_id: Option<String>,
}

impl TicketRow {
    fn read(row: &PgRow) -> Result<TicketRow, StoreError> {
        Ok(TicketRow {
            id: uuid_text(row, "id")?,
            profile_id: uuid_text(row, "profile_id")?,
            rating: get(row, "rating")?,
            mode: get(row, "mode")?,
            frozen_deck: get(row, "frozen_deck")?,
            portrait: get(row, "portrait")?,
            frozen_trio: get(row, "frozen_trio")?,
            catalog_version: get(row, "catalog_version")?,
            status: get(row, "status")?,
            enqueued_at: get(row, "enqueued_at")?,
            match_id: uuid_text_or_null(row, "match_id")?,
        })
    }
}

/// TS's `TICKET_COLUMNS`.
macro_rules! ticket_columns {
    () => {
        "id, profile_id, rating, mode, frozen_deck, portrait, frozen_trio, catalog_version, status,
  enqueued_at, match_id"
    };
}

/// `tickets.status` is queued/claimed/cancelled; the port calls the same three open/matched/cancelled.
fn to_ticket_status(value: &str) -> Result<TicketStatus, StoreError> {
    match value {
        "queued" => from_literal("open"),
        "claimed" => from_literal("matched"),
        "cancelled" => from_literal("cancelled"),
        _ => Err(StoreError::from(format!(
            "tickets.status holds an unknown value: {value}"
        ))),
    }
}

fn to_ticket(row: TicketRow) -> Result<Ticket, StoreError> {
    Ok(Ticket {
        id: row.id,
        profile_id: row.profile_id,
        rating: row.rating,
        mode: queue_mode_of(&row.mode)?,
        deck: card_list_of(&row.frozen_deck)?,
        portrait: Some(row.portrait),
        trio: trio_or_null(row.frozen_trio.as_ref())?,
        catalog_version: row.catalog_version,
        enqueued_at: ms_of(row.enqueued_at),
        status: to_ticket_status(&row.status)?,
        match_id: row.match_id,
    })
}

struct ResultDbRow {
    match_id: String,
    p1_profile_id: Option<String>,
    p2_profile_id: Option<String>,
    winner_profile_id: Option<String>,
    reason: String,
    turns: i32,
    p1_rating_before: f64,
    p1_rating_after: f64,
    p2_rating_before: f64,
    p2_rating_after: f64,
    ended_at: OffsetDateTime,
}

impl ResultDbRow {
    fn read(row: &PgRow) -> Result<ResultDbRow, StoreError> {
        Ok(ResultDbRow {
            match_id: uuid_text(row, "match_id")?,
            p1_profile_id: uuid_text_or_null(row, "p1_profile_id")?,
            p2_profile_id: uuid_text_or_null(row, "p2_profile_id")?,
            winner_profile_id: uuid_text_or_null(row, "winner_profile_id")?,
            reason: get(row, "reason")?,
            turns: get(row, "turns")?,
            p1_rating_before: get(row, "p1_rating_before")?,
            p1_rating_after: get(row, "p1_rating_after")?,
            p2_rating_before: get(row, "p2_rating_before")?,
            p2_rating_after: get(row, "p2_rating_after")?,
            ended_at: get(row, "ended_at")?,
        })
    }
}

fn to_result(row: ResultDbRow) -> Result<ResultRow, StoreError> {
    Ok(ResultRow {
        match_id: row.match_id,
        // See KNOWN DIVERGENCES (deleted accounts): a deleted seat reads back as an empty id.
        players: (
            row.p1_profile_id.unwrap_or_default(),
            row.p2_profile_id.unwrap_or_default(),
        ),
        winner_profile_id: row.winner_profile_id,
        // `results_reason_check` in migration 0004 pins this column to exactly the seven
        // `GameOverReason` strings of the engine's wire, so the parse restates a database constraint.
        reason: from_literal(&row.reason)?,
        turns: i64::from(row.turns),
        ended_at: ms_of(row.ended_at),
        rating_before: (row.p1_rating_before, row.p2_rating_before),
        rating_after: (row.p1_rating_after, row.p2_rating_after),
    })
}

struct RoomRow {
    id: String,
    room_code: String,
    room_mode: Option<String>,
    room_trio: Option<Value>,
    p1_profile_id: String,
    p2_profile_id: Option<String>,
    p1_deck: Value,
    p1_portrait: Option<String>,
    catalog_version: String,
    created_at: OffsetDateTime,
    ceiling_at: OffsetDateTime,
}

impl RoomRow {
    fn read(row: &PgRow) -> Result<RoomRow, StoreError> {
        Ok(RoomRow {
            id: uuid_text(row, "id")?,
            room_code: get(row, "room_code")?,
            room_mode: get(row, "room_mode")?,
            room_trio: get(row, "room_trio")?,
            p1_profile_id: uuid_text(row, "p1_profile_id")?,
            p2_profile_id: uuid_text_or_null(row, "p2_profile_id")?,
            p1_deck: get(row, "p1_deck")?,
            p1_portrait: get(row, "p1_portrait")?,
            catalog_version: get(row, "catalog_version")?,
            created_at: get(row, "created_at")?,
            ceiling_at: get(row, "ceiling_at")?,
        })
    }
}

/// TS's `ROOM_COLUMNS`.
macro_rules! room_columns {
    () => {
        "id, room_code, room_mode, room_trio, p1_profile_id, p2_profile_id, p1_deck,
  p1_portrait, catalog_version, created_at, ceiling_at"
    };
}

fn to_room(row: RoomRow) -> Result<Room, StoreError> {
    // A room's match id IS the row id, and it is only meaningful once a guest has claimed it.
    let match_id = if row.p2_profile_id.is_none() {
        None
    } else {
        Some(row.id.clone())
    };
    Ok(Room {
        code: row.room_code,
        host_profile_id: row.p1_profile_id,
        // R264. A room written before migration 0008, or by 0004's `app.create_room`, has no mode;
        // it could only ever have been a Best-of-1 room. See KNOWN DIVERGENCES (rooms).
        mode: match row.room_mode.as_deref() {
            None => from_literal("bo1")?,
            Some(mode) => queue_mode_of(mode)?,
        },
        host_deck: card_list_of(&row.p1_deck)?,
        // R642: the host's portrait waits in the open row beside the host's deck (migration 0019).
        host_portrait: Some(row.p1_portrait),
        host_trio: trio_or_null(row.room_trio.as_ref())?,
        catalog_version: row.catalog_version,
        created_at: ms_of(row.created_at),
        // See KNOWN DIVERGENCES (rooms): an unclaimed room keeps its joinable-until instant in
        // `ceiling_at`, the one column migration 0004 documents as meaningless while `status = 'open'`.
        expires_at: ms_of(row.ceiling_at),
        guest_profile_id: row.p2_profile_id,
        match_id,
    })
}

struct DeckRow {
    id: String,
    profile_id: String,
    name: String,
    cards: Value,
    portrait: Option<String>,
    catalog_version: String,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl DeckRow {
    fn read(row: &PgRow) -> Result<DeckRow, StoreError> {
        Ok(DeckRow {
            id: uuid_text(row, "id")?,
            profile_id: uuid_text(row, "profile_id")?,
            name: get(row, "name")?,
            cards: get(row, "cards")?,
            portrait: get(row, "portrait")?,
            catalog_version: get(row, "catalog_version")?,
            created_at: get(row, "created_at")?,
            updated_at: get(row, "updated_at")?,
        })
    }
}

/// TS's `DECK_COLUMNS`.
macro_rules! deck_columns {
    () => {
        "id, profile_id, name, cards, portrait, catalog_version, created_at, updated_at"
    };
}

fn to_deck(row: DeckRow) -> Result<SavedDeck, StoreError> {
    Ok(SavedDeck {
        id: row.id,
        profile_id: row.profile_id,
        name: row.name,
        // A jsonb array keeps the order it was written in, so this is the player's order (R250).
        cards: card_list_of(&row.cards)?,
        // R641: `null` is the default — `vanilla` — which a deck saved before 0019 also reads back as.
        portrait: row.portrait,
        catalog_version: row.catalog_version,
        created_at: ms_of(row.created_at),
        updated_at: ms_of(row.updated_at),
    })
}

struct TrioRow {
    id: String,
    profile_id: String,
    name: String,
    deck1_id: Option<String>,
    deck2_id: Option<String>,
    deck3_id: Option<String>,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
}

impl TrioRow {
    fn read(row: &PgRow) -> Result<TrioRow, StoreError> {
        Ok(TrioRow {
            id: uuid_text(row, "id")?,
            profile_id: uuid_text(row, "profile_id")?,
            name: get(row, "name")?,
            deck1_id: uuid_text_or_null(row, "deck1_id")?,
            deck2_id: uuid_text_or_null(row, "deck2_id")?,
            deck3_id: uuid_text_or_null(row, "deck3_id")?,
            created_at: get(row, "created_at")?,
            updated_at: get(row, "updated_at")?,
        })
    }
}

/// TS's `TRIO_COLUMNS`.
macro_rules! trio_columns {
    () => {
        "id, profile_id, name, deck1_id, deck2_id, deck3_id, created_at, updated_at"
    };
}

fn to_trio(row: TrioRow) -> SavedTrio {
    SavedTrio {
        id: row.id,
        profile_id: row.profile_id,
        name: row.name,
        deck_ids: (row.deck1_id, row.deck2_id, row.deck3_id),
        created_at: ms_of(row.created_at),
        updated_at: ms_of(row.updated_at),
    }
}

/// Exactly what `app.upsert_deck` returns (migration 0007); anything else is a schema this was not
/// built against.
const UPSERT_OUTCOMES: &[&str] = &["created", "updated", "limit", "not_owner"];
const TRIO_UPSERT_OUTCOMES: &[&str] = &["created", "updated", "limit", "not_owner", "unknown_deck"];

fn upsert_outcome_of<T: DeserializeOwned>(
    function: &str,
    allowed: &[&str],
    value: Option<String>,
) -> Result<T, StoreError> {
    if let Some(text) = value.as_deref()
        && allowed.contains(&text)
    {
        return from_literal(text);
    }
    Err(StoreError::from(format!(
        "{function} returned {}, which is not one of {} (migration 0007)",
        stringify(&value),
        allowed.join(", ")
    )))
}

/// `public.tutorial_progress` (migration 0011, R320).
struct TutorialDbRow {
    profile_id: String,
    completed: Vec<String>,
    hidden: Option<bool>,
    hidden_at: Option<OffsetDateTime>,
}

impl TutorialDbRow {
    fn read(row: &PgRow) -> Result<TutorialDbRow, StoreError> {
        Ok(TutorialDbRow {
            profile_id: uuid_text(row, "profile_id")?,
            completed: get(row, "completed")?,
            hidden: get(row, "hidden")?,
            hidden_at: get(row, "hidden_at")?,
        })
    }
}

/// TS's `TUTORIAL_COLUMNS`.
macro_rules! tutorial_columns {
    () => {
        "profile_id, completed, hidden, hidden_at"
    };
}

fn to_tutorial(row: TutorialDbRow) -> TutorialProgressRow {
    let at = ms_or_null(row.hidden_at);
    TutorialProgressRow {
        profile_id: row.profile_id,
        // sqlx decodes a text[] into a Vec<String>; the function keeps it sorted and each id once.
        completed: row.completed,
        // `tutorial_progress_choice_pair_check`: both or neither.
        hidden_choice: match (row.hidden, at) {
            (Some(hidden), Some(at)) => Some(TutorialHiddenChoice { hidden, at }),
            _ => None,
        },
    }
}

/// `public.player_settings` (migration 0018, R633): a jsonb object of groups, read in the order
/// Postgres prints it.
struct PlayerSettingsDbRow {
    profile_id: String,
    groups: IndexMap<String, PlayerSettingsGroup>,
}

impl PlayerSettingsDbRow {
    fn read(row: &PgRow) -> Result<PlayerSettingsDbRow, StoreError> {
        Ok(PlayerSettingsDbRow {
            profile_id: uuid_text(row, "profile_id")?,
            groups: json_col(row, "groups")?,
        })
    }
}

/// TS's `PLAYER_SETTINGS_COLUMNS`.
macro_rules! player_settings_columns {
    () => {
        "profile_id, groups"
    };
}

fn to_player_settings(row: PlayerSettingsDbRow) -> PlayerSettingsRow {
    PlayerSettingsRow {
        profile_id: row.profile_id,
        groups: row.groups,
    }
}

/// `public.player_stats` (migration 0021, R654): player stats and privacy flag.
struct PlayerStatsDbRow {
    profile_id: String,
    /// `(typeof stats === "object" && stats !== null ? stats : {})`: anything but an object reads
    /// as an empty one.
    stats: IndexMap<String, Value>,
    is_private: bool,
    updated_at: OffsetDateTime,
}

/// A `stats` column as an object, or `{}` when it is anything else (TS's typeof guard).
fn stats_col(row: &PgRow, column: &str) -> IndexMap<String, Value> {
    get::<Json<IndexMap<String, Value>>>(row, column)
        .map(|stats| stats.0)
        .unwrap_or_default()
}

impl PlayerStatsDbRow {
    fn read(row: &PgRow) -> Result<PlayerStatsDbRow, StoreError> {
        Ok(PlayerStatsDbRow {
            profile_id: uuid_text(row, "profile_id")?,
            stats: stats_col(row, "stats"),
            is_private: get(row, "is_private")?,
            updated_at: get(row, "updated_at")?,
        })
    }
}

/// TS's `PLAYER_STATS_COLUMNS`.
macro_rules! player_stats_columns {
    () => {
        "profile_id, stats, is_private, updated_at"
    };
}

fn to_player_stats(row: PlayerStatsDbRow) -> PlayerStatsRow {
    PlayerStatsRow {
        profile_id: row.profile_id,
        stats: row.stats,
        is_private: row.is_private,
        updated_at: ms_of(row.updated_at),
    }
}

/// `public.series` (migration 0009) keeps as columns what is queried or constrained — the two
/// players, the status, the next match id, the pick deadline, the version, the winner and the
/// timestamps — and everything a series only ever reads back whole in `state`. The players live in
/// the columns alone, so `state.sides` carries each side minus its `profileId` and the two cannot
/// drift apart.
///
/// TS's `SeriesState`: `{ sides: [SeriesSideState, SeriesSideState], games, seedBase, endReason,
/// ratingBefore, ratingAfter }`, with `SeriesSideState = { trio, wins, pick }`. Written by
/// `series_state_of` and read back by `to_series`.
fn series_state_of(row: &SeriesRow) -> Value {
    let side = |s: &SeriesSide| json!({ "trio": s.trio, "wins": s.wins, "pick": s.pick });
    json!({
        "sides": [side(&row.sides.0), side(&row.sides.1)],
        "games": row.games,
        "seedBase": row.seed_base,
        "endReason": row.end_reason,
        "ratingBefore": row.rating_before,
        "ratingAfter": row.rating_after,
    })
}

struct SeriesDbRow {
    id: String,
    p1_profile_id: Option<String>,
    p2_profile_id: Option<String>,
    status: String,
    next_match_id: String,
    pick_deadline_at: Option<OffsetDateTime>,
    version: i32,
    catalog_version: String,
    winner: Option<String>,
    state: Value,
    created_at: OffsetDateTime,
    updated_at: OffsetDateTime,
    ended_at: Option<OffsetDateTime>,
    ranked: bool,
}

impl SeriesDbRow {
    fn read(row: &PgRow) -> Result<SeriesDbRow, StoreError> {
        Ok(SeriesDbRow {
            id: uuid_text(row, "id")?,
            p1_profile_id: uuid_text_or_null(row, "p1_profile_id")?,
            p2_profile_id: uuid_text_or_null(row, "p2_profile_id")?,
            status: get(row, "status")?,
            next_match_id: uuid_text(row, "next_match_id")?,
            pick_deadline_at: get(row, "pick_deadline_at")?,
            version: get(row, "version")?,
            catalog_version: get(row, "catalog_version")?,
            winner: get(row, "winner")?,
            state: get(row, "state")?,
            created_at: get(row, "created_at")?,
            updated_at: get(row, "updated_at")?,
            ended_at: get(row, "ended_at")?,
            ranked: get(row, "ranked")?,
        })
    }
}

/// TS's `SERIES_COLUMNS`.
macro_rules! series_columns {
    () => {
        "id, p1_profile_id, p2_profile_id, status, next_match_id, pick_deadline_at, version,
  catalog_version, winner, state, created_at, updated_at, ended_at, ranked"
    };
}

/// `series_status_check` (0009).
const SERIES_STATUSES: &[&str] = &["picking", "playing", "over"];

fn series_status_of<T: DeserializeOwned>(value: &str) -> Result<T, StoreError> {
    if SERIES_STATUSES.contains(&value) {
        return from_literal(value);
    }
    Err(StoreError::from(format!(
        "series.status holds an unknown value: {value}"
    )))
}

/// `series_winner_check` (0009).
const SERIES_WINNERS: &[&str] = &["p1", "p2", "draw"];

fn series_winner_of<T: DeserializeOwned>(value: Option<&str>) -> Result<T, StoreError> {
    match value {
        None => from_json(Value::Null),
        Some(winner) if SERIES_WINNERS.contains(&winner) => from_literal(winner),
        Some(winner) => Err(StoreError::from(format!(
            "series.winner holds an unknown value: {winner}"
        ))),
    }
}

/// A key of the series state as the port type it holds, `null` when absent (TS's `state.x ?? null`).
fn state_field<T: DeserializeOwned>(state: &Value, key: &str) -> Result<T, StoreError> {
    from_json(state.get(key).cloned().unwrap_or(Value::Null))
}

fn to_series(row: SeriesDbRow) -> Result<SeriesRow, StoreError> {
    let state = &row.state;
    let sides = state
        .get("sides")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let (first, second) = match sides {
        [first, second] if !state.is_null() && state.get("games").is_some_and(Value::is_array) => {
            (first, second)
        }
        _ => {
            return Err(StoreError::from(format!(
                "series {} has a state this store did not write",
                row.id
            )));
        }
    };
    let side = |profile_id: Option<String>, held: &Value| -> Result<SeriesSide, StoreError> {
        Ok(SeriesSide {
            // See KNOWN DIVERGENCES (deleted accounts): a deleted seat reads back as an empty id.
            profile_id: profile_id.unwrap_or_default(),
            trio: frozen_trio_of(held.get("trio").unwrap_or(&Value::Null))?,
            wins: state_field(held, "wins")?,
            pick: state_field(held, "pick")?,
        })
    };
    Ok(SeriesRow {
        sides: (side(row.p1_profile_id, first)?, side(row.p2_profile_id, second)?),
        catalog_version: row.catalog_version,
        seed_base: text_of(state.get("seedBase"))?,
        status: series_status_of(&row.status)?,
        games: state_field(state, "games")?,
        next_match_id: row.next_match_id,
        pick_deadline: ms_or_null(row.pick_deadline_at),
        winner: series_winner_of(row.winner.as_deref())?,
        end_reason: state_field(state, "endReason")?,
        rating_before: state_field(state, "ratingBefore")?,
        rating_after: state_field(state, "ratingAfter")?,
        created_at: ms_of(row.created_at),
        updated_at: ms_of(row.updated_at),
        ended_at: ms_or_null(row.ended_at),
        version: i64::from(row.version),
        // R604: the flag migration 0022 adds. Absent when false, exactly as `SeriesRow` types it —
        // `series.rs` reads a missing flag the same way (unranked, so it never rates).
        ranked: if row.ranked { Some(true) } else { None },
        id: row.id,
    })
}

// ---------------------------------------------------------------------------
// The ranked ladder's rows (SPEC §9.12, migration 0022)
// ---------------------------------------------------------------------------

struct SeasonDbRow {
    id: String,
    patch_version: String,
    started_at: OffsetDateTime,
}

impl SeasonDbRow {
    fn read(row: &PgRow) -> Result<SeasonDbRow, StoreError> {
        Ok(SeasonDbRow {
            id: get(row, "id")?,
            patch_version: get(row, "patch_version")?,
            started_at: get(row, "started_at")?,
        })
    }
}

fn to_season(row: SeasonDbRow) -> Season {
    Season {
        id: row.id,
        patch_version: row.patch_version,
        started_at: ms_of(row.started_at),
    }
}

struct SeasonRankDbRow {
    season_id: String,
    profile_id: String,
    games: i32,
    wins: i32,
    losses: i32,
    draws: i32,
    ladder: Option<i32>,
    floor: i32,
    streak: i32,
    peak_ladder: Option<i32>,
    peak_jlorious: Option<i32>,
    updated_at: OffsetDateTime,
}

impl SeasonRankDbRow {
    fn read(row: &PgRow) -> Result<SeasonRankDbRow, StoreError> {
        Ok(SeasonRankDbRow {
            season_id: get(row, "season_id")?,
            profile_id: uuid_text(row, "profile_id")?,
            games: get(row, "games")?,
            wins: get(row, "wins")?,
            losses: get(row, "losses")?,
            draws: get(row, "draws")?,
            ladder: get(row, "ladder")?,
            floor: get(row, "floor")?,
            streak: get(row, "streak")?,
            peak_ladder: get(row, "peak_ladder")?,
            peak_jlorious: get(row, "peak_jlorious")?,
            updated_at: get(row, "updated_at")?,
        })
    }
}

fn to_season_rank(row: SeasonRankDbRow) -> SeasonRank {
    SeasonRank {
        season_id: row.season_id,
        profile_id: row.profile_id,
        games: row.games,
        wins: row.wins,
        losses: row.losses,
        draws: row.draws,
        ladder: row.ladder,
        floor: row.floor,
        streak: row.streak,
        peak_ladder: row.peak_ladder,
        peak_jlorious: row.peak_jlorious,
        updated_at: ms_of(row.updated_at),
    }
}

struct BotRatingDbRow {
    bot_id: String,
    rating: f64,
    deviation: f64,
    volatility: f64,
    games: i32,
    updated_at: OffsetDateTime,
}

impl BotRatingDbRow {
    fn read(row: &PgRow) -> Result<BotRatingDbRow, StoreError> {
        Ok(BotRatingDbRow {
            bot_id: get(row, "bot_id")?,
            rating: get(row, "rating")?,
            deviation: get(row, "deviation")?,
            volatility: get(row, "volatility")?,
            games: get(row, "games")?,
            updated_at: get(row, "updated_at")?,
        })
    }
}

fn to_bot_rating(row: BotRatingDbRow) -> BotRating {
    BotRating {
        bot_id: row.bot_id,
        glicko: Glicko {
            rating: row.rating,
            deviation: row.deviation,
            volatility: row.volatility,
        },
        games: i64::from(row.games),
        updated_at: ms_of(row.updated_at),
    }
}

struct RatedGameDbRow {
    id: String,
    kind: String,
    season_id: String,
    patch_version: String,
    catalog_version: String,
    p1_profile_id: Option<String>,
    p1_bot_id: Option<String>,
    p1_pilot: String,
    p1_before: Value,
    p1_after: Value,
    p1_rank_before: Option<Value>,
    p1_rank_after: Option<Value>,
    p2_profile_id: Option<String>,
    p2_bot_id: Option<String>,
    p2_pilot: String,
    p2_before: Value,
    p2_after: Value,
    p2_rank_before: Option<Value>,
    p2_rank_after: Option<Value>,
    winner_side: Option<i16>,
    reason: String,
    ended_at: OffsetDateTime,
}

impl RatedGameDbRow {
    fn read(row: &PgRow) -> Result<RatedGameDbRow, StoreError> {
        Ok(RatedGameDbRow {
            id: uuid_text(row, "id")?,
            kind: get(row, "kind")?,
            season_id: get(row, "season_id")?,
            patch_version: get(row, "patch_version")?,
            catalog_version: get(row, "catalog_version")?,
            p1_profile_id: uuid_text_or_null(row, "p1_profile_id")?,
            p1_bot_id: get(row, "p1_bot_id")?,
            p1_pilot: get(row, "p1_pilot")?,
            p1_before: get(row, "p1_before")?,
            p1_after: get(row, "p1_after")?,
            p1_rank_before: get(row, "p1_rank_before")?,
            p1_rank_after: get(row, "p1_rank_after")?,
            p2_profile_id: uuid_text_or_null(row, "p2_profile_id")?,
            p2_bot_id: get(row, "p2_bot_id")?,
            p2_pilot: get(row, "p2_pilot")?,
            p2_before: get(row, "p2_before")?,
            p2_after: get(row, "p2_after")?,
            p2_rank_before: get(row, "p2_rank_before")?,
            p2_rank_after: get(row, "p2_rank_after")?,
            winner_side: get(row, "winner_side")?,
            reason: get(row, "reason")?,
            ended_at: get(row, "ended_at")?,
        })
    }
}

/// TS's `RATED_GAME_COLUMNS`.
macro_rules! rated_game_columns {
    () => {
        "id, kind, season_id, patch_version, catalog_version,
  p1_profile_id, p1_bot_id, p1_pilot, p1_before, p1_after, p1_rank_before, p1_rank_after,
  p2_profile_id, p2_bot_id, p2_pilot, p2_before, p2_after, p2_rank_before, p2_rank_after,
  winner_side, reason, ended_at"
    };
}

/// Advisory lock id for `ranked_lock_seasons` — every season open takes it, so two opens racing
/// in different transactions (even for different season ids) serialize instead of both
/// soft-resetting off a seasons list that lacks the other's row. Distinct from migrate.rs's
/// "jack" id so a deploy and a season open never wait on each other.
const SEASON_LOCK_ID: i64 = 0x7365_6173; // "seas"

/// A Glicko triple as `rated_games.p*_before` / `p*_after` jsonb holds it — this file wrote it.
fn glicko_of(value: &Value) -> Result<Glicko, StoreError> {
    let number = |key: &str| value.get(key).and_then(Value::as_f64);
    match (number("rating"), number("deviation"), number("volatility")) {
        (Some(rating), Some(deviation), Some(volatility)) => Ok(Glicko {
            rating,
            deviation,
            volatility,
        }),
        _ => Err(StoreError::from(format!(
            "expected a Glicko triple, got {}",
            stringify(value)
        ))),
    }
}

/// A `VisibleRank` (ladder.rs) as `rated_games.p*_rank_*` jsonb holds it; null for a bot's side.
fn visible_rank_of<T: DeserializeOwned>(value: Option<Value>) -> Result<T, StoreError> {
    match value {
        None | Some(Value::Null) => from_json(Value::Null),
        Some(rank) if rank.is_object() || rank.is_array() => from_json(rank),
        Some(rank) => Err(StoreError::from(format!(
            "expected a VisibleRank, got {}",
            stringify(&rank)
        ))),
    }
}

/// The two pilots a rated side may have.
const PILOTS: &[&str] = &["human", "ai"];

fn pilot_of<T: DeserializeOwned>(value: &str) -> Result<T, StoreError> {
    if PILOTS.contains(&value) {
        return from_literal(value);
    }
    Err(StoreError::from(format!("expected a pilot, got {value}")))
}

fn to_rated_side(
    profile_id: Option<String>,
    bot_id: Option<String>,
    pilot: &str,
    before: &Value,
    after: &Value,
    rank_before: Option<Value>,
    rank_after: Option<Value>,
) -> Result<RatedSide, StoreError> {
    Ok(RatedSide {
        profile_id,
        bot_id,
        pilot: pilot_of(pilot)?,
        before: glicko_of(before)?,
        after: glicko_of(after)?,
        rank_before: visible_rank_of(rank_before)?,
        rank_after: visible_rank_of(rank_after)?,
    })
}

/// `rated_games.kind`'s two values.
const RATED_GAME_KINDS: &[&str] = &["match", "series"];

fn to_rated_game(row: RatedGameDbRow) -> Result<RatedGameRow, StoreError> {
    if !RATED_GAME_KINDS.contains(&row.kind.as_str()) {
        return Err(StoreError::from(format!(
            "rated_games.kind holds an unknown value: {}",
            row.kind
        )));
    }
    if let Some(side) = row.winner_side
        && side != 0
        && side != 1
    {
        return Err(StoreError::from(format!(
            "rated_games.winner_side holds an unknown value: {side}"
        )));
    }
    Ok(RatedGameRow {
        id: row.id,
        kind: from_literal(&row.kind)?,
        season_id: row.season_id,
        patch_version: row.patch_version,
        catalog_version: row.catalog_version,
        sides: (
            to_rated_side(
                row.p1_profile_id,
                row.p1_bot_id,
                &row.p1_pilot,
                &row.p1_before,
                &row.p1_after,
                row.p1_rank_before,
                row.p1_rank_after,
            )?,
            to_rated_side(
                row.p2_profile_id,
                row.p2_bot_id,
                &row.p2_pilot,
                &row.p2_before,
                &row.p2_after,
                row.p2_rank_before,
                row.p2_rank_after,
            )?,
        ),
        winner_side: from_json(json!(row.winner_side))?,
        // `rated_games_reason_check` (0022) pins the column to the two reason sets, so the parse
        // restates a database constraint, as `to_result`'s does.
        reason: from_literal(&row.reason)?,
        ended_at: ms_of(row.ended_at),
    })
}

// ---------------------------------------------------------------------------
// The store
// ---------------------------------------------------------------------------

/// Maximum pooled connections when the caller names none. Small by default: every call is one
/// short transaction.
const POOL_MAX_DEFAULT: u32 = 10;

/// Supabase's Supavisor closes a client that has been idle on its side, and the driver does not know
/// until it tries to use it — which surfaces as `read ETIMEDOUT` on a request that did nothing
/// wrong. Recycling an idle connection after 10 s means the pool retires connections before the
/// pooler does, so a checkout is far more likely to hand back a live socket.
const POOL_IDLE_TIMEOUT_MS: u64 = 10_000;

/// Fail a checkout that cannot get a connection rather than hanging the request forever.
const POOL_ACQUIRE_TIMEOUT_MS: u64 = 10_000;

/// TS's `PostgresStoreOptions`.
#[derive(Clone, Debug)]
pub struct PostgresStoreOptions {
    /// A direct Postgres connection string (`DATABASE_URL`; SPEC §9.2's Postgres edge).
    pub connection_string: String,
    /// Maximum pooled connections. Small by default: every call is one short transaction.
    pub max: Option<u32>,
}

/// Exactly the strings `app.redeem_invite_code` returns (migration 0001 §6) — defined by the port,
/// because both stores answer with them. Anything else out of the function is a schema this store
/// was not built against.
const REDEEM_RESULTS: &[&str] = &[
    "ok",
    "not_pending",
    "email_unverified",
    "rate_limited_profile",
    "rate_limited_ip",
    "circuit_open",
    "invalid_code",
];

fn to_redeem_result(value: Option<String>) -> Result<RedeemResult, StoreError> {
    if let Some(text) = value.as_deref()
        && REDEEM_RESULTS.contains(&text)
    {
        return from_literal(text);
    }
    Err(StoreError::from(format!(
        "app.redeem_invite_code returned {}, which is not one of {} (migration 0001 §6)",
        stringify(&value),
        REDEEM_RESULTS.join(", ")
    )))
}

/// The pool `Db::Pg` holds. `app.rs` calls this with `{ connection_string: env.DATABASE_URL }`.
///
/// The connection string is checked here rather than on first use so that a misconfigured
/// deployment fails at boot with a sentence naming the variable, and so that end-to-end mode's
/// `DATABASE_URL=memory://e2e-fixture-store` placeholder can never be mistaken for a database. The
/// pool connects lazily, as `pg`'s did: the first transaction opens the first connection.
///
/// An idle connection in the pool can be closed by the *server* at any time — Supabase's Supavisor
/// does it on its own idle timeout, and any network blip does it too. TS needed a `pool.on("error")`
/// handler (and its `onError` option) so that Node would not re-throw that event and kill the
/// process; sqlx retires a broken connection itself, the next checkout opens a fresh one, and no
/// query is lost — a query that was in flight fails at its own call site, which is where the caller
/// can do something about it. `pg`'s `keepAlive: true` has no sqlx option; the idle timeout below
/// is what keeps a quiet connection from going stale.
pub fn create_postgres_store(options: &PostgresStoreOptions) -> Result<PgPool, StoreError> {
    assert_postgres_url(&options.connection_string)?;
    PgPoolOptions::new()
        .max_connections(options.max.unwrap_or(POOL_MAX_DEFAULT))
        .idle_timeout(Duration::from_millis(POOL_IDLE_TIMEOUT_MS))
        .acquire_timeout(Duration::from_millis(POOL_ACQUIRE_TIMEOUT_MS))
        .connect_lazy(&options.connection_string)
        .map_err(db_error)
}

/// Closes the pool. Nothing in `app.rs` calls it; tests and a graceful shutdown do.
pub async fn close(pool: &PgPool) {
    pool.close().await;
}

/// How much of a refused connection string the refusal quotes.
const URL_PREVIEW_CHARS: usize = 16;

pub(crate) fn assert_postgres_url(connection_string: &str) -> Result<(), StoreError> {
    let trimmed = connection_string.trim().to_ascii_lowercase();
    let ok = trimmed.starts_with("postgres://") || trimmed.starts_with("postgresql://");
    if !ok {
        let preview: String = connection_string.chars().take(URL_PREVIEW_CHARS).collect();
        return Err(StoreError::from(format!(
            "DATABASE_URL must be a Postgres connection string (postgres://... or postgresql://...), \
             got {}…. The in-memory store of src/db/fake.rs is reachable only with E2E=1.",
            stringify(&preview)
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Redemption (SPEC §9.4) — rule 1 of this file's header, at its clearest: the whole six-step
// transaction is one `app.*` call and the SQL stays the authority.
// ---------------------------------------------------------------------------

/// ports.ts, `Store.redeem`: SPEC §9.4's redemption, whole. `app.redeem_invite_code` (migration
/// 0001 §6) holds all six steps under one profile row lock and one code row lock, never raises
/// for an expected rejection, and returns one of seven strings — which is why the port's result
/// type is those strings and nothing friendlier.
///
/// It runs in the caller's transaction and stamps the subject like every other method here, so a
/// caller that has already opened a transaction gets the function's work committed with theirs
/// rather than beside it.
///
/// A `None` `code_hash` is passed through as SQL NULL on purpose: `where code_hash = null` matches
/// no row, so a code that could never exist is refused by the same lookup that refuses one that
/// was simply never minted — after the attempt has been logged, which is what ports.ts asks for.
pub async fn redeem(t: &mut PgTx<'_>, input: &RedeemInviteCodeInput) -> Result<RedeemResult, StoreError> {
    run_as(t, Some(input.profile_id.as_str())).await?;
    let row = sqlx::query("select app.redeem_invite_code($1::uuid, $2::text, $3::text)")
        .bind(input.profile_id.as_str())
        .bind(input.code_hash.as_deref())
        .bind(input.ip_hash.as_str())
        .fetch_optional(&mut **t)
        .await
        .map_err(db_error)?;
    let Some(row) = row else {
        return Err(StoreError::from("app.redeem_invite_code returned no row"));
    };
    to_redeem_result(get(&row, "redeem_invite_code")?)
}

/// `redeem` under the name `tests/store/postgres.rs` drives it by, kept because that suite is about
/// `app.redeem_invite_code` specifically rather than about the port. It is the same function;
/// `code_hash` is narrowed to text because a spec that means "no such code" says so with a hash
/// that matches nothing.
pub async fn redeem_invite_code(
    t: &mut PgTx<'_>,
    profile_id: &str,
    code_hash: &str,
    ip_hash: &str,
) -> Result<RedeemResult, StoreError> {
    redeem(
        t,
        &RedeemInviteCodeInput {
            profile_id: profile_id.to_string(),
            code_hash: Some(code_hash.to_string()),
            ip_hash: ip_hash.to_string(),
        },
    )
    .await
}

/// The retention purge, as one call to `app.purge_expired_rows` (migration 0013): the only path
/// `match_actions`' append-only guard lets a delete through.
pub async fn purge_expired(
    t: &mut PgTx<'_>,
    input: &RetentionPurgeInput,
) -> Result<RetentionPurgeResult, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select code_attempts, match_actions from app.purge_expired_rows(",
        ts!("$1"),
        ", ",
        ts!("$2"),
        ")"
    ))
    .bind(input.code_attempts_before)
    .bind(input.match_actions_ended_before)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let Some(row) = row else {
        return Err(StoreError::from("app.purge_expired_rows returned no row"));
    };
    Ok(RetentionPurgeResult {
        code_attempts: get::<i64>(&row, "code_attempts")?,
        match_actions: get::<i64>(&row, "match_actions")?,
    })
}

// ---------------------------------------------------------------------------
// Profiles (SPEC §9.4)
// ---------------------------------------------------------------------------

pub async fn profiles_get_by_id(t: &mut PgTx<'_>, profile_id: &str) -> Result<Option<Profile>, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let row = sqlx::query(concat!(
        "select ",
        profile_columns!(),
        " ",
        profile_from!(),
        " where p.id = $1::uuid"
    ))
    .bind(profile_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| ProfileRow::read(&row).and_then(to_profile))
        .transpose()
}

/// The same lookup as `profiles_get_by_id`: migration 0001 keys `profiles.id` to `auth.users(id)`,
/// so the managed-auth user id and the profile id are one value (see `to_profile`).
pub async fn profiles_get_by_user_id(t: &mut PgTx<'_>, user_id: &str) -> Result<Option<Profile>, StoreError> {
    run_as(t, Some(user_id)).await?;
    let row = sqlx::query(concat!(
        "select ",
        profile_columns!(),
        " ",
        profile_from!(),
        " where p.id = $1::uuid"
    ))
    .bind(user_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| ProfileRow::read(&row).and_then(to_profile))
        .transpose()
}

pub async fn profiles_get_many(t: &mut PgTx<'_>, profile_ids: &[String]) -> Result<Vec<Profile>, StoreError> {
    if profile_ids.is_empty() {
        return Ok(Vec::new());
    }
    run_as(t, None).await?;
    let rows = sqlx::query(concat!(
        "select ",
        profile_columns!(),
        " ",
        profile_from!(),
        " where p.id = any($1::uuid[])"
    ))
    .bind(profile_ids.to_vec())
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| ProfileRow::read(row).and_then(to_profile))
        .collect()
}

/// §9.4: "the account exists the moment auth says so and stays pending until a code is
/// redeemed" (`resolve_caller` in http.rs). In a real Supabase project migration 0001's
/// `on_auth_user_created` trigger has usually made this row already, in which case
/// `profiles_get_by_user_id` finds it and this is never called; the insert is the path for a
/// project whose trigger has not run (a user created before the migration, say).
///
/// `email` is accepted and ignored: `public.profiles` has no email column — §9.4's managed auth
/// provider owns it on `auth.users`, which is where every read here takes it from. The insert
/// fails with a foreign-key violation if that user does not exist, which is the honest answer:
/// a profile without a managed-auth identity is not a thing this schema can hold.
pub async fn profiles_create(t: &mut PgTx<'_>, input: &ProfileCreateInput) -> Result<Profile, StoreError> {
    let user_id = input.user_id.as_str();
    let (rating, at) = (input.rating, input.at);
    // TS `displayName ?? null`: absent and null both write NULL.
    let display_name = input.display_name.as_ref().and_then(Option::as_deref);
    run_as(t, Some(user_id)).await?;
    sqlx::query(concat!(
        "insert into public.profiles (id, status, rating, created_at, display_name)
           values ($1::uuid, 'pending', $2::double precision, ",
        ts!("$3"),
        ", $4::text)"
    ))
    .bind(user_id)
    .bind(rating)
    .bind(at)
    .bind(display_name)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    let row = sqlx::query(concat!(
        "select ",
        profile_columns!(),
        " ",
        profile_from!(),
        " where p.id = $1::uuid"
    ))
    .bind(user_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let Some(row) = row else {
        return Err(StoreError::from(format!(
            "profiles.create wrote no row for {user_id}"
        )));
    };
    to_profile(ProfileRow::read(&row)?)
}

/// R111 rides on this write. Migration 0002 attaches `profiles_grant_launch_collection` to the
/// `pending -> active` transition, so becoming active grants one copy of every non-token card
/// in the current catalog version through `app.grant_cards` — both ledger tables, one
/// transaction, idempotent. Nothing here grants anything: the trigger is the implementation,
/// exactly as the fake says it is ("R111 IS A DATABASE TRIGGER").
pub async fn profiles_set_status(
    t: &mut PgTx<'_>,
    profile_id: &str,
    status: ProfileStatus,
) -> Result<(), StoreError> {
    run_as(t, Some(profile_id)).await?;
    let done = sqlx::query("update public.profiles set status = $2::text where id = $1::uuid")
        .bind(profile_id)
        .bind(literal(&status)?)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    if done.rows_affected() == 0 {
        return Err(StoreError::from(format!("no profile {profile_id}")));
    }
    Ok(())
}

pub async fn profiles_set_display_name(
    t: &mut PgTx<'_>,
    profile_id: &str,
    display_name: Option<&str>,
) -> Result<(), StoreError> {
    run_as(t, Some(profile_id)).await?;
    let done = sqlx::query("update public.profiles set display_name = $2::text where id = $1::uuid")
        .bind(profile_id)
        .bind(display_name)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    if done.rows_affected() == 0 {
        return Err(StoreError::from(format!("no profile {profile_id}")));
    }
    Ok(())
}

// TS's `profiles.setRating` — migration 0004's write, which rated a match by moving `rating` alone
// — is not ported (SURFACE §11.2: no caller). R603's rated path writes the whole triple through
// `profiles_set_glicko`.

/// R603: a rated game's whole Glicko triple, one statement.
pub async fn profiles_set_glicko(
    t: &mut PgTx<'_>,
    profile_id: &str,
    glicko: &Glicko,
) -> Result<(), StoreError> {
    run_as(t, Some(profile_id)).await?;
    let done = sqlx::query(
        "update public.profiles
           set rating = $2::double precision,
               rating_deviation = $3::double precision,
               rating_volatility = $4::double precision
         where id = $1::uuid",
    )
    .bind(profile_id)
    .bind(glicko.rating)
    .bind(glicko.deviation)
    .bind(glicko.volatility)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    if done.rows_affected() == 0 {
        return Err(StoreError::from(format!("no profile {profile_id}")));
    }
    Ok(())
}

/// §9.5: set when a match starts and cleared by every ending.
pub async fn profiles_set_in_match(
    t: &mut PgTx<'_>,
    profile_id: &str,
    match_id: Option<&str>,
) -> Result<(), StoreError> {
    run_as(t, Some(profile_id)).await?;
    let done = sqlx::query("update public.profiles set current_match_id = $2::uuid where id = $1::uuid")
        .bind(profile_id)
        .bind(match_id)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    if done.rows_affected() == 0 {
        return Err(StoreError::from(format!("no profile {profile_id}")));
    }
    Ok(())
}

/// `DELETE /api/account`'s store half: one delete, and migration 0012's foreign keys do the
/// rest. Rows only this profile owns cascade; its invite-code attempts, the codes it minted and
/// the finished matches, results, series and logged actions it played in keep their rows with
/// its seat set to null; `profiles_release_open_matches` removes a room it opened that nobody
/// joined. A profile still seated in a live match or an unfinished series makes the delete
/// raise, by constraint; `src/api/auth.rs` refuses that case first. No `app.*` function: the
/// schema is the rule here, and the statement is the whole of it.
pub async fn profiles_remove(t: &mut PgTx<'_>, profile_id: &str) -> Result<bool, StoreError> {
    if !is_uuid(profile_id) {
        return Ok(false);
    }
    run_as(t, Some(profile_id)).await?;
    let done = sqlx::query("delete from public.profiles where id = $1::uuid")
        .bind(profile_id)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(done.rows_affected() == 1)
}

// ---------------------------------------------------------------------------
// Invite codes (SPEC §9.4)
// ---------------------------------------------------------------------------

pub async fn codes_insert(t: &mut PgTx<'_>, code: &InviteCode) -> Result<(), StoreError> {
    run_as(t, None).await?;
    sqlx::query(concat!(
        "insert into public.invite_codes (id, code_hash, max_uses, uses, expires_at, revoked_at, created_at)
         values ($1::uuid, $2::text, $3::int, $4::int,
                 case when $5::double precision is null then null else ",
        ts!("$5"),
        " end,
                 case when $6::boolean then now() else null end,
                 ",
        ts!("$7"),
        ")"
    ))
    .bind(code.id.as_str())
    .bind(code.code_hash.as_str())
    .bind(code.max_uses)
    .bind(code.uses)
    .bind(code.expires_at)
    .bind(code.revoked)
    .bind(code.created_at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub async fn codes_find_by_hash(t: &mut PgTx<'_>, code_hash: &str) -> Result<Option<InviteCode>, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query(
        "select id, code_hash, max_uses, uses, revoked_at, expires_at, created_at
           from public.invite_codes where code_hash = $1::text",
    )
    .bind(code_hash)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(InviteCode {
        id: uuid_text(&row, "id")?,
        code_hash: get(&row, "code_hash")?,
        // `int` columns (migration 0001), widened to the store's i64 (store.rs's header).
        max_uses: i64::from(get::<i32>(&row, "max_uses")?),
        uses: i64::from(get::<i32>(&row, "uses")?),
        revoked: get::<Option<OffsetDateTime>>(&row, "revoked_at")?.is_some(),
        expires_at: ms_or_null(get(&row, "expires_at")?),
        created_at: ms_of(get(&row, "created_at")?),
    }))
}

/// §9.4 step 6, "one atomic statement": the guard is in the `where`, so the increment and the
/// checks cannot be separated by a concurrent caller. `invite_codes.uses` also carries
/// `check (uses >= 0 and uses <= max_uses)`, so even a bug here cannot over-consume a code.
pub async fn codes_claim(t: &mut PgTx<'_>, code_id: &str, now: i64) -> Result<bool, StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query(concat!(
        "update public.invite_codes
            set uses = uses + 1
          where id = $1::uuid
            and revoked_at is null
            and (expires_at is null or expires_at > ",
        ts!("$2"),
        ")
            and uses < max_uses"
    ))
    .bind(code_id)
    .bind(now)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(done.rows_affected() == 1)
}

/// §9.4 step 4: "log the attempt either way". `CodeAttempt.reason` has no column in
/// `public.code_attempts` — see KNOWN DIVERGENCES (attempt reason).
pub async fn codes_log_attempt(t: &mut PgTx<'_>, attempt: &CodeAttempt) -> Result<(), StoreError> {
    run_as(t, attempt.profile_id.as_deref()).await?;
    sqlx::query(concat!(
        "insert into public.code_attempts (profile_id, ip_hash, succeeded, at)
         values ($1::uuid, $2::text, $3::boolean, ",
        ts!("$4"),
        ")"
    ))
    .bind(attempt.profile_id.as_deref())
    .bind(attempt.ip_hash.as_str())
    .bind(literal(&attempt.result)? == "ok")
    .bind(attempt.at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub async fn codes_count_attempts_by_profile(
    t: &mut PgTx<'_>,
    profile_id: &str,
    since: i64,
) -> Result<i64, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let row = sqlx::query(concat!(
        "select count(*)::int as n from public.code_attempts
          where profile_id = $1::uuid and at >= ",
        ts!("$2")
    ))
    .bind(profile_id)
    .bind(since)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map_or(Ok(0), |row| get::<i32>(&row, "n").map(i64::from))
}

pub async fn codes_oldest_attempt_at_by_profile(
    t: &mut PgTx<'_>,
    profile_id: &str,
    since: i64,
) -> Result<Option<i64>, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let row = sqlx::query(concat!(
        "select min(at) as at from public.code_attempts
          where profile_id = $1::uuid and at >= ",
        ts!("$2")
    ))
    .bind(profile_id)
    .bind(since)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    match row {
        None => Ok(None),
        Some(row) => Ok(ms_or_null(get(&row, "at")?)),
    }
}

pub async fn codes_count_attempts_by_ip(
    t: &mut PgTx<'_>,
    ip_hash: &str,
    since: i64,
) -> Result<i64, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select count(*)::int as n from public.code_attempts
          where ip_hash = $1::text and at >= ",
        ts!("$2")
    ))
    .bind(ip_hash)
    .bind(since)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map_or(Ok(0), |row| get::<i32>(&row, "n").map(i64::from))
}

pub async fn codes_count_failures(t: &mut PgTx<'_>, since: i64) -> Result<i64, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select count(*)::int as n from public.code_attempts
          where succeeded = false and at >= ",
        ts!("$1")
    ))
    .bind(since)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map_or(Ok(0), |row| get::<i32>(&row, "n").map(i64::from))
}

// ---------------------------------------------------------------------------
// Collection (SPEC §9.4's entitlement ledger)
// ---------------------------------------------------------------------------

pub async fn collection_get(t: &mut PgTx<'_>, profile_id: &str) -> Result<Vec<CollectionEntry>, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let rows = sqlx::query(
        "select card_id, quantity from public.collection where profile_id = $1::uuid order by card_id",
    )
    .bind(profile_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| -> Result<CollectionEntry, StoreError> {
            // `quantity` is an `int` (migration 0002), widened to the store's i64.
            Ok(CollectionEntry {
                card_id: get(row, "card_id")?,
                quantity: i64::from(get::<i32>(row, "quantity")?),
            })
        })
        .collect()
}

/// ports.ts is explicit, and it is the opposite of `app.grant_cards`: "SETS each card's quantity
/// to the absolute value given; it does not add to it ... a Postgres adapter must
/// `set quantity = excluded.quantity`, never `quantity + excluded`." `grant_cards` in
/// `src/api/collection.rs` has already read the current total inside this transaction and added
/// its delta, so adding again here would double every grant.
///
/// That is why this pair does not call `app.grant_cards`, which takes deltas and writes both
/// tables itself: the port splits the ledger's two writes and puts them in one transaction
/// instead, which is the same §9.4 guarantee reached the other way round.
pub async fn collection_upsert_quantities(
    t: &mut PgTx<'_>,
    profile_id: &str,
    entries: &[CollectionEntry],
) -> Result<(), StoreError> {
    if entries.is_empty() {
        return Ok(());
    }
    let rows: Vec<Value> = entries
        .iter()
        .map(|entry| json!({ "card_id": entry.card_id, "quantity": entry.quantity }))
        .collect();
    run_as(t, Some(profile_id)).await?;
    sqlx::query(
        "insert into public.collection (profile_id, card_id, quantity, updated_at)
         select $1::uuid, entry.card_id, entry.quantity, now()
           from jsonb_to_recordset($2::jsonb) as entry(card_id text, quantity int)
         on conflict (profile_id, card_id)
         do update set quantity = excluded.quantity, updated_at = now()",
    )
    .bind(profile_id)
    .bind(json(&rows)?)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

/// Append-only (§9.4); `collection_grants` carries `check (delta <> 0)` and a deny trigger.
pub async fn collection_append_grants(
    t: &mut PgTx<'_>,
    grants: &[CollectionGrant],
) -> Result<(), StoreError> {
    let Some(first) = grants.first() else {
        return Ok(());
    };
    let rows: Vec<Value> = grants
        .iter()
        .map(|grant| {
            json!({
                "profile_id": grant.profile_id,
                "card_id": grant.card_id,
                "delta": grant.delta,
                "reason": grant.reason,
                "at": grant.at,
            })
        })
        .collect();
    run_as(t, Some(first.profile_id.as_str())).await?;
    sqlx::query(concat!(
        "insert into public.collection_grants (profile_id, card_id, delta, reason, at)
         select g.profile_id::uuid, g.card_id, g.delta, g.reason, ",
        ts!("g.at"),
        "
           from jsonb_to_recordset($1::jsonb)
             as g(profile_id text, card_id text, delta int, reason text, at double precision)"
    ))
    .bind(json(&rows)?)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Saved decks (SPEC §9.4, R250, R256)
//
// Migration 0007. The loadout tables of 0003 are no longer read or written (R254): the migration
// turned each loadout into three of these decks and one trio.
// ---------------------------------------------------------------------------

/// Oldest first, ties on id: the order R257's legacy `deckIndex` counts in.
pub async fn decks_list(t: &mut PgTx<'_>, profile_id: &str) -> Result<Vec<SavedDeck>, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let rows = sqlx::query(concat!(
        "select ",
        deck_columns!(),
        " from public.decks where profile_id = $1::uuid order by created_at, id"
    ))
    .bind(profile_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| DeckRow::read(row).and_then(to_deck))
        .collect()
}

pub async fn decks_get(t: &mut PgTx<'_>, deck_id: &str) -> Result<Option<SavedDeck>, StoreError> {
    if !is_uuid(deck_id) {
        return Ok(None);
    }
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select ",
        deck_columns!(),
        " from public.decks where id = $1::uuid"
    ))
    .bind(deck_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| DeckRow::read(&row).and_then(to_deck)).transpose()
}

/// R250, R256: one call to `app.upsert_deck`, which takes the profile row lock, re-checks the
/// draft's shape (D1, D2, D4), refuses another profile's id and counts the cap under the lock,
/// then inserts or updates. Its answer is the port's `UpsertOutcome` verbatim.
///
/// The function takes one instant, `p_at`: `updated_at` always, and `created_at` when the row is
/// new. It is handed `updated_at`, which is the save being made; see KNOWN DIVERGENCES (deck and
/// trio timestamps). The cap passed is the caller's, and the function applies the smaller of it
/// and `app.settings.max_saved_decks` (KNOWN DIVERGENCES, caps).
pub async fn decks_upsert(
    t: &mut PgTx<'_>,
    deck: &SavedDeck,
    max_decks: i64,
) -> Result<UpsertOutcome, StoreError> {
    let max_decks = i32::try_from(max_decks).unwrap_or(i32::MAX);
    run_as(t, Some(deck.profile_id.as_str())).await?;
    let row = sqlx::query(concat!(
        "select app.upsert_deck($1::uuid, $2::uuid, $3::text, $4::jsonb, $5::text, ",
        ts!("$6"),
        ", $7::int,
           $8::text)
           as outcome"
    ))
    .bind(deck.profile_id.as_str())
    .bind(deck.id.as_str())
    .bind(deck.name.as_str())
    .bind(json(&deck.cards)?)
    .bind(deck.catalog_version.as_str())
    .bind(deck.updated_at)
    .bind(max_decks)
    .bind(deck.portrait.as_deref())
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let outcome = match row {
        None => None,
        Some(row) => get::<Option<String>>(&row, "outcome")?,
    };
    upsert_outcome_of("app.upsert_deck", UPSERT_OUTCOMES, outcome)
}

/// Not an `app.*` function: a delete of the profile's own row is one statement with nothing to
/// decide, and R252's "deleting a deck empties every slot that named it" is the database's own
/// behaviour — the three `on delete set null (deckN_id)` foreign keys on `public.trios` empty the
/// slots in this same statement.
pub async fn decks_remove(t: &mut PgTx<'_>, profile_id: &str, deck_id: &str) -> Result<bool, StoreError> {
    if !is_uuid(deck_id) {
        return Ok(false);
    }
    run_as(t, Some(profile_id)).await?;
    let done = sqlx::query("delete from public.decks where id = $1::uuid and profile_id = $2::uuid")
        .bind(deck_id)
        .bind(profile_id)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(done.rows_affected() == 1)
}

// ---------------------------------------------------------------------------
// Saved trios (SPEC §9.4, R252, R256)
// ---------------------------------------------------------------------------

pub async fn trios_list(t: &mut PgTx<'_>, profile_id: &str) -> Result<Vec<SavedTrio>, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let rows = sqlx::query(concat!(
        "select ",
        trio_columns!(),
        " from public.trios where profile_id = $1::uuid order by created_at, id"
    ))
    .bind(profile_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(|row| TrioRow::read(row).map(to_trio)).collect()
}

pub async fn trios_get(t: &mut PgTx<'_>, trio_id: &str) -> Result<Option<SavedTrio>, StoreError> {
    if !is_uuid(trio_id) {
        return Ok(None);
    }
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select ",
        trio_columns!(),
        " from public.trios where id = $1::uuid"
    ))
    .bind(trio_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| TrioRow::read(&row).map(to_trio)).transpose()
}

/// R252, R256: one call to `app.upsert_trio`, as `decks_upsert`, plus `unknown_deck` for a slot
/// that is not one of this profile's decks. A deck in two slots raises `trios_decks_distinct`
/// (R252 T3), which the caller has already refused with `check_trio_draft`, so it errors here as it
/// does in the in-memory store.
///
/// A slot holding something that is not a uuid cannot name any deck; it is answered here, as
/// `unknown_deck`, because Postgres would refuse the cast before the function could say so.
pub async fn trios_upsert(
    t: &mut PgTx<'_>,
    trio: &SavedTrio,
    max_trios: i64,
) -> Result<TrioUpsertOutcome, StoreError> {
    let max_trios = i32::try_from(max_trios).unwrap_or(i32::MAX);
    let (deck1, deck2, deck3) = &trio.deck_ids;
    if [deck1, deck2, deck3]
        .into_iter()
        .flatten()
        .any(|deck_id| !is_uuid(deck_id))
    {
        return from_literal("unknown_deck");
    }
    run_as(t, Some(trio.profile_id.as_str())).await?;
    let row = sqlx::query(concat!(
        "select app.upsert_trio($1::uuid, $2::uuid, $3::text, $4::uuid, $5::uuid, $6::uuid, ",
        ts!("$7"),
        ",
                                $8::int) as outcome"
    ))
    .bind(trio.profile_id.as_str())
    .bind(trio.id.as_str())
    .bind(trio.name.as_str())
    .bind(deck1.as_deref())
    .bind(deck2.as_deref())
    .bind(deck3.as_deref())
    .bind(trio.updated_at)
    .bind(max_trios)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let outcome = match row {
        None => None,
        Some(row) => get::<Option<String>>(&row, "outcome")?,
    };
    upsert_outcome_of("app.upsert_trio", TRIO_UPSERT_OUTCOMES, outcome)
}

/// As `decks_remove`: the profile's own row, one statement.
pub async fn trios_remove(t: &mut PgTx<'_>, profile_id: &str, trio_id: &str) -> Result<bool, StoreError> {
    if !is_uuid(trio_id) {
        return Ok(false);
    }
    run_as(t, Some(profile_id)).await?;
    let done = sqlx::query("delete from public.trios where id = $1::uuid and profile_id = $2::uuid")
        .bind(trio_id)
        .bind(profile_id)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(done.rows_affected() == 1)
}

// ---------------------------------------------------------------------------
// Matches (SPEC §9.3's log, §9.5's lifecycle)
// ---------------------------------------------------------------------------

/// The 21 values `matches_create`'s insert and update share after `$1` (TS's `values`).
macro_rules! match_values {
    () => {
        concat!(
            "
          $2::text, $3::text, $4::uuid, $5::uuid, $6::jsonb, $7::jsonb, $8::text,
          ",
            nullable_ts!("$9"),
            ", ",
            nullable_ts!("$10"),
            ", ",
            nullable_ts!("$11"),
            ", ",
            nullable_ts!("$12"),
            ",
          ",
            nullable_ts!("$13"),
            ", ",
            ts!("$14"),
            ", ",
            ts!("$15"),
            ", ",
            nullable_ts!("$16"),
            ", $17::jsonb, $18::jsonb,
          $19::text, $20::text, $22::text, $23::smallint"
        )
    };
}

/// Writes the match row for both entry points §9.5 has — a paired queue match and a claimed
/// room — which is why it is an insert OR a completion rather than only an insert.
///
/// `public.matches` is one table for both: a room is a row with `status = 'open'` and no second
/// player, and `rooms_claim` / `tickets_claim_pair` below have already written that row under the
/// id the caller minted (they must: `tickets.match_id` and `profiles.current_match_id` are
/// foreign keys into this table, and both are written before the actor starts). So this method
/// finds one of two states under `id`:
///
///  - nothing      -> insert the live match;
///  - an `open` row -> fill in the seed, the decks, the clocks and flip it `live`;
///  - anything else -> the id is taken, which ports.ts and the fake both make an error.
///
/// The `for update` is what makes the read-then-write safe against a second caller.
pub async fn matches_create(t: &mut PgTx<'_>, m: &MatchRow) -> Result<(), StoreError> {
    run_as(t, Some(m.players.0.as_str())).await?;
    let existing = sqlx::query("select status from public.matches where id = $1::uuid for update")
        .bind(m.id.as_str())
        .fetch_optional(&mut **t)
        .await
        .map_err(db_error)?;
    let status: Option<String> = existing.map(|row| get(&row, "status")).transpose()?;
    if let Some(status) = status.as_deref()
        && status != "open"
    {
        return Err(StoreError::from(format!("matches.id is unique: {}", m.id)));
    }

    let statement = if status.is_none() {
        concat!(
            "insert into public.matches (
               id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck, catalog_version,
               turn_deadline_at, prompt_deadline_at, p1_disconnected_at, p2_disconnected_at,
               grace_deadline_at, ceiling_at, created_at, ended_at, p1_last_board, p2_last_board,
               p1_portrait, p2_portrait, mode, stake, ranked, started_at, last_seq, p1_glitch_board, p2_glitch_board)
             values ($1::uuid, ",
            match_values!(),
            ", $21::boolean, ",
            ts!("$15"),
            ", 0, $24::jsonb, $25::jsonb)"
        )
    } else {
        concat!(
            "update public.matches set
             status = $2::text, seed = $3::text, p1_profile_id = $4::uuid, p2_profile_id = $5::uuid,
             p1_deck = $6::jsonb, p2_deck = $7::jsonb, catalog_version = $8::text,
             turn_deadline_at = ",
            nullable_ts!("$9"),
            ", prompt_deadline_at = ",
            nullable_ts!("$10"),
            ",
             p1_disconnected_at = ",
            nullable_ts!("$11"),
            ", p2_disconnected_at = ",
            nullable_ts!("$12"),
            ",
             grace_deadline_at = ",
            nullable_ts!("$13"),
            ", ceiling_at = ",
            ts!("$14"),
            ",
             created_at = ",
            ts!("$15"),
            ", ended_at = ",
            nullable_ts!("$16"),
            ", started_at = ",
            ts!("$15"),
            ",
             p1_last_board = $17::jsonb, p2_last_board = $18::jsonb,
             p1_portrait = $19::text, p2_portrait = $20::text, ranked = $21::boolean,
             mode = $22::text, stake = $23::smallint,
             p1_glitch_board = $24::jsonb, p2_glitch_board = $25::jsonb
           where id = $1::uuid"
        )
    };

    let no_board: Vec<LastBoardEntry> = Vec::new();
    let status_text = if literal(&m.status)? == "finished" {
        "over"
    } else {
        "live"
    };
    sqlx::query(statement)
        .bind(m.id.as_str())
        .bind(status_text)
        .bind(m.seed.as_str())
        .bind(m.players.0.as_str())
        .bind(m.players.1.as_str())
        .bind(json(&m.decks.0)?)
        .bind(json(&m.decks.1)?)
        .bind(m.catalog_version.as_str())
        .bind(m.clocks.turn_deadline)
        .bind(m.clocks.prompt_deadline)
        .bind(m.clocks.grace_deadline.p1)
        .bind(m.clocks.grace_deadline.p2)
        .bind(nearest_grace(&m.clocks))
        .bind(m.clocks.ceiling_at)
        .bind(m.created_at)
        .bind(m.finished_at)
        .bind(json(
            m.last_boards.as_ref().map_or(&no_board, |boards| &boards.0),
        )?)
        .bind(json(
            m.last_boards.as_ref().map_or(&no_board, |boards| &boards.1),
        )?)
        .bind(
            m.portraits
                .as_ref()
                .map(|portraits| literal(&portraits.0))
                .transpose()?,
        )
        .bind(
            m.portraits
                .as_ref()
                .map(|portraits| literal(&portraits.1))
                .transpose()?,
        )
        // R604: false for a room and for the `open` skeletons this UPDATE turns live — a room
        // never calls this, and a queue skeleton's own write is what stamps the flag.
        .bind(m.ranked.unwrap_or(false))
        // R672: null on every row but a rematch's, which states both (migration 0023).
        .bind(m.mode.as_ref().map(literal).transpose()?)
        .bind(int_or_null(&m.stake)?)
        // R678: the Glitch boards sampled at creation (migration 0024), '[]' when absent.
        .bind(json(
            m.glitch_boards.as_ref().map_or(&no_board, |boards| &boards.0),
        )?)
        .bind(json(
            m.glitch_boards.as_ref().map_or(&no_board, |boards| &boards.1),
        )?)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(())
}

pub async fn matches_get(t: &mut PgTx<'_>, match_id: &str) -> Result<Option<MatchRow>, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select ",
        match_columns!(),
        " from public.matches
          where id = $1::uuid and status in ('live', 'over')"
    ))
    .bind(match_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| MatchDbRow::read(&row).and_then(to_match))
        .transpose()
}

/// §9.3's append-only log, written by `app.append_match_action` (migration 0004) and by nothing
/// else: it assigns `seq` from `matches.last_seq` under a row lock and returns the original
/// `seq` for a repeated nonce, which is BUILD M6-T4's "a reused nonce returns the original ack".
///
/// The port hands in the `seq` the actor already used, so the two must agree: a mismatch means
/// the log and the actor have diverged (a replayed nonce, a gap, a second writer), and it is
/// raised rather than swallowed — ports.ts: "Rejects a seq that already exists."
pub async fn matches_append_actions(t: &mut PgTx<'_>, rows: &[MatchActionRow]) -> Result<(), StoreError> {
    run_as(t, None).await?;
    for row in rows {
        let action: &Action = &row.action;
        let seat = action.player_id.as_str();
        let out = sqlx::query(
            "select app.append_match_action(
               $1::uuid,
               $2::text,
               (select case $2::text when 'p1' then m.p1_profile_id when 'p2' then m.p2_profile_id end
                  from public.matches m where m.id = $1::uuid),
               $3::text,
               $4::jsonb)::text as seq",
        )
        .bind(row.match_id.as_str())
        .bind(seat)
        .bind(action.nonce.as_str())
        .bind(json(&row.action)?)
        .fetch_optional(&mut **t)
        .await
        .map_err(db_error)?;
        let seq: Option<String> = match out {
            None => None,
            Some(out) => get(&out, "seq")?,
        };
        let assigned = int_of(seq.as_deref().unwrap_or("0"))?;
        // The error happens INSIDE the transaction the function's own insert ran in, so the
        // rollback takes that insert with it. A store that wrote the row and then complained
        // would be worse than one that refused.
        if assigned != row.seq {
            return Err(StoreError::from(format!(
                "match_actions is append-only: seq {} exists (app.append_match_action assigned {} for nonce {})",
                row.seq, assigned, action.nonce
            )));
        }
    }
    Ok(())
}

pub async fn matches_actions(t: &mut PgTx<'_>, match_id: &str) -> Result<Vec<MatchActionRow>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select seq::text, action, at from public.match_actions
          where match_id = $1::uuid order by seq",
    )
    .bind(match_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| -> Result<MatchActionRow, StoreError> {
            Ok(MatchActionRow {
                match_id: match_id.to_string(),
                seq: int_of(&get::<String>(row, "seq")?)?,
                action: from_json(get::<Value>(row, "action")?)?,
                at: ms_of(get(row, "at")?),
            })
        })
        .collect()
}

pub async fn matches_set_clocks(
    t: &mut PgTx<'_>,
    match_id: &str,
    clocks: &MatchClocks,
) -> Result<(), StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query(concat!(
        "update public.matches set
           turn_deadline_at = ",
        nullable_ts!("$2"),
        ", prompt_deadline_at = ",
        nullable_ts!("$3"),
        ",
           p1_disconnected_at = ",
        nullable_ts!("$4"),
        ", p2_disconnected_at = ",
        nullable_ts!("$5"),
        ",
           grace_deadline_at = ",
        nullable_ts!("$6"),
        ", ceiling_at = ",
        ts!("$7"),
        "
         where id = $1::uuid"
    ))
    .bind(match_id)
    .bind(clocks.turn_deadline)
    .bind(clocks.prompt_deadline)
    .bind(clocks.grace_deadline.p1)
    .bind(clocks.grace_deadline.p2)
    .bind(nearest_grace(clocks))
    .bind(clocks.ceiling_at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    if done.rows_affected() == 0 {
        return Err(StoreError::from(format!("no match {match_id}")));
    }
    Ok(())
}

/// Not `app.end_match`. That function ends a match AND writes the results row AND moves both
/// ratings AND clears both `current_match_id`s AND cancels stray tickets — every one of which
/// `src/api/results.rs` already does through `results_insert`, `profiles_set_glicko`,
/// `profiles_set_in_match` and `tickets_cancel`, inside the one transaction this call runs in. So
/// the ending is still one transaction with the same five writes; calling `app.end_match` here
/// would do the other four a second time.
pub async fn matches_finish(t: &mut PgTx<'_>, match_id: &str, at: i64) -> Result<(), StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query(concat!(
        "update public.matches set status = 'over', ended_at = ",
        ts!("$2"),
        "
            where id = $1::uuid and status <> 'over'"
    ))
    .bind(match_id)
    .bind(at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    if done.rows_affected() == 0 {
        // Already over is a no-op (§9.5: the actor and the reaper can both reach an ending);
        // a match that was never there is an error, as it is in the fake.
        let rows = sqlx::query("select id from public.matches where id = $1::uuid")
            .bind(match_id)
            .fetch_all(&mut **t)
            .await
            .map_err(db_error)?;
        if rows.is_empty() {
            return Err(StoreError::from(format!("no match {match_id}")));
        }
    }
    Ok(())
}

/// §9.5's crash recovery and the reaper's input, straight from `app.live_matches()`.
pub async fn matches_live(t: &mut PgTx<'_>) -> Result<Vec<MatchRow>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(concat!("select ", match_columns!(), " from app.live_matches()"))
        .fetch_all(&mut **t)
        .await
        .map_err(db_error)?;
    let mut live = Vec::with_capacity(rows.len());
    for row in &rows {
        let row = MatchDbRow::read(row)?;
        if row.p2_profile_id.is_some() {
            live.push(to_match(row)?);
        }
    }
    Ok(live)
}

/// R376: a Conquest game is its series' (`series_with_game`). Otherwise a room's match is the row
/// the room was (`room_code` set) and has its `room_mode`, `bo1` when it has none as `rooms_get`
/// reads it; a queue match is the skeleton `tickets_claim_pair` wrote, and has its tickets' mode.
pub async fn matches_mode_of(t: &mut PgTx<'_>, match_id: &str) -> Result<Option<QueueMode>, StoreError> {
    if !is_uuid(match_id) {
        return Ok(None);
    }
    if series_with_game(t, match_id).await?.is_some() {
        return from_literal("bo3").map(Some);
    }
    run_as(t, None).await?;
    let row = sqlx::query(
        "select m.mode, m.room_code, m.room_mode,
                (select t.mode from public.tickets t where t.match_id = m.id
                  order by t.enqueued_at, t.id limit 1) as ticket_mode
           from public.matches m
          where m.id = $1::uuid",
    )
    .bind(match_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let mode: Option<String> = get(&row, "mode")?;
    let room_code: Option<String> = get(&row, "room_code")?;
    let room_mode: Option<String> = get(&row, "room_mode")?;
    let ticket_mode: Option<String> = get(&row, "ticket_mode")?;
    // R672: a rematch states its own mode, since no ticket, room or series made it.
    if let Some(mode) = mode.as_deref() {
        return queue_mode_of(mode).map(Some);
    }
    if room_code.is_some() {
        return match room_mode.as_deref() {
            None => from_literal("bo1").map(Some),
            Some(mode) => queue_mode_of(mode).map(Some),
        };
    }
    ticket_mode.as_deref().map(queue_mode_of).transpose()
}

/// R263: "A series that ends before its first game releases the id it reserved." In this schema
/// a reserved id is a row — the `open` skeleton `tickets_claim_pair` writes, or a room
/// `rooms_claim` renamed to it — so releasing it is deleting that row, and only while it is still
/// `open`: the `status` guard is what makes this a no-op on a live or finished match however it
/// is called.
///
/// Nothing is left pointing at the deleted id: an `open` row has no actions and no result (both
/// would cascade anyway), `tickets.match_id` and `profiles.current_match_id` are `on delete set
/// null` (0004), and `series.next_match_id` deliberately has no foreign key (0009). Deleting a
/// claimed room's row also frees its code at once (R110's partial unique index covers only the
/// rows that exist). See KNOWN DIVERGENCES (reserved match ids).
pub async fn matches_discard_open(t: &mut PgTx<'_>, match_id: &str) -> Result<(), StoreError> {
    if !is_uuid(match_id) {
        return Ok(());
    }
    run_as(t, None).await?;
    sqlx::query("delete from public.matches where id = $1::uuid and status = 'open'")
        .bind(match_id)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(())
}

/// R679: migration 0024's `app.forget_voided_match`, the one path that erases a live match and
/// its append-only log. The foreign keys let both players go (`profiles.current_match_id` and
/// `tickets.match_id` are `on delete set null`); a finished match, or one with a result, stays.
pub async fn matches_forget_voided(t: &mut PgTx<'_>, match_id: &str) -> Result<(), StoreError> {
    if !is_uuid(match_id) {
        return Ok(());
    }
    run_as(t, None).await?;
    sqlx::query("select app.forget_voided_match($1::uuid)")
        .bind(match_id)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(())
}

/// R768 (migration 0027): the `final_hash is null` guard is what makes it written once.
pub async fn matches_record_final_hash(t: &mut PgTx<'_>, match_id: &str, final_hash: &str) -> Result<(), StoreError> {
    if !is_uuid(match_id) {
        return Ok(());
    }
    run_as(t, None).await?;
    sqlx::query("update public.matches set final_hash = $2::text where id = $1::uuid and final_hash is null")
        .bind(match_id)
        .bind(final_hash)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(())
}

/// R768: read off the row whatever its status. A claimed room's or a queue pair's reservation is an
/// `open` row under the match id (KNOWN DIVERGENCES, reserved match ids), which `matches_get` skips.
pub async fn matches_seats(t: &mut PgTx<'_>, match_id: &str) -> Result<Option<MatchSeats>, StoreError> {
    if !is_uuid(match_id) {
        return Ok(None);
    }
    run_as(t, None).await?;
    let row = sqlx::query("select status, p1_profile_id, p2_profile_id from public.matches where id = $1::uuid")
        .bind(match_id)
        .fetch_optional(&mut **t)
        .await
        .map_err(db_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let status: String = get(&row, "status")?;
    let players = [
        uuid_text_or_null(&row, "p1_profile_id")?,
        uuid_text_or_null(&row, "p2_profile_id")?,
    ]
    .into_iter()
    .flatten()
    .collect();
    Ok(Some(MatchSeats {
        phase: from_literal(&status)?,
        players,
    }))
}

// ---------------------------------------------------------------------------
// Replays (R768)
// ---------------------------------------------------------------------------

/// One finished match with a result, read for its replay. Unlike `to_match`'s other callers it
/// reads a deleted second seat as the empty id too (KNOWN DIVERGENCES, deleted accounts), so a
/// player keeps the replay of a match whose opponent has deleted their account.
async fn replay_row(t: &mut PgTx<'_>, match_id: &str, actions: i64) -> Result<Option<ReplayRow>, StoreError> {
    let row = sqlx::query(concat!(
        "select ",
        match_columns!(),
        " from public.matches
          where id = $1::uuid and status = 'over'"
    ))
    .bind(match_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let Some(row) = row else {
        return Ok(None);
    };
    let mut row = MatchDbRow::read(&row)?;
    row.p2_profile_id.get_or_insert_with(String::new);
    let row = to_match(row)?;
    let Some(result) = results_get_by_match(t, match_id).await? else {
        return Ok(None);
    };
    Ok(Some(ReplayRow { row, result, actions }))
}

pub async fn replays_list(
    t: &mut PgTx<'_>,
    profile_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<ReplayRow>, StoreError> {
    if !is_uuid(profile_id) {
        return Ok(Vec::new());
    }
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select m.id, (select count(*) from public.match_actions a where a.match_id = m.id) as actions
           from public.matches m
          where m.status = 'over'
            and (m.p1_profile_id = $1::uuid or m.p2_profile_id = $1::uuid)
            and exists (select 1 from public.results r where r.match_id = m.id)
            and exists (select 1 from public.match_actions a where a.match_id = m.id)
          order by m.ended_at desc nulls last, m.id desc
          limit $2::bigint offset $3::bigint",
    )
    .bind(profile_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    let mut replays = Vec::with_capacity(rows.len());
    for row in &rows {
        let match_id = uuid_text(row, "id")?;
        if let Some(replay) = replay_row(t, &match_id, get(row, "actions")?).await? {
            replays.push(replay);
        }
    }
    Ok(replays)
}

pub async fn replays_get(t: &mut PgTx<'_>, match_id: &str) -> Result<Option<ReplayRow>, StoreError> {
    if !is_uuid(match_id) {
        return Ok(None);
    }
    run_as(t, None).await?;
    let counted = sqlx::query("select count(*) as actions from public.match_actions where match_id = $1::uuid")
        .bind(match_id)
        .fetch_one(&mut **t)
        .await
        .map_err(db_error)?;
    replay_row(t, match_id, get(&counted, "actions")?).await
}

// ---------------------------------------------------------------------------
// Rooms (SPEC §9.5's direct challenge)
// ---------------------------------------------------------------------------

/// A room is a `public.matches` row with `status = 'open'`, which is what migrations 0004's
/// `app.create_room` / `app.join_room` make it; there is no `rooms` table and this file may not
/// add one (the migrations are append-only and checksum-locked).
///
/// `app.create_room` is not called for two reasons: it mints the match id itself while the port
/// has `rooms_claim` receive an id the server minted, and it sets the host's
/// `current_match_id` while `src/api/rooms.rs` sets both players' only after the claim (§9.5
/// "not in a match" would otherwise refuse the host their own room). Everything else it does is
/// done here, including `matches_room_code_open_key` — the partial unique index is inferred in
/// the `on conflict` clause, so a taken code returns `false` instead of raising, which is the
/// port's contract.
///
/// R264: the room's mode goes in `room_mode` and a Conquest host's frozen trio in `room_trio`
/// (migration 0008); `p1_deck` holds the Best-of-1 deck, `[]` in the other two modes.
pub async fn rooms_create(t: &mut PgTx<'_>, room: &Room) -> Result<bool, StoreError> {
    run_as(t, Some(room.host_profile_id.as_str())).await?;
    let host_trio = json_or_null(&room.host_trio)?;
    let done = sqlx::query(concat!(
        "insert into public.matches (
           room_code, room_mode, room_trio, status, seed, p1_profile_id, p1_deck, p1_portrait,
           catalog_version, ceiling_at, created_at)
         values ($1::text, $2::text, $3::jsonb, 'open', '', $4::uuid, $5::jsonb, $6::text,
                 $7::text, ",
        ts!("$8"),
        ", ",
        ts!("$9"),
        ")
         on conflict (room_code) where room_code is not null and status <> 'over' do nothing"
    ))
    .bind(room.code.as_str())
    .bind(literal(&room.mode)?)
    .bind(host_trio)
    .bind(room.host_profile_id.as_str())
    .bind(json(&room.host_deck)?)
    .bind(room.host_portrait.as_ref().and_then(Option::as_deref))
    .bind(room.catalog_version.as_str())
    .bind(room.expires_at)
    .bind(room.created_at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(done.rows_affected() == 1)
}

pub async fn rooms_get(t: &mut PgTx<'_>, code: &str) -> Result<Option<Room>, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select ",
        room_columns!(),
        " from public.matches
          where room_code = $1::text and status <> 'over'"
    ))
    .bind(code)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| RoomRow::read(&row).and_then(to_room)).transpose()
}

/// §9.5's atomic single-claim, as one statement: the loser of a join race updates no row and
/// gets `None`. The row keeps `status = 'open'` until `matches_create` completes it, so a
/// half-claimed room is never visible to `matches_get`, `matches_live` or the reaper.
///
/// `id = $3` is the one place this file rewrites a primary key. It is deliberate: the port mints
/// the match id at join time, `matches_create` will be called with it moments later, and a room
/// that is not yet a match has nothing pointing at it — no actions, no result, no ticket, and
/// `src/api/rooms.rs` sets `current_match_id` only after this call returns. Renaming the row
/// is what keeps "a room is the match it becomes" true of the schema.
pub async fn rooms_claim(
    t: &mut PgTx<'_>,
    code: &str,
    guest_profile_id: &str,
    match_id: &str,
    at: i64,
) -> Result<Option<Room>, StoreError> {
    run_as(t, Some(guest_profile_id)).await?;
    let row = sqlx::query(concat!(
        "update public.matches
            set id = $3::uuid, p2_profile_id = $2::uuid
          where room_code = $1::text
            and status = 'open'
            and p2_profile_id is null
            and ceiling_at > ",
        ts!("$4"),
        "
        returning ",
        room_columns!()
    ))
    .bind(code)
    .bind(guest_profile_id)
    .bind(match_id)
    .bind(at)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| RoomRow::read(&row).and_then(to_room)).transpose()
}

// ---------------------------------------------------------------------------
// Tickets (SPEC §9.5's ranked queue)
// ---------------------------------------------------------------------------

/// `tickets_profile_queued_key` (migration 0004) is the race-proof half of §9.5's "not already
/// queued", and `src/api/queue.rs` relies on the insert RAISING for the second one — it catches
/// the error and re-reads the open ticket (in a fresh transaction: see `begin`). So this is a plain
/// insert with no `on conflict`.
///
/// R257, R259: the mode, and a Conquest ticket's frozen trio (migration 0008, whose
/// `tickets_frozen_trio_check` holds "a trio exactly when the mode is bo3"). `slot` — 0004's
/// loadout slot — is left NULL: a ticket now freezes a saved deck or a trio, not a slot, and 0008
/// dropped the column's `not null` for exactly that.
pub async fn tickets_insert(t: &mut PgTx<'_>, ticket: &Ticket) -> Result<(), StoreError> {
    run_as(t, Some(ticket.profile_id.as_str())).await?;
    let trio = json_or_null(&ticket.trio)?;
    sqlx::query(concat!(
        "insert into public.tickets
           (id, profile_id, rating, mode, frozen_deck, portrait, frozen_trio, catalog_version, status,
            enqueued_at, match_id)
         values ($1::uuid, $2::uuid, $3::double precision, $4::text, $5::jsonb, $6::text, $7::jsonb, $8::text,
                 $9::text, ",
        ts!("$10"),
        ", $11::uuid)"
    ))
    .bind(ticket.id.as_str())
    .bind(ticket.profile_id.as_str())
    .bind(ticket.rating)
    .bind(literal(&ticket.mode)?)
    .bind(json(&ticket.deck)?)
    .bind(ticket.portrait.as_ref().and_then(Option::as_deref))
    .bind(trio)
    .bind(ticket.catalog_version.as_str())
    .bind(from_ticket_status(&ticket.status)?)
    .bind(ticket.enqueued_at)
    .bind(ticket.match_id.as_deref())
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub async fn tickets_get(t: &mut PgTx<'_>, ticket_id: &str) -> Result<Option<Ticket>, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select ",
        ticket_columns!(),
        " from public.tickets where id = $1::uuid"
    ))
    .bind(ticket_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| TicketRow::read(&row).and_then(to_ticket))
        .transpose()
}

pub async fn tickets_open_for_profile(
    t: &mut PgTx<'_>,
    profile_id: &str,
) -> Result<Option<Ticket>, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let row = sqlx::query(concat!(
        "select ",
        ticket_columns!(),
        " from public.tickets
          where profile_id = $1::uuid and status = 'queued'"
    ))
    .bind(profile_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| TicketRow::read(&row).and_then(to_ticket))
        .transpose()
}

pub async fn tickets_list_open(t: &mut PgTx<'_>) -> Result<Vec<Ticket>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(concat!(
        "select ",
        ticket_columns!(),
        " from public.tickets where status = 'queued' order by enqueued_at, id"
    ))
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| TicketRow::read(row).and_then(to_ticket))
        .collect()
}

pub async fn tickets_count_open(t: &mut PgTx<'_>) -> Result<i64, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query("select count(*)::int as n from public.tickets where status = 'queued'")
        .fetch_optional(&mut **t)
        .await
        .map_err(db_error)?;
    row.map_or(Ok(0), |row| get::<i32>(&row, "n").map(i64::from))
}

/// R257: "the queue population is reported per mode". Every mode is present, at 0 if empty.
pub async fn tickets_count_open_by_mode(t: &mut PgTx<'_>) -> Result<PerMode<i64>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select mode, count(*)::int as n from public.tickets where status = 'queued' group by mode",
    )
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    let mut counts: PerMode<i64> = PerMode::default();
    for row in &rows {
        let mode = queue_mode_of(&get::<String>(row, "mode")?)?;
        counts[mode] = i64::from(get::<i32>(row, "n")?);
    }
    Ok(counts)
}

/// §9.5: "both tickets are claimed in one atomic statement." That statement is
/// `app.claim_ticket_pair` (migration 0004), which returns true only when it moved exactly two
/// still-queued rows, so two matchers racing over one ticket cannot both win.
///
/// It is called with a NULL match id and the link is written immediately after, inside the same
/// transaction, because `tickets.match_id` is a foreign key into `public.matches` and the match
/// does not exist yet — `src/api/queue.rs` claims first and creates the match second. The skeleton
/// row written here is the `open` row `matches_create` then completes (see its comment); it
/// carries the two frozen decks the tickets already hold, so nothing is invented.
pub async fn tickets_claim_pair(
    t: &mut PgTx<'_>,
    a_id: &str,
    b_id: &str,
    match_id: &str,
    at: i64,
) -> Result<bool, StoreError> {
    if a_id == b_id {
        return Ok(false);
    }
    run_as(t, None).await?;
    let claimed = sqlx::query("select app.claim_ticket_pair($1::uuid, $2::uuid, null) as claimed")
        .bind(a_id)
        .bind(b_id)
        .fetch_optional(&mut **t)
        .await
        .map_err(db_error)?;
    let claimed: Option<bool> = match claimed {
        None => None,
        Some(row) => get(&row, "claimed")?,
    };
    if claimed != Some(true) {
        return Ok(false);
    }

    sqlx::query(concat!(
        "insert into public.matches (
             id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck, catalog_version,
             ceiling_at, created_at)
           select $3::uuid, 'open', '', a.profile_id, b.profile_id, a.frozen_deck, b.frozen_deck,
                  a.catalog_version, ",
        ts!("$4"),
        ", ",
        ts!("$4"),
        "
             from public.tickets a, public.tickets b
            where a.id = $1::uuid and b.id = $2::uuid"
    ))
    .bind(a_id)
    .bind(b_id)
    .bind(match_id)
    .bind(at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    sqlx::query("update public.tickets set match_id = $3::uuid where id in ($1::uuid, $2::uuid)")
        .bind(a_id)
        .bind(b_id)
        .bind(match_id)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(true)
}

pub async fn tickets_cancel(t: &mut PgTx<'_>, ticket_id: &str, _at: i64) -> Result<(), StoreError> {
    run_as(t, None).await?;
    sqlx::query("update public.tickets set status = 'cancelled' where id = $1::uuid and status = 'queued'")
        .bind(ticket_id)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Results (SPEC §2.5, §9.5)
// ---------------------------------------------------------------------------

/// A profile's finished-match record, counted in one pass over `results`.
///
/// A draw is a row with no winner — §9.5 makes the ceiling, a mutual hero death and an
/// accepted draw all winnerless — so the three counts partition every finished match and no
/// separate "played" column can drift from them. The profile may sit on either side, hence
/// the `in (p1, p2)` rather than a join.
///
/// Since migration 0012 a row also has no winner when the winner deleted their account. The
/// reason tells the two apart: a `DECISIVE_REASONS` ending always had a winner, so for the
/// player who is left it is still a loss, not a draw.
pub async fn results_record_for(t: &mut PgTx<'_>, profile_id: &str) -> Result<ProfileRecord, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let decisive: Vec<String> = DECISIVE_REASONS.iter().map(|reason| reason.to_string()).collect();
    let row = sqlx::query(
        "select
           count(*) filter (where winner_profile_id = $1::uuid)                         as wins,
           count(*) filter (where winner_profile_id is distinct from $1::uuid
                              and (winner_profile_id is not null
                                   or reason = any($2::text[])))                         as losses,
           count(*) filter (where winner_profile_id is null
                              and reason <> all($2::text[]))                             as draws
         from public.results
        where p1_profile_id = $1::uuid or p2_profile_id = $1::uuid",
    )
    .bind(profile_id)
    .bind(decisive)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let count = |column: &str| -> Result<i64, StoreError> {
        match &row {
            None => Ok(0),
            Some(row) => Ok(get::<Option<i64>>(row, column)?.unwrap_or(0)),
        }
    };
    Ok(ProfileRecord {
        wins: count("wins")?,
        losses: count("losses")?,
        draws: count("draws")?,
    })
}

/// One row per match: `results.match_id` is the primary key, so a second insert raises.
pub async fn results_insert(t: &mut PgTx<'_>, row: &ResultRow) -> Result<(), StoreError> {
    run_as(t, None).await?;
    let written = sqlx::query(concat!(
        "insert into public.results (
             match_id, p1_profile_id, p2_profile_id, winner_profile_id, reason, turns,
             p1_rating_before, p1_rating_after, p2_rating_before, p2_rating_after, ended_at)
           values ($1::uuid, $2::uuid, $3::uuid, $4::uuid, $5::text, $6::int,
                   $7::double precision, $8::double precision, $9::double precision,
                   $10::double precision, ",
        ts!("$11"),
        ")"
    ))
    .bind(row.match_id.as_str())
    .bind(row.players.0.as_str())
    .bind(row.players.1.as_str())
    .bind(row.winner_profile_id.as_deref())
    .bind(literal(&row.reason)?)
    .bind(row.turns)
    .bind(row.rating_before.0)
    .bind(row.rating_after.0)
    .bind(row.rating_before.1)
    .bind(row.rating_after.1)
    .bind(row.ended_at)
    .execute(&mut **t)
    .await;
    match written {
        Ok(_) => Ok(()),
        // `results_pkey` answering a write whose `results_get_by_match` ran before a concurrent
        // first writer committed: surfaced as the port's own error so `results.rs`'s retry sees
        // the collision for what it is rather than as a failure of the statement.
        Err(error) if is_results_key_conflict(&error) => Err(StoreError::Duplicate(row.match_id.clone())),
        Err(error) => Err(db_error(error)),
    }
}

pub async fn results_get_by_match(t: &mut PgTx<'_>, match_id: &str) -> Result<Option<ResultRow>, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query(
        "select match_id, p1_profile_id, p2_profile_id, winner_profile_id, reason, turns,
                p1_rating_before, p1_rating_after, p2_rating_before, p2_rating_after, ended_at
           from public.results where match_id = $1::uuid",
    )
    .bind(match_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| ResultDbRow::read(&row).and_then(to_result))
        .transpose()
}

// ---------------------------------------------------------------------------
// The Conquest series (SPEC §9.5, R259-R263)
//
// Migration 0009. None of these reaches an `app.*` function, because none has a rule to hold
// that one statement does not already hold: `update` is compare-and-set in its `where`, the
// lookups are single selects, and the transitions themselves are the server's pure functions
// (`src/api/series_rules.rs`), written back whole.
// ---------------------------------------------------------------------------

/// TS's `seriesParams(row)`: the fourteen values `series_create` and `series_update` bind, in order.
struct SeriesParams {
    id: String,
    p1_profile_id: String,
    p2_profile_id: String,
    status: String,
    next_match_id: String,
    pick_deadline: Option<i64>,
    version: i32,
    catalog_version: String,
    winner: Option<String>,
    state: String,
    created_at: i64,
    updated_at: i64,
    ended_at: Option<i64>,
    ranked: bool,
}

fn series_params(row: &SeriesRow) -> Result<SeriesParams, StoreError> {
    Ok(SeriesParams {
        id: row.id.clone(),
        p1_profile_id: row.sides.0.profile_id.clone(),
        p2_profile_id: row.sides.1.profile_id.clone(),
        status: literal(&row.status)?,
        next_match_id: row.next_match_id.clone(),
        pick_deadline: row.pick_deadline,
        version: i32::try_from(row.version).unwrap_or(i32::MAX),
        catalog_version: row.catalog_version.clone(),
        winner: literal_or_null(&row.winner)?,
        state: json(&series_state_of(row))?,
        created_at: row.created_at,
        updated_at: row.updated_at,
        ended_at: row.ended_at,
        ranked: row.ranked.unwrap_or(false),
    })
}

/// A second series with the same id raises (`series_pkey`), as the port requires.
pub async fn series_create(t: &mut PgTx<'_>, row: &SeriesRow) -> Result<(), StoreError> {
    let p = series_params(row)?;
    run_as(t, Some(p.p1_profile_id.as_str())).await?;
    sqlx::query(concat!(
        "insert into public.series (
           id, p1_profile_id, p2_profile_id, status, next_match_id, pick_deadline_at, version,
           catalog_version, winner, state, created_at, updated_at, ended_at, ranked)
         values ($1::uuid, $2::uuid, $3::uuid, $4::text, $5::uuid, ",
        nullable_ts!("$6"),
        ", $7::int,
                 $8::text, $9::text, $10::jsonb, ",
        ts!("$11"),
        ", ",
        ts!("$12"),
        ", ",
        nullable_ts!("$13"),
        ",
                 $14::boolean)"
    ))
    .bind(p.id.as_str())
    .bind(p.p1_profile_id.as_str())
    .bind(p.p2_profile_id.as_str())
    .bind(p.status.as_str())
    .bind(p.next_match_id.as_str())
    .bind(p.pick_deadline)
    .bind(p.version)
    .bind(p.catalog_version.as_str())
    .bind(p.winner.as_deref())
    .bind(p.state.as_str())
    .bind(p.created_at)
    .bind(p.updated_at)
    .bind(p.ended_at)
    .bind(p.ranked)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub async fn series_get(t: &mut PgTx<'_>, series_id: &str) -> Result<Option<SeriesRow>, StoreError> {
    if !is_uuid(series_id) {
        return Ok(None);
    }
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select ",
        series_columns!(),
        " from public.series where id = $1::uuid"
    ))
    .bind(series_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| SeriesDbRow::read(&row).and_then(to_series))
        .transpose()
}

/// R263: "written only by compare-and-set on its version". The guard is the `where`: the row is
/// replaced only while it still holds the version this transition was computed from, so of two
/// writers that read the same version exactly one updates a row and the other gets `false`, re-
/// reads and re-applies. One statement, so there is no window between the check and the write.
pub async fn series_update(t: &mut PgTx<'_>, next: &SeriesRow) -> Result<bool, StoreError> {
    let p = series_params(next)?;
    run_as(t, Some(p.p1_profile_id.as_str())).await?;
    let done = sqlx::query(concat!(
        "update public.series set
           p1_profile_id = $2::uuid, p2_profile_id = $3::uuid, status = $4::text,
           next_match_id = $5::uuid, pick_deadline_at = ",
        nullable_ts!("$6"),
        ", version = $7::int,
           catalog_version = $8::text, winner = $9::text, state = $10::jsonb,
           created_at = ",
        ts!("$11"),
        ", updated_at = ",
        ts!("$12"),
        ", ended_at = ",
        nullable_ts!("$13"),
        ",
           ranked = $14::boolean
         where id = $1::uuid and version = $7::int - 1"
    ))
    .bind(p.id.as_str())
    .bind(p.p1_profile_id.as_str())
    .bind(p.p2_profile_id.as_str())
    .bind(p.status.as_str())
    .bind(p.next_match_id.as_str())
    .bind(p.pick_deadline)
    .bind(p.version)
    .bind(p.catalog_version.as_str())
    .bind(p.winner.as_deref())
    .bind(p.state.as_str())
    .bind(p.created_at)
    .bind(p.updated_at)
    .bind(p.ended_at)
    .bind(p.ranked)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(done.rows_affected() == 1)
}

/// The series whose game in play is this match: `series_next_match_id_key` (0009) makes it one.
pub async fn series_by_match(t: &mut PgTx<'_>, match_id: &str) -> Result<Option<SeriesRow>, StoreError> {
    if !is_uuid(match_id) {
        return Ok(None);
    }
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select ",
        series_columns!(),
        " from public.series
          where next_match_id = $1::uuid and status = 'playing'"
    ))
    .bind(match_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| SeriesDbRow::read(&row).and_then(to_series))
        .transpose()
}

/// Any game of any series, whatever its status: a jsonb containment test over `state -> 'games'`,
/// which `series_games_idx` (GIN, jsonb_path_ops, 0009) answers without a scan.
pub async fn series_with_game(t: &mut PgTx<'_>, match_id: &str) -> Result<Option<SeriesRow>, StoreError> {
    if !is_uuid(match_id) {
        return Ok(None);
    }
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select ",
        series_columns!(),
        " from public.series
          where state -> 'games' @> $1::jsonb
          order by created_at, id
          limit 1"
    ))
    .bind(json(&json!([{ "matchId": match_id }]))?)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| SeriesDbRow::read(&row).and_then(to_series))
        .transpose()
}

pub async fn series_active_for(t: &mut PgTx<'_>, profile_id: &str) -> Result<Option<SeriesRow>, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let row = sqlx::query(concat!(
        "select ",
        series_columns!(),
        " from public.series
          where status <> 'over' and (p1_profile_id = $1::uuid or p2_profile_id = $1::uuid)
          order by created_at, id
          limit 1"
    ))
    .bind(profile_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| SeriesDbRow::read(&row).and_then(to_series))
        .transpose()
}

/// The sweeper's input (R263), oldest first, over `series_active_idx`.
pub async fn series_active(t: &mut PgTx<'_>) -> Result<Vec<SeriesRow>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(concat!(
        "select ",
        series_columns!(),
        " from public.series where status <> 'over' order by created_at, id"
    ))
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| SeriesDbRow::read(row).and_then(to_series))
        .collect()
}

// ---------------------------------------------------------------------------
// Tutorial progress on the account (SPEC §9.10, R320)
// ---------------------------------------------------------------------------

pub async fn tutorial_get(
    t: &mut PgTx<'_>,
    profile_id: &str,
) -> Result<Option<TutorialProgressRow>, StoreError> {
    if !is_uuid(profile_id) {
        return Ok(None);
    }
    run_as(t, Some(profile_id)).await?;
    let row = sqlx::query(concat!(
        "select ",
        tutorial_columns!(),
        " from public.tutorial_progress where profile_id = $1::uuid"
    ))
    .bind(profile_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| TutorialDbRow::read(&row).map(to_tutorial))
        .transpose()
}

/// R320: one call to `app.merge_tutorial_progress` (0011), which takes the profile row lock,
/// re-checks the shape, unions the lessons and keeps the strictly newer choice; then the row as
/// it now stands, read in the same transaction so the answer is exactly what this write left.
pub async fn tutorial_merge(
    t: &mut PgTx<'_>,
    input: &TutorialMergeInput,
    max_lessons: i64,
) -> Result<TutorialMergeOutcome, StoreError> {
    let max_lessons = i32::try_from(max_lessons).unwrap_or(i32::MAX);
    run_as(t, Some(input.profile_id.as_str())).await?;
    let choice = input.hidden_choice.as_ref();
    let row = sqlx::query(concat!(
        "select app.merge_tutorial_progress($1::uuid, $2::text[], $3::boolean, ",
        nullable_ts!("$4"),
        ",
                                              ",
        ts!("$5"),
        ", $6::int) as outcome"
    ))
    .bind(input.profile_id.as_str())
    .bind(input.completed.to_vec())
    .bind(choice.map(|choice| choice.hidden))
    .bind(choice.map(|choice| choice.at))
    .bind(input.at)
    .bind(max_lessons)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let outcome: Option<String> = match row {
        None => None,
        Some(row) => get(&row, "outcome")?,
    };
    match outcome.as_deref() {
        Some("limit") => return Ok(TutorialMergeOutcome::Limit),
        Some("merged") => {}
        _ => {
            return Err(StoreError::from(format!(
                "app.merge_tutorial_progress returned {}, which is not merged or limit (migration 0011)",
                stringify(&outcome)
            )));
        }
    }
    let read = sqlx::query(concat!(
        "select ",
        tutorial_columns!(),
        " from public.tutorial_progress where profile_id = $1::uuid"
    ))
    .bind(input.profile_id.as_str())
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let Some(read) = read else {
        return Err(StoreError::from(
            "app.merge_tutorial_progress answered merged and wrote no row",
        ));
    };
    Ok(TutorialMergeOutcome::Merged {
        progress: to_tutorial(TutorialDbRow::read(&read)?),
    })
}

// ---------------------------------------------------------------------------
// Player settings on the account (SPEC §9.1, R633, R634)
// ---------------------------------------------------------------------------

pub async fn player_settings_get(
    t: &mut PgTx<'_>,
    profile_id: &str,
) -> Result<Option<PlayerSettingsRow>, StoreError> {
    if !is_uuid(profile_id) {
        return Ok(None);
    }
    run_as(t, Some(profile_id)).await?;
    let row = sqlx::query(concat!(
        "select ",
        player_settings_columns!(),
        " from public.player_settings where profile_id = $1::uuid"
    ))
    .bind(profile_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| PlayerSettingsDbRow::read(&row).map(to_player_settings))
        .transpose()
}

/// R634: one call to `app.merge_player_settings` (0018), which takes the profile row lock,
/// re-checks the shape, replaces each group only with a strictly later one and caps the result;
/// then the row as it now stands, read in the same transaction so the answer is exactly what
/// this write left.
pub async fn player_settings_merge(
    t: &mut PgTx<'_>,
    input: &PlayerSettingsMergeInput,
    limits: &PlayerSettingsLimits,
) -> Result<PlayerSettingsMergeOutcome, StoreError> {
    run_as(t, Some(input.profile_id.as_str())).await?;
    let row = sqlx::query(concat!(
        "select app.merge_player_settings($1::uuid, $2::jsonb, ",
        ts!("$3"),
        ", $4::int, $5::int) as outcome"
    ))
    .bind(input.profile_id.as_str())
    .bind(json(&input.groups)?)
    .bind(input.at)
    .bind(limits.max_groups)
    .bind(limits.max_bytes)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let outcome: Option<String> = match row {
        None => None,
        Some(row) => get(&row, "outcome")?,
    };
    match outcome.as_deref() {
        Some("limit") => return Ok(PlayerSettingsMergeOutcome::Limit),
        Some("merged") => {}
        _ => {
            return Err(StoreError::from(format!(
                "app.merge_player_settings returned {}, which is not merged or limit (migration 0018)",
                stringify(&outcome)
            )));
        }
    }
    let read = sqlx::query(concat!(
        "select ",
        player_settings_columns!(),
        " from public.player_settings where profile_id = $1::uuid"
    ))
    .bind(input.profile_id.as_str())
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let Some(read) = read else {
        return Err(StoreError::from(
            "app.merge_player_settings answered merged and wrote no row",
        ));
    };
    Ok(PlayerSettingsMergeOutcome::Merged {
        settings: to_player_settings(PlayerSettingsDbRow::read(&read)?),
    })
}

// ---------------------------------------------------------------------------
// Last boards (C+ #29, R417, R565). No `app.*` function: the one rule is "replace", which the
// primary key on (profile_id, kind) says.
// ---------------------------------------------------------------------------

pub async fn last_boards_get(
    t: &mut PgTx<'_>,
    profile_id: &str,
    kind: LastBoardKind,
) -> Result<Option<Vec<LastBoardEntry>>, StoreError> {
    if !is_uuid(profile_id) {
        return Ok(None);
    }
    run_as(t, Some(profile_id)).await?;
    let row =
        sqlx::query("select board from public.last_boards where profile_id = $1::uuid and kind = $2::text")
            .bind(profile_id)
            .bind(literal(&kind)?)
            .fetch_optional(&mut **t)
            .await
            .map_err(db_error)?;
    row.map(|row| -> Result<Vec<LastBoardEntry>, StoreError> { last_board_of(&get::<Value>(&row, "board")?) })
        .transpose()
}

pub async fn last_boards_put(
    t: &mut PgTx<'_>,
    profile_id: &str,
    kind: LastBoardKind,
    board: &[LastBoardEntry],
    at: i64,
) -> Result<(), StoreError> {
    run_as(t, Some(profile_id)).await?;
    sqlx::query(concat!(
        "insert into public.last_boards (profile_id, kind, board, created_at, updated_at)
         values ($1::uuid, $2::text, $3::jsonb, ",
        ts!("$4"),
        ", ",
        ts!("$4"),
        ")
         on conflict (profile_id, kind) do update set board = excluded.board, updated_at = excluded.updated_at"
    ))
    .bind(profile_id)
    .bind(literal(&kind)?)
    .bind(json(board)?)
    .bind(at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

/// R678: random other players' non-empty server boards, one per profile (the primary key).
pub async fn last_boards_sample_others(
    t: &mut PgTx<'_>,
    exclude_profile_ids: &[String],
    count: i64,
) -> Result<Vec<Vec<LastBoardEntry>>, StoreError> {
    let count = i32::try_from(count).unwrap_or(i32::MAX);
    if count <= 0 {
        return Ok(Vec::new());
    }
    let excluded: Vec<String> = exclude_profile_ids
        .iter()
        .filter(|id| is_uuid(id))
        .cloned()
        .collect();
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select board from public.last_boards
          where kind = 'server' and jsonb_array_length(board) > 0
            and profile_id <> all($1::uuid[])
          order by random()
          limit $2::int",
    )
    .bind(excluded)
    .bind(count)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| -> Result<Vec<LastBoardEntry>, StoreError> { last_board_of(&get::<Value>(row, "board")?) })
        .collect()
}

// ---------------------------------------------------------------------------
// Game records for the card statistics (SPEC §9.11, R376)
// ---------------------------------------------------------------------------

/// R376: one record per id; `on conflict do nothing` answers a second write of a game with false.
pub async fn game_records_insert(t: &mut PgTx<'_>, record: &GameRecord) -> Result<bool, StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query(
        "insert into public.game_records (id, source, mode, patch, record)
         values ($1::text, $2::text, $3::text, $4::text, $5::jsonb)
         on conflict (id) do nothing",
    )
    .bind(record.id.as_str())
    .bind(literal(&record.source)?)
    .bind(literal(&record.mode)?)
    .bind(record.patch.as_str())
    .bind(json(record)?)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(done.rows_affected() == 1)
}

/// The filter on the indexed columns; each row read back through `parse_game_record`.
pub async fn game_records_list(
    t: &mut PgTx<'_>,
    query: &GameRecordQuery,
) -> Result<Vec<GameRecord>, StoreError> {
    let sources = sources_of(query.source)
        .iter()
        .map(literal)
        .collect::<Result<Vec<String>, StoreError>>()?;
    let mode = query.mode.as_ref().map(literal).transpose()?;
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select record from public.game_records
          where source = any($1::text[])
            and ($2::text is null or mode = $2::text)
            and ($3::text is null or patch = $3::text)
          order by id collate \"C\"",
    )
    .bind(sources)
    .bind(mode)
    .bind(query.patch.as_deref())
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| -> Result<GameRecord, StoreError> {
            let record: Value = get(row, "record")?;
            parse_game_record(&record).map_err(|error| StoreError::from(error.to_string()))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Ranked ladder (SPEC §9.12): migration 0022's four tables — `seasons`, `season_ranks`,
// `bot_ratings` and `rated_games` — plus the Glicko triple on `profiles`. None of these reaches
// an `app.*` function, because none has a rule to hold that one statement does not already hold:
// `put_rank`/`put_bot` are upserts, `create_season`/`record_game` are one-statement idempotent
// writes, `reset_ratings` is one `unnest` update, and the lookups are single selects.
// ---------------------------------------------------------------------------

/// One advisory lock for every season open, transaction-scoped like migrate.rs's lock id: two
/// opens — same season or different seasons — run one after the other, so the second reads a
/// seasons list that already holds the first's row. TS interpolated the id into the statement; it
/// is bound here, the same number either way.
pub async fn ranked_lock_seasons(t: &mut PgTx<'_>) -> Result<(), StoreError> {
    run_as(t, None).await?;
    sqlx::query("select pg_advisory_xact_lock($1::bigint)")
        .bind(SEASON_LOCK_ID)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(())
}

/// Every season, oldest first, as the fake's ranked store answers it.
pub async fn ranked_seasons(t: &mut PgTx<'_>) -> Result<Vec<Season>, StoreError> {
    run_as(t, None).await?;
    let rows =
        sqlx::query("select id, patch_version, started_at from public.seasons order by started_at, id")
            .fetch_all(&mut **t)
            .await
            .map_err(db_error)?;
    rows.iter()
        .map(|row| SeasonDbRow::read(row).map(to_season))
        .collect()
}

/// `on conflict` answers a race to open the same season with `false`, nothing written.
pub async fn ranked_create_season(t: &mut PgTx<'_>, season: &Season) -> Result<bool, StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query(concat!(
        "insert into public.seasons (id, patch_version, started_at)
         values ($1::text, $2::text, ",
        ts!("$3"),
        ")
         on conflict (id) do nothing"
    ))
    .bind(season.id.as_str())
    .bind(season.patch_version.as_str())
    .bind(season.started_at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(done.rows_affected() == 1)
}

/// R609's input: every profile a rated game has touched, with its Glicko triple. The memory
/// store derives the same set by scanning `ratedGames`; here it is one EXISTS per side, over
/// `rated_games_p1_profile_idx` / `p2`, and a deleted profile is already gone from
/// `public.profiles`, so the join needs no liveness check.
pub async fn ranked_rated_players(t: &mut PgTx<'_>) -> Result<Vec<ResetPlayer>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select p.id, p.rating, p.rating_deviation, p.rating_volatility
           from public.profiles p
          where exists (select 1 from public.rated_games g
                         where (g.p1_profile_id = p.id and g.p1_bot_id is null)
                            or (g.p2_profile_id = p.id and g.p2_bot_id is null))
          order by p.id",
    )
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| -> Result<ResetPlayer, StoreError> {
            Ok(ResetPlayer {
                profile_id: uuid_text(row, "id")?,
                glicko: Glicko {
                    rating: get(row, "rating")?,
                    deviation: get(row, "rating_deviation")?,
                    volatility: get(row, "rating_volatility")?,
                },
            })
        })
        .collect()
}

/// One statement for the whole reset (`unnest` is how a set of rows arrives as parameters).
pub async fn ranked_reset_ratings(t: &mut PgTx<'_>, changes: &[ResetChange]) -> Result<(), StoreError> {
    if changes.is_empty() {
        return Ok(());
    }
    run_as(t, None).await?;
    sqlx::query(
        "update public.profiles p
            set rating = c.rating, rating_deviation = c.deviation, rating_volatility = c.volatility
           from unnest($1::uuid[], $2::float8[], $3::float8[], $4::float8[])
             as c(id, rating, deviation, volatility)
          where p.id = c.id",
    )
    .bind(
        changes
            .iter()
            .map(|change| change.profile_id.clone())
            .collect::<Vec<String>>(),
    )
    .bind(
        changes
            .iter()
            .map(|change| change.after.rating)
            .collect::<Vec<f64>>(),
    )
    .bind(
        changes
            .iter()
            .map(|change| change.after.deviation)
            .collect::<Vec<f64>>(),
    )
    .bind(
        changes
            .iter()
            .map(|change| change.after.volatility)
            .collect::<Vec<f64>>(),
    )
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

/// A season's rows joined to each player's CURRENT rating — what percentiles and Jlorious read.
/// TS's `{ ...toSeasonRank(row), rating }`: the rank's own fields plus the rating, through their
/// serde form.
pub async fn ranked_standings(t: &mut PgTx<'_>, season_id: &str) -> Result<Vec<SeasonStanding>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select r.season_id, r.profile_id, r.games, r.wins, r.losses, r.draws, r.ladder, r.floor,
                r.streak, r.peak_ladder, r.peak_jlorious, r.updated_at, p.rating
           from public.season_ranks r
           join public.profiles p on p.id = r.profile_id
          where r.season_id = $1::text
          order by r.profile_id",
    )
    .bind(season_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| -> Result<SeasonStanding, StoreError> {
            let rank = to_season_rank(SeasonRankDbRow::read(row)?);
            let rating: f64 = get(row, "rating")?;
            let mut standing =
                serde_json::to_value(&rank).map_err(|error| StoreError::from(error.to_string()))?;
            if let Value::Object(fields) = &mut standing {
                fields.insert(String::from("rating"), json!(rating));
            }
            from_json(standing)
        })
        .collect()
}

pub async fn ranked_rank(
    t: &mut PgTx<'_>,
    season_id: &str,
    profile_id: &str,
) -> Result<Option<SeasonRank>, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query(
        "select season_id, profile_id, games, wins, losses, draws, ladder, floor, streak,
                peak_ladder, peak_jlorious, updated_at
           from public.season_ranks
          where season_id = $1::text and profile_id = $2::uuid",
    )
    .bind(season_id)
    .bind(profile_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| SeasonRankDbRow::read(&row).map(to_season_rank))
        .transpose()
}

/// A profile's badges (R607), oldest season first, the season itself ordering them.
pub async fn ranked_ranks_of(t: &mut PgTx<'_>, profile_id: &str) -> Result<Vec<SeasonRank>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select r.season_id, r.profile_id, r.games, r.wins, r.losses, r.draws, r.ladder, r.floor,
                r.streak, r.peak_ladder, r.peak_jlorious, r.updated_at
           from public.season_ranks r
           join public.seasons s on s.id = r.season_id
          where r.profile_id = $1::uuid
          order by s.started_at, r.season_id",
    )
    .bind(profile_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| SeasonRankDbRow::read(row).map(to_season_rank))
        .collect()
}

/// Insert or replace: the primary key is what R262's rate-once bookkeeping is keyed on.
pub async fn ranked_put_rank(t: &mut PgTx<'_>, row: &SeasonRank) -> Result<(), StoreError> {
    run_as(t, Some(row.profile_id.as_str())).await?;
    sqlx::query(concat!(
        "insert into public.season_ranks
           (season_id, profile_id, games, wins, losses, draws, ladder, floor, streak,
            peak_ladder, peak_jlorious, updated_at)
         values ($1::text, $2::uuid, $3::int, $4::int, $5::int, $6::int, $7::int, $8::int, $9::int,
                 $10::int, $11::int, ",
        ts!("$12"),
        ")
         on conflict (season_id, profile_id) do update set
           games = excluded.games, wins = excluded.wins, losses = excluded.losses,
           draws = excluded.draws, ladder = excluded.ladder, floor = excluded.floor,
           streak = excluded.streak, peak_ladder = excluded.peak_ladder,
           -- least() like notePeakJlorious: a bystander's notePeakJlorious can land between
           -- this tx's rankFor read and this upsert, and an absolute write would lose it.
           peak_jlorious = least(season_ranks.peak_jlorious, excluded.peak_jlorious),
           updated_at = excluded.updated_at"
    ))
    .bind(row.season_id.as_str())
    .bind(row.profile_id.as_str())
    .bind(row.games)
    .bind(row.wins)
    .bind(row.losses)
    .bind(row.draws)
    .bind(row.ladder)
    .bind(row.floor)
    .bind(row.streak)
    .bind(row.peak_ladder)
    .bind(row.peak_jlorious)
    .bind(row.updated_at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

/// R608: `least(a, b)` ignores NULL in Postgres, so a null `peak_jlorious` takes the position
/// and a recorded one keeps the better (lower) of the two — `min` with the null case the
/// memory store writes out.
pub async fn ranked_note_peak_jlorious(
    t: &mut PgTx<'_>,
    season_id: &str,
    profile_id: &str,
    position: i64,
) -> Result<(), StoreError> {
    let position = i32::try_from(position).unwrap_or(i32::MAX);
    run_as(t, Some(profile_id)).await?;
    sqlx::query(
        "update public.season_ranks set peak_jlorious = least(peak_jlorious, $3::int)
          where season_id = $1::text and profile_id = $2::uuid",
    )
    .bind(season_id)
    .bind(profile_id)
    .bind(position)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub async fn ranked_bot(t: &mut PgTx<'_>, bot_id: &str) -> Result<Option<BotRating>, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query(
        "select bot_id, rating, deviation, volatility, games, updated_at
           from public.bot_ratings where bot_id = $1::text",
    )
    .bind(bot_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| BotRatingDbRow::read(&row).map(to_bot_rating))
        .transpose()
}

pub async fn ranked_put_bot(t: &mut PgTx<'_>, bot: &BotRating) -> Result<(), StoreError> {
    run_as(t, None).await?;
    sqlx::query(concat!(
        "insert into public.bot_ratings (bot_id, rating, deviation, volatility, games, updated_at)
         values ($1::text, $2::float8, $3::float8, $4::float8, $5::int, ",
        ts!("$6"),
        ")
         on conflict (bot_id) do update set
           rating = excluded.rating, deviation = excluded.deviation,
           volatility = excluded.volatility, games = excluded.games, updated_at = excluded.updated_at"
    ))
    .bind(bot.bot_id.as_str())
    .bind(bot.glicko.rating)
    .bind(bot.glicko.deviation)
    .bind(bot.glicko.volatility)
    .bind(bot.games)
    .bind(bot.updated_at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

/// R611's row, and R262's rate-once guard: `rated_games_pkey` refuses a second row for the same
/// match or series id, and `on conflict` turns that refusal into the error the port raises.
pub async fn ranked_record_game(t: &mut PgTx<'_>, row: &RatedGameRow) -> Result<(), StoreError> {
    let (first, second) = &row.sides;
    run_as(t, None).await?;
    let done = sqlx::query(concat!(
        "insert into public.rated_games (
           id, kind, season_id, patch_version, catalog_version,
           p1_profile_id, p1_bot_id, p1_pilot, p1_before, p1_after, p1_rank_before, p1_rank_after,
           p2_profile_id, p2_bot_id, p2_pilot, p2_before, p2_after, p2_rank_before, p2_rank_after,
           winner_side, reason, ended_at)
         values ($1::uuid, $2::text, $3::text, $4::text, $5::text,
                 $6::uuid, $7::text, $8::text, $9::jsonb, $10::jsonb, $11::jsonb, $12::jsonb,
                 $13::uuid, $14::text, $15::text, $16::jsonb, $17::jsonb, $18::jsonb, $19::jsonb,
                 $20::smallint, $21::text, ",
        ts!("$22"),
        ")
         on conflict (id) do nothing"
    ))
    .bind(row.id.as_str())
    .bind(literal(&row.kind)?)
    .bind(row.season_id.as_str())
    .bind(row.patch_version.as_str())
    .bind(row.catalog_version.as_str())
    .bind(first.profile_id.as_deref())
    .bind(first.bot_id.as_deref())
    .bind(literal(&first.pilot)?)
    .bind(json(&first.before)?)
    .bind(json(&first.after)?)
    .bind(json_or_null(&first.rank_before)?)
    .bind(json_or_null(&first.rank_after)?)
    .bind(second.profile_id.as_deref())
    .bind(second.bot_id.as_deref())
    .bind(literal(&second.pilot)?)
    .bind(json(&second.before)?)
    .bind(json(&second.after)?)
    .bind(json_or_null(&second.rank_before)?)
    .bind(json_or_null(&second.rank_after)?)
    .bind(int_or_null(&row.winner_side)?)
    .bind(literal(&row.reason)?)
    .bind(row.ended_at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    if done.rows_affected() == 0 {
        return Err(StoreError::from(format!(
            "rated_games already holds a row for {}",
            row.id
        )));
    }
    Ok(())
}

pub async fn ranked_game(t: &mut PgTx<'_>, game_id: &str) -> Result<Option<RatedGameRow>, StoreError> {
    if !is_uuid(game_id) {
        return Ok(None);
    }
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select ",
        rated_game_columns!(),
        " from public.rated_games where id = $1::uuid"
    ))
    .bind(game_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| RatedGameDbRow::read(&row).and_then(to_rated_game))
        .transpose()
}

// ---------------------------------------------------------------------------
// Player statistics on the account (SPEC §9.11, R639, R654)
// ---------------------------------------------------------------------------

pub async fn player_stats_get(
    t: &mut PgTx<'_>,
    profile_id: &str,
) -> Result<Option<PlayerStatsRow>, StoreError> {
    if !is_uuid(profile_id) {
        return Ok(None);
    }
    run_as(t, Some(profile_id)).await?;
    let row = sqlx::query(concat!(
        "select ",
        player_stats_columns!(),
        " from public.player_stats where profile_id = $1::uuid"
    ))
    .bind(profile_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| PlayerStatsDbRow::read(&row).map(to_player_stats))
        .transpose()
}

pub async fn player_stats_put(
    t: &mut PgTx<'_>,
    profile_id: &str,
    stats: &IndexMap<String, Value>,
    is_private: bool,
    at: i64,
) -> Result<(), StoreError> {
    if !is_uuid(profile_id) {
        return Ok(());
    }
    run_as(t, Some(profile_id)).await?;
    sqlx::query(concat!(
        "insert into public.player_stats (profile_id, stats, is_private, created_at, updated_at)
         values ($1::uuid, $2::jsonb, $3::boolean, ",
        ts!("$4"),
        ", ",
        ts!("$4"),
        ")
         on conflict (profile_id) do update set stats = excluded.stats, is_private = excluded.is_private, updated_at = excluded.updated_at"
    ))
    .bind(profile_id)
    .bind(json(stats)?)
    .bind(is_private)
    .bind(at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub async fn player_stats_list_public(
    t: &mut PgTx<'_>,
    options: &PlayerStatsListOptions,
) -> Result<Vec<PublicPlayerSummary>, StoreError> {
    let term = options
        .search
        .as_deref()
        .map(str::trim)
        .filter(|term| !term.is_empty());
    // The SQL keeps TS's `$2::int`/`$3::int`, so the binds are `int4`.
    let limit = i32::try_from(options.limit).unwrap_or(i32::MAX);
    let offset = i32::try_from(options.offset).unwrap_or(i32::MAX);
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select ps.profile_id, p.display_name, ps.stats, ps.updated_at
         from public.player_stats ps
         join public.profiles p on p.id = ps.profile_id
         where ps.is_private = false
           and ($1::text is null or (p.display_name is not null and position(lower($1::text) in lower(p.display_name)) > 0))
         order by
           case when (ps.stats->>'games') ~ '^[0-9]+$' then (ps.stats->>'games')::bigint else 0 end desc,
           ps.updated_at desc,
           ps.profile_id asc
         limit $2::int offset $3::int",
    )
    .bind(term)
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| -> Result<PublicPlayerSummary, StoreError> {
            Ok(to_public_player_summary(
                &uuid_text(row, "profile_id")?,
                get::<Option<String>>(row, "display_name")?.as_deref(),
                &stats_col(row, "stats"),
                ms_of(get(row, "updated_at")?),
            ))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Small helpers
// ---------------------------------------------------------------------------

fn from_ticket_status(status: &TicketStatus) -> Result<&'static str, StoreError> {
    Ok(match literal(status)?.as_str() {
        "open" => "queued",
        "matched" => "claimed",
        _ => "cancelled",
    })
}

// =============================================================================
// KNOWN DIVERGENCES from `src/db/fake.rs` (TS: `src/api/e2e-store.ts`)
//
// Every one of these is a place where the port and the schema (which this file may not change:
// the migrations are append-only and checksum-locked) do not hold the same information. They are
// listed here, asserted in `tests/store/contract.rs` where they are observable, and repeated in the
// report that came with this file.
//
//  * redemption limits. §9.4's "more than 5 attempts", "more than 20" and "the last hour" are
//    written into `app.redeem_invite_code`'s body, and the breaker's knobs into `app.settings`;
//    the migrations are checksum-locked, so neither can be made to read `ApiLimits`. The in-memory
//    store takes all three as options defaulting to the same `src/config.rs` constants
//    `default_limits()` copies into `ApiLimits`, so the two agree at today's values and a test can
//    shrink them on the fake path only. Changing a number for a real deployment means
//    `app.settings` and a new migration, not `src/config.rs`.
//  * attempt reason. `CodeAttempt.reason` ("a coarse reason for operators") has no column in
//    `public.code_attempts`, which stores only `succeeded`. It is dropped on write, so a store
//    round-trip cannot return it. Nothing reads it back today (the code store has no attempt read),
//    and `redeem` returns a result code rather than a reason for the same reason: the
//    granularity the fake can record is granularity Postgres cannot.
//  * redemption's email check. `app.redeem_invite_code` reads `auth.users.email_confirmed_at`
//    itself; the in-memory store has no such table and answers through
//    `RedemptionSettings.email_verified`, true unless a caller says otherwise. The server refuses an
//    unverified caller from the access token (R159) before either store is reached, so this only
//    shows up in a test that calls the port directly — which `tests/store/contract.rs` does.
//  * redemption's circuit breaker. The SQL function refuses when `app.settings.redemption_enabled`
//    is false OR when its own count of recent failures crosses R106's threshold. The in-memory
//    store implements the switch and not the counter, because the server's own R106 breaker
//    (`src/api/codes.rs`) is checked before the store is touched, holds the same numbers and is
//    what alerts — a fake that counted as well would open during BUILD M6-T1's 150-sample timing
//    test, whose whole point is 150 uninterrupted failures.
//  * profile email. `public.profiles` has no email column — §9.4's managed auth provider owns it
//    on `auth.users` — so `Profile.email` is read from there (`service_role` needs SELECT on
//    `auth.users`, which Supabase grants) and the `email` argument to `profiles_create` is ignored.
//    `profiles_create` also requires the `auth.users` row to exist, because `profiles.id`
//    references it; the fake has no such requirement.
//  * foreign keys. `results.match_id`, `tickets.match_id` and `profiles.current_match_id` all
//    reference `public.matches`, and `decks`, `trios` and both sides of `series` reference
//    `public.profiles`, so a result or an in-match pointer for a match that was never created, or
//    a deck, a trio or a series for a profile that does not exist, raises here and is accepted by
//    the fake. Every caller in `src/api/**` writes the match or has the profile first, so this
//    only shows up in a test that skipped it. `series.next_match_id` is the one match id with NO
//    foreign key, on purpose (R263: it is reserved before its match exists).
//  * deck and trio strictness. `app.upsert_deck` and `app.upsert_trio` (0007) refuse — by raising —
//    a profile that is not active, a blank name, a name past `deck_name_max_length` or holding a
//    control character (D1, T1), more than `deck_size` cards (D2), more than `max_copies` of one id
//    (D4) and a cards value that is not an array of strings; the in-memory store checks none of
//    these, because the server's `check_deck_draft` / `check_trio_draft` refuse them first with a
//    sentence a player reads. The real store is strictly stricter, which is §9.4's "defense in
//    depth" by design. D3 (a deckable card) is checked by neither store: see 0007.
//  * tutorial strictness (R320). `app.merge_tutorial_progress` (0011) refuses — by raising — a
//    profile that is not active, a lesson id that is not a lower-case slug of at most
//    `tutorial_lesson_id_max_length` characters, and a choice with no time; and a profile with no
//    `public.profiles` row fails its foreign key. The in-memory store checks none of these, because
//    `src/api/tutorial.rs` refuses a malformed body first and its route is `active`. The merge
//    itself (the union, the strictly-newer choice, the `limit` outcome) is the same in both and is
//    asserted in `tests/store/contract.rs`. Its cap is the smaller of the caller's and
//    `app.settings.tutorial_lessons_max`, as for the deck caps below.
//  * player settings strictness (R633). `app.merge_player_settings` (0018) refuses — by raising — a
//    profile that is not active and a group that is not `{ at: whole milliseconds, values: an
//    object }`; the in-memory store checks neither, because `src/api/settings.rs` refuses a
//    malformed body first and its route is `active`. The merge itself (the strictly-later group,
//    the groups a write leaves alone, the `limit` outcome) is the same in both and is asserted in
//    `tests/store/contract.rs`. Its caps are the smaller of the caller's and `app.settings`'
//    `player_settings_groups_max` / `player_settings_bytes_max`. The byte cap counts the text
//    jsonb prints, which the in-memory store reproduces (`jsonb_text_bytes`) for ordinary values.
//  * caps. The port passes the cap (`max_decks`, `max_trios`); the SQL applies the smaller of it and
//    `app.settings.max_saved_decks` / `max_saved_trios` (0007, mirroring `MAX_SAVED_DECKS` and
//    `MAX_SAVED_TRIOS`). Equal today. Raising a cap in `src/config.rs` alone raises it only in
//    memory; the database needs a migration updating its row, as for the redemption limits.
//  * deck and trio timestamps. `app.upsert_deck` / `app.upsert_trio` take one instant: `updated_at`
//    always, `created_at` too when the row is new, and this store hands them `updated_at`. The fake
//    stores a new deck's `created_at` as given. The two agree for any caller that stamps a new
//    deck's `created_at` and `updated_at` from one clock read, which is what a save is.
//  * R254's decks. A deck migration 0007 made from a loadout holds its cards in
//    `app.resolve_deck` order, which is card-id order: a loadout never had any other. Every deck
//    saved since keeps the order it was saved in.
//  * launch grant. Migration 0002's trigger grants every NON-TOKEN card of the current catalog
//    version; the fake also skips BANNED ids (§9.4 L6). With no banned card in the catalog the two
//    agree exactly; with one, the database grants a card the fake does not.
//  * action timestamps. `app.append_match_action` stamps `at` with the database clock and
//    `match_actions` is append-only, so `MatchActionRow.at` cannot be written by the caller. The
//    log's order (`seq`) is unaffected, and nothing reads `at` back except a replay tool.
//  * rooms. There is no `rooms` table: a room is a `public.matches` row with `status = 'open'`, and
//    `Room.expires_at` is kept in `ceiling_at`, the column migration 0004 documents as meaningless
//    while a room is open. `rooms_claim` rewrites the row's id to the match id the server minted.
//    `Room.mode` and `Room.host_trio` are `room_mode` / `room_trio` (0008); a room row with no mode
//    — written before 0008, or by 0004's `app.create_room` — reads as `bo1`, which is all a room
//    could be then.
//  * reserved match ids. In Postgres the id `tickets_claim_pair` or `rooms_claim` reserves is an
//    `open` row, so `matches_discard_open` (R263) deletes a row: the claimed tickets' `match_id`
//    goes back to null (`tickets.match_id` is `on delete set null`) and a claimed room's code is
//    free again. The fake keeps no such row, and its `discard_open` changes nothing; there a
//    matched ticket keeps the discarded id.
//  * deleted accounts. Migration 0012 sets a deleted profile's seat on its finished matches,
//    results and series to NULL, and migration 0022 does the same on `rated_games`' sides while
//    cascading its `season_ranks` away, so `matches_get`, `results_get_by_match`, `series_get` and
//    `ranked_game` can read back a null where the port types a profile id. TS handed that null on in
//    a string-typed slot; a Rust `String` cannot hold it, so a deleted seat reads back as the empty
//    string (a match's second seat excepted: `to_match` refuses a row with no `p2_profile_id`, as
//    TS's did, except for a replay's read, `replay_row`, R768). The in-memory store keeps the id (it
//    has no foreign keys). Only a replay reads a finished match's seats back, and a live match or
//    series cannot lose a seat: the delete is refused, by constraint here and by
//    `DELETE /api/account` first.
//  * clocks. `MatchClocks` has a grace deadline per player; `public.matches` has one
//    `grace_deadline_at` plus two `*_disconnected_at`. The per-player deadlines are stored in the
//    two `*_disconnected_at` columns and `grace_deadline_at` keeps the nearer of them. A migration
//    adding `p1_grace_deadline_at` / `p2_grace_deadline_at` would retire this.
// =============================================================================
