// crates/server/tests/deploy/rehearse.sh's split modes (#1556), which CI runs as jobs of their own:
// REHEARSAL_STAGE builds one Dockerfile stage (`deps`, then `crates`) into the layer cache and starts
// nothing (REHEARSAL_DEPS_ONLY=1 is `deps`); REHEARSAL_STAGE=image builds the whole image and saves it
// with REHEARSAL_SAVE_IMAGE; REHEARSAL_LOAD_IMAGE with REHEARSAL_BOOT runs one boot shard on it.
// A stand-in `docker` on PATH records each call, so no Docker is needed. Only the validation and the
// build/save paths are tested here: past the build the script needs real `psql`/Postgres (the
// readiness loop would burn 60 s in vitest), so the full boot sequencing is proven by CI's own
// rehearsal jobs and the daily full run instead.

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

/** The script run deps-only with `env` added: its stdout and every `docker` call, one per line. */
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

  it("refuses a REHEARSAL_STAGE that is neither deps, crates nor image, before any docker call", () => {
    const { status, stderr, calls } = rehearse({ REHEARSAL_STAGE: "build" });
    expect(status).not.toBe(0);
    expect(stderr).toContain("REHEARSAL_STAGE is deps, crates or image, not build");
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

  it("keeps the cook in a deps stage, the workspace's crates in a crates stage on it, and the binary on that", () => {
    const dockerfile = readFileSync(DOCKERFILE, "utf8");
    const at = [
      "\nFROM chef AS deps\n",
      "\nRUN cargo chef cook ",
      "\nFROM deps AS crates\n",
      "\nRUN cargo build --release --locked -p jackioh-server --lib\n",
      "\nFROM crates AS build\n",
      "\nRUN cargo build --release --locked -p jackioh-server \\\n",
    ].map((needle) => dockerfile.indexOf(needle));
    for (const index of at) expect(index).toBeGreaterThanOrEqual(0);
    expect(at).toEqual([...at].sort((a, b) => a - b));
    expect(new Set(at).size).toBe(at.length);
    const buildAt = at[4] ?? -1;
    const build = dockerfile.slice(buildAt, dockerfile.indexOf("\nFROM ", buildAt + 1));
    expect(build).not.toContain("\nCOPY ");
  });
});
