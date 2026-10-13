import { fileURLToPath } from "node:url";

import react from "@vitejs/plugin-react";
import { defineConfig, type Alias } from "vite";

import { staticPages } from "./static-pages.ts";

// The client is a pure `PlayerView` renderer (CLAUDE.md rule 7, SPEC §10.8): it sends intent
// and draws what the engine hands it. The rules are the Rust engine, compiled to WebAssembly
// (`src/wasm/`, built into `src/wasm/pkg/` by `scripts/build-wasm.sh`, which `predev`, `prebuild`,
// `prebuild:e2e` and `pretest` run): the page loads the module for the hotseat and the deck
// builder's validator, and the practice worker loads its own and runs the engine and the AI off the
// page's thread (SPEC §9.9, R187).

/** The repository root and the client's wire layer, as absolute paths with a trailing slash. */
const REPO = fileURLToPath(new URL("../../", import.meta.url));
const WIRE = fileURLToPath(new URL("./src/wire/", import.meta.url));

/**
 * The `@jackioh/*` specifiers the client has always imported, resolved to its own wire layer
 * (docs/v0.3.0/SURFACE.md §10.4), so the files that import them need no edit. Exact matches only:
 * `@jackioh/cards` and `@jackioh/cards/catalog.json` are different modules. `vitest.config.ts` and
 * the component tests' Vite server (`e2e/cypress.config.ts`) use the same list; `tsconfig.json`'s
 * `paths` state it again for the type checker.
 */
export const jackiohAliases: Alias[] = [
  { find: /^@jackioh\/shared$/, replacement: `${WIRE}index.ts` },
  { find: /^@jackioh\/engine\/config$/, replacement: `${WIRE}engineConfig.ts` },
  { find: /^@jackioh\/engine$/, replacement: `${WIRE}engine.ts` },
  { find: /^@jackioh\/validator$/, replacement: `${WIRE}validator.ts` },
  { find: /^@jackioh\/cards\/(catalog|flavour|chinese|chinese-terms)\.json$/, replacement: `${REPO}crates/cards/$1.json` },
  { find: /^@jackioh\/cards$/, replacement: `${WIRE}cards.ts` },
  { find: /^@jackioh\/ai(?:\/config)?$/, replacement: `${WIRE}ai.ts` },
  { find: /^@jackioh\/server-config$/, replacement: `${WIRE}serverConfig.ts` },
];

export default defineConfig({
  // `staticPages` writes each public page's own HTML and `sitemap.xml` after the bundle (static-pages.ts).
  plugins: [react(), staticPages()],
  resolve: { alias: jackiohAliases },
  server: {
    port: 5173,
    // The card data (`crates/cards/*.json`, the patch history) lives outside this package; name the
    // repository root so the dev server may serve it.
    fs: { allow: [REPO] },
  },
  // The generated glue fetches its own `.wasm` with `new URL(…, import.meta.url)`; pre-bundling it
  // would move it away from the file it fetches.
  optimizeDeps: { exclude: ["./src/wasm/pkg/jackioh_wasm.js"] },
  // The practice worker (`src/practice/practice.worker.ts`) is a module worker; "es" keeps its
  // chunk an ES module, so a dynamic import inside it cannot break the build the way it would an
  // IIFE worker bundle.
  worker: { format: "es" },
  build: {
    // Rollup warns about nothing else, so keep warnings fatal.
    rollupOptions: { onwarn: (warning, warn) => warn(warning) },
  },
});
