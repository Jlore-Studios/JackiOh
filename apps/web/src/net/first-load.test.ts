// #3499: the first load downloads no patch history. Only `patches/source.ts`'s lazy glob loads
// `patches.json` for the Patch notes page; the entry chunk and the files it preloads hold no
// patch-notes text and fit a gzip budget.

import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { gzipSync } from "node:zlib";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { soundTablesNow } from "../audio/soundTablesLoader.ts";
import "../audio/engine.ts";
import "../routes/landing.tsx";

const WEB = join(dirname(fileURLToPath(import.meta.url)), "../..");
const SRC = join(WEB, "src");
const BUILT = join(WEB, "dist");
const PATCHES = join(WEB, "../../crates/cards/patches/patches.json");
const SOURCE = join(SRC, "patches/source.ts");

/**
 * Gzipped bytes the entry chunk and its preloads may take (#3499). A backstop, not a tight fit:
 * putting `patches.json` back into the first load would add about 107.5 KB gzipped (656,124 bytes
 * raw), far more than the headroom. The exact guards are the two tests around it, which look for
 * patch-notes text in the built files and for a static import of `patches.json`.
 *
 * The number was chosen as 120 × 8192 (983,040), about 23 KB (2.4%) over the 960,130 bytes the
 * merge queue measured on `main` at f811a74 with this branch merged in. A change that crosses it
 * raises it on purpose, with its measurement in the commit or the pull request.
 *
 * #3394 crossed it: its 48 Stinkyscape alt scene recipes are data in the preloaded art chunk, which
 * a card's picture reads at once. `vite build --mode development`, as CI builds it, measured 980,369
 * bytes on `main` at 62b8ed2e3d and 987,635 with #3394 merged in (7,266 more). The budget is now
 * 123 × 8192 (1,007,616), about 20 KB (2.0%) over that: putting `patches.json` back would still cross it.
 *
 * The Stinkyscape+ release (#3596) crossed it: the set's 204 entries reach the first load in the catalog,
 * their words and Chinese, their art and their effects table. CI's build of the pull request merged into
 * `main` (b237155b5) measured 1,055,996 bytes. The budget is now 132 × 8192 (1,081,344), about 25 KB (2.4%)
 * over that: putting `patches.json` back would still cross it.
 *
 * The in-game notification feed (#4039) crossed it: its toast is mounted for every signed-in tab, so it, the
 * feed's lines and the social socket's `notification` frame are in the first load. CI's build of the pull
 * request merged into `main` (#4068, run 43822008191) measured 1,081,650 bytes. The budget is now 135 × 8192
 * (1,105,920), about 24 KB (2.2%) over that: putting `patches.json` back would still cross it.
 *
 * #4130 lowered it: the sound tables (voice lines, card audio settings, music manifest) load when
 * sound first needs them rather than with the entry chunk. `vite build --mode development`, as CI builds
 * it, measured 980,254 bytes (101,386 less). The budget is now 122 × 8192 (999,424), about 19 KB (2.0%)
 * over that: putting either `patches.json` or the sound tables back would cross it.
 *
 * #4180 lowered it: the catalog the engine module already ships leaves the entry chunk (every screen
 * reads the module's `currentCatalog()`), and the Chinese card tables load with the reader's language
 * or beside the module. `vite build --mode development`, as CI builds it, measured 834,287 bytes
 * (165,137 less). The budget is now 104 × 8192 (851,968), about 18 KB (2.1%) over that: putting the
 * catalog or the Chinese tables back would cross it.
 */
export const FIRST_LOAD_GZIP_BUDGET_BYTES = 851_968;

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "test" || name === "pkg" ? [] : sources(path);
    return /\.(ts|tsx)$/u.test(name) && !/\.test\.tsx?$/u.test(name) ? [path] : [];
  });
}

it("#3499 no non-test module imports patches.json statically", () => {
  const files = sources(SRC);
  expect(files.length).toBeGreaterThan(100);
  const staticImport = /from\s+["'][^"']*patches\/patches\.json["']/u;
  const dynamicImport = /import\(\s*["'][^"']*patches\/patches\.json["']\s*\)/u;
  const offenders = files.filter((path) => {
    const text = readFileSync(path, "utf8");
    return staticImport.test(text) || dynamicImport.test(text);
  });
  expect(offenders.map((path) => relative(SRC, path))).toEqual([]);
  expect(readFileSync(SOURCE, "utf8")).toContain("import.meta.glob");
});

it("#4180 no non-test module imports catalog.json statically or dynamically", () => {
  const files = sources(SRC);
  expect(files.length).toBeGreaterThan(100);
  const staticImport = /from\s+["'][^"']*catalog\.json["']/u;
  const dynamicImport = /import\(\s*["'][^"']*catalog\.json["']\s*\)/u;
  const offenders = files.filter((path) => {
    // The `@jackioh/cards` surface: the lazy chunks' shared catalog, never the entry's.
    if (relative(SRC, path) === "wire/cards.ts") return false;
    const text = readFileSync(path, "utf8");
    return staticImport.test(text) || dynamicImport.test(text);
  });
  expect(offenders.map((path) => relative(SRC, path))).toEqual([]);
});

it("#4180 only cards/chinese.ts names the Chinese sidecars, through a lazy glob", () => {
  const files = sources(SRC);
  expect(files.length).toBeGreaterThan(100);
  const sidecar = /chinese(-terms)?\.json/u;
  const namers = files.filter((path) => sidecar.test(readFileSync(path, "utf8")));
  expect(namers.map((path) => relative(SRC, path))).toEqual(["cards/chinese.ts"]);
  const loader = readFileSync(join(SRC, "cards/chinese.ts"), "utf8");
  expect(loader).toContain("import.meta.glob");
  expect(loader).not.toMatch(/from\s+["'][^"']*chinese(-terms)?\.json["']/u);
  expect(loader).not.toMatch(/import\(\s*["'][^"']*chinese(-terms)?\.json["']\s*\)/u);
});

it("#4130 no non-test module imports sound tables or raw sound data files statically", () => {
  const files = sources(SRC);
  expect(files.length).toBeGreaterThan(100);
  const staticDataImport = /from\s+["'][^"']*(?:voice-manifest\.json|voice-text\.json|card-audio\.json5|music-manifest\.json|music-cards\.json)["']/u;
  const rawDataImport = /["'][^"']*(?:voice-manifest\.json|voice-text\.json|card-audio\.json5|music-manifest\.json|music-cards\.json)\?raw["']/u;
  const staticTablesImport = /from\s+["'][^"']*soundTables(?:\.ts)?["']/u;
  const offenders = files.filter((path) => {
    const rel = relative(SRC, path);
    if (rel === "audio/soundTables.ts") return false;
    const text = readFileSync(path, "utf8");
    return staticDataImport.test(text) || rawDataImport.test(text) || staticTablesImport.test(text);
  });
  expect(offenders.map((path) => relative(SRC, path))).toEqual([]);
});

it("#4130 importing engine.ts and the landing does not run parseCardAudioText", () => {
  expect(soundTablesNow()).toBeNull();
});

describe.skipIf(!existsSync(BUILT))("#3499 first load", () => {
  /** The entry script and every file it preloads, as files under the built site. */
  function entryAndPreloads(): string[] {
    const html = readFileSync(join(BUILT, "index.html"), "utf8");
    const entry = /<script[^>]*type="module"[^>]*src="([^"]+)"[^>]*>/u.exec(html)?.[1];
    expect(entry, "no module entry script in dist/index.html").toBeDefined();
    const preloads = [...html.matchAll(/<link[^>]*rel="modulepreload"[^>]*href="([^"]+)"[^>]*>/gu)].map(
      (match) => match[1],
    );
    const files = [entry, ...preloads]
      .filter((href): href is string => typeof href === "string")
      .map((href) => join(BUILT, (href.split(/[#?]/u)[0] ?? href).replace(/^\//u, "")));
    for (const file of files) expect(existsSync(file), file).toBe(true);
    return files;
  }

  /** Three patch-notes strings no card catalog shares: the first and newest notes, the newest title. */
  function probes(): string[] {
    const patches = JSON.parse(readFileSync(PATCHES, "utf8")) as {
      readonly notes?: unknown;
      readonly player?: { readonly title?: unknown };
    }[];
    if (!Array.isArray(patches) || patches.length === 0) throw new Error("patches.json is not a list of patches");
    const found = [patches[0]?.notes, patches.at(-1)?.notes, patches.at(-1)?.player?.title];
    expect(found.every((probe) => typeof probe === "string" && probe.length > 0)).toBe(true);
    return found.filter((probe): probe is string => typeof probe === "string");
  }

  it("#3499 the entry chunk and its preloads hold no patch-notes text", () => {
    const texts = entryAndPreloads().map((file) => readFileSync(file, "utf8"));
    for (const probe of probes()) {
      for (const text of texts) expect(text).not.toContain(probe);
    }
  });

  it("#4130 the entry chunk and its preloads hold no sound tables or json5", () => {
    const texts = entryAndPreloads().map((file) => readFileSync(file, "utf8"));
    const soundProbes = [
      "Stand behind me. Way behind.",
      "Every card's sounds (SPEC §10.11, R655)",
      "aac-lc 44100 stereo 80k",
      "JSON5: invalid character",
    ];
    for (const probe of soundProbes) {
      for (const text of texts) expect(text).not.toContain(probe);
    }
  });

  it("#4180 the entry chunk and its preloads hold no Chinese card name", () => {
    const texts = entryAndPreloads().map((file) => readFileSync(file, "utf8"));
    const cards = JSON.parse(readFileSync(join(WEB, "../../crates/cards/chinese.json"), "utf8")) as Record<string, { name?: unknown }>;
    const ids = Object.keys(cards).filter((id) => id !== "//");
    const first = cards[ids[0] ?? ""];
    const last = cards[ids.at(-1) ?? ""];
    const terms = JSON.parse(readFileSync(join(WEB, "../../crates/cards/chinese-terms.json"), "utf8")) as { radiant?: unknown };
    const probes = [first?.name, last?.name, terms.radiant];
    expect(ids.length).toBeGreaterThan(0);
    expect(probes.every((probe) => typeof probe === "string" && probe.length > 0)).toBe(true);
    for (const probe of probes) {
      for (const text of texts) expect(text).not.toContain(probe as string);
    }
  });

  it("#3499 no built script, a worker's included, holds patch-meta.ts's disk reader or patch-meta.browser.ts's bundle", () => {
    // A bundle that lacks `patchMeta()` (a worker's has its own plugin list) takes `patch-meta.ts`
    // (whose code carries `holds an entry with no version`) or, under the `browser` condition from
    // `package.json`'s `imports`, `patch-meta.browser.ts` (whose code carries `holds a patch with no
    // version string` and statically imports the 650 KB history): either way the worker would carry
    // the patch history.
    const assets = join(BUILT, "assets");
    const scripts = readdirSync(assets).filter((name) => name.endsWith(".js"));
    expect(scripts.length).toBeGreaterThan(0);
    const readers = scripts.filter((name) => {
      const text = readFileSync(join(assets, name), "utf8");
      return text.includes("holds an entry with no version") || text.includes("holds a patch with no version string");
    });
    expect(readers).toEqual([]);
  });

  it("#3499 the entry chunk and its preloads fit the gzip budget", () => {
    const files = entryAndPreloads().map((file) => readFileSync(file));
    expect(gzipSync(Buffer.concat(files)).length).toBeLessThanOrEqual(FIRST_LOAD_GZIP_BUDGET_BYTES);
  });
});
