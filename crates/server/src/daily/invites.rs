//! Starting a daily game (SPEC §9.13, #3727): a challenge to a friend, or a seek in the daily queue.
//!
//! - `POST /api/daily/challenges` `{friendId, mode, perMoveDays, deckId?, leanSet?}`: any friend, online
//!   or not, in a game or not; one open to each friend at a time (`already_challenged`). It makes no
//!   room and reserves nothing, and it lapses `DAILY_CHALLENGE_TTL_DAYS` after it was sent. A block or
//!   an unfriending cancels it (R3727007, `social::friends`).
//! - `POST /api/daily/challenges/:id/accept` `{mode, deckId?, leanSet?}`: its friend starts the game
//!   with a deck of their own; `…/:id/decline` and `DELETE /api/daily/challenges/:id` close it.
//! - `POST /api/daily/seeks` `{mode, perMoveDays, …}`: pairs at once with the oldest open seek of the
//!   same mode and time from someone else (no rating window while daily games are unranked, never a
//!   ladder bot: R1452's fallback is the live queue's), or else waits up to `DAILY_SEEK_TTL_DAYS`;
//!   `DELETE /api/daily/seeks/:id` cancels it.
//!
//! An account holds at most `DAILY_MAX_ACTIVE_GAMES` running games, open seeks and sent challenges
//! together (`daily_full`), and none of them is a seat of R1523000's ledger (R3727004).

use std::sync::Arc;

use serde_json::{Value, json};

use crate::actor::rooms::{HostSide, seats_of_sides};
use crate::api::collection::caller_profile;
use crate::api::decks::{FrozenChoice, ModeChoiceInput, freeze_choice, read_mode_choice};
use crate::api::http::{ApiError, ApiResult, Req, account_key, now_ms, rate_limited, to_json};
use crate::api::queue::{new_seed, new_uuid, seat_coin_p1_first};
use crate::app::App;
use crate::config::{DAILY_CHALLENGE_TTL_DAYS, DAILY_MAX_ACTIVE_GAMES, DAILY_SEEK_TTL_DAYS, MS_PER_DAY};
use crate::daily::games::{NewDaily, prepare_daily};
use crate::db::store::{DailyInviteKind, DailyInviteRow, DailyInviteState, QueueMode};
use crate::social::friends::are_friends;
use crate::social::live::push_social;
use crate::social::{created, no_content, not_found, param, refused};
use jackioh_shared::host::daily::is_daily_time;

/// The time per move a body names, if it is one of `DAILY_TIME_PER_MOVE_DAYS`.
fn time_of(body: &Value) -> Option<i32> {
    body.get("perMoveDays")
        .and_then(Value::as_i64)
        .and_then(|days| i32::try_from(days).ok())
        .filter(|days| is_daily_time(*days))
}

/// The mode a choice plays: Best of 1 or All Random; a Conquest trio is no daily game (§9.13).
fn daily_mode(choice: &ModeChoiceInput) -> Option<QueueMode> {
    match choice {
        ModeChoiceInput::Bo1 { .. } => Some(QueueMode::Bo1),
        ModeChoiceInput::Random { .. } => Some(QueueMode::Random),
        ModeChoiceInput::Bo3 { .. } => None,
    }
}

/// What a body asks for: its choice and its mode, and its time per move when `timed`.
type Asked = (ModeChoiceInput, QueueMode, i32);

/// Reads a challenge's or a seek's body, or the refusal to answer it with.
fn asked(body: &Value, timed: bool) -> Result<Result<Asked, ApiResult>, ApiError> {
    let choice = read_mode_choice(body)?;
    let Some(mode) = daily_mode(&choice) else {
        return Ok(Err(refused(
            422,
            "bad_mode",
            "A daily game is Best of 1 or All Random.",
        )));
    };
    let days = if timed {
        match time_of(body) {
            Some(days) => days,
            None => {
                return Ok(Err(refused(
                    422,
                    "bad_time",
                    "Pick 1 day, 3 days, 1 week or 2 weeks per move.",
                )));
            }
        }
    } else {
        0
    };
    Ok(Ok((choice, mode, days)))
}

/// R3727004: whether `profile_id` may hold one daily game, seek or challenge more.
async fn has_room(app: &App, profile_id: &str, now: i64) -> Result<bool, ApiError> {
    let mut tx = app.db.begin(Some(profile_id)).await?;
    let held = tx.daily_active_count(profile_id, now).await?;
    tx.commit().await?;
    Ok(held < i64::from(DAILY_MAX_ACTIVE_GAMES))
}

fn full() -> ApiResult {
    refused(
        409,
        "daily_full",
        "You have as many daily games going as you can. Finish one first.",
    )
}

/// The frozen choice an invite stored for its sender.
fn stored_choice(row: &DailyInviteRow) -> Result<FrozenChoice, ApiError> {
    serde_json::from_value(row.choice.clone())
        .map_err(|error| ApiError::internal(format!("daily invite {} holds no choice: {error}", row.id)))
}

/// Seats the sender of `invite` (who froze its choice) and `joiner_id` (who froze `joiner`) by the
/// seed's coin (R1989000), and prepares their game, starting at `now`, for the caller to write.
async fn prepare_between(
    app: &Arc<App>,
    invite: &DailyInviteRow,
    match_id: String,
    joiner_id: &str,
    joiner: &FrozenChoice,
    now: i64,
) -> Result<NewDaily, ApiError> {
    let host_choice = stored_choice(invite)?;
    let host = HostSide::of_choice(&invite.from_profile, &host_choice)
        .ok_or_else(|| ApiError::internal(format!("daily invite {} froze a trio", invite.id)))?;
    let seed = new_seed();
    let host_is_p1 = seat_coin_p1_first(&seed);
    let (host_seat, joiner_seat) = seats_of_sides(&host, joiner_id, joiner, &seed, host_is_p1)?;
    // The row is seat-ordered: index 0 is p1.
    let seats = if host_is_p1 {
        (host_seat, joiner_seat)
    } else {
        (joiner_seat, host_seat)
    };
    prepare_daily(app, match_id, seed, seats, invite.mode, invite.per_move_days, now).await
}

/// A claimed seek whose game failed to start goes back to the queue where it stood (its own
/// `created_at`), under a new id: the claim committed on its own, and a failure after it must not
/// lose the other player's seek without a word. Logged, never the caller's failure.
async fn reopen_seek(app: &Arc<App>, claimed: &DailyInviteRow, now: i64) {
    let row = DailyInviteRow {
        id: new_uuid(),
        state: DailyInviteState::Open,
        match_id: None,
        ..claimed.clone()
    };
    let reopened = async {
        let mut tx = app.db.begin(Some(&row.from_profile)).await?;
        let opened = tx.daily_invites_create(&row, now).await?;
        tx.commit().await?;
        Ok::<bool, ApiError>(opened)
    }
    .await;
    match reopened {
        Ok(true) => push_social(app, &[&row.from_profile]).await,
        Ok(false) => tracing::warn!(event = "daily.seek.not_reopened", seekId = %claimed.id),
        Err(error) => {
            tracing::error!(event = "daily.seek.reopen_failed", seekId = %claimed.id, message = %error.message)
        }
    }
}

/// `POST /api/daily/challenges`: `201 {id}`.
pub async fn challenge(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let friend_id = crate::api::http::str(&req.body, "friendId")?;
    // §9.8: counted with the live challenges, on every attempt, before anything is read.
    let key = account_key(&me);
    if !app.social.challenges.allow(&key, now_ms()) {
        return Err(rate_limited(
            "too many challenges; try again later",
            app.social.challenges.retry_after_ms(&key, now_ms()),
        ));
    }
    let (choice, mode, days) = match asked(&req.body, true)? {
        Ok(asked) => asked,
        Err(refusal) => return refusal,
    };
    let now = app.daily.now();
    {
        let mut tx = app.db.begin(Some(&me)).await?;
        let friends = are_friends(&mut tx, &me, &friend_id).await?;
        tx.commit().await?;
        if !friends {
            return Err(not_found("that player is not your friend"));
        }
    }
    if !has_room(app, &me, now).await? {
        return full();
    }
    // R253: checked and frozen now, as a room freezes it; the friend's is frozen at its accept.
    let frozen = freeze_choice(app, &me, &choice).await?;
    let row = DailyInviteRow {
        id: new_uuid(),
        kind: DailyInviteKind::Challenge,
        from_profile: me.clone(),
        to_profile: Some(friend_id.clone()),
        mode,
        per_move_days: days,
        choice: to_json(&frozen)?,
        state: DailyInviteState::Open,
        match_id: None,
        created_at: now,
        expires_at: now + DAILY_CHALLENGE_TTL_DAYS * MS_PER_DAY,
    };
    let mut tx = app.db.begin(Some(&me)).await?;
    // R1689000, R3727007: read again where the challenge is written, so a block or an unfriending
    // made meanwhile leaves none open.
    if !are_friends(&mut tx, &me, &friend_id).await? {
        tx.commit().await?;
        return Err(not_found("that player is not your friend"));
    }
    if !tx.daily_invites_create(&row, now).await? {
        tx.commit().await?;
        return refused(
            409,
            "already_challenged",
            "You've already challenged them to a daily game.",
        );
    }
    tx.commit().await?;
    push_social(app, &[&me, &friend_id]).await;
    tracing::info!(event = "daily.challenged", challengeId = %row.id, mode = ?mode, perMoveDays = days);
    Ok(created(json!({ "id": row.id })))
}

/// The open, unlapsed challenge `:id` whose side `mine` says the caller holds, or 404.
async fn open_challenge(
    app: &App,
    req: &Req,
    me: &str,
    now: i64,
    mine: impl Fn(&DailyInviteRow) -> bool,
) -> Result<DailyInviteRow, ApiError> {
    let id = param(req, "id");
    let mut tx = app.db.begin(Some(me)).await?;
    let row = tx.daily_invites_get(&id).await?;
    tx.commit().await?;
    row.filter(|row| row.state == DailyInviteState::Open && row.expires_at > now && mine(row))
        .ok_or_else(|| not_found("no such daily challenge"))
}

/// `POST /api/daily/challenges/:id/accept`: `201 {matchId}`.
pub async fn accept(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let now = app.daily.now();
    let invite = open_challenge(app, &req, &me, now, |row| {
        row.kind == DailyInviteKind::Challenge && row.to_profile.as_deref() == Some(me.as_str())
    })
    .await?;
    let (choice, mode, _) = match asked(&req.body, false)? {
        Ok(asked) => asked,
        Err(refusal) => return refusal,
    };
    if mode != invite.mode {
        return refused(
            409,
            "mode_mismatch",
            "That challenge is for another mode. Pick a deck for its mode.",
        );
    }
    if !has_room(app, &me, now).await? {
        return full();
    }
    let frozen = freeze_choice(app, &me, &choice).await?;
    // The game is prepared first and written in the accept's own transaction, so a start that fails
    // leaves the challenge open, and an accepted challenge always names a game that exists.
    let game = prepare_between(app, &invite, new_uuid(), &me, &frozen, now).await?;
    let mut tx = app.db.begin(Some(&me)).await?;
    // R1689000: still friends as it starts.
    if !are_friends(&mut tx, &me, &invite.from_profile).await? {
        tx.commit().await?;
        return Err(not_found("no such daily challenge"));
    }
    // One accept wins: a cancel, a decline or a second accept meanwhile finds it closed.
    let closed = tx
        .daily_invites_close(&invite.id, DailyInviteState::Accepted, Some(game.match_id()))
        .await?;
    if closed.is_none() {
        tx.commit().await?;
        return Err(not_found("no such daily challenge"));
    }
    game.create(&mut tx).await?;
    tx.commit().await?;
    game.started();
    let match_id = game.match_id().to_string();
    push_social(app, &[&me, &invite.from_profile]).await;
    crate::daily::notify::after_move(app, &[&me, &invite.from_profile], now).await;
    Ok(created(json!({ "matchId": match_id })))
}

/// Closes the open invite `:id` the caller holds the side of to `state`: 204, or 404.
async fn close(
    app: &Arc<App>,
    req: Req,
    state: DailyInviteState,
    mine: impl Fn(&DailyInviteRow, &str) -> bool,
) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let now = app.daily.now();
    let invite = open_challenge(app, &req, &me, now, |row| mine(row, &me)).await?;
    let mut tx = app.db.begin(Some(&me)).await?;
    let closed = tx.daily_invites_close(&invite.id, state, None).await?;
    tx.commit().await?;
    if closed.is_none() {
        return Err(not_found("no such daily challenge"));
    }
    let mut told = vec![invite.from_profile.as_str()];
    told.extend(invite.to_profile.as_deref());
    push_social(app, &told).await;
    Ok(no_content())
}

/// `POST /api/daily/challenges/:id/decline` (its friend): 204.
pub async fn decline(app: &Arc<App>, req: Req) -> ApiResult {
    close(app, req, DailyInviteState::Declined, |row, me| {
        row.kind == DailyInviteKind::Challenge && row.to_profile.as_deref() == Some(me)
    })
    .await
}

/// `DELETE /api/daily/challenges/:id` (its challenger): 204.
pub async fn cancel(app: &Arc<App>, req: Req) -> ApiResult {
    close(app, req, DailyInviteState::Cancelled, |row, me| {
        row.kind == DailyInviteKind::Challenge && row.from_profile == me
    })
    .await
}

/// `DELETE /api/daily/seeks/:id` (its seeker): 204.
pub async fn cancel_seek(app: &Arc<App>, req: Req) -> ApiResult {
    close(app, req, DailyInviteState::Cancelled, |row, me| {
        row.kind == DailyInviteKind::Seek && row.from_profile == me
    })
    .await
}

/// `POST /api/daily/seeks`: `201 {matchId}` when it paired at once, else `201 {seekId}`.
pub async fn seek(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let (choice, mode, days) = match asked(&req.body, true)? {
        Ok(asked) => asked,
        Err(refusal) => return refusal,
    };
    let now = app.daily.now();
    if !has_room(app, &me, now).await? {
        return full();
    }
    let frozen = freeze_choice(app, &me, &choice).await?;
    let match_id = new_uuid();
    let mut tx = app.db.begin(None).await?;
    let claimed = tx.daily_seek_claim(mode, days, &me, &match_id, now).await?;
    tx.commit().await?;
    if let Some(other) = claimed {
        // The claim names the game written next; a start that fails puts the other's seek back.
        let started = async {
            let game = prepare_between(app, &other, match_id, &me, &frozen, now).await?;
            let mut tx = app.db.begin(None).await?;
            game.create(&mut tx).await?;
            tx.commit().await?;
            game.started();
            Ok::<String, ApiError>(game.match_id().to_string())
        }
        .await;
        let match_id = match started {
            Ok(match_id) => match_id,
            Err(error) => {
                reopen_seek(app, &other, now).await;
                return Err(error);
            }
        };
        push_social(app, &[&me, &other.from_profile]).await;
        crate::daily::notify::after_move(app, &[&me, &other.from_profile], now).await;
        return Ok(created(json!({ "matchId": match_id })));
    }
    let row = DailyInviteRow {
        id: new_uuid(),
        kind: DailyInviteKind::Seek,
        from_profile: me.clone(),
        to_profile: None,
        mode,
        per_move_days: days,
        choice: to_json(&frozen)?,
        state: DailyInviteState::Open,
        match_id: None,
        created_at: now,
        expires_at: now + DAILY_SEEK_TTL_DAYS * MS_PER_DAY,
    };
    let mut tx = app.db.begin(Some(&me)).await?;
    let opened = tx.daily_invites_create(&row, now).await?;
    tx.commit().await?;
    if !opened {
        return refused(
            409,
            "already_seeking",
            "You're already looking for a daily game like that one.",
        );
    }
    push_social(app, &[&me]).await;
    Ok(created(json!({ "seekId": row.id })))
}
