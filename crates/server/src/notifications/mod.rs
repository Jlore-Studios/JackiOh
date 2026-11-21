//! The in-game notification feed (SPEC §9.4, §9.13; R4039000; issue #4039): what happened to an
//! account, kept until it has seen it, in place of email. One row per event for one account
//! (`public.notifications`, migration 0093; `store`), written by the code that already handles the
//! event, after that code's own commit (`notify`):
//!
//! - `dailyMove`: a daily game began waiting on its recipient's move (`daily::games`, `daily::invites`'s
//!   seek); `dailyEnded`: a daily game ended, on time and by concession included, told to both players
//!   with the result each sees (`daily::games`);
//! - `dailyChallenge`, `dailyAnswered`: a daily challenge sent, and accepted or declined (`daily::invites`);
//! - `friendRequest`, `friendAccepted`: a friend request sent, and accepted (`social::friends`);
//! - `lobbyInvite`: a friend's lobby invite (`lobby`), listed while its lobby is open and unstarted;
//! - `replayShared`: a friend shared a replay (`social::shares`).
//!
//! A row replaces its recipient's unread row of the same kind, actor and subject, so a daily game's
//! newer move never stacks; nobody is told of their own act, and a block either way writes nothing
//! and deletes the rows between the two, either way (R4348000).
//! Each account keeps its newest `NOTIFICATIONS_KEEP` rows, and the daily sweep deletes rows older than
//! `NOTIFICATIONS_KEEP_DAYS` (`daily::notify::sweep`).
//!
//! - `GET /api/notifications?before=<id>&limit=<n>`: `{notifications, nextBefore, unread}`, the caller's
//!   own rows, newest first, a page at a time.
//! - `POST /api/notifications/read` `{upTo?}`: marks the caller's rows up to that id (inclusive), or all
//!   of them, read: `{unread}`.
//! - `/ws/social` sends each new row to its recipient's tabs as a `notification` frame
//!   (`social::live::send_notification`), and the social read counts them as `notificationsUnread`.

pub mod store;

use std::sync::Arc;

use serde_json::{Value, json};

use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiResult, Req, bad_request, now_ms, ok};
use crate::app::App;
use crate::config::{NOTIFICATIONS_KEEP, NOTIFICATIONS_PAGE, NOTIFICATIONS_PAGE_MAX};
use crate::social::live::{push_social, send_notification};
use crate::social::{PlayerRef, profiles_by_id};
use store::{NewNotification, NotificationKind, NotificationRow};

/// One event to tell one account of: `to` is told, `actor` did it, `subject` is what it is about.
pub struct Notice {
    pub to: String,
    pub kind: NotificationKind,
    pub actor: Option<String>,
    pub subject: Option<String>,
    pub payload: Value,
    /// R4233000: written already read (nobody's dot lights for their own act).
    pub read: bool,
}

impl Notice {
    /// `kind`, done by `actor` to `to`, about `subject`, with no words of its own.
    pub fn new(kind: NotificationKind, to: &str, actor: &str, subject: &str) -> Notice {
        Notice {
            to: to.to_string(),
            kind,
            actor: Some(actor.to_string()),
            subject: Some(subject.to_string()),
            payload: json!({}),
            read: false,
        }
    }

    /// The notice with `payload` for its words.
    pub fn with(self, payload: Value) -> Notice {
        Notice { payload, ..self }
    }

    /// The notice written already read: listed under Recent, never a toast, never unread.
    pub fn read(self) -> Notice {
        Notice { read: true, ..self }
    }
}

/// R4039000: writes `notices` in one transaction of their own, then sends each row written to its
/// recipient's social sockets and their social read again. A notice its own actor would get, or one
/// between two accounts a block stands between (either way), writes nothing. Called after the event's
/// own commit, on its success path only; a failure is logged and the notices are lost, never the
/// event's failure.
pub async fn notify(app: &App, notices: Vec<Notice>) {
    if notices.is_empty() {
        return;
    }
    match write(app, notices).await {
        Ok(written) => {
            for (row, frame) in &written {
                // R4233000: a row written already read is history, not news: no toast for it.
                if row.read_at.is_none() {
                    send_notification(app, &row.profile_id, frame);
                }
            }
            let mut told: Vec<&str> = written.iter().map(|(row, _)| row.profile_id.as_str()).collect();
            told.sort_unstable();
            told.dedup();
            push_social(app, &told).await;
        }
        Err(error) => {
            tracing::warn!(event = "notifications.write_failed", message = %error.message);
        }
    }
}

/// R4233000: marks the caller's unread rows of `kinds` about `subject` (by `actor` when
/// `Some`) read, after the act that dealt with them committed and in a transaction of its own.
/// A mark that changed any row pushes the caller's social frame, so every one of their tabs drops
/// the dot. A failure is logged, as `notify`'s is, and never fails the caller.
pub async fn mark_subject(
    app: &App,
    profile_id: &str,
    kinds: &[NotificationKind],
    subject: &str,
    actor: Option<&str>,
) {
    let changed = async {
        let mut tx = app.db.begin(Some(profile_id)).await?;
        let changed = tx
            .notifications_mark_subject(profile_id, kinds, subject, actor, now_ms())
            .await?;
        tx.commit().await?;
        Ok::<u64, ApiError>(changed)
    }
    .await;
    match changed {
        Ok(0) => {}
        Ok(_) => push_social(app, &[profile_id]).await,
        Err(error) => {
            tracing::warn!(event = "notifications.mark_failed", message = %error.message);
        }
    }
}

/// `notify`'s writes: each row written, with its JSON as its recipient reads it.
async fn write(app: &App, notices: Vec<Notice>) -> Result<Vec<(NotificationRow, Value)>, ApiError> {
    let now = now_ms();
    let mut tx = app.db.begin(None).await?;
    let mut rows = Vec::new();
    for notice in notices {
        if let Some(actor) = notice.actor.as_deref()
            && (actor == notice.to || tx.blocks_between(actor, &notice.to).await?)
        {
            continue;
        }
        let new = NewNotification {
            profile_id: notice.to,
            kind: notice.kind,
            actor_id: notice.actor,
            subject: notice.subject,
            payload: notice.payload,
            created_at: now,
            read: notice.read,
        };
        rows.push(tx.notifications_add(&new, NOTIFICATIONS_KEEP).await?);
    }
    let actors: Vec<String> = rows.iter().filter_map(|row| row.actor_id.clone()).collect();
    let people = profiles_by_id(&mut tx, &actors).await?;
    tx.commit().await?;
    Ok(rows
        .into_iter()
        .map(|row| {
            let actor = row
                .actor_id
                .as_deref()
                .and_then(|id| people.get(id))
                .map(PlayerRef::of);
            let frame = notification_json(&row, actor.as_ref());
            (row, frame)
        })
        .collect())
}

/// One row as its recipient reads it: `{id, kind, actor: {profileId, username} | null, subject,
/// payload, createdAt, read}`. Nothing in it is anyone's hidden information: the actor is a
/// `PlayerRef`, and the payload holds only what its recipient may already read (a result, a format).
pub fn notification_json(row: &NotificationRow, actor: Option<&PlayerRef>) -> Value {
    json!({
        "id": row.id,
        "kind": row.kind,
        "actor": actor,
        "subject": row.subject,
        "payload": row.payload,
        "createdAt": row.created_at,
        "read": row.read_at.is_some(),
    })
}

/// A query parameter that, when present, must be a whole number in `min..=max`.
fn query_number(req: &Req, name: &str, min: i64, max: i64) -> Result<Option<i64>, ApiError> {
    let Some(text) = req.query.get(name) else {
        return Ok(None);
    };
    text.parse::<i64>()
        .ok()
        .filter(|n| (min..=max).contains(n))
        .map(Some)
        .ok_or_else(|| bad_request(format!("\"{name}\" must be a whole number from {min} to {max}")))
}

/// `GET /api/notifications?before=<id>&limit=<n>`: `200 {notifications, nextBefore, unread}`, the
/// caller's rows with an id below `before` (the newest without it), newest first, `limit` of them
/// (`NOTIFICATIONS_PAGE` without it, at most `NOTIFICATIONS_PAGE_MAX`); `nextBefore` is the `before` of
/// the page after, null on the last. R4039000: a `lobbyInvite` whose lobby is gone, started or expired
/// is deleted as it is read, never listed.
pub async fn list(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let before = query_number(&req, "before", 1, i64::MAX)?;
    let limit = query_number(&req, "limit", 1, NOTIFICATIONS_PAGE_MAX)?.unwrap_or(NOTIFICATIONS_PAGE);
    let now = now_ms();
    let mut tx = app.db.begin(Some(&me)).await?;
    let mut rows = tx.notifications_page(&me, before, limit + 1).await?;
    let next_before = if rows.len() > usize::try_from(limit).unwrap_or(0) {
        rows.truncate(usize::try_from(limit).unwrap_or(0));
        rows.last().map(|row| row.id)
    } else {
        None
    };
    let mut closed = Vec::new();
    for row in rows
        .iter()
        .filter(|row| row.kind == NotificationKind::LobbyInvite)
    {
        let open = match row.subject.as_deref() {
            Some(code) => tx
                .lobbies_get(code)
                .await?
                .is_some_and(|lobby| lobby.open_at(now)),
            None => false,
        };
        if !open {
            closed.push(row.id);
        }
    }
    if !closed.is_empty() {
        tx.notifications_delete(&me, &closed).await?;
        rows.retain(|row| !closed.contains(&row.id));
    }
    let unread = tx.notifications_unread(&me, now).await?;
    let actors: Vec<String> = rows.iter().filter_map(|row| row.actor_id.clone()).collect();
    let people = profiles_by_id(&mut tx, &actors).await?;
    tx.commit().await?;
    if !closed.is_empty() {
        // A dropped invite may have been the only unread row: the bell's dot follows the count again.
        push_social(app, &[&me]).await;
    }
    let notifications: Vec<Value> = rows
        .iter()
        .map(|row| {
            let actor = row
                .actor_id
                .as_deref()
                .and_then(|id| people.get(id))
                .map(PlayerRef::of);
            notification_json(row, actor.as_ref())
        })
        .collect();
    Ok(ok(json!({
        "notifications": notifications,
        "nextBefore": next_before,
        "unread": unread,
    })))
}

/// `POST /api/notifications/read` `{upTo?}`: `200 {unread}`. Marks the caller's unread rows with an id
/// up to `upTo`, inclusive (every one when it is absent or null), read; nobody else's. The caller's
/// other tabs hear the count through their social read.
pub async fn mark_read(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let up_to = match req.body.get("upTo") {
        None | Some(Value::Null) => None,
        Some(value) => Some(
            value
                .as_i64()
                .filter(|id| *id > 0)
                .ok_or_else(|| bad_request("\"upTo\" must be a notification's id"))?,
        ),
    };
    let now = now_ms();
    let mut tx = app.db.begin(Some(&me)).await?;
    tx.notifications_mark_read(&me, up_to, now).await?;
    let unread = tx.notifications_unread(&me, now).await?;
    tx.commit().await?;
    push_social(app, &[&me]).await;
    Ok(ok(json!({ "unread": unread })))
}
