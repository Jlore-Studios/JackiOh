// A stubbed `fetch` for the Play together tests (#936): every request is recorded with its method,
// path, query, headers and JSON body, and answered by the test's handler, or by a 404 `not_found`
// when the handler has no answer. Test support only; no component imports it.
//
// A refusal carries its code twice: inside `error`, the shape `crates/server/src/api/http.rs`
// answers with and `net/api.ts` reads, and at the top level, as SPEC S6/S7 write it
// (`409 {code:"full"}`), so a wrapper reading either finds it.

import { vi } from "vitest";

export type Call = {
  method: string;
  path: string;
  query: URLSearchParams;
  /** The parsed JSON body, or undefined when the request sent none. */
  body: unknown;
  /** Lower-cased header names. */
  headers: Record<string, string>;
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

/** Replace `fetch` with the handler. Undo with `vi.unstubAllGlobals()` in an `afterEach`. */
export function stubServer(handler: Handler): StubServer {
  const calls: Call[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn((input: unknown, init?: RequestInit) => {
      const url = urlOf(input);
      const requestMethod = (input as { method?: unknown } | null)?.method;
      const method = (init?.method ?? (typeof requestMethod === "string" ? requestMethod : "GET")).toUpperCase();
      const call: Call = { method, path: url.pathname, query: url.searchParams, body: bodyOf(init), headers: headersOf(init) };
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

/** `n` as a whole number somewhere in `text` ("14" in "14 / 20", not in "114"). */
export function showsNumber(text: string | null | undefined, n: number): boolean {
  return new RegExp(`(^|[^0-9])${String(n)}([^0-9]|$)`).test(text ?? "");
}
