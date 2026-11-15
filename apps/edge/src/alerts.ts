// #3618: what the Worker tells people when it cannot reach the game server. It is `crates/server/src/alerts.rs`'s
// notice, with the same title, labels, email and idempotency key: a server that cannot boot, or that the Worker
// cannot reach, tells nobody itself. Not in SPEC and no R-row: operational, as #3595 was.
//
// The Worker raises one event, `server.unreachable`, and tells two ways, each of which is optional:
//
// - **An issue** in `ALERT_REPO`, with `CREATE_ISSUES_TOKEN`, titled `Patch: alert server.unreachable on <site>`.
//   The repository is public, so an issue names the event, the site, the build and the time, never the alert's
//   fields. While it is open the next notice comments on it; once it is closed the next opens another.
// - **An email** through Resend to `ALERT_EMAILS`, with `ALERT_RESEND_API_KEY`, holding every field. It goes
//   when a notice opens an issue, or when no issue carries the notice; a comment on an open issue sends none.
//   Its `Idempotency-Key` (`emailKey`) has Resend send one per new issue, or one per site a day, so the
//   Worker never spends the 100 emails a day that the sign-in mail shares.
//
// What this file does not do is decide when to tell: `Server` in index.ts keeps when it last told, in its
// Durable Object's storage, and calls `tell` from `ctx.waitUntil`. A notice that fails is logged and never
// raises another alert, so it cannot loop.

import type { ServerSettings } from "./routing.ts";

/** The one event the Worker raises (`alerts.rs`' `alert.event`, in an issue's title and an email's subject). */
export const UNREACHABLE_EVENT = "server.unreachable";

/** Where the issues go: restates `BUG_REPORT_REPO_DEFAULT` in crates/server/src/config.rs, which the Worker cannot import. */
export const ALERT_REPO = "Jlore-Studios/JackiOh";

/** GitHub's REST API: restates `BUG_REPORT_API_URL` in crates/server/src/config.rs. */
export const GITHUB_API_URL = "https://api.github.com";

/** The labels of an alert's issue: restates `ALERT_ISSUE_LABELS` in crates/server/src/config.rs. */
export const ALERT_ISSUE_LABELS = ["architecture", "human"];

/** Resend's sending endpoint: restates `ALERT_EMAIL_API_URL` in crates/server/src/config.rs. */
export const ALERT_EMAIL_API_URL = "https://api.resend.com/emails";

/** The sender of an alert's email: restates `ALERT_EMAIL_FROM` in crates/server/src/config.rs. */
export const ALERT_EMAIL_FROM = "JackiOh alerts <no-reply@jackioh.com>";

/** The least time between two notices (an hour): restates `ALERT_WINDOW_MS` in crates/server/src/config.rs. */
export const ALERT_WINDOW_MS = 60 * 60 * 1000;

/** How long one request to GitHub or Resend may take: restates `ALERT_TIMEOUT_MS` in crates/server/src/config.rs. */
export const ALERT_TIMEOUT_MS = 10_000;

/** What the Worker names itself to GitHub, which rejects a request with no `User-Agent` (the server's is `jackioh-server`). */
export const ALERT_USER_AGENT = "jackioh-worker";

/** The site a notice names when `PUBLIC_ORIGINS` names none (`alerts.rs`' `site_of`). */
const UNNAMED_SITE = "an unnamed site";

/** The Resend status for a key it kept: another notice emailed this already. */
const RESEND_CONFLICT = 409;

/** One alert: the details the email carries and the issue never does, and when it was raised (epoch ms). */
export interface Alert {
  fields: Record<string, string>;
  at: number;
}

/** The settings a notice reads: where it goes, the site it names and the build it came from. */
export type AlertSettings = Pick<
  ServerSettings,
  "PUBLIC_ORIGINS" | "DEPLOYED_COMMIT" | "CREATE_ISSUES_TOKEN" | "ALERT_RESEND_API_KEY" | "ALERT_EMAILS"
>;

/** What became of a notice's issue (`alerts.rs`' `Issue`). */
export type Issue =
  | { kind: "notFiled" }
  | { kind: "opened"; url: string }
  | { kind: "commented"; url: string }
  | { kind: "failed" };

/**
 * The addresses of `ALERT_EMAILS` (`env.rs`' `parse_alert_emails`): comma-separated, each a name, one `@`
 * and a domain with a dot, no whitespace. An entry that is none is left out and counted in a warning, which
 * never prints one: the addresses are a secret, since the repository is public.
 */
export function parseAlertEmails(value: string | undefined): string[] {
  const entries = (value ?? "")
    .split(",")
    .map((entry) => entry.trim())
    .filter((entry) => entry !== "");
  const addresses = entries.filter((entry) => {
    if (/\s/u.test(entry)) return false;
    const at = entry.indexOf("@");
    if (at <= 0) return false;
    const domain = entry.slice(at + 1);
    return domain.includes(".") && !domain.includes("@");
  });
  if (addresses.length < entries.length) {
    console.warn(
      `ALERT_EMAILS: ${entries.length - addresses.length} of ${entries.length} entries are not email addresses and were left out`,
    );
  }
  return addresses;
}

/** Whether a notice has anywhere to go: a token, or a Resend key and an address (either alone works, as the server's). */
export function canTell(settings: AlertSettings): boolean {
  if (nonEmpty(settings.CREATE_ISSUES_TOKEN) !== undefined) return true;
  return nonEmpty(settings.ALERT_RESEND_API_KEY) !== undefined && parseAlertEmails(settings.ALERT_EMAILS).length > 0;
}

function nonEmpty(value: string | undefined): string | undefined {
  const trimmed = value?.trim();
  return trimmed === undefined || trimmed === "" ? undefined : trimmed;
}

/** The site a notice names: the host of the first browser origin (`jackioh.com`, `staging.jackioh.com`; `alerts.rs`' `site_of`). */
export function siteOf(origins: string | undefined): string {
  const first = (origins ?? "").split(",")[0]?.trim() ?? "";
  if (first === "") return UNNAMED_SITE;
  const at = first.indexOf("://");
  const host = at === -1 ? first : first.slice(at + 3);
  return host.replace(/\/+$/u, "");
}

/** The issue's title, which the next notice searches for: one open issue per site. */
export function issueTitle(site: string): string {
  return `Patch: alert ${UNREACHABLE_EVENT} on ${site}`;
}

/** What the event means, for a person reading the notice on a phone. */
const MEANING =
  "The Worker could not start the game server or got no answer from it: a boot that failed (`release` exits on a " +
  "failed migration or catalog seed), a boot slower than the Worker's wait (`START_WAIT_MS`), or the daily " +
  "`/readyz` check failing. Nobody can play until it is fixed (docs/deploy-cloudflare.md, \"The server will not start\").";

function when(build: string | undefined, alert: Alert): string {
  return `${new Date(alert.at).toISOString()} on build \`${build ?? "unknown"}\``;
}

/** The new issue's text. Public: it names no field of the alert. */
export function issueBody(site: string, build: string | undefined, alert: Alert): string {
  return (
    `The Worker for \`${site}\` raised the alert \`${UNREACHABLE_EVENT}\` at ${when(build, alert)}.\n\n${MEANING}\n\n` +
    `Its details went by email to the owners (\`ALERT_EMAILS\`) and are in the Worker's logs ` +
    `(Workers & Pages, the site's Worker, Logs, \`${UNREACHABLE_EVENT}\`). The repository is public, so ` +
    `this issue holds none of them.\n\n` +
    `While this issue is open, the alert recurring adds a comment here, at most once an hour, and sends no email. ` +
    `Close it once it is handled: the next one opens a new issue and emails again (\`apps/edge/src/alerts.ts\`, #3618).`
  );
}

/** A notice on the open issue. Public, as the issue is. */
export function commentOf(build: string | undefined, alert: Alert): string {
  return `Seen again at ${when(build, alert)}.`;
}

function issueLineOf(issue: Issue): string {
  switch (issue.kind) {
    case "opened":
      return `Issue: ${issue.url}`;
    case "commented":
      return `Issue (open already): ${issue.url}`;
    case "notFiled":
      return "Issue: none (no CREATE_ISSUES_TOKEN)";
    case "failed":
      return "Issue: GitHub failed, see `alert.issue.failed` in the Worker's logs";
  }
}

/** The email, `{ subject, text }`: private, so it holds every field of the alert. */
export function emailOf(
  site: string,
  build: string | undefined,
  alert: Alert,
  issue: Issue,
): { subject: string; text: string } {
  const subject = `JackiOh alert on ${site}: ${UNREACHABLE_EVENT}`;
  const issueLine = issueLineOf(issue);
  const entries = Object.entries(alert.fields);
  const details = entries.length === 0 ? "  (none)\n" : entries.map(([name, value]) => `  ${name}: ${value}\n`).join("");
  const text =
    `The Worker for ${site} raised the alert ${UNREACHABLE_EVENT} at ${when(build, alert)}.\n\n${MEANING}\n\n` +
    `${issueLine}\n\nDetails:\n${details}\n` +
    `The Worker's logs hold every line: Workers & Pages, the site's Worker, Logs, ${UNREACHABLE_EVENT}.\n`;
  return { subject, text };
}

/**
 * The email's `Idempotency-Key`, which Resend keeps a day and sends once: one email per new issue, and
 * otherwise one per site per UTC day (`alerts.rs`' `email_key`). Without it a boot loop that GitHub cannot
 * record would email at every notice, spending the 100 a day the sign-in mail needs.
 */
export function emailKey(site: string, alert: Alert, issue: Issue): string {
  if (issue.kind === "opened") return `jackioh-alert/${site}/${UNREACHABLE_EVENT}/${issue.url}`;
  return `jackioh-alert/${site}/${UNREACHABLE_EVENT}/${new Date(alert.at).toISOString().slice(0, 10)}`;
}

/** A GitHub request's JSON answer, or a thrown `Error` that names the status and never the token. */
async function github(token: string, path: string, body?: unknown): Promise<unknown> {
  const response = await fetch(`${GITHUB_API_URL}${path}`, {
    method: body === undefined ? "GET" : "POST",
    headers: {
      authorization: `Bearer ${token}`,
      accept: "application/vnd.github+json",
      "user-agent": ALERT_USER_AGENT,
      "x-github-api-version": "2022-11-28",
      ...(body === undefined ? {} : { "content-type": "application/json" }),
    },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
    signal: AbortSignal.timeout(ALERT_TIMEOUT_MS),
  });
  if (!response.ok) throw new Error(`GitHub answered ${response.status}`);
  return await response.json();
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function urlOf(answer: unknown): string {
  return isRecord(answer) && typeof answer.html_url === "string" ? answer.html_url : "";
}

/** Comments on the site's open issue, or opens one; a failure is logged as `alert.issue.failed`, never raised. */
async function fileIssue(token: string, site: string, build: string | undefined, alert: Alert): Promise<Issue> {
  try {
    const title = issueTitle(site);
    const query = new URLSearchParams({
      q: `repo:${ALERT_REPO} is:issue is:open in:title "${title}"`,
      per_page: "10",
    });
    const found = await github(token, `/search/issues?${query.toString()}`);
    const items = isRecord(found) && Array.isArray(found.items) ? (found.items as unknown[]) : [];
    // The search matches words, so only an issue with exactly this title is the event's.
    const open = items.find((item) => isRecord(item) && item.title === title);
    let issue: Issue;
    if (isRecord(open)) {
      if (typeof open.number !== "number") throw new Error("GitHub answered an issue with no number");
      await github(token, `/repos/${ALERT_REPO}/issues/${open.number}/comments`, { body: commentOf(build, alert) });
      issue = { kind: "commented", url: typeof open.html_url === "string" ? open.html_url : "" };
    } else {
      const created = await github(token, `/repos/${ALERT_REPO}/issues`, {
        title,
        body: issueBody(site, build, alert),
        labels: ALERT_ISSUE_LABELS,
      });
      issue = { kind: "opened", url: urlOf(created) };
    }
    console.info("alert.filed", issue);
    return issue;
  } catch (error) {
    console.error("alert.issue.failed", error instanceof Error ? error.message : String(error));
    return { kind: "failed" };
  }
}

/** Emails the alert through Resend; a failure is logged as `alert.email.failed`, never raised. */
async function sendEmail(
  key: string,
  to: string[],
  site: string,
  build: string | undefined,
  alert: Alert,
  issue: Issue,
): Promise<void> {
  try {
    const { subject, text } = emailOf(site, build, alert, issue);
    const response = await fetch(ALERT_EMAIL_API_URL, {
      method: "POST",
      headers: {
        authorization: `Bearer ${key}`,
        "user-agent": ALERT_USER_AGENT,
        "content-type": "application/json",
        "idempotency-key": emailKey(site, alert, issue),
      },
      body: JSON.stringify({ from: ALERT_EMAIL_FROM, to, subject, text }),
      signal: AbortSignal.timeout(ALERT_TIMEOUT_MS),
    });
    if (response.ok) console.info("alert.emailed", to.length);
    // Resend kept the key: another notice emailed this already (`emailKey`).
    else if (response.status === RESEND_CONFLICT) console.info("alert.email.repeat");
    else console.error("alert.email.failed", response.status);
  } catch (error) {
    console.error("alert.email.failed", error instanceof Error ? error.message : String(error));
  }
}

/**
 * Tells people the Worker cannot reach the server: the issue (a comment while one is open), then the email
 * unless a comment carried the notice. The alert's fields go to the Worker's log first, which the issue
 * points to. Never rejects: each step logs its own failure.
 */
export async function tell(settings: AlertSettings, alert: Alert): Promise<void> {
  console.error(UNREACHABLE_EVENT, alert.fields);
  const site = siteOf(settings.PUBLIC_ORIGINS);
  const build = nonEmpty(settings.DEPLOYED_COMMIT);
  const token = nonEmpty(settings.CREATE_ISSUES_TOKEN);
  const issue: Issue = token === undefined ? { kind: "notFiled" } : await fileIssue(token, site, build, alert);
  if (issue.kind === "commented") return;
  const key = nonEmpty(settings.ALERT_RESEND_API_KEY);
  const to = parseAlertEmails(settings.ALERT_EMAILS);
  if (key !== undefined && to.length > 0) await sendEmail(key, to, site, build, alert, issue);
}
