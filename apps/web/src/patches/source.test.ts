// R388: the one seam onto packages/cards/patches/. Each file is its own lazy chunk, loaded once and
// only when asked for; the order of patches is patches.json's; no data at all is an empty history.

import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it, vi } from "vitest";

import { EMPTY_PATCH_SOURCE, fileStem, loadersByStem, realPatchSource, sourceFromLoaders, type Loader } from "./source.ts";

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), "../../../..");

function spyLoaders(files: Record<string, unknown>): { loaders: Record<string, Loader>; calls: string[] } {
  const calls: string[] = [];
  const loaders: Record<string, Loader> = {};
  for (const [stem, value] of Object.entries(files)) {
    loaders[stem] = () => {
      calls.push(stem);
      return Promise.resolve(value);
    };
  }
  return { loaders, calls };
}

describe("R388 the patch source", () => {
  it("R388 names a file by its stem, whatever the path the glob hands out", () => {
    expect(fileStem("../../../../packages/cards/patches/v0.1.0d.json")).toBe("v0.1.0d");
    expect(fileStem("/patches/patches.json")).toBe("patches");
    expect(Object.keys(loadersByStem({ "../a/v1.json": () => Promise.resolve(1), "../a/index.json": () => Promise.resolve(2) }))).toEqual([
      "v1",
      "index",
    ]);
  });

  it("R388 loads nothing until asked, then each file once", async () => {
    const { loaders, calls } = spyLoaders({ patches: [{ version: "b", changes: [] }], index: {}, b: { "core-001": {} } });
    const source = sourceFromLoaders(loaders);
    expect(calls).toEqual([]);
    await source.patches();
    await source.patches();
    expect(calls).toEqual(["patches"]);
    expect(await source.snapshot("b")).toEqual({ "core-001": {} });
    await source.snapshot("b");
    expect(calls).toEqual(["patches", "b"]);
    expect(await source.snapshot("missing")).toBeNull();
    // patches.json and index.json are never snapshots.
    expect(await source.snapshot("index")).toBeNull();
    expect(await source.snapshot("patches")).toBeNull();
  });

  it("R388 forgets a failed load, so asking again retries it", async () => {
    let fail = true;
    const patches = vi.fn(() => (fail ? Promise.reject(new Error("offline")) : Promise.resolve([])));
    const source = sourceFromLoaders({ patches });
    await expect(source.patches()).rejects.toThrow("offline");
    fail = false;
    await expect(source.patches()).resolves.toEqual([]);
    expect(patches).toHaveBeenCalledTimes(2);
  });

  it("R388 refuses a file that is not what it should be", async () => {
    await expect(sourceFromLoaders({ patches: () => Promise.resolve({}) }).patches()).rejects.toThrow(/not a list/);
    await expect(sourceFromLoaders({ patches: () => Promise.resolve([{ title: "x" }]) }).patches()).rejects.toThrow(/not a patch/);
    await expect(sourceFromLoaders({ index: () => Promise.resolve([]) }).index()).rejects.toThrow(/not a map/);
    await expect(sourceFromLoaders({ v1: () => Promise.resolve("x") }).snapshot("v1")).rejects.toThrow(/not a catalog snapshot/);
  });

  it("R388 no patch files at all is an empty history, not an error", async () => {
    const source = sourceFromLoaders({});
    expect(await source.patches()).toEqual([]);
    expect(await source.index()).toEqual({});
    expect(await source.snapshot("v0.1.0")).toBeNull();
    expect(await EMPTY_PATCH_SOURCE.patches()).toEqual([]);
  });

  it("R388 the real source reads packages/cards/patches/: the file's order, the index and every snapshot", async () => {
    const patches = await realPatchSource.patches();
    const shipped = JSON.parse(
      readFileSync(resolve(REPO, "packages/cards/patches/patches.json"), "utf8"),
    ) as { version: string }[];
    expect(patches.map((patch) => patch.version)).toEqual(shipped.map((patch) => patch.version));
    expect(shipped.map((patch) => patch.version)).toEqual(
      expect.arrayContaining(["v0.1.0", "v0.1.0b", "v0.1.0c", "v0.1.0d", "v0.1.1", "v0.2.0", "v0.2.4", "v0.2.10"]),
    );
    const index = await realPatchSource.index();
    expect(index["core-065"]).toEqual(["v0.1.0", "v0.1.0d", "v0.1.1", "v0.2.0", "v0.2.10"]);
    for (const patch of patches) {
      const snapshot = await realPatchSource.snapshot(patch.version);
      expect(snapshot, patch.version).not.toBeNull();
      for (const change of patch.changes.filter((entry) => entry.kind !== "removed")) {
        expect(snapshot?.[change.id]?.name, `${patch.version} ${change.id}`).toBe(change.name);
      }
    }
    expect(Object.keys((await realPatchSource.snapshot("v0.2.4")) ?? {})).toHaveLength(317);
  });

  it("R388 the index lists, for every card, exactly the versions whose snapshot differs from the one before", async () => {
    const patches = await realPatchSource.patches();
    const index = await realPatchSource.index();
    const computed: Record<string, string[]> = {};
    let previous: Record<string, unknown> = {};
    for (const patch of patches) {
      const snapshot = (await realPatchSource.snapshot(patch.version)) ?? {};
      for (const id of new Set([...Object.keys(previous), ...Object.keys(snapshot)])) {
        if (JSON.stringify(previous[id]) !== JSON.stringify(snapshot[id])) (computed[id] ??= []).push(patch.version);
      }
      previous = snapshot;
    }
    expect(index).toEqual(computed);
  });
});
