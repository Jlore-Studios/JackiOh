//! The social poll's one read (SPEC S7, §9.4, R3210000): `GET /api/social` answers
//! `{incoming, outgoing, unread}`, what `GET /api/challenges` and `GET /api/chat/unread` answer, by the
//! same reads in one transaction, for the caller alone, and `dailyWaiting` beside them (#3727, below). A pending or banned account, no token and a
//! blocked pair get what the two routes give them. The two routes stay for older open tabs and other
//! callers (CLAUDE.md rule 13). `/ws/social` pushes the same body, `social_snapshot` (R3214000).
//! #3727: `dailyWaiting` is how many daily games wait on the caller's move (§9.13), the Play button's
//! and the Daily screen's badge.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::api::collection::caller_profile;
use crate::api::http::{ApiError, ApiResult, Req, now_ms, ok, to_json};
use crate::app::App;
use crate::daily::games::waiting_count;
use crate::social::challenges::open_challenges;
use crate::social::chat::unread_counts;

/// `GET /api/social` (R3210000): `200 {incoming, outgoing, unread, dailyWaiting}`, the open challenges not lapsed,
/// received and sent, oldest first (B48), and the unread counts by friend (B53).
pub async fn get_social(app: &Arc<App>, req: Req) -> ApiResult {
    Ok(ok(social_snapshot(app, &caller_profile(&req)?.id).await?))
}

/// `GET /api/social`'s body for `me`, also what `/ws/social` pushes it (R3214000).
pub(crate) async fn social_snapshot(app: &App, me: &str) -> Result<Value, ApiError> {
    let now = now_ms();
    let mut tx = app.db.begin(Some(me)).await?;
    let (incoming, outgoing) = open_challenges(&mut tx, me, now).await?;
    let unread = unread_counts(&mut tx, me).await?;
    let daily_waiting = waiting_count(&mut tx, me, app.daily.now()).await?;
    tx.commit().await?;
    Ok(json!({
        "incoming": to_json(&incoming)?,
        "outgoing": to_json(&outgoing)?,
        "unread": Value::Object(unread),
        "dailyWaiting": daily_waiting,
    }))
}
