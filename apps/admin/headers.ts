// The dashboard's security headers (public/_headers): Vite copies the file into the build as it is, and
// this plugin fills in its CSP's `connect-src`, which names the build's own Supabase project
// (production's and staging's differ, and a development build names none).

import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";

import type { Plugin } from "vite";

/** The headers Cloudflare sends with the build's files (wrangler.jsonc `assets`), copied from `public/`. */
export const HEADERS_FILE = "_headers";

/** What `public/_headers` spells in place of its CSP's `connect-src` sources. */
export const CONNECT_SRC_PLACEHOLDER = "%CONNECT_SRC%";

/** Any `%NAME%` left in the written file: a placeholder the build did not fill. */
const PLACEHOLDER = /%[A-Z][A-Z0-9_]*%/u;

/**
 * The CSP's `connect-src`: 'self', where the Worker answers `/api/*`, and the origin of the build's
 * `VITE_SUPABASE_URL`, which the page signs in against (src/auth.ts). A build without one names only 'self'.
 */
export function connectSources(supabaseUrl: string | undefined): string {
  const sources = ["'self'"];
  if (supabaseUrl !== undefined && supabaseUrl.length > 0) sources.push(new URL(supabaseUrl).origin);
  return sources.join(" ");
}

/** `public/_headers` with its placeholder filled; throws on a template without one or a placeholder left. */
export function filledHeaders(template: string, supabaseUrl: string | undefined): string {
  if (!template.includes(CONNECT_SRC_PLACEHOLDER)) {
    throw new Error(`headers.ts: ${HEADERS_FILE} does not spell ${CONNECT_SRC_PLACEHOLDER}`);
  }
  const filled = template.replaceAll(CONNECT_SRC_PLACEHOLDER, connectSources(supabaseUrl));
  const left = PLACEHOLDER.exec(filled)?.[0];
  if (left !== undefined) throw new Error(`headers.ts: ${HEADERS_FILE} still holds ${left}, which nothing fills`);
  return filled;
}

export function securityHeaders(): Plugin {
  let outDir = "";
  let supabaseUrl: string | undefined;
  return {
    name: "jackioh-admin:security-headers",
    apply: "build",
    configResolved(config) {
      outDir = resolve(config.root, config.build.outDir);
      // `config.env` is Vite's loadEnv: a variable the build's process sets (staging's, from
      // .env.staging by `build:staging`) wins over `.env.production`'s.
      const configured: unknown = config.env.VITE_SUPABASE_URL;
      supabaseUrl = typeof configured === "string" ? configured : undefined;
    },
    // Vite has copied `public/` into the build by now.
    writeBundle() {
      const file = join(outDir, HEADERS_FILE);
      writeFileSync(file, filledHeaders(readFileSync(file, "utf8"), supabaseUrl));
    },
  };
}
