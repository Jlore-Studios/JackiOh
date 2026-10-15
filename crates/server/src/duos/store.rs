//! The Duos co-op store (fullsend SPEC S6, B33, B34, B40, B42; migration 0037): co-op rooms, co-op
//! matches and their append-only log, over Postgres (`pg` below) or the in-memory fake (`fake`
//! below, whose tables are `FakeTables::duos`).
//!
//! Same shape as `db::store`: every store method is one method on `Tx`, named `duos_<table>_<verb>`,
//! whose body dispatches to this file's `pg::<name>` or `fake::<name>`. The impl lives here, beside
//! its two halves, so the co-op store is one file (an inherent impl may sit anywhere in the crate).
//! The fake is strict where Postgres is strict: a room code is taken once, a claim is single-shot, a
//! match id is written once, and the log answers a nonce already logged at its seq with that row and
//! refuses any other clash as `StoreError::SeqClash` (R776000's rule, as `matches_append_actions`).
//!
//! Integers are `i64` epoch milliseconds and seqs, as everywhere in the store.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use jackioh_engine::Handicap;
use jackioh_engine::config::Difficulty;
use jackioh_engine::wire::{DuosAction, DuosActionBody, DuosSeat, DuosTeamRules};

use crate::db::store::{FrozenDeck, StoreResult, Tx};

// ---------------------------------------------------------------------------
// The rows
// ---------------------------------------------------------------------------

/// What a person brings to a co-op seat (S6's `deckId` or `random: true`), frozen when the room is
/// made or joined: one of their saved decks, checked legal then (R253's `freeze_choice`) under the
/// catalog version it names, or a deal from the match seed when the match starts (R258), leaning on
/// the newest set when they asked (R1372). The `host_choice` / `guest_choice` column.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum DuosChoice {
    Deck {
        deck: FrozenDeck,
        /// §9.4: the catalog the deck was checked against. A join under another catalog is refused
        /// (`update_required`), as a 1v1 room's is: the running catalog may not deal it.
        #[serde(default)]
        catalog_version: String,
    },
    Random {
        lean_newest: bool,
    },
}

/// One row of `public.duos_rooms` (S6, B33, B34).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuosRoom {
    /// `ROOM_CODE_LENGTH` characters of the room-code alphabet, normalised.
    pub code: String,
    /// The profile that made it: seat a1.
    pub host: String,
    /// The profile whose join claimed it: seat a2. `None` while it waits.
    pub guest: Option<String>,
    /// The AI team's tier (§9.9): its rules (`duos_ai_team`) and its seats' handicaps.
    pub difficulty: Difficulty,
    pub host_choice: DuosChoice,
    pub guest_choice: Option<DuosChoice>,
    /// The match the join started.
    pub match_id: Option<String>,
    pub created_at: i64,
    /// `created_at + DUOS_ROOM_TTL_MS`; unjoined past it, the room is gone (B34).
    pub expires_at: i64,
}

/// Every seat's deck, in library order as the engine is handed it (`DuosDecks`'s fields).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeatDecks {
    pub a1: Vec<String>,
    pub a2: Vec<String>,
    pub b1: Vec<String>,
    pub b2: Vec<String>,
}

/// Both teams' rules (`PerTeam<DuosTeamRules>`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TeamRules {
    pub a: DuosTeamRules,
    pub b: DuosTeamRules,
}

/// Each seat's §9.9 handicap (`DuosHandicaps`); `None` is `HUMAN_HANDICAP`.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SeatHandicaps {
    pub a1: Option<Handicap>,
    pub a2: Option<Handicap>,
    pub b1: Option<Handicap>,
    pub b2: Option<Handicap>,
}

/// What a co-op match was created with besides its seed: the `options` column. Frozen at the start,
/// like a 1v1 match's decks, so every rebuild and every replay folds the same game whatever the
/// tiers' constants become later.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuosMatchOptions {
    pub decks: SeatDecks,
    /// R433: the seats whose deck was dealt rather than built (a person's random deck; never an AI's).
    pub dealt: Vec<DuosSeat>,
    pub teams: TeamRules,
    pub handicaps: SeatHandicaps,
    /// R767000: the catalog version the match was dealt and is played on, so its rebuilds and its
    /// replay fold under that version's catalog whatever this build's is.
    #[serde(default)]
    pub catalog_version: String,
}

/// One row of `public.duos_matches` (S6, B40, B42).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuosMatch {
    pub id: String,
    pub seed: String,
    /// The room's host.
    pub a1: String,
    /// The room's guest.
    pub a2: String,
    pub difficulty: Difficulty,
    pub options: DuosMatchOptions,
    pub created_at: i64,
    /// Written once, with `result`, when the game ends; `None` while it is live.
    pub ended_at: Option<i64>,
    /// The engine's `DuosResult` as JSON.
    pub result: Option<Value>,
}

/// One row of `public.duos_match_actions` (B40).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DuosActionRow {
    pub match_id: String,
    pub seq: i64,
    pub nonce: String,
    /// The applied `DuosAction`, `{ seat, nonce, body }` (`action_json`).
    pub action: Value,
    pub at: i64,
}

/// The fake's three tables (`FakeTables::duos`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DuosTables {
    pub rooms: Vec<DuosRoom>,
    pub matches: Vec<DuosMatch>,
    pub actions: Vec<DuosActionRow>,
}

/// A `DuosAction` as the log holds it: `{ "seat", "nonce", "body" }`, the engine type's own JSON.
#[derive(Serialize)]
struct LoggedRef<'a> {
    seat: DuosSeat,
    nonce: &'a str,
    body: &'a DuosActionBody,
}

#[derive(Deserialize)]
struct Logged {
    seat: DuosSeat,
    nonce: String,
    body: DuosActionBody,
}

/// The log's JSON for one action.
pub fn action_json(seat: DuosSeat, nonce: &str, body: &DuosActionBody) -> Result<Value, serde_json::Error> {
    serde_json::to_value(LoggedRef { seat, nonce, body })
}

/// The action a log row holds, as the engine folds it.
pub fn action_of(value: &Value) -> Result<DuosAction, serde_json::Error> {
    let logged = Logged::deserialize(value)?;
    Ok(DuosAction {
        seat: logged.seat,
        nonce: logged.nonce,
        body: logged.body,
    })
}

/// The seat a log row's action was taken for, without reading its body.
pub fn seat_of_logged(value: &Value) -> Option<DuosSeat> {
    value.get("seat")?.as_str()?.parse::<DuosSeat>().ok()
}

// ---------------------------------------------------------------------------
// The store methods
// ---------------------------------------------------------------------------

/// `duos_dispatch!(self, name(args))`: the same call against whichever store this transaction is on.
macro_rules! duos_dispatch {
    ($self:ident, $name:ident ( $($arg:expr),* $(,)? )) => {
        match $self {
            Tx::Pg(t) => pg::$name(t, $($arg),*).await,
            Tx::Fake(f) => fake::$name(f, $($arg),*),
        }
    };
}

impl Tx<'_> {
    /// B33: a new room. False when the code is already held by a room (the caller mints another).
    pub async fn duos_rooms_create(&mut self, room: &DuosRoom) -> StoreResult<bool> {
        duos_dispatch!(self, rooms_create(room))
    }

    /// The room under `code` (normalised), joined or not, expired or not.
    pub async fn duos_rooms_get(&mut self, code: &str) -> StoreResult<Option<DuosRoom>> {
        duos_dispatch!(self, rooms_get(code))
    }

    /// B33's atomic single-claim: sets the guest, the guest's choice and the match id only while the
    /// room is unjoined and unexpired at `at`. The claimed room, or `None` when it was not (a second
    /// joiner, a race, an expiry).
    pub async fn duos_rooms_claim(
        &mut self,
        code: &str,
        guest: &str,
        guest_choice: &DuosChoice,
        match_id: &str,
        at: i64,
    ) -> StoreResult<Option<DuosRoom>> {
        duos_dispatch!(self, rooms_claim(code, guest, guest_choice, match_id, at))
    }

    /// B34: deletes `host`'s unjoined room `code`. False when there is no such room.
    pub async fn duos_rooms_delete(&mut self, code: &str, host: &str) -> StoreResult<bool> {
        duos_dispatch!(self, rooms_delete(code, host))
    }

    /// B34: deletes every unjoined room that expired at or before `unjoined_before`, and every joined
    /// one that expired at or before `joined_before` (kept longer, so a host's poll still reads the
    /// match its guest started). Answers how many went.
    pub async fn duos_rooms_reap(&mut self, unjoined_before: i64, joined_before: i64) -> StoreResult<u64> {
        duos_dispatch!(self, rooms_reap(unjoined_before, joined_before))
    }

    pub async fn duos_matches_create(&mut self, row: &DuosMatch) -> StoreResult<()> {
        duos_dispatch!(self, matches_create(row))
    }

    pub async fn duos_matches_get(&mut self, match_id: &str) -> StoreResult<Option<DuosMatch>> {
        duos_dispatch!(self, matches_get(match_id))
    }

    /// B42: the newest live co-op match that holds `profile_id` in either seat.
    pub async fn duos_matches_live_for(&mut self, profile_id: &str) -> StoreResult<Option<String>> {
        duos_dispatch!(self, matches_live_for(profile_id))
    }

    /// Every live co-op match, oldest first: what the sweeper resumes after a restart.
    pub async fn duos_matches_live(&mut self) -> StoreResult<Vec<String>> {
        duos_dispatch!(self, matches_live())
    }

    /// B40: writes the result and `ended_at` once. False when the match already has them (or there is
    /// no such match): the first write stands.
    pub async fn duos_matches_finish(
        &mut self,
        match_id: &str,
        result: &Value,
        at: i64,
    ) -> StoreResult<bool> {
        duos_dispatch!(self, matches_finish(match_id, result, at))
    }

    /// B40: appends one row. A nonce already logged at its own seq answers that row; any other clash
    /// of seq or nonce is `StoreError::SeqClash` (R776000).
    pub async fn duos_actions_append(&mut self, row: &DuosActionRow) -> StoreResult<()> {
        duos_dispatch!(self, actions_append(row))
    }

    /// The match's log in seq order.
    pub async fn duos_actions(&mut self, match_id: &str) -> StoreResult<Vec<DuosActionRow>> {
        duos_dispatch!(self, actions(match_id))
    }

    /// The retention purge (`api/retention.rs`): deletes the log of every co-op match that ended
    /// before `ended_before`, as a 1v1 match's log goes (`MATCH_ACTION_RETENTION_DAYS`). The match
    /// row, its result with it, stays. Answers how many rows went.
    pub async fn duos_actions_purge(&mut self, ended_before: i64) -> StoreResult<u64> {
        duos_dispatch!(self, actions_purge(ended_before))
    }
}

/// Migration 0037's `on delete cascade`, in the fake (`db::fake::remove_profile_rows`): a deleted
/// profile's rooms (host or guest) and co-op matches (either seat) go, each match with its log.
pub fn remove_duos_rows(tables: &mut DuosTables, profile_id: &str) {
    tables
        .rooms
        .retain(|room| room.host != profile_id && room.guest.as_deref() != Some(profile_id));
    let gone: Vec<String> = tables
        .matches
        .iter()
        .filter(|row| row.a1 == profile_id || row.a2 == profile_id)
        .map(|row| row.id.clone())
        .collect();
    tables.matches.retain(|row| !gone.contains(&row.id));
    tables.actions.retain(|row| !gone.contains(&row.match_id));
}

// ---------------------------------------------------------------------------
// Postgres (migration 0037)
// ---------------------------------------------------------------------------

mod pg {
    use serde_json::Value;
    use sqlx::Row;
    use sqlx::postgres::PgRow;
    use time::OffsetDateTime;
    use uuid::Uuid;

    use jackioh_engine::config::Difficulty;

    use super::{DuosActionRow, DuosChoice, DuosMatch, DuosRoom};
    use crate::auth::is_uuid;
    use crate::db::pg::PgTx;
    use crate::db::store::StoreError;

    /// `db::pg`'s `SESSION_SQL` and acting role: every method re-stamps its subject first, so it
    /// runs as `service_role` with `request.jwt.claim.sub` the profile the call is about (or none).
    const SESSION_SQL: &str =
        "select set_config('role', $1, true), set_config('request.jwt.claim.sub', $2, true)";
    const ACTING_ROLE: &str = "service_role";

    /// Nanoseconds in a millisecond.
    const NANOS_PER_MS: i128 = 1_000_000;

    async fn run_as(t: &mut PgTx<'_>, subject: Option<&str>) -> Result<(), StoreError> {
        sqlx::query(SESSION_SQL)
            .bind(ACTING_ROLE)
            .bind(subject.unwrap_or(""))
            .execute(&mut **t)
            .await?;
        Ok(())
    }

    fn ms_of(value: OffsetDateTime) -> i64 {
        (value.unix_timestamp_nanos() / NANOS_PER_MS) as i64
    }

    fn json_text<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, StoreError> {
        Ok(serde_json::to_string(value)?)
    }

    fn difficulty_of(text: &str) -> Result<Difficulty, StoreError> {
        text.parse::<Difficulty>().map_err(StoreError::Other)
    }

    fn uuid_text(row: &PgRow, column: &str) -> Result<String, StoreError> {
        Ok(row.try_get::<Uuid, _>(column)?.to_string())
    }

    fn uuid_or_null(row: &PgRow, column: &str) -> Result<Option<String>, StoreError> {
        Ok(row.try_get::<Option<Uuid>, _>(column)?.map(|id| id.to_string()))
    }

    /// A macro rather than a `const`, so every query stays a `&'static str` (`concat!`).
    macro_rules! room_columns {
        () => {
            "code, host, guest, difficulty, host_choice, guest_choice, match_id, created_at, expires_at"
        };
    }

    fn room_of(row: &PgRow) -> Result<DuosRoom, StoreError> {
        let guest_choice: Option<Value> = row.try_get("guest_choice")?;
        Ok(DuosRoom {
            code: row.try_get("code")?,
            host: uuid_text(row, "host")?,
            guest: uuid_or_null(row, "guest")?,
            difficulty: difficulty_of(&row.try_get::<String, _>("difficulty")?)?,
            host_choice: serde_json::from_value(row.try_get::<Value, _>("host_choice")?)?,
            guest_choice: guest_choice
                .map(serde_json::from_value::<DuosChoice>)
                .transpose()?,
            match_id: uuid_or_null(row, "match_id")?,
            created_at: ms_of(row.try_get("created_at")?),
            expires_at: ms_of(row.try_get("expires_at")?),
        })
    }

    macro_rules! match_columns {
        () => {
            "id, seed, a1, a2, difficulty, options, created_at, ended_at, result"
        };
    }

    fn match_of(row: &PgRow) -> Result<DuosMatch, StoreError> {
        let ended_at: Option<OffsetDateTime> = row.try_get("ended_at")?;
        Ok(DuosMatch {
            id: uuid_text(row, "id")?,
            seed: row.try_get("seed")?,
            a1: uuid_text(row, "a1")?,
            a2: uuid_text(row, "a2")?,
            difficulty: difficulty_of(&row.try_get::<String, _>("difficulty")?)?,
            options: serde_json::from_value(row.try_get::<Value, _>("options")?)?,
            created_at: ms_of(row.try_get("created_at")?),
            ended_at: ended_at.map(ms_of),
            result: row.try_get("result")?,
        })
    }

    pub async fn rooms_create(t: &mut PgTx<'_>, room: &DuosRoom) -> Result<bool, StoreError> {
        run_as(t, Some(room.host.as_str())).await?;
        let done = sqlx::query(
            "insert into public.duos_rooms (code, host, difficulty, host_choice, created_at, expires_at)
             values ($1::text, $2::uuid, $3::text, $4::jsonb,
                     to_timestamp($5::double precision / 1000.0),
                     to_timestamp($6::double precision / 1000.0))
             on conflict (code) do nothing",
        )
        .bind(room.code.as_str())
        .bind(room.host.as_str())
        .bind(room.difficulty.as_str())
        .bind(json_text(&room.host_choice)?)
        .bind(room.created_at)
        .bind(room.expires_at)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected() == 1)
    }

    pub async fn rooms_get(t: &mut PgTx<'_>, code: &str) -> Result<Option<DuosRoom>, StoreError> {
        run_as(t, None).await?;
        let row = sqlx::query(concat!(
            "select ",
            room_columns!(),
            " from public.duos_rooms where code = $1::text"
        ))
        .bind(code)
        .fetch_optional(&mut **t)
        .await?;
        row.as_ref().map(room_of).transpose()
    }

    /// B33's single-claim as one statement: the loser of a race updates no row.
    pub async fn rooms_claim(
        t: &mut PgTx<'_>,
        code: &str,
        guest: &str,
        guest_choice: &DuosChoice,
        match_id: &str,
        at: i64,
    ) -> Result<Option<DuosRoom>, StoreError> {
        run_as(t, Some(guest)).await?;
        let row = sqlx::query(concat!(
            "update public.duos_rooms
                set guest = $2::uuid, guest_choice = $3::jsonb, match_id = $4::uuid
              where code = $1::text
                and guest is null
                and host <> $2::uuid
                and expires_at > to_timestamp($5::double precision / 1000.0)
          returning ",
            room_columns!()
        ))
        .bind(code)
        .bind(guest)
        .bind(json_text(guest_choice)?)
        .bind(match_id)
        .bind(at)
        .fetch_optional(&mut **t)
        .await?;
        row.as_ref().map(room_of).transpose()
    }

    pub async fn rooms_delete(t: &mut PgTx<'_>, code: &str, host: &str) -> Result<bool, StoreError> {
        run_as(t, Some(host)).await?;
        if !is_uuid(host) {
            return Ok(false);
        }
        let done = sqlx::query(
            "delete from public.duos_rooms where code = $1::text and host = $2::uuid and guest is null",
        )
        .bind(code)
        .bind(host)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected() == 1)
    }

    pub async fn rooms_reap(
        t: &mut PgTx<'_>,
        unjoined_before: i64,
        joined_before: i64,
    ) -> Result<u64, StoreError> {
        run_as(t, None).await?;
        let done = sqlx::query(
            "delete from public.duos_rooms
              where (guest is null and expires_at <= to_timestamp($1::double precision / 1000.0))
                 or (guest is not null and expires_at <= to_timestamp($2::double precision / 1000.0))",
        )
        .bind(unjoined_before)
        .bind(joined_before)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected())
    }

    pub async fn matches_create(t: &mut PgTx<'_>, row: &DuosMatch) -> Result<(), StoreError> {
        run_as(t, None).await?;
        sqlx::query(
            "insert into public.duos_matches (id, seed, a1, a2, difficulty, options, created_at)
             values ($1::uuid, $2::text, $3::uuid, $4::uuid, $5::text, $6::jsonb,
                     to_timestamp($7::double precision / 1000.0))",
        )
        .bind(row.id.as_str())
        .bind(row.seed.as_str())
        .bind(row.a1.as_str())
        .bind(row.a2.as_str())
        .bind(row.difficulty.as_str())
        .bind(json_text(&row.options)?)
        .bind(row.created_at)
        .execute(&mut **t)
        .await?;
        Ok(())
    }

    pub async fn matches_get(t: &mut PgTx<'_>, match_id: &str) -> Result<Option<DuosMatch>, StoreError> {
        run_as(t, None).await?;
        // A path segment the caller typed: an id that is no uuid names no match, rather than failing
        // the cast.
        if !is_uuid(match_id) {
            return Ok(None);
        }
        let row = sqlx::query(concat!(
            "select ",
            match_columns!(),
            " from public.duos_matches where id = $1::uuid"
        ))
        .bind(match_id)
        .fetch_optional(&mut **t)
        .await?;
        row.as_ref().map(match_of).transpose()
    }

    pub async fn matches_live_for(t: &mut PgTx<'_>, profile_id: &str) -> Result<Option<String>, StoreError> {
        run_as(t, Some(profile_id)).await?;
        if !is_uuid(profile_id) {
            return Ok(None);
        }
        let row = sqlx::query(
            "select id from public.duos_matches
              where (a1 = $1::uuid or a2 = $1::uuid) and ended_at is null
              order by created_at desc, id
              limit 1",
        )
        .bind(profile_id)
        .fetch_optional(&mut **t)
        .await?;
        row.as_ref().map(|row| uuid_text(row, "id")).transpose()
    }

    pub async fn matches_live(t: &mut PgTx<'_>) -> Result<Vec<String>, StoreError> {
        run_as(t, None).await?;
        let rows =
            sqlx::query("select id from public.duos_matches where ended_at is null order by created_at, id")
                .fetch_all(&mut **t)
                .await?;
        rows.iter().map(|row| uuid_text(row, "id")).collect()
    }

    pub async fn matches_finish(
        t: &mut PgTx<'_>,
        match_id: &str,
        result: &Value,
        at: i64,
    ) -> Result<bool, StoreError> {
        run_as(t, None).await?;
        if !is_uuid(match_id) {
            return Ok(false);
        }
        let done = sqlx::query(
            "update public.duos_matches
                set result = $2::jsonb, ended_at = to_timestamp($3::double precision / 1000.0)
              where id = $1::uuid and ended_at is null",
        )
        .bind(match_id)
        .bind(json_text(result)?)
        .bind(at)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected() == 1)
    }

    pub async fn actions_append(t: &mut PgTx<'_>, row: &DuosActionRow) -> Result<(), StoreError> {
        run_as(t, None).await?;
        let done = sqlx::query(
            "insert into public.duos_match_actions (match_id, seq, nonce, action, created_at)
             values ($1::uuid, $2::bigint, $3::text, $4::jsonb,
                     to_timestamp($5::double precision / 1000.0))
             on conflict do nothing",
        )
        .bind(row.match_id.as_str())
        .bind(row.seq)
        .bind(row.nonce.as_str())
        .bind(json_text(&row.action)?)
        .bind(row.at)
        .execute(&mut **t)
        .await?;
        if done.rows_affected() == 1 {
            return Ok(());
        }
        // The seq or the nonce is taken. The same nonce at its own seq is this very action, written
        // before an answer was lost: the log already says so. Anything else is a clash.
        let held = sqlx::query(
            "select seq, nonce from public.duos_match_actions
              where match_id = $1::uuid and (seq = $2::bigint or nonce = $3::text)",
        )
        .bind(row.match_id.as_str())
        .bind(row.seq)
        .bind(row.nonce.as_str())
        .fetch_all(&mut **t)
        .await?;
        let same = held.iter().any(|existing| {
            existing.try_get::<i64, _>("seq").ok() == Some(row.seq)
                && existing.try_get::<String, _>("nonce").ok().as_deref() == Some(row.nonce.as_str())
        });
        if same {
            return Ok(());
        }
        Err(StoreError::SeqClash(format!(
            "duos_match_actions is append-only: seq {} or nonce {} is taken",
            row.seq, row.nonce
        )))
    }

    pub async fn actions_purge(t: &mut PgTx<'_>, ended_before: i64) -> Result<u64, StoreError> {
        run_as(t, None).await?;
        let done = sqlx::query(
            "delete from public.duos_match_actions a
              using public.duos_matches m
              where a.match_id = m.id
                and m.ended_at < to_timestamp($1::double precision / 1000.0)",
        )
        .bind(ended_before)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected())
    }

    pub async fn actions(t: &mut PgTx<'_>, match_id: &str) -> Result<Vec<DuosActionRow>, StoreError> {
        run_as(t, None).await?;
        if !is_uuid(match_id) {
            return Ok(Vec::new());
        }
        let rows = sqlx::query(
            "select seq, nonce, action, created_at from public.duos_match_actions
              where match_id = $1::uuid order by seq",
        )
        .bind(match_id)
        .fetch_all(&mut **t)
        .await?;
        rows.iter()
            .map(|row| -> Result<DuosActionRow, StoreError> {
                Ok(DuosActionRow {
                    match_id: match_id.to_string(),
                    seq: row.try_get("seq")?,
                    nonce: row.try_get("nonce")?,
                    action: row.try_get("action")?,
                    at: ms_of(row.try_get("created_at")?),
                })
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// The in-memory fake (`FakeTables::duos`)
// ---------------------------------------------------------------------------

mod fake {
    use serde_json::Value;

    use super::{DuosActionRow, DuosChoice, DuosMatch, DuosRoom, DuosTables};
    use crate::db::fake::FakeTx;
    use crate::db::store::StoreError;

    /// The unit fake's two seams (`db::fake`'s `call`): its fault-injection hook, then its race hook,
    /// each told the method's name.
    fn call<'t>(f: &'t mut FakeTx<'_>, method: &str) -> Result<&'t mut DuosTables, StoreError> {
        let data = f.data();
        if let Some(hook) = data.on_call.clone() {
            hook(method)?;
        }
        if let Some(hook) = data.before_call.clone() {
            hook(method, &mut data.tables);
        }
        Ok(&mut f.tables().duos)
    }

    pub fn rooms_create(f: &mut FakeTx<'_>, room: &DuosRoom) -> Result<bool, StoreError> {
        let tables = call(f, "duos.rooms.create")?;
        if tables.rooms.iter().any(|held| held.code == room.code) {
            return Ok(false);
        }
        tables.rooms.push(DuosRoom {
            guest: None,
            guest_choice: None,
            match_id: None,
            ..room.clone()
        });
        Ok(true)
    }

    pub fn rooms_get(f: &mut FakeTx<'_>, code: &str) -> Result<Option<DuosRoom>, StoreError> {
        let tables = call(f, "duos.rooms.get")?;
        Ok(tables.rooms.iter().find(|room| room.code == code).cloned())
    }

    pub fn rooms_claim(
        f: &mut FakeTx<'_>,
        code: &str,
        guest: &str,
        guest_choice: &DuosChoice,
        match_id: &str,
        at: i64,
    ) -> Result<Option<DuosRoom>, StoreError> {
        let tables = call(f, "duos.rooms.claim")?;
        let Some(room) = tables.rooms.iter_mut().find(|room| {
            room.code == code && room.guest.is_none() && room.host != guest && room.expires_at > at
        }) else {
            return Ok(None);
        };
        room.guest = Some(guest.to_string());
        room.guest_choice = Some(guest_choice.clone());
        room.match_id = Some(match_id.to_string());
        Ok(Some(room.clone()))
    }

    pub fn rooms_delete(f: &mut FakeTx<'_>, code: &str, host: &str) -> Result<bool, StoreError> {
        let tables = call(f, "duos.rooms.delete")?;
        let before = tables.rooms.len();
        tables
            .rooms
            .retain(|room| !(room.code == code && room.host == host && room.guest.is_none()));
        Ok(tables.rooms.len() != before)
    }

    pub fn rooms_reap(
        f: &mut FakeTx<'_>,
        unjoined_before: i64,
        joined_before: i64,
    ) -> Result<u64, StoreError> {
        let tables = call(f, "duos.rooms.reap")?;
        let before = tables.rooms.len();
        tables.rooms.retain(|room| match room.guest {
            None => room.expires_at > unjoined_before,
            Some(_) => room.expires_at > joined_before,
        });
        Ok((before - tables.rooms.len()) as u64)
    }

    pub fn matches_create(f: &mut FakeTx<'_>, row: &DuosMatch) -> Result<(), StoreError> {
        let tables = call(f, "duos.matches.create")?;
        if tables.matches.iter().any(|held| held.id == row.id) {
            return Err(StoreError::from("duplicate duos match"));
        }
        tables.matches.push(DuosMatch {
            ended_at: None,
            result: None,
            ..row.clone()
        });
        Ok(())
    }

    pub fn matches_get(f: &mut FakeTx<'_>, match_id: &str) -> Result<Option<DuosMatch>, StoreError> {
        let tables = call(f, "duos.matches.get")?;
        Ok(tables.matches.iter().find(|row| row.id == match_id).cloned())
    }

    pub fn matches_live_for(f: &mut FakeTx<'_>, profile_id: &str) -> Result<Option<String>, StoreError> {
        let tables = call(f, "duos.matches.liveFor")?;
        let newest = tables
            .matches
            .iter()
            .filter(|row| row.ended_at.is_none() && (row.a1 == profile_id || row.a2 == profile_id))
            .max_by(|a, b| a.created_at.cmp(&b.created_at).then_with(|| b.id.cmp(&a.id)));
        Ok(newest.map(|row| row.id.clone()))
    }

    pub fn matches_live(f: &mut FakeTx<'_>) -> Result<Vec<String>, StoreError> {
        let tables = call(f, "duos.matches.live")?;
        let mut live: Vec<&DuosMatch> = tables
            .matches
            .iter()
            .filter(|row| row.ended_at.is_none())
            .collect();
        live.sort_by(|a, b| a.created_at.cmp(&b.created_at).then_with(|| a.id.cmp(&b.id)));
        Ok(live.into_iter().map(|row| row.id.clone()).collect())
    }

    pub fn matches_finish(
        f: &mut FakeTx<'_>,
        match_id: &str,
        result: &Value,
        at: i64,
    ) -> Result<bool, StoreError> {
        let tables = call(f, "duos.matches.finish")?;
        let Some(row) = tables
            .matches
            .iter_mut()
            .find(|row| row.id == match_id && row.ended_at.is_none())
        else {
            return Ok(false);
        };
        row.ended_at = Some(at);
        row.result = Some(result.clone());
        Ok(true)
    }

    pub fn actions_append(f: &mut FakeTx<'_>, row: &DuosActionRow) -> Result<(), StoreError> {
        let tables = call(f, "duos.actions.append")?;
        // The foreign key Postgres holds.
        if !tables.matches.iter().any(|held| held.id == row.match_id) {
            return Err(StoreError::from(format!(
                "duos_match_actions: no duos match {}",
                row.match_id
            )));
        }
        let mine = |held: &&DuosActionRow| held.match_id == row.match_id;
        if let Some(held) = tables
            .actions
            .iter()
            .filter(mine)
            .find(|held| held.seq == row.seq)
        {
            if held.nonce == row.nonce {
                return Ok(());
            }
            return Err(StoreError::SeqClash(format!(
                "duos_match_actions is append-only: seq {} exists",
                row.seq
            )));
        }
        if tables
            .actions
            .iter()
            .filter(mine)
            .any(|held| held.nonce == row.nonce)
        {
            return Err(StoreError::SeqClash(format!(
                "duos_match_actions: nonce {} is logged at another seq",
                row.nonce
            )));
        }
        tables.actions.push(row.clone());
        Ok(())
    }

    pub fn actions_purge(f: &mut FakeTx<'_>, ended_before: i64) -> Result<u64, StoreError> {
        let tables = call(f, "duos.actions.purge")?;
        let expired: Vec<String> = tables
            .matches
            .iter()
            .filter(|row| row.ended_at.is_some_and(|ended_at| ended_at < ended_before))
            .map(|row| row.id.clone())
            .collect();
        let before = tables.actions.len();
        tables.actions.retain(|row| !expired.contains(&row.match_id));
        Ok((before - tables.actions.len()) as u64)
    }

    pub fn actions(f: &mut FakeTx<'_>, match_id: &str) -> Result<Vec<DuosActionRow>, StoreError> {
        let tables = call(f, "duos.actions.list")?;
        let mut rows: Vec<DuosActionRow> = tables
            .actions
            .iter()
            .filter(|row| row.match_id == match_id)
            .cloned()
            .collect();
        rows.sort_by_key(|row| row.seq);
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use jackioh_engine::config::DUOS_HUMAN_TEAM;
    use serde_json::json;

    use super::*;
    use crate::db::store::{Db, StoreError};

    fn room(code: &str, host: &str, expires_at: i64) -> DuosRoom {
        DuosRoom {
            code: code.to_string(),
            host: host.to_string(),
            guest: None,
            difficulty: Difficulty::Easy,
            host_choice: DuosChoice::Random { lean_newest: false },
            guest_choice: None,
            match_id: None,
            created_at: 0,
            expires_at,
        }
    }

    fn duos_match(id: &str, a1: &str, a2: &str, created_at: i64) -> DuosMatch {
        DuosMatch {
            id: id.to_string(),
            seed: format!("seed-{id}"),
            a1: a1.to_string(),
            a2: a2.to_string(),
            difficulty: Difficulty::Easy,
            options: DuosMatchOptions {
                decks: SeatDecks {
                    a1: Vec::new(),
                    a2: Vec::new(),
                    b1: Vec::new(),
                    b2: Vec::new(),
                },
                dealt: vec![DuosSeat::A1],
                teams: TeamRules {
                    a: DUOS_HUMAN_TEAM,
                    b: DUOS_HUMAN_TEAM,
                },
                handicaps: SeatHandicaps {
                    a1: None,
                    a2: None,
                    b1: None,
                    b2: None,
                },
                catalog_version: "test".to_string(),
            },
            created_at,
            ended_at: None,
            result: None,
        }
    }

    fn row(match_id: &str, seq: i64, nonce: &str) -> DuosActionRow {
        DuosActionRow {
            match_id: match_id.to_string(),
            seq,
            nonce: nonce.to_string(),
            action: json!({ "seat": "a1", "nonce": nonce, "body": { "kind": "concede" } }),
            at: seq,
        }
    }

    #[tokio::test]
    async fn b33_a_room_is_claimed_once_and_never_by_its_host() {
        let db = Db::fake();
        let mut t = db.begin(None).await.expect("a transaction");
        assert_eq!(
            t.duos_rooms_create(&room("ABCDEF", "host", 1_000)).await.ok(),
            Some(true)
        );
        assert_eq!(
            t.duos_rooms_create(&room("ABCDEF", "other", 1_000)).await.ok(),
            Some(false)
        );
        let choice = DuosChoice::Random { lean_newest: true };
        let by_host = t
            .duos_rooms_claim("ABCDEF", "host", &choice, "m1", 10)
            .await
            .expect("a claim");
        assert!(by_host.is_none());
        let by_guest = t
            .duos_rooms_claim("ABCDEF", "guest", &choice, "m1", 10)
            .await
            .expect("a claim");
        assert_eq!(by_guest.and_then(|room| room.match_id), Some("m1".to_string()));
        let by_third = t
            .duos_rooms_claim("ABCDEF", "third", &choice, "m2", 10)
            .await
            .expect("a claim");
        assert!(by_third.is_none());
        // B34: a joined room is not the host's to free.
        assert_eq!(t.duos_rooms_delete("ABCDEF", "host").await.ok(), Some(false));
    }

    #[tokio::test]
    async fn b34_an_expired_room_is_not_claimed_and_is_reaped() {
        let db = Db::fake();
        let mut t = db.begin(None).await.expect("a transaction");
        t.duos_rooms_create(&room("ABCDEF", "host", 100))
            .await
            .expect("a room");
        t.duos_rooms_create(&room("GHJKLM", "host", 500))
            .await
            .expect("a room");
        let choice = DuosChoice::Random { lean_newest: false };
        let late = t
            .duos_rooms_claim("ABCDEF", "guest", &choice, "m1", 100)
            .await
            .expect("a claim");
        assert!(late.is_none());
        assert_eq!(t.duos_rooms_reap(200, 200).await.ok(), Some(1));
        assert!(t.duos_rooms_get("ABCDEF").await.expect("a read").is_none());
        assert!(t.duos_rooms_get("GHJKLM").await.expect("a read").is_some());
        assert_eq!(t.duos_rooms_delete("GHJKLM", "host").await.ok(), Some(true));
    }

    #[tokio::test]
    async fn b40_the_log_answers_a_nonce_at_its_own_seq_and_refuses_any_other_clash() {
        let db = Db::fake();
        let mut t = db.begin(None).await.expect("a transaction");
        assert!(
            t.duos_actions_append(&row("m1", 1, "n1")).await.is_err(),
            "no match, no row"
        );
        t.duos_matches_create(&duos_match("m1", "p1", "p2", 0))
            .await
            .expect("a match");
        t.duos_actions_append(&row("m1", 1, "n1"))
            .await
            .expect("the first row");
        t.duos_actions_append(&row("m1", 1, "n1"))
            .await
            .expect("the same row again");
        assert!(matches!(
            t.duos_actions_append(&row("m1", 1, "n2")).await,
            Err(StoreError::SeqClash(_))
        ));
        assert!(matches!(
            t.duos_actions_append(&row("m1", 2, "n1")).await,
            Err(StoreError::SeqClash(_))
        ));
        t.duos_actions_append(&row("m1", 2, "n2"))
            .await
            .expect("the second row");
        let log = t.duos_actions("m1").await.expect("the log");
        assert_eq!(log.iter().map(|entry| entry.seq).collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(seat_of_logged(&log[0].action), Some(DuosSeat::A1));
    }

    #[tokio::test]
    async fn the_purge_takes_the_log_of_a_match_that_ended_before_the_cutoff_and_nothing_else() {
        let db = Db::fake();
        let mut t = db.begin(None).await.expect("a transaction");
        for (id, created_at) in [("old", 1), ("new", 2), ("live", 3)] {
            t.duos_matches_create(&duos_match(id, "p1", "p2", created_at))
                .await
                .expect("a match");
            for seq in 1..=2 {
                t.duos_actions_append(&row(id, seq, &format!("{id}-{seq}")))
                    .await
                    .expect("a row");
            }
        }
        let result = json!({ "outcome": "draw", "winner": null, "reason": "turnCap" });
        t.duos_matches_finish("old", &result, 100)
            .await
            .expect("finished");
        t.duos_matches_finish("new", &result, 300)
            .await
            .expect("finished");

        assert_eq!(t.duos_actions_purge(200).await.ok(), Some(2));

        assert!(t.duos_actions("old").await.expect("the log").is_empty());
        assert_eq!(t.duos_actions("new").await.expect("the log").len(), 2);
        assert_eq!(t.duos_actions("live").await.expect("the log").len(), 2);
        let kept = t
            .duos_matches_get("old")
            .await
            .expect("a read")
            .expect("the match stays");
        assert_eq!(kept.result, Some(result));
    }

    #[tokio::test]
    async fn b42_a_result_is_written_once_and_ends_the_live_match() {
        let db = Db::fake();
        let mut t = db.begin(None).await.expect("a transaction");
        t.duos_matches_create(&duos_match("m1", "p1", "p2", 1))
            .await
            .expect("a match");
        t.duos_matches_create(&duos_match("m2", "p3", "p1", 2))
            .await
            .expect("a match");
        assert!(
            t.duos_matches_create(&duos_match("m1", "p1", "p2", 3))
                .await
                .is_err()
        );
        assert_eq!(
            t.duos_matches_live_for("p1").await.ok(),
            Some(Some("m2".to_string()))
        );
        let result = json!({ "outcome": "win", "winner": "a", "reason": "poolEmptied" });
        assert_eq!(t.duos_matches_finish("m2", &result, 10).await.ok(), Some(true));
        assert_eq!(
            t.duos_matches_finish("m2", &json!({}), 11).await.ok(),
            Some(false)
        );
        assert_eq!(
            t.duos_matches_live_for("p1").await.ok(),
            Some(Some("m1".to_string()))
        );
        assert_eq!(t.duos_matches_live().await.ok(), Some(vec!["m1".to_string()]));
        let finished = t
            .duos_matches_get("m2")
            .await
            .expect("a read")
            .expect("the match");
        assert_eq!((finished.ended_at, finished.result), (Some(10), Some(result)));
        assert_eq!(
            t.duos_matches_live_for("p2").await.ok(),
            Some(Some("m1".to_string()))
        );
    }
}
