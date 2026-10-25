//! The daily super run on `/stats`'s Daily runs tab (SPEC §9.11, R1935000): `GET /api/stats/ci`
//! over a test app that answers in GitHub's place (`CiRuns::canned`), with GitHub's own body shapes.

use std::sync::Arc;
use std::time::Duration;

use jackioh_server::app::App;
use jackioh_server::config::{CI_RUNS_CACHE_TTL_SECONDS, CI_RUNS_LISTED};
use serde_json::{Value, json};

use crate::support::deps::{call, test_app};

const PATH: &str = "/api/stats/ci";

/// How long the server keeps what GitHub answered, in milliseconds.
fn ttl_ms() -> u64 {
    u64::try_from(CI_RUNS_CACHE_TTL_SECONDS).expect("the TTL is not negative") * 1000
}

async fn get(app: &Arc<App>) -> (u16, Option<String>, Value) {
    let (status, headers, body) = call(app, "GET", PATH, None, Value::Null).await;
    let cache = headers
        .get("cache-control")
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    (status, cache, body)
}

/// A run as GitHub's `GET /repos/{repo}/actions/workflows/{file}/runs` lists it.
fn run(id: u64, attempt: u64, status: &str, conclusion: Option<&str>) -> Value {
    json!({
        "id": id,
        "run_attempt": attempt,
        "status": status,
        "conclusion": conclusion,
        "head_sha": format!("sha{id}"),
        "run_started_at": format!("2026-10-2{}T07:17:00Z", id % 10),
        "html_url": format!("https://github.com/o/r/actions/runs/{id}"),
        "event": "schedule",
    })
}

fn runs(listed: Vec<Value>) -> Value {
    json!({ "total_count": listed.len(), "workflow_runs": listed })
}

/// A job as GitHub's `GET /repos/{repo}/actions/runs/{id}/attempts/{n}/jobs` lists it.
fn job(name: &str, status: &str, conclusion: Option<&str>) -> Value {
    json!({
        "name": name,
        "status": status,
        "conclusion": conclusion,
        "html_url": format!("https://github.com/o/r/job/{name}"),
    })
}

mod r1935000_daily_runs {
    use super::*;

    #[tokio::test]
    async fn r1935000_the_runs_newest_first_with_a_failed_runs_jobs_that_did_not_pass() {
        let app = test_app().await;
        app.ci_runs.answer_runs(runs(vec![
            run(3, 1, "in_progress", None),
            run(2, 2, "completed", Some("failure")),
            run(1, 1, "completed", Some("success")),
        ]));
        app.ci_runs.answer_jobs(
            2,
            2,
            json!({ "total_count": 4, "jobs": [
                job("fuzz (1/4)", "completed", Some("failure")),
                job("golden", "completed", Some("success")),
                job("fuzz (4/4)", "completed", Some("cancelled")),
                job("notebooks", "completed", Some("skipped")),
            ] }),
        );

        let (status, cache, data) = get(&app).await;
        assert_eq!(status, 200);
        assert_eq!(
            cache.as_deref(),
            Some(format!("public, max-age={CI_RUNS_CACHE_TTL_SECONDS}").as_str())
        );
        assert_eq!(
            data,
            json!({ "runs": [
                {
                    "id": 3,
                    "url": "https://github.com/o/r/actions/runs/3",
                    "startedAt": "2026-10-23T07:17:00Z",
                    "commit": "sha3",
                    "result": "running",
                    "failedJobs": [],
                },
                {
                    "id": 2,
                    "url": "https://github.com/o/r/actions/runs/2",
                    "startedAt": "2026-10-22T07:17:00Z",
                    "commit": "sha2",
                    "result": "failed",
                    "failedJobs": [
                        {
                            "name": "fuzz (1/4)",
                            "url": "https://github.com/o/r/job/fuzz (1/4)",
                            "conclusion": "failure",
                        },
                        {
                            "name": "fuzz (4/4)",
                            "url": "https://github.com/o/r/job/fuzz (4/4)",
                            "conclusion": "cancelled",
                        },
                    ],
                },
                {
                    "id": 1,
                    "url": "https://github.com/o/r/actions/runs/1",
                    "startedAt": "2026-10-21T07:17:00Z",
                    "commit": "sha1",
                    "result": "passed",
                    "failedJobs": [],
                },
            ] })
        );
        // Only the failed run's jobs were read.
        assert_eq!(app.ci_runs.reads(), (1, 1));
    }

    #[tokio::test]
    async fn r1935000_a_run_that_did_not_succeed_is_failed_and_no_more_than_ci_runs_listed_are_shown() {
        let app = test_app().await;
        let extra = 3;
        let listed = (0..CI_RUNS_LISTED + extra)
            .map(|i| run(100 + i as u64, 1, "completed", Some("cancelled")))
            .collect();
        app.ci_runs.answer_runs(runs(listed));
        for i in 0..CI_RUNS_LISTED + extra {
            app.ci_runs.answer_jobs(100 + i as u64, 1, json!({ "jobs": [] }));
        }

        let (status, _, data) = get(&app).await;
        assert_eq!(status, 200);
        let shown = data["runs"].as_array().expect("runs is a list");
        assert_eq!(shown.len(), CI_RUNS_LISTED);
        assert!(shown.iter().all(|run| run["result"] == json!("failed")));
        assert_eq!(app.ci_runs.reads(), (1, CI_RUNS_LISTED));
    }

    #[tokio::test(start_paused = true)]
    async fn r1935000_what_github_answered_is_kept_for_the_ttl_and_a_failed_runs_jobs_are_read_once() {
        let app = test_app().await;
        app.ci_runs
            .answer_runs(runs(vec![run(2, 1, "completed", Some("failure"))]));
        app.ci_runs.answer_jobs(
            2,
            1,
            json!({ "jobs": [job("e2e (4/8)", "completed", Some("failure"))] }),
        );

        let (_, _, first) = get(&app).await;
        // A new run GitHub would now list is not shown until the kept answer expires.
        app.ci_runs.answer_runs(runs(vec![
            run(3, 1, "in_progress", None),
            run(2, 1, "completed", Some("failure")),
        ]));
        tokio::time::advance(Duration::from_millis(ttl_ms() - 1)).await;
        let (_, _, within) = get(&app).await;
        assert_eq!(within, first);
        assert_eq!(app.ci_runs.reads(), (1, 1));

        tokio::time::advance(Duration::from_millis(1)).await;
        let (_, _, after) = get(&app).await;
        assert_eq!(after["runs"][0]["result"], json!("running"));
        assert_eq!(after["runs"][1], first["runs"][0]);
        // The runs were read again; the finished run's jobs were not.
        assert_eq!(app.ci_runs.reads(), (2, 1));
    }

    #[tokio::test]
    async fn r1935000_github_that_cannot_be_read_answers_unavailable_and_is_kept_for_no_one() {
        let app = test_app().await;

        let (status, _, body) = get(&app).await;
        assert_eq!(status, 503);
        assert_eq!(body["error"]["code"], json!("unavailable"));

        // A failed run whose jobs cannot be read fails the read too.
        app.ci_runs
            .answer_runs(runs(vec![run(5, 1, "completed", Some("failure"))]));
        let (status, _, _) = get(&app).await;
        assert_eq!(status, 503);

        // Nothing was kept: the next request reads again, at once.
        app.ci_runs.answer_jobs(5, 1, json!({ "jobs": [] }));
        let (status, _, data) = get(&app).await;
        assert_eq!(status, 200);
        assert_eq!(data["runs"][0]["result"], json!("failed"));
        assert_eq!(app.ci_runs.reads(), (3, 2));
    }
}
