//! Telling a player it is their move (SPEC §9.13, #3727), and the sweep that keeps the daily games going
//! while nobody is online. What to send is `jackioh_shared::notify::decide_notify`'s, pure; this file
//! only reads its inputs and sends what it answers.
//!
//! - Email goes through Resend (`PLAYER_EMAIL_API_URL`) under the player mail's own key
//!   (`PLAYER_RESEND_API_KEY`), with RFC 8058's one-click unsubscribe (`List-Unsubscribe`,
//!   `List-Unsubscribe-Post`) and a link to each waiting game's board. With the key unset none is sent.
//! - Web Push goes to every device the account turned it on for (`daily::webpush`); a subscription
//!   its push service answers 404 or 410 is deleted. With the VAPID keys unset none is sent.
//! - The sweep (`POST /daily-sweep`, the Worker's hourly cron, and `run_sweeper` while the server is
//!   awake) lapses old invites, ends every game whose move ran out (R3727000), and decides for every
//!   player a game waits on.
//! - `GET`/`PUT /api/daily/settings`, `POST`/`DELETE /api/daily/push`, and
//!   `GET`/`POST /api/daily/unsubscribe?token=`.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{StatusCode, header};
use axum::response::Response;
use indexmap::{IndexMap, IndexSet};
use serde_json::{Value, json};

use jackioh_shared::notify::{NotifyInput, NotifyPlan, WaitingGame, decide_notify};

use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiResult, Req, bad_request, bool as body_bool, log_warn, ok, str};
use crate::api::queue::new_uuid;
use crate::app::App;
use crate::config::{
    DAILY_SWEEP_MINUTES, PLAYER_EMAIL_FROM, PLAYER_EMAIL_TIMEOUT_MS, PUSH_FIELD_MAX_LENGTH, PUSH_MAX_DEVICES,
};
use crate::daily::games::{load, settle, waiting_of};
use crate::daily::webpush::{self, PushOutcome, Subscription};
use crate::db::store::PushSubRow;
use crate::loops::FailingStreak;
use crate::social::live::push_social;
use crate::social::{no_content, profiles_by_id};

/// The site's own address, which the emails link back to: the first public origin.
fn site(app: &App) -> String {
    app.env
        .public_origins
        .first()
        .map(|origin| origin.trim_end_matches('/').to_string())
        .unwrap_or_default()
}

/// What one decision sent.
#[derive(Default)]
struct Sent {
    emailed: bool,
    pushed: bool,
}

/// Decides for `profile_id` at `now` and sends what is due, marking each channel that went.
async fn notify_one(app: &Arc<App>, profile_id: &str, now: i64) -> Result<Sent, ApiError> {
    let mut tx = app.db.begin(None).await?;
    let rows = tx.daily_games_of(profile_id, 0).await?;
    let waiting: Vec<WaitingGame> = waiting_of(&rows, profile_id)
        .into_iter()
        .map(|(match_id, since, deadline)| WaitingGame {
            match_id,
            since,
            deadline,
        })
        .collect();
    if waiting.is_empty() {
        tx.commit().await?;
        return Ok(Sent::default());
    }
    let settings = tx.daily_notify_get(profile_id).await?;
    let profile = tx.profiles_get_by_id(profile_id).await?;
    let subscriptions = tx.push_subs_of(profile_id).await?;
    // The opponents' names, for the email's lines.
    let opponents: IndexMap<String, String> = {
        let ids: Vec<String> = rows
            .iter()
            .flat_map(|row| [row.players.0.clone(), row.players.1.clone()])
            .filter(|id| id != profile_id)
            .collect();
        let people = profiles_by_id(&mut tx, &ids).await?;
        rows.iter()
            .filter_map(|row| {
                let other = if row.players.0 == profile_id {
                    &row.players.1
                } else {
                    &row.players.0
                };
                Some((row.match_id.clone(), people.get(other)?.username()))
            })
            .collect()
    };
    tx.commit().await?;

    let email = profile.map(|profile| profile.email).unwrap_or_default();
    let input = NotifyInput {
        // A channel whose keys the server was not given is off (§9.13).
        email_on: settings.email_on && app.daily.mailer.is_some(),
        has_email: !email.trim().is_empty(),
        push_devices: if app.daily.push.is_some() {
            u32::try_from(subscriptions.len()).unwrap_or(u32::MAX)
        } else {
            0
        },
        last_email_at: settings.last_email_at,
        last_push_at: settings.last_push_at,
        waiting,
    };
    let plan = decide_notify(&input, now);
    let mut sent = Sent::default();
    if plan.email {
        sent.emailed = send_email(
            app,
            profile_id,
            &email,
            &settings.unsubscribe_token,
            &plan,
            &opponents,
        )
        .await;
    }
    if plan.push {
        sent.pushed = send_pushes(app, &subscriptions, &plan, now).await;
    }
    if sent.emailed || sent.pushed {
        let mut tx = app.db.begin(None).await?;
        tx.daily_notify_sent(
            profile_id,
            sent.emailed.then_some(now),
            sent.pushed.then_some(now),
        )
        .await?;
        tx.commit().await?;
    }
    Ok(sent)
}

/// The words of a "your move" message: its subject and the line for each game.
fn subject_of(plan: &NotifyPlan) -> String {
    match plan.games.len() {
        1 => "Your move in a JackiOh daily game".to_string(),
        n => format!("Your move in {n} JackiOh daily games"),
    }
}

/// Sends one email listing every game waiting; whether Resend took it.
async fn send_email(
    app: &App,
    profile_id: &str,
    to: &str,
    token: &str,
    plan: &NotifyPlan,
    opponents: &IndexMap<String, String>,
) -> bool {
    let Some(mailer) = &app.daily.mailer else {
        return false;
    };
    let site = site(app);
    let unsubscribe = format!("{site}/api/daily/unsubscribe?token={token}");
    let lines: Vec<String> = plan
        .games
        .iter()
        .map(|game| {
            let against = opponents
                .get(&game.match_id)
                .map(|name| format!(" against {name}"))
                .unwrap_or_default();
            format!("- Your game{against}: {site}/daily/{}", game.match_id)
        })
        .collect();
    let text = format!(
        "It's your move.\n\n{}\n\nTo stop these emails: {unsubscribe}\n",
        lines.join("\n")
    );
    let sent = mailer
        .http
        .post(&mailer.api)
        .bearer_auth(&mailer.key)
        .header("user-agent", "jackioh-server")
        .header("idempotency-key", format!("daily/{profile_id}/{}", new_uuid()))
        .timeout(Duration::from_millis(PLAYER_EMAIL_TIMEOUT_MS))
        .json(&json!({
            "from": PLAYER_EMAIL_FROM,
            "to": [to],
            "subject": subject_of(plan),
            "text": text,
            // RFC 2369 and RFC 8058: one click unsubscribes.
            "headers": {
                "List-Unsubscribe": format!("<{unsubscribe}>"),
                "List-Unsubscribe-Post": "List-Unsubscribe=One-Click",
            },
        }))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status);
    match sent {
        Ok(_) => {
            tracing::info!(event = "daily.emailed", profileId = %profile_id, games = plan.games.len());
            true
        }
        Err(error) => {
            tracing::warn!(event = "daily.email.failed", profileId = %profile_id, message = %error);
            false
        }
    }
}

/// Pushes to each device; deletes a subscription its push service says is gone. Whether one went.
async fn send_pushes(app: &App, subscriptions: &[PushSubRow], plan: &NotifyPlan, now: i64) -> bool {
    let Some(keys) = &app.daily.push else {
        return false;
    };
    let first = plan
        .games
        .first()
        .map(|game| game.match_id.as_str())
        .unwrap_or_default();
    let payload = json!({
        "title": subject_of(plan),
        "body": "Open JackiOh to play your move.",
        "url": if plan.games.len() == 1 { format!("/daily/{first}") } else { "/daily".to_string() },
    });
    let mut any = false;
    for row in subscriptions {
        let subscription = Subscription {
            endpoint: &row.endpoint,
            p256dh: &row.p256dh,
            auth: &row.auth,
        };
        match webpush::send(&app.daily.http, keys, &subscription, &payload, now / 1000).await {
            PushOutcome::Sent => any = true,
            PushOutcome::Gone => {
                tracing::info!(event = "daily.push.gone", profileId = %row.profile_id);
                let deleted = async {
                    let mut tx = app.db.begin(None).await?;
                    tx.push_subs_delete(&row.endpoint, None).await?;
                    tx.commit().await
                }
                .await;
                if let Err(error) = deleted {
                    tracing::warn!(event = "daily.push.delete_failed", message = %error);
                }
            }
            PushOutcome::Failed(reason) => {
                tracing::warn!(event = "daily.push.failed", profileId = %row.profile_id, reason = %reason);
            }
        }
    }
    any
}

/// After a move or a start: decides for both players (the one the move passed to is told), never at
/// the move's expense.
pub(crate) async fn after_move(app: &Arc<App>, profiles: &[&str], now: i64) {
    for profile_id in profiles {
        if let Err(error) = notify_one(app, profile_id, now).await {
            log_warn(
                "daily.notify_failed",
                json!({ "profileId": profile_id, "message": error.message }),
            );
        }
    }
}

/// What one sweep did.
#[derive(Default, serde::Serialize)]
pub struct Swept {
    pub expired: usize,
    pub settled: usize,
    pub emailed: usize,
    pub pushed: usize,
}

/// §9.13: one sweep at `now`.
pub async fn sweep(app: &Arc<App>, now: i64) -> Result<Swept, ApiError> {
    let mut swept = Swept::default();
    let mut tx = app.db.begin(None).await?;
    let lapsed = tx.daily_invites_expire(now).await?;
    let due = tx.daily_games_due(now).await?;
    tx.commit().await?;
    swept.expired = lapsed.len();
    for invite in &lapsed {
        let mut told = vec![invite.from_profile.as_str()];
        told.extend(invite.to_profile.as_deref());
        push_social(app, &told).await;
    }
    for match_id in due {
        // One game's failure is that game's: the next sweep tries it again.
        let settled = async {
            match load(app, &match_id).await? {
                Some(loaded) => settle(app, loaded, now).await.map(|_| true),
                None => Ok(false),
            }
        }
        .await;
        match settled {
            Ok(true) => swept.settled += 1,
            Ok(false) => {}
            Err(error) => log_warn(
                "daily.settle_failed",
                json!({ "matchId": match_id, "message": error.message }),
            ),
        }
    }
    let mut tx = app.db.begin(None).await?;
    let running = tx.daily_games_running().await?;
    tx.commit().await?;
    let mut holders: IndexSet<String> = IndexSet::new();
    for row in &running {
        for player in [&row.players.0, &row.players.1] {
            if !waiting_of(std::slice::from_ref(row), player).is_empty() {
                holders.insert(player.clone());
            }
        }
    }
    for profile_id in holders {
        match notify_one(app, &profile_id, now).await {
            Ok(sent) => {
                swept.emailed += usize::from(sent.emailed);
                swept.pushed += usize::from(sent.pushed);
            }
            Err(error) => log_warn(
                "daily.notify_failed",
                json!({ "profileId": profile_id, "message": error.message }),
            ),
        }
    }
    Ok(swept)
}

/// `POST /daily-sweep`: the Worker's hourly cron. Outside `/api`, so only the Worker reaches it.
pub async fn sweep_route(app: &Arc<App>, _req: Req) -> ApiResult {
    let swept = sweep(app, app.daily.now()).await?;
    tracing::info!(
        event = "daily.swept",
        expired = swept.expired,
        settled = swept.settled,
        emailed = swept.emailed,
        pushed = swept.pushed
    );
    Ok(ok(
        serde_json::to_value(&swept).map_err(|error| ApiError::internal(error.to_string()))?
    ))
}

/// The sweep every `DAILY_SWEEP_MINUTES` while the server is awake.
pub async fn run_sweeper(app: Arc<App>) {
    // #3770: a sweep that fails on every pass for a while is an alert, not just a warning each time.
    let mut sweeping = FailingStreak::new("daily.sweeper");
    loop {
        tokio::time::sleep(Duration::from_secs(DAILY_SWEEP_MINUTES * 60)).await;
        if let Err(error) = sweeping.track(sweep(&app, app.daily.now()).await) {
            log_warn("daily.sweep_failed", json!({ "message": error.message }));
        }
    }
}

/// `GET /api/daily/settings`'s answer for `profile_id`.
async fn settings_json(app: &App, profile_id: &str, email: &str) -> Result<Value, ApiError> {
    let mut tx = app.db.begin(None).await?;
    let settings = tx.daily_notify_get(profile_id).await?;
    let devices = tx.push_subs_of(profile_id).await?.len();
    tx.commit().await?;
    Ok(json!({
        "emailOn": settings.email_on,
        "hasEmail": !email.trim().is_empty(),
        // The browser subscribes with it; null while the server sends no push.
        "vapidPublicKey": app.daily.push.as_ref().map(|keys| keys.public_key()),
        "pushDevices": devices,
    }))
}

/// `GET /api/daily/settings`.
pub async fn get_settings(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    Ok(ok(settings_json(app, &profile.id, &profile.email).await?))
}

/// `PUT /api/daily/settings` `{emailOn}`.
pub async fn put_settings(app: &Arc<App>, req: Req) -> ApiResult {
    let profile = caller_profile(&req)?;
    let email_on = body_bool(&req.body, "emailOn")?;
    let mut tx = app.db.begin(None).await?;
    tx.daily_notify_put(&profile.id, email_on).await?;
    tx.commit().await?;
    Ok(ok(settings_json(app, &profile.id, &profile.email).await?))
}

/// A push subscription's field, bounded (§9.8).
fn field(value: Option<&Value>, name: &str) -> Result<String, ApiError> {
    value
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty() && text.len() <= PUSH_FIELD_MAX_LENGTH)
        .map(str::to_string)
        .ok_or_else(|| bad_request(format!("\"{name}\" must be a push subscription's")))
}

/// `POST /api/daily/push` `{endpoint, keys: {p256dh, auth}}`: this device's subscription. 204.
pub async fn add_push(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let endpoint = field(req.body.get("endpoint"), "endpoint")?;
    // §9.8: a browser's push service only, so a push reaches nothing the caller picked.
    if !webpush::is_push_service(&endpoint) {
        return Err(bad_request(
            "We can't send notifications through this browser's push service. Try another browser.",
        ));
    }
    let keys = req.body.get("keys");
    let row = PushSubRow {
        id: new_uuid(),
        profile_id: me.clone(),
        endpoint,
        p256dh: field(keys.and_then(|keys| keys.get("p256dh")), "keys.p256dh")?,
        auth: field(keys.and_then(|keys| keys.get("auth")), "keys.auth")?,
        created_at: app.daily.now(),
    };
    let mut tx = app.db.begin(None).await?;
    tx.push_subs_add(&row).await?;
    // §9.8: a new device past `PUSH_MAX_DEVICES` replaces the oldest.
    let held = tx.push_subs_of(&me).await?;
    for old in held.iter().take(held.len().saturating_sub(PUSH_MAX_DEVICES)) {
        tx.push_subs_delete(&old.endpoint, Some(&me)).await?;
    }
    tx.commit().await?;
    Ok(no_content())
}

/// `DELETE /api/daily/push` `{endpoint}`: this device's subscription goes (signing out). 204.
pub async fn remove_push(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let endpoint = str(&req.body, "endpoint")?;
    let mut tx = app.db.begin(None).await?;
    tx.push_subs_delete(&endpoint, Some(&me)).await?;
    tx.commit().await?;
    Ok(no_content())
}

/// Turns the email off for the token's account; false for a token that is none.
async fn unsubscribe(app: &App, req: &Req) -> Result<bool, ApiError> {
    let token = req.query.get("token").cloned().unwrap_or_default();
    let mut tx = app.db.begin(None).await?;
    let done = tx.daily_notify_unsubscribe(&token).await?;
    tx.commit().await?;
    Ok(done)
}

/// `GET /api/daily/unsubscribe?token=`: the email's own link, a page saying it is done.
pub async fn unsubscribe_page(app: &Arc<App>, req: Req) -> ApiResult {
    let done = unsubscribe(app, &req).await?;
    let said = if done {
        "You won't get emails about your daily games any more. You can turn them back on from the Daily screen."
    } else {
        "That link is not one of ours any more. You can turn daily game emails off from the Daily screen."
    };
    let page = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><title>JackiOh</title></head><body><p>{said}</p></body></html>"
    );
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .body(Body::from(page))
        .map_err(|error| ApiError::internal(error.to_string()))
}

/// `POST /api/daily/unsubscribe?token=`: RFC 8058's one-click unsubscribe. 204.
pub async fn unsubscribe_post(app: &Arc<App>, req: Req) -> ApiResult {
    unsubscribe(app, &req).await?;
    Ok(no_content())
}
