//! #3826: the four socket upgrades (`/ws/match`, `/ws/watch`, `/ws/social`, `/ws/duos`) each keep a run
//! of their own in `api::http::RouteHealth`, under the method `WS`. An upgrade whose profile read keeps
//! failing is refused with `WS_CLOSE.internal` every time; once one path has failed
//! `ROUTE_FAILING_ALERT_COUNT` times in a row over `ROUTE_FAILING_ALERT_MS`, it raises
//! `api.route.failing` for itself, whichever the other three do. A socket let in recovers its path.

use serde_json::json;

use jackioh_server::actor::ws_server::{SOCIAL_PATH, WATCH_PATH, WS_CLOSE, WS_DUOS_PATH, WS_PATH};
use jackioh_server::api::http::SOCKET_METHOD;
use jackioh_server::config::{ROUTE_FAILING_ALERT_COUNT, ROUTE_FAILING_ALERT_MS};

use super::duos::{PROMPTLY, serve};
use crate::support::deps::{empty_test_app, record_logs};
use crate::support::store::{account, advance, fail_on, stop_failing};

#[tokio::test(start_paused = true)]
async fn each_socket_upgrade_alerts_on_its_own_when_its_profile_read_keeps_failing() {
    let logs = record_logs();
    let app = empty_test_app().await;
    let ana = account(&app, 1, "Ana").await;
    fail_on(&app, "profiles.getByUserId", "the profile cannot be read").await;
    let served = serve(&app);

    let paths = [WS_PATH, WATCH_PATH, SOCIAL_PATH, WS_DUOS_PATH];
    for (done, path) in paths.into_iter().enumerate() {
        // Enough failures, not enough time: nothing for this path, whatever the earlier ones raised.
        for _ in 0..ROUTE_FAILING_ALERT_COUNT {
            let client = served.connect(path, &ana.token).await;
            assert_eq!(
                client.wait_closed(PROMPTLY).await,
                Some(WS_CLOSE.internal),
                "{path}"
            );
        }
        assert_eq!(logs.named("api.route.failing").len(), done, "{path}");

        advance(ROUTE_FAILING_ALERT_MS).await;
        let client = served.connect(path, &ana.token).await;
        assert_eq!(
            client.wait_closed(PROMPTLY).await,
            Some(WS_CLOSE.internal),
            "{path}"
        );
        let alerts = logs.named("api.route.failing");
        assert_eq!(alerts.len(), done + 1, "{path}: {alerts:?}");
        let data = &alerts[done].data["data"];
        assert_eq!(data["pattern"], json!(path));
        assert_eq!(data["method"], json!(SOCKET_METHOD));
        assert_eq!(alerts[done].level, "alert");
    }
    assert_eq!(app.route_health.failing_routes(), paths.len());

    // A social socket let in ends its path's run, and no other's.
    stop_failing(&app).await;
    let client = served.connect(SOCIAL_PATH, &ana.token).await;
    client
        .wait_for(0, PROMPTLY, |frame| frame["type"] == "social")
        .await
        .unwrap_or_else(|| panic!("no social frame: {:?}", client.frames()));
    let recovered = logs.named("api.route.recovered");
    assert_eq!(recovered.len(), 1, "{recovered:?}");
    assert_eq!(recovered[0].data["data"]["pattern"], json!(SOCIAL_PATH));
    assert_eq!(recovered[0].level, "info");
    assert_eq!(app.route_health.failing_routes(), paths.len() - 1);
    assert_eq!(
        logs.named("api.route.failing").len(),
        paths.len(),
        "one alert per path, no more"
    );
}
