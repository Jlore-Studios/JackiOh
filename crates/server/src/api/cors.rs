//! CORS for the REST surface. Port of `apps/server/src/api/cors.ts`.
//!
//! WHY IT IS NEEDED. SPEC §9.2 puts the browser and the API on separate arrows, and in this repo
//! they are separate origins: `apps/web` is served from `http://localhost:5173` (Vite) and calls
//! the server on `http://localhost:8787` (`apiBaseUrl()` in `apps/web/src/net/api.ts`). Without
//! these headers every `fetch` from the app is blocked by the browser before the handler is ever
//! reached, while a non-browser caller — `cy.request`, curl, the `wsPlayer` task — is unaffected.
//! That asymmetry is exactly the failure mode worth avoiding: the e2e suite's HTTP assertions would
//! pass while the screens they are about stay empty.
//!
//! `env.rs` already names the list this reads: "PUBLIC_ORIGINS: allowed browser origins for
//! CORS and WebSocket `Origin` checks". `actor/ws_server.rs` is the other reader.
//!
//! THREE RULES THIS FILE KEEPS — never `*` and never credentials, `Vary: Origin` on everything it
//! touches, and an unlisted origin is refused by silence rather than by a status. They are stated
//! once, as the proposal below, rather than twice here.
//!
//! NOT IN SPEC, and no R-row yet — PROPOSED RULING for §11:
//!   Topic: The CORS contract for the REST surface
//!   Ruling: The API answers a browser from an allowed origin by echoing that one origin, never
//!     `*`, and never sends `Access-Control-Allow-Credentials`: §9.1's client authenticates with a
//!     bearer token in a header, so a cookie is never in play and the wildcard is never needed, and
//!     a response that carried credentials would make every allowed origin able to act as the
//!     player. Every response the layer touches carries `Vary: Origin`, because the answer depends
//!     on the request's origin and a shared cache must not serve one origin's answer to another.
//!     An unlisted origin is not an error: it gets the ordinary response with no CORS headers,
//!     which is what the browser needs in order to refuse it, where a 403 would tell a page nothing
//!     it could not already tell and would change the answer a non-browser caller gets — `cy.request`
//!     and the `wsPlayer` task are not subject to CORS at all, and §9.1 puts the rules in the
//!     server, so CORS must never be load-bearing for a refusal. The allowed list is
//!     `PUBLIC_ORIGINS`, the same list §9.2's WebSocket `Origin` check reads, so the two doors
//!     cannot diverge.
//!   Affects: §9.1, §9.2, §9.8; `api/cors.ts`, `match/wsServer.ts`, `env.ts`.
//!
//! §9 never writes out the header set itself, and it is derived rather than chosen: the two request
//! headers below are the two `apps/web/src/net/api.ts` sends (`authorization` and `content-type`)
//! and the methods are the five the router's routes use.

use std::future::Future;

use axum::body::Body;
use axum::extract::Request;
use axum::http::{HeaderValue, Method};
use axum::response::Response;

/// The methods the router's routes use (TS `Route["method"]`), plus the preflight itself.
const ALLOWED_METHODS: &str = "GET, POST, PUT, PATCH, DELETE, OPTIONS";

/// Exactly what `apps/web/src/net/api.ts` sets on a request.
const ALLOWED_HEADERS: &str = "authorization, content-type";

/// Not in SPEC, and no R-row: a tuning value with no consequence. Ten minutes of preflight caching
/// saves a round trip; nothing in the CORS contract above, and nothing a player or a client can
/// observe, changes if it is zero or an hour.
const PREFLIGHT_MAX_AGE_SECONDS: u32 = 600;

/// BUILD M8: "`E2E=1` pnpm --dir apps/web dev — must serve http://localhost:5173" (`e2e/README.md`).
/// End-to-end mode adds these to `PUBLIC_ORIGINS` so a checkout with no `.env` still lets the app
/// talk to the server; production adds nothing and reads only the environment.
pub const VITE_DEV_ORIGINS: &[&str] = &["http://localhost:5173", "http://127.0.0.1:5173"];

/// TS `CorsOptions`. The refused-preflight line goes to the tracing log, always.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CorsOptions {
    /// `env.public_origins`, plus the dev origins end-to-end mode adds (`app::browser_origins`).
    pub origins: Vec<String>,
}

/// `http://localhost:5173/` and `http://localhost:5173` are the same origin; a browser always sends
/// the second form, but a hand-written `PUBLIC_ORIGINS` may hold the first.
fn canonical_origin(value: &str) -> String {
    value.trim().trim_end_matches('/').to_string()
}

pub fn is_origin_allowed(origins: &[String], origin: Option<&str>) -> bool {
    let Some(origin) = origin else { return false };
    if origin.is_empty() {
        return false;
    }
    let wanted = canonical_origin(origin);
    origins.iter().any(|allowed| canonical_origin(allowed) == wanted)
}

/// The headers an allowed origin gets on every response, preflight or not.
fn cors_headers(origin: &str) -> Vec<(&'static str, String)> {
    vec![("access-control-allow-origin", canonical_origin(origin)), ("vary", "Origin".to_string())]
}

fn set_header(response: &mut Response, name: &'static str, value: &str) {
    if let Ok(value) = HeaderValue::from_str(value) {
        response.headers_mut().insert(name, value);
    }
}

fn empty(status: u16) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() = axum::http::StatusCode::from_u16(status).unwrap_or(axum::http::StatusCode::NO_CONTENT);
    response
}

/// TS `withCors(handler, options)`, applied to one request: preflights are answered here and never
/// reach the router, which knows no `OPTIONS` route and would answer 404 for one; every other
/// request runs `handler` and has its CORS headers set on the way out.
///
/// The peer address the router reads for R190 travels in the request's extensions and is passed
/// through unchanged: CORS never reads it.
pub async fn with_cors<F, Fut>(options: &CorsOptions, request: Request, handler: F) -> Response
where
    F: FnOnce(Request) -> Fut,
    Fut: Future<Output = Response>,
{
    let origins: Vec<String> =
        options.origins.iter().map(|origin| canonical_origin(origin)).filter(|origin| !origin.is_empty()).collect();
    let origin: Option<String> =
        request.headers().get("origin").and_then(|value| value.to_str().ok()).map(str::to_string);
    let allowed = is_origin_allowed(&origins, origin.as_deref());

    if request.method() == Method::OPTIONS && request.headers().contains_key("access-control-request-method") {
        if !allowed {
            tracing::warn!(
                event = "cors.preflight_refused",
                origin = %origin.as_deref().unwrap_or("(none)"),
                allowed = %serde_json::to_string(&origins).unwrap_or_default(),
            );
            // No CORS headers: the browser refuses it, which is the correct outcome. `Vary: Origin`
            // still goes on, because R162 puts it on EVERY response this layer touches — the answer
            // depends on the request's origin, so a shared cache must not serve this one to an origin
            // that would have been allowed.
            let mut response = empty(204);
            set_header(&mut response, "vary", "Origin");
            return response;
        }
        let mut response = empty(204);
        for (name, value) in cors_headers(origin.as_deref().unwrap_or("")) {
            set_header(&mut response, name, &value);
        }
        set_header(&mut response, "access-control-allow-methods", ALLOWED_METHODS);
        set_header(&mut response, "access-control-allow-headers", ALLOWED_HEADERS);
        set_header(&mut response, "access-control-max-age", &PREFLIGHT_MAX_AGE_SECONDS.to_string());
        return response;
    }

    let mut response = handler(request).await;
    match origin {
        Some(origin) if allowed => {
            for (name, value) in cors_headers(&origin) {
                set_header(&mut response, name, &value);
            }
        }
        _ => {
            // R162 again: the ordinary response an unlisted or origin-less caller gets is still an
            // answer that depended on the origin, so it is still cacheable-per-origin. Without this a
            // shared cache can store the no-CORS-headers answer and later hand it to an allowed
            // origin, which is exactly the failure the clause names.
            set_header(&mut response, "vary", "Origin");
        }
    }
    response
}
