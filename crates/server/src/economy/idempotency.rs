//! Idempotency keys for the spends (SPEC §14.11, R2124000; issue #2124): a spend sent again after
//! the client gave up on its answer (`API_REQUEST_TIMEOUT_SECONDS`) runs once.
//!
//! Every spend route, and `POST /api/codes/redeem`, takes an optional `Idempotency-Key` header holding a
//! UUID ([`RequestKey`]); anything else there is 400. A key belongs to its account. Under the account's
//! economy lock (R1301003), in the spend's own transaction, a key sent before answers as it committed, or
//! with another route or body is 422, spending nothing ([`replayed`]); only a committed spend is kept, so a
//! refused key may run again once the refusal no longer holds ([`keep`]). The retention loop sweeps expired
//! keys ([`sweep_expired`]); deleting an account takes its keys.
//!
//! The body is kept only as a keyed hash ([`RequestKey::of`]) under `CODE_PEPPER`'s `:body` domain, so
//! no invite code a redemption sent is kept, even hashed under the code's own domain.

use serde::Serialize;
use serde_json::Value;

use crate::api::crypto::digest;
use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, bad_request, json, ok_of, to_json};
use crate::app::App;
use crate::config::IDEMPOTENCY_KEY_TTL_HOURS;
use crate::db::store::{StoreError, Tx};
use crate::economy::store::IdempotencyRow;

/// The header a spend's key comes in, as `apps/web/src/net/api.ts` sends it (HTTP header names
/// are case-insensitive, and `http` keeps them lower case).
pub const IDEMPOTENCY_KEY_HEADER: &str = "idempotency-key";

/// The status a kept answer replays with: only a committed spend is kept, and every spend route
/// answers a commit with 200.
const STORED_STATUS: u16 = 200;

/// Unit conversion, not configuration.
const MS_PER_HOUR: i64 = 3_600_000;

/// A spend's key as the request sent it: the UUID, hyphenated and lower case, the route it came
/// to, and the keyed hash of its body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequestKey {
    pub key: String,
    pub route: &'static str,
    pub body_hash: String,
}

impl RequestKey {
    /// The request's key for `route`: `None` without the header, 400 when it holds no UUID. The
    /// body's hash is an HMAC-SHA256 of its JSON (whose keys `serde_json` writes sorted) under
    /// `<CODE_PEPPER>:body`, a domain of its own as the code's and the address's are.
    pub fn of(app: &App, req: &Req, route: &'static str) -> Result<Option<RequestKey>, ApiError> {
        let Some(value) = req.headers.get(IDEMPOTENCY_KEY_HEADER) else {
            return Ok(None);
        };
        let key = value
            .to_str()
            .ok()
            .and_then(|text| uuid::Uuid::parse_str(text.trim()).ok())
            .ok_or_else(|| bad_request("Idempotency-Key must be a UUID"))?;
        Ok(Some(RequestKey {
            key: key.hyphenated().to_string(),
            route,
            body_hash: digest(&format!("{}:body", app.env.code_pepper), &req.body.to_string()),
        }))
    }
}

/// What a keyed spend did: ran `Now`, or answered as it did `Before`, from its kept answer.
#[derive(Debug)]
pub enum Spent<T> {
    Now(T),
    Before { status: u16, body: Value },
}

impl<T: Serialize> Spent<T> {
    /// The route's answer: the spend's own, or the kept one as it was kept.
    pub fn answer(self) -> ApiResult {
        match self {
            Spent::Now(value) => ok_of(&value),
            Spent::Before { status, body } => Ok(json(status, body)),
        }
    }
}

impl<T> Spent<T> {
    /// The spend's own answer, for a caller that sent no key, which nothing can replay.
    pub fn now(self) -> Result<T, ApiError> {
        match self {
            Spent::Now(value) => Ok(value),
            Spent::Before { .. } => Err(ApiError::internal("a spend with no key replayed an answer")),
        }
    }
}

/// R2124000: the kept answer to `request`'s key, read in the spend's transaction under the
/// account's lock: `None` with no key or a key the account never committed a spend with, the
/// answer when the key came with this route and body, and 422 `idempotency_key_reused` when it
/// came with another.
pub async fn replayed<T>(
    tx: &mut Tx<'_>,
    profile_id: &str,
    request: Option<&RequestKey>,
) -> Result<Option<Spent<T>>, ApiError> {
    let Some(request) = request else {
        return Ok(None);
    };
    let Some(row) = tx.economy_idempotency_get(profile_id, &request.key).await? else {
        return Ok(None);
    };
    if row.route != request.route || row.body_hash != request.body_hash {
        return Err(ApiError::new(
            ApiErrorCode::IdempotencyKeyReused,
            "this Idempotency-Key was sent with another request",
        ));
    }
    let status = u16::try_from(row.status)
        .map_err(|_| ApiError::internal(format!("a kept answer's status {}", row.status)))?;
    tracing::info!(event = "idempotency.replayed", "profileId" = %profile_id, route = request.route);
    Ok(Some(Spent::Before {
        status,
        body: row.body,
    }))
}

/// R2124000: keeps a spend's `answer` under `request`'s key, in the spend's transaction before its
/// commit, so the grant and its answer commit together. No key keeps nothing.
pub async fn keep<T: Serialize>(
    tx: &mut Tx<'_>,
    profile_id: &str,
    request: Option<&RequestKey>,
    answer: &T,
) -> Result<(), ApiError> {
    let Some(request) = request else {
        return Ok(());
    };
    tx.economy_idempotency_put(&IdempotencyRow {
        profile_id: profile_id.to_string(),
        key: request.key.clone(),
        route: request.route.to_string(),
        body_hash: request.body_hash.clone(),
        status: i64::from(STORED_STATUS),
        body: to_json(answer)?,
        created_at: crate::app::now_ms(),
    })
    .await?;
    Ok(())
}

/// R2124000: deletes every key written `IDEMPOTENCY_KEY_TTL_HOURS` ago or more, in one
/// transaction; answers how many. The retention loop runs it.
pub async fn sweep_expired(app: &App) -> Result<u64, StoreError> {
    let mut tx = app.db.begin(None).await?;
    let swept = tx
        .economy_idempotency_sweep(crate::app::now_ms() - IDEMPOTENCY_KEY_TTL_HOURS * MS_PER_HOUR)
        .await?;
    tx.commit().await?;
    Ok(swept)
}
