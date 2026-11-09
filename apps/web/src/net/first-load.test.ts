// #3499: the first load downloads no patch history. Only `patches/source.ts`'s lazy glob loads
// `patches.json` for the Patch notes page; the entry chunk and the files it preloads hold no
// patch-notes text and fit a gzip budget.

import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { gzipSync } from "node:zlib";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const WEB = join(dirname(fileURLToPath(import.meta.url)), "../..");
const SRC = join(WEB, "src");
const BUILT = join(WEB, "dist");
const PATCHES = join(WEB, "../../crates/cards/patches/patches.json");
const SOURCE = join(SRC, "patches/source.ts");

/** Gzipped bytes the entry chunk and its preloads may take (#3499). */
export const FIRST_LOAD_GZIP_BUDGET_BYTES = 901_120;

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

  it("#3499 the entry chunk and its preloads fit the gzip budget", () => {
    const files = entryAndPreloads().map((file) => readFileSync(file));
    expect(gzipSync(Buffer.concat(files)).length).toBeLessThanOrEqual(FIRST_LOAD_GZIP_BUDGET_BYTES);
  });
});
