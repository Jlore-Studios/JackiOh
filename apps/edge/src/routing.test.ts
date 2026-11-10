import { describe, expect, it } from "vitest";

import {
  MODULE_CACHE_CONTROL,
  MODULE_ROUTES,
  SERVER_PORT,
  SERVER_ROUTES,
  WORKER_ROUTES,
  containerEnv,
  forServer,
  isModulePath,
  isServerPath,
  precompressedModule,
} from "./routing.ts";

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

describe("#3183 the engine's brotli-compressed module", () => {
  it("is the Worker's for Vite's name of a brotli'd WebAssembly module alone", () => {
    for (const path of ["/assets/jackioh_wasm_bg.wasm-BIXTgzSO.br", "/assets/jackioh_wasm_bg.wasm-a_b-C.br"]) {
      expect(isModulePath(path), path).toBe(true);
    }
    for (const path of ["/assets/jackioh_wasm_bg-abc.wasm", "/assets/index-abc.js", "/assets/x.br", "/jackioh_wasm_bg.wasm-abc.br", "/api/x.wasm-a.br"]) {
      expect(isModulePath(path), path).toBe(false);
    }
  });

  it("matches what wrangler.jsonc routes: the Worker's routes are the server's and the module's", () => {
    expect(WORKER_ROUTES).toEqual([...SERVER_ROUTES, ...MODULE_ROUTES]);
    expect(MODULE_ROUTES).toEqual(["/assets/*.wasm-*.br"]);
  });

  it("serves the stored bytes as they are, labelled brotli and WebAssembly, its other headers kept", async () => {
    const stored = new Response(new Uint8Array([0x1b, 0x2a, 0x00]), {
      headers: { "Content-Type": "application/octet-stream", ETag: '"v1"', "Cache-Control": "public, max-age=31536000, immutable" },
    });
    const served = precompressedModule(stored);
    expect(served.status).toBe(200);
    expect(served.headers.get("Content-Type")).toBe("application/wasm");
    expect(served.headers.get("Content-Encoding")).toBe("br");
    expect(served.headers.get("ETag")).toBe('"v1"');
    expect(served.headers.get("Cache-Control")).toBe("public, max-age=31536000, immutable");
    expect([...new Uint8Array(await served.arrayBuffer())]).toEqual([0x1b, 0x2a, 0x00]);
  });

  it("passes on an answer with no file to send: a 304 for a held copy, and a 404", () => {
    for (const status of [304, 404]) {
      const stored = new Response(null, { status });
      expect(precompressedModule(stored)).toBe(stored);
    }
  });

  it("adds the year-long Cache-Control when the stored answer lacks one", () => {
    const stored = new Response(new Uint8Array([0x1b]), {
      headers: { "Content-Type": "application/octet-stream", ETag: '"v1"' },
    });
    const served = precompressedModule(stored);
    expect(served.headers.get("Cache-Control")).toBe("public, max-age=31536000, immutable");
    expect(MODULE_CACHE_CONTROL).toBe("public, max-age=31536000, immutable");
    expect(served.headers.get("Content-Type")).toBe("application/wasm");
    expect(served.headers.get("Content-Encoding")).toBe("br");
  });

  it("keeps a stored Cache-Control it did not write", () => {
    const stored = new Response(new Uint8Array([0x1b]), {
      headers: { "Content-Type": "application/octet-stream", "Cache-Control": "public, max-age=60" },
    });
    expect(precompressedModule(stored).headers.get("Cache-Control")).toBe("public, max-age=60");
  });
});
