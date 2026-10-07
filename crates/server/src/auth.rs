//! The `Auth` enum (SURFACE §11.2): the provider half of `apps/server/src/api/auth.ts` (Supabase
//! Auth) and the fixture provider of `apps/server/src/api/e2e.ts` (BUILD M8's `E2E=1` accounts),
//! one variant each. No trait (SURFACE §11.3): `app.rs` chooses one or the other on `env.e2e`, so a
//! real token cannot reach the fixture provider and a fixture token cannot reach the real one.
//!
//! Managed auth (SPEC §9.4: "Managed auth provider" for email and password) over Supabase Auth.
//!
//! Topology, per SPEC §9.2: the browser has its own HTTPS arrow to the auth provider, separate
//! from its arrow to the API functions. The browser signs up and signs in against Supabase Auth
//! directly with the *publishable* key; this server's job is to **verify** the bearer token that
//! comes back, so `verify` is the load-bearing method here. The server-side password path (TS
//! `signUp` / `signInWithPassword` over a publishable key) is not ported (SURFACE §11.3): sign-up
//! is gone and Supabase's `sign_in` answers unavailable, as TS did on every deployment that
//! configured no publishable key. It was never run with the secret key: a service-role sign-up
//! bypasses the provider's own rate limits and its email-confirmation behaviour, which is exactly
//! what §9.4's invite gate relies on.
//!
//! Security notes (the Supabase security checklist, and §9.8's "Invite code brute force" row,
//! whose mitigation list includes "verified email"):
//!
//!  - `email_verified` NEVER comes from the access token's `user_metadata`. In Supabase that claim
//!    is *user-editable* (`raw_user_meta_data` is writable through `auth.updateUser`), so trusting
//!    a self-set `user_metadata.email_verified` would walk straight past §9.4 step 1's "verified
//!    email" requirement and hand a scripted attacker unlimited invite-code attempts. It is read
//!    from the authoritative auth server instead — `email_confirmed_at`, which only GoTrue writes.
//!  - `AuthUser.app_metadata` is filled from the `app_metadata` claim only (provider-controlled).
//!  - the secret key stays inside this struct: it is never returned, never logged, and never put
//!    into a response body. Nothing in this file logs a token or a key either.
//!  - Supabase caveat worth naming: deleting a user does not invalidate tokens already issued.
//!    The admin lookup below turns an explicit "no such user" into a failed verification, and
//!    `profiles.status` (§9.4) remains the only authority on what an account may do.
//!  - The same is true of ending a SESSION (R194). A signature and an unexpired `exp` prove only
//!    that the provider issued the token; a sign-out, a link's session the client dropped, or a
//!    password reset that signed other devices out ends the session at the provider, and its access
//!    token is still well-signed for up to an hour. So a token that names its session
//!    (`session_id`, which every Supabase access token carries) is checked against the provider
//!    (`GET /auth/v1/user`, which refuses a token whose session is gone), and only the answer that
//!    it is live is remembered, per session, for `AUTH_SESSION_LIVE_CACHE_SECONDS`. That one answer
//!    is also the authoritative user, so it stands in for the admin lookup when it is fresh.
//!  - TWO-STEP SIGN-IN (R665). An account with a verified authenticator app (a TOTP factor) is
//!    honoured only with an `aal2` token, the level the provider gives a session once its code was
//!    typed. Without this the second step would be the client's alone: a stolen password signs in at
//!    `aal1`, and every API call would take that token. Whether the account HAS a factor is read
//!    from the provider's user (the same answers as `email_confirmed_at`), never from the token, and
//!    the last answer is remembered per user so an outage cannot lower the bar (`mfa_enrolled`).

use std::sync::{Arc, Mutex};
use std::time::Duration;

use indexmap::{IndexMap, IndexSet};
use jsonwebtoken::jwk::{AlgorithmParameters, JwkSet, PublicKeyUse};
use jsonwebtoken::{Algorithm, AlgorithmFamily, DecodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::config::{AUTH_PROVIDER_TIMEOUT_SECONDS, AUTH_SESSION_LIVE_CACHE_SECONDS};

// ---------------------------------------------------------------------------
// The identity a verified token names (TS `ports.ts`: `AuthUser`, `AuthSession`)
// ---------------------------------------------------------------------------

/// TS `AuthUser`: who a verified bearer token belongs to.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthUser {
    /// The provider's user id (Supabase `auth.users.id`).
    pub user_id: String,
    pub email: Option<String>,
    pub email_verified: bool,
    /// Provider-controlled claims only. Never read user-controlled metadata for an authorization
    /// decision: in Supabase `user_metadata` is user-editable, `app_metadata` is not.
    pub app_metadata: IndexMap<String, Value>,
}

/// TS `AuthSession` (SURFACE §11.2 names it `Session`).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Epoch milliseconds, matching the server's clock (`app::now_ms`), or null.
    pub expires_at: Option<i64>,
    pub user: AuthUser,
}

/// TS's name for `Session`.
pub type AuthSession = Session;

/// Why a provider call did not produce what was asked.
///
/// - `Invalid`: TS `verifyAccessToken`'s `null`: the token is not currently valid.
/// - `Rejected`: TS's plain `Error` from a provider (a sign-in with the wrong password). The
///   routes turn it into the one identical 401 (`api::auth::call_provider`); its text is logged,
///   never sent.
/// - `Unavailable`: TS's `ApiError("unavailable", message)`, passed through to the client as 503
///   with this message.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AuthError {
    #[error("the token is not currently valid")]
    Invalid,
    #[error("{0}")]
    Rejected(String),
    #[error("{0}")]
    Unavailable(String),
}

// ---------------------------------------------------------------------------
// Tunables SPEC does not pin down
// ---------------------------------------------------------------------------

// NOT IN SPEC, and no R-row yet — PROPOSED RULING for §11:
//   Topic: How long a verified email stays verified
//   Ruling: §9.4 step 1's "verified email" is read from the auth provider, and only the
//     *positive* answer may be remembered — for 30 seconds, per user id. Caching the positive is
//     safe because confirmation does not go backwards in normal use, and refusing to cache the
//     negative is what lets an account that has just clicked its confirmation link see the code
//     screen unlock at once rather than after a cache window. The alternative, asking the auth
//     server on every request, puts a round trip in front of every authenticated call, and the
//     alternative of caching both directions makes a freshly verified account wait for no reason.
//     A provider that cannot be reached still fails closed (`AdminLookup::Unavailable`): the
//     identity stands and the email counts as unverified, so the cache can only ever shorten the
//     path to a `yes` the provider already gave.
//   Affects: §9.4 (redemption step 1), §9.2; `api/auth.ts`, `api/codes.ts`.
//
// SPEC §9.4 requires a verified email at redemption but says nothing about how the server learns
// of it, which is the gap above.
const EMAIL_CONFIRMED_CACHE_TTL_MS: i64 = 30_000;

/// Supabase issues project JWTs with this audience for a signed-in user.
const AUTHENTICATED_AUDIENCE: &str = "authenticated";

// Not in SPEC, and no R-row: wording only. §9.2 puts sign-in in the browser against Supabase Auth,
// so a server with no publishable key is the normal deployment and this sentence is an operator
// diagnostic for whoever called a route this deployment does not broker. Nothing branches on it.
pub const PASSWORD_PATH_DISABLED_MESSAGE: &str =
    "This server does not broker passwords: sign up and sign in against Supabase Auth from the client.";

// Not in SPEC, and no R-row: wording only, for `DELETE /api/account`.
pub const ACCOUNT_DELETION_UNAVAILABLE_MESSAGE: &str = "Account deletion is not available on this server.";
pub const ACCOUNT_DELETION_RETRY_MESSAGE: &str =
    "Your account could not be deleted just now. Try again in a minute.";

/// Unit conversion, not configuration.
const MS_PER_SECOND: i64 = 1000;

/// The level a token claims to be (`aal`): `aal2` once a second factor was proved (R665).
const SECOND_FACTOR_LEVEL: &str = "aal2";

/// Not in SPEC, and no R-row: jose's `createRemoteJWKSet` defaults, which TS ran on unchanged and
/// which this port keeps. A fetched key set is kept this long before it is fetched again.
const JWKS_CACHE_MAX_AGE_MS: i64 = 10 * 60 * 1000;
/// jose's `cooldownDuration`: a token whose key is not in the set refetches it at most this often.
const JWKS_COOLDOWN_MS: i64 = 30_000;
/// jose's `timeoutDuration` for the key-set request.
const JWKS_FETCH_TIMEOUT_MS: u64 = 5_000;

// ---------------------------------------------------------------------------
// The slice of the provider's shapes this file reads
// ---------------------------------------------------------------------------

/// GoTrue's user object, narrowed to the fields §9.4 needs. `user_metadata` is deliberately absent:
/// it is user-editable, so this file has no way to read it by accident.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AuthApiUser {
    pub id: String,
    pub email: Option<String>,
    /// The auth server's own verification timestamp; None until the link is clicked.
    pub email_confirmed_at: Option<String>,
    /// Provider-controlled claims (`app_metadata`), safe for authorization.
    pub app_metadata: IndexMap<String, Value>,
    /// R665: whether the account has a VERIFIED TOTP factor (GoTrue's `factors`, `status: "verified"`).
    /// An unverified factor (an enrolment never finished) does not count.
    pub mfa_enrolled: bool,
}

/// What an admin lookup of a user can say.
#[derive(Clone, Debug, PartialEq)]
pub enum AdminLookup {
    Ok(AuthApiUser),
    /// The auth server has no such user any more (deleted); the token must not be honoured.
    Missing,
    /// Nobody answered. The identity stands, but the email counts as unverified (fail closed).
    Unavailable,
}

/// What deleting a user can say: done, already gone, or nobody answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdminDeletion {
    Deleted,
    Missing,
    Unavailable,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn as_record(value: Option<&Value>) -> IndexMap<String, Value> {
    match value {
        Some(Value::Object(map)) => map
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        _ => IndexMap::new(),
    }
}

/// §9.4 step 1's "verified email", as the auth server states it.
fn is_email_confirmed(user: &AuthApiUser) -> bool {
    matches!(&user.email_confirmed_at, Some(at) if !at.is_empty())
}

fn as_auth_api_user(value: &Value) -> Option<AuthApiUser> {
    let record = value.as_object()?;
    let id = record.get("id").and_then(Value::as_str)?;
    if id.is_empty() {
        return None;
    }
    Some(AuthApiUser {
        id: id.to_string(),
        email: record.get("email").and_then(Value::as_str).map(str::to_string),
        email_confirmed_at: record
            .get("email_confirmed_at")
            .and_then(Value::as_str)
            .map(str::to_string),
        app_metadata: as_record(record.get("app_metadata")),
        mfa_enrolled: has_verified_totp(record.get("factors")),
    })
}

/// R665: GoTrue's `factors` list holds a verified TOTP factor.
fn has_verified_totp(factors: Option<&Value>) -> bool {
    let Some(Value::Array(factors)) = factors else {
        return false;
    };
    factors.iter().any(|factor| {
        factor.get("factor_type").and_then(Value::as_str) == Some("totp")
            && factor.get("status").and_then(Value::as_str) == Some("verified")
    })
}

fn to_auth_user(user: &AuthApiUser) -> AuthUser {
    AuthUser {
        user_id: user.id.clone(),
        email: user.email.clone(),
        // Authoritative, not `user_metadata` — see the file header.
        email_verified: is_email_confirmed(user),
        app_metadata: user.app_metadata.clone(),
    }
}

fn trim_trailing_slash(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

/// `new Date(ms).toISOString()`: only ever asked whether it is set, never shown.
fn iso_of(ms: i64) -> String {
    time::OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .ok()
        .and_then(|at| at.format(&time::format_description::well_known::Rfc3339).ok())
        .unwrap_or_else(|| ms.to_string())
}

/// True when the token parses as a JWT whose header names HS256 (a legacy shared-secret token).
fn signed_with_shared_secret(token: &str) -> bool {
    matches!(jsonwebtoken::decode_header(token), Ok(header) if header.alg == Algorithm::HS256)
}

/// supabase-js's admin calls refuse an id that is not a UUID before any request, and TS's catch
/// turned that into "unavailable".
pub(crate) fn is_uuid(id: &str) -> bool {
    uuid::Uuid::parse_str(id).is_ok()
}

// ---------------------------------------------------------------------------
// The real Supabase clients, adapted onto the narrow interface above
// ---------------------------------------------------------------------------

/// The admin half (secret key only): §9.4 step 1's authoritative `email_confirmed_at`, and
/// `DELETE /api/account`'s last step. GoTrue's admin API, as `@supabase/supabase-js` called it.
#[derive(Clone)]
pub struct AdminAuthClient {
    http: reqwest::Client,
    /// `${SUPABASE_URL}/auth/v1`.
    auth_base: String,
    secret_key: String,
}

/// The secret key never reaches a log line, `{:?}` included.
impl std::fmt::Debug for AdminAuthClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AdminAuthClient")
            .field("auth_base", &self.auth_base)
            .finish_non_exhaustive()
    }
}

impl AdminAuthClient {
    pub async fn get_user_by_id(&self, user_id: &str) -> AdminLookup {
        if !is_uuid(user_id) {
            return AdminLookup::Unavailable;
        }
        let response = match self
            .http
            .get(format!("{}/admin/users/{}", self.auth_base, user_id))
            .header("apikey", &self.secret_key)
            .header("authorization", format!("Bearer {}", self.secret_key))
            .header("accept", "application/json")
            .timeout(Duration::from_millis(
                AUTH_PROVIDER_TIMEOUT_SECONDS as u64 * MS_PER_SECOND as u64,
            ))
            .send()
            .await
        {
            Ok(response) => response,
            Err(_) => return AdminLookup::Unavailable,
        };
        let status = response.status().as_u16();
        if !response.status().is_success() {
            // 404 is the auth server stating the user does not exist; anything else is a fault.
            return if status == 404 {
                AdminLookup::Missing
            } else {
                AdminLookup::Unavailable
            };
        }
        let body: Value = match response.json().await {
            Ok(body) => body,
            Err(_) => return AdminLookup::Unavailable,
        };
        match as_auth_api_user(&body) {
            Some(user) => AdminLookup::Ok(user),
            None => AdminLookup::Missing,
        }
    }

    pub async fn delete_user(&self, user_id: &str) -> AdminDeletion {
        if !is_uuid(user_id) {
            return AdminDeletion::Unavailable;
        }
        let response = match self
            .http
            .delete(format!("{}/admin/users/{}", self.auth_base, user_id))
            .header("apikey", &self.secret_key)
            .header("authorization", format!("Bearer {}", self.secret_key))
            .header("accept", "application/json")
            .json(&json!({ "should_soft_delete": false }))
            .timeout(Duration::from_millis(
                AUTH_PROVIDER_TIMEOUT_SECONDS as u64 * MS_PER_SECOND as u64,
            ))
            .send()
            .await
        {
            Ok(response) => response,
            Err(_) => return AdminDeletion::Unavailable,
        };
        if response.status().is_success() {
            AdminDeletion::Deleted
        } else if response.status().as_u16() == 404 {
            AdminDeletion::Missing
        } else {
            AdminDeletion::Unavailable
        }
    }
}

/// TS `SupabaseAuthClients`, minus the password half (not ported, see the file header).
#[derive(Clone, Debug)]
pub struct SupabaseAuthClients {
    pub admin: Option<AdminAuthClient>,
}

/// TS `SupabaseAuthClientInput`. No `Debug`: it holds the secret key.
#[derive(Clone)]
pub struct SupabaseAuthClientInput {
    pub url: String,
    pub secret_key: String,
}

/// TS `createRealClients`. No session is persisted and nothing is refreshed in the background:
/// this process holds no user session of its own.
pub fn create_real_clients(input: &SupabaseAuthClientInput, http: &reqwest::Client) -> SupabaseAuthClients {
    SupabaseAuthClients {
        admin: Some(AdminAuthClient {
            http: http.clone(),
            auth_base: format!("{}/auth/v1", trim_trailing_slash(&input.url)),
            secret_key: input.secret_key.clone(),
        }),
    }
}

// ---------------------------------------------------------------------------
// The provider
// ---------------------------------------------------------------------------

/// TS `SupabaseAuthInput`. No `Debug`: it holds the secret key and the shared secret.
#[derive(Clone, Default)]
pub struct SupabaseAuthInput {
    /// `env.supabase_url`, e.g. `https://<ref>.supabase.co`.
    pub url: String,
    /// `env.supabase_secret_key`. Server-only; bypasses RLS; never leaves this module.
    pub secret_key: String,
    /// `env.supabase_jwks_url`; defaults to the project's own well-known endpoint.
    pub jwks_url: Option<String>,
    /// `env.supabase_jwt_secret`: the legacy HS256 shared secret, if the project still signs with one.
    pub jwt_secret: Option<String>,
    /// Not in SPEC, and no R-row: test seam for the JWKS. Production leaves it unset and fetches
    /// `jwks_url`; a test hands it a local key set instead.
    pub key_set: Option<JwkSet>,
    /// Not in SPEC, and no R-row: test seam for the clock behind the provider's caches
    /// (`EMAIL_CONFIRMED_CACHE_TTL_MS`, R194's live sessions, the key set's age); defaults to the
    /// server's clock (`app::now_ms`). TS `now?: () => number`. A test drives the caches with it
    /// while the provider's requests run in real time: tokio's paused clock would jump to their
    /// timeouts whenever the runtime waits on the socket.
    pub now: Option<ProviderClock>,
}

/// The clock `SupabaseAuthInput.now` hands the provider: epoch milliseconds.
pub type ProviderClock = Arc<dyn Fn() -> i64 + Send + Sync>;

#[derive(Clone, Debug, PartialEq)]
struct LocalClaims {
    sub: String,
    email: Option<String>,
    app_metadata: IndexMap<String, Value>,
    /// The provider's session this token belongs to (`session_id`), when the token names one.
    session_id: Option<String>,
    /// R665: the token's assurance level (`aal`), from the verified payload.
    aal: Option<String>,
}

/// What the provider said about a verified token's session (R194).
#[derive(Clone, Debug, PartialEq)]
enum SessionCheck {
    /// It was live a moment ago (remembered); nothing new was learned about the user.
    Remembered,
    /// Asked just now: live, and this is the authoritative user.
    Live(AuthApiUser),
    /// The provider has ended the session (or the user): the token must not be honoured.
    Ended,
    /// Nobody answered. The identity stands, as for R159's outage; see `verify_access_token`.
    Unavailable,
}

/// A confirmed email as last read from the auth server.
#[derive(Clone, Debug)]
struct Confirmed {
    at: i64,
    email: Option<String>,
}

/// The key set as last fetched (or as a test handed it in, which is never refetched).
#[derive(Debug, Default)]
struct KeyCache {
    keys: Option<JwkSet>,
    fetched_at: i64,
    fixed: bool,
}

/// TS `createSupabaseAuth`'s closure, as a struct. The caches are behaviour (R194, R665, the
/// proposed ruling above), not optimisations.
pub struct SupabaseAuth {
    auth_base: String,
    jwks_url: String,
    secret_key: String,
    http: reqwest::Client,
    clients: SupabaseAuthClients,
    /// The JWKS. Fetched on first use so constructing the provider does no I/O.
    key_set: tokio::sync::Mutex<KeyCache>,
    // env.ts calls the shared secret "discouraged": a leaked one lets an attacker mint any `sub`.
    // It is only tried when the JWKS path has already failed, and only if configured.
    hs_key: Option<Vec<u8>>,
    /// userId -> when its *confirmed* email was last read from the auth server.
    confirmed: Mutex<IndexMap<String, Confirmed>>,
    /// R665: the users the provider last said have a verified authenticator app. Kept for as long as
    /// that stays the latest answer (positives only; a later answer without one removes the entry), so
    /// an `aal1` token for such an account is refused even while the provider cannot be reached or the
    /// answer is being served from `confirmed`.
    mfa_enrolled: Mutex<IndexSet<String>>,
    /// session id -> when the provider last said that session is live (R194). Positives only.
    live_sessions: Mutex<IndexMap<String, i64>>,
    /// The caches' clock (`SupabaseAuthInput.now`).
    now: ProviderClock,
}

/// Neither the secret key nor the shared secret reaches a log line, `{:?}` included.
impl std::fmt::Debug for SupabaseAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SupabaseAuth")
            .field("auth_base", &self.auth_base)
            .field("jwks_url", &self.jwks_url)
            .field("shared_secret", &self.hs_key.is_some())
            .finish_non_exhaustive()
    }
}

impl SupabaseAuth {
    /// TS `createSupabaseAuth(input)`.
    pub fn new(input: SupabaseAuthInput) -> SupabaseAuth {
        let base_url = trim_trailing_slash(&input.url);
        let auth_base = format!("{base_url}/auth/v1");
        let jwks_url = input
            .jwks_url
            .clone()
            .unwrap_or_else(|| format!("{auth_base}/.well-known/jwks.json"));
        let http = reqwest::Client::new();
        let clients = create_real_clients(
            &SupabaseAuthClientInput {
                url: base_url.clone(),
                secret_key: input.secret_key.clone(),
            },
            &http,
        );
        let key_cache = match input.key_set {
            Some(keys) => KeyCache {
                keys: Some(keys),
                fetched_at: 0,
                fixed: true,
            },
            None => KeyCache::default(),
        };
        SupabaseAuth {
            auth_base,
            jwks_url,
            secret_key: input.secret_key,
            http,
            clients,
            key_set: tokio::sync::Mutex::new(key_cache),
            hs_key: input.jwt_secret.map(String::into_bytes),
            confirmed: Mutex::new(IndexMap::new()),
            mfa_enrolled: Mutex::new(IndexSet::new()),
            live_sessions: Mutex::new(IndexMap::new()),
            now: input.now.unwrap_or_else(|| Arc::new(crate::app::now_ms)),
        }
    }

    fn learn_mfa(&self, user: &AuthApiUser) {
        let mut enrolled = self
            .mfa_enrolled
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if user.mfa_enrolled {
            enrolled.insert(user.id.clone());
        } else {
            enrolled.shift_remove(&user.id);
        }
    }

    /// R665: the token's level is short of what the account needs.
    fn missing_second_factor(&self, user_id: &str, aal: Option<&str>) -> bool {
        let enrolled = self
            .mfa_enrolled
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        aal != Some(SECOND_FACTOR_LEVEL) && enrolled.contains(user_id)
    }

    fn remember_confirmed(&self, user_id: &str, at: i64, email: Option<String>) {
        let mut confirmed = self
            .confirmed
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        confirmed.insert(user_id.to_string(), Confirmed { at, email });
    }

    /// jose's `jwtVerify` options: issuer `${SUPABASE_URL}/auth/v1`, audience `authenticated`, both
    /// required when given; `exp` and `nbf` checked when present, with no clock tolerance.
    fn verify_options(&self, algorithms: Vec<Algorithm>) -> Validation {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.algorithms = algorithms;
        validation.set_issuer(&[self.auth_base.as_str()]);
        validation.set_audience(&[AUTHENTICATED_AUDIENCE]);
        validation.set_required_spec_claims(&["iss", "aud"]);
        validation.leeway = 0;
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation
    }

    fn claims_from(payload: &Value) -> Option<LocalClaims> {
        let sub = payload.get("sub").and_then(Value::as_str)?;
        if sub.is_empty() {
            return None;
        }
        let session_id = payload
            .get("session_id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty());
        Some(LocalClaims {
            sub: sub.to_string(),
            email: payload.get("email").and_then(Value::as_str).map(str::to_string),
            // `app_metadata` only. `user_metadata` is user-editable and is never read.
            app_metadata: as_record(payload.get("app_metadata")),
            session_id: session_id.map(str::to_string),
            aal: payload.get("aal").and_then(Value::as_str).map(str::to_string),
        })
    }

    async fn fetch_key_set(&self) -> Option<JwkSet> {
        let response = self
            .http
            .get(&self.jwks_url)
            .header("accept", "application/json")
            .timeout(Duration::from_millis(JWKS_FETCH_TIMEOUT_MS))
            .send()
            .await
            .ok()?;
        if !response.status().is_success() {
            return None;
        }
        response.json::<JwkSet>().await.ok()
    }

    /// The keys of the set that could have signed a token with this header: its `kid` when it names
    /// one, an `alg` the key allows, a key type of the algorithm's family, and a signing use.
    fn candidates(keys: &JwkSet, header: &Header) -> Vec<DecodingKey> {
        keys.keys
            .iter()
            .filter(|jwk| match &header.kid {
                Some(kid) => jwk.common.key_id.as_deref() == Some(kid.as_str()),
                None => true,
            })
            .filter(|jwk| match jwk.common.key_algorithm {
                Some(algorithm) => Algorithm::try_from(algorithm).ok() == Some(header.alg),
                None => true,
            })
            .filter(|jwk| {
                !matches!(
                    jwk.common.public_key_use,
                    Some(PublicKeyUse::Encryption | PublicKeyUse::Other(_))
                )
            })
            .filter(|jwk| {
                matches!(
                    (&jwk.algorithm, header.alg.family()),
                    (AlgorithmParameters::RSA(_), AlgorithmFamily::Rsa)
                        | (AlgorithmParameters::EllipticCurve(_), AlgorithmFamily::Ec)
                        | (AlgorithmParameters::OctetKeyPair(_), AlgorithmFamily::Ed)
                )
            })
            .filter_map(|jwk| DecodingKey::from_jwk(jwk).ok())
            .collect()
    }

    /// TS `keys()` as jose's remote key set ran it: fetched on first use, refetched once it is old,
    /// and refetched (at most once per cooldown) when no key in it matches the token.
    async fn keys_for(&self, header: &Header) -> Vec<DecodingKey> {
        let mut cache = self.key_set.lock().await;
        if cache.fixed {
            return cache
                .keys
                .as_ref()
                .map(|keys| Self::candidates(keys, header))
                .unwrap_or_default();
        }
        let now = (self.now)();
        if cache.keys.is_none() || now - cache.fetched_at >= JWKS_CACHE_MAX_AGE_MS {
            match self.fetch_key_set().await {
                Some(keys) => {
                    cache.keys = Some(keys);
                    cache.fetched_at = now;
                }
                None => return Vec::new(),
            }
        }
        let found = cache
            .keys
            .as_ref()
            .map(|keys| Self::candidates(keys, header))
            .unwrap_or_default();
        if !found.is_empty() || now - cache.fetched_at < JWKS_COOLDOWN_MS {
            return found;
        }
        match self.fetch_key_set().await {
            Some(keys) => {
                let refetched = Self::candidates(&keys, header);
                cache.keys = Some(keys);
                cache.fetched_at = now;
                refetched
            }
            None => Vec::new(),
        }
    }

    // Both verifiers swallow their error: it covers a forged or expired token, a project that
    // signs symmetrically (the JWKS then publishes no matching key) and a JWKS that could not be
    // fetched. The caller falls through to the next tier.
    async fn verify_against_jwks(&self, token: &str) -> Option<LocalClaims> {
        let header = jsonwebtoken::decode_header(token).ok()?;
        if header.alg.family() == AlgorithmFamily::Hmac {
            return None;
        }
        let validation = self.verify_options(vec![header.alg]);
        for key in self.keys_for(&header).await {
            if let Ok(data) = jsonwebtoken::decode::<Value>(token, &key, &validation) {
                return Self::claims_from(&data.claims);
            }
        }
        None
    }

    fn verify_against_secret(&self, token: &str, key: &[u8]) -> Option<LocalClaims> {
        let validation = self.verify_options(AlgorithmFamily::Hmac.algorithms().to_vec());
        let data = jsonwebtoken::decode::<Value>(token, &DecodingKey::from_secret(key), &validation).ok()?;
        Self::claims_from(&data.claims)
    }

    /// Ask the auth server who a token belongs to. Two callers: tier 3, the last-resort verification
    /// for a project still signing with a symmetric secret this server has not been given, and
    /// `check_session` (R194), since the provider refuses a token whose session it has ended. The
    /// `apikey` header is the secret key (TS: the publishable key when one is configured, which no
    /// Rust deployment does) — Supabase's own credential, never echoed back to a caller.
    async fn fetch_user_by_token(&self, token: &str) -> AdminLookup {
        let response = match self
            .http
            .get(format!("{}/user", self.auth_base))
            .header("apikey", &self.secret_key)
            .header("authorization", format!("Bearer {token}"))
            .header("accept", "application/json")
            // The session check (R194) sits in front of API requests: a provider that hangs must cost
            // them a bounded wait, after which the answer is "unavailable" and the identity stands.
            .timeout(Duration::from_millis(
                AUTH_PROVIDER_TIMEOUT_SECONDS as u64 * MS_PER_SECOND as u64,
            ))
            .send()
            .await
        {
            Ok(response) => response,
            Err(_) => return AdminLookup::Unavailable,
        };
        let status = response.status().as_u16();
        if status == 401 || status == 403 {
            return AdminLookup::Missing;
        }
        if !response.status().is_success() {
            return AdminLookup::Unavailable;
        }
        let body: Value = match response.json().await {
            Ok(body) => body,
            Err(_) => return AdminLookup::Unavailable,
        };
        match as_auth_api_user(&body) {
            Some(user) => AdminLookup::Ok(user),
            None => AdminLookup::Unavailable,
        }
    }

    /// §9.4 step 1's input, taken from the auth server rather than from the token.
    async fn authoritative_user(&self, user_id: &str) -> AdminLookup {
        let cached = {
            let confirmed = self
                .confirmed
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            confirmed.get(user_id).cloned()
        };
        if let Some(cached) = cached
            && (self.now)() - cached.at < EMAIL_CONFIRMED_CACHE_TTL_MS
        {
            let mfa_enrolled = {
                let enrolled = self
                    .mfa_enrolled
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                enrolled.contains(user_id)
            };
            return AdminLookup::Ok(AuthApiUser {
                id: user_id.to_string(),
                email: cached.email,
                mfa_enrolled,
                // Only confirmed emails are cached, so a hit means confirmed. The timestamp's value is
                // never shown to anyone; `is_email_confirmed` only asks whether it is set.
                email_confirmed_at: Some(iso_of(cached.at)),
                app_metadata: IndexMap::new(),
            });
        }

        let Some(admin) = self.clients.admin.as_ref() else {
            return AdminLookup::Unavailable;
        };
        let lookup = admin.get_user_by_id(user_id).await;
        if let AdminLookup::Ok(user) = &lookup {
            self.learn_mfa(user);
            if is_email_confirmed(user) {
                self.remember_confirmed(user_id, (self.now)(), user.email.clone());
            }
        }
        lookup
    }

    /// R194: is the session this verified token names still live at the provider? Asked of
    /// `GET /auth/v1/user` with the token itself, which the provider refuses once the session (or the
    /// user) is gone. Only a live answer is remembered, per session, so a session ended at the
    /// provider is refused here within `AUTH_SESSION_LIVE_CACHE_SECONDS`.
    async fn check_session(&self, session_id: &str, sub: &str, token: &str) -> SessionCheck {
        let live_session_ttl_ms = AUTH_SESSION_LIVE_CACHE_SECONDS * MS_PER_SECOND;
        let seen = {
            let live = self
                .live_sessions
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            live.get(session_id).copied()
        };
        if let Some(at) = seen
            && (self.now)() - at < live_session_ttl_ms
        {
            return SessionCheck::Remembered;
        }

        let user = match self.fetch_user_by_token(token).await {
            AdminLookup::Unavailable => return SessionCheck::Unavailable,
            // `/user` answers for the token's own user; any other id is the provider misbehaving, and a
            // token is never honoured on someone else's answer.
            AdminLookup::Ok(user) if user.id == sub => user,
            AdminLookup::Ok(_) | AdminLookup::Missing => {
                let mut live = self
                    .live_sessions
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                live.shift_remove(session_id);
                return SessionCheck::Ended;
            }
        };

        let checked_at = (self.now)();
        {
            let mut live = self
                .live_sessions
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            // Forget sessions whose answer has lapsed, so the map holds only the recently active ones.
            live.retain(|_, seen| checked_at - *seen < live_session_ttl_ms);
            live.insert(session_id.to_string(), checked_at);
        }
        self.learn_mfa(&user);
        // The same answer is R159's authoritative user: a confirmed email is remembered as the admin
        // lookup's would be.
        if is_email_confirmed(&user) {
            self.remember_confirmed(sub, checked_at, user.email.clone());
        }
        SessionCheck::Live(user)
    }

    /// TS `verifyAccessToken`: the user a bearer token names, or `None` for anything not currently
    /// valid.
    pub async fn verify_access_token(&self, token: &str) -> Option<AuthUser> {
        if token.is_empty() {
            return None;
        }

        // Tier 1: the project's published asymmetric keys, its issuer and the `authenticated`
        // audience — no round trip, which is why §9.2's browser-to-auth arrow can stay separate.
        let mut claims = self.verify_against_jwks(token).await;
        // Tier 2: the legacy HS256 shared secret, when the deployment has one.
        if claims.is_none()
            && let Some(key) = self.hs_key.as_deref()
        {
            claims = self.verify_against_secret(token, key);
        }

        if let Some(claims) = claims {
            if let Some(session_id) = claims.session_id.as_deref() {
                match self.check_session(session_id, &claims.sub, token).await {
                    // Ended at the provider (a sign-out, a dropped link, a reset elsewhere): not valid.
                    SessionCheck::Ended => return None,
                    SessionCheck::Live(user) => {
                        // R665: an account with an authenticator app needs the code's level.
                        if self.missing_second_factor(&claims.sub, claims.aal.as_deref()) {
                            return None;
                        }
                        let authoritative = to_auth_user(&user);
                        return Some(AuthUser {
                            user_id: claims.sub,
                            email: authoritative.email.or(claims.email),
                            email_verified: authoritative.email_verified,
                            app_metadata: claims.app_metadata,
                        });
                    }
                    // `Remembered` or `Unavailable`: the admin lookup below decides the email. A provider
                    // that cannot be reached does not sign every player out (sign-in is down with it); the
                    // identity stands, and the email counts as unverified unless it was already known (R159).
                    SessionCheck::Remembered | SessionCheck::Unavailable => {}
                }
            }
            let lookup = self.authoritative_user(&claims.sub).await;
            // The auth server says this user no longer exists: not currently valid.
            if lookup == AdminLookup::Missing {
                return None;
            }
            // R665, after the lookup (which refreshes what is known): on an outage, the last answer.
            if self.missing_second_factor(&claims.sub, claims.aal.as_deref()) {
                return None;
            }
            return match lookup {
                AdminLookup::Ok(user) => {
                    let authoritative = to_auth_user(&user);
                    Some(AuthUser {
                        user_id: claims.sub,
                        email: authoritative.email.or(claims.email),
                        email_verified: authoritative.email_verified,
                        // From the verified token's provider-controlled claim, not from the lookup.
                        app_metadata: claims.app_metadata,
                    })
                }
                // Fail closed on the security-relevant field: the signature proved who this is, but
                // nothing proved the email is confirmed, so §9.4 step 1 must not pass.
                AdminLookup::Unavailable | AdminLookup::Missing => Some(AuthUser {
                    user_id: claims.sub,
                    email: claims.email,
                    email_verified: false,
                    app_metadata: claims.app_metadata,
                }),
            };
        }

        // Tier 3: unverifiable locally — ask the auth server, which is the authority either way.
        // Only for the one token this server could not have checked itself: a legacy HS256 token
        // while no shared secret is configured. Anything else that failed tiers 1 and 2 is forged,
        // expired or not a JWT at all, and asking the provider about it would let any caller make
        // this server send one request upstream per request it receives.
        if self.hs_key.is_some() || !signed_with_shared_secret(token) {
            return None;
        }
        let AdminLookup::Ok(user) = self.fetch_user_by_token(token).await else {
            return None;
        };
        self.learn_mfa(&user);
        // R665. The provider has just accepted this token, so its payload is the provider's own.
        let payload: Value = jsonwebtoken::dangerous::insecure_decode_claims(token).ok()?;
        let aal = payload.get("aal").and_then(Value::as_str);
        if self.missing_second_factor(&user.id, aal) {
            return None;
        }
        Some(to_auth_user(&user))
    }

    /// The server-side password path is not ported (file header): every caller is told where
    /// sign-in actually happens, exactly as TS's `requirePasswordClient` told them on a deployment
    /// with no publishable key.
    pub async fn sign_in_with_password(&self, _email: &str, _password: &str) -> Result<Session, AuthError> {
        Err(AuthError::Unavailable(PASSWORD_PATH_DISABLED_MESSAGE.to_string()))
    }

    // Deleting a user ends its sessions and refresh tokens at the provider, but an access token
    // already issued stays well-signed until it expires. It stops working here at once all the
    // same: the confirmed-email memory is dropped below, so the next verification asks the
    // provider, which answers "no such user" (see `verify_access_token`).
    pub async fn delete_user(&self, user_id: &str) -> Result<(), AuthError> {
        let Some(admin) = self.clients.admin.as_ref() else {
            return Err(AuthError::Unavailable(
                ACCOUNT_DELETION_UNAVAILABLE_MESSAGE.to_string(),
            ));
        };
        if admin.delete_user(user_id).await == AdminDeletion::Unavailable {
            return Err(AuthError::Unavailable(ACCOUNT_DELETION_RETRY_MESSAGE.to_string()));
        }
        {
            let mut confirmed = self
                .confirmed
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            confirmed.shift_remove(user_id);
        }
        let mut enrolled = self
            .mfa_enrolled
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        enrolled.shift_remove(user_id);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// BUILD M8's fixture provider (`apps/server/src/api/e2e.ts`, `createE2EAuth`) and the test double
// (`apps/server/test/fakes/deps.ts`, `createFakeAuth`), one struct
// ---------------------------------------------------------------------------

/// One fixture account (`e2e/support/config.ts`, `accounts`): user id, email, password, token.
/// A private copy of `api::e2e::E2E_ACCOUNTS`' identity half; `api::e2e` owns the reseed.
const E2E_FIXTURE_ACCOUNTS: &[(&str, &str, &str, &str)] = &[
    ("e2e-p1", "e2e-p1@jackioh.test", "e2e-p1-password", "e2e-token-p1"),
    ("e2e-p2", "e2e-p2@jackioh.test", "e2e-p2-password", "e2e-token-p2"),
    (
        "e2e-pending",
        "e2e-pending@jackioh.test",
        "e2e-pending-password",
        "e2e-token-pending",
    ),
];

/// The plain error a fixture sign-in fails with; `api::auth::call_provider` turns it into the one
/// 401 whose wording never distinguishes "no such account" from "wrong password".
const INVALID_LOGIN_CREDENTIALS: &str = "invalid login credentials";

/// What `delete_user` does on this provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum E2eDeletion {
    /// TS's fixture provider had no `deleteUser`, so `DELETE /api/account` answers 503 (the default
    /// under `E2E=1`).
    Unsupported,
    /// The test double's: the user and every token of theirs stop verifying.
    Deletes,
    /// The provider answers this `unavailable` message (a test of the retry path).
    Fails(String),
}

#[derive(Clone, Debug)]
struct E2eUser {
    user: AuthUser,
    password: Option<String>,
}

#[derive(Debug)]
struct E2eState {
    /// userId -> the user, in registration order.
    users: IndexMap<String, E2eUser>,
    /// bearer token -> userId.
    tokens: IndexMap<String, String>,
    deletion: E2eDeletion,
}

/// The `AuthProvider` BUILD M8's fixture accounts sign in with. `verify` maps each static token in
/// `e2e/support/config.ts` to its account and everything else to `Invalid`; `sign_in` accepts the
/// fixture email/password pairs. The test support (`tests/support/deps.rs`) also registers users on
/// it (`add_user`), as TS's `createFakeAuth` did.
///
/// §9.4 makes a verified email a precondition of redemption, and spec 10 asserts
/// `emailVerified === true` on the *pending* account before it redeems. So all three fixtures are
/// verified; what distinguishes `e2e-pending` is its `profiles.status`. `app_metadata` stays empty:
/// nothing here is an authorization claim.
#[derive(Debug)]
pub struct E2eAuth {
    state: Mutex<E2eState>,
}

impl Default for E2eAuth {
    fn default() -> Self {
        E2eAuth::new()
    }
}

impl E2eAuth {
    /// TS `createE2EAuth()`: the three fixture accounts.
    pub fn new() -> E2eAuth {
        let auth = E2eAuth {
            state: Mutex::new(E2eState {
                users: IndexMap::new(),
                tokens: IndexMap::new(),
                deletion: E2eDeletion::Unsupported,
            }),
        };
        for &(user_id, email, password, token) in E2E_FIXTURE_ACCOUNTS {
            auth.register(user_id, email, true, Some(password), token);
        }
        auth
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, E2eState> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn register(
        &self,
        user_id: &str,
        email: &str,
        email_verified: bool,
        password: Option<&str>,
        token: &str,
    ) {
        let mut state = self.lock();
        state.users.insert(
            user_id.to_string(),
            E2eUser {
                user: AuthUser {
                    user_id: user_id.to_string(),
                    email: Some(email.to_string()),
                    email_verified,
                    app_metadata: IndexMap::new(),
                },
                password: password.map(str::to_string),
            },
        );
        state.tokens.insert(token.to_string(), user_id.to_string());
    }

    /// TS `FakeAuth.addUser`: registers a user and returns the bearer token that verifies as them.
    pub fn add_user(&self, user_id: &str, email: &str, email_verified: bool) -> String {
        let token = format!("token-{user_id}");
        self.register(user_id, email, email_verified, None, &token);
        token
    }

    /// TS `FakeAuth.setEmailVerified`.
    pub fn set_email_verified(&self, user_id: &str, verified: bool) {
        let mut state = self.lock();
        if let Some(entry) = state.users.get_mut(user_id) {
            entry.user.email_verified = verified;
        }
    }

    /// Chooses what `delete_user` does (see `E2eDeletion`).
    pub fn set_deletion(&self, deletion: E2eDeletion) {
        self.lock().deletion = deletion;
    }

    pub fn can_delete_users(&self) -> bool {
        self.lock().deletion != E2eDeletion::Unsupported
    }

    pub fn verify_access_token(&self, token: &str) -> Option<AuthUser> {
        let state = self.lock();
        let user_id = state.tokens.get(token)?;
        state.users.get(user_id).map(|entry| entry.user.clone())
    }

    pub fn sign_in_with_password(&self, email: &str, password: &str) -> Result<Session, AuthError> {
        let state = self.lock();
        let wanted = email.trim().to_lowercase();
        let found = state.users.values().find(|entry| {
            entry.user.email.as_deref().map(str::to_lowercase).as_deref() == Some(wanted.as_str())
        });
        // A plain error, not an unavailable one: the route turns it into the one 401.
        let Some(entry) = found else {
            return Err(AuthError::Rejected(INVALID_LOGIN_CREDENTIALS.to_string()));
        };
        if entry.password.as_deref() != Some(password) {
            return Err(AuthError::Rejected(INVALID_LOGIN_CREDENTIALS.to_string()));
        }
        let token = state
            .tokens
            .iter()
            .find(|(_, owner)| **owner == entry.user.user_id)
            .map(|(token, _)| token.clone())
            .unwrap_or_default();
        Ok(Session {
            access_token: token,
            refresh_token: None,
            expires_at: None,
            user: entry.user.clone(),
        })
    }

    // A deleted user's tokens stop verifying, as the real provider's session check makes them.
    pub fn delete_user(&self, user_id: &str) -> Result<(), AuthError> {
        let mut state = self.lock();
        match state.deletion.clone() {
            E2eDeletion::Unsupported => Err(AuthError::Unavailable(
                ACCOUNT_DELETION_UNAVAILABLE_MESSAGE.to_string(),
            )),
            E2eDeletion::Fails(message) => Err(AuthError::Unavailable(message)),
            E2eDeletion::Deletes => {
                state.users.shift_remove(user_id);
                state.tokens.retain(|_, owner| owner != user_id);
                Ok(())
            }
        }
    }
}

// ---------------------------------------------------------------------------
// The enum (SURFACE §11.2)
// ---------------------------------------------------------------------------

/// The auth provider: Supabase in production, the fixtures under `E2E=1` and in the tests.
///
/// SURFACE §11.2 freezes the variants unboxed; there is one `Auth` per `App`, so a `Box` would buy
/// nothing for the size difference clippy reports.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Auth {
    Supabase(SupabaseAuth),
    E2e(E2eAuth),
}

impl Auth {
    /// Verify a bearer token: the user it names, or `AuthError::Invalid` for anything not
    /// currently valid (TS `verifyAccessToken`'s `null`).
    pub async fn verify(&self, token: &str) -> Result<AuthUser, AuthError> {
        let user = match self {
            Auth::Supabase(auth) => auth.verify_access_token(token).await,
            Auth::E2e(auth) => auth.verify_access_token(token),
        };
        user.ok_or(AuthError::Invalid)
    }

    /// TS `signInWithPassword`: the fixture accounts under `E2E=1`; Supabase answers unavailable.
    pub async fn sign_in(&self, email: &str, password: &str) -> Result<Session, AuthError> {
        match self {
            Auth::Supabase(auth) => auth.sign_in_with_password(email, password).await,
            Auth::E2e(auth) => auth.sign_in_with_password(email, password),
        }
    }

    /// Deletes the provider's user for good (`DELETE /api/account`). A user that is already gone
    /// counts as deleted. `AuthError::Unavailable` when the provider cannot be reached or cannot
    /// delete users.
    pub async fn delete_user(&self, user_id: &str) -> Result<(), AuthError> {
        match self {
            Auth::Supabase(auth) => auth.delete_user(user_id).await,
            Auth::E2e(auth) => auth.delete_user(user_id),
        }
    }

    /// TS `deps.auth.deleteUser !== undefined`: whether this provider can delete users at all, which
    /// `DELETE /api/account` asks before it touches anything.
    pub fn can_delete_users(&self) -> bool {
        match self {
            Auth::Supabase(_) => true,
            Auth::E2e(auth) => auth.can_delete_users(),
        }
    }
}
