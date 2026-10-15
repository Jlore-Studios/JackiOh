//! The deploy probes (#861): `GET /healthz` (liveness) and `GET /readyz` (readiness).
//!
//! Not in SPEC, and no R-row: probes are operational, like `loops.rs` (#842). Both are
//! `AuthLevel::None` and name no profile. `/readyz` is render.yaml's `healthCheckPath`: it answers
//! 200 only once the store answers `select 1`, so an instance whose database stops answering after
//! boot fails its check (the pool connects lazily, so nothing else would notice).

use std::sync::Arc;
use std::time::Duration;

use axum::http::HeaderValue;
use axum::response::Response;
use serde_json::json;

use crate::api::catalog::DEPLOYED_COMMIT_HEADER;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, ok};
use crate::app::App;
use crate::config::READINESS_PROBE_TIMEOUT_MS;

/// The answer both probes give when the server is up for them: `{ status, version }` and
/// `x-deployed-commit`, the same version and commit `GET /api/catalog` carries (R163).
fn alive(app: &App) -> Response {
    let mut response = ok(json!({ "status": "ok", "version": app.catalog.version }));
    if let Some(commit) = &app.catalog.commit
        && let Ok(value) = HeaderValue::from_str(commit)
    {
        response.headers_mut().insert(DEPLOYED_COMMIT_HEADER, value);
    }
    response
}

/// `GET /healthz`: liveness only. Never touches the store, so it answers 200 with the database up
/// or down.
pub async fn get_healthz(app: &Arc<App>, _req: Req) -> ApiResult {
    Ok(alive(app))
}

/// `GET /readyz`: readiness. The same answer, but only once the store answers `select 1`; until
/// then 503 `unavailable`, within `READINESS_PROBE_TIMEOUT_MS`. The store's error is logged, never
/// sent.
pub async fn get_readyz(app: &Arc<App>, _req: Req) -> ApiResult {
    let wait = Duration::from_millis(u64::try_from(READINESS_PROBE_TIMEOUT_MS).unwrap_or(0));
    let failure = match tokio::time::timeout(wait, app.db.ping()).await {
        Ok(Ok(())) => return Ok(alive(app)),
        Ok(Err(error)) => error.to_string(),
        Err(_) => format!("no answer within {READINESS_PROBE_TIMEOUT_MS} ms"),
    };
    tracing::warn!(event = "server.not_ready", message = %failure);
    Err(ApiError::new(
        ApiErrorCode::Unavailable,
        "the database is not answering",
    ))
}
