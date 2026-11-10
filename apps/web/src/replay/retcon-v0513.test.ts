// R3631001, issue #3631: the v0.5.13 retcon named the eight micros after v0.5.12z by R3631000's letters
// (the old v0.5.13 is v0.5.12aa, the old v0.5.13h v0.5.12ah), and the next patch takes v0.5.13 again, so
// a file's version says which scheme names its catalog: 3 the v0.5.13 line as it was, 4 today's. This
// file spells old names on purpose, so the retcon's rewrite leaves it alone (`crates/tools/src/retcon.rs`,
// `SKIP_FILES`).

import { versionNameFrom } from "@jackioh/cards";
import { describe, expect, it, vi } from "vitest";

import { DUOS_REPLAY_JSON_VERSION, duosReplayJson, parseDuosReplayJson } from "../duos/download.ts";
import { duosView } from "../test/fixtures.ts";
import { foldedReplaySteps } from "../test/replay.ts";
import { VERSIONS_SCHEME } from "../wire/engineConfig.ts";
import { REPLAY_JSON_FORMAT, REPLAY_JSON_VERSION, replayStepsOf } from "./download.ts";
import { ReplayFailure } from "./source.ts";

/** R1371000, R3631001: the catalog a kept game or save was played on, as it is named now. */
const catalogOf = (kept: { catalog: string; versions?: number }): string => versionNameFrom(kept.versions ?? 1, kept.catalog);

// The build as the v0.5.13 retcon left it, its catalog the old v0.5.13h's under its new name. A file
// opens only while the version it maps to is live, so the files below are read under that build.
vi.mock("@jackioh/cards", async (importOriginal) => ({
  ...(await importOriginal<typeof import("@jackioh/cards")>()),
  CATALOG_VERSION: "v0.5.12ah",
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

describe("R3631001 a file of scheme 3 names the v0.5.13 line as it was before the v0.5.13 retcon", () => {
  it("R3631001 each scheme's name goes by today's", () => {
    expect(versionNameFrom(3, "v0.5.13")).toBe("v0.5.12aa");
    expect(versionNameFrom(3, "v0.5.13h")).toBe("v0.5.12ah");
    expect(versionNameFrom(3, "v0.5.12z")).toBe("v0.5.12z");
    expect(versionNameFrom(2, "v0.5.11")).toBe("v0.5.7");
    expect(versionNameFrom(VERSIONS_SCHEME, "v0.5.13")).toBe("v0.5.13");
  });

  it("R3631001 a version 3 file of the old v0.5.13h opens, and a version 4 one naming v0.5.13h is another patch", () => {
    expect(VERSIONS_SCHEME).toBe(4);
    expect(REPLAY_JSON_VERSION).toBe(VERSIONS_SCHEME);
    expect(replayStepsOf(json(3, "v0.5.13h"))).toHaveLength(steps.length);
    expect(refused(() => replayStepsOf(json(3, "v0.5.13g")))).toBe(OTHER_PATCH);
    expect(replayStepsOf(json(REPLAY_JSON_VERSION, "v0.5.12ah"))).toHaveLength(steps.length);
    expect(refused(() => replayStepsOf(json(REPLAY_JSON_VERSION, "v0.5.13h")))).toBe(OTHER_PATCH);
  });

  it("R3631001 a version 3 co-op file of the old v0.5.13h opens", () => {
    const file = duosReplayJson([duosView(0), duosView(1)]);
    expect(file.version).toBe(DUOS_REPLAY_JSON_VERSION);
    expect(DUOS_REPLAY_JSON_VERSION).toBe(VERSIONS_SCHEME);
    expect(parseDuosReplayJson({ ...file, version: 3, catalogVersion: "v0.5.13h" })).toHaveLength(2);
    expect(refused(() => parseDuosReplayJson({ ...file, version: VERSIONS_SCHEME, catalogVersion: "v0.5.13h" }))).toBe(OTHER_PATCH);
  });

  it("R3631001 a practice save or kept game marked 3 is read through the map", () => {
    expect(catalogOf({ catalog: "v0.5.13b", versions: 3 })).toBe("v0.5.12ab");
    expect(catalogOf({ catalog: "v0.5.13", versions: 4 })).toBe("v0.5.13");
  });
});
