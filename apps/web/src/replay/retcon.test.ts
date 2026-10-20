// R1371000, issue #1371: the version retcon renamed every shipped version, and old and new names
// overlap, so a file says which scheme names its catalog. A replay file of format version 1 was
// written before the retcon and names it the old way, read through `aliases.json`; one of version 2
// names it the new way. This file spells old names on purpose, so the retcon's rewrite leaves it alone
// (`crates/tools/src/retcon.rs`, `SKIP_FILES`).

import { CATALOG_VERSION, currentVersionName } from "@jackioh/cards";
import { describe, expect, it } from "vitest";

import { DUOS_REPLAY_JSON_UNMARKED_VERSION, duosReplayJson, parseDuosReplayJson } from "../duos/download.ts";
import { catalogOf } from "../practice/core.ts";
import { duosView } from "../test/fixtures.ts";
import { foldedReplaySteps } from "../test/replay.ts";
import { REPLAY_JSON_FORMAT, REPLAY_JSON_UNMARKED_VERSION, REPLAY_JSON_VERSION, replayStepsOf } from "./download.ts";
import { ReplayFailure } from "./source.ts";

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

describe("R1371000 a file without the versions marker names its catalog the old way", () => {
  it("R1371000 the build is the old v0.3.17's catalog under its new name, v0.4.0c", () => {
    expect(CATALOG_VERSION).toBe("v0.4.0c");
    expect(currentVersionName("v0.3.17")).toBe(CATALOG_VERSION);
    expect(currentVersionName("v0.3.4")).toBe("v0.2.7b");
    expect(currentVersionName("v0.1.0")).toBe("v0.1.0");
  });

  it("R1371000 a version 1 file of the old v0.3.17 opens, and a version 2 file must name the new name", () => {
    expect(replayStepsOf(json(REPLAY_JSON_UNMARKED_VERSION, "v0.3.17"))).toHaveLength(steps.length);
    expect(replayStepsOf(json(REPLAY_JSON_VERSION, CATALOG_VERSION))).toHaveLength(steps.length);
    expect(refused(() => replayStepsOf(json(REPLAY_JSON_VERSION, "v0.3.17")))).toBe(OTHER_PATCH);
  });

  it("R1371000 a version 1 co-op file of the old v0.3.17 opens", () => {
    const file = duosReplayJson([duosView(0), duosView(1)]);
    const old = { ...file, version: DUOS_REPLAY_JSON_UNMARKED_VERSION, catalogVersion: "v0.3.17" };
    expect(parseDuosReplayJson(old)).toHaveLength(2);
    expect(refused(() => parseDuosReplayJson({ ...file, catalogVersion: "v0.3.17" }))).toBe(OTHER_PATCH);
  });

  it("R1371000 a practice save or kept game without the marker is read through the map", () => {
    expect(catalogOf({ catalog: "v0.3.17" })).toBe(CATALOG_VERSION);
    expect(catalogOf({ catalog: "v0.3.4" })).toBe("v0.2.7b");
    expect(catalogOf({ catalog: "v0.3.4", versions: 2 })).toBe("v0.3.4");
  });
});
