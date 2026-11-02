//! The production store (SPEC §9.2's `API functions -> Postgres` edge) over `crates/server/migrations`, with
//! sqlx (SURFACE §11.1, §11.2): one free async fn per store method, named `<substore>_<method>` (the root
//! `redeem` and `purge_expired` keep their names), taking the open transaction `Tx::Pg` holds. The SQL goes
//! through `sqlx::query` with binds, never the `query!` macros (they need a live database at build time).
//!
//! 1. Call the `app.*` functions, do not re-derive them: SPEC §9.4/§9.5's "one transaction", "one atomic
//!    statement" and "append-only" live inside SECURITY DEFINER functions, so a method here is one call to
//!    one where the port's shape admits it, and says why when it is not.
//! 2. The server is `service_role`, never the owner: every transaction switches to it and stamps
//!    `request.jwt.claim.sub` (the GUC `auth.uid()` reads) with the call's profile. `SET LOCAL` is a no-op
//!    outside a transaction, so every method re-stamps its subject inside the one `begin` opened (`run_as`);
//!    `SESSION_SQL` alone sets either GUC (`tests/store/postgres.rs`).
//! 3. `src/db/fake.rs` is the behavioral specification: `tests/store/contract.rs` runs the invariants against
//!    both stores; where they cannot agree the divergence is listed in `KNOWN DIVERGENCES` at the bottom.

use std::time::Duration;

use indexmap::{IndexMap, IndexSet};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sqlx::postgres::{PgPool, PgPoolOptions, PgRow};
use sqlx::types::Json;
use sqlx::{Postgres, Row, Transaction};
use time::OffsetDateTime;
use uuid::Uuid;

use jackioh_engine::wire::{
    CardTotal, CoCount, DrillCount, FirstMet, FirstSeen, GameEventType, GameRecord, ListTotal, RecordsKey,
    StatsTotals, TotalsKey, parse_game_record, portrait_or_default, sources_of,
};
use jackioh_engine::{Action, CustomCard, LastBoardEntry, LeanPick, PlayerId};

use crate::auth::is_uuid;
use crate::bots::cosmetics::BotCosmetics;
use crate::config::{NANOS_PER_MS, RATING_DEVIATION_START, RATING_START, RATING_VOLATILITY_START};
use crate::db::fake::to_public_player_summary;
use crate::db::store::{
    ActionTimingRow, BatchFolded, BotDeck, BotRating, BotRow, CodeAttempt, CollectionEntry, CollectionGrant,
    ConsentRow, CustomCardCreateOutcome, CustomCardUpdateOutcome, CustomCardWrite, DeckVersion,
    DeckVersionOutcome, EmoteEventRow, FoldedRecords, FrozenDeck, FrozenTrio, GameRecordQuery, InviteCluster,
    InviteCode, LastBoardKind, LoadoutSample, MatchActionRow, MatchCheckpointRow, MatchClocks, MatchRow,
    MatchSeats, MatchSignalRow, NewBot, PerMode, PlayTelemetry, PlayerSettingsGroup, PlayerSettingsLimits,
    PlayerSettingsMergeInput, PlayerSettingsMergeOutcome, PlayerSettingsRow, PlayerStatsListOptions,
    PlayerStatsRow, Profile, ProfileCreateInput, ProfileKind, ProfileRecord, ProfileStatus,
    PublicPlayerSummary, QueueMode, RatedGameRow, RatedSide, RedeemInviteCodeInput, RedeemResult, ReplayRow,
    ReplayWithMode, ResultRow, RetentionPurgeInput, RetentionPurgeResult, Room, SavedCustomCard, SavedDeck,
    SavedTrio, Season, SeasonStanding, SeatedProfile, SeriesRow, SeriesSide, SignalOutcomeRow,
    StatsTotalsQuery, StoreError, TermsAcceptanceRow, TermsSource, Ticket, TicketStatus, TicketWait,
    TrioUpsertOutcome, TutorialHiddenChoice, TutorialMergeInput, TutorialMergeOutcome, TutorialProgressRow,
    UnreadableGameRecord, UpsertOutcome, UsernameClaim, UsernameClaimOutcome, deck_version_step,
};
use crate::loadout::{DeckSlots, Loadout};
use crate::ranked::glicko2::Glicko;
use crate::ranked::ladder::SeasonRank;
use crate::ranked::season::{ResetChange, ResetPlayer};
use crate::username::{render_username, username_key};

pub type PgTx<'a> = Transaction<'a, Postgres>;

/// The endings that always have a winner (`GameOverReason`; `won-by-effect` R1412, `voted-out` R1843000,
/// `fled-decisive` R1843002). The others are draws, but `alt-win` is either (two players holding an
/// alternative win draw, R850) and `fled` is the winner's win and the other's draw, or both draw when two
/// fled (R1843004).
const DECISIVE_REASONS: &[&str] = &[
    "hero-death",
    "concede",
    "disconnect",
    "won-by-effect",
    "voted-out",
    "fled-decisive",
];

/// R1843004: the engine's `LOSER_DRAWS_REASONS` as the strings `results.reason` holds.
fn loser_draws_reasons() -> Vec<String> {
    jackioh_engine::config::LOSER_DRAWS_REASONS
        .iter()
        .map(|reason| reason.as_str().to_string())
        .collect()
}

/// Migration 0001 §8's `service_role`: the only role granted EXECUTE on the `app.*` functions, so a call this
/// file gets wrong fails with `insufficient_privilege` instead of succeeding as the table's owner.
const ACTING_ROLE: &str = "service_role";

const UNIQUE_VIOLATION: &str = "23505";

/// The primary key whose violation is the port's `StoreError::Duplicate`.
const RESULTS_PKEY: &str = "results_pkey";

fn is_results_key_conflict(error: &sqlx::Error) -> bool {
    match error {
        sqlx::Error::Database(held) => {
            held.code().as_deref() == Some(UNIQUE_VIOLATION) && held.constraint() == Some(RESULTS_PKEY)
        }
        _ => false,
    }
}

/// One statement, two `SET LOCAL`s: `set_config(name, value, true)` takes parameters, so the profile id is
/// bound rather than interpolated. `service_role` carries BYPASSRLS, so the subject changes no result today;
/// it is set so a future policy, trigger or SECURITY INVOKER helper consulting `auth.uid()`
/// (`tests/sql/00_supabase_stub.sql` stands it up for a plain Postgres) sees the profile the call is about.
const SESSION_SQL: &str =
    "select set_config('role', $1, true), set_config('request.jwt.claim.sub', $2, true)";

fn db_error(error: sqlx::Error) -> StoreError {
    StoreError::Db(error)
}

// Value conversions. The port speaks epoch milliseconds and plain JSON; Postgres speaks
// timestamptz, jsonb, uuid and int8.

/// `timestamptz` -> epoch ms, the microseconds dropped. The co-op and economy stores read theirs with
/// it too (`duos::store`, `economy::store`).
pub(crate) fn ms_of(value: OffsetDateTime) -> i64 {
    (value.unix_timestamp_nanos() / NANOS_PER_MS) as i64
}

fn ms_or_null(value: Option<OffsetDateTime>) -> Option<i64> {
    value.map(ms_of)
}

macro_rules! ts {
    ($param:literal) => {
        concat!("to_timestamp(", $param, "::double precision / 1000.0)")
    };
}

/// `ts!` for a nullable epoch-ms parameter. A macro is defined before its first use, hence here.
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

/// An int8 that arrives as text (`seq::text`, the `::text` of `app.append_match_action`).
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

/// A frozen trio (R259) as `tickets.frozen_trio`, `matches.room_trio` and the series state hold it. Migration
/// 0008 guarantees three decks on the two columns; the rest is checked just far enough to fail here, by name.
fn frozen_trio_of(value: &Value) -> Result<FrozenTrio, StoreError> {
    // An array passes this check and fails the next, at its missing `decks`.
    if !(value.is_object() || value.is_array()) {
        return Err(StoreError::from("expected a frozen trio object"));
    }
    let Some(decks) = value.get("decks").and_then(Value::as_array) else {
        return Err(StoreError::from("expected a frozen trio's decks array"));
    };
    let empty = Value::Object(serde_json::Map::new());
    let mut frozen: Vec<FrozenDeck> = Vec::with_capacity(decks.len());
    for deck in decks {
        let entry = if deck.is_null() { &empty } else { deck };
        let name = text_of(entry.get("name"))?;
        let cards = card_list_of(entry.get("cards").unwrap_or(&Value::Null))?;
        // R642: the portrait freezes with the deck; absent stays absent, null stays null.
        let portrait = match entry.get("portrait") {
            None => None,
            Some(Value::Null) => Some(None),
            Some(other) => Some(Some(text_of(Some(other))?)),
        };
        let slots = match entry.get("slots") {
            None | Some(Value::Null) => None,
            Some(slots) => Some(
                serde_json::from_value(slots.clone())
                    .map_err(|error| StoreError::from(format!("expected a frozen deck's slots: {error}")))?,
            ),
        };
        frozen.push(FrozenDeck {
            name,
            cards,
            portrait,
            slots,
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

/// R924000: a stored lean (`tickets.lean_set`, `matches.room_lean_set`): `null` is none, the column's check
/// holds the rest to "newest" and the sets' names. A set that no longer ships is an error, never another
/// lean.
fn lean_or_none(value: Option<&str>) -> Result<Option<LeanPick>, StoreError> {
    value
        .map(|text| {
            LeanPick::parse(text).ok_or_else(|| StoreError::from(format!("expected a lean, got {text:?}")))
        })
        .transpose()
}

/// R1834002: a frozen deck's slots (`tickets.deck_cosmetics`, `matches.room_deck_cosmetics`):
/// `null` is none, and the columns' checks hold the rest to an object.
fn deck_slots_or_none(value: Option<Value>) -> Result<Option<DeckSlots>, StoreError> {
    value
        .filter(|value| !value.is_null())
        .map(|value| {
            serde_json::from_value(value)
                .map_err(|error| StoreError::from(format!("expected a deck's slots: {error}")))
        })
        .transpose()
}

/// R1834002: `matches.loadouts`, both seats' in seat order, or `null` on a match from before 0061.
fn loadouts_or_none(value: Option<Value>) -> Result<Option<(Loadout, Loadout)>, StoreError> {
    value
        .filter(|value| !value.is_null())
        .map(|value| {
            serde_json::from_value(value)
                .map_err(|error| StoreError::from(format!("expected both seats' loadouts: {error}")))
        })
        .transpose()
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

const STAKES: &[i16] = &[1, 2];

/// `matches.stake` carries `check (stake is null or stake in (1, 2))` (migration 0023, R672). Read
/// into whatever `MatchRow.stake` holds, through its serde form.
fn stake_of<T: DeserializeOwned>(value: i16) -> Result<T, StoreError> {
    if STAKES.contains(&value) {
        return from_json(json!(value));
    }
    Err(StoreError::from(format!(
        "expected rematch stakes of 1 or 2, got {}",
        stringify(&value)
    )))
}

fn json<T: Serialize + ?Sized>(value: &T) -> Result<String, StoreError> {
    serde_json::to_string(value).map_err(|error| StoreError::from(error.to_string()))
}

fn json_or_null<T: Serialize>(value: &Option<T>) -> Result<Option<String>, StoreError> {
    value.as_ref().map(json).transpose()
}

fn stringify<T: Serialize + ?Sized>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| String::from("undefined"))
}

/// A string union's literal as its serde form writes it, the text the column holds (SURFACE §4.3).
fn literal<T: Serialize + ?Sized>(value: &T) -> Result<String, StoreError> {
    match serde_json::to_value(value) {
        Ok(Value::String(text)) => Ok(text),
        Ok(other) => Err(StoreError::from(format!(
            "expected a string literal, got {other}"
        ))),
        Err(error) => Err(StoreError::from(error.to_string())),
    }
}

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

fn from_literal<T: DeserializeOwned>(text: &str) -> Result<T, StoreError> {
    from_json(Value::String(text.to_string()))
}

fn from_json<T: DeserializeOwned>(value: Value) -> Result<T, StoreError> {
    serde_json::from_value(value).map_err(|error| StoreError::from(error.to_string()))
}

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

// Reading a row. sqlx decodes by the column's own type, strictly: a uuid column is a `Uuid`, an
// int4 an `i32`, an int8 an `i64`.

fn get<'r, T>(row: &'r PgRow, column: &str) -> Result<T, StoreError>
where
    T: sqlx::Decode<'r, Postgres> + sqlx::Type<Postgres>,
{
    row.try_get(column).map_err(db_error)
}

/// A `uuid` column as the text the port carries (lower case, hyphenated: Postgres's own text form).
/// The co-op store reads its ids with it and the next one too (`duos::store`).
pub(crate) fn uuid_text(row: &PgRow, column: &str) -> Result<String, StoreError> {
    Ok(get::<Uuid>(row, column)?.to_string())
}

pub(crate) fn uuid_text_or_null(row: &PgRow, column: &str) -> Result<Option<String>, StoreError> {
    Ok(get::<Option<Uuid>>(row, column)?.map(|id| id.to_string()))
}

/// A jsonb column read straight into a port type, so an object keeps the key order Postgres printed
/// it in; `serde_json::Value` would sort the keys.
fn json_col<T: DeserializeOwned>(row: &PgRow, column: &str) -> Result<T, StoreError> {
    Ok(get::<Json<T>>(row, column)?.0)
}

// Sessions: one transaction per call, the role switched inside it

/// `Db::begin`'s Postgres half. Every statement between `begin` and `commit` lands on the one connection the
/// transaction holds, which is what makes SPEC §9.4's "Redemption is one server-side transaction" and R263's
/// one-transaction result, series record and rating move true. A failed statement aborts the transaction, so
/// a caller that catches a store error and goes on (`api/queue.rs`, `api/results.rs`) begins a fresh `Tx`.
pub async fn begin(pool: &PgPool, claim_sub: Option<&str>) -> Result<PgTx<'static>, StoreError> {
    let mut t = pool.begin().await.map_err(db_error)?;
    run_as(&mut t, claim_sub).await?;
    Ok(t)
}

pub async fn commit(t: PgTx<'_>) -> Result<(), StoreError> {
    t.commit().await.map_err(db_error)
}

/// Re-stamps the subject inside an open transaction (the role stays `service_role` until commit): the
/// profile the call is about, or `None` for a call about nobody. The method's later statements commit or
/// roll back with it, which `matches_create` (read-lock, then write) and `tickets_claim_pair` (claim, then
/// link) need to be safe, and a mid-method error leaves nothing behind. The co-op and economy stores'
/// methods re-stamp theirs with it too (`duos::store`, `economy::store`).
pub(crate) async fn run_as(t: &mut PgTx<'_>, subject: Option<&str>) -> Result<(), StoreError> {
    sqlx::query(SESSION_SQL)
        .bind(ACTING_ROLE)
        .bind(subject.unwrap_or(""))
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(())
}

// Row shapes

struct ProfileRow {
    id: String,
    kind: String,
    status: String,
    rating: f64,
    rating_deviation: f64,
    rating_volatility: f64,
    current_match_id: Option<String>,
    created_at: OffsetDateTime,
    email: Option<String>,
    username_base: String,
    username_key: String,
    username_tag: Option<i32>,
    username_changed_at: Option<OffsetDateTime>,
    username_prompted: bool,
}

impl ProfileRow {
    fn read(row: &PgRow) -> Result<ProfileRow, StoreError> {
        Ok(ProfileRow {
            id: uuid_text(row, "id")?,
            kind: get(row, "kind")?,
            status: get(row, "status")?,
            rating: get(row, "rating")?,
            rating_deviation: get(row, "rating_deviation")?,
            rating_volatility: get(row, "rating_volatility")?,
            current_match_id: uuid_text_or_null(row, "current_match_id")?,
            created_at: get(row, "created_at")?,
            email: get(row, "email")?,
            username_base: get(row, "username_base")?,
            username_key: get(row, "username_key")?,
            username_tag: get(row, "username_tag")?,
            username_changed_at: get(row, "username_changed_at")?,
            username_prompted: get(row, "username_prompted")?,
        })
    }
}

macro_rules! profile_columns {
    () => {
        "p.id, p.kind, p.status, p.rating, p.rating_deviation, p.rating_volatility,
  p.current_match_id, p.created_at, u.email, p.username_base, p.username_key, p.username_tag,
  p.username_changed_at, p.username_prompted"
    };
}

macro_rules! profile_from {
    () => {
        "from public.profiles p left join auth.users u on u.id = p.id"
    };
}

const PROFILE_STATUSES: &[&str] = &["pending", "active", "banned"];

fn to_profile(row: ProfileRow) -> Result<Profile, StoreError> {
    let status = row.status;
    if !PROFILE_STATUSES.contains(&status.as_str()) {
        return Err(StoreError::from(format!(
            "profiles.status holds an unknown value: {status}"
        )));
    }
    let status: ProfileStatus = from_literal(&status)?;
    let kind: ProfileKind = from_literal(&row.kind)?;
    Ok(Profile {
        kind,
        // Migration 0001 keys `profiles.id` 1:1 to `auth.users(id)` (SPEC §9.4): the profile id is the user
        // id.
        user_id: row.id.clone(),
        id: row.id,
        email: row.email.unwrap_or_default(),
        username_base: row.username_base,
        username_key: row.username_key,
        username_tag: row.username_tag.map(i64::from),
        username_changed_at: row.username_changed_at.map(ms_of),
        username_prompted: row.username_prompted,
        status,
        rating: row.rating,
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
    mode: Option<String>,
    stake: Option<i16>,
    final_hash: Option<String>,
    loadouts: Option<Value>,
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
            loadouts: get(row, "loadouts")?,
        })
    }
}

macro_rules! match_columns {
    () => {
        "id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck,
  catalog_version, turn_deadline_at, prompt_deadline_at, p1_disconnected_at, p2_disconnected_at,
  ceiling_at, created_at, ended_at, p1_last_board, p2_last_board, ranked, p1_portrait, p2_portrait,
  mode, stake, p1_glitch_board, p2_glitch_board, final_hash, loadouts"
    };
}

/// R417: a stored board (`last_boards.board`, `matches.p*_last_board`), as migration 0017's CHECK
/// admits it. Each entry is taken field by field and unchecked.
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
    // R642: absent when neither seat carried a portrait; both then read as `vanilla`.
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
        // See KNOWN DIVERGENCES (deleted accounts): a deleted seat, either one, reads back as an empty id.
        players: (
            row.p1_profile_id.unwrap_or_default(),
            row.p2_profile_id.unwrap_or_default(),
        ),
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
        // R604: absent when false, as `MatchRow` types it; `results.rs` reads a missing flag as unranked.
        ranked: if row.ranked { Some(true) } else { None },
        // R672: a null mode is "derive it" (`matches_mode_of` reads the row first) and a null stake a normal
        // game; both stay absent as `MatchRow` types them.
        mode: row.mode.as_deref().map(queue_mode_of).transpose()?,
        stake: row.stake.map(stake_of).transpose()?,
        portraits,
        loadouts: loadouts_or_none(row.loadouts)?,
        final_hash: row.final_hash,
    })
}

/// The nearer per-player deadline, kept in `grace_deadline_at` so a SQL reader finds the grace deadline where
/// migration 0004 says it is (KNOWN DIVERGENCES, clocks).
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
    deck_cosmetics: Option<Value>,
    frozen_trio: Option<Value>,
    lean_set: Option<String>,
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
            deck_cosmetics: get(row, "deck_cosmetics")?,
            frozen_trio: get(row, "frozen_trio")?,
            lean_set: get(row, "lean_set")?,
            catalog_version: get(row, "catalog_version")?,
            status: get(row, "status")?,
            enqueued_at: get(row, "enqueued_at")?,
            match_id: uuid_text_or_null(row, "match_id")?,
        })
    }
}

macro_rules! ticket_columns {
    () => {
        "id, profile_id, rating, mode, frozen_deck, portrait, deck_cosmetics, frozen_trio, lean_set,
  catalog_version, status, enqueued_at, match_id"
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
        deck_slots: deck_slots_or_none(row.deck_cosmetics)?,
        trio: trio_or_null(row.frozen_trio.as_ref())?,
        // R1372, R924000: migration 0036's column (0027's boolean, `true` read as "newest").
        lean: lean_or_none(row.lean_set.as_deref())?,
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
        // `results_reason_check` (0004) pins this column to the engine's `GameOverReason` strings, so the
        // parse restates a database constraint.
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
    room_deck_cosmetics: Option<Value>,
    room_lean_set: Option<String>,
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
            room_deck_cosmetics: get(row, "room_deck_cosmetics")?,
            room_lean_set: get(row, "room_lean_set")?,
            catalog_version: get(row, "catalog_version")?,
            created_at: get(row, "created_at")?,
            ceiling_at: get(row, "ceiling_at")?,
        })
    }
}

macro_rules! room_columns {
    () => {
        "id, room_code, room_mode, room_trio, p1_profile_id, p2_profile_id, p1_deck,
  p1_portrait, room_deck_cosmetics, room_lean_set, catalog_version, created_at, ceiling_at"
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
        // R264: a room with no mode could only ever have been Best-of-1 (KNOWN DIVERGENCES, rooms).
        mode: match row.room_mode.as_deref() {
            None => from_literal("bo1")?,
            Some(mode) => queue_mode_of(mode)?,
        },
        host_deck: card_list_of(&row.p1_deck)?,
        host_portrait: Some(row.p1_portrait),
        host_deck_slots: deck_slots_or_none(row.room_deck_cosmetics)?,
        host_trio: trio_or_null(row.room_trio.as_ref())?,
        host_lean: lean_or_none(row.room_lean_set.as_deref())?,
        catalog_version: row.catalog_version,
        created_at: ms_of(row.created_at),
        // An unclaimed room keeps its joinable-until instant in `ceiling_at` (KNOWN DIVERGENCES, rooms).
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
    table_cosmetic: Option<String>,
    field_cosmetic: Option<String>,
    card_back_cosmetic: Option<String>,
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
            table_cosmetic: get(row, "table_cosmetic")?,
            field_cosmetic: get(row, "field_cosmetic")?,
            card_back_cosmetic: get(row, "card_back_cosmetic")?,
            catalog_version: get(row, "catalog_version")?,
            created_at: get(row, "created_at")?,
            updated_at: get(row, "updated_at")?,
        })
    }
}

macro_rules! deck_columns {
    () => {
        "id, profile_id, name, cards, portrait, table_cosmetic, field_cosmetic, card_back_cosmetic,
        catalog_version, created_at, updated_at"
    };
}

fn to_deck(row: DeckRow) -> Result<SavedDeck, StoreError> {
    Ok(SavedDeck {
        id: row.id,
        profile_id: row.profile_id,
        name: row.name,
        // A jsonb array keeps the order it was written in, so this is the player's order (R250).
        cards: card_list_of(&row.cards)?,
        // R641: `null` is the default, `vanilla`.
        portrait: row.portrait,
        // R1297000: `null` is "the account's choice, then the default".
        table_cosmetic: row.table_cosmetic,
        field_cosmetic: row.field_cosmetic,
        card_back_cosmetic: row.card_back_cosmetic,
        catalog_version: row.catalog_version,
        created_at: ms_of(row.created_at),
        updated_at: ms_of(row.updated_at),
    })
}

/// R2100000: a deck version's columns (migration 0066), as `deck_version_of` reads them.
macro_rules! deck_version_columns {
    () => {
        "id, profile_id, deck_id, name, cards, saved_at"
    };
}

fn deck_version_of(row: &PgRow) -> Result<DeckVersion, StoreError> {
    Ok(DeckVersion {
        id: uuid_text(row, "id")?,
        deck_id: uuid_text(row, "deck_id")?,
        profile_id: uuid_text(row, "profile_id")?,
        name: get(row, "name")?,
        // In the order the save wrote them, as a deck's (R250).
        cards: card_list_of(&get::<Value>(row, "cards")?)?,
        saved_at: ms_of(get(row, "saved_at")?),
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

/// Exactly what `app.upsert_deck` returns (migration 0007).
const UPSERT_OUTCOMES: &[&str] = &["created", "updated", "limit", "not_owner"];
const TRIO_UPSERT_OUTCOMES: &[&str] = &["created", "updated", "limit", "not_owner", "unknown_deck"];

fn upsert_outcome_of<T: DeserializeOwned>(
    function: &str,
    allowed: &[&str],
    value: Option<String>,
) -> Result<T, StoreError> {
    outcome_of(function, "0007", allowed, value)
}

/// A write function's answer read back as the port's outcome: one of `allowed`, which is exactly
/// what the function returns in `migration`; anything else is a schema this was not built against.
fn outcome_of<T: DeserializeOwned>(
    function: &str,
    migration: &str,
    allowed: &[&str],
    value: Option<String>,
) -> Result<T, StoreError> {
    if let Some(text) = value.as_deref()
        && allowed.contains(&text)
    {
        return from_literal(text);
    }
    Err(StoreError::from(format!(
        "{function} returned {}, which is not one of {} (migration {migration})",
        stringify(&value),
        allowed.join(", ")
    )))
}

/// Exactly what `app.create_custom_card` and `app.update_custom_card` return (migration 0037).
const CUSTOM_CARD_CREATE_OUTCOMES: &[&str] = &["created", "limit"];
const CUSTOM_CARD_UPDATE_OUTCOMES: &[&str] = &["updated", "stale", "missing"];

/// `public.custom_cards`' columns (migration 0037, R915008).
macro_rules! custom_card_columns {
    () => {
        "id, profile_id, version, frame, source, builder, created_at, updated_at"
    };
}

/// One `public.custom_cards` row as the port's card. The frame is read straight into the engine's
/// `CustomFrame`, so a row some other writer made that is not one fails here, by name.
fn to_custom_card(row: &PgRow) -> Result<SavedCustomCard, StoreError> {
    Ok(SavedCustomCard {
        profile_id: uuid_text(row, "profile_id")?,
        card: CustomCard {
            id: get(row, "id")?,
            version: get(row, "version")?,
            frame: json_col(row, "frame")?,
            source: get(row, "source")?,
            builder: get(row, "builder")?,
        },
        created_at: ms_of(get(row, "created_at")?),
        updated_at: ms_of(get(row, "updated_at")?),
    })
}

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

macro_rules! tutorial_columns {
    () => {
        "profile_id, completed, hidden, hidden_at"
    };
}

fn to_tutorial(row: TutorialDbRow) -> TutorialProgressRow {
    let at = ms_or_null(row.hidden_at);
    TutorialProgressRow {
        profile_id: row.profile_id,
        // The function keeps it sorted and each id once.
        completed: row.completed,
        // `tutorial_progress_choice_pair_check`: both or neither.
        hidden_choice: match (row.hidden, at) {
            (Some(hidden), Some(at)) => Some(TutorialHiddenChoice { hidden, at }),
            _ => None,
        },
    }
}

/// `public.player_settings` (migration 0018, R633): a jsonb object of groups, read in Postgres's print order.
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

struct PlayerStatsDbRow {
    profile_id: String,
    stats: IndexMap<String, Value>,
    is_private: bool,
    updated_at: OffsetDateTime,
}

/// A `stats` column as an object, or `{}` when it is anything else.
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

/// `public.series` (migration 0009) keeps as columns what is queried or constrained, and everything a series
/// only ever reads back whole in `state`. The players live in the columns alone, so `state.sides` carries
/// each side minus its `profileId` and the two cannot drift apart.
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
        // R604: absent when false, as `SeriesRow` types it; `series.rs` reads a missing flag as unranked, so
        // it never rates.
        ranked: if row.ranked { Some(true) } else { None },
        id: row.id,
    })
}

// The ranked ladder's rows (SPEC §9.12, migration 0022)

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
    ai_generation: Option<i32>,
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
            ai_generation: get(row, "ai_generation")?,
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
        ai_generation: row.ai_generation.map(i64::from),
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

macro_rules! rated_game_columns {
    () => {
        "id, kind, season_id, patch_version, catalog_version,
  p1_profile_id, p1_bot_id, p1_pilot, p1_before, p1_after, p1_rank_before, p1_rank_after,
  p2_profile_id, p2_bot_id, p2_pilot, p2_before, p2_after, p2_rank_before, p2_rank_after,
  winner_side, reason, ended_at"
    };
}

/// Advisory lock id for `ranked_lock_seasons`, transaction-scoped. Distinct from migrate.rs's
/// "jack" id so a deploy and a season open never wait on each other.
const SEASON_LOCK_ID: i64 = 0x7365_6173; // "seas"

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
        // `rated_games_reason_check` (0022) pins the column to the two reason sets, so the parse restates it.
        reason: from_literal(&row.reason)?,
        ended_at: ms_of(row.ended_at),
    })
}

/// Maximum pooled connections when the caller names none; small, since every call is one short transaction.
const POOL_MAX_DEFAULT: u32 = 10;

/// Supabase's Supavisor closes a client idle on its side and the driver learns it only on use, as
/// `read ETIMEDOUT` on a request that did nothing wrong; recycling an idle connection after 10 s retires it
/// first.
const POOL_IDLE_TIMEOUT_MS: u64 = 10_000;

/// Fail a checkout that cannot get a connection rather than hanging the request forever.
const POOL_ACQUIRE_TIMEOUT_MS: u64 = 10_000;

#[derive(Clone, Debug)]
pub struct PostgresStoreOptions {
    /// A direct Postgres connection string (`DATABASE_URL`; SPEC §9.2's Postgres edge).
    pub connection_string: String,
    /// Maximum pooled connections, `POOL_MAX_DEFAULT` when `None`.
    pub max: Option<u32>,
}

/// Exactly the strings `app.redeem_invite_code` returns (migration 0001 §6) — defined by the port,
/// because both stores answer with them.
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

/// The pool `Db::Pg` holds. The connection string is checked here, not on first use, so a misconfigured
/// deployment fails at boot naming the variable and end-to-end mode's
/// `DATABASE_URL=memory://e2e-fixture-store` placeholder can never be mistaken for a database. The pool
/// connects lazily.
pub fn create_postgres_store(options: &PostgresStoreOptions) -> Result<PgPool, StoreError> {
    assert_postgres_url(&options.connection_string)?;
    PgPoolOptions::new()
        .max_connections(options.max.unwrap_or(POOL_MAX_DEFAULT))
        .idle_timeout(Duration::from_millis(POOL_IDLE_TIMEOUT_MS))
        .acquire_timeout(Duration::from_millis(POOL_ACQUIRE_TIMEOUT_MS))
        .connect_lazy(&options.connection_string)
        .map_err(db_error)
}

pub async fn close(pool: &PgPool) {
    pool.close().await;
}

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

// Redemption (SPEC §9.4): the whole six-step transaction is one `app.*` call.

/// `app.redeem_invite_code` (migration 0001 §6) holds all six steps under one profile row lock and one code
/// row lock, and never raises for an expected rejection. A `None` `code_hash` is bound as SQL NULL on
/// purpose: `where code_hash = null` matches no row, so a code that could never exist is refused by the same
/// lookup as one never minted, after the attempt is logged.
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

/// The retention purge, as one call to `app.purge_expired_rows` (migration 0013, with 0029's third
/// cutoff for the play telemetry, R1442, and 0041's fourth for the game records' detail, R1044001):
/// the only path `match_actions`' append-only guard lets a delete through.
pub async fn purge_expired(
    t: &mut PgTx<'_>,
    input: &RetentionPurgeInput,
) -> Result<RetentionPurgeResult, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select code_attempts, match_actions, play_telemetry, game_records from app.purge_expired_rows(",
        ts!("$1"),
        ", ",
        ts!("$2"),
        ", ",
        ts!("$3"),
        ", ",
        ts!("$4"),
        ")"
    ))
    .bind(input.code_attempts_before)
    .bind(input.match_actions_ended_before)
    .bind(input.play_telemetry_ended_before)
    .bind(input.game_records_recorded_before)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let Some(row) = row else {
        return Err(StoreError::from("app.purge_expired_rows returned no row"));
    };
    Ok(RetentionPurgeResult {
        code_attempts: get::<i64>(&row, "code_attempts")?,
        match_actions: get::<i64>(&row, "match_actions")?,
        play_telemetry: get::<i64>(&row, "play_telemetry")?,
        game_records: get::<i64>(&row, "game_records")?,
    })
}

// Profiles (SPEC §9.4)

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

/// The same lookup as `profiles_get_by_id`: the user id is the profile id (see `to_profile`).
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
    // A deleted seat reads back as the empty id (KNOWN DIVERGENCES, deleted accounts); an id that is not a
    // uuid names no profile and Postgres would refuse it, so it is left out.
    let ids: Vec<String> = profile_ids.iter().filter(|id| is_uuid(id)).cloned().collect();
    if ids.is_empty() {
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
    .bind(ids)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| ProfileRow::read(row).and_then(to_profile))
        .collect()
}

/// R1443: read on migration 0028's unique index, `(username_key, coalesce(username_tag, 0))`. A tag
/// too large for `profiles.username_tag` names nobody.
pub async fn profiles_get_by_username(
    t: &mut PgTx<'_>,
    key: &str,
    tag: Option<i64>,
) -> Result<Option<Profile>, StoreError> {
    let Ok(tag) = i32::try_from(tag.unwrap_or(0)) else {
        return Ok(None);
    };
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select ",
        profile_columns!(),
        " ",
        profile_from!(),
        " where p.username_key = $1::text and coalesce(p.username_tag, 0) = $2::int"
    ))
    .bind(key)
    .bind(tag)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| ProfileRow::read(&row).and_then(to_profile))
        .transpose()
}

/// §9.4: "the account exists the moment auth says so and stays pending until a code is redeemed"
/// (`resolve_caller` in http.rs). Migration 0001's `on_auth_user_created` trigger has usually made the row
/// already; this insert is the path for a project whose trigger has not run. `email` is ignored and the
/// `auth.users` row must exist (KNOWN DIVERGENCES, profile email).
pub async fn profiles_create(t: &mut PgTx<'_>, input: &ProfileCreateInput) -> Result<Profile, StoreError> {
    let user_id = input.user_id.as_str();
    let (rating, at) = (input.rating, input.at);
    run_as(t, Some(user_id)).await?;
    // R1434: no username is written here: migration 0028's `profiles_assign_default_username` trigger names
    // every new row the lowest free `Player#n`, under the same lock a claim of `Player` takes.
    sqlx::query(concat!(
        "insert into public.profiles (id, status, rating, created_at)
           values ($1::uuid, 'pending', $2::double precision, ",
        ts!("$3"),
        ")"
    ))
    .bind(user_id)
    .bind(rating)
    .bind(at)
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

/// R111 rides on this write: migration 0002's `profiles_grant_launch_collection` trigger on the
/// `pending -> active` transition grants one copy of every non-token card in the current catalog version
/// through `app.grant_cards` (idempotent, one transaction). Nothing here grants anything.
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

/// R1434: one lock per username key, transaction-scoped like `ranked_lock_seasons`, so two claims of one key
/// run one after the other. Migration 0028's `app.assign_default_username` takes the same lock on `player`,
/// so a claim of `Player` and a sign-up never hand out one tag twice; `seed-accounts` takes it too
/// (`cli/seed_accounts.rs`).
pub const USERNAME_LOCK_SQL: &str =
    "select pg_advisory_xact_lock(hashtextextended('jackioh.username:' || $1::text, 0))";

/// R1434's tag for a claim of key `$2` by profile `$1`: null while no other profile holds the bare name, else
/// 1 when nobody else holds `#1`, else the lowest free tag. Every lookup is on the unique index's own
/// expression, `(username_key, coalesce(username_tag, 0))`.
const USERNAME_TAG_FOR_SQL: &str = "select case
  when not exists (select 1 from public.profiles
                    where username_key = $2::text and coalesce(username_tag, 0) = 0 and id <> $1::uuid)
    then null
  when not exists (select 1 from public.profiles
                    where username_key = $2::text and coalesce(username_tag, 0) = 1 and id <> $1::uuid)
    then 1::bigint
  else (select min(held.username_tag)::bigint + 1 from public.profiles held
         where held.username_key = $2::text and coalesce(held.username_tag, 0) >= 1 and held.id <> $1::uuid
           and not exists (select 1 from public.profiles after_held
                            where after_held.username_key = $2::text
                              and coalesce(after_held.username_tag, 0) = held.username_tag + 1
                              and after_held.id <> $1::uuid))
end as tag";

pub async fn profiles_username_tag_for(
    t: &mut PgTx<'_>,
    profile_id: &str,
    key: &str,
) -> Result<Option<i64>, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let row = sqlx::query(USERNAME_TAG_FOR_SQL)
        .bind(profile_id)
        .bind(key)
        .fetch_one(&mut **t)
        .await
        .map_err(db_error)?;
    get(&row, "tag")
}

pub async fn profiles_claim_username(
    t: &mut PgTx<'_>,
    claim: &UsernameClaim,
) -> Result<UsernameClaimOutcome, StoreError> {
    let profile_id = claim.profile_id.as_str();
    run_as(t, Some(profile_id)).await?;
    sqlx::query(USERNAME_LOCK_SQL)
        .bind(&claim.key)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    let row = sqlx::query("select username_changed_at from public.profiles where id = $1::uuid for update")
        .bind(profile_id)
        .fetch_optional(&mut **t)
        .await
        .map_err(db_error)?;
    let Some(row) = row else {
        return Err(StoreError::from(format!("no profile {profile_id}")));
    };
    let changed_at: Option<OffsetDateTime> = get(&row, "username_changed_at")?;
    if let Some(changed_at) = changed_at.map(ms_of) {
        let next_change_at = changed_at + claim.cooldown_ms;
        if next_change_at > claim.at {
            return Ok(UsernameClaimOutcome::Cooldown { next_change_at });
        }
    }
    let tag = profiles_username_tag_for(t, profile_id, &claim.key).await?;
    if tag != claim.expected_tag {
        return Ok(UsernameClaimOutcome::Changed { tag });
    }
    let tag_column = tag.map(i32::try_from).transpose().map_err(|error| {
        StoreError::from(format!(
            "username tag {tag:?} does not fit profiles.username_tag: {error}"
        ))
    })?;
    sqlx::query(concat!(
        "update public.profiles
            set username_base = $2::text, username_key = $3::text, username_tag = $4::int,
                username_changed_at = ",
        ts!("$5"),
        ", username_prompted = true
          where id = $1::uuid"
    ))
    .bind(profile_id)
    .bind(&claim.base)
    .bind(&claim.key)
    .bind(tag_column)
    .bind(claim.at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(UsernameClaimOutcome::Claimed { tag })
}

pub async fn profiles_answer_username_prompt(t: &mut PgTx<'_>, profile_id: &str) -> Result<(), StoreError> {
    run_as(t, Some(profile_id)).await?;
    let done = sqlx::query("update public.profiles set username_prompted = true where id = $1::uuid")
        .bind(profile_id)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    if done.rows_affected() == 0 {
        return Err(StoreError::from(format!("no profile {profile_id}")));
    }
    Ok(())
}

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

/// R890000: one statement seats only a free profile, so of the writers racing for one seat exactly
/// one wins.
pub async fn profiles_set_in_match(
    t: &mut PgTx<'_>,
    profile_id: &str,
    match_id: Option<&str>,
) -> Result<bool, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let done = sqlx::query(
        "update public.profiles set current_match_id = $2::uuid
          where id = $1::uuid and ($2::uuid is null or current_match_id is null)",
    )
    .bind(profile_id)
    .bind(match_id)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    if done.rows_affected() == 1 {
        return Ok(true);
    }
    // No row moved: a profile that was never there is an error, as `matches_finish` re-reads its id; a
    // profile that is there is held by another match.
    let rows = sqlx::query("select id from public.profiles where id = $1::uuid")
        .bind(profile_id)
        .fetch_all(&mut **t)
        .await
        .map_err(db_error)?;
    if rows.is_empty() {
        return Err(StoreError::from(format!("no profile {profile_id}")));
    }
    Ok(false)
}

/// R890000, R1896000: clears the seat only while it points at `match_id`.
pub async fn profiles_clear_in_match(
    t: &mut PgTx<'_>,
    profile_id: &str,
    match_id: &str,
) -> Result<(), StoreError> {
    run_as(t, Some(profile_id)).await?;
    sqlx::query(
        "update public.profiles set current_match_id = null
          where id = $1::uuid and current_match_id = $2::uuid",
    )
    .bind(profile_id)
    .bind(match_id)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub async fn profiles_seated(t: &mut PgTx<'_>) -> Result<Vec<SeatedProfile>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select id, current_match_id from public.profiles
          where current_match_id is not null order by id",
    )
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| {
            Ok(SeatedProfile {
                profile_id: uuid_text(row, "id")?,
                match_id: uuid_text(row, "current_match_id")?,
            })
        })
        .collect()
}

// The seat ledger's epoch (R1523000, migration 0054)

/// Shared locks never wait on each other, so claims run side by side; a newer process's bump waits
/// for every claim that took one, and a claim that reads after the bump finds no row.
pub async fn seats_fence(t: &mut PgTx<'_>, epoch: i64) -> Result<(), StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query("select 1 from app.seat_epoch where epoch = $1 for share")
        .bind(epoch)
        .fetch_optional(&mut **t)
        .await
        .map_err(db_error)?;
    match row {
        Some(_) => Ok(()),
        None => Err(StoreError::Fenced),
    }
}

pub async fn seats_bump_epoch(t: &mut PgTx<'_>) -> Result<i64, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query("update app.seat_epoch set epoch = epoch + 1 returning epoch")
        .fetch_one(&mut **t)
        .await
        .map_err(db_error)?;
    get::<i64>(&row, "epoch")
}

/// `DELETE /api/account`'s store half: one delete, and migration 0012's foreign keys do the rest;
/// `profiles_release_open_matches` removes a room it opened that nobody joined. A profile still seated in a
/// live match, an unfinished series or a live co-op match (migration 0051, R1497000) makes the delete raise,
/// by constraint; `src/api/auth.rs` refuses that case first. No `app.*` function: the schema is the rule.
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

// Invite codes (SPEC §9.4)

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

pub async fn codes_revoke_by_hash(t: &mut PgTx<'_>, code_hash: &str) -> Result<bool, StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query(
        "update public.invite_codes set revoked_at = now() where code_hash = $1::text and revoked_at is null",
    )
    .bind(code_hash)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(done.rows_affected() == 1)
}

/// `CodeAttempt.reason` has no column: see KNOWN DIVERGENCES (attempt reason).
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

/// §9.8: `count(distinct profile_id)` skips the null profile of an attempt nobody was signed in
/// for, as the fake does.
pub async fn codes_invite_clusters(t: &mut PgTx<'_>, since: i64) -> Result<Vec<InviteCluster>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(concat!(
        "select ip_hash, count(*)::int as attempts, count(distinct profile_id)::int as accounts
           from public.code_attempts
          where at >= ",
        ts!("$1"),
        "
          group by ip_hash
          order by ip_hash collate \"C\""
    ))
    .bind(since)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| {
            Ok(InviteCluster {
                ip_hash: get(row, "ip_hash")?,
                accounts: get::<i32>(row, "accounts").map(i64::from)?,
                attempts: get::<i32>(row, "attempts").map(i64::from)?,
            })
        })
        .collect()
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

/// A missing `app.settings` row is a store error: migration 0001 seeds it, so its absence means
/// the database was never migrated.
pub async fn redemption_set_enabled(t: &mut PgTx<'_>, enabled: bool) -> Result<bool, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query("select value from app.settings where key = 'redemption_enabled'")
        .fetch_optional(&mut **t)
        .await
        .map_err(db_error)?;
    let Some(row) = row else {
        return Err(StoreError::from("app.settings has no redemption_enabled row"));
    };
    let previous: bool = json_col(&row, "value")?;
    sqlx::query("update app.settings set value = to_jsonb($1::boolean) where key = 'redemption_enabled'")
        .bind(enabled)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(previous)
}

// Collection (SPEC §9.4's entitlement ledger)

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
            Ok(CollectionEntry {
                card_id: get(row, "card_id")?,
                quantity: i64::from(get::<i32>(row, "quantity")?),
            })
        })
        .collect()
}

/// Not `app.grant_cards`, which takes deltas and writes both ledger tables itself: `grant_cards` in
/// `src/api/collection.rs` has already added its delta to the total read in this transaction, so this sets
/// `quantity = excluded.quantity`; adding again would double every grant. The port splits the ledger's two
/// writes and puts them in one transaction, the same §9.4 guarantee reached the other way round.
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
                "dev": grant.dev,
                "at": grant.at,
            })
        })
        .collect();
    run_as(t, Some(first.profile_id.as_str())).await?;
    sqlx::query(concat!(
        "insert into public.collection_grants (profile_id, card_id, delta, reason, dev, at)
         select g.profile_id::uuid, g.card_id, g.delta, g.reason, g.dev, ",
        ts!("g.at"),
        "
           from jsonb_to_recordset($1::jsonb)
             as g(profile_id text, card_id text, delta int, reason text, dev boolean, at double precision)"
    ))
    .bind(json(&rows)?)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

// Saved decks (SPEC §9.4, R250, R256)

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

/// R250, R256: one call to `app.upsert_deck`, which takes the profile row lock, re-checks the draft's shape
/// (D1, D2, D4), refuses another profile's id and counts the cap under the lock; its answer is the port's
/// `UpsertOutcome`. Its one instant `p_at` gets `updated_at` (KNOWN DIVERGENCES, deck and trio timestamps);
/// the cap is the smaller of the caller's and `app.settings.max_saved_decks` (KNOWN DIVERGENCES, caps).
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
           $8::text, $9::text, $10::text, $11::text)
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
    .bind(deck.table_cosmetic.as_deref())
    .bind(deck.field_cosmetic.as_deref())
    .bind(deck.card_back_cosmetic.as_deref())
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let outcome = match row {
        None => None,
        Some(row) => get::<Option<String>>(&row, "outcome")?,
    };
    upsert_outcome_of("app.upsert_deck", UPSERT_OUTCOMES, outcome)
}

/// Not an `app.*` function: a delete of the profile's own row has nothing to decide, and R252's "deleting a
/// deck empties every slot that named it" is the database's own behavior: the three
/// `on delete set null (deckN_id)` foreign keys on `public.trios` empty the slots in this same statement.
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

// A deck's earlier versions (R2100000, migration 0066)
//
// Not `app.*` functions: what a save keeps is `deck_version_step`'s, in Rust, so both stores decide it by
// the same code. Every statement names the profile and the deck; the deck's row lock orders two saves.

/// Newest first, ties on id descending: the order `deck_versions_deck_saved_idx` holds.
pub async fn deck_versions_list(
    t: &mut PgTx<'_>,
    profile_id: &str,
    deck_id: &str,
) -> Result<Vec<DeckVersion>, StoreError> {
    if !is_uuid(deck_id) {
        return Ok(Vec::new());
    }
    run_as(t, Some(profile_id)).await?;
    let rows = sqlx::query(concat!(
        "select ",
        deck_version_columns!(),
        " from public.deck_versions where profile_id = $1::uuid and deck_id = $2::uuid
          order by saved_at desc, id desc"
    ))
    .bind(profile_id)
    .bind(deck_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(deck_version_of).collect()
}

pub async fn deck_versions_keep(
    t: &mut PgTx<'_>,
    save: &DeckVersion,
    debounce_ms: Option<i64>,
    max_versions: i64,
) -> Result<DeckVersionOutcome, StoreError> {
    if !is_uuid(&save.deck_id) {
        return Ok(DeckVersionOutcome::Missing);
    }
    run_as(t, Some(save.profile_id.as_str())).await?;
    // A second save of this deck waits here until this transaction ends, so the newest version it
    // reads is the one this save left.
    let deck =
        sqlx::query("select 1 from public.decks where id = $1::uuid and profile_id = $2::uuid for update")
            .bind(save.deck_id.as_str())
            .bind(save.profile_id.as_str())
            .fetch_optional(&mut **t)
            .await
            .map_err(db_error)?;
    if deck.is_none() {
        return Ok(DeckVersionOutcome::Missing);
    }
    let newest = sqlx::query(concat!(
        "select ",
        deck_version_columns!(),
        " from public.deck_versions where profile_id = $1::uuid and deck_id = $2::uuid
          order by saved_at desc, id desc limit 1"
    ))
    .bind(save.profile_id.as_str())
    .bind(save.deck_id.as_str())
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?
    .map(|row| deck_version_of(&row))
    .transpose()?;
    let outcome = deck_version_step(newest.as_ref(), save, debounce_ms);
    match (outcome, newest) {
        (DeckVersionOutcome::Merged, Some(newest)) => {
            sqlx::query(concat!(
                "update public.deck_versions set name = $2::text, cards = $3::jsonb, saved_at = ",
                ts!("$4"),
                " where id = $1::uuid"
            ))
            .bind(newest.id.as_str())
            .bind(save.name.as_str())
            .bind(json(&save.cards)?)
            .bind(save.saved_at)
            .execute(&mut **t)
            .await
            .map_err(db_error)?;
        }
        (DeckVersionOutcome::Added, _) => {
            sqlx::query(concat!(
                "insert into public.deck_versions (id, profile_id, deck_id, name, cards, saved_at)
                 values ($1::uuid, $2::uuid, $3::uuid, $4::text, $5::jsonb, ",
                ts!("$6"),
                ")"
            ))
            .bind(save.id.as_str())
            .bind(save.profile_id.as_str())
            .bind(save.deck_id.as_str())
            .bind(save.name.as_str())
            .bind(json(&save.cards)?)
            .bind(save.saved_at)
            .execute(&mut **t)
            .await
            .map_err(db_error)?;
            sqlx::query(
                "delete from public.deck_versions where id in (
                   select id from public.deck_versions where profile_id = $1::uuid and deck_id = $2::uuid
                    order by saved_at desc, id desc offset $3::bigint)",
            )
            .bind(save.profile_id.as_str())
            .bind(save.deck_id.as_str())
            .bind(max_versions.max(0))
            .execute(&mut **t)
            .await
            .map_err(db_error)?;
        }
        _ => {}
    }
    Ok(outcome)
}

// Saved trios (SPEC §9.4, R252, R256)

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

/// R252, R256: one call to `app.upsert_trio`, as `decks_upsert`. A deck in two slots raises
/// `trios_decks_distinct` (R252 T3). A slot that is not a uuid cannot name any deck; it is answered
/// here, as `unknown_deck`, because Postgres would refuse the cast before the function could say so.
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

// Custom cards on the account (SPEC §12.9, R915008, migration 0037)
//
// A create and a save are one call each to `app.create_custom_card` and `app.update_custom_card`, which count
// the cap under the profile lock and the version under the card's; the rest are single statements.

/// Oldest first, ties on the id's bytes (`collate "C"`, so a collation that skips hyphens cannot
/// order them otherwise than the in-memory store does).
pub async fn custom_cards_list(
    t: &mut PgTx<'_>,
    profile_id: &str,
) -> Result<Vec<SavedCustomCard>, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let rows = sqlx::query(concat!(
        "select ",
        custom_card_columns!(),
        " from public.custom_cards where profile_id = $1::uuid order by created_at, id collate \"C\""
    ))
    .bind(profile_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(to_custom_card).collect()
}

pub async fn custom_cards_get(
    t: &mut PgTx<'_>,
    profile_id: &str,
    card_id: &str,
) -> Result<Option<SavedCustomCard>, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let row = sqlx::query(concat!(
        "select ",
        custom_card_columns!(),
        " from public.custom_cards where profile_id = $1::uuid and id = $2::text"
    ))
    .bind(profile_id)
    .bind(card_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.as_ref().map(to_custom_card).transpose()
}

/// The cap is the smaller of `max_cards` and `app.settings.max_custom_cards` (KNOWN DIVERGENCES,
/// caps).
pub async fn custom_cards_create(
    t: &mut PgTx<'_>,
    card: &CustomCardWrite,
    max_cards: i64,
) -> Result<CustomCardCreateOutcome, StoreError> {
    let max_cards = i32::try_from(max_cards).unwrap_or(i32::MAX);
    run_as(t, Some(card.profile_id.as_str())).await?;
    let row = sqlx::query(concat!(
        "select app.create_custom_card($1::uuid, $2::text, $3::jsonb, $4::text, $5::jsonb, ",
        ts!("$6"),
        ", $7::int) as outcome"
    ))
    .bind(card.profile_id.as_str())
    .bind(card.id.as_str())
    .bind(json(&card.frame)?)
    .bind(card.source.as_str())
    .bind(json_or_null(&card.builder)?)
    .bind(card.at)
    .bind(max_cards)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let outcome = match row {
        None => None,
        Some(row) => get::<Option<String>>(&row, "outcome")?,
    };
    outcome_of(
        "app.create_custom_card",
        "0037",
        CUSTOM_CARD_CREATE_OUTCOMES,
        outcome,
    )
}

pub async fn custom_cards_update(
    t: &mut PgTx<'_>,
    card: &CustomCardWrite,
    replaces: i32,
) -> Result<CustomCardUpdateOutcome, StoreError> {
    run_as(t, Some(card.profile_id.as_str())).await?;
    let row = sqlx::query(concat!(
        "select app.update_custom_card($1::uuid, $2::text, $3::int, $4::jsonb, $5::text, $6::jsonb, ",
        ts!("$7"),
        ") as outcome"
    ))
    .bind(card.profile_id.as_str())
    .bind(card.id.as_str())
    .bind(replaces)
    .bind(json(&card.frame)?)
    .bind(card.source.as_str())
    .bind(json_or_null(&card.builder)?)
    .bind(card.at)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    let outcome = match row {
        None => None,
        Some(row) => get::<Option<String>>(&row, "outcome")?,
    };
    outcome_of(
        "app.update_custom_card",
        "0037",
        CUSTOM_CARD_UPDATE_OUTCOMES,
        outcome,
    )
}

/// Not an `app.*` function, as `decks_remove`: a delete of the profile's own row has nothing to
/// decide. A deck naming the card keeps naming it (§12.9).
pub async fn custom_cards_remove(
    t: &mut PgTx<'_>,
    profile_id: &str,
    card_id: &str,
) -> Result<bool, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let done = sqlx::query("delete from public.custom_cards where id = $1::text and profile_id = $2::uuid")
        .bind(card_id)
        .bind(profile_id)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(done.rows_affected() == 1)
}

// Matches (SPEC §9.3's log, §9.5's lifecycle)

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

/// An insert OR a completion: for both entry points §9.5 has (a paired queue match and a claimed room)
/// `tickets_claim_pair` / `rooms_claim` have already written an `open` row under the id the caller minted
/// (`tickets.match_id` and `profiles.current_match_id` are foreign keys into this table). Any other row
/// under `id` means the id is taken, an error as in the fake; the `for update` makes read-then-write safe.
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
               p1_portrait, p2_portrait, mode, stake, ranked, started_at, last_seq, p1_glitch_board, p2_glitch_board,
               loadouts)
             values ($1::uuid, ",
            match_values!(),
            ", $21::boolean, ",
            ts!("$15"),
            ", 0, $24::jsonb, $25::jsonb, $26::jsonb)"
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
             p1_glitch_board = $24::jsonb, p2_glitch_board = $25::jsonb, loadouts = $26::jsonb
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
        // R604: false for a room and for the `open` skeletons this UPDATE turns live; a queue skeleton's own
        // write stamps the flag.
        .bind(m.ranked.unwrap_or(false))
        .bind(m.mode.as_ref().map(literal).transpose()?)
        .bind(int_or_null(&m.stake)?)
        // R678: the Glitch boards sampled at creation (migration 0024), '[]' when absent.
        .bind(json(
            m.glitch_boards.as_ref().map_or(&no_board, |boards| &boards.0),
        )?)
        .bind(json(
            m.glitch_boards.as_ref().map_or(&no_board, |boards| &boards.1),
        )?)
        .bind(json_or_null(&m.loadouts)?)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(())
}

/// R1834006: one row per person's seat of a match that froze loadouts, read off `matches.loadouts`
/// by the seat's index; a bot's seat and a deleted account's (a null profile) join no person.
pub async fn matches_loadout_samples(t: &mut PgTx<'_>, since: i64) -> Result<Vec<LoadoutSample>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(concat!(
        "select p.id as profile_id, p.created_at as account_created_at, m.created_at as match_created_at,
                case s.seat when 0 then m.p1_deck else m.p2_deck end as deck, m.loadouts -> s.seat as loadout
           from public.matches m
           cross join lateral (values (0, m.p1_profile_id), (1, m.p2_profile_id)) as s(seat, profile_id)
           join public.profiles p on p.id = s.profile_id and p.kind = 'human'
          where m.loadouts is not null and m.status <> 'open' and m.created_at >= ",
        ts!("$1"),
        "
          order by m.created_at, m.id, s.seat"
    ))
    .bind(since)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| {
            Ok(LoadoutSample {
                profile_id: uuid_text(row, "profile_id")?,
                account_created_at: ms_of(get(row, "account_created_at")?),
                match_created_at: ms_of(get(row, "match_created_at")?),
                deck: card_list_of(&get::<Value>(row, "deck")?)?,
                loadout: json_col(row, "loadout")?,
            })
        })
        .collect()
}

pub async fn matches_get(t: &mut PgTx<'_>, match_id: &str) -> Result<Option<MatchRow>, StoreError> {
    if !is_uuid(match_id) {
        return Ok(None);
    }
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

/// §9.3's append-only log, written by `app.append_match_action` (migration 0004) and nothing else: it assigns
/// `seq` from `matches.last_seq` under a row lock and returns the original `seq` for a repeated nonce. The
/// port hands in the `seq` the actor already used, so a mismatch means the log and the actor have diverged:
/// it is raised as `StoreError::SeqClash` (R776000), which the actor answers by letting go of the match.
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
        // Raised INSIDE the transaction the function's own insert ran in, so the rollback takes that insert
        // with it.
        if assigned != row.seq {
            return Err(StoreError::SeqClash(format!(
                "match_actions is append-only: seq {} exists (app.append_match_action assigned {} for nonce {})",
                row.seq, assigned, action.nonce
            )));
        }
    }
    Ok(())
}

pub async fn matches_actions(t: &mut PgTx<'_>, match_id: &str) -> Result<Vec<MatchActionRow>, StoreError> {
    run_as(t, None).await?;
    // Ordered by the table's column, qualified: a bare `order by seq` would sort the text output column
    // (`seq::text`), 1, 10, 11, …, 2
    // (`store::contract`'s `reads_a_log_of_ten_or_more_actions_back_in_seq_order`).
    let rows = sqlx::query(
        "select a.seq::text as seq, a.action, a.at from public.match_actions a
          where a.match_id = $1::uuid order by a.seq",
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

pub async fn matches_has_actions(t: &mut PgTx<'_>, match_id: &str) -> Result<bool, StoreError> {
    if !is_uuid(match_id) {
        return Ok(false);
    }
    run_as(t, None).await?;
    let row =
        sqlx::query("select exists (select 1 from public.match_actions where match_id = $1::uuid) as logged")
            .bind(match_id)
            .fetch_one(&mut **t)
            .await
            .map_err(db_error)?;
    get(&row, "logged")
}

/// #1169: one statement, so the guards and the write cannot come apart.
pub async fn matches_put_checkpoint(t: &mut PgTx<'_>, row: &MatchCheckpointRow) -> Result<bool, StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query(concat!(
        "insert into public.match_checkpoints
           (match_id, seq, nonce, catalog_version, rules, checkpoint_version, body, body_hash, at)
         select $1::uuid, $2::bigint, $3::text, $4::text, $5::text, $6::integer, $7::text, $8::text, ",
        ts!("$9"),
        "
          where exists (select 1 from public.matches m where m.id = $1::uuid and m.status = 'live')
            and exists (select 1 from public.match_actions a
                         where a.match_id = $1::uuid and a.seq = $2::bigint and a.nonce = $3::text)
         on conflict (match_id) do update set
           seq = excluded.seq, nonce = excluded.nonce, catalog_version = excluded.catalog_version,
           rules = excluded.rules, checkpoint_version = excluded.checkpoint_version, body = excluded.body,
           body_hash = excluded.body_hash, at = excluded.at
          where public.match_checkpoints.seq < excluded.seq"
    ))
    .bind(row.match_id.as_str())
    .bind(row.seq)
    .bind(row.nonce.as_str())
    .bind(row.catalog_version.as_str())
    .bind(row.rules.as_str())
    .bind(row.checkpoint_version)
    .bind(row.body.as_str())
    .bind(row.body_hash.as_str())
    .bind(row.at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(done.rows_affected() > 0)
}

pub async fn matches_checkpoint(
    t: &mut PgTx<'_>,
    match_id: &str,
) -> Result<Option<MatchCheckpointRow>, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query(
        "select seq::text as seq, nonce, catalog_version, rules, checkpoint_version, body, body_hash, at
           from public.match_checkpoints where match_id = $1::uuid",
    )
    .bind(match_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map(|row| -> Result<MatchCheckpointRow, StoreError> {
        Ok(MatchCheckpointRow {
            match_id: match_id.to_string(),
            seq: int_of(&get::<String>(&row, "seq")?)?,
            nonce: get(&row, "nonce")?,
            catalog_version: get(&row, "catalog_version")?,
            rules: get(&row, "rules")?,
            checkpoint_version: get(&row, "checkpoint_version")?,
            body: get(&row, "body")?,
            body_hash: get(&row, "body_hash")?,
            at: ms_of(get(&row, "at")?),
        })
    })
    .transpose()
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

/// Not `app.end_match`, which also writes the results row, moves both ratings, clears both
/// `current_match_id`s and cancels stray tickets: `src/api/results.rs` already does each through
/// `results_insert`, `profiles_set_glicko`, `profiles_clear_in_match` and `tickets_cancel` inside
/// the one transaction this call runs in, so calling it here would do those four a second time.
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
    // #1169: a match that has ended is never rebuilt from its checkpoint, so it goes with the ending.
    sqlx::query("delete from public.match_checkpoints where match_id = $1::uuid")
        .bind(match_id)
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

/// R263: a reserved id is an `open` row, so releasing it deletes that row, only while it is still `open`
/// (KNOWN DIVERGENCES, reserved match ids). Nothing is left pointing at it: an `open` row has no actions or
/// result, `tickets.match_id` and `profiles.current_match_id` are `on delete set null` (0004), and
/// `series.next_match_id` has no foreign key (0009). R110's partial unique index frees a room's code at once.
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

/// R1450: one delete; `on delete set null` (0004) lets go of `profiles.current_match_id` and
/// `tickets.match_id`, and the room code is free again (R110). `before` is the server's clock less the grace,
/// the clock `rooms_create` and `tickets_claim_pair` stamped `ceiling_at` with.
pub async fn matches_reap_open(t: &mut PgTx<'_>, before: i64) -> Result<u64, StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query(concat!(
        "delete from public.matches where status = 'open' and ceiling_at < ",
        ts!("$1")
    ))
    .bind(before)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(done.rows_affected())
}

/// R768 (migration 0035): the `final_hash is null` guard is what makes it written once.
pub async fn matches_record_final_hash(
    t: &mut PgTx<'_>,
    match_id: &str,
    final_hash: &str,
) -> Result<(), StoreError> {
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
    let row =
        sqlx::query("select status, p1_profile_id, p2_profile_id from public.matches where id = $1::uuid")
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

// Replays (R768)

/// One finished match with a result, read for its replay. A deleted seat reads as the empty id (KNOWN
/// DIVERGENCES, deleted accounts), so a player keeps the replay when the opponent deleted their account.
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
    let row = MatchDbRow::read(&row)?;
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

/// R768: one query, the matches, their results and their action counts joined, with the mode
/// derived in Rust by the same precedence `matches_mode_of` uses.
pub async fn replays_list_with_modes(
    t: &mut PgTx<'_>,
    profile_id: &str,
    limit: i64,
    offset: i64,
) -> Result<Vec<ReplayWithMode>, StoreError> {
    if !is_uuid(profile_id) {
        return Ok(Vec::new());
    }
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select m.id, m.status, m.seed, m.p1_profile_id, m.p2_profile_id, m.p1_deck, m.p2_deck,
                m.catalog_version, m.turn_deadline_at, m.prompt_deadline_at, m.p1_disconnected_at,
                m.p2_disconnected_at, m.ceiling_at, m.created_at, m.ended_at, m.p1_last_board,
                m.p2_last_board, m.ranked, m.p1_portrait, m.p2_portrait, m.mode, m.stake,
                m.p1_glitch_board, m.p2_glitch_board, m.final_hash, m.loadouts,
                r.match_id as r_match_id, r.p1_profile_id as r_p1_profile_id,
                r.p2_profile_id as r_p2_profile_id, r.winner_profile_id as r_winner_profile_id,
                r.reason as r_reason, r.turns as r_turns,
                r.p1_rating_before as r_p1_rating_before, r.p1_rating_after as r_p1_rating_after,
                r.p2_rating_before as r_p2_rating_before, r.p2_rating_after as r_p2_rating_after,
                r.ended_at as r_ended_at,
                (select count(*) from public.match_actions a where a.match_id = m.id) as actions,
                m.room_code, m.room_mode,
                (select t.mode from public.tickets t where t.match_id = m.id
                  order by t.enqueued_at, t.id limit 1) as ticket_mode,
                exists (select 1 from public.series s where s.state -> 'games'
                  @> jsonb_build_array(jsonb_build_object('matchId', m.id::text))) as in_series
           from public.matches m
           join public.results r on r.match_id = m.id
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
        let shape = MatchDbRow::read(row)?;
        let replay = ReplayRow {
            row: to_match(shape)?,
            result: to_result(ResultDbRow {
                match_id: uuid_text(row, "r_match_id")?,
                p1_profile_id: uuid_text_or_null(row, "r_p1_profile_id")?,
                p2_profile_id: uuid_text_or_null(row, "r_p2_profile_id")?,
                winner_profile_id: uuid_text_or_null(row, "r_winner_profile_id")?,
                reason: get(row, "r_reason")?,
                turns: get(row, "r_turns")?,
                p1_rating_before: get(row, "r_p1_rating_before")?,
                p1_rating_after: get(row, "r_p1_rating_after")?,
                p2_rating_before: get(row, "r_p2_rating_before")?,
                p2_rating_after: get(row, "r_p2_rating_after")?,
                ended_at: get(row, "r_ended_at")?,
            })?,
            actions: get(row, "actions")?,
        };
        // R376, R672: the precedence `matches_mode_of` answers, in the same order.
        let own: Option<String> = get(row, "mode")?;
        let room_code: Option<String> = get(row, "room_code")?;
        let room_mode: Option<String> = get(row, "room_mode")?;
        let ticket_mode: Option<String> = get(row, "ticket_mode")?;
        let mode = if get::<bool>(row, "in_series")? {
            from_literal("bo3").map(Some)?
        } else if let Some(own) = own.as_deref() {
            queue_mode_of(own).map(Some)?
        } else if room_code.is_some() {
            match room_mode.as_deref() {
                None => from_literal("bo1").map(Some)?,
                Some(mode) => queue_mode_of(mode).map(Some)?,
            }
        } else {
            ticket_mode.as_deref().map(queue_mode_of).transpose()?
        };
        replays.push(ReplayWithMode { replay, mode });
    }
    Ok(replays)
}

pub async fn replays_get(t: &mut PgTx<'_>, match_id: &str) -> Result<Option<ReplayRow>, StoreError> {
    if !is_uuid(match_id) {
        return Ok(None);
    }
    run_as(t, None).await?;
    let counted =
        sqlx::query("select count(*) as actions from public.match_actions where match_id = $1::uuid")
            .bind(match_id)
            .fetch_one(&mut **t)
            .await
            .map_err(db_error)?;
    replay_row(t, match_id, get(&counted, "actions")?).await
}

// Rooms (SPEC §9.5's direct challenge)

/// A room is a `public.matches` row with `status = 'open'` (KNOWN DIVERGENCES, rooms). 0004's
/// `app.create_room` is not called: it mints the match id itself, while `rooms_claim` receives the server's,
/// and it sets the host's `current_match_id`, which `src/api/rooms.rs` sets only after the claim (§9.5 "not
/// in a match" would otherwise refuse the host their own room). A taken code returns `false`, by the partial
/// unique index `matches_room_code_open_key`. R264: `p1_deck` holds the Best-of-1 deck, `[]` in other modes.
pub async fn rooms_create(t: &mut PgTx<'_>, room: &Room) -> Result<bool, StoreError> {
    run_as(t, Some(room.host_profile_id.as_str())).await?;
    let host_trio = json_or_null(&room.host_trio)?;
    let done = sqlx::query(concat!(
        "insert into public.matches (
           room_code, room_mode, room_trio, status, seed, p1_profile_id, p1_deck, p1_portrait,
           catalog_version, ceiling_at, created_at, room_lean_set, room_deck_cosmetics)
         values ($1::text, $2::text, $3::jsonb, 'open', '', $4::uuid, $5::jsonb, $6::text,
                 $7::text, ",
        ts!("$8"),
        ", ",
        ts!("$9"),
        ", $10::text, $11::jsonb)
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
    .bind(room.host_lean.map(LeanPick::as_str))
    .bind(json_or_null(&room.host_deck_slots)?)
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

/// §9.5's atomic single-claim, as one statement. The row keeps `status = 'open'` until `matches_create`
/// completes it, so a half-claimed room is never visible to `matches_get`, `matches_live` or the reaper.
/// `id = $3` is the one place this file rewrites a primary key, on purpose: a room that is not yet a match
/// has nothing pointing at it (`current_match_id` is set only after this returns).
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

pub async fn rooms_count_open_by_host(
    t: &mut PgTx<'_>,
    host_profile_id: &str,
    at: i64,
) -> Result<i64, StoreError> {
    if !is_uuid(host_profile_id) {
        return Ok(0);
    }
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select count(*)::int as n from public.matches
          where p1_profile_id = $1::uuid
            and status = 'open'
            and room_code is not null
            and p2_profile_id is null
            and ceiling_at > ",
        ts!("$2")
    ))
    .bind(host_profile_id)
    .bind(at)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.map_or(Ok(0), |row| get::<i32>(&row, "n").map(i64::from))
}

/// R1075000: newest first, the code breaking a tie.
pub async fn rooms_open_by_host(
    t: &mut PgTx<'_>,
    host_profile_id: &str,
    at: i64,
) -> Result<Vec<Room>, StoreError> {
    if !is_uuid(host_profile_id) {
        return Ok(Vec::new());
    }
    run_as(t, None).await?;
    let rows = sqlx::query(concat!(
        "select ",
        room_columns!(),
        " from public.matches
          where p1_profile_id = $1::uuid
            and status = 'open'
            and room_code is not null
            and p2_profile_id is null
            and ceiling_at > ",
        ts!("$2"),
        "
          order by created_at desc, room_code"
    ))
    .bind(host_profile_id)
    .bind(at)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| RoomRow::read(row).and_then(to_room))
        .collect()
}

// Tickets (SPEC §9.5's ranked queue)

/// `tickets_profile_queued_key` (migration 0004) is the race-proof half of §9.5's "not already queued":
/// `src/api/queue.rs` relies on the insert RAISING for the second one and re-reads the open ticket in a fresh
/// transaction (see `begin`), so there is no `on conflict`. R257, R259: migration 0008's
/// `tickets_frozen_trio_check` holds "a trio exactly when the mode is bo3"; `slot` (0004's) stays NULL.
pub async fn tickets_insert(t: &mut PgTx<'_>, ticket: &Ticket) -> Result<(), StoreError> {
    run_as(t, Some(ticket.profile_id.as_str())).await?;
    let trio = json_or_null(&ticket.trio)?;
    sqlx::query(concat!(
        "insert into public.tickets
           (id, profile_id, rating, mode, frozen_deck, portrait, frozen_trio, catalog_version, status,
            enqueued_at, match_id, lean_set, deck_cosmetics)
         values ($1::uuid, $2::uuid, $3::double precision, $4::text, $5::jsonb, $6::text, $7::jsonb, $8::text,
                 $9::text, ",
        ts!("$10"),
        ", $11::uuid, $12::text, $13::jsonb)"
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
    .bind(ticket.lean.map(LeanPick::as_str))
    .bind(json_or_null(&ticket.deck_slots)?)
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

pub async fn tickets_recent_waits(t: &mut PgTx<'_>, since: i64) -> Result<Vec<TicketWait>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(concat!(
        "select mode, enqueued_at, claimed_at from public.tickets
          where status = 'claimed' and claimed_at >= ",
        ts!("$1"),
        " order by claimed_at, id"
    ))
    .bind(since)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| {
            let enqueued: OffsetDateTime = get(row, "enqueued_at")?;
            let claimed: OffsetDateTime = get(row, "claimed_at")?;
            Ok(TicketWait {
                mode: queue_mode_of(&get::<String>(row, "mode")?)?,
                waited_ms: (ms_of(claimed) - ms_of(enqueued)).max(0),
            })
        })
        .collect()
}

/// §9.5: "both tickets are claimed in one atomic statement": `app.claim_ticket_pair` (migration 0004), true
/// only when it moved exactly two still-queued rows. It gets a NULL match id and the link is written right
/// after, in the same transaction (`tickets.match_id` is a foreign key into `public.matches`); the skeleton
/// is the `open` row `matches_create` completes. Before answering false the function flips whichever ticket
/// is still queued, so a pair that lost one to another matcher or a bot's claim (R1453) would strand the
/// other, claimed with no match; the savepoint takes that flip back.
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
    sqlx::query("savepoint claim_ticket_pair")
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
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
        sqlx::query("rollback to savepoint claim_ticket_pair")
            .execute(&mut **t)
            .await
            .map_err(db_error)?;
        return Ok(false);
    }
    sqlx::query("release savepoint claim_ticket_pair")
        .execute(&mut **t)
        .await
        .map_err(db_error)?;

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
    // R640002: the claim is stamped with the sweep's clock, as the fake stamps it, not the
    // transaction's `now()` that `app.claim_ticket_pair` wrote: a wait runs from enqueue to here.
    sqlx::query(concat!(
        "update public.tickets set match_id = $3::uuid, claimed_at = ",
        ts!("$4"),
        " where id in ($1::uuid, $2::uuid)"
    ))
    .bind(a_id)
    .bind(b_id)
    .bind(match_id)
    .bind(at)
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

// Results (SPEC §2.5, §9.5)

/// A profile's finished-match record, counted in one pass over `results`. A draw is a row with no winner
/// (§9.5), so the three counts partition every finished match. A row also has no winner when the winner
/// deleted their account (migration 0012): a `DECISIVE_REASONS` ending always had a winner, so for the player
/// left it is still a loss. R1843004: a `fled` row (`LOSER_DRAWS_REASONS`) is the winner's win and the other
/// player's draw, whether its winner is still there or not.
pub async fn results_record_for(t: &mut PgTx<'_>, profile_id: &str) -> Result<ProfileRecord, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let decisive: Vec<String> = DECISIVE_REASONS.iter().map(|reason| reason.to_string()).collect();
    let row = sqlx::query(
        "select
           count(*) filter (where winner_profile_id = $1::uuid)                         as wins,
           count(*) filter (where winner_profile_id is distinct from $1::uuid
                              and ((winner_profile_id is not null
                                    and reason <> all($3::text[]))
                                   or (winner_profile_id is null
                                       and reason = any($2::text[]))))                   as losses,
           count(*) filter (where (winner_profile_id is null
                                   and reason <> all($2::text[]))
                              or (winner_profile_id is distinct from $1::uuid
                                  and winner_profile_id is not null
                                  and reason = any($3::text[])))                         as draws
         from public.results
        where p1_profile_id = $1::uuid or p2_profile_id = $1::uuid",
    )
    .bind(profile_id)
    .bind(decisive)
    .bind(loser_draws_reasons())
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
        // `results_pkey` answering a write whose `results_get_by_match` ran before a concurrent first writer
        // committed: surfaced as the port's own error so `results.rs`'s retry sees the collision.
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

// The Conquest series (SPEC §9.5, R259-R263), migration 0009
//
// None of these reaches an `app.*` function: `update` is compare-and-set in its `where`, the lookups are
// single selects, and the transitions are `src/api/series_rules.rs`'s pure functions, written back whole.

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

/// R263: "written only by compare-and-set on its version". The guard is the `where` of one
/// statement, so there is no window between the check and the write.
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

// Tutorial progress on the account (SPEC §9.10, R320)

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

/// R320: one call to `app.merge_tutorial_progress` (0011), under the profile row lock; then the row
/// as it now stands, read in the same transaction so the answer is exactly what this write left.
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

// Player settings on the account (SPEC §9.1, R633, R634)

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

/// R634: one call to `app.merge_player_settings` (0018), under the profile row lock, which also
/// caps the result; then the row as it now stands, read in the same transaction so the answer is
/// exactly what this write left.
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

// Last boards (C+ #29, R417, R565). No `app.*` function: the one rule is "replace", which the
// primary key on (profile_id, kind) says.

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

// Game records for the card statistics (SPEC §9.11, R376)

/// R376: one record per id; `on conflict do nothing` answers a second write of a game with false, a row whose
/// detail the purge emptied included (R1044001). R1044000: the records written are written folded and their
/// counts added to the totals (`stats_totals_add`) before the transaction commits, so a record is in the
/// totals exactly when it is in the table. No `app.*` function: the fold is the engine's, in Rust.
pub async fn game_records_insert_batch(
    t: &mut PgTx<'_>,
    records: &[GameRecord],
) -> Result<Vec<bool>, StoreError> {
    run_as(t, None).await?;
    let mut written = Vec::with_capacity(records.len());
    let mut delta = StatsTotals::default();
    for record in records {
        let done = sqlx::query(
            "insert into public.game_records (id, source, mode, patch, record, folded)
             values ($1::text, $2::text, $3::text, $4::text, $5::jsonb, true)
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
        let wrote = done.rows_affected() == 1;
        if wrote {
            delta.fold(record, true);
        }
        written.push(wrote);
    }
    stats_totals_add(t, &delta).await?;
    Ok(written)
}

/// R1976000: the ids `game_records` holds are left out, the batch is claimed with the count it will fold
/// (`on conflict do nothing`: a batch held already answers `None`), and the rest are added to the totals. One
/// transaction, so a claim is never left without its counts; a second import of the same file waits on the
/// first's claim and then finds it held. No `app.*` function.
pub async fn game_record_batch_fold(
    t: &mut PgTx<'_>,
    batch: &str,
    records: &[GameRecord],
) -> Result<Option<BatchFolded>, StoreError> {
    run_as(t, None).await?;
    let ids: Vec<&str> = records.iter().map(|record| record.id.as_str()).collect();
    let held: Vec<String> =
        sqlx::query_scalar("select id from public.game_records where id = any($1::text[])")
            .bind(&ids)
            .fetch_all(&mut **t)
            .await
            .map_err(db_error)?;
    let mut seen: IndexSet<&str> = held.iter().map(String::as_str).collect();
    let mut delta = StatsTotals::default();
    let mut folded = 0;
    for record in records {
        if seen.insert(record.id.as_str()) {
            delta.fold(record, true);
            folded += 1;
        }
    }
    let claimed = sqlx::query(
        "insert into public.game_record_batches (batch, records) values ($1::text, $2::int)
         on conflict (batch) do nothing",
    )
    .bind(batch)
    .bind(i32::try_from(folded).map_err(|_| StoreError::from(format!("a batch of {folded} records")))?)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    if claimed.rows_affected() == 0 {
        return Ok(None);
    }
    stats_totals_add(t, &delta).await?;
    Ok(Some(BatchFolded {
        folded,
        left_out: records.len() - folded,
    }))
}

/// The filter on the indexed columns; each row whose detail is still held (R1044001) read back
/// through `parse_game_record`.
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
            and record is not null
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

/// R1044000: `skip locked`, so two folds running at once take different records and none is folded
/// twice. R1715000: a record whose detail does not read is marked folded with its detail emptied,
/// so it never holds back the records behind it.
pub async fn game_records_fold_pending(t: &mut PgTx<'_>, limit: usize) -> Result<FoldedRecords, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select id, record from public.game_records
          where not folded
          order by id collate \"C\"
          limit $1
          for update skip locked",
    )
    .bind(i64::try_from(limit).unwrap_or(i64::MAX))
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    let mut delta = StatsTotals::default();
    let mut ids: Vec<String> = Vec::with_capacity(rows.len());
    let mut unreadable: Vec<UnreadableGameRecord> = Vec::new();
    for row in &rows {
        let id: String = get(row, "id")?;
        let detail: Value = get(row, "record")?;
        match parse_game_record(&detail) {
            Ok(record) => {
                delta.fold(&record, true);
                ids.push(id);
            }
            Err(error) => unreadable.push(UnreadableGameRecord {
                id,
                message: error.to_string(),
            }),
        }
    }
    if !ids.is_empty() {
        sqlx::query("update public.game_records set folded = true where id = any($1::text[])")
            .bind(&ids)
            .execute(&mut **t)
            .await
            .map_err(db_error)?;
        stats_totals_add(t, &delta).await?;
    }
    if !unreadable.is_empty() {
        let unread: Vec<String> = unreadable.iter().map(|record| record.id.clone()).collect();
        sqlx::query("update public.game_records set folded = true, record = null where id = any($1::text[])")
            .bind(&unread)
            .execute(&mut **t)
            .await
            .map_err(db_error)?;
    }
    Ok(FoldedRecords {
        folded: ids.len(),
        unreadable,
    })
}

/// R1596000 (R768, R112, R1253000, R1044001): the rows `Tx::game_records_unfiled` names, in its order.
pub async fn game_records_unfiled(
    t: &mut PgTx<'_>,
    ended_after: i64,
    ended_before: i64,
) -> Result<Vec<String>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(concat!(
        "select m.id from public.matches m",
        " where m.status = 'over'",
        " and m.final_hash is not null",
        " and m.ended_at >= ",
        ts!("$1"),
        " and m.ended_at < ",
        ts!("$2"),
        " and exists (select 1 from public.results r where r.match_id = m.id)",
        " and exists (select 1 from public.match_actions a where a.match_id = m.id)",
        " and not exists (select 1 from public.game_records g where g.id = m.id::text)",
        " order by m.ended_at, m.id",
    ))
    .bind(ended_after)
    .bind(ended_before)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(|row| uuid_text(row, "id")).collect()
}

/// `card_totals.co` as stored: each other card's `[games, wins, record, seat, index]`.
fn co_json(co: &IndexMap<String, CoCount>) -> Value {
    Value::Object(
        co.iter()
            .map(|(card, count)| {
                (
                    card.clone(),
                    json!([
                        count.games,
                        count.wins,
                        count.first.record,
                        count.first.seat,
                        count.first.index
                    ]),
                )
            })
            .collect(),
    )
}

fn co_of(value: Value) -> Result<IndexMap<String, CoCount>, StoreError> {
    let bad = || {
        StoreError::from("card_totals.co: expected { card: [games, wins, record, seat, index] }".to_string())
    };
    let Value::Object(entries) = value else {
        return Err(bad());
    };
    entries
        .into_iter()
        .map(|(card, entry)| {
            let (games, wins, record, seat, index): (i64, i64, String, u8, u32) =
                serde_json::from_value(entry).map_err(|_| bad())?;
            Ok((
                card,
                CoCount {
                    games,
                    wins,
                    first: FirstMet { record, seat, index },
                },
            ))
        })
        .collect()
}

/// `card_totals.turns` as stored: each turn's `[games, wins]`.
fn turns_json(turns: &IndexMap<i32, DrillCount>) -> Value {
    Value::Object(
        turns
            .iter()
            .map(|(turn, count)| (turn.to_string(), json!([count.games, count.wins])))
            .collect(),
    )
}

fn turns_of(value: Value) -> Result<IndexMap<i32, DrillCount>, StoreError> {
    let bad = || StoreError::from("card_totals.turns: expected { turn: [games, wins] }".to_string());
    let Value::Object(entries) = value else {
        return Err(bad());
    };
    entries
        .into_iter()
        .map(|(turn, entry)| {
            let turn: i32 = turn.parse().map_err(|_| bad())?;
            let (games, wins): (i64, i64) = serde_json::from_value(entry).map_err(|_| bad())?;
            Ok((turn, DrillCount { games, wins }))
        })
        .collect()
}

/// R1044000, R1994000: the record counts are added in SQL, the rest read, added to in Rust
/// (`StatsTotals::add`, the engine's counting) and written back. A transaction-level advisory lock
/// on each patch and source the delta touches, taken in order, keeps two folds into the same rows
/// from interleaving.
async fn stats_totals_add(t: &mut PgTx<'_>, delta: &StatsTotals) -> Result<(), StoreError> {
    if delta.records.is_empty() {
        return Ok(());
    }
    let mut locks: Vec<String> = Vec::new();
    for key in delta.records.keys() {
        locks.push(format!(
            "jackioh.stats_totals:{}:{}",
            key.patch,
            literal(&key.source)?
        ));
    }
    locks.sort();
    locks.dedup();
    for lock in &locks {
        sqlx::query("select pg_advisory_xact_lock(hashtextextended($1::text, 0))")
            .bind(lock)
            .execute(&mut **t)
            .await
            .map_err(db_error)?;
    }

    let mut columns: [Vec<String>; 5] = Default::default();
    let mut counts: Vec<i64> = Vec::new();
    for (key, records) in &delta.records {
        columns[0].push(key.patch.clone());
        columns[1].push(literal(&key.source)?);
        columns[2].push(literal(&key.mode)?);
        columns[3].push(literal(&key.pilots.0)?);
        columns[4].push(literal(&key.pilots.1)?);
        counts.push(*records);
    }
    let [patches, sources, modes, p1_pilots, p2_pilots] = columns;
    sqlx::query(
        "insert into public.record_totals (patch, source, mode, p1_pilot, p2_pilot, records)
         select * from unnest($1::text[], $2::text[], $3::text[], $4::text[], $5::text[], $6::int8[])
         on conflict (patch, source, mode, p1_pilot, p2_pilot)
         do update set records = public.record_totals.records + excluded.records",
    )
    .bind(patches)
    .bind(sources)
    .bind(modes)
    .bind(p1_pilots)
    .bind(p2_pilots)
    .bind(counts)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;

    let mut keys: [Vec<String>; 5] = Default::default();
    for (key, cards) in &delta.cards {
        for card in cards.keys() {
            keys[0].push(key.patch.clone());
            keys[1].push(literal(&key.source)?);
            keys[2].push(literal(&key.mode)?);
            keys[3].push(literal(&key.pilot)?);
            keys[4].push(card.clone());
        }
    }
    let [patches, sources, modes, pilots, cards] = keys;
    let rows = sqlx::query(
        "select c.patch, c.source, c.mode, c.pilot, c.card, c.stats, c.turns, c.co
           from public.card_totals c
           join unnest($1::text[], $2::text[], $3::text[], $4::text[], $5::text[])
             as k(patch, source, mode, pilot, card)
             on c.patch = k.patch and c.source = k.source and c.mode = k.mode
            and c.pilot = k.pilot and c.card = k.card",
    )
    .bind(&patches)
    .bind(&sources)
    .bind(&modes)
    .bind(&pilots)
    .bind(&cards)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    let mut stored = StatsTotals::default();
    for row in &rows {
        let key = TotalsKey {
            patch: get(row, "patch")?,
            source: from_literal(&get::<String>(row, "source")?)?,
            mode: from_literal(&get::<String>(row, "mode")?)?,
            pilot: from_literal(&get::<String>(row, "pilot")?)?,
        };
        let card: String = get(row, "card")?;
        let total = CardTotal {
            stats: from_json(get::<Value>(row, "stats")?)?,
            turns: turns_of(get::<Value>(row, "turns")?)?,
            co: co_of(get::<Value>(row, "co")?)?,
        };
        stored.cards.entry(key).or_default().insert(card, total);
    }

    let mut list_patches: Vec<String> = Vec::new();
    let mut list_keys: Vec<String> = Vec::new();
    for (patch, lists) in &delta.lists {
        for list in lists.keys() {
            list_patches.push(patch.clone());
            list_keys.push(list.clone());
        }
    }
    if !list_keys.is_empty() {
        let rows = sqlx::query(
            "select d.patch, d.list, d.deck, d.games, d.wins, d.first_record, d.first_seat
               from public.decklist_totals d
               join unnest($1::text[], $2::text[]) as k(patch, list)
                 on d.patch = k.patch and d.list = k.list",
        )
        .bind(&list_patches)
        .bind(&list_keys)
        .fetch_all(&mut **t)
        .await
        .map_err(db_error)?;
        for row in &rows {
            let patch: String = get(row, "patch")?;
            let list: String = get(row, "list")?;
            let total = list_total_of(row)?;
            stored.lists.entry(patch).or_default().insert(list, total);
        }
    }

    let mut keys: [Vec<String>; 5] = Default::default();
    for key in delta.game_totals.keys() {
        keys[0].push(key.patch.clone());
        keys[1].push(literal(&key.source)?);
        keys[2].push(literal(&key.mode)?);
        keys[3].push(literal(&key.pilots.0)?);
        keys[4].push(literal(&key.pilots.1)?);
    }
    let [patches, sources, modes, p1_pilots, p2_pilots] = keys;
    let rows = sqlx::query(
        "select g.patch, g.source, g.mode, g.p1_pilot, g.p2_pilot, g.total
           from public.game_totals g
           join unnest($1::text[], $2::text[], $3::text[], $4::text[], $5::text[])
             as k(patch, source, mode, p1_pilot, p2_pilot)
             on g.patch = k.patch and g.source = k.source and g.mode = k.mode
            and g.p1_pilot = k.p1_pilot and g.p2_pilot = k.p2_pilot",
    )
    .bind(&patches)
    .bind(&sources)
    .bind(&modes)
    .bind(&p1_pilots)
    .bind(&p2_pilots)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    for row in &rows {
        stored
            .game_totals
            .insert(records_key_of(row)?, from_json(get::<Value>(row, "total")?)?);
    }

    stored.add(delta);

    let mut columns: [Vec<String>; 8] = Default::default();
    for (key, cards) in &stored.cards {
        for (card, total) in cards {
            columns[0].push(key.patch.clone());
            columns[1].push(literal(&key.source)?);
            columns[2].push(literal(&key.mode)?);
            columns[3].push(literal(&key.pilot)?);
            columns[4].push(card.clone());
            columns[5].push(json(&total.stats)?);
            columns[6].push(turns_json(&total.turns).to_string());
            columns[7].push(co_json(&total.co).to_string());
        }
    }
    let [patches, sources, modes, pilots, cards, stats, turns, co] = columns;
    sqlx::query(
        "insert into public.card_totals (patch, source, mode, pilot, card, stats, turns, co)
         select k.patch, k.source, k.mode, k.pilot, k.card, k.stats::jsonb, k.turns::jsonb, k.co::jsonb
           from unnest($1::text[], $2::text[], $3::text[], $4::text[], $5::text[], $6::text[], $7::text[],
                       $8::text[])
             as k(patch, source, mode, pilot, card, stats, turns, co)
         on conflict (patch, source, mode, pilot, card)
         do update set stats = excluded.stats, turns = excluded.turns, co = excluded.co",
    )
    .bind(patches)
    .bind(sources)
    .bind(modes)
    .bind(pilots)
    .bind(cards)
    .bind(stats)
    .bind(turns)
    .bind(co)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;

    let mut columns: [Vec<String>; 6] = Default::default();
    for (key, total) in &stored.game_totals {
        columns[0].push(key.patch.clone());
        columns[1].push(literal(&key.source)?);
        columns[2].push(literal(&key.mode)?);
        columns[3].push(literal(&key.pilots.0)?);
        columns[4].push(literal(&key.pilots.1)?);
        columns[5].push(json(total)?);
    }
    let [patches, sources, modes, p1_pilots, p2_pilots, totals] = columns;
    sqlx::query(
        "insert into public.game_totals (patch, source, mode, p1_pilot, p2_pilot, total)
         select k.patch, k.source, k.mode, k.p1_pilot, k.p2_pilot, k.total::jsonb
           from unnest($1::text[], $2::text[], $3::text[], $4::text[], $5::text[], $6::text[])
             as k(patch, source, mode, p1_pilot, p2_pilot, total)
         on conflict (patch, source, mode, p1_pilot, p2_pilot)
         do update set total = excluded.total",
    )
    .bind(patches)
    .bind(sources)
    .bind(modes)
    .bind(p1_pilots)
    .bind(p2_pilots)
    .bind(totals)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;

    if !list_keys.is_empty() {
        let mut texts: [Vec<String>; 4] = Default::default();
        let mut games: Vec<i64> = Vec::new();
        let mut wins: Vec<i64> = Vec::new();
        let mut seats: Vec<i16> = Vec::new();
        for (patch, lists) in &stored.lists {
            for (list, total) in lists {
                texts[0].push(patch.clone());
                texts[1].push(list.clone());
                texts[2].push(json(&total.deck)?);
                texts[3].push(total.first.record.clone());
                games.push(total.games);
                wins.push(total.wins);
                seats.push(i16::from(total.first.seat));
            }
        }
        let [patches, lists, decks, first_records] = texts;
        sqlx::query(
            "insert into public.decklist_totals (patch, list, deck, games, wins, first_record, first_seat)
             select k.patch, k.list, k.deck::jsonb, k.games, k.wins, k.first_record, k.first_seat
               from unnest($1::text[], $2::text[], $3::text[], $4::int8[], $5::int8[], $6::text[],
                           $7::int2[])
                 as k(patch, list, deck, games, wins, first_record, first_seat)
             on conflict (patch, list)
             do update set deck = excluded.deck, games = excluded.games, wins = excluded.wins,
                           first_record = excluded.first_record, first_seat = excluded.first_seat",
        )
        .bind(patches)
        .bind(lists)
        .bind(decks)
        .bind(games)
        .bind(wins)
        .bind(first_records)
        .bind(seats)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    }
    Ok(())
}

/// A `decklist_totals` row read back: its key is its deck, sorted.
fn list_total_of(row: &PgRow) -> Result<ListTotal, StoreError> {
    let deck: Vec<String> = from_json(get::<Value>(row, "deck")?)?;
    let mut key = deck.clone();
    key.sort();
    let seat: i16 = get(row, "first_seat")?;
    Ok(ListTotal {
        deck,
        key,
        games: get(row, "games")?,
        wins: get(row, "wins")?,
        first: FirstSeen {
            record: get(row, "first_record")?,
            seat: u8::try_from(seat)
                .map_err(|_| StoreError::from(format!("decklist_totals.first_seat {seat}")))?,
        },
    })
}

/// A `record_totals` or `game_totals` row's key: its patch, source, mode and both seats' pilots.
fn records_key_of(row: &PgRow) -> Result<RecordsKey, StoreError> {
    Ok(RecordsKey {
        patch: get(row, "patch")?,
        source: from_literal(&get::<String>(row, "source")?)?,
        mode: from_literal(&get::<String>(row, "mode")?)?,
        pilots: (
            from_literal(&get::<String>(row, "p1_pilot")?)?,
            from_literal(&get::<String>(row, "p2_pilot")?)?,
        ),
    })
}

/// R1044000: the totals the query reads, each card's turn and co-played counts only with `drills`.
/// R1994000: and each key's whole games.
pub async fn stats_totals_load(
    t: &mut PgTx<'_>,
    query: &StatsTotalsQuery,
) -> Result<StatsTotals, StoreError> {
    let sources = sources_of(query.source)
        .iter()
        .map(literal)
        .collect::<Result<Vec<String>, StoreError>>()?;
    let mode = query.mode.as_ref().map(literal).transpose()?;
    run_as(t, None).await?;
    let mut totals = StatsTotals::default();
    let rows = sqlx::query(
        "select patch, source, mode, p1_pilot, p2_pilot, records from public.record_totals
          where source = any($1::text[])
            and ($2::text is null or mode = $2::text)
            and ($3::text is null or patch = $3::text)",
    )
    .bind(&sources)
    .bind(mode.as_deref())
    .bind(query.patch.as_deref())
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    for row in &rows {
        totals.records.insert(records_key_of(row)?, get(row, "records")?);
    }
    let rows = sqlx::query(
        "select patch, source, mode, p1_pilot, p2_pilot, total from public.game_totals
          where source = any($1::text[])
            and ($2::text is null or mode = $2::text)
            and ($3::text is null or patch = $3::text)",
    )
    .bind(&sources)
    .bind(mode.as_deref())
    .bind(query.patch.as_deref())
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    for row in &rows {
        totals
            .game_totals
            .insert(records_key_of(row)?, from_json(get::<Value>(row, "total")?)?);
    }
    let rows = sqlx::query(
        "select patch, source, mode, pilot, card, stats,
                case when $4::boolean then turns end as turns,
                case when $4::boolean then co end as co
           from public.card_totals
          where source = any($1::text[])
            and ($2::text is null or mode = $2::text)
            and ($3::text is null or patch = $3::text)",
    )
    .bind(&sources)
    .bind(mode.as_deref())
    .bind(query.patch.as_deref())
    .bind(query.drills)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    for row in &rows {
        let key = TotalsKey {
            patch: get(row, "patch")?,
            source: from_literal(&get::<String>(row, "source")?)?,
            mode: from_literal(&get::<String>(row, "mode")?)?,
            pilot: from_literal(&get::<String>(row, "pilot")?)?,
        };
        let card: String = get(row, "card")?;
        let total = CardTotal {
            stats: from_json(get::<Value>(row, "stats")?)?,
            turns: get::<Option<Value>>(row, "turns")?
                .map(turns_of)
                .transpose()?
                .unwrap_or_default(),
            co: get::<Option<Value>>(row, "co")?
                .map(co_of)
                .transpose()?
                .unwrap_or_default(),
        };
        totals.cards.entry(key).or_default().insert(card, total);
    }
    Ok(totals)
}

/// R1044000 (R1454, R750000): the patch's live Best-of-1 human decklists, first held first.
pub async fn stats_decklists(t: &mut PgTx<'_>, patch: &str) -> Result<Vec<ListTotal>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select deck, games, wins, first_record, first_seat from public.decklist_totals
          where patch = $1::text
          order by first_record collate \"C\", first_seat",
    )
    .bind(patch)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(list_total_of).collect()
}

// Play telemetry (SPEC §9.11, R1442): migration 0029's three tables. No `app.*` function: each read
// is a single select.

/// R1442: one `jsonb_to_recordset` insert per table that has rows, each skipping a key already
/// held and a seat whose account is deleted (0012 empties its profile column on the match, and
/// 0029's trigger has already deleted its rows), in the one transaction. A row naming a match the
/// store does not hold fails the insert on the foreign key, and the transaction with it.
pub async fn play_telemetry_insert(t: &mut PgTx<'_>, telemetry: &PlayTelemetry) -> Result<(), StoreError> {
    run_as(t, None).await?;
    if !telemetry.action_timings.is_empty() {
        let rows = telemetry
            .action_timings
            .iter()
            .map(|row| -> Result<Value, StoreError> {
                Ok(json!({
                    "match_id": row.match_id,
                    "seq": row.seq,
                    "seat": row.seat.as_str(),
                    "action_kind": row.action_kind.as_str(),
                    "legal_count": row.legal_count,
                    "turn": row.turn,
                    "think_ms": row.think_ms,
                    "clock_left_ms": row.clock_left_ms,
                    "first_in_turn": row.first_in_turn,
                    "rank_bucket": literal_or_null(&row.rank_bucket)?,
                    "pilot": row.pilot.as_str(),
                }))
            })
            .collect::<Result<Vec<Value>, StoreError>>()?;
        sqlx::query(
            "insert into public.action_timings
               (match_id, seq, seat, action_kind, legal_count, turn, think_ms, clock_left_ms,
                first_in_turn, rank_bucket, pilot)
             select r.match_id::uuid, r.seq, r.seat, r.action_kind, r.legal_count, r.turn, r.think_ms,
                    r.clock_left_ms, r.first_in_turn, r.rank_bucket, r.pilot
               from jsonb_to_recordset($1::jsonb)
                 as r(match_id text, seq bigint, seat text, action_kind text, legal_count int, turn int,
                      think_ms bigint, clock_left_ms bigint, first_in_turn boolean, rank_bucket text,
                      pilot text)
             where not exists (
               select 1 from public.matches m
                where m.id = r.match_id::uuid
                  and (case r.seat when 'p1' then m.p1_profile_id else m.p2_profile_id end) is null)
             on conflict do nothing",
        )
        .bind(json(&rows)?)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    }
    if !telemetry.emote_events.is_empty() {
        let rows: Vec<Value> = telemetry
            .emote_events
            .iter()
            .map(|row| {
                json!({
                    "match_id": row.match_id,
                    "ordinal": row.ordinal,
                    "seat": row.seat.as_str(),
                    "emote_id": row.emote_id.as_str(),
                    "turn": row.turn,
                    "trigger_event": row.trigger_event.map(GameEventType::as_str),
                    "ms_since_trigger": row.ms_since_trigger,
                    "reply_to_opponent_ms": row.reply_to_opponent_ms,
                    "pilot": row.pilot.as_str(),
                })
            })
            .collect();
        sqlx::query(
            "insert into public.emote_events
               (match_id, ordinal, seat, emote_id, turn, trigger_event, ms_since_trigger,
                reply_to_opponent_ms, pilot)
             select r.match_id::uuid, r.ordinal, r.seat, r.emote_id, r.turn, r.trigger_event,
                    r.ms_since_trigger, r.reply_to_opponent_ms, r.pilot
               from jsonb_to_recordset($1::jsonb)
                 as r(match_id text, ordinal int, seat text, emote_id text, turn int, trigger_event text,
                      ms_since_trigger bigint, reply_to_opponent_ms bigint, pilot text)
             where not exists (
               select 1 from public.matches m
                where m.id = r.match_id::uuid
                  and (case r.seat when 'p1' then m.p1_profile_id else m.p2_profile_id end) is null)
             on conflict do nothing",
        )
        .bind(json(&rows)?)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    }
    if !telemetry.match_signals.is_empty() {
        let rows: Vec<Value> = telemetry
            .match_signals
            .iter()
            .map(|row| {
                json!({
                    "match_id": row.match_id,
                    "seat": row.seat.as_str(),
                    "conceded_turn": row.conceded_turn,
                    "concede_eval_deficit": row.concede_eval_deficit,
                    "draw_offers": row.draw_offers,
                    "draw_accepted": row.draw_accepted,
                    "rematch_offered": row.rematch_offered,
                    "rematch_accepted": row.rematch_accepted,
                    "timeouts": row.timeouts,
                    "pilot": row.pilot.as_str(),
                })
            })
            .collect();
        sqlx::query(
            "insert into public.match_signals
               (match_id, seat, conceded_turn, concede_eval_deficit, draw_offers, draw_accepted,
                rematch_offered, rematch_accepted, timeouts, pilot)
             select r.match_id::uuid, r.seat, r.conceded_turn, r.concede_eval_deficit, r.draw_offers,
                    r.draw_accepted, r.rematch_offered, r.rematch_accepted, r.timeouts, r.pilot
               from jsonb_to_recordset($1::jsonb)
                 as r(match_id text, seat text, conceded_turn int, concede_eval_deficit double precision,
                      draw_offers int, draw_accepted boolean, rematch_offered boolean,
                      rematch_accepted boolean, timeouts int, pilot text)
             where not exists (
               select 1 from public.matches m
                where m.id = r.match_id::uuid
                  and (case r.seat when 'p1' then m.p1_profile_id else m.p2_profile_id end) is null)
             on conflict do nothing",
        )
        .bind(json(&rows)?)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    }
    Ok(())
}

/// R672, R1442: marks the seat's offer, and whether it made the rematch; an offer once made stays.
pub async fn play_telemetry_note_rematch(
    t: &mut PgTx<'_>,
    match_id: &str,
    seat: PlayerId,
    made: bool,
) -> Result<(), StoreError> {
    // A non-uuid id names no match, as the fake finds none.
    if !is_uuid(match_id) {
        return Ok(());
    }
    run_as(t, None).await?;
    sqlx::query(
        "update public.match_signals
            set rematch_offered = true, rematch_accepted = rematch_accepted or $3::boolean
          where match_id = $1::uuid and seat = $2::text",
    )
    .bind(match_id)
    .bind(seat.as_str())
    .bind(made)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

macro_rules! action_timing_columns {
    () => {
        "match_id, seq, seat, action_kind, legal_count, turn, think_ms, clock_left_ms, first_in_turn, \
         rank_bucket, pilot"
    };
}

fn action_timing_of(row: &PgRow) -> Result<ActionTimingRow, StoreError> {
    Ok(ActionTimingRow {
        match_id: uuid_text(row, "match_id")?,
        seat: from_literal(&get::<String>(row, "seat")?)?,
        seq: get(row, "seq")?,
        action_kind: from_literal(&get::<String>(row, "action_kind")?)?,
        legal_count: get(row, "legal_count")?,
        turn: get(row, "turn")?,
        think_ms: get(row, "think_ms")?,
        clock_left_ms: get(row, "clock_left_ms")?,
        first_in_turn: get(row, "first_in_turn")?,
        rank_bucket: get::<Option<String>>(row, "rank_bucket")?
            .map(|tier| from_literal(&tier))
            .transpose()?,
        pilot: from_literal(&get::<String>(row, "pilot")?)?,
    })
}

macro_rules! emote_event_columns {
    () => {
        "match_id, ordinal, seat, emote_id, turn, trigger_event, ms_since_trigger, reply_to_opponent_ms, \
         pilot"
    };
}

fn emote_event_of(row: &PgRow) -> Result<EmoteEventRow, StoreError> {
    Ok(EmoteEventRow {
        match_id: uuid_text(row, "match_id")?,
        seat: from_literal(&get::<String>(row, "seat")?)?,
        ordinal: get(row, "ordinal")?,
        emote_id: from_literal(&get::<String>(row, "emote_id")?)?,
        turn: get(row, "turn")?,
        trigger_event: get::<Option<String>>(row, "trigger_event")?
            .map(|event| from_literal(&event))
            .transpose()?,
        ms_since_trigger: get(row, "ms_since_trigger")?,
        reply_to_opponent_ms: get(row, "reply_to_opponent_ms")?,
        pilot: from_literal(&get::<String>(row, "pilot")?)?,
    })
}

fn match_signal_of(row: &PgRow) -> Result<MatchSignalRow, StoreError> {
    Ok(MatchSignalRow {
        match_id: uuid_text(row, "match_id")?,
        seat: from_literal(&get::<String>(row, "seat")?)?,
        conceded_turn: get(row, "conceded_turn")?,
        concede_eval_deficit: get(row, "concede_eval_deficit")?,
        draw_offers: get(row, "draw_offers")?,
        draw_accepted: get(row, "draw_accepted")?,
        rematch_offered: get(row, "rematch_offered")?,
        rematch_accepted: get(row, "rematch_accepted")?,
        timeouts: get(row, "timeouts")?,
        pilot: from_literal(&get::<String>(row, "pilot")?)?,
    })
}

pub async fn play_telemetry_of(t: &mut PgTx<'_>, match_id: &str) -> Result<PlayTelemetry, StoreError> {
    if !is_uuid(match_id) {
        return Ok(PlayTelemetry::default());
    }
    run_as(t, None).await?;
    let timings = sqlx::query(concat!(
        "select ",
        action_timing_columns!(),
        " from public.action_timings where match_id = $1::uuid order by seq"
    ))
    .bind(match_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    let emotes = sqlx::query(concat!(
        "select ",
        emote_event_columns!(),
        " from public.emote_events where match_id = $1::uuid order by ordinal"
    ))
    .bind(match_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    let signals = sqlx::query(
        "select match_id, seat, conceded_turn, concede_eval_deficit, draw_offers, draw_accepted,
                rematch_offered, rematch_accepted, timeouts, pilot
           from public.match_signals where match_id = $1::uuid order by seat",
    )
    .bind(match_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(PlayTelemetry {
        action_timings: timings.iter().map(action_timing_of).collect::<Result<_, _>>()?,
        emote_events: emotes.iter().map(emote_event_of).collect::<Result<_, _>>()?,
        match_signals: signals.iter().map(match_signal_of).collect::<Result<_, _>>()?,
    })
}

pub async fn play_telemetry_timings(t: &mut PgTx<'_>) -> Result<Vec<ActionTimingRow>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(concat!(
        "select ",
        action_timing_columns!(),
        " from public.action_timings order by match_id, seq"
    ))
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(action_timing_of).collect()
}

pub async fn play_telemetry_emotes(t: &mut PgTx<'_>) -> Result<Vec<EmoteEventRow>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(concat!(
        "select ",
        emote_event_columns!(),
        " from public.emote_events order by match_id, ordinal"
    ))
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(emote_event_of).collect()
}

/// R639000: `won` is null for a draw, and otherwise whether the winner is the account that began in
/// the row's seat: the match's own seats, since the result's are the seats as credited after a
/// Glitch's swap (R677). A match is rematchable when it was ranked and no series lists it among its
/// games (`state.games[].matchId`).
pub async fn play_telemetry_outcomes(t: &mut PgTx<'_>) -> Result<Vec<SignalOutcomeRow>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select s.match_id, s.seat, s.conceded_turn, s.concede_eval_deficit, s.draw_offers,
                s.draw_accepted, s.rematch_offered, s.rematch_accepted, s.timeouts, s.pilot,
                case when r.winner_profile_id is null then null
                     else r.winner_profile_id
                          = (case s.seat when 'p1' then m.p1_profile_id else m.p2_profile_id end)
                end as won,
                r.reason, r.turns,
                (coalesce(m.ranked, false)
                 and not exists (
                   select 1 from public.series x
                    where x.state -> 'games'
                          @> jsonb_build_array(jsonb_build_object('matchId', s.match_id::text))))
                  as rematchable
           from public.match_signals s
           join public.results r on r.match_id = s.match_id
           join public.matches m on m.id = s.match_id
          order by s.match_id, s.seat",
    )
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| -> Result<SignalOutcomeRow, StoreError> {
            Ok(SignalOutcomeRow {
                signal: match_signal_of(row)?,
                won: get(row, "won")?,
                reason: from_literal(&get::<String>(row, "reason")?)?,
                turns: i64::from(get::<i32>(row, "turns")?),
                rematchable: get(row, "rematchable")?,
            })
        })
        .collect()
}

pub async fn play_telemetry_unfolded(t: &mut PgTx<'_>) -> Result<Vec<String>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select m.id from public.matches m
          where m.status = 'over'
            and (m.p1_profile_id is not null or m.p2_profile_id is not null)
            and exists (select 1 from public.match_actions a where a.match_id = m.id)
            and not exists (select 1 from public.action_timings s where s.match_id = m.id)
          order by m.ended_at, m.id",
    )
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(|row| uuid_text(row, "id")).collect()
}

// Ranked ladder (SPEC §9.12): migration 0022's four tables plus the Glicko triple on `profiles`. None of
// these reaches an `app.*` function: the writes are upserts or one-statement idempotent writes, the lookups
// single selects.

/// One lock for every season open, so two opens run one after the other and the second reads the
/// first's row.
pub async fn ranked_lock_seasons(t: &mut PgTx<'_>) -> Result<(), StoreError> {
    run_as(t, None).await?;
    sqlx::query("select pg_advisory_xact_lock($1::bigint)")
        .bind(SEASON_LOCK_ID)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(())
}

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

/// R609's input: every profile a rated game has touched, with its Glicko triple. One EXISTS per
/// side, over `rated_games_p1_profile_idx` / `p2`; a deleted profile is already gone from
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

/// What `ranked_standings` and `ranked_standings_subset` select, up to their `where`: a season row
/// joined to its player's CURRENT rating, a ladder bot's being the bot's own (R640000, R610).
macro_rules! standings_from {
    () => {
        "select r.season_id, r.profile_id, r.games, r.wins, r.losses, r.draws, r.ladder, r.floor,
                r.streak, r.peak_ladder, r.peak_jlorious, r.updated_at,
                coalesce(b.rating, p.rating) as rating, p.kind
           from public.season_ranks r
           join public.profiles p on p.id = r.profile_id
           left join public.bot_ratings b on p.kind = 'bot' and b.bot_id = p.id::text"
    };
}

/// One `standings_from!` row: the rank's own fields plus the rating, through their serde form,
/// marked `bot` for a ladder bot's row (R640000).
fn standing_of(row: &PgRow) -> Result<SeasonStanding, StoreError> {
    let rank = to_season_rank(SeasonRankDbRow::read(row)?);
    let rating: f64 = get(row, "rating")?;
    let kind: String = get(row, "kind")?;
    let mut standing = serde_json::to_value(&rank).map_err(|error| StoreError::from(error.to_string()))?;
    if let Value::Object(fields) = &mut standing {
        fields.insert(String::from("rating"), json!(rating));
        fields.insert(String::from("bot"), json!(kind == "bot"));
    }
    from_json(standing)
}

pub async fn ranked_standings(t: &mut PgTx<'_>, season_id: &str) -> Result<Vec<SeasonStanding>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(concat!(
        standings_from!(),
        " where r.season_id = $1::text
          order by r.profile_id"
    ))
    .bind(season_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(standing_of).collect()
}

/// `ranked_standings` cut (#2022). An id that is no uuid (a deleted account's empty seat) names no
/// row and is not bound. Migration 0064's `season_ranks_season_ladder_idx` serves the ladder range
/// (tests/sql CHECK 21).
pub async fn ranked_standings_subset(
    t: &mut PgTx<'_>,
    season_id: &str,
    profile_ids: &[String],
    min_ladder: i32,
) -> Result<Vec<SeasonStanding>, StoreError> {
    let ids: Vec<String> = profile_ids.iter().filter(|id| is_uuid(id)).cloned().collect();
    run_as(t, None).await?;
    let rows = sqlx::query(concat!(
        standings_from!(),
        " where r.season_id = $1::text
            and (r.profile_id = any($2::uuid[]) or r.ladder >= $3::integer)
          order by r.profile_id"
    ))
    .bind(season_id)
    .bind(ids)
    .bind(min_ladder)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(standing_of).collect()
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
        "select bot_id, rating, deviation, volatility, games, updated_at, ai_generation
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
        "insert into public.bot_ratings (bot_id, rating, deviation, volatility, games, updated_at, ai_generation)
         values ($1::text, $2::float8, $3::float8, $4::float8, $5::int, ",
        ts!("$6"),
        ", $7::int)
         on conflict (bot_id) do update set
           rating = excluded.rating, deviation = excluded.deviation,
           volatility = excluded.volatility, games = excluded.games, updated_at = excluded.updated_at,
           ai_generation = excluded.ai_generation"
    ))
    .bind(bot.bot_id.as_str())
    .bind(bot.glicko.rating)
    .bind(bot.glicko.deviation)
    .bind(bot.glicko.volatility)
    .bind(bot.games)
    .bind(bot.updated_at)
    .bind(bot.ai_generation)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

// The ladder bots (R1453, R1454, R1451, R1452, migration 0032)

fn to_bot_row(row: &PgRow) -> Result<BotRow, StoreError> {
    Ok(BotRow {
        profile_id: uuid_text(row, "profile_id")?,
        strength: usize::try_from(get::<i16>(row, "strength")?)
            .map_err(|_| StoreError::from("bots.strength is negative"))?,
        speed: get(row, "speed")?,
        persona: from_literal(&get::<String>(row, "persona")?)?,
        active_from_hour: i32::from(get::<i16>(row, "active_from_hour")?),
        active_hours: i32::from(get::<i16>(row, "active_hours")?),
        decks: json_col(row, "decks")?,
        decks_catalog_version: get(row, "decks_catalog_version")?,
        trio: trio_or_null(get::<Option<Value>>(row, "trio")?.as_ref())?,
        trio_catalog_version: get(row, "trio_catalog_version")?,
        created_at: ms_of(get(row, "created_at")?),
        retired_at: ms_or_null(get(row, "retired_at")?),
        rating: get::<Option<f64>>(row, "rating")?.unwrap_or(RATING_START),
        in_match_id: uuid_text_or_null(row, "current_match_id")?,
        grinding: get(row, "grinding")?,
        grind_wins: get(row, "grind_wins")?,
        cosmetics: get::<Option<Json<BotCosmetics>>>(row, "cosmetics")?.map(|cosmetics| cosmetics.0),
    })
}

pub async fn bots_list(t: &mut PgTx<'_>) -> Result<Vec<BotRow>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select b.profile_id, b.strength, b.speed, b.persona, b.active_from_hour, b.active_hours,
                b.decks, b.decks_catalog_version, b.trio, b.trio_catalog_version, b.retired_at,
                b.grinding, b.grind_wins, b.cosmetics, p.created_at, p.current_match_id, r.rating
           from public.bots b
           join public.profiles p on p.id = b.profile_id
           left join public.bot_ratings r on r.bot_id = b.profile_id::text
          order by p.created_at, b.profile_id",
    )
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(to_bot_row).collect()
}

fn small(value: i64, what: &str) -> Result<i16, StoreError> {
    i16::try_from(value).map_err(|_| StoreError::from(format!("a bot's {what} is out of range: {value}")))
}

/// R1454: the bot's profile, its row and its R610 rating. Migration 0032's
/// `profile_needs_auth_user` lets a profile of kind `bot` in with no auth user, and 0028's
/// `profiles_assign_default_username` names it the lowest free `Player#n`, as any new profile.
pub async fn bots_create(t: &mut PgTx<'_>, bot: &NewBot) -> Result<(), StoreError> {
    run_as(t, None).await?;
    sqlx::query(concat!(
        "insert into public.profiles (id, kind, status, rating, rating_deviation, rating_volatility,
                                      created_at)
           values ($1::uuid, 'bot', 'active', $2::double precision, $3::double precision,
                   $4::double precision, ",
        ts!("$5"),
        ")"
    ))
    .bind(bot.profile_id.as_str())
    .bind(RATING_START)
    .bind(RATING_DEVIATION_START)
    .bind(RATING_VOLATILITY_START)
    .bind(bot.created_at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    sqlx::query(
        "insert into public.bots (profile_id, strength, speed, persona, active_from_hour, active_hours,
                                  decks, decks_catalog_version)
           values ($1::uuid, $2::smallint, $3::double precision, $4::text, $5::smallint, $6::smallint,
                   $7::jsonb, $8::text)",
    )
    .bind(bot.profile_id.as_str())
    .bind(small(
        i64::try_from(bot.strength).unwrap_or(i64::MAX),
        "strength",
    )?)
    .bind(bot.speed)
    .bind(bot.persona.as_str())
    .bind(small(i64::from(bot.active_from_hour), "hour")?)
    .bind(small(i64::from(bot.active_hours), "hours")?)
    .bind(json(&bot.decks)?)
    .bind(bot.decks_catalog_version.as_deref())
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    sqlx::query(
        "insert into public.bot_ratings (bot_id, rating, deviation, volatility, games, updated_at)
           values ($1::text, $2::float8, $3::float8, $4::float8, 0, now())
           on conflict (bot_id) do update set
             rating = excluded.rating, deviation = excluded.deviation,
             volatility = excluded.volatility, games = excluded.games, updated_at = excluded.updated_at",
    )
    .bind(bot.profile_id.as_str())
    .bind(bot.seed_rating)
    .bind(RATING_DEVIATION_START)
    .bind(RATING_VOLATILITY_START)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub async fn bots_set_retired(t: &mut PgTx<'_>, profile_id: &str, at: Option<i64>) -> Result<(), StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query(concat!(
        "update public.bots set retired_at = ",
        nullable_ts!("$2"),
        " where profile_id = $1::uuid"
    ))
    .bind(profile_id)
    .bind(at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    if done.rows_affected() == 0 {
        return Err(StoreError::from(format!("no bot {profile_id}")));
    }
    Ok(())
}

pub async fn bots_set_decks(
    t: &mut PgTx<'_>,
    profile_id: &str,
    decks: &[BotDeck],
    catalog_version: &str,
) -> Result<(), StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query(
        "update public.bots set decks = $2::jsonb, decks_catalog_version = $3::text
          where profile_id = $1::uuid",
    )
    .bind(profile_id)
    .bind(json(decks)?)
    .bind(catalog_version)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    if done.rows_affected() == 0 {
        return Err(StoreError::from(format!("no bot {profile_id}")));
    }
    Ok(())
}

pub async fn bots_set_cosmetics(
    t: &mut PgTx<'_>,
    profile_id: &str,
    cosmetics: &BotCosmetics,
) -> Result<(), StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query("update public.bots set cosmetics = $2::jsonb where profile_id = $1::uuid")
        .bind(profile_id)
        .bind(json(cosmetics)?)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    if done.rows_affected() == 0 {
        return Err(StoreError::from(format!("no bot {profile_id}")));
    }
    Ok(())
}

pub async fn bots_set_trio(
    t: &mut PgTx<'_>,
    profile_id: &str,
    trio: &FrozenTrio,
    catalog_version: &str,
) -> Result<(), StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query(
        "update public.bots set trio = $2::jsonb, trio_catalog_version = $3::text
          where profile_id = $1::uuid",
    )
    .bind(profile_id)
    .bind(json(trio)?)
    .bind(catalog_version)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    if done.rows_affected() == 0 {
        return Err(StoreError::from(format!("no bot {profile_id}")));
    }
    Ok(())
}

/// R1453, R641000: locks the bot's profile and `bots` rows (every claim of a bot takes them first) and
/// answers whether it is free: in no match, not retired and a side of no series that is not over. The series
/// check is its own statement, run after the lock: each READ COMMITTED statement reads a fresh snapshot, so
/// it sees what an earlier claim committed (`tickets_claim_for_bot_series` leaves the bot in no match).
async fn lock_free_bot(t: &mut PgTx<'_>, bot_id: &str) -> Result<bool, StoreError> {
    let bot = sqlx::query(
        "select p.id
           from public.profiles p
           join public.bots b on b.profile_id = p.id
          where p.id = $1::uuid and p.kind = 'bot' and p.current_match_id is null
            and b.retired_at is null
          for update of p, b",
    )
    .bind(bot_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    if bot.is_none() {
        return Ok(false);
    }
    let held = sqlx::query(
        "select 1 as held
           from public.series
          where status <> 'over' and (p1_profile_id = $1::uuid or p2_profile_id = $1::uuid)
          limit 1",
    )
    .bind(bot_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(held.is_none())
}

/// R1453: the bot is locked first (free: `lock_free_bot`), then the ticket claimed
/// (`status = 'queued'`, as `app.claim_ticket_pair` claims), and only then is the `open` skeleton
/// written, the ticket linked to it and the bot put in it. The skeleton's bot deck is empty until
/// `matches_create` writes the match.
pub async fn tickets_claim_for_bot(
    t: &mut PgTx<'_>,
    ticket_id: &str,
    bot_id: &str,
    match_id: &str,
    at: i64,
) -> Result<bool, StoreError> {
    run_as(t, None).await?;
    if !lock_free_bot(t, bot_id).await? {
        return Ok(false);
    }
    let claimed = sqlx::query(concat!(
        "update public.tickets set status = 'claimed', claimed_at = ",
        ts!("$2"),
        "
          where id = $1::uuid and status = 'queued'
          returning id"
    ))
    .bind(ticket_id)
    .bind(at)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    if claimed.is_none() {
        return Ok(false);
    }
    sqlx::query(concat!(
        "insert into public.matches (
             id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck, catalog_version,
             ceiling_at, created_at)
           select $3::uuid, 'open', '', a.profile_id, $2::uuid, a.frozen_deck, '[]'::jsonb,
                  a.catalog_version, ",
        ts!("$4"),
        ", ",
        ts!("$4"),
        "
             from public.tickets a
            where a.id = $1::uuid"
    ))
    .bind(ticket_id)
    .bind(bot_id)
    .bind(match_id)
    .bind(at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    sqlx::query("update public.tickets set match_id = $2::uuid where id = $1::uuid")
        .bind(ticket_id)
        .bind(match_id)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    sqlx::query("update public.profiles set current_match_id = $2::uuid where id = $1::uuid")
        .bind(bot_id)
        .bind(match_id)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(true)
}

/// R641000: as `tickets_claim_for_bot`, but the bot is put in no match. The series the caller
/// writes in this transaction is what holds it: `lock_free_bot`'s series check keeps every other
/// claim off it, and with no `current_match_id` to find it by, `bots_release_stale_starts` leaves
/// game 1's skeleton alone through the pick phase.
pub async fn tickets_claim_for_bot_series(
    t: &mut PgTx<'_>,
    ticket_id: &str,
    bot_id: &str,
    match_id: &str,
    at: i64,
) -> Result<bool, StoreError> {
    run_as(t, None).await?;
    if !lock_free_bot(t, bot_id).await? {
        return Ok(false);
    }
    let claimed = sqlx::query(concat!(
        "update public.tickets set status = 'claimed', claimed_at = ",
        ts!("$2"),
        "
          where id = $1::uuid and status = 'queued' and mode = 'bo3'
          returning id"
    ))
    .bind(ticket_id)
    .bind(at)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    if claimed.is_none() {
        return Ok(false);
    }
    sqlx::query(concat!(
        "insert into public.matches (
             id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck, catalog_version,
             ceiling_at, created_at)
           select $3::uuid, 'open', '', a.profile_id, $2::uuid, a.frozen_deck, '[]'::jsonb,
                  a.catalog_version, ",
        ts!("$4"),
        ", ",
        ts!("$4"),
        "
             from public.tickets a
            where a.id = $1::uuid"
    ))
    .bind(ticket_id)
    .bind(bot_id)
    .bind(match_id)
    .bind(at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    sqlx::query("update public.tickets set match_id = $2::uuid where id = $1::uuid")
        .bind(ticket_id)
        .bind(match_id)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(true)
}

/// R639000: the bot is locked (free: `lock_free_bot`) before the `open` skeleton is written. Its
/// decks are empty until `matches_create` writes the match; a start that never comes leaves it to
/// `bots_release_stale_starts`.
pub async fn bots_claim_for_rematch(
    t: &mut PgTx<'_>,
    bot_id: &str,
    players: &(String, String),
    match_id: &str,
    catalog_version: &str,
    at: i64,
) -> Result<bool, StoreError> {
    run_as(t, None).await?;
    if !lock_free_bot(t, bot_id).await? {
        return Ok(false);
    }
    sqlx::query(concat!(
        "insert into public.matches (
             id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck, catalog_version,
             ceiling_at, created_at)
           values ($1::uuid, 'open', '', $2::uuid, $3::uuid, '[]'::jsonb, '[]'::jsonb, $4::text, ",
        ts!("$5"),
        ", ",
        ts!("$5"),
        ")"
    ))
    .bind(match_id)
    .bind(players.0.as_str())
    .bind(players.1.as_str())
    .bind(catalog_version)
    .bind(at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    sqlx::query("update public.profiles set current_match_id = $2::uuid where id = $1::uuid")
        .bind(bot_id)
        .bind(match_id)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(true)
}

pub async fn bots_set_grinding(t: &mut PgTx<'_>, profile_id: &str, grinding: bool) -> Result<(), StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query(
        "update public.bots set grinding = $2::boolean, grind_wins = 0 where profile_id = $1::uuid",
    )
    .bind(profile_id)
    .bind(grinding)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    if done.rows_affected() == 0 {
        return Err(StoreError::from(format!("no bot {profile_id}")));
    }
    Ok(())
}

/// R1555000: one statement on the grinding bot's row alone, so the result's transaction locks no
/// other bot. The id is compared as text, as `bot_ratings.bot_id` holds it, so a side that is no
/// bot's uuid matches no row rather than failing the cast.
pub async fn bots_note_grind_win(
    t: &mut PgTx<'_>,
    bot_id: &str,
    step_games: i32,
    strongest: usize,
) -> Result<(), StoreError> {
    run_as(t, None).await?;
    sqlx::query(
        "update public.bots
            set strength = case when grind_wins + 1 >= $2::integer
                                then least(strength + 1, $3::smallint)::smallint else strength end,
                grind_wins = case when grind_wins + 1 >= $2::integer then 0 else grind_wins + 1 end
          where profile_id::text = $1::text and grinding",
    )
    .bind(bot_id)
    .bind(step_games)
    .bind(small(i64::try_from(strongest).unwrap_or(i64::MAX), "strength")?)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

/// R1555000: both bots are locked (free: `lock_free_bot`) in id order, so two claims that share a
/// bot cannot deadlock, before the `open` skeleton is written; a start that never comes leaves it
/// to `bots_release_stale_starts`.
pub async fn bots_claim_pair(
    t: &mut PgTx<'_>,
    players: &(String, String),
    match_id: &str,
    catalog_version: &str,
    at: i64,
) -> Result<bool, StoreError> {
    run_as(t, None).await?;
    if players.0 == players.1 {
        return Ok(false);
    }
    let (first, second) = if players.0 < players.1 {
        (&players.0, &players.1)
    } else {
        (&players.1, &players.0)
    };
    if !lock_free_bot(t, first).await? || !lock_free_bot(t, second).await? {
        return Ok(false);
    }
    sqlx::query(concat!(
        "insert into public.matches (
             id, status, seed, p1_profile_id, p2_profile_id, p1_deck, p2_deck, catalog_version,
             ceiling_at, created_at)
           values ($1::uuid, 'open', '', $2::uuid, $3::uuid, '[]'::jsonb, '[]'::jsonb, $4::text, ",
        ts!("$5"),
        ", ",
        ts!("$5"),
        ")"
    ))
    .bind(match_id)
    .bind(players.0.as_str())
    .bind(players.1.as_str())
    .bind(catalog_version)
    .bind(at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    sqlx::query("update public.profiles set current_match_id = $2::uuid where id in ($1::uuid, $3::uuid)")
        .bind(players.0.as_str())
        .bind(match_id)
        .bind(players.1.as_str())
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(true)
}

/// R1454: `profiles.current_match_id` and `tickets.match_id` are `on delete set null`, so deleting a skeleton
/// lets the bot and the ticket's owner go, and the ticket stays claimed. A room is an `open` row too, but
/// never a bot's. R1555000: a skeleton two bots were claimed into answers both. Each skeleton is locked once,
/// so a second bot's row never meets one this statement already deleted, and the bots are read as they stood.
pub async fn bots_release_stale_starts(t: &mut PgTx<'_>, before: i64) -> Result<Vec<String>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(concat!(
        "with stale as (
           select m.id
             from public.matches m
            where m.status = 'open' and m.created_at < ",
        ts!("$1"),
        "
              and exists (select 1 from public.profiles p where p.current_match_id = m.id and p.kind = 'bot')
            for update of m),
         gone as (
           delete from public.matches m
            using stale
            where m.id = stale.id and m.status = 'open'
            returning m.id)
         select p.id as bot_id
           from public.profiles p
           join gone on p.current_match_id = gone.id
          where p.kind = 'bot'"
    ))
    .bind(before)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(|row| uuid_text(row, "bot_id")).collect()
}

pub async fn results_recent_opponents(
    t: &mut PgTx<'_>,
    profile_id: &str,
    limit: usize,
) -> Result<Vec<String>, StoreError> {
    run_as(t, Some(profile_id)).await?;
    let rows = sqlx::query(
        "select case when p1_profile_id = $1::uuid then p2_profile_id else p1_profile_id end
                  as opponent
           from public.results
          where p1_profile_id = $1::uuid or p2_profile_id = $1::uuid
          order by ended_at desc, match_id desc
          limit $2::bigint",
    )
    .bind(profile_id)
    .bind(i64::try_from(limit).unwrap_or(i64::MAX))
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| Ok(uuid_text_or_null(row, "opponent")?.unwrap_or_default()))
        .collect()
}

/// R1454: the distinct people (no bot, no deleted account) on a result that ended at or after `since`.
pub async fn results_count_active_humans(t: &mut PgTx<'_>, since: i64) -> Result<i64, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select count(distinct p.id) as active
           from public.results r
           join public.profiles p on p.id in (r.p1_profile_id, r.p2_profile_id)
          where p.kind = 'human' and r.ended_at >= ",
        ts!("$1")
    ))
    .bind(since)
    .fetch_one(&mut **t)
    .await
    .map_err(db_error)?;
    get(&row, "active")
}

pub async fn results_count_since(t: &mut PgTx<'_>, since: i64) -> Result<i64, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select count(*) as games from public.results where ended_at >= ",
        ts!("$1")
    ))
    .bind(since)
    .fetch_one(&mut **t)
    .await
    .map_err(db_error)?;
    get(&row, "games")
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

pub async fn ranked_bot_games_since(t: &mut PgTx<'_>, since: i64) -> Result<Vec<RatedGameRow>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(concat!(
        "select ",
        rated_game_columns!(),
        " from public.rated_games where (p1_bot_id is not null or p2_bot_id is not null) and ended_at >= ",
        ts!("$1"),
        " order by ended_at, id"
    ))
    .bind(since)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| RatedGameDbRow::read(row).and_then(to_rated_game))
        .collect()
}

// Player statistics on the account (SPEC §9.11, R639, R654)

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

// Consent on the account (R757000, R757001, migration 0034). No `app.*` function: each table's
// one rule is "the first row stands", which its primary key and `on conflict do nothing` say.

pub async fn consent_get(t: &mut PgTx<'_>, profile_id: &str) -> Result<ConsentRow, StoreError> {
    let mut consent = ConsentRow {
        profile_id: profile_id.to_string(),
        age_checked_at: None,
        age_min: None,
        terms: Vec::new(),
    };
    if !is_uuid(profile_id) {
        return Ok(consent);
    }
    run_as(t, Some(profile_id)).await?;
    let age = sqlx::query("select min_age, passed_at from public.age_checks where profile_id = $1::uuid")
        .bind(profile_id)
        .fetch_optional(&mut **t)
        .await
        .map_err(db_error)?;
    if let Some(row) = age {
        consent.age_min = Some(i32::from(get::<i16>(&row, "min_age")?));
        consent.age_checked_at = Some(ms_of(get(&row, "passed_at")?));
    }
    let rows = sqlx::query(
        "select terms_version, source, accepted_at from public.terms_acceptances
          where profile_id = $1::uuid
          order by accepted_at, terms_version",
    )
    .bind(profile_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    for row in rows {
        consent.terms.push(TermsAcceptanceRow {
            version: get(&row, "terms_version")?,
            source: get::<String>(&row, "source")?.parse::<TermsSource>()?,
            accepted_at: ms_of(get(&row, "accepted_at")?),
        });
    }
    Ok(consent)
}

pub async fn consent_record_age(
    t: &mut PgTx<'_>,
    profile_id: &str,
    min_age: i32,
    at: i64,
) -> Result<(), StoreError> {
    if !is_uuid(profile_id) {
        return Ok(());
    }
    let min_age = i16::try_from(min_age).map_err(|_| StoreError::Other(format!("min_age {min_age}")))?;
    run_as(t, Some(profile_id)).await?;
    sqlx::query(concat!(
        "insert into public.age_checks (profile_id, min_age, passed_at)
         values ($1::uuid, $2::smallint, ",
        ts!("$3"),
        ")
         on conflict (profile_id) do nothing"
    ))
    .bind(profile_id)
    .bind(min_age)
    .bind(at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub async fn consent_record_terms(
    t: &mut PgTx<'_>,
    profile_id: &str,
    version: &str,
    source: TermsSource,
    at: i64,
) -> Result<(), StoreError> {
    if !is_uuid(profile_id) {
        return Ok(());
    }
    run_as(t, Some(profile_id)).await?;
    sqlx::query(concat!(
        "insert into public.terms_acceptances (profile_id, terms_version, source, accepted_at)
         values ($1::uuid, $2::text, $3::text, ",
        ts!("$4"),
        ")
         on conflict (profile_id, terms_version) do nothing"
    ))
    .bind(profile_id)
    .bind(version)
    .bind(source.as_str())
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
    // The SQL casts `$2::int`/`$3::int`, so the binds are `int4`.
    let limit = i32::try_from(options.limit).unwrap_or(i32::MAX);
    let offset = i32::try_from(options.offset).unwrap_or(i32::MAX);
    // R1436: the search matches the username by the key it clashes on (R1434), the NFKC case fold
    // computed here in Rust, so no Postgres collation decides; `Max#3` finds `max#3`.
    let term = term.map(username_key);
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select ps.profile_id, p.username_base, p.username_tag, ps.stats, ps.updated_at
         from public.player_stats ps
         join public.profiles p on p.id = ps.profile_id
         where ps.is_private = false
           and ($1::text is null
                or position($1::text in (p.username_key || coalesce('#' || p.username_tag::text, ''))) > 0)
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
                &render_username(
                    &get::<String>(row, "username_base")?,
                    get::<Option<i32>>(row, "username_tag")?.map(i64::from),
                ),
                &stats_col(row, "stats"),
                ms_of(get(row, "updated_at")?),
            ))
        })
        .collect()
}

// Social: friends, presence, challenges, chat (#937, #938, #941)
// SPEC S7 over migration 0038's tables. No `app.*` function: each method is one or two plain statements on
// its own table, and the schema's keys and checks are the rule. An id that is not a UUID names no row, as
// everywhere here.

use crate::db::store::{
    BlockRow, ChallengeRow, ChallengeState, ChatCursor, ChatMessageRow, ChatReportListing, ChatReportRow,
    FriendRequestRow, FriendshipRow,
};

/// `public.challenges`' columns, in `challenge_of`'s order.
macro_rules! challenge_columns {
    () => {
        "id, from_profile, to_profile, room_code, mode, created_at, expires_at, state"
    };
}

/// `public.chat_messages`' columns, in `chat_message_of`'s order.
macro_rules! chat_message_columns {
    () => {
        "id, from_profile, to_profile, body, created_at, read_at"
    };
}

fn friend_request_of(row: &PgRow) -> Result<FriendRequestRow, StoreError> {
    Ok(FriendRequestRow {
        id: uuid_text(row, "id")?,
        from_profile: uuid_text(row, "from_profile")?,
        to_profile: uuid_text(row, "to_profile")?,
        created_at: ms_of(get(row, "created_at")?),
    })
}

fn friendship_of(row: &PgRow) -> Result<FriendshipRow, StoreError> {
    Ok(FriendshipRow {
        a: uuid_text(row, "a")?,
        b: uuid_text(row, "b")?,
        since: ms_of(get(row, "since")?),
    })
}

fn block_of(row: &PgRow) -> Result<BlockRow, StoreError> {
    Ok(BlockRow {
        blocker: uuid_text(row, "blocker")?,
        blocked: uuid_text(row, "blocked")?,
        created_at: ms_of(get(row, "created_at")?),
    })
}

fn challenge_of(row: &PgRow) -> Result<ChallengeRow, StoreError> {
    Ok(ChallengeRow {
        id: uuid_text(row, "id")?,
        from_profile: uuid_text(row, "from_profile")?,
        to_profile: uuid_text(row, "to_profile")?,
        room_code: get(row, "room_code")?,
        mode: queue_mode_of(&get::<String>(row, "mode")?)?,
        created_at: ms_of(get(row, "created_at")?),
        expires_at: ms_of(get(row, "expires_at")?),
        state: get::<String>(row, "state")?.parse::<ChallengeState>()?,
    })
}

/// A `returning` clause's rows in the port's order: oldest first, the id breaking a tie.
fn challenges_of(rows: &[PgRow]) -> Result<Vec<ChallengeRow>, StoreError> {
    let mut challenges = rows.iter().map(challenge_of).collect::<Result<Vec<_>, _>>()?;
    challenges.sort_by(|a, b| (a.created_at, &a.id).cmp(&(b.created_at, &b.id)));
    Ok(challenges)
}

fn chat_message_of(row: &PgRow) -> Result<ChatMessageRow, StoreError> {
    Ok(ChatMessageRow {
        id: uuid_text(row, "id")?,
        from_profile: uuid_text(row, "from_profile")?,
        to_profile: uuid_text(row, "to_profile")?,
        body: get(row, "body")?,
        created_at: ms_of(get(row, "created_at")?),
        read_at: ms_or_null(get(row, "read_at")?),
    })
}

// friends (B43–B45)

pub async fn friends_requests_of(
    t: &mut PgTx<'_>,
    profile_id: &str,
) -> Result<Vec<FriendRequestRow>, StoreError> {
    if !is_uuid(profile_id) {
        return Ok(Vec::new());
    }
    run_as(t, Some(profile_id)).await?;
    let rows = sqlx::query(
        "select id, from_profile, to_profile, created_at from public.friend_requests
          where from_profile = $1::uuid or to_profile = $1::uuid
          order by created_at, id",
    )
    .bind(profile_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(friend_request_of).collect()
}

pub async fn friends_request_get(
    t: &mut PgTx<'_>,
    request_id: &str,
) -> Result<Option<FriendRequestRow>, StoreError> {
    if !is_uuid(request_id) {
        return Ok(None);
    }
    run_as(t, None).await?;
    let row = sqlx::query(
        "select id, from_profile, to_profile, created_at from public.friend_requests where id = $1::uuid",
    )
    .bind(request_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.as_ref().map(friend_request_of).transpose()
}

pub async fn friends_request_between(
    t: &mut PgTx<'_>,
    from: &str,
    to: &str,
) -> Result<Option<FriendRequestRow>, StoreError> {
    if !is_uuid(from) || !is_uuid(to) {
        return Ok(None);
    }
    run_as(t, Some(from)).await?;
    let row = sqlx::query(
        "select id, from_profile, to_profile, created_at from public.friend_requests
          where from_profile = $1::uuid and to_profile = $2::uuid",
    )
    .bind(from)
    .bind(to)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.as_ref().map(friend_request_of).transpose()
}

/// The unique key on `(from_profile, to_profile)` is inferred in `on conflict`, so a second request
/// of the pair answers false instead of raising.
pub async fn friends_request_create(t: &mut PgTx<'_>, row: &FriendRequestRow) -> Result<bool, StoreError> {
    run_as(t, Some(row.from_profile.as_str())).await?;
    let done = sqlx::query(concat!(
        "insert into public.friend_requests (id, from_profile, to_profile, created_at)
         values ($1::uuid, $2::uuid, $3::uuid, ",
        ts!("$4"),
        ")
         on conflict (from_profile, to_profile) do nothing"
    ))
    .bind(row.id.as_str())
    .bind(row.from_profile.as_str())
    .bind(row.to_profile.as_str())
    .bind(row.created_at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(done.rows_affected() == 1)
}

pub async fn friends_request_delete(t: &mut PgTx<'_>, request_id: &str) -> Result<bool, StoreError> {
    if !is_uuid(request_id) {
        return Ok(false);
    }
    run_as(t, None).await?;
    let done = sqlx::query("delete from public.friend_requests where id = $1::uuid")
        .bind(request_id)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(done.rows_affected() > 0)
}

pub async fn friends_requests_delete_between(t: &mut PgTx<'_>, x: &str, y: &str) -> Result<i64, StoreError> {
    if !is_uuid(x) || !is_uuid(y) {
        return Ok(0);
    }
    run_as(t, Some(x)).await?;
    let done = sqlx::query(
        "delete from public.friend_requests
          where (from_profile = $1::uuid and to_profile = $2::uuid)
             or (from_profile = $2::uuid and to_profile = $1::uuid)",
    )
    .bind(x)
    .bind(y)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(i64::try_from(done.rows_affected()).unwrap_or(i64::MAX))
}

pub async fn friends_of(t: &mut PgTx<'_>, profile_id: &str) -> Result<Vec<FriendshipRow>, StoreError> {
    if !is_uuid(profile_id) {
        return Ok(Vec::new());
    }
    run_as(t, Some(profile_id)).await?;
    let rows = sqlx::query(
        "select a, b, since from public.friendships
          where a = $1::uuid or b = $1::uuid
          order by since, a, b",
    )
    .bind(profile_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(friendship_of).collect()
}

pub async fn friends_count(t: &mut PgTx<'_>, profile_id: &str) -> Result<i64, StoreError> {
    if !is_uuid(profile_id) {
        return Ok(0);
    }
    run_as(t, Some(profile_id)).await?;
    let row = sqlx::query("select count(*) as n from public.friendships where a = $1::uuid or b = $1::uuid")
        .bind(profile_id)
        .fetch_one(&mut **t)
        .await
        .map_err(db_error)?;
    get(&row, "n")
}

pub async fn friends_get(t: &mut PgTx<'_>, x: &str, y: &str) -> Result<Option<FriendshipRow>, StoreError> {
    if !is_uuid(x) || !is_uuid(y) {
        return Ok(None);
    }
    run_as(t, Some(x)).await?;
    let row = sqlx::query(
        "select a, b, since from public.friendships
          where a = least($1::uuid, $2::uuid) and b = greatest($1::uuid, $2::uuid)",
    )
    .bind(x)
    .bind(y)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.as_ref().map(friendship_of).transpose()
}

/// The pair is ordered by the database (`least`, `greatest`), so `a < b` holds whatever text form
/// the ids arrived in; a friendship with oneself fails that check and raises.
pub async fn friends_make(t: &mut PgTx<'_>, x: &str, y: &str, at: i64) -> Result<FriendshipRow, StoreError> {
    run_as(t, Some(x)).await?;
    sqlx::query(concat!(
        "insert into public.friendships (a, b, since)
         values (least($1::uuid, $2::uuid), greatest($1::uuid, $2::uuid), ",
        ts!("$3"),
        ")
         on conflict (a, b) do nothing"
    ))
    .bind(x)
    .bind(y)
    .bind(at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    sqlx::query(
        "delete from public.friend_requests
          where (from_profile = $1::uuid and to_profile = $2::uuid)
             or (from_profile = $2::uuid and to_profile = $1::uuid)",
    )
    .bind(x)
    .bind(y)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    let row = sqlx::query(
        "select a, b, since from public.friendships
          where a = least($1::uuid, $2::uuid) and b = greatest($1::uuid, $2::uuid)",
    )
    .bind(x)
    .bind(y)
    .fetch_one(&mut **t)
    .await
    .map_err(db_error)?;
    friendship_of(&row)
}

pub async fn friends_remove(t: &mut PgTx<'_>, x: &str, y: &str) -> Result<bool, StoreError> {
    if !is_uuid(x) || !is_uuid(y) {
        return Ok(false);
    }
    run_as(t, Some(x)).await?;
    let done = sqlx::query(
        "delete from public.friendships
          where a = least($1::uuid, $2::uuid) and b = greatest($1::uuid, $2::uuid)",
    )
    .bind(x)
    .bind(y)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(done.rows_affected() > 0)
}

// blocks (B46)

pub async fn blocks_of(t: &mut PgTx<'_>, blocker: &str) -> Result<Vec<BlockRow>, StoreError> {
    if !is_uuid(blocker) {
        return Ok(Vec::new());
    }
    run_as(t, Some(blocker)).await?;
    let rows = sqlx::query(
        "select blocker, blocked, created_at from public.blocks
          where blocker = $1::uuid
          order by created_at, blocked",
    )
    .bind(blocker)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(block_of).collect()
}

pub async fn blocks_between(t: &mut PgTx<'_>, x: &str, y: &str) -> Result<bool, StoreError> {
    if !is_uuid(x) || !is_uuid(y) {
        return Ok(false);
    }
    run_as(t, Some(x)).await?;
    let row = sqlx::query(
        "select exists (
           select 1 from public.blocks
            where (blocker = $1::uuid and blocked = $2::uuid)
               or (blocker = $2::uuid and blocked = $1::uuid)) as blocked",
    )
    .bind(x)
    .bind(y)
    .fetch_one(&mut **t)
    .await
    .map_err(db_error)?;
    get(&row, "blocked")
}

pub async fn blocks_add(t: &mut PgTx<'_>, blocker: &str, blocked: &str, at: i64) -> Result<(), StoreError> {
    run_as(t, Some(blocker)).await?;
    sqlx::query(concat!(
        "insert into public.blocks (blocker, blocked, created_at)
         values ($1::uuid, $2::uuid, ",
        ts!("$3"),
        ")
         on conflict (blocker, blocked) do nothing"
    ))
    .bind(blocker)
    .bind(blocked)
    .bind(at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub async fn blocks_remove(t: &mut PgTx<'_>, blocker: &str, blocked: &str) -> Result<bool, StoreError> {
    if !is_uuid(blocker) || !is_uuid(blocked) {
        return Ok(false);
    }
    run_as(t, Some(blocker)).await?;
    let done = sqlx::query("delete from public.blocks where blocker = $1::uuid and blocked = $2::uuid")
        .bind(blocker)
        .bind(blocked)
        .execute(&mut **t)
        .await
        .map_err(db_error)?;
    Ok(done.rows_affected() > 0)
}

// presence (B47)

pub async fn presence_touch(t: &mut PgTx<'_>, profile_id: &str, at: i64) -> Result<(), StoreError> {
    run_as(t, Some(profile_id)).await?;
    sqlx::query(concat!(
        "insert into public.presence (profile_id, last_seen)
         values ($1::uuid, ",
        ts!("$2"),
        ")
         on conflict (profile_id)
         do update set last_seen = greatest(public.presence.last_seen, excluded.last_seen)"
    ))
    .bind(profile_id)
    .bind(at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

/// In the order the ids were named, as the fake answers.
pub async fn presence_last_seen(
    t: &mut PgTx<'_>,
    profile_ids: &[String],
) -> Result<IndexMap<String, i64>, StoreError> {
    let ids: Vec<String> = profile_ids.iter().filter(|id| is_uuid(id)).cloned().collect();
    if ids.is_empty() {
        return Ok(IndexMap::new());
    }
    run_as(t, None).await?;
    let rows =
        sqlx::query("select profile_id, last_seen from public.presence where profile_id = any($1::uuid[])")
            .bind(ids.clone())
            .fetch_all(&mut **t)
            .await
            .map_err(db_error)?;
    let mut seen: IndexMap<String, i64> = IndexMap::new();
    for row in &rows {
        seen.insert(uuid_text(row, "profile_id")?, ms_of(get(row, "last_seen")?));
    }
    Ok(ids
        .iter()
        .filter_map(|id| seen.get(id).map(|at| (id.clone(), *at)))
        .collect())
}

/// One statement over `series_active_idx`'s rows; in the order the ids were named, as the fake
/// answers.
pub async fn presence_in_series(t: &mut PgTx<'_>, profile_ids: &[String]) -> Result<Vec<String>, StoreError> {
    let ids: Vec<String> = profile_ids.iter().filter(|id| is_uuid(id)).cloned().collect();
    if ids.is_empty() {
        return Ok(Vec::new());
    }
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select p1_profile_id, p2_profile_id from public.series
          where status <> 'over'
            and (p1_profile_id = any($1::uuid[]) or p2_profile_id = any($1::uuid[]))",
    )
    .bind(ids.clone())
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    let mut held: Vec<String> = Vec::new();
    for row in &rows {
        held.extend(uuid_text_or_null(row, "p1_profile_id")?);
        held.extend(uuid_text_or_null(row, "p2_profile_id")?);
    }
    Ok(ids.into_iter().filter(|id| held.contains(id)).collect())
}

/// One statement: each named profile's newest live co-op match (`distinct on`, ordered as
/// `duos_matches_live_for` orders), in the order the ids were named.
pub async fn presence_in_duos(
    t: &mut PgTx<'_>,
    profile_ids: &[String],
) -> Result<IndexMap<String, String>, StoreError> {
    let ids: Vec<String> = profile_ids.iter().filter(|id| is_uuid(id)).cloned().collect();
    if ids.is_empty() {
        return Ok(IndexMap::new());
    }
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select distinct on (p.id) p.id as profile_id, d.id as match_id
           from unnest($1::uuid[]) as p(id)
           join public.duos_matches d on d.ended_at is null and (d.a1 = p.id or d.a2 = p.id)
          order by p.id, d.created_at desc, d.id",
    )
    .bind(ids.clone())
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    let mut live: IndexMap<String, String> = IndexMap::new();
    for row in &rows {
        live.insert(uuid_text(row, "profile_id")?, uuid_text(row, "match_id")?);
    }
    Ok(ids
        .iter()
        .filter_map(|id| live.get(id).map(|match_id| (id.clone(), match_id.clone())))
        .collect())
}

/// One statement: each named profile's in-match flag joined to its match, with B50's store checks
/// (`social::watch::watchable_in`'s, for many friends at once); in the order the ids were named.
pub async fn presence_watchable(
    t: &mut PgTx<'_>,
    viewer: &str,
    profile_ids: &[String],
) -> Result<Vec<String>, StoreError> {
    let ids: Vec<String> = profile_ids.iter().filter(|id| is_uuid(id)).cloned().collect();
    if ids.is_empty() || !is_uuid(viewer) {
        return Ok(Vec::new());
    }
    run_as(t, Some(viewer)).await?;
    let rows = sqlx::query(
        "select p.id from public.profiles p
           join public.matches m on m.id = p.current_match_id
          where p.id = any($2::uuid[]) and p.id <> $1::uuid
            and m.status = 'live'
            and p.id in (m.p1_profile_id, m.p2_profile_id)
            and $1::uuid is distinct from m.p1_profile_id
            and $1::uuid is distinct from m.p2_profile_id
            and exists (select 1 from public.friendships f
                         where f.a = least($1::uuid, p.id) and f.b = greatest($1::uuid, p.id))
            and not exists (select 1 from public.blocks k
                             where (k.blocker = $1::uuid and k.blocked = p.id)
                                or (k.blocker = p.id and k.blocked = $1::uuid))",
    )
    .bind(viewer)
    .bind(ids.clone())
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    let watchable = rows
        .iter()
        .map(|row| uuid_text(row, "id"))
        .collect::<Result<Vec<String>, StoreError>>()?;
    Ok(ids.into_iter().filter(|id| watchable.contains(id)).collect())
}

// profiles (B46)

pub async fn profiles_find(t: &mut PgTx<'_>, profile_id: &str) -> Result<Option<Profile>, StoreError> {
    if !is_uuid(profile_id) {
        return Ok(None);
    }
    profiles_get_by_id(t, profile_id).await
}

// challenges (B48, B49)

/// Two statements in the caller's transaction: the pair's lapsed open row (if any) is marked
/// expired, then the insert, whose `on conflict` names `challenges_open_pair_key`'s columns and
/// predicate, so a live open challenge of the pair answers false instead of raising.
pub async fn challenges_create(t: &mut PgTx<'_>, row: &ChallengeRow, now: i64) -> Result<bool, StoreError> {
    run_as(t, Some(row.from_profile.as_str())).await?;
    sqlx::query(concat!(
        "update public.challenges set state = 'expired'
          where from_profile = $1::uuid and to_profile = $2::uuid and state = 'open'
            and expires_at <= ",
        ts!("$3")
    ))
    .bind(row.from_profile.as_str())
    .bind(row.to_profile.as_str())
    .bind(now)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    let done = sqlx::query(concat!(
        "insert into public.challenges
           (id, from_profile, to_profile, room_code, mode, created_at, expires_at, state)
         values ($1::uuid, $2::uuid, $3::uuid, $4::text, $5::text, ",
        ts!("$6"),
        ", ",
        ts!("$7"),
        ", $8::text)
         on conflict (from_profile, to_profile) where state = 'open' do nothing"
    ))
    .bind(row.id.as_str())
    .bind(row.from_profile.as_str())
    .bind(row.to_profile.as_str())
    .bind(row.room_code.as_str())
    .bind(literal(&row.mode)?)
    .bind(row.created_at)
    .bind(row.expires_at)
    .bind(literal(&row.state)?)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(done.rows_affected() == 1)
}

pub async fn challenges_get(
    t: &mut PgTx<'_>,
    challenge_id: &str,
) -> Result<Option<ChallengeRow>, StoreError> {
    if !is_uuid(challenge_id) {
        return Ok(None);
    }
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select ",
        challenge_columns!(),
        " from public.challenges where id = $1::uuid"
    ))
    .bind(challenge_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.as_ref().map(challenge_of).transpose()
}

pub async fn challenges_open_of(
    t: &mut PgTx<'_>,
    profile_id: &str,
    now: i64,
) -> Result<Vec<ChallengeRow>, StoreError> {
    if !is_uuid(profile_id) {
        return Ok(Vec::new());
    }
    run_as(t, Some(profile_id)).await?;
    let rows = sqlx::query(concat!(
        "select ",
        challenge_columns!(),
        " from public.challenges
          where (from_profile = $1::uuid or to_profile = $1::uuid)
            and state = 'open' and expires_at > ",
        ts!("$2"),
        "
          order by created_at, id"
    ))
    .bind(profile_id)
    .bind(now)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(challenge_of).collect()
}

pub async fn challenges_open_for_room(
    t: &mut PgTx<'_>,
    code: &str,
    now: i64,
) -> Result<Option<ChallengeRow>, StoreError> {
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select ",
        challenge_columns!(),
        " from public.challenges
          where room_code = $1::text and state = 'open' and expires_at > ",
        ts!("$2"),
        "
          order by created_at desc, id desc
          limit 1"
    ))
    .bind(code)
    .bind(now)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.as_ref().map(challenge_of).transpose()
}

pub async fn challenges_close(
    t: &mut PgTx<'_>,
    challenge_id: &str,
    state: ChallengeState,
) -> Result<Option<ChallengeRow>, StoreError> {
    if !is_uuid(challenge_id) {
        return Ok(None);
    }
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "update public.challenges set state = $2::text
          where id = $1::uuid and state = 'open'
          returning ",
        challenge_columns!()
    ))
    .bind(challenge_id)
    .bind(literal(&state)?)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.as_ref().map(challenge_of).transpose()
}

pub async fn challenges_close_between(
    t: &mut PgTx<'_>,
    x: &str,
    y: &str,
    state: ChallengeState,
) -> Result<Vec<ChallengeRow>, StoreError> {
    if !is_uuid(x) || !is_uuid(y) {
        return Ok(Vec::new());
    }
    run_as(t, Some(x)).await?;
    let rows = sqlx::query(concat!(
        "update public.challenges set state = $3::text
          where state = 'open'
            and ((from_profile = $1::uuid and to_profile = $2::uuid)
              or (from_profile = $2::uuid and to_profile = $1::uuid))
          returning ",
        challenge_columns!()
    ))
    .bind(x)
    .bind(y)
    .bind(literal(&state)?)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    challenges_of(&rows)
}

pub async fn challenges_accept_room(
    t: &mut PgTx<'_>,
    code: &str,
    guest: &str,
    now: i64,
) -> Result<bool, StoreError> {
    if !is_uuid(guest) {
        return Ok(false);
    }
    run_as(t, Some(guest)).await?;
    let done = sqlx::query(concat!(
        "update public.challenges set state = 'accepted'
          where room_code = $1::text and to_profile = $2::uuid and state = 'open' and expires_at > ",
        ts!("$3")
    ))
    .bind(code)
    .bind(guest)
    .bind(now)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(done.rows_affected() > 0)
}

pub async fn challenges_expire(t: &mut PgTx<'_>, now: i64) -> Result<Vec<ChallengeRow>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(concat!(
        "update public.challenges set state = 'expired'
          where state = 'open' and expires_at <= ",
        ts!("$1"),
        "
          returning ",
        challenge_columns!()
    ))
    .bind(now)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    challenges_of(&rows)
}

pub async fn challenges_purge(t: &mut PgTx<'_>, now: i64) -> Result<i64, StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query(concat!(
        "delete from public.challenges where expires_at <= ",
        ts!("$1")
    ))
    .bind(now)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(i64::try_from(done.rows_affected()).unwrap_or(i64::MAX))
}

// rooms

/// An unjoined room is an `open` row with no second seat (see `rooms_claim`); deleting it frees the
/// code (R110) as `matches_reap_open` does for a lapsed one. A claimed row keeps `p2_profile_id`
/// and is never touched.
pub async fn rooms_release(t: &mut PgTx<'_>, code: &str) -> Result<bool, StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query(
        "delete from public.matches
          where room_code = $1::text and status = 'open' and p2_profile_id is null",
    )
    .bind(code)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(done.rows_affected() > 0)
}

// chat (B53)

pub async fn chat_insert(t: &mut PgTx<'_>, row: &ChatMessageRow) -> Result<(), StoreError> {
    run_as(t, Some(row.from_profile.as_str())).await?;
    sqlx::query(concat!(
        "insert into public.chat_messages (id, from_profile, to_profile, body, created_at, read_at)
         values ($1::uuid, $2::uuid, $3::uuid, $4::text, ",
        ts!("$5"),
        ", ",
        nullable_ts!("$6"),
        ")"
    ))
    .bind(row.id.as_str())
    .bind(row.from_profile.as_str())
    .bind(row.to_profile.as_str())
    .bind(row.body.as_str())
    .bind(row.created_at)
    .bind(row.read_at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub async fn chat_get(t: &mut PgTx<'_>, message_id: &str) -> Result<Option<ChatMessageRow>, StoreError> {
    if !is_uuid(message_id) {
        return Ok(None);
    }
    run_as(t, None).await?;
    let row = sqlx::query(concat!(
        "select ",
        chat_message_columns!(),
        " from public.chat_messages where id = $1::uuid"
    ))
    .bind(message_id)
    .fetch_optional(&mut **t)
    .await
    .map_err(db_error)?;
    row.as_ref().map(chat_message_of).transpose()
}

/// Read newest first for the page, then turned round: the port answers newest last. R1384000: the
/// messages before the cursor by a row comparison on `(created_at, id)`, the fake's tuple order.
pub async fn chat_page(
    t: &mut PgTx<'_>,
    x: &str,
    y: &str,
    before: Option<&ChatCursor>,
    limit: i64,
) -> Result<Vec<ChatMessageRow>, StoreError> {
    if !is_uuid(x) || !is_uuid(y) {
        return Ok(Vec::new());
    }
    run_as(t, Some(x)).await?;
    let rows = sqlx::query(concat!(
        "select ",
        chat_message_columns!(),
        " from public.chat_messages
          where ((from_profile = $1::uuid and to_profile = $2::uuid)
              or (from_profile = $2::uuid and to_profile = $1::uuid))
            and ($3::double precision is null or (created_at, id) < (",
        ts!("$3"),
        ", $4::uuid))
          order by created_at desc, id desc
          limit $5::bigint"
    ))
    .bind(x)
    .bind(y)
    .bind(before.map(|cursor| cursor.created_at))
    .bind(before.map(|cursor| cursor.id.as_str()))
    .bind(limit)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    let mut messages = rows.iter().map(chat_message_of).collect::<Result<Vec<_>, _>>()?;
    messages.reverse();
    Ok(messages)
}

pub async fn chat_mark_read(
    t: &mut PgTx<'_>,
    reader: &str,
    message_ids: &[String],
    at: i64,
) -> Result<(), StoreError> {
    let ids: Vec<String> = message_ids.iter().filter(|id| is_uuid(id)).cloned().collect();
    if ids.is_empty() || !is_uuid(reader) {
        return Ok(());
    }
    run_as(t, Some(reader)).await?;
    sqlx::query(concat!(
        "update public.chat_messages set read_at = ",
        ts!("$3"),
        "
          where to_profile = $1::uuid and id = any($2::uuid[]) and read_at is null"
    ))
    .bind(reader)
    .bind(ids)
    .bind(at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub async fn chat_unread(t: &mut PgTx<'_>, profile_id: &str) -> Result<IndexMap<String, i64>, StoreError> {
    if !is_uuid(profile_id) {
        return Ok(IndexMap::new());
    }
    run_as(t, Some(profile_id)).await?;
    let rows = sqlx::query(
        "select from_profile, count(*) as n from public.chat_messages
          where to_profile = $1::uuid and read_at is null
          group by from_profile
          order by from_profile",
    )
    .bind(profile_id)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    let mut unread = IndexMap::new();
    for row in &rows {
        unread.insert(uuid_text(row, "from_profile")?, get::<i64>(row, "n")?);
    }
    Ok(unread)
}

/// The unique key on `(message_id, reporter)` is inferred in `on conflict`: a second report by the
/// same reporter answers false.
pub async fn chat_report(t: &mut PgTx<'_>, row: &ChatReportRow) -> Result<bool, StoreError> {
    run_as(t, Some(row.reporter.as_str())).await?;
    let done = sqlx::query(concat!(
        "insert into public.chat_reports (id, message_id, reporter, reason, created_at)
         values ($1::uuid, $2::uuid, $3::uuid, $4::text, ",
        ts!("$5"),
        ")
         on conflict (message_id, reporter) do nothing"
    ))
    .bind(row.id.as_str())
    .bind(row.message_id.as_str())
    .bind(row.reporter.as_str())
    .bind(row.reason.as_str())
    .bind(row.created_at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(done.rows_affected() == 1)
}

/// The reports go with their messages (`chat_reports.message_id`, `on delete cascade`).
pub async fn chat_purge(t: &mut PgTx<'_>, before: i64) -> Result<i64, StoreError> {
    run_as(t, None).await?;
    let done = sqlx::query(concat!(
        "delete from public.chat_messages where created_at < ",
        ts!("$1")
    ))
    .bind(before)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(i64::try_from(done.rows_affected()).unwrap_or(i64::MAX))
}

/// The operator's read (`moderate chat-reports`), so as no profile: service_role reads every row.
pub async fn chat_reports_all(t: &mut PgTx<'_>) -> Result<Vec<ChatReportListing>, StoreError> {
    run_as(t, None).await?;
    let rows = sqlx::query(
        "select r.id as report_id, r.reporter, r.reason, r.created_at as reported_at,
                m.id, m.from_profile, m.to_profile, m.body, m.created_at, m.read_at
           from public.chat_reports r
           join public.chat_messages m on m.id = r.message_id
          order by r.created_at, r.id",
    )
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| {
            let message = chat_message_of(row)?;
            Ok(ChatReportListing {
                report: ChatReportRow {
                    id: uuid_text(row, "report_id")?,
                    message_id: message.id.clone(),
                    reporter: uuid_text(row, "reporter")?,
                    reason: get(row, "reason")?,
                    created_at: ms_of(get(row, "reported_at")?),
                },
                message,
            })
        })
        .collect()
}

// Small helpers

fn from_ticket_status(status: &TicketStatus) -> Result<&'static str, StoreError> {
    Ok(match literal(status)?.as_str() {
        "open" => "queued",
        "matched" => "claimed",
        _ => "cancelled",
    })
}

// Social: watching and shared replays (#939, #940)
// Watching a friend (B50) and sharing a replay with one (B52), over migration 0038's tables. Plain
// statements: 0038 adds no `app.*` function for them. An id that is not a UUID names nobody, as
// `matches_seats` reads one.

use crate::db::store::{ReplayShare, SharedReplayRow};

/// B52: what a share `s`, joined to its match `m`, is read through on: the match is over, the
/// sharer held a seat in it, the retention purge has not taken its action log (as `replays_list`
/// does), and the two are still friends (`a < b`) with neither blocking the other.
macro_rules! readable_share {
    () => {
        "m.status = 'over'
            and s.sharer in (m.p1_profile_id, m.p2_profile_id)
            and exists (select 1 from public.match_actions a where a.match_id = m.id)
            and exists (select 1 from public.friendships f
                         where f.a = least(s.sharer, s.friend) and f.b = greatest(s.sharer, s.friend))
            and not exists (select 1 from public.blocks k
                             where (k.blocker = s.sharer and k.blocked = s.friend)
                                or (k.blocker = s.friend and k.blocked = s.sharer))"
    };
}

pub async fn shares_put(t: &mut PgTx<'_>, share: &ReplayShare) -> Result<(), StoreError> {
    if !is_uuid(&share.match_id) || !is_uuid(&share.sharer) || !is_uuid(&share.friend) {
        return Err(StoreError::from(format!(
            "a replay share names an id that is not a UUID: {}",
            stringify(share)
        )));
    }
    run_as(t, Some(&share.sharer)).await?;
    sqlx::query(concat!(
        "insert into public.replay_shares (match_id, sharer, friend, created_at)
         values ($1::uuid, $2::uuid, $3::uuid, ",
        ts!("$4"),
        ")
         on conflict (match_id, sharer, friend) do nothing"
    ))
    .bind(share.match_id.as_str())
    .bind(share.sharer.as_str())
    .bind(share.friend.as_str())
    .bind(share.created_at)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub async fn shares_remove(
    t: &mut PgTx<'_>,
    match_id: &str,
    sharer: &str,
    friend: &str,
) -> Result<(), StoreError> {
    if !is_uuid(match_id) || !is_uuid(sharer) || !is_uuid(friend) {
        return Ok(());
    }
    run_as(t, Some(sharer)).await?;
    sqlx::query(
        "delete from public.replay_shares
          where match_id = $1::uuid and sharer = $2::uuid and friend = $3::uuid",
    )
    .bind(match_id)
    .bind(sharer)
    .bind(friend)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(())
}

pub async fn shares_remove_between(t: &mut PgTx<'_>, x: &str, y: &str) -> Result<i64, StoreError> {
    if !is_uuid(x) || !is_uuid(y) {
        return Ok(0);
    }
    run_as(t, Some(x)).await?;
    let done = sqlx::query(
        "delete from public.replay_shares
          where (sharer = $1::uuid and friend = $2::uuid)
             or (sharer = $2::uuid and friend = $1::uuid)",
    )
    .bind(x)
    .bind(y)
    .execute(&mut **t)
    .await
    .map_err(db_error)?;
    Ok(i64::try_from(done.rows_affected()).unwrap_or(i64::MAX))
}

pub async fn shares_sharers(
    t: &mut PgTx<'_>,
    match_id: &str,
    friend: &str,
) -> Result<Vec<String>, StoreError> {
    if !is_uuid(match_id) || !is_uuid(friend) {
        return Ok(Vec::new());
    }
    run_as(t, Some(friend)).await?;
    let rows = sqlx::query(concat!(
        "select s.sharer
           from public.replay_shares s
           join public.matches m on m.id = s.match_id
          where s.match_id = $1::uuid and s.friend = $2::uuid and ",
        readable_share!(),
        "
          order by s.created_at, s.sharer"
    ))
    .bind(match_id)
    .bind(friend)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(|row| uuid_text(row, "sharer")).collect()
}

pub async fn shares_for(
    t: &mut PgTx<'_>,
    friend: &str,
    limit: i64,
) -> Result<Vec<SharedReplayRow>, StoreError> {
    if !is_uuid(friend) {
        return Ok(Vec::new());
    }
    run_as(t, Some(friend)).await?;
    let rows = sqlx::query(concat!(
        "select s.match_id, s.sharer, s.created_at, m.ended_at
           from public.replay_shares s
           join public.matches m on m.id = s.match_id
          where s.friend = $1::uuid and ",
        readable_share!(),
        "
          order by s.created_at desc, s.match_id desc, s.sharer desc
          limit $2::bigint"
    ))
    .bind(friend)
    .bind(limit)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter()
        .map(|row| -> Result<SharedReplayRow, StoreError> {
            Ok(SharedReplayRow {
                match_id: uuid_text(row, "match_id")?,
                sharer: uuid_text(row, "sharer")?,
                created_at: ms_of(get(row, "created_at")?),
                ended_at: ms_or_null(get(row, "ended_at")?),
            })
        })
        .collect()
}

pub async fn shares_by(t: &mut PgTx<'_>, match_id: &str, sharer: &str) -> Result<Vec<String>, StoreError> {
    if !is_uuid(match_id) || !is_uuid(sharer) {
        return Ok(Vec::new());
    }
    run_as(t, Some(sharer)).await?;
    let rows = sqlx::query(concat!(
        "select s.friend
           from public.replay_shares s
           join public.matches m on m.id = s.match_id
          where s.match_id = $1::uuid and s.sharer = $2::uuid and ",
        readable_share!(),
        "
          order by s.created_at, s.friend"
    ))
    .bind(match_id)
    .bind(sharer)
    .fetch_all(&mut **t)
    .await
    .map_err(db_error)?;
    rows.iter().map(|row| uuid_text(row, "friend")).collect()
}

// KNOWN DIVERGENCES from `src/db/fake.rs`
//
// Where the port and the schema (the migrations are append-only and checksum-locked) hold different
// information. Each is asserted in `tests/store/contract.rs` where it is observable.
//
//  * redemption limits. §9.4's "more than 5 attempts", "more than 20" and "the last hour" are written into
//    `app.redeem_invite_code`'s body, and the breaker's knobs into `app.settings`, so neither can read
//    `ApiLimits`; the in-memory store takes all three as options defaulting to the `src/config.rs` constants.
//  * attempt reason. `CodeAttempt.reason` has no column in `public.code_attempts`: it is dropped on write,
//    and `redeem` returns only a result code.
//  * redemption's email check. The SQL reads `auth.users.email_confirmed_at` itself; the in-memory store uses
//    `RedemptionSettings.email_verified`. The server refuses an unverified caller (R159) before either store.
//  * redemption's circuit breaker. The SQL also refuses on its own count of recent failures (R106); the
//    in-memory store has only the `app.settings.redemption_enabled` switch: the server's R106 breaker
//    (`src/api/codes.rs`) runs first, and a counting fake would trip BUILD M6-T1's 150-failure timing test.
//  * profile email. `public.profiles` has no email column (§9.4's managed auth provider owns it on
//    `auth.users`), so `Profile.email` is read from there; `profiles_create` ignores its `email` argument and
//    needs the `auth.users` row, because `profiles.id` references it.
//  * foreign keys. `results.match_id`, `tickets.match_id`, `profiles.current_match_id`, `decks`, `trios` and
//    both sides of `series` are foreign keys: a pointer to a row never created raises here and passes in the
//    fake (`src/api/**` writes the target first). `series.next_match_id` has NONE, on purpose (R263).
//  * deck and trio strictness. `app.upsert_deck` / `app.upsert_trio` (0007) raise on an inactive profile, a
//    bad name (D1, T1), too many cards (D2) or copies of one id (D4) and a malformed cards value; the
//    in-memory store checks none, because `check_deck_draft` / `check_trio_draft` refuse them first. Stricter
//    by design (§9.4's "defense in depth"). D3 (a deckable card) is checked by neither store.
//  * tutorial, player settings and custom card strictness: the SQL raises where the in-memory store checks
//    nothing, because the route refuses first. `app.merge_tutorial_progress` (0011, R320) raises on an
//    inactive profile, a bad lesson id, a choice with no time or a missing profile row
//    (`src/api/tutorial.rs`); its cap is the smaller of the caller's and `app.settings.tutorial_lessons_max`.
//    `app.merge_player_settings` (0018, R633) raises on an inactive profile or a malformed group
//    (`src/api/settings.rs`); its caps are the smaller of the caller's and `app.settings`'
//    `player_settings_groups_max` / `player_settings_bytes_max`, the byte cap counting the text jsonb prints
//    (`jsonb_text_bytes`). `app.create_custom_card` / `app.update_custom_card` (0037, R915008) and
//    `public.custom_cards`' checks refuse a missing or inactive profile and a bad id, version, frame, source
//    or builder (`src/api/custom_cards.rs` refuses what the engine's validator does, R915001).
//  * caps. The port passes the cap (`max_decks`, `max_trios`, `max_cards`); the SQL applies the smaller of it
//    and `app.settings.max_saved_decks` / `max_saved_trios` (0007, mirroring `MAX_SAVED_DECKS` and
//    `MAX_SAVED_TRIOS`) / `max_custom_cards` (0037, mirroring `CUSTOM_CARDS_PER_ACCOUNT`). Equal today;
//    raising a cap in `src/config.rs` alone raises it only in memory, as for the redemption limits.
//  * deck and trio timestamps. `app.upsert_deck` / `app.upsert_trio` take one instant: `updated_at` always,
//    `created_at` too when the row is new, and this store hands them `updated_at`. The fake stores a new
//    deck's `created_at` as given; the two agree for a caller that stamps both from one clock read.
//  * R254's decks. A deck migration 0007 made from a loadout holds its cards in `app.resolve_deck` order,
//    which is card-id order; every deck saved since keeps the order it was saved in.
//  * launch grant. Migration 0002's trigger grants every NON-TOKEN card of the current catalog version; the
//    fake also skips BANNED ids (§9.4 L6), which the database grants.
//  * action timestamps. `app.append_match_action` stamps `at` with the database clock and `match_actions` is
//    append-only, so the caller cannot write `MatchActionRow.at`. `seq` order is unaffected; only a replay
//    tool and `timing-backfill` (R1442) read `at`, so their think times run from commit to commit.
//  * rooms. There is no `rooms` table: a room is a `public.matches` row with `status = 'open'`, its
//    `Room.expires_at` in `ceiling_at` (0004: meaningless while open) and `rooms_claim` rewriting the row's
//    id to the minted match id. `Room.mode` / `Room.host_trio` are `room_mode` / `room_trio` (0008); a row
//    with no mode (before 0008, or from 0004's `app.create_room`) reads as `bo1`, all a room could be then.
//  * reserved match ids. In Postgres the id `tickets_claim_pair`, `tickets_claim_for_bot` or `rooms_claim`
//    reserves is an `open` row, so `matches_discard_open` (R263) deletes a row: the claimed tickets'
//    `match_id` goes back to null and a claimed room's code is free again. The fake keeps only the claim's
//    time (`FakeTables.reserved`, R1450, R1454), which its `discard_open` drops; a matched ticket keeps the
//    discarded id there. `matches_reap_open` (R1450) deletes the same in both stores.
//  * deleted accounts. Migrations 0012 and 0022 null a deleted profile's seat on its finished matches,
//    results, series and `rated_games` sides (cascading its `season_ranks` away). A `String` cannot hold the
//    null, so `matches_get`, `results_get_by_match`, `series_get` and `ranked_game` read such a seat, either
//    one, back as the empty string (`to_match`); the in-memory store keeps the id (no foreign keys). Reads
//    that bind the seats a finished match hands on (R768) take an id that is no uuid as naming nothing
//    (`profiles_get_many`, `ranked_standings_subset`, `blocks_between`, the economy's `game_jlore_totals`).
//    A live match or series cannot lose a seat: the delete is refused, by constraint here and by
//    `DELETE /api/account` first.
//  * clocks. `MatchClocks` has a grace deadline per player; `public.matches` has one `grace_deadline_at` plus
//    two `*_disconnected_at`. The per-player deadlines live in the `*_disconnected_at` columns and
//    `grace_deadline_at` keeps the nearer; a migration adding per-player deadline columns would retire this.
