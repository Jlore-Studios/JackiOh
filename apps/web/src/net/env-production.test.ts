// apps/web/.env.production: the production build's settings, committed so that no host has to hold
// them (promote-production.yml builds from it; check-production-bundle.mjs refuses a bundle without
// them). It is shipped to every browser, so it may only ever hold the public half of the environment
// contract, and the two hosts it names must be ones the CSP lets the page reach.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const WEB = join(HERE, "../..");
const ROOT = join(WEB, "../..");

/** apps/server/src/env.ts PUBLIC_ENV_VARS, the variables a browser bundle may carry. */
const PUBLIC = [
  "VITE_SUPABASE_URL",
  "VITE_SUPABASE_PUBLISHABLE_KEY",
  "VITE_SERVER_HTTP_URL",
  "VITE_SERVER_WS_URL",
  "VITE_CATALOG_VERSION",
  // R666: provider names only; set once a provider is set up in the Supabase dashboard.
  "VITE_AUTH_OAUTH_PROVIDERS",
];

function entries(): [string, string][] {
  return readFileSync(join(WEB, ".env.production"), "utf8")
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "" && !line.startsWith("#"))
    .map((line) => {
      const at = line.indexOf("=");
      return [line.slice(0, at), line.slice(at + 1)];
    });
}

describe("apps/web/.env.production", () => {
  it("holds only public VITE_ settings, and nothing shaped like a secret", () => {
    for (const [key, value] of entries()) {
      expect(PUBLIC, key).toContain(key);
      expect(value, key).not.toMatch(/sb_secret_|service_role|postgres(ql)?:\/\/|-----BEGIN/u);
    }
    // The server's own list says the same four names are public.
    const server = readFileSync(join(ROOT, "apps/server/src/env.ts"), "utf8");
    for (const [key] of entries()) expect(server, key).toContain(`"${key}"`);
  });

  it("names the production hosts, which the CSP lets the page reach", () => {
    const values = Object.fromEntries(entries());
    expect(values["VITE_SUPABASE_URL"]).toBe("https://exmjdaswedxhnzmpzqrq.supabase.co");
    // The publishable key, which Supabase makes to ship to browsers; a secret key fails the test above.
    expect(values["VITE_SUPABASE_PUBLISHABLE_KEY"]).toMatch(/^sb_publishable_[A-Za-z0-9_-]+$/u);
    expect(values["VITE_SERVER_HTTP_URL"]).toBe("https://jackioh-server.onrender.com");
    expect(values["VITE_SERVER_WS_URL"]).toBe("wss://jackioh-server.onrender.com/ws/match");
    const csp = readFileSync(join(WEB, "public/_headers"), "utf8");
    for (const key of ["VITE_SUPABASE_URL", "VITE_SERVER_HTTP_URL", "VITE_SERVER_WS_URL"]) {
      const origin = new URL(values[key] ?? "").origin;
      expect(csp, `${key}: connect-src must allow ${origin}`).toContain(origin);
    }
  });

  it("stays out of every mode but production: the e2e build and the dev server never read it", () => {
    const scripts = (JSON.parse(readFileSync(join(WEB, "package.json"), "utf8")) as { scripts: Record<string, string> }).scripts;
    expect(scripts["build:e2e"]).toBe("vite build --mode development");
    expect(scripts["dev"]).toBe("vite");
  });
});
