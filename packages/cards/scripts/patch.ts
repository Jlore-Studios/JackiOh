/**
 * Every file that carries the catalog version (B4.2, R388, R646), and how to rewrite it there.
 * `patches.ts`'s promotion bumps them to the newest shipped patch; `test/patches.test.ts` holds
 * all of them to it. A deployment then reseeds: `db:seed-catalog` stamps every `cards` row and
 * `app.settings.catalog_version` with the new version.
 *
 * Patches used to be made by this module (`pnpm --filter @jackioh/cards patch …`), which snapshotted
 * the catalog and appended to `patches.json` directly. Several patches are now built at once, so
 * branches add a fragment under `patches/pending/` instead (`patches.ts`, R646) and nothing here
 * writes the history anymore. A micro `vA.B.Y` (R650) keeps its `Y` in the fragment until that
 * promotion names it (`versions.ts`).
 */

import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const REPO_ROOT = fileURLToPath(new URL("../../../", import.meta.url));

/** Every file that carries the catalog version, and how to rewrite it there. */
export const VERSION_SITES: readonly { file: string; pattern: RegExp; render: (version: string) => string }[] = [
  {
    file: "packages/cards/src/catalog-data.ts",
    pattern: /export const CATALOG_VERSION = "[^"]*";/,
    render: (v) => `export const CATALOG_VERSION = "${v}";`,
  },
  {
    file: "apps/server/.env.example",
    pattern: /^CATALOG_VERSION=.*$/m,
    render: (v) => `CATALOG_VERSION=${v}`,
  },
  {
    file: "render.yaml",
    pattern: /(- key: CATALOG_VERSION\n\s+value: )\S+/,
    render: (v) => `$1${v}`,
  },
  {
    file: "apps/server/src/index.ts",
    pattern: /(\n\s+CATALOG_VERSION: )"[^"]*"(,)/,
    render: (v) => `$1"${v}"$2`,
  },
];

/** The version each site carries now, or `undefined` where the pattern finds none. */
export function versionsAtSites(): { file: string; version: string | undefined }[] {
  return VERSION_SITES.map((site) => {
    const text = readFileSync(`${REPO_ROOT}${site.file}`, "utf8");
    const match = /"([^"]+)"|=(\S+)|value: (\S+)/.exec(text.match(site.pattern)?.[0] ?? "");
    return { file: site.file, version: match?.[1] ?? match?.[2] ?? match?.[3] };
  });
}

/**
 * Rewrites `CATALOG_VERSION` everywhere the string lives (promotion bumps it, R646). Every site is
 * read and matched before the first one is written, so a missing site leaves all of them alone.
 */
export function bumpSites(version: string, repoRoot: string = REPO_ROOT): void {
  const rewritten = VERSION_SITES.map((site) => {
    const path = `${repoRoot}/${site.file}`;
    const text = readFileSync(path, "utf8");
    if (!site.pattern.test(text)) throw new Error(`${site.file}: no CATALOG_VERSION to rewrite`);
    return { path, text: text.replace(site.pattern, site.render(version)) };
  });
  for (const { path, text } of rewritten) writeFileSync(path, text, "utf8");
}
