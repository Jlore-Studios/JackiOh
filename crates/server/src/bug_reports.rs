//! Bug reports. Not in SPEC, and no R-row: operational, as #861's probes are.
//!
//! When the server finds a match its own log does not reproduce (§9.3's determinism break), it keeps
//! the match going, as the rebuild always has ("a live match is better than a dead one",
//! `actor::registry::rebuild`), logs the finding (`bug.found`) and files one GitHub issue the night
//! bot builds (`BUG_REPORT_LABELS`: `bot:build`, bot/README.md). It looks in six places:
//!
//!  - a 1v1 rebuild whose fold refuses an action the log holds (`actor::registry`);
//!  - a co-op rebuild that does (`duos::registry`);
//!  - the reaper's recovery fold that does (`api::results`);
//!  - a finished match whose log does not fold to its end (`api::game_records`);
//!  - a 1v1 live action whose `reduce` panicked (`actor::match_actor`, R1253000);
//!  - a co-op one (`duos::actor`).
//!
//! What a report says. The repository is public and the match may still be live, so a report names
//! each match, where it was found, the build and the catalog, and each refused row's seq, seat and
//! action type, never a refusal's text or a panic's message, a card or an instance id, any of which
//! could show a hand to the opponent. A maintainer reads the rest from the database (crates/server/README.md, "A match its
//! log does not reproduce").
//!
//! How often. Findings group by `Signature`: where found, the build, the catalog, and the first
//! refused action's type. A signature files one issue, titled with its first match and the
//! signature, whose body lists each match the process sees with it, up to
//! `BUG_REPORT_LISTED_MATCHES`, edited in as each arrives. A match is listed once per process. At
//! most `BUG_REPORTS_PER_WINDOW` issues are opened in `BUG_REPORT_WINDOW_MS`, and no issue is
//! opened when one whose title holds the signature exists already, open or closed, so a restart
//! files nothing twice, and that signature's later matches are only logged. Every finding is logged
//! whatever is filed. Without `CREATE_ISSUES_TOKEN` nothing leaves the process, and a test's app
//! records what it would file.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use indexmap::{IndexMap, IndexSet};
use jackioh_engine::FoldError;
use serde_json::{Value, json};

use crate::api::http::{lock, log_alert, now_ms};
use crate::config::{
    BUG_REPORT_API_URL, BUG_REPORT_LABELS, BUG_REPORT_LISTED_ACTIONS, BUG_REPORT_LISTED_MATCHES,
    BUG_REPORT_TIMEOUT_MS, BUG_REPORT_WINDOW_MS, BUG_REPORTS_PER_WINDOW,
};
use crate::db::store::MatchActionRow;
use crate::env::Env;

/// Where a finding was made.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FoundIn {
    /// A 1v1 match's actor rebuilt from its log (§9.5).
    Rebuild,
    /// A co-op match's actor rebuilt from its log (§13).
    DuosRebuild,
    /// The reaper's fold of a match past its ceiling (R833000).
    Reaper,
    /// A finished match's log, folded for its game record (R376).
    FinishedLog,
    /// A 1v1 match's live action whose `reduce` panicked (R1253000).
    Live,
    /// A co-op match's live action that did.
    DuosLive,
}

impl FoundIn {
    fn title(self) -> &'static str {
        match self {
            FoundIn::Rebuild => "does not rebuild from its log",
            FoundIn::DuosRebuild => "(co-op) does not rebuild from its log",
            FoundIn::Reaper => "does not fold from its log for the reaper",
            FoundIn::FinishedLog => "does not fold from its log to its result",
            FoundIn::Live => "panicked on a live action",
            FoundIn::DuosLive => "(co-op) panicked on a live action",
        }
    }

    fn is_live(self) -> bool {
        matches!(self, FoundIn::Live | FoundIn::DuosLive)
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
            FoundIn::Live => {
                "the 1v1 match's live actor; the action was answered `internal` and not applied, and a \
                 clock's action ends the match as the ceiling's draw"
            }
            FoundIn::DuosLive => {
                "the co-op match's live actor; the action was answered `internal` and not applied, and the \
                 AI's or the ceiling's ends the match as the ceiling's draw"
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

/// What a report groups findings by: where they were found, the build and the catalog, and the
/// first refused action's type (`none` when nothing was refused). Findings that share one are one
/// bug as far as a report can tell; each match's rows let a reader split them.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Signature {
    pub found_in: FoundIn,
    pub build: String,
    pub catalog_version: String,
    pub action: String,
}

impl Signature {
    pub fn of(deployed_commit: Option<&str>, finding: &Finding) -> Signature {
        Signature {
            found_in: finding.found_in,
            build: deployed_commit.unwrap_or("unknown").to_string(),
            catalog_version: finding.catalog_version.clone(),
            action: finding
                .refused
                .first()
                .map_or_else(|| "none".to_string(), |row| row.action.clone()),
        }
    }

    /// The text a report's title shows and the exists-check searches for.
    pub fn text(&self) -> String {
        format!(
            "{:?}/{}/{}/{}",
            self.found_in, self.build, self.catalog_version, self.action
        )
    }
}

/// An issue as it is filed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BugReport {
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
    /// A test's app: its groups are what would be filed (`recorded`).
    Recording,
}

/// The matches listed, the signatures reported, and when each issue of the window was opened.
#[derive(Default)]
struct Filed {
    matches: IndexSet<String>,
    groups: IndexMap<Signature, Group>,
    at: VecDeque<i64>,
}

/// One signature's findings, one per match, oldest first, and where its issue stands.
struct Group {
    findings: Vec<Finding>,
    issue: Issue,
}

/// Where a group's GitHub issue stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Issue {
    /// A task is opening it or editing its body, and lists any finding added meanwhile before it stops.
    Busy,
    /// Opened as issue `number`, its body listing the group's first `listed` findings; no task on it.
    Listed { number: u64, listed: usize },
    /// Filed already by an earlier process: never edited, so the group's later findings are only logged.
    Left,
}

/// What `Filed::take` made of a finding.
#[derive(Debug, PartialEq, Eq)]
enum Taken {
    /// A new signature within the window's allowance: open its issue.
    Open,
    /// A new match of a signature whose issue is open and idle: edit its body.
    List { number: u64, listed: usize },
    /// Listed already, the group full, a task on its issue, an earlier process's issue, or the window spent.
    Nothing,
}

impl Filed {
    fn take(&mut self, signature: &Signature, finding: Finding, now: i64) -> Taken {
        if self.matches.contains(&finding.match_id) {
            return Taken::Nothing;
        }
        if let Some(group) = self.groups.get_mut(signature) {
            if group.findings.len() >= BUG_REPORT_LISTED_MATCHES {
                tracing::warn!(
                    event = "bug.report.full",
                    matchId = %finding.match_id,
                    signature = %signature.text(),
                    listed = BUG_REPORT_LISTED_MATCHES
                );
                return Taken::Nothing;
            }
            self.matches.insert(finding.match_id.clone());
            group.findings.push(finding);
            let Issue::Listed { number, listed } = group.issue else {
                return Taken::Nothing;
            };
            group.issue = Issue::Busy;
            return Taken::List { number, listed };
        }
        while self.at.front().is_some_and(|at| now - at >= BUG_REPORT_WINDOW_MS) {
            self.at.pop_front();
        }
        if self.at.len() >= BUG_REPORTS_PER_WINDOW {
            tracing::warn!(
                event = "bug.report.capped",
                matchId = %finding.match_id,
                perWindow = BUG_REPORTS_PER_WINDOW,
                signature = %signature.text()
            );
            return Taken::Nothing;
        }
        self.at.push_back(now);
        self.matches.insert(finding.match_id.clone());
        self.groups.insert(
            signature.clone(),
            Group {
                findings: vec![finding],
                issue: Issue::Busy,
            },
        );
        Taken::Open
    }
}

/// The response header `GET /api/catalog` says whether reports can be filed in
/// (`Readiness::as_str`), beside `x-deployed-commit`, so `deploy-watch.yml` shows it after a deploy.
pub const BUG_REPORTS_HEADER: &str = "x-bug-reports";

/// Whether a report can be filed, as the boot's `probe` found it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Readiness {
    /// No token (or a test's app): findings are only logged.
    Off,
    /// A token, not yet probed.
    Checking,
    /// GitHub accepted the token for opening issues in the repository.
    Ready,
    /// GitHub refused it: expired, revoked, or without Issues write on the repository.
    Refused,
    /// GitHub did not answer.
    Unreachable,
}

impl Readiness {
    pub fn as_str(self) -> &'static str {
        match self {
            Readiness::Off => "off",
            Readiness::Checking => "checking",
            Readiness::Ready => "ready",
            Readiness::Refused => "refused",
            Readiness::Unreachable => "unreachable",
        }
    }
}

/// The app's reporter (`App::bug_reports`).
pub struct BugReports {
    sink: Sink,
    filed: Arc<Mutex<Filed>>,
    readiness: Mutex<Readiness>,
}

impl BugReports {
    fn with(sink: Sink) -> BugReports {
        let readiness = match sink {
            Sink::GitHub { .. } => Readiness::Checking,
            Sink::Off | Sink::Recording => Readiness::Off,
        };
        BugReports {
            sink,
            filed: Arc::new(Mutex::new(Filed::default())),
            readiness: Mutex::new(readiness),
        }
    }

    /// What the boot's `probe` found.
    pub fn readiness(&self) -> Readiness {
        *lock(&self.readiness)
    }

    /// At boot: asks GitHub to open an issue with no title, which it refuses as unprocessable (422)
    /// when the token may open issues in the repository and as forbidden or not found otherwise, so
    /// the token is checked and nothing is filed. Logs `bug.reports.ready`, or the alert
    /// `bug.reports.refused` with GitHub's answer.
    pub async fn probe(&self) {
        let Sink::GitHub { http, token, repo } = &self.sink else {
            return;
        };
        let answer = github(
            http.post(format!("{BUG_REPORT_API_URL}/repos/{repo}/issues")),
            token,
        )
        .json(&json!({}))
        .send()
        .await;
        let readiness = match answer {
            Ok(response) if response.status() == reqwest::StatusCode::UNPROCESSABLE_ENTITY => {
                tracing::info!(event = "bug.reports.ready", repo = %repo);
                Readiness::Ready
            }
            Ok(response) => {
                let status = response.status().as_u16();
                let message = response.text().await.unwrap_or_default();
                log_alert(
                    "bug.reports.refused",
                    json!({ "repo": repo, "status": status, "message": message }),
                );
                Readiness::Refused
            }
            Err(error) => {
                tracing::warn!(event = "bug.reports.unreachable", repo = %repo, message = %error);
                Readiness::Unreachable
            }
        };
        *lock(&self.readiness) = readiness;
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
        BugReports::with(Sink::Recording)
    }

    /// One report per signature, oldest first, as GitHub would show it now; nothing for any other.
    pub fn recorded(&self) -> Vec<BugReport> {
        match self.sink {
            Sink::Recording => lock(&self.filed)
                .groups
                .iter()
                .map(|(signature, group)| report_of(signature, &group.findings))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Logs every finding and files it under its signature (see the module docs). Never waits on
    /// GitHub: the request runs on its own task, and its failure is logged.
    pub fn file(&self, deployed_commit: Option<&str>, finding: Finding) {
        let signature = Signature::of(deployed_commit, &finding);
        let refused: Vec<i64> = finding.refused.iter().map(|row| row.seq).collect();
        log_alert(
            "bug.found",
            json!({
                "matchId": finding.match_id,
                "foundIn": format!("{:?}", finding.found_in),
                "actions": finding.actions,
                "refusedSeqs": refused,
                "signature": signature.text(),
            }),
        );
        if matches!(self.sink, Sink::Off) {
            return;
        }
        let taken = lock(&self.filed).take(&signature, finding, now_ms());
        let Sink::GitHub { http, token, repo } = &self.sink else {
            return;
        };
        let filing = Filing {
            http: http.clone(),
            token: token.clone(),
            repo: repo.clone(),
            filed: Arc::clone(&self.filed),
            signature,
        };
        match taken {
            Taken::Open => {
                tokio::spawn(filing.open());
            }
            Taken::List { number, listed } => {
                tokio::spawn(async move { filing.keep_listing(number, listed).await });
            }
            Taken::Nothing => {}
        }
    }
}

/// One signature's GitHub work, on its own task.
struct Filing {
    http: reqwest::Client,
    token: String,
    repo: String,
    filed: Arc<Mutex<Filed>>,
    signature: Signature,
}

impl Filing {
    async fn open(self) {
        let rendered = lock(&self.filed)
            .groups
            .get(&self.signature)
            .map(|group| (report_of(&self.signature, &group.findings), group.findings.len()));
        let Some((report, listed)) = rendered else {
            return;
        };
        match self.post(&report).await {
            Ok(Some((number, url))) => {
                tracing::info!(event = "bug.reported", signature = %self.signature.text(), url = %url);
                self.keep_listing(number, listed).await;
            }
            Ok(None) => {
                tracing::info!(event = "bug.report.exists", signature = %self.signature.text());
                if let Some(group) = lock(&self.filed).groups.get_mut(&self.signature) {
                    group.issue = Issue::Left;
                }
            }
            Err(message) => {
                tracing::error!(event = "bug.report.failed", signature = %self.signature.text(), message = %message);
                lock(&self.filed).groups.shift_remove(&self.signature);
            }
        }
    }

    async fn keep_listing(&self, number: u64, mut listed: usize) {
        loop {
            let (body, count) = {
                let mut filed = lock(&self.filed);
                let Some(group) = filed.groups.get_mut(&self.signature) else {
                    return;
                };
                if group.findings.len() == listed {
                    group.issue = Issue::Listed { number, listed };
                    return;
                }
                (
                    report_of(&self.signature, &group.findings).body,
                    group.findings.len(),
                )
            };
            if let Err(message) = self.edit(number, &body).await {
                tracing::error!(event = "bug.report.failed", signature = %self.signature.text(), message = %message);
                if let Some(group) = lock(&self.filed).groups.get_mut(&self.signature) {
                    group.issue = Issue::Listed { number, listed };
                }
                return;
            }
            listed = count;
        }
    }

    /// Files `report` in `repo` unless an issue whose title holds its signature exists:
    /// `Ok(Some((number, url)))` filed, `Ok(None)` already there.
    async fn post(&self, report: &BugReport) -> Result<Option<(u64, String)>, String> {
        let request = |builder: reqwest::RequestBuilder| github(builder, &self.token);
        let query = format!(
            "repo:{} is:issue in:title \"{}\"",
            self.repo,
            self.signature.text()
        );
        let search = reqwest::Url::parse_with_params(
            &format!("{BUG_REPORT_API_URL}/search/issues"),
            &[("q", query.as_str()), ("per_page", "1")],
        )
        .map_err(|error| error.to_string())?;
        let found: Value = request(self.http.get(search))
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
        let created: Value = request(
            self.http
                .post(format!("{BUG_REPORT_API_URL}/repos/{}/issues", self.repo)),
        )
        .json(&json!({ "title": report.title, "body": report.body, "labels": report.labels }))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())?;
        let number = created["number"]
            .as_u64()
            .ok_or_else(|| "GitHub answered no issue number".to_string())?;
        Ok(Some((
            number,
            created["html_url"].as_str().unwrap_or_default().to_string(),
        )))
    }

    async fn edit(&self, number: u64, body: &str) -> Result<(), String> {
        github(
            self.http.patch(format!(
                "{BUG_REPORT_API_URL}/repos/{}/issues/{number}",
                self.repo
            )),
            &self.token,
        )
        .json(&json!({ "body": body }))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map(drop)
        .map_err(|error| error.to_string())
    }
}

/// The issue a signature files: the micro patch's title (docs/issues-and-patches.md, Titles)
/// naming the first match and the signature, so the next process finds the issue, and a body that
/// shows no hidden information.
pub fn report_of(signature: &Signature, findings: &[Finding]) -> BugReport {
    let first = findings.first().map_or("?", |finding| finding.match_id.as_str());
    let title = format!(
        "Patch v0.3.Y: match {first} {} ({})",
        signature.found_in.title(),
        signature.text()
    );
    let live = signature.found_in.is_live();
    let mut body = if live {
        format!(
            "The engine panicked on a live action (SPEC §9.3: `reduce` is pure, so the same action on \
             the same state panics again), in {}.\n\n\
             - Signature: `{}`\n\
             - Build: `{}`, catalog `{}`\n\
             - Matches: {}\n",
            signature.found_in.said(),
            signature.text(),
            signature.build,
            signature.catalog_version,
            findings.len(),
        )
    } else {
        format!(
            "The server found a match its own log does not reproduce (SPEC §9.3), in {}.\n\n\
             - Signature: `{}`\n\
             - Build: `{}`, catalog `{}`\n\
             - Matches: {}\n",
            signature.found_in.said(),
            signature.text(),
            signature.build,
            signature.catalog_version,
            findings.len(),
        )
    };
    for finding in findings {
        body.push_str(&format!("\n### Match `{}`\n\n", finding.match_id));
        if live {
            body.push_str(&format!(
                "- Log: {} actions before it; the action that panicked was never logged\n",
                finding.actions,
            ));
        } else {
            body.push_str(&format!(
                "- Log: {} actions read; the fold refused {}\n",
                finding.actions,
                finding.refused.len(),
            ));
        }
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
    }
    body.push_str(if live {
        "\nThe server answered the action `internal` and keeps that answer for a resend. This report \
         holds no hidden information and not the panic's message, which can name a card; the \
         server's log line `match.action.panicked` (`duos.action.panicked` for co-op) holds it. Read \
         the log, the seed, the decks and the catalog version from the database as \
         crates/server/README.md (\"A match its log does not reproduce\") says, fold them with \
         `cargo jackioh replay`, keeping the catalog version in the file, and find the loop or cap \
         the action above reaches from the folded state.\n\n"
    } else {
        "\nA refused action is one the engine accepted live and refuses on the fold: either the log \
         was read or written out of order, or `reduce` is not deterministic. This report holds no \
         hidden information; read the log, the seed, the decks and the catalog version from the \
         database as crates/server/README.md (\"A match its log does not reproduce\") says, and fold \
         them with `cargo jackioh replay`, keeping the catalog version in the file: without it the \
         match folds under this build's catalog, not the one it was played on.\n\n"
    });
    body.push_str(&format!(
        "Filed by the server (`crates/server/src/bug_reports.rs`), one issue per signature, listing \
         each match a process saw with it up to {BUG_REPORT_LISTED_MATCHES}; the server's \
         `bug.found` log lines name every one.\n"
    ));
    BugReport {
        title,
        body,
        labels: BUG_REPORT_LABELS
            .iter()
            .map(|label| (*label).to_string())
            .collect(),
    }
}

/// A request to GitHub's REST API with the token and the headers it asks for.
fn github(builder: reqwest::RequestBuilder, token: &str) -> reqwest::RequestBuilder {
    builder
        .bearer_auth(token)
        .header("accept", "application/vnd.github+json")
        .header("user-agent", "jackioh-server")
        .header("x-github-api-version", "2022-11-28")
        .timeout(Duration::from_millis(BUG_REPORT_TIMEOUT_MS))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn finding(match_id: &str) -> Finding {
        Finding {
            match_id: match_id.to_string(),
            found_in: FoundIn::Rebuild,
            catalog_version: "v0.3.1".to_string(),
            actions: 29,
            refused: vec![RefusedRow::of(
                10,
                &json!({ "type": "endTurn", "playerId": "p1", "nonce": "n" }),
            )],
        }
    }

    fn report(deployed_commit: Option<&str>, finding: &Finding) -> BugReport {
        report_of(
            &Signature::of(deployed_commit, finding),
            std::slice::from_ref(finding),
        )
    }

    #[test]
    fn files_a_match_once_and_at_most_the_windows_allowance() {
        let reports = BugReports::recording();
        reports.file(None, finding("m-0"));
        reports.file(None, finding("m-0"));
        let recorded = reports.recorded();
        assert_eq!(recorded.len(), 1, "{recorded:?}");
        assert_eq!(
            recorded[0].body.matches("### Match `m-0`").count(),
            1,
            "{}",
            recorded[0].body
        );
        for n in 1..=BUG_REPORTS_PER_WINDOW {
            reports.file(
                None,
                Finding {
                    catalog_version: format!("v0.3.{n}"),
                    ..finding(&format!("m-{n}"))
                },
            );
        }
        assert_eq!(reports.recorded().len(), BUG_REPORTS_PER_WINDOW);
        reports.file(None, finding("m-late"));
        let recorded = reports.recorded();
        assert_eq!(recorded.len(), BUG_REPORTS_PER_WINDOW, "{recorded:?}");
        assert!(
            recorded[0].body.contains("### Match `m-late`"),
            "{}",
            recorded[0].body
        );
    }

    #[test]
    fn the_window_lets_reports_through_again_once_an_hour_has_passed() {
        let mut filed = Filed::default();
        for n in 0..BUG_REPORTS_PER_WINDOW {
            let found = Finding {
                catalog_version: format!("v0.3.{n}"),
                ..finding(&format!("m-{n}"))
            };
            let signature = Signature::of(None, &found);
            assert_eq!(filed.take(&signature, found, 0), Taken::Open);
        }
        let late = finding("m-late");
        let signature = Signature::of(None, &late);
        assert_eq!(
            filed.take(&signature, late, BUG_REPORT_WINDOW_MS - 1),
            Taken::Nothing
        );
        let late = finding("m-late");
        let signature = Signature::of(None, &late);
        assert_eq!(filed.take(&signature, late, BUG_REPORT_WINDOW_MS), Taken::Open);
    }

    #[test]
    fn two_matches_with_one_signature_file_one_issue_naming_both() {
        let reports = BugReports::recording();
        reports.file(Some("325bb54"), finding("m-0"));
        reports.file(Some("325bb54"), finding("m-1"));
        let recorded = reports.recorded();
        assert_eq!(recorded.len(), 1, "{recorded:?}");
        assert_eq!(
            recorded[0].title,
            "Patch v0.3.Y: match m-0 does not rebuild from its log \
             (Rebuild/325bb54/v0.3.1/endTurn)"
        );
        for needle in ["### Match `m-0`", "### Match `m-1`", "- Matches: 2"] {
            assert!(
                recorded[0].body.contains(needle),
                "{needle:?} missing from:\n{}",
                recorded[0].body
            );
        }
    }

    #[test]
    fn findings_with_different_signatures_file_one_issue_each() {
        let reports = BugReports::recording();
        reports.file(None, finding("m-0"));
        reports.file(
            None,
            Finding {
                found_in: FoundIn::Reaper,
                ..finding("m-1")
            },
        );
        reports.file(
            None,
            Finding {
                catalog_version: "v0.3.2".to_string(),
                ..finding("m-2")
            },
        );
        reports.file(Some("325bb54"), finding("m-3"));
        reports.file(
            None,
            Finding {
                refused: vec![RefusedRow::of(4, &json!({ "type": "play", "playerId": "p2" }))],
                ..finding("m-4")
            },
        );
        let recorded = reports.recorded();
        assert_eq!(recorded.len(), 5, "{recorded:?}");
        for (n, report) in recorded.iter().enumerate() {
            assert!(
                report.title.contains(&format!("match m-{n} ")),
                "{}",
                report.title
            );
        }
    }

    #[test]
    fn a_match_is_listed_once_whatever_its_signatures() {
        let reports = BugReports::recording();
        reports.file(None, finding("m-0"));
        reports.file(
            None,
            Finding {
                found_in: FoundIn::FinishedLog,
                ..finding("m-0")
            },
        );
        assert_eq!(reports.recorded().len(), 1, "{:?}", reports.recorded());
    }

    #[test]
    fn a_signatures_issue_lists_at_most_its_allowance_of_matches() {
        let reports = BugReports::recording();
        for n in 0..=BUG_REPORT_LISTED_MATCHES {
            reports.file(None, finding(&format!("m-{n}")));
        }
        let recorded = reports.recorded();
        assert_eq!(recorded.len(), 1, "{recorded:?}");
        assert!(
            recorded[0]
                .body
                .contains(&format!("- Matches: {BUG_REPORT_LISTED_MATCHES}\n")),
            "{}",
            recorded[0].body
        );
        assert!(
            !recorded[0]
                .body
                .contains(&format!("### Match `m-{BUG_REPORT_LISTED_MATCHES}`")),
            "{}",
            recorded[0].body
        );
    }

    #[test]
    fn a_match_found_while_its_issue_is_busy_waits_for_the_task_on_it() {
        let mut filed = Filed::default();
        let signature = Signature::of(None, &finding("m-0"));
        let take = |filed: &mut Filed, match_id: &str| {
            let found = finding(match_id);
            filed.take(&signature, found, 0)
        };
        assert_eq!(take(&mut filed, "m-0"), Taken::Open);
        assert_eq!(take(&mut filed, "m-1"), Taken::Nothing);
        filed.groups[&signature].issue = Issue::Listed { number: 7, listed: 2 };
        assert_eq!(take(&mut filed, "m-2"), Taken::List { number: 7, listed: 2 });
        assert_eq!(filed.groups[&signature].issue, Issue::Busy);
        assert_eq!(take(&mut filed, "m-3"), Taken::Nothing);
        filed.groups[&signature].issue = Issue::Left;
        assert_eq!(take(&mut filed, "m-4"), Taken::Nothing);
        assert_eq!(filed.groups[&signature].findings.len(), 5);
        assert_eq!(take(&mut filed, "m-4"), Taken::Nothing);
        assert_eq!(filed.groups[&signature].findings.len(), 5);
    }

    #[test]
    fn a_fold_that_refused_nothing_is_signed_none() {
        let found = Finding {
            found_in: FoundIn::FinishedLog,
            refused: Vec::new(),
            ..finding("m-0")
        };
        assert_eq!(
            Signature::of(None, &found).text(),
            "FinishedLog/unknown/v0.3.1/none"
        );
    }

    #[test]
    fn a_report_names_the_match_and_the_rows_and_carries_the_night_bots_labels() {
        let found = finding("561d159d-a78c-495c-83e2-95357c18b850");
        let report = report(Some("325bb54"), &found);
        assert_eq!(
            report.title,
            "Patch v0.3.Y: match 561d159d-a78c-495c-83e2-95357c18b850 does not rebuild from its log \
             (Rebuild/325bb54/v0.3.1/endTurn)"
        );
        assert!(report.body.contains("| 10 | p1 | `endTurn` |"), "{}", report.body);
        assert!(
            report.body.contains("`325bb54`, catalog `v0.3.1`"),
            "{}",
            report.body
        );
        assert!(report.labels.contains(&"bot:build".to_string()));
    }

    #[test]
    fn the_report_asks_for_the_catalog_version_in_the_folded_file() {
        let body = report(None, &finding("m-0")).body;
        for needle in [
            "the catalog version from the database",
            "fold them with `cargo jackioh replay`, keeping the catalog version in the file",
        ] {
            assert!(body.contains(needle), "{needle:?} missing from:\n{body}");
        }
    }

    #[test]
    fn a_live_panic_reports_the_action_it_would_have_logged_and_no_fold() {
        let live = Finding {
            found_in: FoundIn::Live,
            ..finding("m-0")
        };
        let live_report = report(None, &live);
        assert_eq!(
            live_report.title,
            "Patch v0.3.Y: match m-0 panicked on a live action (Live/unknown/v0.3.1/endTurn)"
        );
        for needle in ["The engine panicked on a live action", "| 10 | p1 | `endTurn` |"] {
            assert!(
                live_report.body.contains(needle),
                "{needle:?} missing from:\n{}",
                live_report.body
            );
        }
        assert!(
            !live_report.body.contains("the fold refused"),
            "{}",
            live_report.body
        );
        let duos = Finding {
            found_in: FoundIn::DuosLive,
            ..finding("m-1")
        };
        assert!(
            report(None, &duos)
                .title
                .contains("(co-op) panicked on a live action")
        );
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
        assert_eq!(reports.readiness(), Readiness::Off);
    }

    #[tokio::test]
    async fn probes_nothing_without_a_token() {
        let reports = BugReports::off();
        reports.probe().await;
        assert_eq!(reports.readiness().as_str(), "off");
    }
}
