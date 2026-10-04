/**
 * E36: writes each card's `loc` — the non-blank, non-comment lines of its script file, import
 * declarations excluded — into `packages/cards/catalog.json`, and keeps the current patch's snapshot
 * (`patches/<CATALOG_VERSION>.json`, B4.2) equal to it, since a card's lines of code are card data
 * and so part of a patch.
 *
 *   pnpm --filter @jackioh/cards gen          # the barrel, then this
 *   pnpm exec tsx packages/cards/scripts/gen-loc.ts
 *
 * `test/loc.test.ts` holds the catalog to what this computes and names this command when it is
 * stale. A card with no script file yet carries no `loc`. A catalog entry is rewritten only when its
 * `loc` changes, so running this twice is a no-op.
 *
 * Shipped snapshots are never amended (R641): a change that moves a card's `loc` is a catalog
 * change like any other, so claim it with a pending fragment (`pnpm --filter @jackioh/cards
 * patches <version> "<title>"`) and let `patches ship` snapshot it. `patches check` fails until
 * the moved card is claimed.
 */

import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";
import { SET_SUBFOLDERS, resolveRelPath } from "./naming";
import { orderEntry, writeJson, type Catalog } from "./patches-io";

const SCRIPTS_DIR = fileURLToPath(new URL("../src/scripts/", import.meta.url));
export const CATALOG_PATH = fileURLToPath(new URL("../catalog.json", import.meta.url));

/**
 * The lines of code in one script's source: every line that holds code once comments are removed,
 * except the lines of an `import` declaration. Strings and template literals are read as such, so a
 * `//` inside a string is code, not a comment.
 */
export function countLoc(source: string): number {
  const lines = source.split("\n");
  const codeLines: string[] = [];
  let inBlock = false;
  let quote: string | null = null;
  for (const line of lines) {
    let code = "";
    for (let i = 0; i < line.length; i += 1) {
      const ch = line[i] ?? "";
      const next = line[i + 1] ?? "";
      if (inBlock) {
        if (ch === "*" && next === "/") {
          inBlock = false;
          i += 1;
        }
        continue;
      }
      if (quote !== null) {
        code += ch;
        if (ch === "\\") {
          code += next;
          i += 1;
        } else if (ch === quote) {
          quote = null;
        }
        continue;
      }
      if (ch === "/" && next === "/") break;
      if (ch === "/" && next === "*") {
        inBlock = true;
        i += 1;
        continue;
      }
      if (ch === '"' || ch === "'" || ch === "`") quote = ch;
      code += ch;
    }
    // A plain string ends with its line; only a template literal runs on.
    if (quote === '"' || quote === "'") quote = null;
    codeLines.push(code.trim());
  }

  let count = 0;
  let inImport = false;
  for (const code of codeLines) {
    if (code === "") continue;
    if (!inImport && /^import\b/.test(code)) inImport = true;
    if (inImport) {
      if (/\bfrom\s*["'][^"']*["']\s*;?$/.test(code) || /^import\s*["'][^"']*["']\s*;?$/.test(code)) inImport = false;
      continue;
    }
    count += 1;
  }
  return count;
}

/** Every card script, as `{ id, relPath }`, read from `src/scripts/` and its set folders. */
export function scriptPaths(ids: readonly string[]): { id: string; file: string }[] {
  const out: { id: string; file: string }[] = [];
  for (const folder of ["", ...SET_SUBFOLDERS]) {
    let names: string[];
    try {
      names = readdirSync(folder === "" ? SCRIPTS_DIR : `${SCRIPTS_DIR}${folder}/`);
    } catch {
      continue;
    }
    for (const name of names) {
      if (!name.endsWith(".ts") || name.endsWith(".d.ts") || name.endsWith(".test.ts") || name.startsWith("_")) continue;
      const rel = `${folder === "" ? "" : `${folder}/`}${name.slice(0, -".ts".length)}`;
      const id = resolveRelPath(rel, ids);
      if (id !== undefined) out.push({ id, file: `${SCRIPTS_DIR}${rel}.ts` });
    }
  }
  return out;
}

/** What every catalog entry's `loc` should be now: the count of its script, absent without one. */
export function expectedLocs(ids: readonly string[]): Map<string, number> {
  const locs = new Map<string, number>();
  for (const { id, file } of scriptPaths(ids)) locs.set(id, countLoc(readFileSync(file, "utf8")));
  return locs;
}

function withLoc(catalog: Catalog, locs: ReadonlyMap<string, number>): { next: Catalog; changed: string[] } {
  const next: Catalog = {};
  const changed: string[] = [];
  for (const [id, def] of Object.entries(catalog)) {
    const loc = locs.get(id);
    if (def["loc"] === loc) {
      next[id] = def;
      continue;
    }
    changed.push(id);
    const copy: Record<string, unknown> = { ...def };
    if (loc === undefined) delete copy["loc"];
    else copy["loc"] = loc;
    next[id] = orderEntry(copy);
  }
  return { next, changed };
}

export type LocResult = { changed: readonly string[] };

/**
 * Rewrites `loc` in catalog.json; returns the ids it moved. The snapshots are shipped history
 * and are never amended (R641): a moved `loc` is a catalog change for a pending fragment to
 * claim, and `patches ship` snapshots it.
 */
export function generateLoc(): LocResult {
  const catalog = JSON.parse(readFileSync(CATALOG_PATH, "utf8")) as Catalog;
  const locs = expectedLocs(Object.keys(catalog));
  const { next, changed } = withLoc(catalog, locs);
  if (changed.length > 0) writeJson(CATALOG_PATH, next);
  return { changed };
}

function main(): void {
  const result = generateLoc();
  const n = result.changed.length;
  console.log(`gen-loc: ${n === 0 ? "unchanged" : `moved loc of ${n} card${n === 1 ? "" : "s"}`} (catalog.json)`);
}

const entry = process.argv[1];
if (entry !== undefined && pathToFileURL(entry).href === import.meta.url) main();
