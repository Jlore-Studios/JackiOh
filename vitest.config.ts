import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    // The web's unit tests (apps/web/vitest.config.ts, project `web`), the Worker's
    // (apps/edge/vitest.config.ts, project `edge`) and the dev dashboard's (apps/admin/vitest.config.ts,
    // project `admin`, #3074): the rules, the cards, the AI and the server are tested by `cargo test`.
    projects: ["apps/admin", "apps/edge", "apps/web"],
  },
});
