//! `POST /api/client-errors` (#3619): a crash in a player's browser, told to the owners.
//!
//! Not in SPEC, and no R-row: operational, like `alerts.rs` (#3595) and `bug_reports.rs`. The web's
//! `net/clientErrors.ts` sends one small report for an error its route boundary caught, an uncaught
//! `error` or an `unhandledrejection`. The route is `AuthLevel::None`, so a signed-out visitor can
//! report; a bearer token that verifies adds the per-account limit on top of the per-address one, and
//! one that does not is treated as no token at all. The handler builds what it logs from five known
//! fields, each cut to its cap, so nothing else in the body (a hand, a board) is kept or logged, and
//! raises the `client.error` alert: `alerts.rs` files the public issue (no field) and emails the
//! details under its hourly window and daily email cap.

use serde_json::json;
use std::sync::Arc;

use crate::api::http::{
    ApiResult, RateLimiter, Req, account_key, address_key, bad_request, bearer_token, create_rate_limiter,
    log_alert, log_warn, ok, optional_str, rate_limited, str,
};
use crate::app::{App, now_ms};
use crate::config::{
    CLIENT_ERROR_BODY_MAX_BYTES, CLIENT_ERROR_FIELD_MAX_CHARS, CLIENT_ERROR_MESSAGE_MAX_CHARS,
    CLIENT_ERROR_REPORTS_PER_ACCOUNT, CLIENT_ERROR_REPORTS_PER_ADDRESS, CLIENT_ERROR_STACK_MAX_CHARS,
    CLIENT_ERROR_TRIGGERS, CLIENT_ERROR_WINDOW_MS,
};

/// The two allowances of the route, one per `App`.
pub struct ClientErrors {
    address: RateLimiter,
    account: RateLimiter,
}

impl Default for ClientErrors {
    fn default() -> Self {
        ClientErrors {
            address: create_rate_limiter(CLIENT_ERROR_REPORTS_PER_ADDRESS, CLIENT_ERROR_WINDOW_MS),
            account: create_rate_limiter(CLIENT_ERROR_REPORTS_PER_ACCOUNT, CLIENT_ERROR_WINDOW_MS),
        }
    }
}

/// `text` cut to `max` characters.
fn cut(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

/// `POST /api/client-errors`: `{ trigger, message, stack, route?, catalogVersion? }`. 200
/// `{ reported: true }` after the `client.error` alert is raised; 400 for a body over
/// `CLIENT_ERROR_BODY_MAX_BYTES`, an unknown trigger or a missing message or stack; 429 once the
/// address or the account has sent its allowance in the window.
pub async fn post_client_error(app: &Arc<App>, req: Req) -> ApiResult {
    if req.body.to_string().len() > CLIENT_ERROR_BODY_MAX_BYTES {
        return Err(bad_request("the report is too large"));
    }
    let now = now_ms();

    let key = address_key(&req.address);
    if !app.client_errors.address.allow(&key, now) {
        log_warn("client.error.limited", json!({ "scope": "address" }));
        return Err(rate_limited(
            "too many error reports; slow down",
            app.client_errors.address.retry_after_ms(&key, now),
        ));
    }
    if let Some(token) = bearer_token(&req.headers)
        && let Ok(user) = app.auth.verify(&token).await
    {
        let key = account_key(&user.user_id);
        if !app.client_errors.account.allow(&key, now) {
            log_warn("client.error.limited", json!({ "scope": "account" }));
            return Err(rate_limited(
                "too many error reports; slow down",
                app.client_errors.account.retry_after_ms(&key, now),
            ));
        }
    }

    let trigger = str(&req.body, "trigger")?;
    if !CLIENT_ERROR_TRIGGERS.contains(&trigger.as_str()) {
        return Err(bad_request("\"trigger\" is not one a client reports"));
    }
    let message = str(&req.body, "message")?;
    let stack = str(&req.body, "stack")?;
    let route = optional_str(&req.body, "route")?;
    let catalog_version = optional_str(&req.body, "catalogVersion")?;

    log_alert(
        "client.error",
        json!({
            "trigger": trigger,
            "message": cut(&message, CLIENT_ERROR_MESSAGE_MAX_CHARS),
            "stack": cut(&stack, CLIENT_ERROR_STACK_MAX_CHARS),
            "route": route.map(|route| cut(&route, CLIENT_ERROR_FIELD_MAX_CHARS)),
            "catalogVersion": catalog_version.map(|version| cut(&version, CLIENT_ERROR_FIELD_MAX_CHARS)),
        }),
    );
    Ok(ok(json!({ "reported": true })))
}
