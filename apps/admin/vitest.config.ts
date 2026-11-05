import react from "@vitejs/plugin-react";
import { defineConfig } from "vitest/config";

export default defineConfig({
  plugins: [react()],
  test: {
    name: "admin",
    environment: "jsdom",
    globals: false,
    setupFiles: ["./src/test/setup.ts"],
    // The page's tests (src/), the Worker's (worker/, node environment), and wrangler.jsonc's and
    // the build's (wrangler.test.ts, headers.test.ts, build.test.ts).
    include: ["src/**/*.test.{ts,tsx}", "worker/**/*.test.ts", "*.test.ts"],
  },
});
