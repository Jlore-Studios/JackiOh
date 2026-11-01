//! The daily super run's last runs (SPEC §9.11, R1935000), which `/stats`'s Daily runs tab listed until
//! R1973000 took the tab off the page; the route stays for whatever calls it (CLAUDE.md rule 13).
//!
//! `GET /api/stats/ci` is open: the last `CI_RUNS_LISTED` runs of `CI_RUNS_WORKFLOW` on
//! `CI_RUNS_BRANCH`, newest first, read from GitHub's Actions API, each with when it started, its
//! commit, whether it passed, failed or is still running and, for a finished run that did not pass,
//! the jobs that did not pass, as the workflow's own report lists them. What GitHub answered is kept
//! `CI_RUNS_CACHE_TTL_SECONDS` however many ask, as R1583000 keeps the play rates: requests that miss
//! together wait for one read, and a read that fails is kept for no one and answers `unavailable`. A
//! finished run attempt's jobs never change, so they are read once and kept while the run is listed.
//! The repository is public, so a deployment reads it with `CREATE_ISSUES_TOKEN` when that is set,
//! for GitHub's larger allowance, and without a token otherwise; the end-to-end server reads nothing
//! and lists no run.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use indexmap::IndexMap;
use serde::Serialize;
use serde_json::{Value, json};
use tokio::sync::OnceCell;

use crate::api::http::{ApiError, ApiErrorCode, ApiResult, Req, lock, now_ms, to_json};
use crate::api::stats::cached_ok;
use crate::app::App;
use crate::config::{
    CI_RUNS_API_URL, CI_RUNS_BRANCH, CI_RUNS_CACHE_TTL_SECONDS, CI_RUNS_JOBS_PAGE, CI_RUNS_LISTED,
    CI_RUNS_TIMEOUT_MS, CI_RUNS_WORKFLOW, MS_PER_SECOND,
};
use crate::env::Env;

/// GitHub's conclusions of a job that did not fail: the workflow's report leaves these out.
const JOB_PASSED: [&str; 3] = ["success", "skipped", "neutral"];

/// One run as the tab shows it.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CiRun {
    pub id: u64,
    /// The run on GitHub.
    pub url: String,
    /// When it started, as GitHub writes it (ISO 8601, UTC).
    pub started_at: String,
    /// The commit it ran on.
    pub commit: String,
    /// `running` until it finishes, then `passed` when GitHub concluded `success` and `failed` for
    /// any other conclusion, as the workflow's report counts a run red.
    pub result: &'static str,
    /// A failed run's jobs that did not pass, in GitHub's order; none for any other run.
    pub failed_jobs: Vec<CiJob>,
}

/// A job that did not pass.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CiJob {
    pub name: String,
    /// The job's log on GitHub.
    pub url: String,
    /// GitHub's conclusion: `failure`, `cancelled`, `timed_out` and so on.
    pub conclusion: String,
}

/// `GET /api/stats/ci`'s body.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct CiRunsResponse {
    pub runs: Vec<CiRun>,
}

/// What a test's app answers in GitHub's place, and how often each was read.
#[derive(Default)]
struct Canned {
    runs: Option<Value>,
    jobs: IndexMap<(u64, u64), Value>,
    runs_read: usize,
    jobs_read: usize,
}

/// Where the runs are read from.
enum Source {
    /// The end-to-end server: nothing is read, and no run is listed.
    Off,
    /// A deployment: GitHub, in `repo`.
    GitHub {
        http: reqwest::Client,
        token: Option<String>,
        repo: String,
    },
    /// A test's app: what it was given (`answer_runs`, `answer_jobs`).
    Canned(Mutex<Canned>),
}

/// The kept answer, filled by the first request that asks for it; the others wait on it.
type KeptCell = Arc<OnceCell<Arc<CiRunsResponse>>>;

/// The app's daily runs (`App::ci_runs`).
pub struct CiRuns {
    source: Source,
    /// The kept answer, with when it was first asked for.
    kept: Mutex<Option<(i64, KeptCell)>>,
    /// Each listed failed run attempt's failed jobs, by run id and attempt.
    jobs: Mutex<IndexMap<(u64, u64), Vec<CiJob>>>,
}

impl CiRuns {
    fn with(source: Source) -> CiRuns {
        CiRuns {
            source,
            kept: Mutex::new(None),
            jobs: Mutex::new(IndexMap::new()),
        }
    }

    /// A deployment's: GitHub, unless the server is the end-to-end one.
    pub fn from_env(env: &Env) -> CiRuns {
        if env.e2e {
            return CiRuns::with(Source::Off);
        }
        CiRuns::with(Source::GitHub {
            http: reqwest::Client::new(),
            token: env.github_issues_token.clone(),
            repo: env.github_issues_repo.clone(),
        })
    }

    /// Reads nothing and lists no run.
    pub fn off() -> CiRuns {
        CiRuns::with(Source::Off)
    }

    /// A test's: answers what it is given, and nothing (a failed read) until then.
    pub fn canned() -> CiRuns {
        CiRuns::with(Source::Canned(Mutex::new(Canned::default())))
    }

    /// A canned app's answer to the runs' read: GitHub's body for the workflow's runs.
    pub fn answer_runs(&self, body: Value) {
        if let Source::Canned(canned) = &self.source {
            lock(canned).runs = Some(body);
        }
    }

    /// A canned app's answer to the read of run `id`'s attempt `attempt`'s jobs: GitHub's body.
    pub fn answer_jobs(&self, id: u64, attempt: u64, body: Value) {
        if let Source::Canned(canned) = &self.source {
            lock(canned).jobs.insert((id, attempt), body);
        }
    }

    /// How often a canned app's runs and jobs were read; (0, 0) for any other.
    pub fn reads(&self) -> (usize, usize) {
        match &self.source {
            Source::Canned(canned) => {
                let canned = lock(canned);
                (canned.runs_read, canned.jobs_read)
            }
            _ => (0, 0),
        }
    }

    /// The kept answer's cell made less than `CI_RUNS_CACHE_TTL_SECONDS` before `now`, or a new
    /// empty one in its place.
    fn cell(&self, now: i64) -> KeptCell {
        let mut kept = lock(&self.kept);
        if let Some((made_at, cell)) = kept.as_ref()
            && now - *made_at < CI_RUNS_CACHE_TTL_SECONDS * MS_PER_SECOND
        {
            return Arc::clone(cell);
        }
        let cell: KeptCell = Arc::new(OnceCell::new());
        *kept = Some((now, Arc::clone(&cell)));
        cell
    }

    /// GitHub's body for the workflow's last runs on the branch.
    async fn read_runs(&self) -> Result<Value, String> {
        match &self.source {
            Source::Off => Ok(json!({ "workflow_runs": [] })),
            Source::GitHub { http, token, repo } => {
                let path = format!(
                    "/repos/{repo}/actions/workflows/{CI_RUNS_WORKFLOW}/runs\
                     ?branch={CI_RUNS_BRANCH}&per_page={CI_RUNS_LISTED}&exclude_pull_requests=true"
                );
                github_get(http, token.as_deref(), &path).await
            }
            Source::Canned(canned) => {
                let mut canned = lock(canned);
                canned.runs_read += 1;
                canned.runs.clone().ok_or_else(|| "no runs answered".to_string())
            }
        }
    }

    /// GitHub's body for run `id`'s attempt `attempt`'s jobs.
    async fn read_jobs(&self, id: u64, attempt: u64) -> Result<Value, String> {
        match &self.source {
            Source::Off => Ok(json!({ "jobs": [] })),
            Source::GitHub { http, token, repo } => {
                // maxi-tail: one page, GitHub's largest; super.yml runs well under 100 jobs. Past
                // that, the later pages' failed jobs go unlisted until this follows `Link: next`.
                let path = format!(
                    "/repos/{repo}/actions/runs/{id}/attempts/{attempt}/jobs?per_page={CI_RUNS_JOBS_PAGE}"
                );
                github_get(http, token.as_deref(), &path).await
            }
            Source::Canned(canned) => {
                let mut canned = lock(canned);
                canned.jobs_read += 1;
                canned
                    .jobs
                    .get(&(id, attempt))
                    .cloned()
                    .ok_or_else(|| format!("no jobs answered for run {id} attempt {attempt}"))
            }
        }
    }

    /// The runs as the tab shows them, read afresh but for the jobs of a failed run attempt kept
    /// from an earlier read. Only the listed runs' jobs stay kept.
    async fn load(&self) -> Result<CiRunsResponse, String> {
        let body = self.read_runs().await?;
        let listed = body["workflow_runs"]
            .as_array()
            .ok_or_else(|| "GitHub answered no workflow_runs".to_string())?;
        let mut runs = Vec::new();
        let mut kept = IndexMap::new();
        for run in listed.iter().take(CI_RUNS_LISTED) {
            let id = run["id"]
                .as_u64()
                .ok_or_else(|| "GitHub answered a run with no id".to_string())?;
            let result = match (run["status"].as_str(), run["conclusion"].as_str()) {
                (Some("completed"), Some("success")) => "passed",
                (Some("completed"), _) => "failed",
                _ => "running",
            };
            let failed_jobs = if result == "failed" {
                let attempt = run["run_attempt"].as_u64().unwrap_or(1);
                let known = lock(&self.jobs).get(&(id, attempt)).cloned();
                let jobs = match known {
                    Some(jobs) => jobs,
                    None => failed_jobs_of(&self.read_jobs(id, attempt).await?),
                };
                kept.insert((id, attempt), jobs.clone());
                jobs
            } else {
                Vec::new()
            };
            runs.push(CiRun {
                id,
                url: text(run, "html_url"),
                started_at: text(run, "run_started_at"),
                commit: text(run, "head_sha"),
                result,
                failed_jobs,
            });
        }
        *lock(&self.jobs) = kept;
        Ok(CiRunsResponse { runs })
    }
}

/// A field of GitHub's as text, or empty when it is missing.
fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or_default().to_string()
}

/// The finished jobs in GitHub's body that did not pass, in its order.
fn failed_jobs_of(body: &Value) -> Vec<CiJob> {
    body["jobs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|job| job["status"].as_str() == Some("completed"))
        .filter_map(|job| {
            let conclusion = job["conclusion"].as_str()?;
            (!JOB_PASSED.contains(&conclusion)).then(|| CiJob {
                name: text(job, "name"),
                url: text(job, "html_url"),
                conclusion: conclusion.to_string(),
            })
        })
        .collect()
}

/// A read of GitHub's REST API, with the headers it asks for and the token when there is one.
async fn github_get(http: &reqwest::Client, token: Option<&str>, path: &str) -> Result<Value, String> {
    let mut request = http
        .get(format!("{CI_RUNS_API_URL}{path}"))
        .header("accept", "application/vnd.github+json")
        .header("user-agent", "jackioh-server")
        .header("x-github-api-version", "2022-11-28")
        .timeout(Duration::from_millis(CI_RUNS_TIMEOUT_MS));
    if let Some(token) = token {
        request = request.bearer_auth(token);
    }
    request
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())
}

/// GET /api/stats/ci (R1935000): read from GitHub once per `CI_RUNS_CACHE_TTL_SECONDS` however many
/// ask (a failed read is kept for no one), cacheable as long.
pub async fn get_ci_runs(app: &Arc<App>, _req: Req) -> ApiResult {
    let cell = app.ci_runs.cell(now_ms());
    let runs = cell
        .get_or_try_init(|| async {
            app.ci_runs.load().await.map(Arc::new).map_err(|message| {
                tracing::warn!(event = "stats.ci.unreadable", message = %message);
                ApiError::new(
                    ApiErrorCode::Unavailable,
                    "The daily runs could not be read from GitHub.",
                )
            })
        })
        .await?;
    Ok(cached_ok(to_json(runs.as_ref())?, CI_RUNS_CACHE_TTL_SECONDS))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_end_to_end_server_reads_nothing_and_lists_no_run() {
        assert_eq!(
            CiRuns::off().load().await,
            Ok(CiRunsResponse { runs: Vec::new() })
        );
    }

    #[test]
    fn a_job_that_succeeded_was_skipped_or_is_still_running_is_not_listed() {
        let body = json!({ "jobs": [
            { "name": "a", "html_url": "u/a", "status": "completed", "conclusion": "success" },
            { "name": "b", "html_url": "u/b", "status": "completed", "conclusion": "skipped" },
            { "name": "c", "html_url": "u/c", "status": "completed", "conclusion": "neutral" },
            { "name": "d", "html_url": "u/d", "status": "in_progress", "conclusion": null },
            { "name": "e", "html_url": "u/e", "status": "completed", "conclusion": "timed_out" },
        ] });
        assert_eq!(
            failed_jobs_of(&body),
            vec![CiJob {
                name: "e".to_string(),
                url: "u/e".to_string(),
                conclusion: "timed_out".to_string(),
            }]
        );
    }
}
