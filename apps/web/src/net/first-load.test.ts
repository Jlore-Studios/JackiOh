// #3499: the first load downloads no patch history. Only `patches/source.ts`'s lazy glob loads
// `patches.json` for the Patch notes page; the entry chunk and the files it preloads hold no
// patch-notes text and fit a gzip budget.

import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { gzipSync } from "node:zlib";
import { basename, dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { soundTablesNow } from "../audio/soundTablesLoader.ts";
import { flavourNow } from "../cards/flavour.ts";
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
 *
 * #4222 lowered it: the scene kits and both recipe tables (every `sceneKit*.ts`, `figureKit.ts` and
 * all `*Recipes*.ts`) load in a chunk beside the engine module rather than with the entry chunk.
 * `vite build --mode development`, as CI builds it, measured 473,538 bytes
 * (360,749 less). The budget is now 59 × 8192 (483,328), about 10 KB (2.1%)
 * over that: putting the scene data back would cross it.
 *
 * #4435 lowered it: the flavour sidecar (every card's flavour line and artist credit) loads in a
 * chunk beside the engine module rather than with the entry chunk. `vite build --mode development`,
 * as CI builds it, measured 445,874 bytes (27,664 less). The budget is now 56 × 8192 (458,752),
 * about 13 KB (2.9%) over that: putting the sidecar back would cross it.
 *
 * #4428 lowered it, and merged after #4435: the sound-effect recipes ride their own lazy chunk beside
 * the module rather than with the entry chunk. Its own build, which lacked #4435's change, measured
 * 447,192 bytes (26,346 less than #4222's 473,538); #4435's own build, which lacked this one, measured
 * 445,874. With both merged, `vite build --mode development`, as CI builds it, measured 420,381 bytes
 * on `main` at 59c8529162: 25,493 less than #4435's 445,874. The budget is now 53 × 8192 (434,176),
 * about 13 KB (3.3%) over that: putting the recipes, the sidecar or the scene data back would cross it.
 *
 * #4444 lowered it: the Friends drawer (the friends list and rows, the friend menu, the chat panel
 * and the challenge dialog) rides its own lazy chunk behind the rail button rather than with the
 * entry chunk. `vite build --mode development`, as CI builds it, measured 413,393 bytes
 * (6,988 less). The budget is now 51 × 8192 (417,792), about 4 KB (1.1%)
 * over that: putting the drawer panels back would cross it.
 *
 * #4517 lowered it: the card detail dialog (its patch history and win-rate box) rides its own
 * lazy chunk behind the landing's fan cards, and the account screen rides `routeChunks.account`
 * while only its sign-out helpers stay in the entry. `vite build --mode development`, as CI
 * builds it, measured 384,276 bytes (29,117 less). The budget is now 48 × 8192 (393,216),
 * about 9 KB (2.3%) over that: putting either module back would cross it.
 */
export const FIRST_LOAD_GZIP_BUDGET_BYTES = 393_216;

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

it("#4428 no non-test module statically imports sfx.ts", () => {
  const files = sources(SRC);
  expect(files.length).toBeGreaterThan(100);
  const staticFrom = /from\s+["']([^"']+)["']/gu;
  const importsSfx = (specifier: string): boolean => (specifier.split("/").pop() ?? "") === "sfx.ts";
  const offenders = files.filter((path) => {
    const rel = relative(SRC, path);
    // The sound tables' lazy chunk reads the recipes' ids and timbres (#4428): it is itself reached
    // only through a dynamic import, never the entry chunk.
    if (rel === "audio/soundTables.ts") return false;
    // `import type` lines emit no bytes: only a value import joins the chunk.
    const text = readFileSync(path, "utf8")
      .split("\n")
      .filter((line) => !/^\s*import\s+type\b/.test(line))
      .join("\n");
    return [...text.matchAll(staticFrom)].some((match) => importsSfx(match[1] ?? ""));
  });
  expect(offenders.map((path) => relative(SRC, path))).toEqual([]);
  // The one module allowed to pull the recipes in, dynamically, is their loader.
  const dynamicSfx = /import\(\s*["'][^"']*sfx\.ts["']\s*\)/u;
  const holders = files.filter((path) => dynamicSfx.test(readFileSync(path, "utf8")));
  expect(holders.map((path) => relative(SRC, path))).toEqual(["audio/sfxLoader.ts"]);
});

it("#4444 no non-test module statically imports the friends drawer panels", () => {
  const files = sources(SRC);
  expect(files.length).toBeGreaterThan(100);
  const staticFrom = /from\s+["']([^"']+)["']/gu;
  const drawerPanel = (specifier: string): boolean =>
    ["ChallengeDialog.tsx", "ChatPanel.tsx", "FriendList.tsx", "FriendRows.tsx", "FriendMenu.tsx"].includes(specifier.split("/").pop() ?? "");
  // Already reached only through a lazy chunk: the drawer's own module and the lazy Friends
  // page (`TogetherRoute.tsx`, `TogetherPanels.tsx`), and the list's own rows.
  const allowed = new Set([
    "together/FriendsDrawer.tsx",
    "together/TogetherRoute.tsx",
    "together/TogetherPanels.tsx",
    "together/FriendList.tsx",
    "together/FriendRows.tsx",
  ]);
  const offenders = files.filter((path) => {
    const rel = relative(SRC, path);
    if (allowed.has(rel)) return false;
    // `import type` lines emit no bytes: only a value import joins the chunk.
    const text = readFileSync(path, "utf8")
      .split("\n")
      .filter((line) => !/^\s*import\s+type\b/.test(line))
      .join("\n");
    return [...text.matchAll(staticFrom)].some((match) => drawerPanel(match[1] ?? ""));
  });
  expect(offenders.map((path) => relative(SRC, path))).toEqual([]);
  // The one module allowed to pull the drawer in, dynamically, is its chunk.
  const dynamicDrawer = /import\(\s*["'][^"']*FriendsDrawer\.tsx["']\s*\)/u;
  const holders = files.filter((path) => dynamicDrawer.test(readFileSync(path, "utf8")));
  expect(holders.map((path) => relative(SRC, path))).toEqual(["together/friendsDrawerChunk.ts"]);
});

it("#4517 no lazy chunk is already in the entry's static graph", () => {
  // A `lazy()` or `routeChunk` `import()` that resolves to code the entry already loads
  // statically is dead weight in disguise: the bundler keeps the module in the entry chunk
  // and the dynamic import fetches nothing new (#4517: the landing's `lazy()` CardDetail rode
  // in through the `cards/` barrels). Walk the static import graph from `main.tsx` (value
  // imports and re-exports; `import type` and `export type` lines emit no bytes, and test
  // files are never the entry's) and fail on any non-test `import()` target inside it.
  // Regexes over source, like this file's other checks: they cannot see a specifier built at
  // runtime, and the client uses none outside `import.meta.glob`.
  const entry = join(SRC, "main.tsx");
  const staticFrom = /from\s+["']([^"']+)["']/gu;
  const dynamicTo = /import\(\s*["']([^"']+)["']/gu;
  const resolveRelative = (importer: string, specifier: string): string | null => {
    if (!specifier.startsWith(".")) return null;
    const base = join(dirname(importer), specifier);
    const candidates = [base, `${base}.ts`, `${base}.tsx`, join(base, "index.ts"), join(base, "index.tsx")];
    return candidates.find((candidate) => existsSync(candidate) && statSync(candidate).isFile()) ?? null;
  };
  const valueTextOf = (path: string): string =>
    readFileSync(path, "utf8")
      .split("\n")
      .filter((line) => !/^\s*import\s+type\b/u.test(line) && !/^\s*export\s+type\b/u.test(line))
      .join("\n");
  const reachable = new Set<string>([entry]);
  const queue = [entry];
  while (queue.length > 0) {
    const current = queue.pop() as string;
    if (/\.test\.tsx?$/u.test(current)) continue;
    for (const match of valueTextOf(current).matchAll(staticFrom)) {
      const next = resolveRelative(current, match[1] ?? "");
      if (next !== null && !reachable.has(next)) {
        reachable.add(next);
        queue.push(next);
      }
    }
  }
  expect(reachable.size).toBeGreaterThan(100);
  // The only loaders allowed in the entry's static graph, each with its reason: their payloads
  // sit behind lazy globs or lazy imports this file already guards.
  const allowed = new Map<string, string>([
    // Payload rides its lazy glob beside the engine; the small loader sits in the entry on purpose.
    ["cards/chinese.ts", "payload rides its lazy glob beside the engine"],
    // Recipes ride its lazy import; the small loader sits in the entry on purpose.
    ["audio/sfxLoader.ts", "recipes ride its lazy import"],
  ]);
  const offenders: string[] = [];
  for (const importer of [...reachable].sort()) {
    if (/\.test\.tsx?$/u.test(importer)) continue;
    for (const match of readFileSync(importer, "utf8").matchAll(dynamicTo)) {
      const target = resolveRelative(importer, match[1] ?? "");
      if (target === null || !reachable.has(target)) continue;
      const rel = relative(SRC, target);
      if (allowed.has(rel)) continue;
      offenders.push(`${relative(SRC, importer)} → ${rel}`);
    }
  }
  expect(offenders.sort()).toEqual([]);
  expect([...allowed.keys()].sort()).toEqual(["audio/sfxLoader.ts", "cards/chinese.ts"]);
});

it("#4435 no non-test module names flavour.json except cards/flavour.ts, which never imports it statically", () => {
  const files = sources(SRC);
  expect(files.length).toBeGreaterThan(100);
  const namers = files.filter((path) => /flavour\.json/u.test(readFileSync(path, "utf8")));
  expect(namers.map((path) => relative(SRC, path))).toEqual(["cards/flavour.ts"]);
  const loader = readFileSync(join(SRC, "cards/flavour.ts"), "utf8");
  expect(loader).not.toMatch(/from\s+["'][^"']*flavour\.json["']/u);
  expect(loader).toContain("import(");
});

it("#4435 importing landing and engine leaves flavourNow() null", () => {
  expect(flavourNow()).toBeNull();
});

it("#4222 no non-test module statically imports scene data", () => {
  const files = sources(SRC);
  expect(files.length).toBeGreaterThan(100);
  const staticFrom = /from\s+["']([^"']+)["']/gu;
  const importsSceneData = (specifier: string): boolean => {
    const base = specifier.split("/").pop() ?? "";
    if (!base.endsWith(".ts")) return false;
    if (base === "sceneData.ts" || base === "figureKit.ts") return true;
    if (base.startsWith("sceneKit")) return true;
    return base.includes("Recipes");
  };
  const exempt = new Set<string>();
  const offenders = files.filter((path) => {
    const rel = relative(SRC, path);
    const base = basename(rel);
    const dataFile =
      rel === "cards/art/sceneData.ts" ||
      (rel.startsWith("cards/art/") && (base.startsWith("sceneKit") || base === "figureKit.ts" || base.includes("Recipes")));
    if (dataFile) {
      exempt.add(rel);
      return false;
    }
    // `import type` lines emit no bytes: only a value import joins the chunk.
    const text = readFileSync(path, "utf8")
      .split("\n")
      .filter((line) => !/^\s*import\s+type\b/.test(line))
      .join("\n");
    return [...text.matchAll(staticFrom)].some((match) => importsSceneData(match[1] ?? ""));
  });
  expect(offenders.map((path) => relative(SRC, path))).toEqual([]);
  for (const name of ["cards/art/sceneKit.ts", "cards/art/figureKit.ts", "cards/art/sceneRecipes.ts"]) {
    expect(exempt, name).toContain(name);
  }
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

  it("#4435 the entry chunk and its preloads hold no flavour line or artist credit", () => {
    const texts = entryAndPreloads().map((file) => readFileSync(file, "utf8"));
    const catalogText = readFileSync(join(WEB, "../../crates/cards/catalog.json"), "utf8");
    const chineseText = readFileSync(join(WEB, "../../crates/cards/chinese.json"), "utf8");
    const sidecar = JSON.parse(readFileSync(join(WEB, "../../crates/cards/flavour.json"), "utf8")) as Record<
      string,
      { flavour?: unknown; artist?: unknown }
    >;
    const ids = Object.keys(sidecar).filter((id) => id !== "//");
    expect(ids.length).toBeGreaterThan(0);
    // A probe string that also occurs in the catalog or the Chinese tables proves nothing.
    const unique = (value: unknown): value is string =>
      typeof value === "string" && value.length > 0 && !catalogText.includes(value) && !chineseText.includes(value);
    const flavours = ids.map((id) => sidecar[id]?.flavour).filter(unique);
    const artists = ids.map((id) => sidecar[id]?.artist).filter(unique);
    expect(flavours.length).toBeGreaterThan(0);
    // No shipped entry names an artist yet, so a third flavour line stands in until one does.
    const probes = [flavours[0], flavours.at(-1), artists[0] ?? flavours[Math.floor(flavours.length / 2)]];
    expect(probes.every((probe) => typeof probe === "string" && probe.length > 0)).toBe(true);
    for (const probe of probes) {
      for (const text of texts) expect(text).not.toContain(probe as string);
    }
  });

  it("#4222 the entry chunk and its preloads hold no scene kit or recipe text", () => {
    const texts = entryAndPreloads().map((file) => readFileSync(file, "utf8"));
    // Each occurs in `src` only in the scene data files, their tests, or `ART.md`: a kit part, a
    // figure pose, and a figure item each a recipe places.
    const sceneProbes = ["bone-scarecrow", "pose-genuflect", "item-trunk-club"];
    for (const probe of sceneProbes) {
      for (const text of texts) expect(text).not.toContain(probe);
    }
  });

  it("#4517 the entry chunk and its preloads hold no card-history text", () => {
    const texts = entryAndPreloads().map((file) => readFileSync(file, "utf8"));
    // Each probe is a runtime string, so minification keeps it, and each occurs in `src` only in
    // the card history view, its tests, or comments: the history's loading line, its failure
    // line, and its line for a card no patch recorded.
    const historyProbes = ["Loading this card's history…", "This card's history didn't load.", "No patch has recorded this card."];
    for (const probe of historyProbes) {
      for (const text of texts) expect(text).not.toContain(probe);
    }
  });

  it("#4517 the entry chunk and its preloads hold no account-screen text", () => {
    const texts = entryAndPreloads().map((file) => readFileSync(file, "utf8"));
    // Each probe is a runtime string, so minification keeps it, and each occurs in `src` only in
    // the account screen or its tests: the sign-out note, the screen's loading line, and the
    // profile header's member line. Never the sign-out buttons' own words: those stay in the entry.
    const accountProbes = [
      "Signing out forgets this session on this device and revokes it.",
      "Loading your account…",
      "Member since",
    ];
    for (const probe of accountProbes) {
      for (const text of texts) expect(text).not.toContain(probe);
    }
  });

  it("#4444 the entry chunk and its preloads hold no friends-drawer text", () => {
    const texts = entryAndPreloads().map((file) => readFileSync(file, "utf8"));
    // Each probe is a runtime string, so minification keeps it, and each occurs in `src` only in
    // the drawer panels, their tests, or comments: the chat's earlier page, the drawer's link to
    // the Friends page, and the friend menu's daily item.
    const drawerProbes = ["Show earlier messages", "See all friends", "Daily challenge"];
    for (const probe of drawerProbes) {
      for (const text of texts) expect(text).not.toContain(probe);
    }
  });

  it("#4428 the entry chunk and its preloads hold no sfx recipes", () => {
    const texts = entryAndPreloads().map((file) => readFileSync(file, "utf8"));
    // Each probe is a runtime string, so minification keeps it, and each is absent from the entry
    // once the recipes leave it: a dotted run of recipe ids out of the id switch the bundler
    // compiles (never dotted in `src`, and never dotted in the entry's cue map), and two
    // oscillator/filter words the entry's own audio code never uses (`crowd.ts` reads them too, but
    // only through the lazy `noise.ts` leaf). The issue's suggested `CENTS_PER_OCTAVE` and
    // `CRUSH_CYCLE` cannot serve: the minifier renames them away, so they are absent from every
    // built asset and would pass vacuously.
    const sfxProbes = ["trapSet.trapSting.spell.mana", "sawtooth", "bandpass"];
    for (const probe of sfxProbes) {
      for (const text of texts) expect(text).not.toContain(probe);
    }
  });

  it("#4222 the practice worker's bundle holds no scene data", () => {
    const assets = join(BUILT, "assets");
    const scripts = readdirSync(assets).filter((name) => name.endsWith(".js"));
    expect(scripts.length).toBeGreaterThan(0);
    const workers = scripts.filter((name) => {
      const text = readFileSync(join(assets, name), "utf8");
      return text.includes("the practice engine could not be loaded");
    });
    expect(workers.length).toBeGreaterThan(0);
    const sceneProbes = ["bone-scarecrow", "pose-genuflect", "item-trunk-club"];
    for (const name of workers) {
      const text = readFileSync(join(assets, name), "utf8");
      for (const probe of sceneProbes) expect(text, name).not.toContain(probe);
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
