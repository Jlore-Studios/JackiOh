// Cloudflare serves the production client as static assets only: no Worker script, no
// runtime variables, no bindings, no secrets. wrangler.jsonc must stay an assets-only
// worker, and public/404.html must exist because not_found_handling asks Cloudflare to
// answer unrouted paths with it. cloudflare-config.test.ts holds the three serving files
// equal to vercel.json; this file holds the serving contract vercel.json cannot express.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");
const PUBLIC = join(HERE, "../../public");

describe("wrangler.jsonc", () => {
  it("declares only static-asset keys: no Worker script, vars, bindings or secrets", () => {
    const text = readFileSync(join(ROOT, "wrangler.jsonc"), "utf8")
      .split("\n")
      .filter((line) => !line.trim().startsWith("//"))
      .join("\n");
    const config = JSON.parse(text) as Record<string, unknown>;
    const allowed = ["$schema", "name", "compatibility_date", "assets"];
    for (const key of Object.keys(config)) {
      expect(allowed, `unexpected wrangler.jsonc top-level key: ${key}`).toContain(key);
    }
    const assets = config["assets"] as { directory?: string; not_found_handling?: string };
    expect(assets.directory).toBe("./apps/web/dist");
    expect(assets.not_found_handling).toBe("404-page");
  });
});

describe("public/404.html", () => {
  it("ships a fallback page for not_found_handling 404-page", () => {
    const page = readFileSync(join(PUBLIC, "404.html"), "utf8");
    expect(page.trim()).not.toBe("");
    expect(page).toContain('href="/"');
  });
});
