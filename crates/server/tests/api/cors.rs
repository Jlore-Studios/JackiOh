//! Port of `apps/server/test/api/cors.test.ts`: SPEC §11 R162 — the CORS contract for the REST
//! surface (`src/api/cors.rs`).
//!
//! R162 fixes four things, and every one of them is a *negative* that a broken layer would satisfy
//! by accident:
//!
//!  - an allowed origin is echoed back **as itself**, never `*`;
//!  - `Access-Control-Allow-Credentials` is never sent, because §9.1's client carries a bearer
//!    token in a header and a credentialed response would let every allowed origin act as the
//!    player;
//!  - `Vary: Origin`, because the answer depends on the request's origin;
//!  - an unlisted origin gets **the ordinary response with no CORS headers**, not a 403 — CORS is
//!    never load-bearing for a refusal, since §9.1 puts the rules in the server and a non-browser
//!    caller (`cy.request`, the `wsPlayer` task) is not subject to CORS at all.
//!
//! The last one is the reason this file asserts the live case first, every time. "No CORS headers
//! for an unlisted origin" is also what a middleware that never ran produces, and "not a 403" is
//! what a handler that answered 500 produces — so each negative here is paired with the allowed
//! request that proves the layer is awake, and with a count of how many times the wrapped handler
//! was actually entered.
//!
//! Deterministic by construction: `with_cors` wraps a router with no clock, no store and no network,
//! so nothing here needs a timer. TS's `withCors(handler, { origins, log })` wrapped any
//! `Request -> Response` function; here it wraps an `axum::Router` (the one `app::router` hands it,
//! or the counting one below), and its `log` is `tracing`'s (SURFACE §11.3).

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use axum::body::Body;
use axum::http::{HeaderMap, Request, header};
use indexmap::IndexMap;
use tower::ServiceExt;

use jackioh_server::api::cors::{CorsOptions, VITE_DEV_ORIGINS, is_origin_allowed, with_cors};
use jackioh_server::app::browser_origins;
use jackioh_server::env::{Env, load_env};

/// A complete environment, so `browser_origins` is called with the shape it is declared for.
fn test_env(public_origins: &str, e2e: bool) -> Env {
    let mut source: IndexMap<String, String> = IndexMap::new();
    for (name, value) in [
        ("SUPABASE_URL", "https://project.supabase.test"),
        ("SUPABASE_SECRET_KEY", "secret"),
        ("DATABASE_URL", "postgres://localhost/jackioh"),
        (
            "SUPABASE_JWKS_URL",
            "https://project.supabase.test/auth/v1/.well-known/jwks.json",
        ),
        ("CODE_PEPPER", "pepper-pepper-pepper-pepper-pepper-pepper"),
        ("PORT", "8787"),
        ("PUBLIC_ORIGINS", public_origins),
        ("NODE_ENV", "test"),
        ("E2E", if e2e { "1" } else { "0" }),
    ] {
        source.insert(name.to_string(), value.to_string());
    }
    source.insert(
        "CATALOG_VERSION".to_string(),
        jackioh_cards::catalog_version().to_string(),
    );
    match load_env(&source) {
        Ok(env) => env,
        Err(problems) => panic!("the test environment loads: {problems}"),
    }
}

/// Two allowed origins, so "echoes the one that asked" can be told from "echoes the first".
const APP: &str = "http://localhost:5173";
const OTHER: &str = "https://play.jackioh.test";
const ALLOWED: [&str; 2] = [APP, OTHER];

/// Never in the list.
const STRANGER: &str = "https://evil.example";

const PATH: &str = "/api/queue/population";
const BODY: &str = r#"{"population":3}"#;

/// One response, read whole.
struct Reply {
    status: u16,
    headers: HeaderMap,
    text: String,
}

impl Reply {
    fn header(&self, name: &str) -> Option<String> {
        self.headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
    }
}

/// What a test sends: an `Origin` or none, a method, and whether it is a preflight.
#[derive(Default)]
struct Init<'a> {
    origin: Option<&'a str>,
    method: Option<&'a str>,
    preflight: bool,
}

struct Wrapped {
    /// How many times the wrapped handler was entered — the premise every negative leans on.
    calls: Arc<AtomicUsize>,
    service: axum::Router,
}

impl Wrapped {
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }

    async fn send(&self, init: Init<'_>) -> Reply {
        let method = if init.preflight {
            "OPTIONS"
        } else {
            init.method.unwrap_or("GET")
        };
        let mut builder = Request::builder().method(method).uri(PATH);
        if let Some(origin) = init.origin {
            builder = builder.header("origin", origin);
        }
        if init.preflight {
            builder = builder.header("access-control-request-method", "POST");
        }
        let request = builder.body(Body::empty()).expect("a well-formed request");
        let response = self
            .service
            .clone()
            .oneshot(request)
            .await
            .expect("the router always answers");
        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("the body reads");
        Reply {
            status,
            headers,
            text: String::from_utf8(bytes.to_vec()).expect("the body is UTF-8"),
        }
    }
}

/// `with_cors` around a handler that records every entry and answers the same 200 every time, so a
/// difference in any response below comes from the CORS layer and from nothing else.
fn wrapped(origins: &[&str]) -> Wrapped {
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = calls.clone();
    let handler = axum::Router::new().fallback(move || {
        let seen = seen.clone();
        async move {
            seen.fetch_add(1, Ordering::SeqCst);
            ([(header::CONTENT_TYPE, "application/json")], BODY)
        }
    });
    let options = Arc::new(CorsOptions {
        origins: origins.iter().map(|origin| origin.to_string()).collect(),
    });
    let service = axum::Router::new().fallback(move |request: axum::extract::Request| {
        let (options, handler) = (options.clone(), handler.clone());
        async move {
            with_cors(&options, request, |request| async move {
                handler
                    .oneshot(request)
                    .await
                    .unwrap_or_else(|never| match never {})
            })
            .await
        }
    });
    Wrapped { calls, service }
}

// ---------------------------------------------------------------------------
// R162
// ---------------------------------------------------------------------------

mod r162_the_cors_contract_for_the_rest_surface_section_9_1_section_9_2_section_9_8 {
    use super::*;

    #[tokio::test]
    async fn r162_echoes_the_one_origin_that_asked_never_star_and_never_allows_credentials() {
        let layer = wrapped(&ALLOWED);

        let first = layer
            .send(Init {
                origin: Some(APP),
                ..Init::default()
            })
            .await;
        let second = layer
            .send(Init {
                origin: Some(OTHER),
                ..Init::default()
            })
            .await;

        // The premise: the layer is awake and the handler's own answer came through untouched.
        assert_eq!(layer.calls(), 2);
        assert_eq!(first.status, 200);
        assert_eq!(first.text, BODY);

        // Echoed *as itself*: each request gets its own origin back, not the first in the list.
        assert_eq!(first.header("access-control-allow-origin").as_deref(), Some(APP));
        assert_eq!(
            second.header("access-control-allow-origin").as_deref(),
            Some(OTHER)
        );
        // Never the wildcard, whichever origin asked.
        for response in [&first, &second] {
            assert_ne!(
                response.header("access-control-allow-origin").as_deref(),
                Some("*")
            );
            // §9.1's bearer token is in a header, so no cookie is ever in play.
            assert_eq!(response.header("access-control-allow-credentials"), None);
            // The answer depends on the request's origin: a shared cache must key on it.
            assert_eq!(response.header("vary").as_deref(), Some("Origin"));
        }
    }

    #[tokio::test]
    async fn r162_answers_an_allowed_preflight_itself_with_the_same_echo_and_no_credentials() {
        let layer = wrapped(&ALLOWED);

        let preflight = layer
            .send(Init {
                origin: Some(APP),
                preflight: true,
                ..Init::default()
            })
            .await;

        // The preflight never reaches the router, which knows no OPTIONS route and would 404 it.
        assert_eq!(layer.calls(), 0);
        assert_eq!(preflight.status, 204);
        assert_eq!(
            preflight.header("access-control-allow-origin").as_deref(),
            Some(APP)
        );
        assert_ne!(
            preflight.header("access-control-allow-origin").as_deref(),
            Some("*")
        );
        assert_eq!(preflight.header("access-control-allow-credentials"), None);
        assert_eq!(preflight.header("vary").as_deref(), Some("Origin"));
        // The methods the router allows and the two headers `apps/web/src/net/api.ts` sends.
        assert!(
            preflight
                .header("access-control-allow-methods")
                .unwrap_or_default()
                .contains("POST")
        );
        assert_eq!(
            preflight.header("access-control-allow-headers").as_deref(),
            Some("authorization, content-type")
        );
    }

    #[tokio::test]
    async fn r162_gives_an_unlisted_origin_the_ordinary_response_with_no_cors_headers_not_a_403() {
        let layer = wrapped(&ALLOWED);

        // PREMISE FIRST. Without this the assertions below would pass against a layer that never ran,
        // a handler that always 403s, or an `origins` list this test misspelled.
        let allowed = layer
            .send(Init {
                origin: Some(APP),
                ..Init::default()
            })
            .await;
        assert_eq!(allowed.status, 200);
        assert_eq!(
            allowed.header("access-control-allow-origin").as_deref(),
            Some(APP)
        );

        let unlisted = layer
            .send(Init {
                origin: Some(STRANGER),
                ..Init::default()
            })
            .await;

        // The handler ran for the stranger too — the request was not refused by the CORS layer.
        assert_eq!(layer.calls(), 2);
        // The *ordinary* response: byte for byte what the allowed origin got, minus the headers.
        assert_eq!(unlisted.status, 200);
        assert_ne!(unlisted.status, 403);
        assert_eq!(unlisted.text, BODY);
        assert_eq!(
            unlisted.header("content-type").as_deref(),
            Some("application/json")
        );
        // …and no CORS headers at all, which is what the browser needs in order to refuse it.
        assert_eq!(unlisted.header("access-control-allow-origin"), None);
        assert_eq!(unlisted.header("access-control-allow-credentials"), None);
    }

    #[tokio::test]
    async fn r162_refuses_an_unlisted_preflight_by_silence_rather_than_by_a_status() {
        let layer = wrapped(&ALLOWED);

        // PREMISE: the allowed preflight really is decorated, so "no headers" below means something.
        let allowed = layer
            .send(Init {
                origin: Some(APP),
                preflight: true,
                ..Init::default()
            })
            .await;
        assert_eq!(
            allowed.header("access-control-allow-origin").as_deref(),
            Some(APP)
        );

        let unlisted = layer
            .send(Init {
                origin: Some(STRANGER),
                preflight: true,
                ..Init::default()
            })
            .await;

        assert_eq!(unlisted.status, 204);
        assert_ne!(unlisted.status, 403);
        assert_eq!(unlisted.header("access-control-allow-origin"), None);
        assert_eq!(unlisted.header("access-control-allow-methods"), None);
        // Neither preflight reached the router.
        assert_eq!(layer.calls(), 0);
    }

    #[tokio::test]
    async fn r162_leaves_a_caller_with_no_origin_at_all_completely_alone_section_9_1_not_a_browser() {
        let layer = wrapped(&ALLOWED);

        // PREMISE: the same request *with* an allowed origin is decorated.
        let decorated = layer
            .send(Init {
                origin: Some(APP),
                ..Init::default()
            })
            .await;
        assert_eq!(
            decorated.header("access-control-allow-origin").as_deref(),
            Some(APP)
        );

        let curl = layer.send(Init::default()).await;

        assert_eq!(layer.calls(), 2);
        assert_eq!(curl.status, 200);
        assert_eq!(curl.text, BODY);
        assert_eq!(curl.header("access-control-allow-origin"), None);
        // `cy.request` and the e2e `wsPlayer` task are not subject to CORS, so the server's own rules
        // must be the only thing that ever refuses them.
        assert_eq!(curl.header("access-control-allow-credentials"), None);
    }

    #[test]
    fn r162_reads_one_list_for_both_doors_so_cors_and_the_websocket_origin_check_cannot_diverge() {
        // `app.rs` builds the list once and hands the same list to `with_cors` and to the WebSocket
        // upgrade's origin check; `is_origin_allowed` is the predicate the CORS half applies to it.
        let origins = browser_origins(&test_env(OTHER, true));
        assert!(origins.iter().any(|origin| origin == OTHER));
        for dev in VITE_DEV_ORIGINS {
            assert!(origins.iter().any(|origin| origin == dev), "{dev}");
        }

        assert!(is_origin_allowed(&origins, Some(OTHER)));
        assert!(is_origin_allowed(&origins, Some(APP)));
        assert!(!is_origin_allowed(&origins, Some(STRANGER)));
        // An absent origin is not "allowed"; it is simply not a browser (see the test above).
        assert!(!is_origin_allowed(&origins, None));

        // A hand-written PUBLIC_ORIGINS may carry a trailing slash; a browser never sends one.
        assert!(is_origin_allowed(&[format!("{OTHER}/")], Some(OTHER)));
    }

    // R162's remaining clause, which was a real gap when this file was written and is now closed:
    // "every response the layer touches carries `Vary: Origin`". The CORS headers go on only on the
    // two ALLOWED paths (asserted above), so the refused preflight and the ordinary response to an
    // unlisted or absent origin used to come back bare — and a shared cache could then store the
    // no-CORS answer and hand it to an origin that would have been allowed, which is the exact
    // failure the clause names.
    #[tokio::test]
    async fn r162_carries_vary_origin_on_every_response_the_layer_touches_refusals_included() {
        let layer = wrapped(&ALLOWED);

        // Premise: the allowed path really does set it, so a blanket failure cannot pass this test.
        let allowed = layer
            .send(Init {
                origin: Some(APP),
                ..Init::default()
            })
            .await;
        assert_eq!(allowed.header("vary").as_deref(), Some("Origin"));

        // A refused preflight: no CORS headers, but still an origin-dependent answer, so still varied.
        let refused = layer
            .send(Init {
                origin: Some(STRANGER),
                preflight: true,
                ..Init::default()
            })
            .await;
        assert_eq!(refused.status, 204);
        assert_eq!(refused.header("access-control-allow-origin"), None);
        assert_eq!(refused.header("vary").as_deref(), Some("Origin"));

        // An unlisted origin gets the ordinary answer — body intact, no CORS headers, still varied.
        let unlisted = layer
            .send(Init {
                origin: Some(STRANGER),
                ..Init::default()
            })
            .await;
        assert_eq!(unlisted.status, 200);
        assert_eq!(unlisted.text, BODY);
        assert_eq!(unlisted.header("access-control-allow-origin"), None);
        assert_eq!(unlisted.header("vary").as_deref(), Some("Origin"));

        // And a caller with no Origin at all, which is every non-browser client.
        let anonymous = layer.send(Init::default()).await;
        assert_eq!(anonymous.status, 200);
        assert_eq!(anonymous.header("vary").as_deref(), Some("Origin"));
    }
}
