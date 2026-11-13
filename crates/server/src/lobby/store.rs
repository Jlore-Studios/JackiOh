//! The private lobbies' store (SPEC §9.5, §13.9; migration 0090; R3730000 to R3730004; issue #3730):
//! `public.lobbies` over Postgres (`pg` below) or the in-memory fake (`fake` below, whose rows are
//! `FakeTables::lobbies`).
//!
//! Same shape as `duos::store`: each store method is a `Tx` method `lobbies_<verb>` dispatching to
//! this file's `pg::<name>` or `fake::<name>`, in the caller's transaction. Every change is
//! `lobbies_update`'s compare-and-set on `rev`, so a write that read an older lobby loses rather
//! than undo a newer one (R3730002: a Start never seats a guest who left). Timestamps are epoch
//! milliseconds.

use serde::{Deserialize, Serialize};

use jackioh_engine::config::Difficulty;

use crate::api::decks::{FrozenChoice, ModeChoiceInput};
use crate::db::store::{QueueMode, StoreResult, Tx};

/// R3730003: what the host sets, everyone sees, and what a lobby plays: a 1v1 game in one of the
/// room modes (§9.5, R264) or a co-op game against the AI team at a difficulty (§13.7, §13.9). The
/// format is fixed when the lobby is made; the mode or the difficulty may change.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(tag = "format", rename_all = "camelCase")]
pub enum LobbySettings {
    Duel { mode: QueueMode },
    Coop { difficulty: Difficulty },
}

impl LobbySettings {
    /// The format's word (`settings.format`), which the cap counts by (R3730004).
    pub fn format(&self) -> &'static str {
        match self {
            LobbySettings::Duel { .. } => FORMAT_DUEL,
            LobbySettings::Coop { .. } => FORMAT_COOP,
        }
    }
}

/// The two formats' words.
pub const FORMAT_DUEL: &str = "duel";
pub const FORMAT_COOP: &str = "coop";

/// R3730001: a member's pick, what they asked for and what it froze to when they made it (R253).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LobbyPick {
    pub choice: ModeChoiceInput,
    pub frozen: FrozenChoice,
}

/// One row of `public.lobbies` (R3730000).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Lobby {
    /// The code (`ROOM_CODE_LENGTH`, normalized), which no 1v1 or co-op room holds when it is made.
    pub code: String,
    pub host: String,
    pub guest: Option<String>,
    pub settings: LobbySettings,
    pub host_pick: Option<LobbyPick>,
    pub guest_pick: Option<LobbyPick>,
    pub host_ready: bool,
    pub guest_ready: bool,
    /// Every write's count: `lobbies_update` writes only over the revision it read.
    pub rev: i64,
    /// §9.4: the catalog the lobby was made under; a Start under another is `update_required`.
    pub catalog_version: String,
    pub created_at: i64,
    /// R3730004: the last change plus the format's TTL.
    pub expires_at: i64,
    /// R3730002: what the Start began, the series for Conquest (whose first game has its own id).
    pub match_id: Option<String>,
    pub series_id: Option<String>,
    pub started_at: Option<i64>,
}

/// Dispatches to `pg` or `fake`.
macro_rules! lobby_dispatch {
    ($self:ident, $name:ident ( $($arg:expr),* $(,)? )) => {
        match $self {
            Tx::Pg(t) => pg::$name(t, $($arg),*).await,
            Tx::Fake(f) => fake::$name(f, $($arg),*),
        }
    };
}

impl Tx<'_> {
    /// R3730000: a new lobby. False when a lobby already holds the code (the caller mints another).
    pub async fn lobbies_create(&mut self, lobby: &Lobby) -> StoreResult<bool> {
        lobby_dispatch!(self, create(lobby))
    }

    /// The lobby under `code` (normalized), started or not, expired or not.
    pub async fn lobbies_get(&mut self, code: &str) -> StoreResult<Option<Lobby>> {
        lobby_dispatch!(self, get(code))
    }

    /// Writes `next`'s every column but its code, host and creation over the row whose revision is
    /// still `expected_rev`. False when the row is gone or another write landed first.
    pub async fn lobbies_update(&mut self, next: &Lobby, expected_rev: i64) -> StoreResult<bool> {
        lobby_dispatch!(self, update(next, expected_rev))
    }

    /// R3730004: deletes the lobby (the host left). False when there was none.
    pub async fn lobbies_delete(&mut self, code: &str) -> StoreResult<bool> {
        lobby_dispatch!(self, delete(code))
    }

    /// R890000, R3730004: `host`'s unstarted, unexpired lobbies of one format, for
    /// `MAX_OPEN_ROOMS_PER_HOST`.
    pub async fn lobbies_count_open_by_host(
        &mut self,
        host: &str,
        format: &str,
        at: i64,
    ) -> StoreResult<i64> {
        lobby_dispatch!(self, count_open_by_host(host, format, at))
    }

    /// R3730004: the unexpired lobbies `profile_id` hosts or sits in, newest first, the code
    /// breaking a tie.
    pub async fn lobbies_of_member(&mut self, profile_id: &str, at: i64) -> StoreResult<Vec<Lobby>> {
        lobby_dispatch!(self, of_member(profile_id, at))
    }

    /// R3730004: deletes every lobby expired at `before`.
    pub async fn lobbies_reap(&mut self, before: i64) -> StoreResult<u64> {
        lobby_dispatch!(self, reap(before))
    }
}

/// Migration 0090: a deleted profile takes the lobbies it hosts or sits in (`on delete cascade`).
pub fn remove_lobby_rows(lobbies: &mut Vec<Lobby>, profile_id: &str) {
    lobbies.retain(|lobby| lobby.host != profile_id && lobby.guest.as_deref() != Some(profile_id));
}

mod pg {
    use serde_json::Value;
    use sqlx::Row;
    use sqlx::postgres::PgRow;
    use time::OffsetDateTime;

    use super::{Lobby, LobbyPick};
    use crate::auth::is_uuid;
    // `db::pg`'s session and row helpers: every method re-stamps its subject first (`run_as`).
    use crate::db::pg::{PgTx, ms_of, run_as, uuid_text, uuid_text_or_null};
    use crate::db::store::StoreError;

    fn json_text<T: serde::Serialize + ?Sized>(value: &T) -> Result<String, StoreError> {
        Ok(serde_json::to_string(value)?)
    }

    fn pick_text(pick: &Option<LobbyPick>) -> Result<Option<String>, StoreError> {
        pick.as_ref().map(json_text).transpose()
    }

    macro_rules! lobby_columns {
        () => {
            "code, host, guest, settings, host_pick, guest_pick, host_ready, guest_ready, rev,
             catalog_version, created_at, expires_at, match_id, series_id, started_at"
        };
    }

    fn lobby_of(row: &PgRow) -> Result<Lobby, StoreError> {
        let pick = |column: &str| -> Result<Option<LobbyPick>, StoreError> {
            let value: Option<Value> = row.try_get(column)?;
            Ok(value.map(serde_json::from_value).transpose()?)
        };
        let started_at: Option<OffsetDateTime> = row.try_get("started_at")?;
        Ok(Lobby {
            code: row.try_get("code")?,
            host: uuid_text(row, "host")?,
            guest: uuid_text_or_null(row, "guest")?,
            settings: serde_json::from_value(row.try_get::<Value, _>("settings")?)?,
            host_pick: pick("host_pick")?,
            guest_pick: pick("guest_pick")?,
            host_ready: row.try_get("host_ready")?,
            guest_ready: row.try_get("guest_ready")?,
            rev: row.try_get("rev")?,
            catalog_version: row.try_get("catalog_version")?,
            created_at: ms_of(row.try_get("created_at")?),
            expires_at: ms_of(row.try_get("expires_at")?),
            match_id: uuid_text_or_null(row, "match_id")?,
            series_id: uuid_text_or_null(row, "series_id")?,
            started_at: started_at.map(ms_of),
        })
    }

    pub async fn create(t: &mut PgTx<'_>, lobby: &Lobby) -> Result<bool, StoreError> {
        run_as(t, Some(lobby.host.as_str())).await?;
        let done = sqlx::query(
            "insert into public.lobbies (code, host, settings, rev, catalog_version, created_at, expires_at)
             values ($1::text, $2::uuid, $3::jsonb, $4::bigint, $5::text,
                     to_timestamp($6::double precision / 1000.0),
                     to_timestamp($7::double precision / 1000.0))
             on conflict (code) do nothing",
        )
        .bind(lobby.code.as_str())
        .bind(lobby.host.as_str())
        .bind(json_text(&lobby.settings)?)
        .bind(lobby.rev)
        .bind(lobby.catalog_version.as_str())
        .bind(lobby.created_at)
        .bind(lobby.expires_at)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected() == 1)
    }

    pub async fn get(t: &mut PgTx<'_>, code: &str) -> Result<Option<Lobby>, StoreError> {
        run_as(t, None).await?;
        let row = sqlx::query(concat!(
            "select ",
            lobby_columns!(),
            " from public.lobbies where code = $1::text"
        ))
        .bind(code)
        .fetch_optional(&mut **t)
        .await?;
        row.as_ref().map(lobby_of).transpose()
    }

    pub async fn update(t: &mut PgTx<'_>, next: &Lobby, expected_rev: i64) -> Result<bool, StoreError> {
        run_as(t, Some(next.host.as_str())).await?;
        let done = sqlx::query(
            "update public.lobbies set
                 guest = $2::uuid, settings = $3::jsonb, host_pick = $4::jsonb, guest_pick = $5::jsonb,
                 host_ready = $6::boolean, guest_ready = $7::boolean, rev = $8::bigint,
                 catalog_version = $9::text,
                 expires_at = to_timestamp($10::double precision / 1000.0),
                 match_id = $11::uuid, series_id = $12::uuid,
                 started_at = to_timestamp($13::double precision / 1000.0)
              where code = $1::text and rev = $14::bigint",
        )
        .bind(next.code.as_str())
        .bind(next.guest.as_deref())
        .bind(json_text(&next.settings)?)
        .bind(pick_text(&next.host_pick)?)
        .bind(pick_text(&next.guest_pick)?)
        .bind(next.host_ready)
        .bind(next.guest_ready)
        .bind(next.rev)
        .bind(next.catalog_version.as_str())
        .bind(next.expires_at)
        .bind(next.match_id.as_deref())
        .bind(next.series_id.as_deref())
        .bind(next.started_at)
        .bind(expected_rev)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected() == 1)
    }

    pub async fn delete(t: &mut PgTx<'_>, code: &str) -> Result<bool, StoreError> {
        run_as(t, None).await?;
        let done = sqlx::query("delete from public.lobbies where code = $1::text")
            .bind(code)
            .execute(&mut **t)
            .await?;
        Ok(done.rows_affected() == 1)
    }

    pub async fn count_open_by_host(
        t: &mut PgTx<'_>,
        host: &str,
        format: &str,
        at: i64,
    ) -> Result<i64, StoreError> {
        run_as(t, Some(host)).await?;
        if !is_uuid(host) {
            return Ok(0);
        }
        let row = sqlx::query(
            "select count(*)::bigint as n from public.lobbies
              where host = $1::uuid and settings->>'format' = $2::text and started_at is null
                and expires_at > to_timestamp($3::double precision / 1000.0)",
        )
        .bind(host)
        .bind(format)
        .bind(at)
        .fetch_one(&mut **t)
        .await?;
        Ok(row.try_get::<i64, _>("n")?)
    }

    pub async fn of_member(t: &mut PgTx<'_>, profile_id: &str, at: i64) -> Result<Vec<Lobby>, StoreError> {
        run_as(t, Some(profile_id)).await?;
        if !is_uuid(profile_id) {
            return Ok(Vec::new());
        }
        let rows = sqlx::query(concat!(
            "select ",
            lobby_columns!(),
            " from public.lobbies
              where (host = $1::uuid or guest = $1::uuid)
                and expires_at > to_timestamp($2::double precision / 1000.0)
              order by created_at desc, code"
        ))
        .bind(profile_id)
        .bind(at)
        .fetch_all(&mut **t)
        .await?;
        rows.iter().map(lobby_of).collect()
    }

    pub async fn reap(t: &mut PgTx<'_>, before: i64) -> Result<u64, StoreError> {
        run_as(t, None).await?;
        let done = sqlx::query(
            "delete from public.lobbies where expires_at <= to_timestamp($1::double precision / 1000.0)",
        )
        .bind(before)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected())
    }
}

mod fake {
    use super::Lobby;
    use crate::db::fake::FakeTx;
    use crate::db::store::StoreError;

    /// Fault-injection and race hooks for fake calls (`db::fake`).
    fn call<'t>(f: &'t mut FakeTx<'_>, method: &str) -> Result<&'t mut Vec<Lobby>, StoreError> {
        crate::db::fake::call(f, method)?;
        Ok(&mut f.tables().lobbies)
    }

    pub fn create(f: &mut FakeTx<'_>, lobby: &Lobby) -> Result<bool, StoreError> {
        let lobbies = call(f, "lobbies.create")?;
        if lobbies.iter().any(|held| held.code == lobby.code) {
            return Ok(false);
        }
        // What the insert writes; the rest is the columns' defaults.
        lobbies.push(Lobby {
            guest: None,
            host_pick: None,
            guest_pick: None,
            host_ready: false,
            guest_ready: false,
            match_id: None,
            series_id: None,
            started_at: None,
            ..lobby.clone()
        });
        Ok(true)
    }

    pub fn get(f: &mut FakeTx<'_>, code: &str) -> Result<Option<Lobby>, StoreError> {
        let lobbies = call(f, "lobbies.get")?;
        Ok(lobbies.iter().find(|lobby| lobby.code == code).cloned())
    }

    pub fn update(f: &mut FakeTx<'_>, next: &Lobby, expected_rev: i64) -> Result<bool, StoreError> {
        let lobbies = call(f, "lobbies.update")?;
        let Some(row) = lobbies
            .iter_mut()
            .find(|lobby| lobby.code == next.code && lobby.rev == expected_rev)
        else {
            return Ok(false);
        };
        // Postgres's check: a lobby's guest is never its host.
        if next.guest.as_deref() == Some(row.host.as_str()) {
            return Err(StoreError::from("lobbies_guest_not_host_check"));
        }
        *row = Lobby {
            code: row.code.clone(),
            host: row.host.clone(),
            created_at: row.created_at,
            ..next.clone()
        };
        Ok(true)
    }

    pub fn delete(f: &mut FakeTx<'_>, code: &str) -> Result<bool, StoreError> {
        let lobbies = call(f, "lobbies.delete")?;
        let before = lobbies.len();
        lobbies.retain(|lobby| lobby.code != code);
        Ok(lobbies.len() != before)
    }

    pub fn count_open_by_host(
        f: &mut FakeTx<'_>,
        host: &str,
        format: &str,
        at: i64,
    ) -> Result<i64, StoreError> {
        let lobbies = call(f, "lobbies.countOpenByHost")?;
        Ok(lobbies
            .iter()
            .filter(|lobby| {
                lobby.host == host
                    && lobby.settings.format() == format
                    && lobby.started_at.is_none()
                    && lobby.expires_at > at
            })
            .count() as i64)
    }

    pub fn of_member(f: &mut FakeTx<'_>, profile_id: &str, at: i64) -> Result<Vec<Lobby>, StoreError> {
        let lobbies = call(f, "lobbies.ofMember")?;
        let mut held: Vec<Lobby> = lobbies
            .iter()
            .filter(|lobby| {
                (lobby.host == profile_id || lobby.guest.as_deref() == Some(profile_id))
                    && lobby.expires_at > at
            })
            .cloned()
            .collect();
        held.sort_by(|a, b| b.created_at.cmp(&a.created_at).then_with(|| a.code.cmp(&b.code)));
        Ok(held)
    }

    pub fn reap(f: &mut FakeTx<'_>, before: i64) -> Result<u64, StoreError> {
        let lobbies = call(f, "lobbies.reap")?;
        let held = lobbies.len();
        lobbies.retain(|lobby| lobby.expires_at > before);
        Ok((held - lobbies.len()) as u64)
    }
}
