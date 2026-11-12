// apps/web/.env.production and .env.staging, committed so no host holds the builds' settings
// (promote-production.yml and deploy-staging.yml build from them). Each ships to every browser, so it may
// hold only public variables, and the hosts it names must be ones its build's CSP lets the page reach.

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import JSON5 from "json5";
import { describe, expect, it } from "vitest";

import { connectSources } from "../../static-pages.ts";
import { GOOGLE_CLIENT_ID_PATTERN } from "./auth.ts";
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
  // R2983000: Google's web client ID, which Google's own button ships to every page; never its secret.
  "VITE_GOOGLE_CLIENT_ID",
];

/** What a secret looks like: Supabase's secret key, a service role, a database URL, a key file, Google's client secret. */
const SECRET = /sb_secret_|service_role|postgres(ql)?:\/\/|-----BEGIN|GOCSPX-/u;

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
        expect(value, key).not.toMatch(SECRET);
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

describe("R2983000 the builds' Google client ID", () => {
  const CLIENT_ID = "1234567890-abcdefghijklmnop.apps.googleusercontent.com";

  it("R2983000 each build names one, empty or shaped like a Google client ID", () => {
    for (const { file } of BUILDS) {
      const values = Object.fromEntries(entries(file));
      expect(Object.keys(values), file).toContain("VITE_GOOGLE_CLIENT_ID");
      const value = values["VITE_GOOGLE_CLIENT_ID"] ?? "";
      // Empty until a person fills it in from issue #2983: R666's redirect signs in meanwhile.
      if (value !== "") expect(value, file).toMatch(GOOGLE_CLIENT_ID_PATTERN);
    }
  });

  it("R2983000 refuses anything shaped like a secret", () => {
    for (const secret of ["GOCSPX-abcdefghijklmnop", "sb_secret_abc", `${CLIENT_ID} GOCSPX-abc`, `${CLIENT_ID}x=1`, " "]) {
      expect(GOOGLE_CLIENT_ID_PATTERN.test(secret), secret).toBe(false);
    }
    expect("GOCSPX-abcdefghijklmnop").toMatch(SECRET);
    expect(GOOGLE_CLIENT_ID_PATTERN.test(CLIENT_ID)).toBe(true);
    expect(CLIENT_ID).not.toMatch(SECRET);
  });
});

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

const WORKFLOWS = join(ROOT, ".github/workflows");
const workflowText = (file: string): string => readFileSync(join(WORKFLOWS, file), "utf8");
const workflowFiles = readdirSync(WORKFLOWS).filter((file) => file.endsWith(".yml"));

/** training/file-records.sh: every development filing, gather's, the learn lane's shards' and its import's. */
const FILING = join(ROOT, "training/file-records.sh");
const runsFiling = (text: string): boolean => /\bsh "?\S*training\/file-records\.sh"? /u.test(text);

describe("R1933001 the development lanes file into both projects' card statistics", () => {
  it("files development records only from the development lanes, through staging's and production's import logins", () => {
    const importers = workflowFiles.filter((file) => workflowText(file).includes("jackioh-server stats-import") || runsFiling(workflowText(file)));
    expect(importers.sort()).toEqual(["gather.yml", "learn.yml", "selfplay-shards.yml"]);
    for (const file of importers) {
      const secrets = [...workflowText(file).matchAll(/DATABASE_URL: \$\{\{ secrets\.([A-Z_]+) \}\}/gu)].map((match) => match[1]);
      expect(new Set(secrets), file).toEqual(new Set(["STATS_IMPORT_DATABASE_URL", "PRODUCTION_STATS_IMPORT_DATABASE_URL"]));
      // Each files through the one script, and none around it.
      expect(runsFiling(workflowText(file)), file).toBe(true);
      expect(workflowText(file), file).not.toMatch(/DATABASE_URL=\S+ \S*jackioh-server stats-import/u);
    }
    // The script names both, staging's first: a run files into each database it has a login for.
    const script = readFileSync(FILING, "utf8");
    const filing = script.slice(script.lastIndexOf("for where in"));
    expect(filing.startsWith("for where in staging production;")).toBe(true);
    expect(filing).toMatch(/staging\) url=\$\{STATS_IMPORT_DATABASE_URL:-\} ;;/u);
    expect(filing).toMatch(/production\) url=\$\{PRODUCTION_STATS_IMPORT_DATABASE_URL:-\} ;;/u);
    expect(filing).toMatch(/DATABASE_URL=\$url "\$server" stats-import --totals-only "\$file"/u);
  });
});

describe("R1933000 production's figures are read through STATS_DATABASE_URL, which no workflow imports through", () => {
  it("is read by the win-rate refresh and the size check alone, neither of which imports", () => {
    const readers = workflowFiles.filter((file) => /\bSTATS_DATABASE_URL\b/u.test(workflowText(file)));
    expect(readers.sort()).toEqual(["db-size.yml", "winrates-refresh.yml"]);
    for (const file of readers) expect(workflowText(file), file).not.toContain("stats-import");
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
