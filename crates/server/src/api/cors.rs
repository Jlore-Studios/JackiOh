//! CORS for the REST surface. SPEC §9.2 puts the browser and the API on separate origins, so
//! without these headers every browser `fetch` is blocked while `cy.request`, curl and `wsPlayer`
//! are not. The allowed list is `PUBLIC_ORIGINS` (`env.rs`; `actor/ws_server.rs` also reads it).
//! §9 never writes out the header set: it is derived from what `apps/web/src/net/api.ts` sends.
//!
//! NOT IN SPEC, and no R-row yet — PROPOSED RULING for §11:
//!   Topic: The CORS contract for the REST surface
//!   Ruling: Echo an allowed origin, never `*`, and never send `Access-Control-Allow-Credentials`
//!     (§9.1's client uses a bearer header). Every response the layer touches carries `Vary:
//!     Origin`. An unlisted origin gets the ordinary response with no CORS headers, not a 403:
//!     §9.1 puts the rules in the server, so CORS is never load-bearing for a refusal. The list is
//!     the one §9.2's WebSocket `Origin` check reads.
//!   Affects: §9.1, §9.2, §9.8; `env.rs`.

use std::future::Future;

use axum::body::Body;
use axum::extract::Request;
use axum::http::{HeaderValue, Method};
use axum::response::Response;

/// The methods the router's routes use, plus the preflight itself.
const ALLOWED_METHODS: &str = "GET, POST, PUT, PATCH, DELETE, OPTIONS";

/// Exactly what `apps/web/src/net/api.ts` sets on a request.
const ALLOWED_HEADERS: &str = "authorization, content-type";

/// Not in SPEC, and no R-row: a tuning value with no consequence; ten minutes of preflight caching
/// saves a round trip.
const PREFLIGHT_MAX_AGE_SECONDS: u32 = 600;

/// BUILD M8: "`E2E=1` pnpm --dir apps/web dev — must serve http://localhost:5173" (`e2e/README.md`).
/// End-to-end mode adds these to `PUBLIC_ORIGINS`; production adds nothing.
pub const VITE_DEV_ORIGINS: &[&str] = &["http://localhost:5173", "http://127.0.0.1:5173"];

/// The refused-preflight line goes to the tracing log, always.
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
    vec![
        ("access-control-allow-origin", canonical_origin(origin)),
        ("vary", "Origin".to_string()),
    ]
}

fn set_header(response: &mut Response, name: &'static str, value: &str) {
    if let Ok(value) = HeaderValue::from_str(value) {
        response.headers_mut().insert(name, value);
    }
}

fn empty(status: u16) -> Response {
    let mut response = Response::new(Body::empty());
    *response.status_mut() =
        axum::http::StatusCode::from_u16(status).unwrap_or(axum::http::StatusCode::NO_CONTENT);
    response
}

/// Applied to one request: preflights are answered here and never reach the router (which would
/// answer 404); every other request runs `handler` and has its CORS headers set on the way out.
/// The peer address the router reads for R190 passes through unchanged.
pub async fn with_cors<F, Fut>(options: &CorsOptions, request: Request, handler: F) -> Response
where
    F: FnOnce(Request) -> Fut,
    Fut: Future<Output = Response>,
{
    let origins: Vec<String> = options
        .origins
        .iter()
        .map(|origin| canonical_origin(origin))
        .filter(|origin| !origin.is_empty())
        .collect();
    let origin: Option<String> = request
        .headers()
        .get("origin")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    let allowed = is_origin_allowed(&origins, origin.as_deref());

    if request.method() == Method::OPTIONS && request.headers().contains_key("access-control-request-method")
    {
        if !allowed {
            tracing::warn!(
                event = "cors.preflight_refused",
                origin = %origin.as_deref().unwrap_or("(none)"),
                allowed = %serde_json::to_string(&origins).unwrap_or_default(),
            );
            // No CORS headers: the browser refuses it. `Vary: Origin` still goes on, because R162 puts it on
            // EVERY response this layer touches: a shared cache must not serve this answer to an origin that
            // would have been allowed.
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
        set_header(
            &mut response,
            "access-control-max-age",
            &PREFLIGHT_MAX_AGE_SECONDS.to_string(),
        );
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
            // R162 again: the ordinary response an unlisted or origin-less caller gets also depended on the
            // origin; without `Vary` a shared cache could later hand it to an allowed origin.
            set_header(&mut response, "vary", "Origin");
        }
    }
    response
}
