// Holds wrangler.jsonc to what the Worker and the server expect of it, in both environments.

import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import JSON5 from "json5";
import { describe, expect, it } from "vitest";

import { HEALTH_PATH, MODULE_CACHE_CONTROL, READY_PATH, SERVER_SECRETS, SERVER_VARS, WORKER_ROUTES } from "./routing.ts";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "../../..");

interface WorkerConfig {
  name: string;
  workers_dev: boolean;
  preview_urls: boolean;
  assets: { directory: string; binding: string; run_worker_first: string[] };
  containers: { name: string; class_name: string; image: string; image_build_context: string; max_instances: number }[];
  durable_objects: { bindings: { name: string; class_name: string }[] };
  migrations: { tag: string; new_sqlite_classes: string[] }[];
  triggers: { crons: string[] };
  vars: Record<string, string>;
}

const config = JSON5.parse(readFileSync(join(ROOT, "wrangler.jsonc"), "utf8")) as WorkerConfig & {
  env: { staging: WorkerConfig };
};

const ENVIRONMENTS = [
  { label: "production", worker: config, name: "jackioh", origin: "https://jackioh.com" },
  { label: "staging", worker: config.env.staging, name: "jackioh-staging", origin: "https://staging.jackioh.com" },
] as const;

describe.each(ENVIRONMENTS)("wrangler.jsonc, $label", ({ worker, name, origin }) => {
  it("is its own Worker, serving the web build", () => {
    expect(worker.name).toBe(name);
    expect(worker.assets.directory).toBe("./apps/web/dist");
  });

  it("runs the Worker for the server's paths and the engine's compressed module alone", () => {
    expect(worker.assets.run_worker_first).toEqual([...WORKER_ROUTES]);
  });

  it("#3183 binds the web build's files as ASSETS, which the Worker serves the module from", () => {
    expect(worker.assets.binding).toBe("ASSETS");
  });

  it("runs exactly one container, of the server's image, behind the Server object", () => {
    expect(worker.containers).toHaveLength(1);
    const [container] = worker.containers;
    expect(container?.max_instances).toBe(1);
    expect(container?.class_name).toBe("Server");
    expect(existsSync(join(ROOT, container?.image ?? ""))).toBe(true);
    expect(container?.image_build_context).toBe(".");
    expect(worker.durable_objects.bindings).toEqual([{ name: "SERVER", class_name: "Server" }]);
    expect(worker.migrations).toEqual([{ tag: "v1", new_sqlite_classes: ["Server"] }]);
    expect(worker.triggers.crons).toHaveLength(1);
  });

  it("holds the server's settings that are not secrets, its own origin among them, and no secret", () => {
    expect(Object.keys(worker.vars).sort()).toEqual([...SERVER_VARS].sort());
    expect(worker.vars.PUBLIC_ORIGINS).toBe(origin);
    for (const secret of SERVER_SECRETS) expect(worker.vars).not.toHaveProperty(secret);
  });
});

describe("wrangler.jsonc, staging behind Access", () => {
  it("has no workers.dev or preview address, which would be a way around Access", () => {
    expect(config.env.staging.workers_dev).toBe(false);
    expect(config.env.staging.preview_urls).toBe(false);
  });

  it("names its container apart from production's", () => {
    expect(config.env.staging.containers[0]?.name).not.toBe(config.containers[0]?.name);
  });
});

describe("the server's probes the Worker calls", () => {
  // scripts/ci-scope.sh runs the web job, which runs these, for crates/server/src/app.rs.
  const routes = readFileSync(join(ROOT, "crates/server/src/app.rs"), "utf8");

  it.each([HEALTH_PATH, READY_PATH])("the server routes %s, unauthenticated", (path) => {
    expect(routes).toContain(`("GET", "${path}", AuthLevel::None,`);
  });
});

describe("#3183 the module's cache rule", () => {
  // The Worker gives the module `_headers`' rule for every built file should the binding's answer lack it.
  it("is the one public/_headers gives /assets/*", () => {
    const headers = readFileSync(join(ROOT, "apps/web/public/_headers"), "utf8");
    const rule = /^\/assets\/\*\n\s+Cache-Control: (.+)$/mu.exec(headers)?.[1];
    expect(rule).toBe(MODULE_CACHE_CONTROL);
  });
});
