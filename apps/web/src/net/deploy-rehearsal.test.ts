// crates/server/tests/deploy/rehearse.sh's REHEARSAL_DEPS_ONLY, which CI runs as a job of its own
// before the deploy rehearsal: it builds only the Dockerfile's `deps` stage into the layer cache and
// starts nothing. A stand-in `docker` on PATH records each call, so no Docker is needed.

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

/** The script run deps-only with `env` added: its stdout and every `docker` call, one per line. */
function depsOnly(env: Record<string, string>): { stdout: string; calls: string[] } {
  const log = join(mkdtempSync(join(bin, "run-")), "calls.log");
  writeFileSync(log, "");
  const run = spawnSync("sh", [SCRIPT], {
    encoding: "utf8",
    env: { PATH: `${bin}:${process.env.PATH ?? ""}`, DOCKER_LOG: log, REHEARSAL_DEPS_ONLY: "1", ...env },
  });
  expect(run.status, run.stderr).toBe(0);
  return { stdout: run.stdout, calls: readFileSync(log, "utf8").trim().split("\n") };
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

  it("keeps the cook in a deps stage that the build stage builds on", () => {
    const dockerfile = readFileSync(DOCKERFILE, "utf8");
    const at = [
      "\nFROM chef AS deps\n",
      "\nRUN cargo chef cook ",
      "\nFROM deps AS build\n",
      "\nRUN cargo build --release ",
    ].map((needle) => dockerfile.indexOf(needle));
    for (const index of at) expect(index).toBeGreaterThanOrEqual(0);
    expect(at).toEqual([...at].sort((a, b) => a - b));
    expect(new Set(at).size).toBe(at.length);
  });
});
