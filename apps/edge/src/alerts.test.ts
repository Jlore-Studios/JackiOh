// Holds what the Worker tells people when it cannot reach the server (#3618), with GitHub and Resend faked
// (network.fake.ts): the notice's text and key, and the calls `tell` makes.

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import {
  ALERT_EMAIL_API_URL,
  ALERT_EMAIL_FROM,
  ALERT_ISSUE_LABELS,
  ALERT_REPO,
  ALERT_TIMEOUT_MS,
  ALERT_USER_AGENT,
  UNREACHABLE_EVENT,
  canTell,
  commentOf,
  emailKey,
  emailOf,
  issueBody,
  issueTitle,
  parseAlertEmails,
  siteOf,
  tell,
  type Alert,
  type AlertSettings,
  type Issue,
} from "./alerts.ts";
import { FakeNetwork } from "./network.fake.ts";

const SITE = "jackioh.com";
const BUILD = "abc123";
const AT = Date.parse("2026-11-15T03:00:00Z");
const SECRET = "profile-0b6f-secret";
const ALERT: Alert = { fields: { cause: "start", message: SECRET }, at: AT };
const SETTINGS: AlertSettings = {
  PUBLIC_ORIGINS: `https://${SITE}`,
  DEPLOYED_COMMIT: BUILD,
  CREATE_ISSUES_TOKEN: "ghp_token",
  ALERT_RESEND_API_KEY: "re_key",
  ALERT_EMAILS: "owner@example.com, second@example.com",
};
const TITLE = `Patch: alert server.unreachable on ${SITE}`;

let network: FakeNetwork;
let logs: Record<"info" | "warn" | "error", ReturnType<typeof vi.spyOn>>;

beforeEach(() => {
  network = new FakeNetwork();
  vi.stubGlobal("fetch", network.fetch);
  logs = {
    info: vi.spyOn(console, "info").mockImplementation(() => undefined),
    warn: vi.spyOn(console, "warn").mockImplementation(() => undefined),
    error: vi.spyOn(console, "error").mockImplementation(() => undefined),
  };
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
});

describe("parseAlertEmails", () => {
  it("keeps the addresses, trimmed, and leaves out what is none, counting without printing", () => {
    const kept = parseAlertEmails(" a@example.com , nobody, b c@example.com, d@nodot, e@@example.com, @example.com,, f@x.org ");

    expect(kept).toEqual(["a@example.com", "f@x.org"]);
    expect(logs.warn).toHaveBeenCalledOnce();
    expect(logs.warn).toHaveBeenCalledWith(
      "ALERT_EMAILS: 5 of 7 entries are not email addresses and were left out",
    );
  });

  it("says nothing when every entry is an address, or there are none", () => {
    expect(parseAlertEmails("a@example.com")).toEqual(["a@example.com"]);
    expect(parseAlertEmails(undefined)).toEqual([]);
    expect(parseAlertEmails(" , ")).toEqual([]);
    expect(logs.warn).not.toHaveBeenCalled();
  });
});

describe("canTell", () => {
  it("is true with a token alone, or with a Resend key and an address", () => {
    expect(canTell({ CREATE_ISSUES_TOKEN: "t" })).toBe(true);
    expect(canTell({ ALERT_RESEND_API_KEY: "k", ALERT_EMAILS: "a@example.com" })).toBe(true);
  });

  it("is false with nothing, a blank token, or a key and no address, or an address and no key", () => {
    expect(canTell({})).toBe(false);
    expect(canTell({ CREATE_ISSUES_TOKEN: "  " })).toBe(false);
    expect(canTell({ ALERT_RESEND_API_KEY: "k" })).toBe(false);
    expect(canTell({ ALERT_RESEND_API_KEY: "k", ALERT_EMAILS: "not an address" })).toBe(false);
    expect(canTell({ ALERT_RESEND_API_KEY: " ", ALERT_EMAILS: "a@example.com" })).toBe(false);
  });
});

describe("siteOf", () => {
  it("names the host of the first origin", () => {
    expect(siteOf("https://jackioh.com")).toBe("jackioh.com");
    expect(siteOf("https://staging.jackioh.com/")).toBe("staging.jackioh.com");
    expect(siteOf(" https://jackioh.com , https://www.jackioh.com")).toBe("jackioh.com");
    expect(siteOf("jackioh.com")).toBe("jackioh.com");
  });

  it("names an unnamed site when there is no origin", () => {
    expect(siteOf(undefined)).toBe("an unnamed site");
    expect(siteOf("")).toBe("an unnamed site");
    expect(siteOf(" ,https://jackioh.com")).toBe("an unnamed site");
  });
});

describe("the issue's text", () => {
  it("is titled as the server's alert is, for this event and site", () => {
    expect(issueTitle(SITE)).toBe("Patch: alert server.unreachable on jackioh.com");
    expect(UNREACHABLE_EVENT).toBe("server.unreachable");
  });

  it("names the event, site, build and time, and none of the alert's fields", () => {
    const body = issueBody(SITE, BUILD, ALERT);

    expect(body).toContain("server.unreachable");
    expect(body).toContain(SITE);
    expect(body).toContain("2026-11-15T03:00:00.000Z on build `abc123`");
    expect(body).toContain("`START_WAIT_MS`");
    expect(body).toContain("holds none of them");
    expect(body).not.toContain(SECRET);
    expect(body).not.toContain("cause");
  });

  it("says unknown for a build the Worker was not given", () => {
    expect(issueBody(SITE, undefined, ALERT)).toContain("on build `unknown`");
    expect(commentOf(undefined, ALERT)).toBe("Seen again at 2026-11-15T03:00:00.000Z on build `unknown`.");
  });

  it("makes a comment of the time and build alone", () => {
    const comment = commentOf(BUILD, ALERT);

    expect(comment).toBe("Seen again at 2026-11-15T03:00:00.000Z on build `abc123`.");
    expect(comment).not.toContain(SECRET);
  });
});

describe("the email's text", () => {
  it("holds every field and a line for the issue", () => {
    const { subject, text } = emailOf(SITE, BUILD, ALERT, { kind: "opened", url: "https://github.com/x/1" });

    expect(subject).toBe("JackiOh alert on jackioh.com: server.unreachable");
    expect(text).toContain("The Worker for jackioh.com raised the alert server.unreachable at 2026-11-15T03:00:00.000Z");
    expect(text).toContain("Issue: https://github.com/x/1");
    expect(text).toContain("Details:\n  cause: start\n  message: " + SECRET + "\n");
    expect(text).toContain("The Worker's logs hold every line");
  });

  it("says what became of the issue, whichever it was", () => {
    const line = (issue: Issue) => emailOf(SITE, BUILD, ALERT, issue).text;

    expect(line({ kind: "commented", url: "https://github.com/x/2" })).toContain("Issue (open already): https://github.com/x/2");
    expect(line({ kind: "notFiled" })).toContain("Issue: none (no CREATE_ISSUES_TOKEN)");
    expect(line({ kind: "failed" })).toContain("Issue: GitHub failed, see `alert.issue.failed` in the Worker's logs");
  });

  it("says none for an alert with no fields", () => {
    expect(emailOf(SITE, BUILD, { fields: {}, at: AT }, { kind: "notFiled" }).text).toContain("Details:\n  (none)\n");
  });
});

describe("emailKey", () => {
  it("is per new issue", () => {
    expect(emailKey(SITE, ALERT, { kind: "opened", url: "https://github.com/x/1" })).toBe(
      "jackioh-alert/jackioh.com/server.unreachable/https://github.com/x/1",
    );
  });

  it("is per site and UTC day when no issue is new", () => {
    const later = { ...ALERT, at: Date.parse("2026-11-15T23:59:59Z") };
    const next = { ...ALERT, at: Date.parse("2026-11-16T00:00:00Z") };
    for (const issue of [{ kind: "notFiled" }, { kind: "failed" }, { kind: "commented", url: "u" }] satisfies Issue[]) {
      expect(emailKey(SITE, ALERT, issue)).toBe("jackioh-alert/jackioh.com/server.unreachable/2026-11-15");
      expect(emailKey(SITE, later, issue)).toBe(emailKey(SITE, ALERT, issue));
      expect(emailKey(SITE, next, issue)).toBe("jackioh-alert/jackioh.com/server.unreachable/2026-11-16");
    }
  });
});

describe("tell, with a token and a Resend key", () => {
  it("opens one issue and sends one email, with the server's title, labels and key", async () => {
    await tell(SETTINGS, ALERT);

    expect(network.calls).toHaveLength(3);
    const [search] = network.to("search");
    expect(search?.method).toBe("GET");
    expect(search?.url.searchParams.get("q")).toBe(`repo:${ALERT_REPO} is:issue is:open in:title "${TITLE}"`);
    expect(search?.url.searchParams.get("per_page")).toBe("10");

    const [create] = network.to("create");
    expect(create?.method).toBe("POST");
    expect(create?.url.pathname).toBe("/repos/Jlore-Studios/JackiOh/issues");
    expect(create?.body).toEqual({ title: TITLE, body: issueBody(SITE, BUILD, ALERT), labels: ["architecture", "human"] });
    expect(ALERT_ISSUE_LABELS).toEqual(["architecture", "human"]);
    expect(network.to("comment")).toHaveLength(0);

    const [email] = network.to("email");
    expect(email?.url.href).toBe(ALERT_EMAIL_API_URL);
    expect(email?.method).toBe("POST");
    expect(email?.headers.authorization).toBe("Bearer re_key");
    expect(email?.headers["idempotency-key"]).toBe(`jackioh-alert/${SITE}/server.unreachable/${FakeNetwork.issueUrl(1)}`);
    expect(email?.headers["user-agent"]).toBe(ALERT_USER_AGENT);
    expect(email?.body).toEqual({
      from: ALERT_EMAIL_FROM,
      to: ["owner@example.com", "second@example.com"],
      subject: "JackiOh alert on jackioh.com: server.unreachable",
      text: emailOf(SITE, BUILD, ALERT, { kind: "opened", url: FakeNetwork.issueUrl(1) }).text,
    });
  });

  it("sends GitHub the token, a user agent and the API version, and bounds each call", async () => {
    const timeout = vi.spyOn(AbortSignal, "timeout");

    await tell(SETTINGS, ALERT);

    for (const call of [...network.to("search"), ...network.to("create")]) {
      expect(call.headers.authorization).toBe("Bearer ghp_token");
      expect(call.headers["user-agent"]).toBe(ALERT_USER_AGENT);
      expect(call.headers.accept).toBe("application/vnd.github+json");
      expect(call.headers["x-github-api-version"]).toBe("2022-11-28");
    }
    expect(network.to("create")[0]?.headers["content-type"]).toBe("application/json");
    expect(network.to("search")[0]?.headers["content-type"]).toBeUndefined();
    expect(timeout).toHaveBeenCalledTimes(3);
    expect(timeout).toHaveBeenCalledWith(ALERT_TIMEOUT_MS);
  });

  it("logs the alert's fields for the issue to point to, and never the secrets", async () => {
    await tell(SETTINGS, ALERT);

    expect(logs.error).toHaveBeenCalledWith("server.unreachable", ALERT.fields);
    const printed = JSON.stringify([...logs.info.mock.calls, ...logs.warn.mock.calls, ...logs.error.mock.calls]);
    expect(printed).not.toContain("ghp_token");
    expect(printed).not.toContain("re_key");
    expect(printed).not.toContain("owner@example.com");
  });

  it("comments, and sends no email, on the open issue with exactly the title", async () => {
    network.openIssues = [
      { number: 9, title: `${TITLE} (older)`, html_url: "https://github.com/x/9" },
      { number: 7, title: TITLE, html_url: "https://github.com/x/7" },
    ];

    await tell(SETTINGS, ALERT);

    expect(network.to("create")).toHaveLength(0);
    expect(network.to("email")).toHaveLength(0);
    const [comment] = network.to("comment");
    expect(comment?.url.pathname).toBe("/repos/Jlore-Studios/JackiOh/issues/7/comments");
    expect(comment?.body).toEqual({ body: commentOf(BUILD, ALERT) });
  });

  it("ignores an issue whose title only holds the words, and opens a new one", async () => {
    network.openIssues = [{ number: 9, title: `Re: ${TITLE}`, html_url: "https://github.com/x/9" }];

    await tell(SETTINGS, ALERT);

    expect(network.to("comment")).toHaveLength(0);
    expect(network.to("create")).toHaveLength(1);
    expect(network.to("email")).toHaveLength(1);
  });

  it("falls back to the day's email when GitHub answers an error, and never throws", async () => {
    network.status.create = 500;

    await expect(tell(SETTINGS, ALERT)).resolves.toBeUndefined();

    expect(logs.error).toHaveBeenCalledWith("alert.issue.failed", "GitHub answered 500");
    expect(network.to("email")[0]?.headers["idempotency-key"]).toBe(`jackioh-alert/${SITE}/server.unreachable/2026-11-15`);
    expect(network.to("email")[0]?.body?.text).toContain("Issue: GitHub failed");
  });

  it("falls back to the day's email when the search fails", async () => {
    network.status.search = 403;

    await tell(SETTINGS, ALERT);

    expect(network.to("create")).toHaveLength(0);
    expect(logs.error).toHaveBeenCalledWith("alert.issue.failed", "GitHub answered 403");
    expect(network.to("email")).toHaveLength(1);
  });

  it("falls back when the comment fails: the notice goes by email too", async () => {
    network.openIssues = [{ number: 7, title: TITLE, html_url: "https://github.com/x/7" }];
    network.status.comment = 500;

    await tell(SETTINGS, ALERT);

    expect(network.to("email")).toHaveLength(1);
    expect(network.to("email")[0]?.body?.text).toContain("Issue: GitHub failed");
  });

  it("falls back when GitHub's open issue has no number", async () => {
    network.openIssues = [{ title: TITLE, html_url: "https://github.com/x/7" } as never];

    await tell(SETTINGS, ALERT);

    expect(network.to("comment")).toHaveLength(0);
    expect(logs.error).toHaveBeenCalledWith("alert.issue.failed", "GitHub answered an issue with no number");
    expect(network.to("email")).toHaveLength(1);
  });

  it("falls back when GitHub cannot be reached at all", async () => {
    vi.stubGlobal("fetch", (url: string) =>
      url.startsWith(ALERT_EMAIL_API_URL) ? network.fetch(url) : Promise.reject(new Error("offline")),
    );

    await expect(tell(SETTINGS, ALERT)).resolves.toBeUndefined();

    expect(logs.error).toHaveBeenCalledWith("alert.issue.failed", "offline");
    expect(network.to("email")).toHaveLength(1);
  });

  it("takes Resend's 409 for a notice already sent", async () => {
    network.status.email = 409;

    await expect(tell(SETTINGS, ALERT)).resolves.toBeUndefined();

    expect(logs.info).toHaveBeenCalledWith("alert.email.repeat");
    expect(logs.error).not.toHaveBeenCalledWith("alert.email.failed", expect.anything());
  });

  it("logs another Resend failure, with the status and never the key, and never throws", async () => {
    network.status.email = 422;

    await expect(tell(SETTINGS, ALERT)).resolves.toBeUndefined();

    expect(logs.error).toHaveBeenCalledWith("alert.email.failed", 422);
  });

  it("logs a Resend that cannot be reached, and never throws", async () => {
    vi.stubGlobal("fetch", (url: string, init?: RequestInit) =>
      url.startsWith(ALERT_EMAIL_API_URL) ? Promise.reject(new Error("offline")) : network.fetch(url, init),
    );

    await expect(tell(SETTINGS, ALERT)).resolves.toBeUndefined();

    expect(logs.error).toHaveBeenCalledWith("alert.email.failed", "offline");
  });

  it("logs a failure that is no Error by its text", async () => {
    vi.stubGlobal("fetch", () => Promise.reject("plain text"));

    await tell(SETTINGS, ALERT);

    expect(logs.error).toHaveBeenCalledWith("alert.issue.failed", "plain text");
    expect(logs.error).toHaveBeenCalledWith("alert.email.failed", "plain text");
  });
});

describe("tell, with one way to tell", () => {
  it("emails alone with no token, the day's key and a line saying no issue", async () => {
    await tell({ ...SETTINGS, CREATE_ISSUES_TOKEN: undefined }, ALERT);

    expect(network.to("search")).toHaveLength(0);
    expect(network.to("create")).toHaveLength(0);
    const [email] = network.to("email");
    expect(email?.headers["idempotency-key"]).toBe(`jackioh-alert/${SITE}/server.unreachable/2026-11-15`);
    expect(email?.body?.text).toContain("Issue: none (no CREATE_ISSUES_TOKEN)");
  });

  it("files an issue alone with no Resend key", async () => {
    await tell({ ...SETTINGS, ALERT_RESEND_API_KEY: undefined }, ALERT);

    expect(network.to("create")).toHaveLength(1);
    expect(network.to("email")).toHaveLength(0);
  });

  it("files an issue alone with no valid address", async () => {
    await tell({ ...SETTINGS, ALERT_EMAILS: "nobody" }, ALERT);

    expect(network.to("create")).toHaveLength(1);
    expect(network.to("email")).toHaveLength(0);
  });

  it("names an unnamed site and an unknown build when the Worker holds neither", async () => {
    await tell({ ...SETTINGS, PUBLIC_ORIGINS: undefined, DEPLOYED_COMMIT: undefined }, ALERT);

    expect(network.to("create")[0]?.body?.title).toBe("Patch: alert server.unreachable on an unnamed site");
    expect(network.to("create")[0]?.body?.body).toContain("on build `unknown`");
  });
});
