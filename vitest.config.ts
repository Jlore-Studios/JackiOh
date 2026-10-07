import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // The web's unit tests (apps/web/vitest.config.ts, project `web`) are the one vitest project:
    // the rules, the cards, the AI and the server are tested by `cargo test`.
    projects: ["apps/web"],
  },
});
