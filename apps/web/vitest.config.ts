import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

import { jackiohAliases } from "./vite.config.ts";

export default defineConfig({
  plugins: [react()],
  // The `@jackioh/*` specifiers resolve to the client's wire layer, as in the page's build
  // (docs/v0.3.0/SURFACE.md §10.4).
  resolve: { alias: jackiohAliases },
  test: {
    name: "web",
    environment: "jsdom",
    globals: false,
    // Loads the engine's WebAssembly module from disk before any test file (`src/test/setup.ts`).
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
