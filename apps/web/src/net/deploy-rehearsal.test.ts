// crates/server/tests/deploy/rehearse.sh's split modes (#1556), which CI runs as jobs of their own, and its
// space-separated REHEARSAL_CACHE_FROM (#1787). A stand-in `docker` on PATH records each call, so no Docker
// is needed. Only the validation and the build/save paths are tested here: past the build the script needs
// real `psql`/Postgres (the readiness loop would burn 60 s in vitest), so CI's rehearsal jobs prove the boot.

import { spawnSync } from "node:child_process";
import { chmodSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { afterAll, beforeAll, describe, expect, it } from "vitest";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = join(HERE, "../../../..");
const SCRIPT = join(ROOT, "crates/server/tests/deploy/rehearse.sh");
const DOCKERFILE = join(ROOT, "crates/server/Dockerfile");

let bin = "";

beforeAll(() => {
  bin = mkdtempSync(join(tmpdir(), "rehearse-"));
  writeFileSync(join(bin, "docker"), '#!/bin/sh\nprintf "%s\\n" "$*" >> "$DOCKER_LOG"\n');
  chmodSync(join(bin, "docker"), 0o755);
});

afterAll(() => rmSync(bin, { recursive: true, force: true }));

/** The script run with `env` added: its exit status, its output and every `docker` call, one per line. */
function rehearse(env: Record<string, string>): { status: number | null; stdout: string; stderr: string; calls: string[] } {
  const log = join(mkdtempSync(join(bin, "run-")), "calls.log");
  writeFileSync(log, "");
  const run = spawnSync("sh", [SCRIPT], {
    encoding: "utf8",
    env: { PATH: `${bin}:${process.env.PATH ?? ""}`, DOCKER_LOG: log, ...env },
  });
  const calls = readFileSync(log, "utf8").trim().split("\n").filter(Boolean);
  return { status: run.status, stdout: run.stdout, stderr: run.stderr, calls };
}

function depsOnly(env: Record<string, string>): { stdout: string; calls: string[] } {
  const run = rehearse({ REHEARSAL_DEPS_ONLY: "1", ...env });
  expect(run.status, run.stderr).toBe(0);
  return { stdout: run.stdout, calls: run.calls };
}

const builds = (calls: string[]): string[] => calls.filter((call) => /^(buildx )?build /.test(call));

describe("crates/server/tests/deploy/rehearse.sh", () => {
  it("builds only the deps stage through the cache and starts no container", () => {
    const { stdout, calls } = depsOnly({
      REHEARSAL_CACHE_FROM: "type=gha,scope=t",
      REHEARSAL_CACHE_TO: "type=gha,mode=max,scope=t",
    });
    const [build] = builds(calls);
    expect(builds(calls)).toHaveLength(1);
    expect(build).toMatch(/^buildx build --target deps /);
    expect(build).toContain("--cache-from type=gha,scope=t");
    expect(build).toContain("--cache-to type=gha,mode=max,scope=t");
    expect(build).not.toContain("--load");
    expect(build).not.toContain(" -t ");
    expect(calls.filter((call) => call.startsWith("run "))).toEqual([]);
    expect(stdout).toContain("the deps stage is built and cached");
  });

  it("builds only the deps stage with plain docker build when no cache is given", () => {
    const { calls } = depsOnly({});
    const [build] = builds(calls);
    expect(builds(calls)).toHaveLength(1);
    expect(build).toMatch(/^build --target deps -f /);
    expect(calls.filter((call) => call.startsWith("buildx") || call.startsWith("run "))).toEqual([]);
  });

  it("builds only the game stage through the cache and starts no container", () => {
    const { status, stdout, calls } = rehearse({
      REHEARSAL_STAGE: "game",
      REHEARSAL_CACHE_FROM: "type=gha,scope=t",
      REHEARSAL_CACHE_TO: "type=gha,mode=max,scope=t",
    });
    const [build] = builds(calls);
    expect(status).toBe(0);
    expect(builds(calls)).toHaveLength(1);
    expect(build).toMatch(/^buildx build --target game /);
    expect(build).toContain("--cache-from type=gha,scope=t");
    expect(build).toContain("--cache-to type=gha,mode=max,scope=t");
    expect(build).not.toContain("--load");
    expect(build).not.toContain(" -t ");
    expect(calls.filter((call) => call.startsWith("run "))).toEqual([]);
    expect(stdout).toContain("the game stage is built and cached");
  });

  it("refuses an image flag with the game stage, before any docker call", () => {
    const { status, stderr, calls } = rehearse({ REHEARSAL_STAGE: "game", REHEARSAL_SAVE_IMAGE: "/tmp/deploy-image.tgz" });
    expect(status).not.toBe(0);
    expect(stderr).toContain("need the built image, not the game stage");
    expect(calls).toEqual([]);
  });

  it("builds only the crates stage through the cache and starts no container", () => {
    const { status, stdout, calls } = rehearse({
      REHEARSAL_STAGE: "crates",
      REHEARSAL_CACHE_FROM: "type=gha,scope=t",
      REHEARSAL_CACHE_TO: "type=gha,mode=max,scope=t",
    });
    const [build] = builds(calls);
    expect(status).toBe(0);
    expect(builds(calls)).toHaveLength(1);
    expect(build).toMatch(/^buildx build --target crates /);
    expect(build).toContain("--cache-from type=gha,scope=t");
    expect(build).toContain("--cache-to type=gha,mode=max,scope=t");
    expect(build).not.toContain("--load");
    expect(build).not.toContain(" -t ");
    expect(calls.filter((call) => call.startsWith("run "))).toEqual([]);
    expect(stdout).toContain("the crates stage is built and cached");
  });

  it("reads each cache REHEARSAL_CACHE_FROM names with a --cache-from of its own (#1787)", () => {
    const { status, stderr, calls } = rehearse({
      REHEARSAL_STAGE: "crates",
      REHEARSAL_CACHE_FROM: "type=gha,scope=deps type=gha,scope=crates",
      REHEARSAL_CACHE_TO: "type=gha,mode=max,scope=crates",
    });
    const [build = ""] = builds(calls);
    expect(status, stderr).toBe(0);
    expect(builds(calls)).toHaveLength(1);
    expect(build).toContain("--cache-from type=gha,scope=deps --cache-from type=gha,scope=crates --cache-to ");
    expect(build.match(/--cache-from /g)).toHaveLength(2);
    expect(build).toContain("--cache-to type=gha,mode=max,scope=crates");
  });

  it("refuses a REHEARSAL_STAGE that is neither deps, game, crates nor image, before any docker call", () => {
    const { status, stderr, calls } = rehearse({ REHEARSAL_STAGE: "build" });
    expect(status).not.toBe(0);
    expect(stderr).toContain("REHEARSAL_STAGE is deps, game, crates or image, not build");
    expect(calls).toEqual([]);
  });

  it("refuses a REHEARSAL_BOOT outside all, migrate or stale, before any docker call", () => {
    const { status, stderr, calls } = rehearse({ REHEARSAL_BOOT: "everything" });
    expect(status).not.toBe(0);
    expect(stderr).toContain("REHEARSAL_BOOT is all, migrate or stale, not everything");
    expect(calls).toEqual([]);
  });

  it("refuses a stage with a boot, and a save with a load, before any docker call", () => {
    const staged = rehearse({ REHEARSAL_STAGE: "image", REHEARSAL_BOOT: "migrate" });
    expect(staged.status).not.toBe(0);
    expect(staged.stderr).toContain("REHEARSAL_STAGE");
    expect(staged.stderr).toContain("REHEARSAL_BOOT");
    expect(staged.calls).toEqual([]);
    const saved = rehearse({
      REHEARSAL_SAVE_IMAGE: "/tmp/deploy-image.tgz",
      REHEARSAL_LOAD_IMAGE: "/tmp/deploy-image.tgz",
    });
    expect(saved.status).not.toBe(0);
    expect(saved.stderr).toContain("REHEARSAL_SAVE_IMAGE");
    expect(saved.stderr).toContain("REHEARSAL_LOAD_IMAGE");
    expect(saved.calls).toEqual([]);
  });

  it("builds the whole image and saves it for REHEARSAL_STAGE=image, starting no container", () => {
    const save = join(mkdtempSync(join(bin, "save-")), "deploy-image.tgz");
    const { status, stdout, calls } = rehearse({
      REHEARSAL_STAGE: "image",
      REHEARSAL_CACHE_FROM: "type=gha,scope=t",
      REHEARSAL_CACHE_TO: "type=gha,mode=max,scope=t",
      REHEARSAL_SAVE_IMAGE: save,
    });
    const [build] = builds(calls);
    expect(status).toBe(0);
    expect(builds(calls)).toHaveLength(1);
    expect(build).toMatch(/^buildx build -t jackioh-server:rehearsal /);
    expect(build).toContain("--cache-from type=gha,scope=t");
    expect(build).toContain("--cache-to type=gha,mode=max,scope=t");
    expect(build).toContain("--load");
    expect(calls.filter((call) => call.startsWith("save "))).toHaveLength(1);
    expect(calls.filter((call) => call.startsWith("run "))).toEqual([]);
    expect(stdout).toContain("image stage is built");
  });

  it("keeps the cook in a deps stage, the engine, cards and AI in a game stage on it, the server's library in a crates stage on that, and the binary last", () => {
    const dockerfile = readFileSync(DOCKERFILE, "utf8");
    const at = [
      "\nFROM chef AS deps\n",
      "\nRUN cargo chef cook ",
      "\nFROM deps AS game\n",
      "\nRUN cargo build --release --locked -p jackioh-server --lib \\\n && cargo clean --release --locked -p jackioh-server\n",
      "\nFROM game AS crates\n",
      "\nCOPY crates/server ./crates/server\n",
      "\nRUN cargo build --release --locked -p jackioh-server --lib\n",
      "\nFROM crates AS build\n",
      "\nRUN cargo build --release --locked -p jackioh-server \\\n",
    ].map((needle) => dockerfile.indexOf(needle));
    for (const index of at) expect(index).toBeGreaterThanOrEqual(0);
    expect(at).toEqual([...at].sort((a, b) => a - b));
    expect(new Set(at).size).toBe(at.length);
    const stage = (from: number): string => dockerfile.slice(from, dockerfile.indexOf("\nFROM ", from + 1));
    const copied = (text: string): string[] => [...text.matchAll(/^(COPY crates\/\S+ \.\/\S+)$/gm)].map((match) => match[1] ?? "");
    const game = stage(at[2] ?? -1);
    const crates = stage(at[4] ?? -1);
    expect(copied(game)).toEqual([
      "COPY crates/engine ./crates/engine",
      "COPY crates/cards ./crates/cards",
      "COPY crates/ai ./crates/ai",
    ]);
    expect(copied(crates)).toEqual(["COPY crates/server ./crates/server"]);
    expect(game).not.toContain("\nCOPY crates ./crates\n");
    expect(crates).not.toContain("\nCOPY crates ./crates\n");
    expect(stage(at[7] ?? -1)).not.toContain("\nCOPY ");
  });
});
