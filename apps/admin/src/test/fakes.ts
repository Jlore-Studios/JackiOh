// A fake `fetch` that answers by `"METHOD url"` and records every call, and the tokens the tests sign in with.

import { vi } from "vitest";

export type Call = { method: string; url: string; headers: Record<string, string>; body: unknown };

export type Reply = { status?: number; body?: unknown };

/** A route's answer, fixed or worked out from the call; `"never"` leaves the request hanging. */
export type Route = Reply | ((call: Call) => Reply) | "never";

export type FakeFetch = {
  calls: Call[];
  routes: Record<string, Route>;
  /** The calls made to `method url`. */
  sent: (method: string, url: string) => Call[];
};

export const SUPABASE = "https://project.supabase.test";
export const PUBLISHABLE_KEY = "sb_publishable_test";

/** The build's two Supabase settings, as `.env.production` sets them. */
export function stubSupabaseEnv(): void {
  vi.stubEnv("VITE_SUPABASE_URL", SUPABASE);
  vi.stubEnv("VITE_SUPABASE_PUBLISHABLE_KEY", PUBLISHABLE_KEY);
}

export function fakeFetch(routes: Record<string, Route> = {}): FakeFetch {
  const calls: Call[] = [];
  const fake: FakeFetch = {
    calls,
    routes,
    sent: (method, url) => calls.filter((call) => call.method === method && call.url === url),
  };
  vi.stubGlobal(
    "fetch",
    vi.fn((input: string, init: RequestInit = {}): Promise<Response> => {
      const method = init.method ?? "GET";
      const body = typeof init.body === "string" ? (JSON.parse(init.body) as unknown) : undefined;
      const call: Call = { method, url: input, headers: { ...(init.headers as Record<string, string>) }, body };
      calls.push(call);
      const route = fake.routes[`${method} ${input}`];
      if (route === "never") return new Promise<Response>(() => undefined);
      const reply: Reply =
        route === undefined
          ? { status: 404, body: { error: { code: "not_found", message: `no fake for ${method} ${input}` } } }
          : typeof route === "function"
            ? route(call)
            : route;
      return Promise.resolve(
        new Response(reply.body === undefined ? null : JSON.stringify(reply.body), { status: reply.status ?? 200 }),
      );
    }),
  );
  return fake;
}

/** An unsigned JWT whose payload holds `claims`, as the provider's access tokens are shaped. */
export function jwt(claims: Record<string, unknown>): string {
  const part = (value: unknown): string =>
    btoa(JSON.stringify(value)).replace(/\+/gu, "-").replace(/\//gu, "_").replace(/=+$/u, "");
  return `${part({ alg: "HS256", typ: "JWT" })}.${part(claims)}.signature`;
}

/** A provider token answer. */
export function tokens(accessToken: string, refreshToken: string, expiresIn = 3600): Reply {
  return { body: { access_token: accessToken, refresh_token: refreshToken, expires_in: expiresIn, token_type: "bearer" } };
}
