// A stand-in for GitHub's and Resend's APIs, for the Worker's alert tests (#3618): `vi.stubGlobal("fetch",
// network.fetch)` answers the search, an issue's creation, a comment and an email, and records each call.
// Not Worker code, so Stryker leaves it out (stryker.config.mjs).

import { ALERT_EMAIL_API_URL, ALERT_REPO, GITHUB_API_URL } from "./alerts.ts";

/** One request the Worker made. */
export interface Call {
  method: string;
  url: URL;
  headers: Record<string, string>;
  /** The JSON body, or undefined for a request with none. */
  body: Record<string, unknown> | undefined;
}

export interface OpenIssue {
  number: number;
  title: string;
  html_url: string;
}

export class FakeNetwork {
  readonly calls: Call[] = [];
  /** What the search answers: the open issues, whatever the query. */
  openIssues: OpenIssue[] = [];
  /** The status each route answers; 200 (201 for a creation) unless a test sets another. */
  status = { search: 200, create: 201, comment: 201, email: 200 };
  private opened = 0;

  /** The URL a created issue gets, by how many this network has created. */
  static issueUrl(number: number): string {
    return `https://github.com/${ALERT_REPO}/issues/${number}`;
  }

  /** The calls to one route: `search`, `create`, `comment` or `email`. */
  to(route: "search" | "create" | "comment" | "email"): Call[] {
    return this.calls.filter((call) => this.routeOf(call) === route);
  }

  private routeOf(call: Call): string {
    if (call.url.href.startsWith(ALERT_EMAIL_API_URL)) return "email";
    const path = call.url.pathname;
    if (path === "/search/issues") return "search";
    if (path.endsWith("/comments")) return "comment";
    return path === `/repos/${ALERT_REPO}/issues` ? "create" : "unknown";
  }

  readonly fetch = (input: string | URL | Request, init?: RequestInit): Promise<Response> => {
    const url = new URL(input instanceof Request ? input.url : input);
    const raw = typeof init?.body === "string" ? init.body : undefined;
    const call: Call = {
      method: init?.method ?? "GET",
      url,
      headers: Object.fromEntries(new Headers(init?.headers).entries()),
      body: raw === undefined ? undefined : (JSON.parse(raw) as Record<string, unknown>),
    };
    this.calls.push(call);
    const route = this.routeOf(call);
    if (route === "unknown" || !(url.href.startsWith(GITHUB_API_URL) || url.href.startsWith(ALERT_EMAIL_API_URL))) {
      return Promise.reject(new Error(`unexpected request to ${url.href}`));
    }
    if (route === "search") return answer(this.status.search, { items: this.openIssues });
    if (route === "comment") return answer(this.status.comment, {});
    if (route === "email") return answer(this.status.email, { id: "email-id" });
    this.opened += 1;
    return answer(this.status.create, { number: this.opened, html_url: FakeNetwork.issueUrl(this.opened) });
  };
}

function answer(status: number, body: unknown): Promise<Response> {
  return Promise.resolve(new Response(JSON.stringify(body), { status }));
}
