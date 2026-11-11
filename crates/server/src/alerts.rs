//! Alerts told to people (#3595). Not in SPEC, and no R-row: operational, as `bug_reports.rs` is.
//!
//! An alert is an error log line with `alert = true`: `api::http::log_alert`'s, and the two of
//! `loops.rs`. Before #3595 one reached only the Worker's log. `AlertLayer`, beside the JSON log
//! (`app::init_logging`), now hands each to one task, which makes at most one notice per event in
//! `ALERT_WINDOW_MS` per process (the rest are counted, logged as `alert.held`, and the next notice
//! says how many) and tells people two ways:
//!
//! - **An issue** in `GITHUB_ISSUES_REPO`, with `CREATE_ISSUES_TOKEN`, titled `Patch: alert <event>
//!   on <site>` and labelled `ALERT_ISSUE_LABELS`. The repository is public, so an issue names the
//!   event, the site, the build and the time, never the alert's fields: a profile id is an account
//!   secret ID, a rate-limit key a player's address. While it is open, the event's next notice
//!   comments on it; once it is closed, the next notice opens another. An event in
//!   `ALERTS_FILED_ELSEWHERE` files its own issue (`bug.found`'s report) and gets none here.
//! - **An email** through Resend to `ALERT_EMAILS`, with `ALERT_RESEND_API_KEY`, holding every field.
//!   It goes when a notice opens an issue, or when no issue carries the notice (no token, GitHub
//!   failed, or the event is filed elsewhere). A comment on an open issue sends none: GitHub tells
//!   the people assigned to it. Resend's free plan sends the sign-in mail from the same 100 a day,
//!   so one process sends at most `ALERT_EMAILS_PER_DAY` a day, and each email's idempotency key
//!   (`email_key`) has Resend send one per new issue, or one per event and site a day, across every
//!   restart of the container.
//!
//! Nothing here raises an alert, so a notice that fails cannot loop: it is logged as
//! `alert.issue.failed` or `alert.email.failed`. The end-to-end server tells nobody.

use std::collections::VecDeque;
use std::time::Duration;

use indexmap::IndexMap;
use serde_json::{Value, json};
use tokio::sync::mpsc;
use tracing::field::{Field, Visit};
use tracing_subscriber::layer::Context;

use crate::api::http::now_ms;
use crate::config::{
    ALERT_EMAIL_API_URL, ALERT_EMAIL_DAY_MS, ALERT_EMAIL_FROM, ALERT_EMAILS_PER_DAY, ALERT_ISSUE_LABELS,
    ALERT_QUEUE, ALERT_TIMEOUT_MS, ALERT_WINDOW_MS, ALERTS_FILED_ELSEWHERE, BUG_REPORT_API_URL,
};
use crate::env::Env;
use crate::social::iso;

/// One alert, as its log line raised it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Alert {
    /// The line's `event`: `loop.crashed`, `api.rejections.unusual`, …
    pub event: String,
    /// The line's other fields as text (`data`, `loop`, `message`, …): the email's, never an issue's.
    pub fields: IndexMap<String, String>,
    /// When it was raised, in epoch ms.
    pub at: i64,
}

/// The tracing layer that hands every alert to the notifier's task.
pub struct AlertLayer {
    sender: mpsc::Sender<Alert>,
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for AlertLayer {
    fn on_event(&self, event: &tracing::Event<'_>, _: Context<'_, S>) {
        if *event.metadata().level() != tracing::Level::ERROR {
            return;
        }
        let mut line = Line::default();
        event.record(&mut line);
        if line.alert {
            // A full queue drops the alert; its log line is written all the same.
            let _ = self.sender.try_send(Alert {
                event: line.event.unwrap_or_else(|| "unnamed".to_string()),
                fields: line.fields,
                at: now_ms(),
            });
        }
    }
}

/// A log line's fields, read as text.
#[derive(Default)]
struct Line {
    alert: bool,
    event: Option<String>,
    fields: IndexMap<String, String>,
}

impl Visit for Line {
    fn record_bool(&mut self, field: &Field, value: bool) {
        if field.name() == "alert" {
            self.alert = value;
        } else {
            self.fields.insert(field.name().to_string(), value.to_string());
        }
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "event" {
            self.event = Some(value.to_string());
        } else {
            self.fields.insert(field.name().to_string(), value.to_string());
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.record_str(field, &format!("{value:?}"));
    }
}

/// When an event last made a notice, and how many of it were held back since.
struct Seen {
    noticed_at: i64,
    held: usize,
}

/// What the task has told people: each event's last notice, and when each email of the last day went.
#[derive(Default)]
struct Book {
    events: IndexMap<String, Seen>,
    emails: VecDeque<i64>,
}

impl Book {
    /// `Some(held)` when `event` makes a notice at `now`, `held` being how many were held back since
    /// its last; `None`, counted, within `ALERT_WINDOW_MS` of its last.
    fn notice(&mut self, event: &str, now: i64) -> Option<usize> {
        let Some(seen) = self.events.get_mut(event) else {
            self.events.insert(
                event.to_string(),
                Seen {
                    noticed_at: now,
                    held: 0,
                },
            );
            return Some(0);
        };
        if now - seen.noticed_at < ALERT_WINDOW_MS {
            seen.held += 1;
            return None;
        }
        let held = seen.held;
        *seen = Seen {
            noticed_at: now,
            held: 0,
        };
        Some(held)
    }

    /// Whether an email may go at `now`, under `ALERT_EMAILS_PER_DAY`; one that may is counted.
    fn email(&mut self, now: i64) -> bool {
        while self
            .emails
            .front()
            .is_some_and(|at| now - at >= ALERT_EMAIL_DAY_MS)
        {
            self.emails.pop_front();
        }
        if self.emails.len() >= ALERT_EMAILS_PER_DAY {
            return false;
        }
        self.emails.push_back(now);
        true
    }
}

/// What became of a notice's issue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Issue {
    /// None was asked for: no token, or the event files its own.
    NotFiled,
    /// A new issue, at this address.
    Opened(String),
    /// A comment on the event's open issue, at this address.
    Commented(String),
    /// GitHub failed (logged as `alert.issue.failed`).
    Failed,
}

impl Issue {
    /// Whether the notice is emailed too: unless it went to an issue people already know of.
    fn emails(&self) -> bool {
        !matches!(self, Issue::Commented(_))
    }
}

struct GitHub {
    api: String,
    token: String,
    repo: String,
}

struct Email {
    api: String,
    key: String,
    to: Vec<String>,
}

/// The notifier's task's state: where notices go, and what it has told.
pub struct Notifier {
    http: reqwest::Client,
    site: String,
    build: Option<String>,
    github: Option<GitHub>,
    email: Option<Email>,
    book: Book,
}

impl Notifier {
    /// A deployment's: `None` when it can tell nobody (no token and no email, or the end-to-end server).
    pub fn from_env(env: &Env) -> Option<Notifier> {
        if env.e2e {
            return None;
        }
        let github = env.github_issues_token.as_ref().map(|token| GitHub {
            api: BUG_REPORT_API_URL.to_string(),
            token: token.clone(),
            repo: env.github_issues_repo.clone(),
        });
        let email = env
            .alert_email_key
            .as_ref()
            .filter(|_| !env.alert_emails.is_empty())
            .map(|key| Email {
                api: ALERT_EMAIL_API_URL.to_string(),
                key: key.clone(),
                to: env.alert_emails.clone(),
            });
        if github.is_none() && email.is_none() {
            return None;
        }
        Some(Notifier {
            http: reqwest::Client::new(),
            site: site_of(&env.public_origins),
            build: env.deployed_commit.clone(),
            github,
            email,
            book: Book::default(),
        })
    }

    /// What the boot logs as `alerts.ready`: whether issues are filed, and to how many addresses an
    /// email goes (never which).
    pub fn ready(&self) -> Value {
        json!({
            "site": self.site,
            "issues": self.github.is_some(),
            "emails": self.email.as_ref().map_or(0, |email| email.to.len()),
        })
    }

    /// Starts the task and answers the layer that feeds it.
    pub fn start(mut self) -> AlertLayer {
        let (sender, mut receiver) = mpsc::channel(ALERT_QUEUE);
        tokio::spawn(async move {
            while let Some(alert) = receiver.recv().await {
                self.handle(alert).await;
            }
        });
        AlertLayer { sender }
    }

    /// One alert: held back within its window, or else filed and emailed (see the module docs).
    async fn handle(&mut self, alert: Alert) -> Option<Issue> {
        let Some(held) = self.book.notice(&alert.event, alert.at) else {
            tracing::info!(event = "alert.held", of = %alert.event);
            return None;
        };
        let issue = self.issue(&alert, held).await;
        if issue.emails() && self.email.is_some() {
            if self.book.email(alert.at) {
                self.send_email(&alert, held, &issue).await;
            } else {
                tracing::warn!(event = "alert.email.capped", of = %alert.event, perDay = ALERT_EMAILS_PER_DAY);
            }
        }
        Some(issue)
    }

    async fn issue(&self, alert: &Alert, held: usize) -> Issue {
        let Some(github) = &self.github else {
            return Issue::NotFiled;
        };
        if ALERTS_FILED_ELSEWHERE.contains(&alert.event.as_str()) {
            return Issue::NotFiled;
        }
        match self.file(github, alert, held).await {
            Ok(issue) => {
                tracing::info!(event = "alert.filed", of = %alert.event, issue = ?issue);
                issue
            }
            Err(message) => {
                tracing::error!(event = "alert.issue.failed", of = %alert.event, message = %message);
                Issue::Failed
            }
        }
    }

    /// Comments on the event's open issue, or opens one.
    async fn file(&self, github: &GitHub, alert: &Alert, held: usize) -> Result<Issue, String> {
        let title = issue_title(&alert.event, &self.site);
        let query = format!("repo:{} is:issue is:open in:title \"{title}\"", github.repo);
        let search = reqwest::Url::parse_with_params(
            &format!("{}/search/issues", github.api),
            &[("q", query.as_str()), ("per_page", "10")],
        )
        .map_err(|error| error.to_string())?;
        let found = answer(crate::bug_reports::github(self.http.get(search), &github.token)).await?;
        // The search matches words, so only an issue with exactly this title is the event's.
        let open = found["items"]
            .as_array()
            .and_then(|items| items.iter().find(|item| item["title"] == title.as_str()));
        if let Some(open) = open {
            let number = open["number"]
                .as_u64()
                .ok_or_else(|| "GitHub answered an issue with no number".to_string())?;
            let url = open["html_url"].as_str().unwrap_or_default().to_string();
            answer(github_post(
                &self.http,
                github,
                &format!("issues/{number}/comments"),
                json!({ "body": comment_of(self.build.as_deref(), alert, held) }),
            ))
            .await?;
            return Ok(Issue::Commented(url));
        }
        let created = answer(github_post(
            &self.http,
            github,
            "issues",
            json!({
                "title": title,
                "body": issue_body(&self.site, self.build.as_deref(), alert, held),
                "labels": ALERT_ISSUE_LABELS,
            }),
        ))
        .await?;
        Ok(Issue::Opened(
            created["html_url"].as_str().unwrap_or_default().to_string(),
        ))
    }

    async fn send_email(&self, alert: &Alert, held: usize, issue: &Issue) {
        let Some(email) = &self.email else {
            return;
        };
        let (subject, text) = email_of(&self.site, self.build.as_deref(), alert, held, issue);
        let sent = self
            .http
            .post(&email.api)
            .bearer_auth(&email.key)
            .header("user-agent", "jackioh-server")
            .header("idempotency-key", email_key(&self.site, alert, issue))
            .timeout(Duration::from_millis(ALERT_TIMEOUT_MS))
            .json(&json!({ "from": ALERT_EMAIL_FROM, "to": email.to, "subject": subject, "text": text }))
            .send()
            .await
            .and_then(reqwest::Response::error_for_status);
        match sent {
            Ok(_) => tracing::info!(event = "alert.emailed", of = %alert.event, to = email.to.len()),
            // Resend kept the key: another process emailed this already (`email_key`).
            Err(error) if error.status() == Some(reqwest::StatusCode::CONFLICT) => {
                tracing::info!(event = "alert.email.repeat", of = %alert.event);
            }
            Err(error) => tracing::error!(event = "alert.email.failed", of = %alert.event, message = %error),
        }
    }
}

fn github_post(http: &reqwest::Client, github: &GitHub, path: &str, body: Value) -> reqwest::RequestBuilder {
    crate::bug_reports::github(
        http.post(format!("{}/repos/{}/{path}", github.api, github.repo)),
        &github.token,
    )
    .json(&body)
}

/// A request's JSON answer, or why there is none.
async fn answer(request: reqwest::RequestBuilder) -> Result<Value, String> {
    request
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|error| error.to_string())?
        .json()
        .await
        .map_err(|error| error.to_string())
}

/// The email's `Idempotency-Key`, which Resend keeps a day and sends once: one email per new issue,
/// and otherwise one per event and site per UTC day. Each process's window and allowance are its own,
/// and the container restarts many times a day (it sleeps when idle), so without it an alert raised at
/// every boot while GitHub fails (`bug.reports.refused`) would email at every boot, spending the
/// 100 a day the sign-in mail needs.
pub fn email_key(site: &str, alert: &Alert, issue: &Issue) -> String {
    match issue {
        Issue::Opened(url) => format!("jackioh-alert/{site}/{}/{url}", alert.event),
        _ => {
            let day = iso(alert.at).chars().take(10).collect::<String>();
            format!("jackioh-alert/{site}/{}/{day}", alert.event)
        }
    }
}

/// The site a notice names: the host of the deployment's first browser origin (`jackioh.com`,
/// `staging.jackioh.com`).
pub fn site_of(origins: &[String]) -> String {
    origins.first().map_or_else(
        || "an unnamed site".to_string(),
        |origin| {
            let host = origin.split_once("://").map_or(origin.as_str(), |(_, host)| host);
            host.trim_end_matches('/').to_string()
        },
    )
}

/// An event's issue title, which the next notice searches for: one open issue per event and site.
pub fn issue_title(event: &str, site: &str) -> String {
    format!("Patch: alert {event} on {site}")
}

/// What an event means, for a person reading the notice on a phone.
pub fn meaning(event: &str) -> &'static str {
    match event {
        "loop.crashed" => {
            "A background loop (the matchmaker, the reaper, the series sweeper, the retention purge, the bot \
             pool or the store's heartbeat) panicked. It restarts by itself, but the panic is a bug."
        }
        "loop.returned" => "A background loop returned, which none should. It restarts by itself.",
        "api.rejections.unusual" => {
            "One caller had more requests refused by the API's rate limit than any player should \
             (R1717000): a script, a broken client or an attack."
        }
        "match.rejections.unusual" => {
            "One player had more match actions refused than any player should (R835000): a modified or \
             broken client."
        }
        "ranked.profile_missing" => {
            "A rated game named a profile the database does not hold, and was rated from the starting rating."
        }
        "game_records.fold_poison" => {
            "A game record could not be read into the card statistics and was set aside (R1715000)."
        }
        "bug.reports.refused" => {
            "GitHub refused CREATE_ISSUES_TOKEN, so neither bug reports nor alerts can open issues until \
             it is renewed (crates/server/README.md, A match its log does not reproduce)."
        }
        "bug.found" => {
            "A match did not replay from its own log, or the engine panicked on a live action. Its bug \
             report has an issue of its own for the night bot."
        }
        _ => "An alert `alerts::meaning` does not describe yet (crates/server/src/alerts.rs).",
    }
}

fn when(build: Option<&str>, alert: &Alert) -> String {
    format!("{} on build `{}`", iso(alert.at), build.unwrap_or("unknown"))
}

fn held_note(held: usize) -> String {
    match held {
        0 => String::new(),
        1 => "\n\n1 more was held back since the last notice.".to_string(),
        n => format!("\n\n{n} more were held back since the last notice."),
    }
}

/// An event's new issue. Public: it names no field of the alert.
pub fn issue_body(site: &str, build: Option<&str>, alert: &Alert, held: usize) -> String {
    format!(
        "The server on `{site}` raised the alert `{event}` at {when}.\n\n{meaning}{held}\n\n\
         Its details went by email to the owners (`ALERT_EMAILS`) and are in the Worker's logs \
         (Workers & Pages, the site's Worker, Logs, `event` = `{event}`). The repository is public, so \
         this issue holds none of them.\n\n\
         While this issue is open, the alert recurring adds a comment here, at most once an hour per \
         server process, and sends no email. Close it once it is handled: the next one opens a new \
         issue and emails again (`crates/server/src/alerts.rs`, #3595).",
        event = alert.event,
        when = when(build, alert),
        meaning = meaning(&alert.event),
        held = held_note(held),
    )
}

/// A notice on an event's open issue. Public, as the issue is.
pub fn comment_of(build: Option<&str>, alert: &Alert, held: usize) -> String {
    format!("Seen again at {}.{}", when(build, alert), held_note(held))
}

/// An alert's email, `(subject, text)`: private, so it holds every field of the alert.
pub fn email_of(
    site: &str,
    build: Option<&str>,
    alert: &Alert,
    held: usize,
    issue: &Issue,
) -> (String, String) {
    let subject = format!("JackiOh alert on {site}: {}", alert.event);
    let issue = match issue {
        Issue::Opened(url) => format!("Issue: {url}"),
        Issue::Commented(url) => format!("Issue (open already): {url}"),
        Issue::NotFiled => "Issue: none (no CREATE_ISSUES_TOKEN, or the event files its own)".to_string(),
        Issue::Failed => "Issue: GitHub failed, see `alert.issue.failed` in the Worker's logs".to_string(),
    };
    let fields: String = alert
        .fields
        .iter()
        .map(|(name, value)| format!("  {name}: {value}\n"))
        .collect();
    let text = format!(
        "The server on {site} raised the alert {event} at {when}.\n\n{meaning}{held}\n\n{issue}\n\n\
         Details:\n{fields}\n\
         The Worker's logs hold every line: Workers & Pages, the site's Worker, Logs, event = {event}.\n",
        event = alert.event,
        when = when(build, alert),
        meaning = meaning(&alert.event),
        held = held_note(held),
        fields = if fields.is_empty() {
            "  (none)\n".to_string()
        } else {
            fields
        },
    );
    (subject, text)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    use axum::Router;
    use axum::extract::{Path, Query, State};
    use axum::http::{HeaderMap, StatusCode, Uri};
    use axum::response::{IntoResponse, Response};
    use tracing_subscriber::layer::SubscriberExt;

    use super::*;

    fn alert(event: &str, at: i64) -> Alert {
        Alert {
            event: event.to_string(),
            fields: [
                (
                    "data".to_string(),
                    r#"{"profileId":"0b6f-secret","key":"ip:203.0.113.9"}"#.to_string(),
                ),
                ("loop".to_string(), "matchmaker".to_string()),
            ]
            .into_iter()
            .collect(),
            at,
        }
    }

    #[test]
    fn the_layer_hands_on_an_alert_with_its_fields_and_nothing_else() {
        let (sender, mut receiver) = mpsc::channel(ALERT_QUEUE);
        let subscriber = tracing_subscriber::registry().with(AlertLayer { sender });
        tracing::subscriber::with_default(subscriber, || {
            crate::api::http::log_alert("api.rejections.unusual", json!({ "key": "ip:203.0.113.9" }));
            tracing::error!(event = "loop.crashed", alert = true, "loop" = "reaper", message = %"boom");
            tracing::error!(event = "store.failed", message = "an error, not an alert");
            tracing::warn!(event = "a.warning", alert = true);
        });
        let first = receiver.try_recv().expect("log_alert's line");
        assert_eq!(first.event, "api.rejections.unusual");
        assert_eq!(first.fields["data"], r#"{"key":"ip:203.0.113.9"}"#);
        let second = receiver.try_recv().expect("loops.rs's line");
        assert_eq!(second.event, "loop.crashed");
        assert_eq!(second.fields["loop"], "reaper");
        assert_eq!(second.fields["message"], "boom");
        assert!(
            receiver.try_recv().is_err(),
            "an error without `alert` and a warning are not alerts"
        );
    }

    #[test]
    fn an_event_makes_one_notice_an_hour_and_the_next_counts_what_was_held() {
        let mut book = Book::default();
        assert_eq!(book.notice("loop.crashed", 0), Some(0));
        assert_eq!(book.notice("loop.crashed", 1), None);
        assert_eq!(book.notice("loop.crashed", ALERT_WINDOW_MS - 1), None);
        assert_eq!(
            book.notice("loop.returned", 2),
            Some(0),
            "each event has its own window"
        );
        assert_eq!(book.notice("loop.crashed", ALERT_WINDOW_MS), Some(2));
        assert_eq!(book.notice("loop.crashed", ALERT_WINDOW_MS + 1), None);
    }

    #[test]
    fn emails_stop_at_the_days_allowance_and_go_again_a_day_after_the_first() {
        let mut book = Book::default();
        for n in 0..ALERT_EMAILS_PER_DAY {
            assert!(book.email(i64::try_from(n).expect("small")), "email {n}");
        }
        assert!(!book.email(ALERT_EMAIL_DAY_MS - 1));
        assert!(book.email(ALERT_EMAIL_DAY_MS));
        assert!(!book.email(ALERT_EMAIL_DAY_MS));
    }

    #[test]
    fn the_site_is_the_first_origins_host() {
        let origins = |list: &[&str]| {
            list.iter()
                .map(|origin| (*origin).to_string())
                .collect::<Vec<_>>()
        };
        assert_eq!(
            site_of(&origins(&["https://jackioh.com", "https://www.jackioh.com"])),
            "jackioh.com"
        );
        assert_eq!(
            site_of(&origins(&["https://staging.jackioh.com/"])),
            "staging.jackioh.com"
        );
        assert_eq!(site_of(&[]), "an unnamed site");
    }

    #[test]
    fn the_public_issue_and_comment_name_no_field_and_the_private_email_names_them_all() {
        let raised = alert("match.rejections.unusual", 1_762_776_000_000);
        let body = issue_body("jackioh.com", Some("abc1234"), &raised, 3);
        let comment = comment_of(Some("abc1234"), &raised, 0);
        for public in [&body, &comment] {
            assert!(!public.contains("0b6f-secret"), "{public}");
            assert!(!public.contains("203.0.113.9"), "{public}");
            assert!(!public.contains("matchmaker"), "{public}");
            assert!(public.contains("abc1234"), "{public}");
        }
        assert!(
            body.contains("`match.rejections.unusual`") && body.contains("R835000"),
            "{body}"
        );
        assert!(body.contains("3 more were held back"), "{body}");
        assert!(!comment.contains("held back"), "{comment}");

        let (subject, text) = email_of(
            "jackioh.com",
            None,
            &raised,
            0,
            &Issue::Opened("https://github.com/o/r/issues/9".to_string()),
        );
        assert_eq!(subject, "JackiOh alert on jackioh.com: match.rejections.unusual");
        assert!(
            text.contains("0b6f-secret") && text.contains("203.0.113.9"),
            "{text}"
        );
        assert!(text.contains("loop: matchmaker"), "{text}");
        assert!(text.contains("Issue: https://github.com/o/r/issues/9"), "{text}");
        assert!(text.contains("build `unknown`"), "{text}");
    }

    #[test]
    fn the_email_key_is_one_per_new_issue_or_else_one_per_event_site_and_day() {
        let at = 1_794_312_000_000; // 2026-11-10T12:00:00Z
        let raised = alert("bug.reports.refused", at);
        let later = alert("bug.reports.refused", at + 6 * 60 * 60 * 1000);
        let failed = email_key("jackioh.com", &raised, &Issue::Failed);
        assert_eq!(failed, "jackioh-alert/jackioh.com/bug.reports.refused/2026-11-10");
        assert_eq!(
            email_key("jackioh.com", &later, &Issue::NotFiled),
            failed,
            "a restart the same day"
        );
        assert_ne!(email_key("staging.jackioh.com", &raised, &Issue::Failed), failed);
        let next_day = alert("bug.reports.refused", at + ALERT_EMAIL_DAY_MS);
        assert_ne!(email_key("jackioh.com", &next_day, &Issue::Failed), failed);
        let opened = Issue::Opened("https://github.com/o/r/issues/77".to_string());
        let per_issue = email_key("jackioh.com", &raised, &opened);
        assert_eq!(
            per_issue,
            "jackioh-alert/jackioh.com/bug.reports.refused/https://github.com/o/r/issues/77"
        );
        assert!(per_issue.len() <= 256, "Resend's longest key");
    }

    #[test]
    fn every_alert_the_server_raises_says_what_it_means() {
        let fallback = meaning("no.such.event");
        for event in [
            "loop.crashed",
            "loop.returned",
            "api.rejections.unusual",
            "match.rejections.unusual",
            "ranked.profile_missing",
            "game_records.fold_poison",
            "bug.reports.refused",
            "bug.found",
        ] {
            assert_ne!(meaning(event), fallback, "{event}");
        }
    }

    /// What the fake GitHub and Resend were asked: (method and path, what came with it).
    type Asked = Arc<Mutex<Vec<(String, Value)>>>;

    /// The fake's state: what it was asked, what its search finds, and whether GitHub fails.
    type Fake = (Asked, Value, bool);

    async fn fake_search(
        State((asked, found, fail)): State<Fake>,
        uri: Uri,
        Query(query): Query<BTreeMap<String, String>>,
    ) -> Response {
        asked
            .lock()
            .expect("lock")
            .push((format!("GET {}", uri.path()), json!(query)));
        if fail {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        axum::Json(found).into_response()
    }

    async fn fake_post(
        State((asked, _, fail)): State<Fake>,
        uri: Uri,
        headers: HeaderMap,
        Path(rest): Path<String>,
        axum::Json(body): axum::Json<Value>,
    ) -> Response {
        let header = |name: &str| {
            headers
                .get(name)
                .and_then(|value| value.to_str().ok())
                .unwrap_or_default()
                .to_string()
        };
        let key = header("idempotency-key");
        let repeat = rest == "emails"
            && asked
                .lock()
                .expect("lock")
                .iter()
                .any(|(_, earlier)| earlier["key"] == key.as_str());
        asked.lock().expect("lock").push((
            format!("POST {}", uri.path()),
            json!({ "auth": header("authorization"), "key": key, "body": body }),
        ));
        if repeat {
            // Resend's answer to a key it kept, sent again with another payload: nothing is sent.
            return StatusCode::CONFLICT.into_response();
        }
        if fail && rest != "emails" {
            return StatusCode::INTERNAL_SERVER_ERROR.into_response();
        }
        axum::Json(json!({ "number": 77, "html_url": "https://github.com/o/r/issues/77", "id": "e-1" }))
            .into_response()
    }

    /// GitHub's and Resend's API faked on a local port: `open` is the open issue its search finds
    /// beside one whose title only starts the same (`None`: none), and `fail` answers GitHub 500.
    async fn fake(open: Option<(&str, u64)>, fail: bool) -> (SocketAddr, Asked) {
        let asked: Asked = Arc::default();
        let found = open.map_or_else(
            || json!({ "total_count": 0, "items": [] }),
            |(title, number)| {
                json!({ "total_count": 2, "items": [
                    { "title": format!("{title} and more"), "number": 1, "html_url": "https://github.com/o/r/issues/1" },
                    { "title": title, "number": number, "html_url": format!("https://github.com/o/r/issues/{number}") },
                ] })
            },
        );
        let router = Router::new()
            .route("/search/issues", axum::routing::get(fake_search))
            .route("/{*rest}", axum::routing::post(fake_post))
            .with_state((Arc::clone(&asked), found, fail));
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("a local port");
        let address = listener.local_addr().expect("its address");
        tokio::spawn(async move { axum::serve(listener, router).await });
        (address, asked)
    }

    fn notifier(address: SocketAddr, github: bool, email: bool) -> Notifier {
        let api = format!("http://{address}");
        Notifier {
            http: reqwest::Client::new(),
            site: "jackioh.com".to_string(),
            build: Some("abc1234".to_string()),
            github: github.then(|| GitHub {
                api: api.clone(),
                token: "ghp_token".to_string(),
                repo: "o/r".to_string(),
            }),
            email: email.then(|| Email {
                api: format!("{api}/emails"),
                key: "re_key".to_string(),
                to: vec!["one@example.com".to_string(), "two@example.com".to_string()],
            }),
            book: Book::default(),
        }
    }

    fn paths(asked: &Asked) -> Vec<String> {
        asked
            .lock()
            .expect("lock")
            .iter()
            .map(|(path, _)| path.clone())
            .collect()
    }

    #[tokio::test]
    async fn a_first_alert_opens_an_issue_and_emails_it_and_the_window_holds_the_next() {
        let (address, asked) = fake(None, false).await;
        let mut notifier = notifier(address, true, true);
        let issue = notifier.handle(alert("loop.crashed", 1_000)).await;
        assert_eq!(
            issue,
            Some(Issue::Opened("https://github.com/o/r/issues/77".to_string()))
        );
        assert_eq!(
            paths(&asked),
            ["GET /search/issues", "POST /repos/o/r/issues", "POST /emails"]
        );
        let asked_now = asked.lock().expect("lock").clone();
        let query = asked_now[0].1["q"].as_str().unwrap_or_default().to_string();
        assert!(
            query.contains("is:open") && query.contains("\"Patch: alert loop.crashed on jackioh.com\""),
            "{query}"
        );
        let opened = &asked_now[1].1;
        assert_eq!(opened["auth"], "Bearer ghp_token");
        assert_eq!(
            opened["body"]["title"],
            "Patch: alert loop.crashed on jackioh.com"
        );
        assert_eq!(opened["body"]["labels"], json!(ALERT_ISSUE_LABELS));
        assert!(
            !opened["body"]["body"]
                .as_str()
                .unwrap_or_default()
                .contains("0b6f-secret")
        );
        let emailed = &asked_now[2].1;
        assert_eq!(emailed["auth"], "Bearer re_key");
        assert_eq!(emailed["body"]["from"], ALERT_EMAIL_FROM);
        assert_eq!(
            emailed["body"]["to"],
            json!(["one@example.com", "two@example.com"])
        );
        assert!(
            emailed["body"]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("0b6f-secret")
        );
        assert!(
            emailed["body"]["text"]
                .as_str()
                .unwrap_or_default()
                .contains("issues/77")
        );

        assert_eq!(notifier.handle(alert("loop.crashed", 2_000)).await, None);
        assert_eq!(paths(&asked).len(), 3, "held back: nothing more was asked");
    }

    #[tokio::test]
    async fn an_open_issue_takes_a_comment_and_no_email() {
        let (address, asked) = fake(Some(("Patch: alert loop.crashed on jackioh.com", 41)), false).await;
        let mut notifier = notifier(address, true, true);
        let issue = notifier.handle(alert("loop.crashed", 1_000)).await;
        assert_eq!(
            issue,
            Some(Issue::Commented("https://github.com/o/r/issues/41".to_string()))
        );
        assert_eq!(
            paths(&asked),
            ["GET /search/issues", "POST /repos/o/r/issues/41/comments"]
        );
        let body = asked.lock().expect("lock")[1].1["body"]["body"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        assert!(
            body.starts_with("Seen again at ") && !body.contains("0b6f-secret"),
            "{body}"
        );
    }

    #[tokio::test]
    async fn github_failing_or_off_or_filing_elsewhere_still_emails() {
        let (address, asked) = fake(None, true).await;
        let mut failing = notifier(address, true, true);
        assert_eq!(
            failing.handle(alert("loop.crashed", 1_000)).await,
            Some(Issue::Failed)
        );
        assert_eq!(paths(&asked), ["GET /search/issues", "POST /emails"]);

        let (address, asked) = fake(None, false).await;
        let mut elsewhere = notifier(address, true, true);
        assert_eq!(
            elsewhere.handle(alert("bug.found", 1_000)).await,
            Some(Issue::NotFiled)
        );
        let mut off = notifier(address, false, true);
        assert_eq!(
            off.handle(alert("loop.crashed", 1_000)).await,
            Some(Issue::NotFiled)
        );
        assert_eq!(paths(&asked), ["POST /emails", "POST /emails"]);

        let mut no_email = notifier(address, false, false);
        assert_eq!(
            no_email.handle(alert("loop.crashed", 1_000)).await,
            Some(Issue::NotFiled)
        );
        assert_eq!(paths(&asked).len(), 2, "no email set: nothing sent");
    }

    #[tokio::test]
    async fn two_processes_raising_one_alert_the_same_day_send_one_key() {
        let (address, asked) = fake(None, true).await;
        for _process in 0..2 {
            let mut booted = notifier(address, true, true);
            booted.handle(alert("bug.reports.refused", 1_000)).await;
        }
        let keys: Vec<Value> = asked
            .lock()
            .expect("lock")
            .iter()
            .filter(|(path, _)| path == "POST /emails")
            .map(|(_, asked)| asked["key"].clone())
            .collect();
        assert_eq!(keys.len(), 2);
        assert_eq!(keys[0], keys[1], "Resend sends the second key's email no more");
        assert_eq!(
            keys[0],
            "jackioh-alert/jackioh.com/bug.reports.refused/1970-01-01"
        );
    }

    #[tokio::test]
    async fn a_process_emails_at_most_the_days_allowance() {
        let (address, asked) = fake(None, false).await;
        let mut notifier = notifier(address, false, true);
        for n in 0..=ALERT_EMAILS_PER_DAY {
            notifier.handle(alert(&format!("event.{n}"), 1_000)).await;
        }
        assert_eq!(paths(&asked).len(), ALERT_EMAILS_PER_DAY);
    }
}
