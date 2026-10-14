//! Challenges (SPEC S7, B48, B49): a challenge to a friend is a room code sent to them.
//!
//! - `POST /api/challenges` takes the body `POST /api/rooms` takes, plus `friendId`. Only a friend
//!   neither side blocked may be challenged (else 404); one in a match, a series or a co-op game is
//!   409 `busy`, one not online 409 `offline` (B47's status); a second open challenge to the same
//!   friend is 409 `already_challenged`. Then the room is opened exactly as `POST /api/rooms` opens
//!   it (`actor::rooms::open_room`: the same body, mode rules and refusals), open for
//!   `CHALLENGE_TTL_MS` instead of `ROOM_CODE_TTL_SECONDS`, so it lapses with the challenge, and the
//!   answer is `201 {challengeId, roomCode}`.
//! - `GET /api/challenges`: the caller's open challenges that have not lapsed, received and sent.
//! - Accepting is joining the room with its code through the existing `POST /api/rooms/:code/join`,
//!   whose claim marks the challenge accepted in its own transaction (`challenges_accept_room`). A
//!   challenge's room is its friend's to join: anyone else meets the miss an unknown code is
//!   (`assert_may_join`).
//! - `POST /api/challenges/:id/decline` (its friend) and `DELETE /api/challenges/:id` (its
//!   challenger) close it and free its room (`actor::rooms::release_room`); 204, or 404 for anyone
//!   else. Answering one that is no longer open changes nothing and is 204 too.
//! - A challenge lapses `CHALLENGE_TTL_MS` after it was made, its room with it; the sweeper
//!   (`run_sweeper`) then marks it expired.

use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde_json::json;

use crate::actor::rooms::{open_room, release_room};
use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiResult, Req, log_info, log_warn, now_ms, ok, str, to_json};
use crate::api::queue::new_uuid;
use crate::app::App;
use crate::config::{CHALLENGE_SWEEP_INTERVAL_SECONDS, CHALLENGE_TTL_MS};
use crate::db::store::{ChallengeRow, ChallengeState, QueueMode};
use crate::social::friends::are_friends;
use crate::social::presence::{Status, standings};
use crate::social::{PlayerRef, created, iso, no_content, not_found, param, profiles_by_id, refused};

/// S7's `Challenge`.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Challenge {
    pub challenge_id: String,
    pub from: PlayerRef,
    pub to: PlayerRef,
    pub room_code: String,
    pub mode: QueueMode,
    /// When it lapses, its room with it, ISO-8601.
    pub expires_at: String,
}

/// `POST /api/challenges` (B48): `201 {challengeId, roomCode}`.
pub async fn create(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let friend_id = str(&req.body, "friendId")?;
    let now = now_ms();

    let mut tx = app.db.begin(Some(&me)).await?;
    let friend = if are_friends(&mut tx, &me, &friend_id).await? {
        tx.profiles_get_by_id(&friend_id).await?
    } else {
        None
    };
    let open = tx.challenges_open_of(&me, now).await?;
    tx.commit().await?;
    let Some(friend) = friend else {
        return Err(not_found("that player is not your friend"));
    };
    if open
        .iter()
        .any(|challenge| challenge.from_profile == me && challenge.to_profile == friend.id)
    {
        return refused(409, "already_challenged", "You've already challenged them.");
    }
    // B48: an online, idle friend.
    let status = standings(app, std::slice::from_ref(&friend))
        .await?
        .get(&friend.id)
        .map(|standing| standing.status);
    match status {
        Some(Status::Online) => {}
        Some(Status::InMatch) => return refused(409, "busy", "They're playing a game right now."),
        _ => return refused(409, "offline", "They're not online."),
    }

    let room = open_room(app, &req, CHALLENGE_TTL_MS).await?;
    let challenge = ChallengeRow {
        id: new_uuid(),
        from_profile: me.clone(),
        to_profile: friend.id.clone(),
        room_code: room.code.clone(),
        mode: room.mode,
        created_at: now,
        expires_at: room.expires_at,
        state: ChallengeState::Open,
    };
    let mut tx = app.db.begin(Some(&me)).await?;
    if !tx.challenges_create(&challenge, now_ms()).await? {
        // A second challenge raced this one in: this one's room goes, unjoined.
        release_room(&mut tx, &room.code).await?;
        tx.commit().await?;
        return refused(409, "already_challenged", "You've already challenged them.");
    }
    tx.commit().await?;
    log_info(
        "challenge.created",
        json!({
            "challengeId": challenge.id,
            "fromProfileId": me,
            "toProfileId": friend.id,
            "mode": challenge.mode,
        }),
    );
    Ok(created(json!({ "challengeId": challenge.id, "roomCode": challenge.room_code })))
}

/// `GET /api/challenges` (B48, B57): `200 {incoming, outgoing}`, open and not lapsed, oldest first.
pub async fn list(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let now = now_ms();
    let mut tx = app.db.begin(Some(&me)).await?;
    let open = tx.challenges_open_of(&me, now).await?;
    let named: Vec<String> = open
        .iter()
        .flat_map(|row| [row.from_profile.clone(), row.to_profile.clone()])
        .collect();
    let profiles = profiles_by_id(&mut tx, &named).await?;
    tx.commit().await?;

    let challenge_of = |row: &ChallengeRow| -> Option<Challenge> {
        Some(Challenge {
            challenge_id: row.id.clone(),
            from: PlayerRef::of(profiles.get(&row.from_profile)?),
            to: PlayerRef::of(profiles.get(&row.to_profile)?),
            room_code: row.room_code.clone(),
            mode: row.mode,
            expires_at: iso(row.expires_at),
        })
    };
    let incoming: Vec<Challenge> = open
        .iter()
        .filter(|row| row.to_profile == me)
        .filter_map(&challenge_of)
        .collect();
    let outgoing: Vec<Challenge> = open
        .iter()
        .filter(|row| row.from_profile == me)
        .filter_map(&challenge_of)
        .collect();
    Ok(ok(json!({
        "incoming": to_json(&incoming)?,
        "outgoing": to_json(&outgoing)?,
    })))
}

/// `POST /api/challenges/:id/decline` (B49): its friend declines it. 204, or 404.
pub async fn decline(app: &Arc<App>, req: Req) -> ApiResult {
    close(app, req, ChallengeState::Declined, |row, me| row.to_profile == me).await
}

/// `DELETE /api/challenges/:id` (B49): its challenger cancels it. 204, or 404.
pub async fn cancel(app: &Arc<App>, req: Req) -> ApiResult {
    close(app, req, ChallengeState::Cancelled, |row, me| row.from_profile == me).await
}

/// Moves the challenge `:id` to `state` when `mine` says the caller holds the side that may, and
/// frees its room. A room it opened that has lapsed is left alone: it freed itself as it expired,
/// and its code may be another room's by now.
async fn close(
    app: &App,
    req: Req,
    state: ChallengeState,
    mine: fn(&ChallengeRow, &str) -> bool,
) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let challenge_id = param(&req, "id");
    let now = now_ms();
    let mut tx = app.db.begin(Some(&me)).await?;
    let challenge = match tx.challenges_get(&challenge_id).await? {
        Some(challenge) if mine(&challenge, &me) => challenge,
        _ => return Err(not_found("no such challenge")),
    };
    let closed = tx.challenges_close(&challenge.id, state).await?;
    if closed.is_some() && challenge.expires_at > now {
        release_room(&mut tx, &challenge.room_code).await?;
    }
    tx.commit().await?;
    if closed.is_some() {
        log_info(
            "challenge.closed",
            json!({ "challengeId": challenge.id, "profileId": me, "state": state }),
        );
    }
    Ok(no_content())
}

/// B49: a room a challenge opened is its friend's to join; anyone else gets the answer an unknown
/// code gets (`rooms.rs`'s miss), so a code read over a shoulder opens nothing. Any other room
/// passes.
pub async fn assert_may_join(app: &App, code: &str, joiner: &str) -> Result<(), ApiError> {
    let mut tx = app.db.begin(Some(joiner)).await?;
    let challenge = tx.challenges_open_for_room(code, now_ms()).await?;
    tx.commit().await?;
    match challenge {
        Some(challenge) if challenge.to_profile != joiner => Err(not_found("that room code is not open")),
        _ => Ok(()),
    }
}

/// B49: marks every challenge lapsed by now expired; answers how many.
pub async fn sweep(app: &App) -> Result<usize, ApiError> {
    let mut tx = app.db.begin(None).await?;
    let expired = tx.challenges_expire(now_ms()).await?;
    tx.commit().await?;
    Ok(expired.len())
}

/// The challenge sweeper: once now and then every `CHALLENGE_SWEEP_INTERVAL_SECONDS`. A failure is
/// logged and the next tick tries again. Never returns; `app.rs` spawns it.
pub async fn run_sweeper(app: Arc<App>) {
    loop {
        match sweep(&app).await {
            Ok(0) => {}
            Ok(expired) => log_info("challenges.expired", json!({ "count": expired })),
            Err(error) => log_warn("challenges.sweep_failed", json!({ "message": error.message })),
        }
        tokio::time::sleep(Duration::from_secs(
            u64::try_from(CHALLENGE_SWEEP_INTERVAL_SECONDS).unwrap_or(0),
        ))
        .await;
    }
}
