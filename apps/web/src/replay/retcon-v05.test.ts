// R2532000, issue #2532: the v0.5 retcon renamed the v0.5 line a second time, and its old and new names
// overlap (the old v0.5.7 is Training Grounds' release, the new v0.5.7 the old v0.5.11), so a file's
// version says which scheme names its catalog: 1 (no marker) the names before #1371, 2 the v0.5 line
// as it was before #2532, 3 today's. This file spells old names on purpose, so the retcon's rewrite
// leaves it alone (`crates/tools/src/retcon.rs`, `SKIP_FILES`).

import { versionNameFrom } from "@jackioh/cards";
import { describe, expect, it, vi } from "vitest";

import { DUOS_REPLAY_JSON_VERSION, duosReplayJson, parseDuosReplayJson } from "../duos/download.ts";
import { catalogOf } from "../practice/core.ts";
import { duosView } from "../test/fixtures.ts";
import { foldedReplaySteps } from "../test/replay.ts";
import { REPLAY_JSON_FORMAT, REPLAY_JSON_VERSION, replayStepsOf } from "./download.ts";
import { ReplayFailure } from "./source.ts";

// The build as the v0.5 retcon left it, its catalog the old v0.5.11's under its new name. A file opens
// only while the version it maps to is live, so the files below are read under that build.
vi.mock("@jackioh/cards", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@jackioh/cards")>()),
  CATALOG_VERSION: "v0.5.7",
}));

const steps = foldedReplaySteps();
const OTHER_PATCH = "This replay file is from another patch and cannot be opened here.";

function json(version: number, catalogVersion: string): Record<string, unknown> {
  return { format: REPLAY_JSON_FORMAT, version, catalogVersion, savedAt: 0, game: {}, steps };
}

function refused(read: () => unknown): string {
  try {
    read();
  } catch (failure) {
    if (failure instanceof ReplayFailure) return failure.message;
    throw failure;
  }
  throw new Error("the file was not refused");
}

describe("R2532000 a file of scheme 2 names the v0.5 line as it was before the v0.5 retcon", () => {
  it("R2532000 each scheme's name goes by today's", () => {
    expect(versionNameFrom(2, "v0.5.11")).toBe("v0.5.7");
    expect(versionNameFrom(2, "v0.5.7")).toBe("v0.5.5");
    expect(versionNameFrom(2, "v0.4.0")).toBe("v0.4.0");
    expect(versionNameFrom(3, "v0.5.7")).toBe("v0.5.7");
    expect(versionNameFrom(1, "v0.3.17b")).toBe("v0.4.0d");
  });

  it("R2532000 a version 2 file of the old v0.5.11 opens, and one of the old v0.5.7 is another patch", () => {
    expect(REPLAY_JSON_VERSION).toBe(3);
    expect(replayStepsOf(json(2, "v0.5.11"))).toHaveLength(steps.length);
    expect(refused(() => replayStepsOf(json(2, "v0.5.7")))).toBe(OTHER_PATCH);
    expect(replayStepsOf(json(REPLAY_JSON_VERSION, "v0.5.7"))).toHaveLength(steps.length);
    expect(refused(() => replayStepsOf(json(REPLAY_JSON_VERSION + 1, "v0.5.7")))).not.toBe(OTHER_PATCH);
  });

  it("R2532000 a version 2 co-op file of the old v0.5.11 opens", () => {
    const file = duosReplayJson([duosView(0), duosView(1)]);
    expect(file.version).toBe(DUOS_REPLAY_JSON_VERSION);
    expect(parseDuosReplayJson({ ...file, version: 2, catalogVersion: "v0.5.11" })).toHaveLength(2);
    expect(refused(() => parseDuosReplayJson({ ...file, version: 2, catalogVersion: "v0.5.7" }))).toBe(OTHER_PATCH);
  });

  it("R2532000 a practice save or kept game marked 2 is read through the map", () => {
    expect(catalogOf({ catalog: "v0.5.11", versions: 2 })).toBe("v0.5.7");
    expect(catalogOf({ catalog: "v0.5.7", versions: 2 })).toBe("v0.5.5");
    expect(catalogOf({ catalog: "v0.5.7", versions: 3 })).toBe("v0.5.7");
  });
});
