// R375: the patch history as the client reads it (cards/patches.ts), and the proof that the main
// bundle carries no snapshot.
//
// A snapshot is reached only through a dynamic `import()`, which Vite splits into a chunk of its own
// that loads when called. So the guard parses every source file under src/ (tests aside) and fails
// on any static import or re-export of a snapshot, and on a dynamic one anywhere but cards/patches.ts,
// the one module that loads them. Then each loader is called and must give its version's file.

import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

import ts from "typescript";
import { describe, expect, it } from "vitest";

import {
  CURRENT_VERSION,
  PATCHES,
  PATCH_CARDS,
  SNAPSHOT_LOADERS,
  commitUrl,
  loadSnapshots,
  sourceLabel,
  sourceUrl,
  versionCount,
} from "./patches.ts";

const SRC = join(dirname(fileURLToPath(import.meta.url)), "..");
const SNAPSHOTS_DIR = join(SRC, "../../../packages/cards/patches/snapshots");
const SNAPSHOT_SPECIFIER = /patches\/snapshots\//u;
/** The one module that loads the snapshots. */
const LOADER = "cards/patches.ts";

function sources(dir: string): string[] {
  return readdirSync(dir).flatMap((name) => {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) return name === "test" ? [] : sources(path);
    return /\.(ts|tsx)$/.test(name) && !/\.test\.tsx?$/.test(name) ? [path] : [];
  });
}

/** Every module specifier in one file that names a snapshot, and whether it is a dynamic import. */
function snapshotImports(path: string): { specifier: string; dynamic: boolean }[] {
  const text = readFileSync(path, "utf8");
  const file = ts.createSourceFile(path, text, ts.ScriptTarget.Latest, true, path.endsWith(".tsx") ? ts.ScriptKind.TSX : ts.ScriptKind.TS);
  const found: { specifier: string; dynamic: boolean }[] = [];
  const visit = (node: ts.Node): void => {
    if ((ts.isImportDeclaration(node) || ts.isExportDeclaration(node)) && node.moduleSpecifier !== undefined) {
      const specifier = (node.moduleSpecifier as ts.StringLiteral).text;
      if (SNAPSHOT_SPECIFIER.test(specifier)) found.push({ specifier, dynamic: false });
    }
    if (ts.isCallExpression(node) && node.expression.kind === ts.SyntaxKind.ImportKeyword) {
      const argument = node.arguments[0];
      const specifier = argument !== undefined && ts.isStringLiteralLike(argument) ? argument.text : argument?.getText(file) ?? "";
      if (SNAPSHOT_SPECIFIER.test(specifier)) found.push({ specifier, dynamic: true });
    }
    ts.forEachChild(node, visit);
  };
  visit(file);
  return found;
}

describe("R375 the patch history in the client", () => {
  it("R375 names the newest patch as the current version", () => {
    expect(CURRENT_VERSION).toBe(PATCHES[PATCHES.length - 1]?.version);
    expect(PATCHES.map((patch) => patch.version)).toContain("v0.1.1");
  });

  it("R375 links an issue, a pull request and a commit on GitHub", () => {
    expect(sourceUrl({ kind: "issue", number: 27 })).toBe("https://github.com/jgoetzmann/JackiOh/issues/27");
    expect(sourceUrl({ kind: "pr", number: 28 })).toBe("https://github.com/jgoetzmann/JackiOh/pull/28");
    expect(sourceLabel({ kind: "issue", number: 27 })).toBe("Issue #27");
    expect(sourceLabel({ kind: "pr", number: 28 })).toBe("PR #28");
    expect(commitUrl("1005c50df56d244075879403230fa448259e7702")).toBe(
      "https://github.com/jgoetzmann/JackiOh/commit/1005c50df56d244075879403230fa448259e7702",
    );
  });

  it("R375 counts a card's versions off changes.json", () => {
    const listing = (id: string): number =>
      Object.values(PATCH_CARDS).filter((cards) => [...cards.created, ...cards.changed, ...cards.removed].includes(id)).length;
    // True Strike's three versions up to v0.1.1, and whatever later patches add.
    expect(versionCount("core-044")).toBeGreaterThanOrEqual(3);
    expect(versionCount("core-044")).toBe(listing("core-044"));
    expect(versionCount("core-011")).toBe(listing("core-011"));
    expect(versionCount("t-1")).toBe(0);
    expect(Object.keys(PATCH_CARDS)).toEqual(PATCHES.map((patch) => patch.version));
  });

  it("R375 has a loader for exactly the patches patches.json lists, each loading its own version's snapshot", async () => {
    expect(Object.keys(SNAPSHOT_LOADERS)).toEqual(PATCHES.map((patch) => patch.version));
    for (const patch of PATCHES) {
      const loader = SNAPSHOT_LOADERS[patch.version];
      expect(loader, patch.version).toBeDefined();
      const loaded = (await loader?.())?.default;
      const onDisk: unknown = JSON.parse(readFileSync(join(SNAPSHOTS_DIR, `${patch.version}.json`), "utf8"));
      expect(loaded, patch.version).toEqual(onDisk);
    }
    const snapshots = await loadSnapshots();
    expect(Object.keys(snapshots)).toEqual(PATCHES.map((patch) => patch.version));
    // Once per page: a second call is the same load.
    expect(loadSnapshots()).toBe(loadSnapshots());
  });

  it("R375 keeps every snapshot out of the main bundle: only cards/patches.ts reaches one, and only by a dynamic import", () => {
    const found = sources(SRC).flatMap((path) =>
      snapshotImports(path).map((entry) => ({ ...entry, file: relative(SRC, path) })),
    );
    // The control: the loader's own imports are found, so the guard reads what it should.
    expect(found.filter((entry) => entry.file === LOADER)).toHaveLength(PATCHES.length);
    expect(found.filter((entry) => !entry.dynamic)).toEqual([]);
    expect(found.filter((entry) => entry.file !== LOADER)).toEqual([]);
  });
});
