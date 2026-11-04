// @vitest-environment node
// Holds apps/admin/wrangler.jsonc to what the dashboard's deploy expects of it, in both environments.

import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import JSON5 from "json5";
import { describe, expect, it } from "vitest";

import { FORWARDED } from "./worker/routes.ts";

const ADMIN = dirname(fileURLToPath(import.meta.url));
const ROOT = join(ADMIN, "../..");

interface WorkerConfig {
  name: string;
  main?: string;
  workers_dev: boolean;
  preview_urls: boolean;
  observability: { enabled: boolean };
  assets: { directory: string; not_found_handling: string; run_worker_first: string[] };
  services: { binding: string; service: string }[];
  routes?: unknown;
  route?: unknown;
  vars?: unknown;
}

const config = JSON5.parse(readFileSync(join(ADMIN, "wrangler.jsonc"), "utf8")) as WorkerConfig & {
  env: Record<string, WorkerConfig>;
};

/** The site's own Workers, by environment (the repository's wrangler.jsonc). */
const site = JSON5.parse(readFileSync(join(ROOT, "wrangler.jsonc"), "utf8")) as { name: string; env: { staging: { name: string } } };

const ENVIRONMENTS = [
  { label: "production", worker: config, name: "jackioh-admin", siteWorker: site.name },
  { label: "staging", worker: config.env.staging, name: "jackioh-admin-staging", siteWorker: site.env.staging.name },
] as const;

describe("apps/admin/wrangler.jsonc", () => {
  it("runs the Worker in worker/index.ts, and names no environment but staging", () => {
    expect(config.main).toBe("./worker/index.ts");
    expect(existsSync(join(ADMIN, config.main ?? ""))).toBe(true);
    expect(Object.keys(config.env)).toEqual(["staging"]);
  });

  it("binds production to jackioh and staging to jackioh-staging, the site's own Workers", () => {
    expect(site.name).toBe("jackioh");
    expect(site.env.staging.name).toBe("jackioh-staging");
  });
});

describe.each(ENVIRONMENTS)("apps/admin/wrangler.jsonc, $label", ({ worker, name, siteWorker }) => {
  it("is its own Worker", () => {
    expect(worker?.name).toBe(name);
  });

  it("has no workers.dev or preview address, which would be a way around Access", () => {
    expect(worker?.workers_dev).toBe(false);
    expect(worker?.preview_urls).toBe(false);
  });

  it("names no route or custom domain: infra/cloudflare.tf owns the hostname and its Access application", () => {
    expect(worker).not.toHaveProperty("routes");
    expect(worker).not.toHaveProperty("route");
  });

  it("holds no settings: the server's are the site's Worker's", () => {
    expect(worker).not.toHaveProperty("vars");
  });

  it("logs to Workers observability", () => {
    expect(worker?.observability).toEqual({ enabled: true });
  });

  it("serves the dashboard's build as a single-page app and runs the Worker for /api/* alone", () => {
    expect(worker?.assets).toEqual({
      directory: "./dist",
      not_found_handling: "single-page-application",
      run_worker_first: ["/api/*"],
    });
  });

  it("binds SITE to its environment's site Worker, and nothing else", () => {
    expect(worker?.services).toEqual([{ binding: "SITE", service: siteWorker }]);
  });

  it("runs the Worker for every path it hands on", () => {
    const prefixes = (worker?.assets.run_worker_first ?? []).map((pattern) => pattern.replace(/\*$/u, ""));
    for (const route of FORWARDED) {
      const path = "prefix" in route ? route.prefix : route.path;
      expect(
        prefixes.some((prefix) => path.startsWith(prefix)),
        path,
      ).toBe(true);
    }
  });
});
