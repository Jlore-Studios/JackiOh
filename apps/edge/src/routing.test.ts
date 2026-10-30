import { describe, expect, it } from "vitest";

import { SERVER_PORT, containerEnv, forServer, isServerPath } from "./routing.ts";

describe("isServerPath", () => {
  it("sends the API and the sockets to the server", () => {
    for (const path of ["/api/catalog", "/api/decks/abc", "/ws/match", "/ws/duos", "/ws/watch"]) {
      expect(isServerPath(path), path).toBe(true);
    }
  });

  it("leaves the client's screens and files to the assets, the ones that only start alike included", () => {
    for (const path of ["/", "/login", "/watch/abc", "/match/abc", "/apiary", "/wsx", "/api", "/assets/index.js"]) {
      expect(isServerPath(path), path).toBe(false);
    }
  });
});

describe("forServer", () => {
  it("R190: replaces what the caller wrote in X-Forwarded-For with the address Cloudflare saw", () => {
    const request = new Request("https://jackioh.com/api/queue", {
      method: "POST",
      headers: { "X-Forwarded-For": "203.0.113.9, 198.51.100.1", "CF-Connecting-IP": "192.0.2.7" },
      body: "{}",
    });
    const forwarded = forServer(request);
    expect(forwarded.headers.get("X-Forwarded-For")).toBe("192.0.2.7");
    expect(forwarded.method).toBe("POST");
    expect(forwarded.url).toBe("https://jackioh.com/api/queue");
  });

  it("R190: sends no X-Forwarded-For when Cloudflare named no address, so the server keys on its peer", () => {
    const request = new Request("https://jackioh.com/api/catalog", { headers: { "X-Forwarded-For": "203.0.113.9" } });
    expect(forServer(request).headers.has("X-Forwarded-For")).toBe(false);
  });

  it("keeps a WebSocket upgrade's headers", () => {
    const request = new Request("https://jackioh.com/ws/match", {
      headers: { Upgrade: "websocket", "Sec-WebSocket-Protocol": "jackioh.v1", Origin: "https://jackioh.com" },
    });
    const forwarded = forServer(request);
    expect(forwarded.headers.get("Upgrade")).toBe("websocket");
    expect(forwarded.headers.get("Sec-WebSocket-Protocol")).toBe("jackioh.v1");
    expect(forwarded.headers.get("Origin")).toBe("https://jackioh.com");
  });
});

describe("containerEnv", () => {
  it("passes the server's settings, the deployed commit, its port and production's NODE_ENV", () => {
    const vars = containerEnv({
      PUBLIC_ORIGINS: "https://jackioh.com",
      SUPABASE_URL: "https://example.supabase.co",
      TRUSTED_PROXY_HOPS: "1",
      LADDER_BOTS: "on",
      SUPABASE_SECRET_KEY: "secret",
      DATABASE_URL: "postgresql://example",
      CODE_PEPPER: "pepper",
      DEPLOYED_COMMIT: "0123456789abcdef0123456789abcdef01234567",
    });
    expect(vars).toEqual({
      NODE_ENV: "production",
      PORT: String(SERVER_PORT),
      PUBLIC_ORIGINS: "https://jackioh.com",
      SUPABASE_URL: "https://example.supabase.co",
      TRUSTED_PROXY_HOPS: "1",
      LADDER_BOTS: "on",
      SUPABASE_SECRET_KEY: "secret",
      DATABASE_URL: "postgresql://example",
      CODE_PEPPER: "pepper",
      DEPLOYED_COMMIT: "0123456789abcdef0123456789abcdef01234567",
    });
  });

  it("leaves out an optional secret that is unset or empty, and anything that is not a server setting", () => {
    const settings = { CREATE_ISSUES_TOKEN: "", SERVER: "a binding" } as Parameters<typeof containerEnv>[0];
    expect(Object.keys(containerEnv(settings)).sort()).toEqual(["NODE_ENV", "PORT"]);
  });
});
