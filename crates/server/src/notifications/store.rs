//! The notification feed's store (SPEC §9.4, §9.13; migration 0093; R4039000; issue #4039):
//! `public.notifications` over Postgres (`pg` below) or the in-memory fake (`fake` below, whose rows
//! are `FakeTables::notifications`).
//!
//! Same shape as `lobby::store`: each store method is a `Tx` method `notifications_<verb>` dispatching
//! to this file's `pg::<name>` or `fake::<name>`, in the caller's transaction. Ids are the table's
//! identity, so a newer row always has the larger id; timestamps are epoch milliseconds.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::db::store::{StoreResult, Tx};

/// R4039000: what a feed row tells of, as migration 0093's `notifications_kind_check` spells it.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum NotificationKind {
    /// A daily game began waiting on its recipient's move.
    DailyMove,
    /// A daily game its recipient played ended.
    DailyEnded,
    /// A friend challenged its recipient to a daily game.
    DailyChallenge,
    /// Its recipient's daily challenge was accepted or declined.
    DailyAnswered,
    /// Someone asked its recipient to be friends.
    FriendRequest,
    /// Its recipient's friend request was accepted.
    FriendAccepted,
    /// A friend invited its recipient into their lobby.
    LobbyInvite,
    /// A friend shared a replay with its recipient.
    ReplayShared,
}

impl NotificationKind {
    /// The kind's word, the column's text.
    pub fn as_str(self) -> &'static str {
        match self {
            NotificationKind::DailyMove => "dailyMove",
            NotificationKind::DailyEnded => "dailyEnded",
            NotificationKind::DailyChallenge => "dailyChallenge",
            NotificationKind::DailyAnswered => "dailyAnswered",
            NotificationKind::FriendRequest => "friendRequest",
            NotificationKind::FriendAccepted => "friendAccepted",
            NotificationKind::LobbyInvite => "lobbyInvite",
            NotificationKind::ReplayShared => "replayShared",
        }
    }
}

/// A feed row before it is written: whom it tells, of what, by whom, about what, and its words.
/// R4233000: `read` writes it already read (the actor's own game-ending row): it is listed, but no
/// toast goes out for it and it never counts as unread.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NewNotification {
    pub profile_id: String,
    pub kind: NotificationKind,
    pub actor_id: Option<String>,
    pub subject: Option<String>,
    pub payload: Value,
    pub created_at: i64,
    pub read: bool,
}

/// One row of `public.notifications` (R4039000).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationRow {
    pub id: i64,
    pub profile_id: String,
    pub kind: NotificationKind,
    pub actor_id: Option<String>,
    pub subject: Option<String>,
    pub payload: Value,
    pub created_at: i64,
    /// When its recipient's inbox marked it read; `None` while it is unread.
    pub read_at: Option<i64>,
}

impl NotificationRow {
    /// Whether `new` replaces this row as it is written: this row is still unread and tells the same
    /// account of the same kind, by the same actor, about the same subject (R4039000).
    fn replaced_by(&self, new: &NewNotification) -> bool {
        self.read_at.is_none()
            && self.profile_id == new.profile_id
            && self.kind == new.kind
            && self.actor_id == new.actor_id
            && self.subject == new.subject
    }
}

/// The fake's rows and the identity it hands the next one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NotificationTables {
    pub rows: Vec<NotificationRow>,
    pub next_id: i64,
}

/// Dispatches to `pg` or `fake`.
macro_rules! notifications_dispatch {
    ($self:ident, $name:ident ( $($arg:expr),* $(,)? )) => {
        match $self {
            Tx::Pg(t) => pg::$name(t, $($arg),*).await,
            Tx::Fake(f) => fake::$name(f, $($arg),*),
        }
    };
}

impl Tx<'_> {
    /// R4039000: writes `new` and answers the row written. Its recipient's unread row of the same
    /// kind, actor and subject goes first (a daily game's newer move replaces the one before), and
    /// once it is written the recipient keeps only its newest `keep` rows.
    pub async fn notifications_add(
        &mut self,
        new: &NewNotification,
        keep: i64,
    ) -> StoreResult<NotificationRow> {
        notifications_dispatch!(self, add(new, keep))
    }

    /// `profile_id`'s rows with an id below `before` (every row for `None`), newest first, at most
    /// `limit` of them.
    pub async fn notifications_page(
        &mut self,
        profile_id: &str,
        before: Option<i64>,
        limit: i64,
    ) -> StoreResult<Vec<NotificationRow>> {
        notifications_dispatch!(self, page(profile_id, before, limit))
    }

    /// How many of `profile_id`'s rows are unread at `now`. R4233000: a `lobbyInvite` whose lobby
    /// is gone, started or expired counts nowhere, the same test `list` reads by.
    pub async fn notifications_unread(&mut self, profile_id: &str, now: i64) -> StoreResult<i64> {
        notifications_dispatch!(self, unread(profile_id, now))
    }

    /// Marks `profile_id`'s unread rows with an id up to `up_to`, inclusive (every one for `None`),
    /// read at `at`: how many it marked. Nobody else's row is touched.
    pub async fn notifications_mark_read(
        &mut self,
        profile_id: &str,
        up_to: Option<i64>,
        at: i64,
    ) -> StoreResult<u64> {
        notifications_dispatch!(self, mark_read(profile_id, up_to, at))
    }

    /// R4233000: marks `profile_id`'s unread rows of one of `kinds` about `subject` (by `actor`
    /// when `Some`), read at `at`: how many it marked. Opening or answering what a row is about
    /// calls this, so the bell's dot follows only news not yet dealt with. Deletes nothing, never
    /// touches another account's rows, never marks a read row unread.
    pub async fn notifications_mark_subject(
        &mut self,
        profile_id: &str,
        kinds: &[NotificationKind],
        subject: &str,
        actor: Option<&str>,
        at: i64,
    ) -> StoreResult<u64> {
        notifications_dispatch!(self, mark_subject(profile_id, kinds, subject, actor, at))
    }

    /// Deletes `profile_id`'s rows among `ids` (a lobby invite whose lobby is gone): how many.
    pub async fn notifications_delete(&mut self, profile_id: &str, ids: &[i64]) -> StoreResult<u64> {
        notifications_dispatch!(self, delete(profile_id, ids))
    }

    /// R4039000: deletes every row written before `before`, read or not: how many.
    pub async fn notifications_purge(&mut self, before: i64) -> StoreResult<u64> {
        notifications_dispatch!(self, purge(before))
    }
}

/// Migration 0093: a deleted profile takes the rows that tell it and the rows it is the actor of
/// (`on delete cascade`, both ways).
pub fn remove_notification_rows(tables: &mut NotificationTables, profile_id: &str) {
    tables
        .rows
        .retain(|row| row.profile_id != profile_id && row.actor_id.as_deref() != Some(profile_id));
}

mod pg {
    use serde_json::Value;
    use sqlx::Row;
    use sqlx::postgres::PgRow;

    use super::{NewNotification, NotificationKind, NotificationRow};
    use crate::auth::is_uuid;
    // `db::pg`'s session and row helpers: every method re-stamps its subject first (`run_as`).
    use crate::db::pg::{PgTx, json, run_as, uuid_text, uuid_text_or_null};
    use crate::db::store::StoreError;

    macro_rules! notification_columns {
        () => {
            "id, profile_id, kind, actor_id, subject, payload, created_at, read_at"
        };
    }

    fn notification_of(row: &PgRow) -> Result<NotificationRow, StoreError> {
        let kind: String = row.try_get("kind")?;
        Ok(NotificationRow {
            id: row.try_get("id")?,
            profile_id: uuid_text(row, "profile_id")?,
            kind: serde_json::from_value(Value::String(kind))?,
            actor_id: uuid_text_or_null(row, "actor_id")?,
            subject: row.try_get("subject")?,
            payload: row.try_get::<Value, _>("payload")?,
            created_at: row.try_get("created_at")?,
            read_at: row.try_get("read_at")?,
        })
    }

    pub async fn add(
        t: &mut PgTx<'_>,
        new: &NewNotification,
        keep: i64,
    ) -> Result<NotificationRow, StoreError> {
        run_as(t, None).await?;
        sqlx::query(
            "delete from public.notifications
              where profile_id = $1::uuid and kind = $2::text and read_at is null
                and actor_id is not distinct from $3::uuid and subject is not distinct from $4::text",
        )
        .bind(new.profile_id.as_str())
        .bind(new.kind.as_str())
        .bind(new.actor_id.as_deref())
        .bind(new.subject.as_deref())
        .execute(&mut **t)
        .await?;
        let written = sqlx::query(concat!(
            "insert into public.notifications (profile_id, kind, actor_id, subject, payload, created_at, read_at)
             values ($1::uuid, $2::text, $3::uuid, $4::text, $5::jsonb, $6::bigint,
                     case when $7::boolean then $6::bigint end)
             returning ",
            notification_columns!()
        ))
        .bind(new.profile_id.as_str())
        .bind(new.kind.as_str())
        .bind(new.actor_id.as_deref())
        .bind(new.subject.as_deref())
        .bind(json(&new.payload)?)
        .bind(new.created_at)
        .bind(new.read)
        .fetch_one(&mut **t)
        .await?;
        sqlx::query(
            "delete from public.notifications
              where profile_id = $1::uuid
                and id not in (select id from public.notifications where profile_id = $1::uuid
                                order by id desc limit $2::bigint)",
        )
        .bind(new.profile_id.as_str())
        .bind(keep)
        .execute(&mut **t)
        .await?;
        notification_of(&written)
    }

    pub async fn page(
        t: &mut PgTx<'_>,
        profile_id: &str,
        before: Option<i64>,
        limit: i64,
    ) -> Result<Vec<NotificationRow>, StoreError> {
        run_as(t, None).await?;
        if !is_uuid(profile_id) {
            return Ok(Vec::new());
        }
        let rows = sqlx::query(concat!(
            "select ",
            notification_columns!(),
            " from public.notifications
              where profile_id = $1::uuid and ($2::bigint is null or id < $2::bigint)
              order by id desc
              limit $3::bigint"
        ))
        .bind(profile_id)
        .bind(before)
        .bind(limit)
        .fetch_all(&mut **t)
        .await?;
        rows.iter().map(notification_of).collect()
    }

    pub async fn unread(t: &mut PgTx<'_>, profile_id: &str, now: i64) -> Result<i64, StoreError> {
        run_as(t, None).await?;
        if !is_uuid(profile_id) {
            return Ok(0);
        }
        // R4233000: one statement, as `list` reads it: a `lobbyInvite` counts only while its lobby
        // is there, unstarted and unexpired.
        let row = sqlx::query(
            "select count(*)::bigint as n from public.notifications n
              left join public.lobbies l on l.code = n.subject and n.kind = 'lobbyInvite'
              where n.profile_id = $1::uuid and n.read_at is null
                and (n.kind <> 'lobbyInvite'
                     or (l.code is not null and l.started_at is null
                         and l.expires_at > to_timestamp($2::double precision / 1000.0)))",
        )
        .bind(profile_id)
        .bind(now as f64)
        .fetch_one(&mut **t)
        .await?;
        Ok(row.try_get::<i64, _>("n")?)
    }

    pub async fn mark_read(
        t: &mut PgTx<'_>,
        profile_id: &str,
        up_to: Option<i64>,
        at: i64,
    ) -> Result<u64, StoreError> {
        run_as(t, None).await?;
        if !is_uuid(profile_id) {
            return Ok(0);
        }
        let done = sqlx::query(
            "update public.notifications set read_at = $3::bigint
              where profile_id = $1::uuid and read_at is null
                and ($2::bigint is null or id <= $2::bigint)",
        )
        .bind(profile_id)
        .bind(up_to)
        .bind(at)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected())
    }

    pub async fn mark_subject(
        t: &mut PgTx<'_>,
        profile_id: &str,
        kinds: &[NotificationKind],
        subject: &str,
        actor: Option<&str>,
        at: i64,
    ) -> Result<u64, StoreError> {
        run_as(t, None).await?;
        if !is_uuid(profile_id) {
            return Ok(0);
        }
        let kinds: Vec<&str> = kinds.iter().map(|kind| kind.as_str()).collect();
        let done = sqlx::query(
            "update public.notifications set read_at = $5::bigint
              where profile_id = $1::uuid and read_at is null
                and kind = any($2::text[])
                and subject is not distinct from $3::text
                and ($4::uuid is null or actor_id = $4::uuid)",
        )
        .bind(profile_id)
        .bind(kinds)
        .bind(subject)
        .bind(actor)
        .bind(at)
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected())
    }

    pub async fn delete(t: &mut PgTx<'_>, profile_id: &str, ids: &[i64]) -> Result<u64, StoreError> {
        run_as(t, None).await?;
        if !is_uuid(profile_id) || ids.is_empty() {
            return Ok(0);
        }
        let done = sqlx::query(
            "delete from public.notifications where profile_id = $1::uuid and id = any($2::bigint[])",
        )
        .bind(profile_id)
        .bind(ids.to_vec())
        .execute(&mut **t)
        .await?;
        Ok(done.rows_affected())
    }

    pub async fn purge(t: &mut PgTx<'_>, before: i64) -> Result<u64, StoreError> {
        run_as(t, None).await?;
        let done = sqlx::query("delete from public.notifications where created_at < $1::bigint")
            .bind(before)
            .execute(&mut **t)
            .await?;
        Ok(done.rows_affected())
    }
}

mod fake {
    use super::{NewNotification, NotificationRow, NotificationTables};
    use crate::db::fake::FakeTx;
    use crate::db::store::StoreError;

    /// Fault-injection and race hooks for fake calls (`db::fake`).
    fn call<'t>(f: &'t mut FakeTx<'_>, method: &str) -> Result<&'t mut NotificationTables, StoreError> {
        crate::db::fake::call(f, method)?;
        Ok(&mut f.tables().notifications)
    }

    /// `profile_id`'s rows, newest first.
    fn newest_first<'a>(
        rows: &'a [NotificationRow],
        profile_id: &'a str,
    ) -> impl Iterator<Item = &'a NotificationRow> {
        rows.iter().rev().filter(move |row| row.profile_id == profile_id)
    }

    pub fn add(f: &mut FakeTx<'_>, new: &NewNotification, keep: i64) -> Result<NotificationRow, StoreError> {
        let tables = call(f, "notifications.add")?;
        tables.rows.retain(|row| !row.replaced_by(new));
        tables.next_id += 1;
        let written = NotificationRow {
            id: tables.next_id,
            profile_id: new.profile_id.clone(),
            kind: new.kind,
            actor_id: new.actor_id.clone(),
            subject: new.subject.clone(),
            payload: new.payload.clone(),
            created_at: new.created_at,
            read_at: new.read.then_some(new.created_at),
        };
        // Ids only grow, so the rows stay in id order.
        tables.rows.push(written.clone());
        let kept: Vec<i64> = newest_first(&tables.rows, &new.profile_id)
            .take(usize::try_from(keep).unwrap_or(0))
            .map(|row| row.id)
            .collect();
        tables
            .rows
            .retain(|row| row.profile_id != new.profile_id || kept.contains(&row.id));
        Ok(written)
    }

    pub fn page(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        before: Option<i64>,
        limit: i64,
    ) -> Result<Vec<NotificationRow>, StoreError> {
        let tables = call(f, "notifications.page")?;
        Ok(newest_first(&tables.rows, profile_id)
            .filter(|row| before.is_none_or(|before| row.id < before))
            .take(usize::try_from(limit).unwrap_or(0))
            .cloned()
            .collect())
    }

    pub fn unread(f: &mut FakeTx<'_>, profile_id: &str, now: i64) -> Result<i64, StoreError> {
        crate::db::fake::call(f, "notifications.unread")?;
        let tables = f.tables();
        Ok(tables
            .notifications
            .rows
            .iter()
            .filter(|row| {
                row.profile_id == profile_id
                    && row.read_at.is_none()
                    && invite_open(&tables.lobbies, row, now)
            })
            .count() as i64)
    }

    /// R4233000: whether a row counts as unread: every kind but a dead `lobbyInvite` (its lobby
    /// gone, started or expired), the same test `list` reads by.
    fn invite_open(lobbies: &[crate::lobby::store::Lobby], row: &NotificationRow, now: i64) -> bool {
        if row.kind != super::NotificationKind::LobbyInvite {
            return true;
        }
        let Some(subject) = row.subject.as_deref() else {
            return false;
        };
        lobbies
            .iter()
            .any(|lobby| lobby.code == subject && lobby.started_at.is_none() && lobby.expires_at > now)
    }

    pub fn mark_read(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        up_to: Option<i64>,
        at: i64,
    ) -> Result<u64, StoreError> {
        let tables = call(f, "notifications.markRead")?;
        let mut marked = 0;
        for row in tables.rows.iter_mut().filter(|row| {
            row.profile_id == profile_id && row.read_at.is_none() && up_to.is_none_or(|up_to| row.id <= up_to)
        }) {
            row.read_at = Some(at);
            marked += 1;
        }
        Ok(marked)
    }

    pub fn mark_subject(
        f: &mut FakeTx<'_>,
        profile_id: &str,
        kinds: &[super::NotificationKind],
        subject: &str,
        actor: Option<&str>,
        at: i64,
    ) -> Result<u64, StoreError> {
        let tables = call(f, "notifications.markSubject")?;
        let mut marked = 0;
        for row in tables.rows.iter_mut().filter(|row| {
            row.profile_id == profile_id
                && row.read_at.is_none()
                && kinds.contains(&row.kind)
                && row.subject.as_deref() == Some(subject)
                && actor.is_none_or(|actor| row.actor_id.as_deref() == Some(actor))
        }) {
            row.read_at = Some(at);
            marked += 1;
        }
        Ok(marked)
    }

    pub fn delete(f: &mut FakeTx<'_>, profile_id: &str, ids: &[i64]) -> Result<u64, StoreError> {
        let tables = call(f, "notifications.delete")?;
        let held = tables.rows.len();
        tables
            .rows
            .retain(|row| row.profile_id != profile_id || !ids.contains(&row.id));
        Ok((held - tables.rows.len()) as u64)
    }

    pub fn purge(f: &mut FakeTx<'_>, before: i64) -> Result<u64, StoreError> {
        let tables = call(f, "notifications.purge")?;
        let held = tables.rows.len();
        tables.rows.retain(|row| row.created_at >= before);
        Ok((held - tables.rows.len()) as u64)
    }
}
