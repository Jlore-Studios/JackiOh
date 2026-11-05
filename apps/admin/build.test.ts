// @vitest-environment node
// The dashboard's builds, made for real into throwaway folders: a production build reads no e2e session
// (Vite folds src/session.ts's MODE test away, the key with it) and writes its CSP with its own Supabase
// project, while the e2e build (`--mode development`) keeps the e2e session it is made for.

import { mkdtempSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { build } from "vite";
import { afterAll, describe, expect, it } from "vitest";

import { HEADERS_FILE } from "./headers.ts";
import { E2E_SESSION_STORAGE_KEY, SESSION_STORAGE_KEY } from "./src/session.ts";

const ADMIN = dirname(fileURLToPath(import.meta.url));
const made: string[] = [];

/** A Vite build can take a few seconds on a busy machine. */
const BUILD_TIMEOUT_MS = 60_000;

async function bundle(mode: string): Promise<{ scripts: string; headers: string }> {
  const outDir = mkdtempSync(join(tmpdir(), `jackioh-admin-${mode}-`));
  made.push(outDir);
  await build({ root: ADMIN, mode, logLevel: "silent", build: { outDir, emptyOutDir: true } });
  const assets = join(outDir, "assets");
  const scripts = readdirSync(assets)
    .filter((file) => file.endsWith(".js"))
    .map((file) => readFileSync(join(assets, file), "utf8"))
    .join("\n");
  return { scripts, headers: readFileSync(join(outDir, HEADERS_FILE), "utf8") };
}

const production = Object.fromEntries(
  readFileSync(join(ADMIN, ".env.production"), "utf8")
    .split("\n")
    .filter((line) => line.startsWith("VITE_"))
    .map((line) => [line.slice(0, line.indexOf("=")), line.slice(line.indexOf("=") + 1)]),
);

afterAll(() => {
  for (const dir of made) rmSync(dir, { recursive: true, force: true });
});

describe("the dashboard's builds", () => {
  it(
    "a production build carries no e2e session, keeps the tab's, and names its own Supabase project in its CSP",
    async () => {
      const { scripts, headers } = await bundle("production");
      // Booleans, so a failure names the key and not the whole bundle.
      expect(scripts.includes(E2E_SESSION_STORAGE_KEY), E2E_SESSION_STORAGE_KEY).toBe(false);
      expect(scripts.includes(SESSION_STORAGE_KEY), SESSION_STORAGE_KEY).toBe(true);
      expect(scripts.includes(production["VITE_SUPABASE_URL"] ?? "?"), "VITE_SUPABASE_URL").toBe(true);
      expect(headers).toContain(`connect-src 'self' ${production["VITE_SUPABASE_URL"] ?? "?"};`);
      expect(headers).not.toMatch(/%[A-Z_]+%/u);
    },
    BUILD_TIMEOUT_MS,
  );

  it(
    "the e2e build reads the e2e session",
    async () => {
      const { scripts, headers } = await bundle("development");
      expect(scripts.includes(E2E_SESSION_STORAGE_KEY), E2E_SESSION_STORAGE_KEY).toBe(true);
      expect(headers).toContain("connect-src 'self'");
      expect(headers).not.toMatch(/%[A-Z_]+%/u);
    },
    BUILD_TIMEOUT_MS,
  );
});
