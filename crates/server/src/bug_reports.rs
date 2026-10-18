//! Bug reports. Not in SPEC, and no R-row: operational, as #861's probes are.
//!
//! When the server finds a match its own log does not reproduce (§9.3's determinism break), it keeps
//! the match going, as the rebuild always has ("a live match is better than a dead one",
//! `actor::registry::rebuild`), logs the finding (`bug.found`) and files one GitHub issue the night
//! bot builds (`BUG_REPORT_LABELS`: `bot:build`, bot/README.md). It looks in four places:
//!
//!  - a 1v1 rebuild whose fold refuses an action the log holds (`actor::registry`);
//!  - a co-op rebuild that does (`duos::registry`);
//!  - the reaper's recovery fold that does (`api::results`);
//!  - a finished match whose log does not fold to its end (`api::game_records`).
//!
//! What a report says. The repository is public and the match may still be live, so a report names
//! the match, where it was found, the build and the catalog, and each refused row's seq, seat and
//! action type, never a refusal's text, a card or an instance id, any of which could show a hand to
//! the opponent. A maintainer reads the rest from the database (crates/server/README.md, "A match its
//! log does not reproduce").
//!
//! How often. One report per match per process, at most `BUG_REPORTS_PER_WINDOW` in
//! `BUG_REPORT_WINDOW_MS`, and none when an issue whose title names the match exists already, open or
//! closed, so a restart files nothing twice. Every finding is logged whatever is filed. Without
//! `CREATE_ISSUES_TOKEN` nothing leaves the process, and a test's app records what it would file.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;

use indexmap::IndexSet;
use jackioh_engine::FoldError;
use serde_json::{Value, json};

use crate::api::http::{lock, log_alert, now_ms};
use crate::config::{
    BUG_REPORT_API_URL, BUG_REPORT_LABELS, BUG_REPORT_LISTED_ACTIONS, BUG_REPORT_TIMEOUT_MS,
    BUG_REPORT_WINDOW_MS, BUG_REPORTS_PER_WINDOW,
};
use crate::db::store::MatchActionRow;
use crate::env::Env;

/// Where a finding was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FoundIn {
    /// A 1v1 match's actor rebuilt from its log (§9.5).
    Rebuild,
    /// A co-op match's actor rebuilt from its log (§13).
    DuosRebuild,
    /// The reaper's fold of a match past its ceiling (R833000).
    Reaper,
    /// A finished match's log, folded for its game record (R376).
    FinishedLog,
}

impl FoundIn {
    fn title(self) -> &'static str {
        match self {
            FoundIn::Rebuild => "does not rebuild from its log",
            FoundIn::DuosRebuild => "(co-op) does not rebuild from its log",
            FoundIn::Reaper => "does not fold from its log for the reaper",
            FoundIn::FinishedLog => "does not fold from its log to its result",
        }
    }

    fn said(self) -> &'static str {
        match self {
            FoundIn::Rebuild => {
                "rebuilding the match's actor from its log (a restart, a deploy or an evicted actor); \
                 the match went on from the folded state"
            }
            FoundIn::DuosRebuild => {
                "rebuilding the co-op match's actor from its log; the match went on from the state \
                 before the refused action"
            }
            FoundIn::Reaper => "the reaper's recovery fold; the match ends as the ceiling's draw",
            FoundIn::FinishedLog => {
                "folding the finished match's log for its game record; the result stands as the \
                 match recorded it"
            }
        }
    }
}

/// One logged action the fold refused, as much of it as a public issue may show.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RefusedRow {
    pub seq: i64,
    pub seat: String,
    pub action: String,
}

impl RefusedRow {
    /// The row's seq, and its seat and type read off the logged action's JSON: a 1v1 action's
    /// `playerId` and `type`, a co-op action's `seat` and `body.type`.
    pub fn of(seq: i64, action: &Value) -> RefusedRow {
        let text = |value: &Value| value.as_str().unwrap_or("?").to_string();
        let seat = action.get("playerId").or_else(|| action.get("seat"));
        let kind = action
            .get("type")
            .or_else(|| action.get("body").and_then(|body| body.get("type")));
        RefusedRow {
            seq,
            seat: seat.map_or_else(|| "?".to_string(), text),
            action: kind.map_or_else(|| "?".to_string(), text),
        }
    }
}

/// The rows of a 1v1 log whose actions a fold refused, matched by nonce (a log holds each nonce once).
pub fn refused_rows(log: &[MatchActionRow], errors: &[FoldError]) -> Vec<RefusedRow> {
    log.iter()
        .filter(|row| errors.iter().any(|error| error.nonce == row.action.nonce))
        .map(|row| RefusedRow::of(row.seq, &serde_json::to_value(&row.action).unwrap_or(Value::Null)))
        .collect()
}

/// What was found: the match, where, its catalog, how many actions the log held and the ones the
/// fold refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub match_id: String,
    pub found_in: FoundIn,
    pub catalog_version: String,
    pub actions: usize,
    pub refused: Vec<RefusedRow>,
}

/// An issue as it is filed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BugReport {
    pub match_id: String,
    pub title: String,
    pub body: String,
    pub labels: Vec<String>,
}

/// Where a report goes.
enum Sink {
    /// No token: findings are only logged.
    Off,
    /// `CREATE_ISSUES_TOKEN` set: an issue in `repo`.
    GitHub {
        http: reqwest::Client,
        token: String,
        repo: String,
    },
    /// A test's app: what would be filed, kept to read back.
    Recording(Mutex<Vec<BugReport>>),
}

/// The matches already reported, and when each report of the window was filed.
#[derive(Default)]
struct Filed {
    matches: IndexSet<String>,
    at: VecDeque<i64>,
}

/// The app's reporter (`App::bug_reports`).
pub struct BugReports {
    sink: Sink,
    filed: Mutex<Filed>,
}

impl BugReports {
    fn with(sink: Sink) -> BugReports {
        BugReports {
            sink,
            filed: Mutex::new(Filed::default()),
        }
    }

    /// A deployment's: GitHub when the token is set and the server is not the end-to-end one.
    pub fn from_env(env: &Env) -> BugReports {
        match &env.github_issues_token {
            Some(token) if !env.e2e => BugReports::with(Sink::GitHub {
                http: reqwest::Client::new(),
                token: token.clone(),
                repo: env.github_issues_repo.clone(),
            }),
            _ => BugReports::with(Sink::Off),
        }
    }

    /// Logs only.
    pub fn off() -> BugReports {
        BugReports::with(Sink::Off)
    }

    /// A test's: records what it would file.
    pub fn recording() -> BugReports {
        BugReports::with(Sink::Recording(Mutex::new(Vec::new())))
    }

    /// What a recording reporter filed, oldest first; nothing for any other.
    pub fn recorded(&self) -> Vec<BugReport> {
        match &self.sink {
            Sink::Recording(filed) => lock(filed).clone(),
            _ => Vec::new(),
        }
    }

    /// Logs `finding` and, once per match and within the window's allowance, files it. Never
    /// waits on GitHub: the request runs on its own task, and its failure is logged.
    pub fn file(&self, deployed_commit: Option<&str>, finding: Finding) {
        let refused: Vec<i64> = finding.refused.iter().map(|row| row.seq).collect();
        log_alert(
            "bug.found",
            json!({
                "matchId": finding.match_id,
                "foundIn": format!("{:?}", finding.found_in),
                "actions": finding.actions,
                "refusedSeqs": refused,
            }),
        );
        if matches!(self.sink, Sink::Off) || !self.admit(&finding.match_id, now_ms()) {
            return;
        }
        let report = report_of(deployed_commit, &finding);
        match &self.sink {
            Sink::Off => {}
            Sink::Recording(filed) => lock(filed).push(report),
            Sink::GitHub { http, token, repo } => {
                let (http, token, repo) = (http.clone(), token.clone(), repo.clone());
                tokio::spawn(async move {
                    match post(&http, &token, &repo, &report).await {
                        Ok(Some(url)) => {
                            tracing::info!(event = "bug.reported", matchId = %report.match_id, url = %url)
                        }
                        Ok(None) => {
                            tracing::info!(event = "bug.report.exists", matchId = %report.match_id)
                        }
                        Err(message) => {
                            tracing::error!(event = "bug.report.failed", matchId = %report.match_id, message = %message)
                        }
                    }
                });
            }
        }
    }

    /// Whether a report of `match_id` may be filed at `now`: not already, and within the window's
    /// allowance. Admitting it counts it.
    fn admit(&self, match_id: &str, now: i64) -> bool {
        let mut filed = lock(&self.filed);
        if filed.matches.contains(match_id) {
            return false;
        }
        while filed
            .at
            .front()
            .is_some_and(|at| now - at >= BUG_REPORT_WINDOW_MS)
        {
            filed.at.pop_front();
        }
        if filed.at.len() >= BUG_REPORTS_PER_WINDOW {
            tracing::warn!(event = "bug.report.capped", matchId = %match_id, perWindow = BUG_REPORTS_PER_WINDOW);
            return false;
        }
        filed.at.push_back(now);
        filed.matches.insert(match_id.to_string());
        true
    }
}

/// The issue a finding files: the micro patch's title (docs/issues-and-patches.md, Titles) naming
/// the match, so the next process finds it, and a body that shows no hidden information.
pub fn report_of(deployed_commit: Option<&str>, finding: &Finding) -> BugReport {
    let title = format!(
        "Patch v0.3.Y: match {} {}",
        finding.match_id,
        finding.found_in.title()
    );
    let mut body = format!(
        "The server found a match its own log does not reproduce (SPEC §9.3), in {}.\n\n\
         - Match: `{}`\n\
         - Build: `{}`, catalog `{}`\n\
         - Log: {} actions read; the fold refused {}\n",
        finding.found_in.said(),
        finding.match_id,
        deployed_commit.unwrap_or("unknown"),
        finding.catalog_version,
        finding.actions,
        finding.refused.len(),
    );
    if !finding.refused.is_empty() {
        body.push_str("\n| seq | seat | action |\n|---|---|---|\n");
        for row in finding.refused.iter().take(BUG_REPORT_LISTED_ACTIONS) {
            body.push_str(&format!("| {} | {} | `{}` |\n", row.seq, row.seat, row.action));
        }
        let more = finding.refused.len().saturating_sub(BUG_REPORT_LISTED_ACTIONS);
        if more > 0 {
            body.push_str(&format!("\nand {more} more.\n"));
        }
    }
    body.push_str(
        "\nA refused action is one the engine accepted live and refuses on the fold: either the log \
         was read or written out of order, or `reduce` is not deterministic. This report holds no \
         hidden information; read the log, the seed and the decks from the database as \
         crates/server/README.md (\"A match its log does not reproduce\") says, and fold them with \
         `cargo jackioh replay`.\n\n\
         Filed by the server (`crates/server/src/bug_reports.rs`), once per match.\n",
    );
    BugReport {
        match_id: finding.match_id.clone(),
        title,
        body,
        labels: BUG_REPORT_LABELS
            .iter()
            .map(|label| (*label).to_string())
            .collect(),
    }
}

/// Files `report` in `repo` unless an issue whose title names its match exists: `Ok(Some(url))`
/// filed, `Ok(None)` already there.
async fn post(
    http: &reqwest::Client,
    token: &str,
    repo: &str,
    report: &BugReport,
) -> Result<Option<String>, String> {
    let timeout = Duration::from_millis(BUG_REPORT_TIMEOUT_MS);
    let request = |builder: reqwest::RequestBuilder| {
        builder
            .bearer_auth(token)
            .header("accept", "application/vnd.github+json")
            .header("user-agent", "jackioh-server")
            .header("x-github-api-version", "2022-11-28")
            .timeout(timeout)
    };
    let query = format!("repo:{repo} is:issue in:title {}", report.match_id);
    let search = reqwest::Url::parse_with_params(
        &format!("{BUG_REPORT_API_URL}/search/issues"),
        &[("q", query.as_str()), ("per_page", "1")],
    )
    .map_err(|error| error.to_string())?;
    let found: Value = request(http.get(search))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    if found["total_count"].as_u64().unwrap_or(0) > 0 {
        return Ok(None);
    }
    let created: Value = request(http.post(format!("{BUG_REPORT_API_URL}/repos/{repo}/issues")))
        .json(&json!({ "title": report.title, "body": report.body, "labels": report.labels }))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
    Ok(created["html_url"].as_str().map(str::to_string))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(match_id: &str) -> Finding {
        Finding {
            match_id: match_id.to_string(),
            found_in: FoundIn::Rebuild,
            catalog_version: "v0.3.7b".to_string(),
            actions: 29,
            refused: vec![RefusedRow::of(
                10,
                &json!({ "type": "endTurn", "playerId": "p1", "nonce": "n" }),
            )],
        }
    }

    #[test]
    fn files_a_match_once_and_at_most_the_windows_allowance() {
        let reports = BugReports::recording();
        reports.file(None, finding("m-0"));
        reports.file(None, finding("m-0"));
        assert_eq!(reports.recorded().len(), 1);
        for n in 1..=BUG_REPORTS_PER_WINDOW {
            reports.file(None, finding(&format!("m-{n}")));
        }
        assert_eq!(reports.recorded().len(), BUG_REPORTS_PER_WINDOW);
    }

    #[test]
    fn the_window_lets_reports_through_again_once_an_hour_has_passed() {
        let reports = BugReports::recording();
        for n in 0..BUG_REPORTS_PER_WINDOW {
            assert!(reports.admit(&format!("m-{n}"), 0));
        }
        assert!(!reports.admit("late", BUG_REPORT_WINDOW_MS - 1));
        assert!(reports.admit("late", BUG_REPORT_WINDOW_MS));
    }

    #[test]
    fn a_report_names_the_match_and_the_rows_and_carries_the_night_bots_labels() {
        let report = report_of(Some("325bb54"), &finding("561d159d-a78c-495c-83e2-95357c18b850"));
        assert_eq!(
            report.title,
            "Patch v0.3.Y: match 561d159d-a78c-495c-83e2-95357c18b850 does not rebuild from its log"
        );
        assert!(report.body.contains("| 10 | p1 | `endTurn` |"), "{}", report.body);
        assert!(
            report.body.contains("`325bb54`, catalog `v0.3.7b`"),
            "{}",
            report.body
        );
        assert!(report.labels.contains(&"bot:build".to_string()));
    }

    #[test]
    fn reads_a_co_op_rows_seat_and_type_off_its_body() {
        let row = RefusedRow::of(
            7,
            &json!({ "seat": "a1", "nonce": "n", "body": { "type": "play" } }),
        );
        assert_eq!((row.seat.as_str(), row.action.as_str()), ("a1", "play"));
    }

    #[test]
    fn logs_only_without_a_token() {
        let reports = BugReports::off();
        reports.file(None, finding("m-0"));
        assert!(reports.recorded().is_empty());
    }
}
