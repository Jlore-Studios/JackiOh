// apps/web/.env.production and apps/web/.env.staging: the production and the staging build's settings,
// committed so that no host has to hold them (promote-production.yml builds production from the first,
// deploy-staging.yml staging from the second over it, `build:staging`; check-production-bundle.mjs
// refuses a bundle without them). Each is shipped to every browser, so it may only ever hold the
// public half of the environment contract, and the hosts it names must be ones its build's CSP lets
// the page reach.

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import JSON5 from "json5";
import { describe, expect, it } from "vitest";

import { connectSources } from "../../static-pages.ts";
import { originOf } from "./navigate.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const WEB = join(HERE, "../..");
const ROOT = join(WEB, "../..");

/** `PUBLIC_ENV_VARS` in crates/server/src/env.rs, the variables a browser bundle may carry. */
const PUBLIC = [
  "VITE_SUPABASE_URL",
  "VITE_SUPABASE_PUBLISHABLE_KEY",
  "VITE_SERVER_HTTP_URL",
  "VITE_SERVER_WS_URL",
  "VITE_SITE_ORIGIN",
  "VITE_CATALOG_VERSION",
  // R666: provider names only; set once a provider is set up in the Supabase dashboard.
  "VITE_AUTH_OAUTH_PROVIDERS",
];

/**
 * Each build's file, the address it is served from, its Supabase project and the wrangler.jsonc
 * environment its server runs in. Production and staging have a project each (docs/deploy-cloudflare.md
 * section 8): JackiOh-Production and JackiOh-Staging, the original one.
 */
const BUILDS = [
  { file: ".env.production", site: "https://jackioh.com", supabase: "https://fuzpvmjdejvbjcarypyi.supabase.co", env: null },
  { file: ".env.staging", site: "https://staging.jackioh.com", supabase: "https://exmjdaswedxhnzmpzqrq.supabase.co", env: "staging" },
] as const;

type Vars = { vars?: Record<string, string> };
const wrangler = JSON5.parse<Vars & { env?: Record<string, Vars> }>(readFileSync(join(ROOT, "wrangler.jsonc"), "utf8"));

function entries(file: string): [string, string][] {
  return readFileSync(join(WEB, file), "utf8")
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "" && !line.startsWith("#"))
    .map((line) => {
      const at = line.indexOf("=");
      return [line.slice(0, at), line.slice(at + 1)];
    });
}

for (const { file, site, supabase } of BUILDS) {
  describe(`apps/web/${file}`, () => {
    it("holds only public VITE_ settings, and nothing shaped like a secret", () => {
      for (const [key, value] of entries(file)) {
        expect(PUBLIC, key).toContain(key);
        expect(value, key).not.toMatch(/sb_secret_|service_role|postgres(ql)?:\/\/|-----BEGIN/u);
      }
      // The server's own list (PUBLIC_ENV_VARS in the Rust env parser) says the same names are public.
      const server = readFileSync(join(ROOT, "crates/server/src/env.rs"), "utf8");
      for (const [key] of entries(file)) expect(server, key).toContain(`"${key}"`);
    });

    it("names its own site, its server on the site's own origin, and the Supabase project", () => {
      const values = Object.fromEntries(entries(file));
      expect(values["VITE_SITE_ORIGIN"]).toBe(site);
      // wrangler.jsonc sends /api/* and /ws/* to the server, on the address the site is served from.
      expect(values["VITE_SERVER_HTTP_URL"]).toBe(site);
      expect(values["VITE_SERVER_WS_URL"]).toBe(`wss://${new URL(site).host}/ws/match`);
      expect(values["VITE_SUPABASE_URL"]).toBe(supabase);
      // The publishable key, which Supabase makes to ship to browsers; a secret key fails the test above.
      expect(values["VITE_SUPABASE_PUBLISHABLE_KEY"]).toMatch(/^sb_publishable_[A-Za-z0-9_-]+$/u);
    });

    it("names only hosts its build's CSP lets the page reach: its own origin ('self'), its sockets and Supabase", () => {
      const values = Object.fromEntries(entries(file));
      const origin = originOf(values["VITE_SITE_ORIGIN"]);
      const sources = connectSources(origin, values["VITE_SUPABASE_URL"]).split(" ");
      expect(new URL(values["VITE_SERVER_HTTP_URL"] ?? "").origin, "the API is 'self'").toBe(origin);
      expect(sources).toContain("'self'");
      for (const key of ["VITE_SUPABASE_URL", "VITE_SERVER_WS_URL"]) {
        const reached = new URL(values[key] ?? "").origin;
        expect(sources, `${key}: connect-src must allow ${reached}`).toContain(reached);
      }
    });
  });
}

describe("the two builds' Supabase projects", () => {
  it("are two, so nothing staging writes reaches a player's account", () => {
    const [production, staging] = BUILDS.map(({ file }) => Object.fromEntries(entries(file))["VITE_SUPABASE_URL"]);
    expect(production).not.toBe(staging);
  });

  it("are each its own server's, in wrangler.jsonc: the server checks sign-ins against the project the page signed in with", () => {
    for (const { file, env } of BUILDS) {
      const vars = (env === null ? wrangler : wrangler.env?.[env])?.vars ?? {};
      expect(vars["SUPABASE_URL"], file).toBe(Object.fromEntries(entries(file))["VITE_SUPABASE_URL"]);
    }
  });
});

describe("R1933000 production's card statistics hold live games alone", () => {
  const WORKFLOWS = join(ROOT, ".github/workflows");
  const text = (file: string): string => readFileSync(join(WORKFLOWS, file), "utf8");
  const workflows = readdirSync(WORKFLOWS).filter((file) => file.endsWith(".yml"));

  it("files development records only from the development lanes, through staging's STATS_IMPORT_DATABASE_URL", () => {
    const importers = workflows.filter((file) => text(file).includes("jackioh-server stats-import"));
    expect(importers.sort()).toEqual(["gather.yml", "learn.yml"]);
    for (const file of importers) {
      const secrets = [...text(file).matchAll(/DATABASE_URL: \$\{\{ secrets\.([A-Z_]+) \}\}/gu)].map((match) => match[1]);
      expect(secrets.length, file).toBeGreaterThan(0);
      expect(new Set(secrets), file).toEqual(new Set(["STATS_IMPORT_DATABASE_URL"]));
    }
  });

  it("reads production through STATS_DATABASE_URL, which no workflow imports through", () => {
    const readers = workflows.filter((file) => /\bSTATS_DATABASE_URL\b/u.test(text(file)));
    expect(readers.sort()).toEqual(["db-size.yml", "winrates-refresh.yml"]);
    for (const file of readers) expect(text(file), file).not.toContain("stats-import");
  });
});

describe("the staging build", () => {
  it("sets every key the production file sets, so none of production's values reaches staging", () => {
    const staging = new Set(entries(".env.staging").map(([key]) => key));
    for (const [key] of entries(".env.production")) expect(staging.has(key), key).toBe(true);
  });
});

describe("the builds' modes", () => {
  const scripts = (JSON.parse(readFileSync(join(WEB, "package.json"), "utf8")) as { scripts: Record<string, string> }).scripts;

  it("keep both deployed builds in production mode: staging's puts .env.staging in the environment, where Vite never overwrites a variable", () => {
    expect(scripts["build"]).toBe("vite build");
    expect(scripts["build:staging"]).toBe("node --env-file=.env.staging node_modules/vite/bin/vite.js build");
    // The module a player loads, the same for both.
    expect(scripts["prebuild:staging"]).toBe(scripts["prebuild"]);
  });

  it("keep both files out of every other mode: the e2e build and the dev server never read them", () => {
    expect(scripts["build:e2e"]).toBe("vite build --mode development");
    expect(scripts["dev"]).toBe("vite");
  });
});
