// apps/web/scripts/check-assets.mjs (#3183): no built file may come near Cloudflare Workers' cap on one
// static asset, and the engine ships only compressed. Run on stand-ins for apps/web/dist, and on the client
// itself where one is built: CI's unit tests unpack the `web-build` artifact, which holds it, so every pull
// request the web's jobs run on is held to it before a deploy is.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readdirSync, rmSync, truncateSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { afterEach, describe, expect, it } from "vitest";

const WEB = join(dirname(fileURLToPath(import.meta.url)), "../..");
const SCRIPT = join(WEB, "scripts/check-assets.mjs");
const BUILT = join(WEB, "dist");

let dist = "";

afterEach(() => {
  if (dist !== "") rmSync(dist, { recursive: true, force: true });
  dist = "";
});

function check(site: string): { status: number | null; output: string } {
  const run = spawnSync(process.execPath, [SCRIPT, site], { encoding: "utf8" });
  return { status: run.status, output: run.stdout + run.stderr };
}

/** An empty stand-in for a built site, with its assets folder. */
function site(): string {
  dist = mkdtempSync(join(tmpdir(), "asset-check-"));
  mkdirSync(join(dist, "assets"));
  return dist;
}

/** The size the script refuses from, as it says it on a build that passes: no number restated here. */
function refusedFrom(): number {
  const { status, output } = check(site());
  rmSync(dist, { recursive: true, force: true });
  dist = "";
  expect(status).toBe(0);
  const bytes = /under (\d+) B/u.exec(output)?.[1];
  expect(bytes, output).toBeDefined();
  return Number(bytes);
}

/** A file of `size` bytes that takes no disk (a sparse file). */
function sized(path: string, size: number): void {
  writeFileSync(path, "");
  truncateSync(path, size);
}

describe("#3183 check-assets.mjs", () => {
  it("passes a build whose files are all under the limit and whose engine is compressed", () => {
    const limit = refusedFrom();
    const built = site();
    sized(join(built, "assets", "jackioh_wasm_bg.wasm-abc.br"), limit - 1);
    writeFileSync(join(built, "index.html"), "<!doctype html>");
    expect(check(built).status).toBe(0);
  });

  it("refuses a file at the limit, naming it, wherever it is in the build", () => {
    const limit = refusedFrom();
    const built = site();
    mkdirSync(join(built, "audio"));
    sized(join(built, "audio", "theme.m4a"), limit);
    const { status, output } = check(built);
    expect(status).toBe(1);
    expect(output).toContain(join("audio", "theme.m4a"));
    expect(output).toContain("Cloudflare Workers serves no file over");
  });

  it("refuses an uncompressed WebAssembly module however small it is", () => {
    const built = site();
    writeFileSync(join(built, "assets", "jackioh_wasm_bg-abc.wasm"), "\0asm");
    const { status, output } = check(built);
    expect(status).toBe(1);
    expect(output).toContain("uncompressed WebAssembly module");
  });

  // Where a client is built (CI's unit tests, or a local `vite build`), it is held to the same rule.
  it.skipIf(!existsSync(BUILT))("holds the client this checkout built: under the limit, its engine compressed", () => {
    const { status, output } = check(BUILT);
    expect(status, output).toBe(0);
    const assets = readdirSync(join(BUILT, "assets"));
    // Vite keeps a file's last extension and hashes the rest of its name: `jackioh_wasm_bg.wasm-<hash>.br`,
    // which apps/edge's MODULE_ROUTES glob (`/assets/*.wasm-*.br`) sends to the Worker.
    expect(assets.some((name) => /^jackioh_wasm_bg\.wasm-[\w-]+\.br$/u.test(name)), assets.join(", ")).toBe(true);
  });
});
