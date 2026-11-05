// apps/web/scripts/check-production-bundle.mjs, the gate between production's build and its deploy
// (promote-production.yml), run on a stand-in for apps/web/dist.

import { spawnSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseEnv } from "node:util";

import { afterEach, describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const WEB = join(HERE, "../..");
const ROOT = join(WEB, "../..");
const SCRIPT = join(WEB, "scripts/check-production-bundle.mjs");
const PRODUCTION = parseEnv(readFileSync(join(WEB, ".env.production"), "utf8"));
/** Every value apps/web/.env.production sets, as a build from it alone puts them in the bundle. */
const VALUES = Object.values(PRODUCTION).filter((value): value is string => value !== undefined);
const KEY = PRODUCTION.VITE_SUPABASE_PUBLISHABLE_KEY ?? "";

let dist = "";

afterEach(() => {
  if (dist !== "") rmSync(dist, { recursive: true, force: true });
  dist = "";
});

/** R3127000: the client's entry as Vite's build writes it into index.html, and the head line that makes 404.html noindex. */
const ENTRY_TAG = '<script type="module" crossorigin src="/assets/index-test.js"></script>';
const NOINDEX = '<meta name="robots" content="noindex" />';

/**
 * A built site holding `values` in its JavaScript, with every file the check requires and a CSP that reaches both hosts.
 * R3127000: `notFound` is its 404.html, by default the client's own page (its entry script, noindex, no canonical).
 */
function builtWith(values: readonly string[], notFound: string = `${ENTRY_TAG}${NOINDEX}`): string {
  dist = mkdtempSync(join(tmpdir(), "bundle-check-"));
  mkdirSync(join(dist, "assets"));
  mkdirSync(join(dist, "almanac"));
  for (const served of ["_redirects", "sitemap.xml", "almanac/index.html"]) {
    writeFileSync(join(dist, served), "");
  }
  writeFileSync(join(dist, "index.html"), ENTRY_TAG);
  writeFileSync(join(dist, "404.html"), notFound);
  const connect = [PRODUCTION.VITE_SUPABASE_URL, PRODUCTION.VITE_SERVER_WS_URL].map((url) => new URL(url ?? "").origin);
  writeFileSync(join(dist, "_headers"), `/*\n  Content-Security-Policy: connect-src 'self' ${connect.join(" ")}; img-src 'self'\n`);
  writeFileSync(join(dist, "assets", "index.js"), values.map((value) => JSON.stringify(value)).join(";"));
  return dist;
}

/** The check, with only `vite` variables the test gives in its environment. */
function check(site: string, vite: Record<string, string> = {}): { status: number | null; output: string } {
  const env: Record<string, string> = {};
  for (const [name, value] of Object.entries(process.env)) if (!name.startsWith("VITE_") && value !== undefined) env[name] = value;
  const run = spawnSync(process.execPath, [SCRIPT, site], { cwd: ROOT, env: { ...env, ...vite }, encoding: "utf8" });
  return { status: run.status, output: run.stdout + run.stderr };
}

describe("check-production-bundle.mjs", () => {
  it("passes a bundle built from apps/web/.env.production alone", () => {
    const result = check(builtWith(VALUES));
    expect(result.output).toContain("carries all 4 settings");
    expect(result.status).toBe(0);
  });

  it("refuses a bundle whose build environment overrode the file's publishable key with another project's", () => {
    const stale = "sb_publishable_another_projects_key";
    const result = check(builtWith([...VALUES, stale]), { VITE_SUPABASE_PUBLISHABLE_KEY: stale });
    expect(result.status).toBe(1);
    expect(result.output).toContain(
      "VITE_SUPABASE_PUBLISHABLE_KEY in the build's environment differs from apps/web/.env.production's",
    );
  });

  it("takes an override that agrees with the file", () => {
    const result = check(builtWith(VALUES), { VITE_SUPABASE_PUBLISHABLE_KEY: KEY });
    expect(result.status).toBe(0);
  });

  it("R3127000 passes a 404.html that is the client: its entry script and noindex", () => {
    expect(check(builtWith(VALUES)).status).toBe(0);
  });

  it("R3127000 refuses a 404.html that does not load the client's entry script", () => {
    const result = check(builtWith(VALUES, NOINDEX));
    expect(result.status).toBe(1);
    expect(result.output).toContain("404.html does not load the client's entry script (/assets/index-test.js)");
  });

  it("R3127000 refuses a 404.html that crawlers may index", () => {
    const result = check(builtWith(VALUES, ENTRY_TAG));
    expect(result.status).toBe(1);
    expect(result.output).toContain("404.html is not noindex");
  });

  it("R3127000 refuses a 404.html that names a canonical address", () => {
    const result = check(builtWith(VALUES, `${ENTRY_TAG}${NOINDEX}<link rel="canonical" href="https://jackioh.com/" />`));
    expect(result.status).toBe(1);
    expect(result.output).toContain("404.html names a canonical address");
  });
});

describe("promote-production.yml's build", () => {
  it("sets no VITE_ variable from the repository's variables or secrets, which would win over the file", () => {
    const workflow = readFileSync(join(ROOT, ".github/workflows/promote-production.yml"), "utf8");
    expect(workflow).not.toMatch(/(vars|secrets)\.VITE_/u);
    expect(workflow).not.toMatch(/export VITE_/u);
  });
});
