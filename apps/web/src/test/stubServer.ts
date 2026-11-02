// A stubbed `fetch` for the Play together tests (#936): each request is recorded and answered by the
// test's handler, else by a 404 `not_found`; beside it, the helpers most web tests share (#1799). A
// refusal carries its code both inside `error` (as `crates/server/src/api/http.rs` answers and
// `net/api.ts` reads) and at the top level (SPEC S6/S7's `409 {code:"full"}`), for either reader.

import { vi } from "vitest";

import type { MeResponse } from "../net/api.ts";
import { E2E_SESSION_STORAGE_KEY } from "../net/session.ts";

export type Call = {
  method: string;
  path: string;
  query: URLSearchParams;
  /** The parsed JSON body, or undefined when the request sent none. */
  body: unknown;
  /** Lower-cased header names. */
  headers: Record<string, string>;
  /** R2119000: sent as `fetch`'s `keepalive`, to outlive the page. */
  keepalive: boolean;
};

export type Reply = { status: number; body?: unknown };

export type Handler = (call: Call) => Reply | undefined;

export type StubServer = {
  calls: Call[];
  /** The calls made to exactly this method and path (no query). */
  to: (method: string, path: string) => Call[];
};

export function jsonResponse(status: number, body?: unknown): Response {
  const text = body === undefined ? "" : JSON.stringify(body);
  return {
    ok: status >= 200 && status < 300,
    status,
    statusText: String(status),
    headers: new Headers({ "content-type": "application/json" }),
    json: () => (text === "" ? Promise.reject(new SyntaxError("empty body")) : Promise.resolve(JSON.parse(text) as unknown)),
    text: () => Promise.resolve(text),
    clone: () => jsonResponse(status, body),
  } as unknown as Response;
}

export function refusal(status: number, code: string, message: string): Reply {
  return { status, body: { code, message, error: { code, message } } };
}

function urlOf(input: unknown): URL {
  if (typeof input === "string") return new URL(input, "http://localhost");
  if (input instanceof URL) return input;
  const url = (input as { url?: unknown } | null)?.url;
  return new URL(typeof url === "string" ? url : String(input), "http://localhost");
}

function headersOf(init: RequestInit | undefined): Record<string, string> {
  const out: Record<string, string> = {};
  if (init?.headers === undefined) return out;
  new Headers(init.headers).forEach((value, key) => {
    out[key.toLowerCase()] = value;
  });
  return out;
}

function bodyOf(init: RequestInit | undefined): unknown {
  const raw = init?.body;
  if (typeof raw !== "string" || raw.length === 0) return undefined;
  try {
    return JSON.parse(raw) as unknown;
  } catch {
    return raw;
  }
}

/** Undo with `vi.unstubAllGlobals()` in an `afterEach`. */
export function stubServer(handler: Handler): StubServer {
  const calls: Call[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown, init?: RequestInit) => {
      const url = urlOf(input);
      const requestMethod = (input as { method?: unknown } | null)?.method;
      const method = (init?.method ?? (typeof requestMethod === "string" ? requestMethod : "GET")).toUpperCase();
      const call: Call = {
        method,
        path: url.pathname,
        query: url.searchParams,
        body: bodyOf(init),
        headers: headersOf(init),
        keepalive: init?.keepalive === true,
      };
      calls.push(call);
      const reply = handler(call) ?? refusal(404, "not_found", `no stub for ${method} ${url.pathname}`);
      return Promise.resolve(jsonResponse(reply.status, reply.body));
    }),
  );
  return {
    calls,
    to: (method, path) => calls.filter((call) => call.method === method.toUpperCase() && call.path === path),
  };
}

export function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`missing: ${what}`);
  return value;
}

/** Move the page to `path` without a navigation or a history entry. */
export function at(path: string): void {
  window.history.replaceState(null, "", path);
}

/** Sign a test player in with the e2e session the M8 specs seed (`net/session.ts`, e2e/README.md A6). */
export function signIn(token = "e2e-token"): void {
  window.localStorage.setItem(E2E_SESSION_STORAGE_KEY, JSON.stringify({ accessToken: token }));
}

/** `GET /api/auth/me`'s answer: an active, verified player in no match, but for what `over` replaces. */
export function meBody(over: Partial<MeResponse> = {}): MeResponse {
  return {
    profile: { id: "profile-1", status: "active" },
    needsInviteCode: false,
    emailVerified: true,
    currentMatchId: null,
    email: "player@example.test",
    ...over,
  };
}

/** `n` as a whole number somewhere in `text` ("14" in "14 / 20", not in "114"). */
export function showsNumber(text: string | null | undefined, n: number): boolean {
  return new RegExp(`(^|[^0-9])${String(n)}([^0-9]|$)`).test(text ?? "");
}

/**
 * Matches the innermost elements whose whole text holds `text`: words the page draws across elements,
 * such as a username's base and tag (`auth/Username.tsx`, R1436), which `getByText` never finds.
 */
export function wholeText(text: string): (content: string, element: Element | null) => boolean {
  return (_content, element) =>
    element !== null &&
    (element.textContent ?? "").includes(text) &&
    !Array.from(element.children).some((child) => (child.textContent ?? "").includes(text));
}
