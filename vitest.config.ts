import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // The web's unit tests (apps/web/vitest.config.ts, project `web`) and the Worker's
    // (apps/edge/vitest.config.ts, project `edge`): the rules, the cards, the AI and the server are
    // tested by `cargo test`.
    projects: ["apps/edge", "apps/web"],
  },
});
