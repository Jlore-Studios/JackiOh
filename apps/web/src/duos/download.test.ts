// R1242000, issue #1242: a finished co-op replay downloads as one JSON file of the caller's seat's
// views, and opens again from it. `duosReplayJson` holds only the format, the version, the catalog
// version, the seat and each step's index, round and view; `parseDuosReplayJson` reads that shape
// back under the 1v1 file's caps and sentences.

import { CATALOG_VERSION } from "@jackioh/cards";
import { describe, expect, it } from "vitest";

import { REPLAY_JSON_MAX_BYTES, REPLAY_JSON_MAX_STEPS } from "../replay/config.ts";
import { REPLAY_JSON_FORMAT, parseReplayJsonFile, readReplayJsonFile } from "../replay/download.ts";
import { ReplayFailure } from "../replay/source.ts";
import { duosView } from "../test/fixtures.ts";
import type { DuosReplaySteps } from "./DuosReplay.tsx";
import { DUOS_REPLAY_JSON_FORMAT, DUOS_REPLAY_JSON_VERSION, duosReplayJson, duosReplayJsonFile, parseDuosReplayJson } from "./download.ts";

const STEPS: DuosReplaySteps = [duosView(0), duosView(1), duosView(1), duosView(2), duosView(3, { outcome: "win", winner: "b", reason: "concede" })];

/** A file's JSON with `over` set on top, as the page reads it. */
function fileJson(over: Record<string, unknown>): Record<string, unknown> {
  return { ...duosReplayJson(STEPS), ...over };
}

/** The sentence `json` is refused with. */
function refusal(json: Record<string, unknown>): string {
  let message: string | null = null;
  try {
    parseDuosReplayJson(json);
  } catch (cause: unknown) {
    if (!(cause instanceof ReplayFailure)) throw new Error("the file was refused without a ReplayFailure", { cause });
    expect(cause.reason).toBe("error");
    message = cause.message;
  }
  if (message === null) throw new Error("the file was not refused");
  return message;
}

/** A step of no weight, for a file of many. */
function bare(index: number): unknown {
  return { step: index, round: 0, view: { seat: "a1", round: 0 } };
}

describe("R1242000 downloading a co-op replay", () => {
  it("R1242000 the file holds the format, version, catalog version, seat and each step's index, round and view, and nothing else", () => {
    const json = duosReplayJson(STEPS);
    expect(Object.keys(json).sort()).toEqual(["catalogVersion", "format", "seat", "steps", "version"]);
    expect(json).toMatchObject({ format: DUOS_REPLAY_JSON_FORMAT, version: DUOS_REPLAY_JSON_VERSION, catalogVersion: CATALOG_VERSION, seat: "a1" });
    expect(json.steps.map((step) => step.step)).toEqual(json.steps.map((_, index) => index));
    for (const step of json.steps) {
      expect(Object.keys(step).sort()).toEqual(["round", "step", "view"]);
      expect(step.round).toBe(step.view.round);
    }
    expect(json.steps.map((step) => step.view)).toEqual([...STEPS]);
    expect(Object.keys(json.steps[0]?.view ?? {}).sort()).toEqual([
      "concedeOfferedBy",
      "enemy",
      "own",
      "result",
      "round",
      "roundSeats",
      "seat",
      "team",
      "teammate",
    ]);
  });

  it("R1242000 nothing but the seat's views leaves: no seed, log, deck, account, name, ping or match id", async () => {
    const tainted = STEPS.map((view) => ({
      ...view,
      seed: "duos-seed-7",
      log: [],
      decks: {},
      pings: [{ kind: "careful" }],
      host: { profileId: "profile-a1", username: "teammate-name" },
      matchId: "dm-secret",
    })) as unknown as DuosReplaySteps;
    const file = duosReplayJsonFile(tainted);
    expect(file.name).toBe("jackioh-duos-replay.json");
    expect(file.type).toBe("application/json");
    const text = await file.text();
    expect(JSON.parse(text)).toEqual(duosReplayJson(STEPS));
    expect(text).not.toContain("duos-seed-7");
    expect(text).not.toContain("dm-secret");
    expect(text).not.toContain("profile-a1");
    expect(text).not.toContain("teammate-name");
    expect(text).not.toMatch(/"(seed|log|decks|handicaps|state|pings|profileId|username|accountId|matchId)":/u);
  });

  it("R1242000 a file duosReplayJsonFile wrote opens again as the same views, in order", async () => {
    expect(parseDuosReplayJson(await readReplayJsonFile(duosReplayJsonFile(STEPS)))).toEqual([...STEPS]);
  });

  it("R1242000 refuses another format, version or catalog version", async () => {
    const unsupported = "This replay file uses an unsupported format.";
    expect(refusal(fileJson({ format: REPLAY_JSON_FORMAT }))).toBe(unsupported);
    expect(refusal(fileJson({ version: 999 }))).toBe(unsupported);
    const otherPatch = "This replay file is from another patch and cannot be opened here.";
    expect(refusal(fileJson({ catalogVersion: "v0.0.0" }))).toBe(otherPatch);
    expect(refusal(fileJson({ catalogVersion: undefined }))).toBe(otherPatch);
    // The co-op file is no 1v1 replay.
    const failure: unknown = await parseReplayJsonFile(duosReplayJsonFile(STEPS)).then(
      () => null,
      (cause: unknown) => cause,
    );
    expect(failure).toBeInstanceOf(ReplayFailure);
    expect((failure as ReplayFailure).message).toBe(unsupported);
  });

  it("R1242000 refuses a seat that is no Duos seat, and steps missing, out of order, out of round or of another seat", () => {
    const not = "This file is not a JackiOh replay file.";
    const base = duosReplayJson(STEPS);
    expect(refusal(fileJson({ seat: "p1" }))).toBe(not);
    expect(refusal(fileJson({ seat: undefined }))).toBe(not);
    expect(refusal(fileJson({ steps: [] }))).toBe(not);
    expect(refusal(fileJson({ steps: "none" }))).toBe(not);
    expect(refusal(fileJson({ steps: base.steps.filter((_, index) => index !== 1) }))).toBe(not);
    expect(refusal(fileJson({ steps: [...base.steps].reverse() }))).toBe(not);
    const [first, second] = base.steps;
    if (first === undefined || second === undefined) throw new Error("the co-op replay is too short");
    expect(refusal(fileJson({ steps: [first, { ...second, round: second.round + 1 }] }))).toBe(not);
    for (const round of [1.5, "1", -1]) {
      expect(refusal(fileJson({ steps: [first, { ...second, round }] })), `round ${String(round)}`).toBe(not);
    }
    expect(
      refusal(
        fileJson({
          steps: [
            { step: 0, round: 2, view: { ...second.view, round: 2 } },
            { step: 1, round: 1, view: { ...second.view, round: 1 } },
          ],
        }),
      ),
    ).toBe(not);
    expect(refusal(fileJson({ steps: [first, { ...second, view: { ...second.view, seat: "a2" } }] }))).toBe(not);
    expect(refusal(fileJson({ steps: [first, { ...second, view: null }] }))).toBe(not);
    expect(refusal(fileJson({ steps: [first, { ...second, view: [] }] }))).toBe(not);
    expect(refusal(fileJson({ steps: [first, "second"] }))).toBe(not);
  });

  it("R1242000 refuses more steps than REPLAY_JSON_MAX_STEPS, and takes exactly the cap", () => {
    const make = (count: number): unknown[] => Array.from({ length: count }, (_, index) => bare(index));
    expect(refusal(fileJson({ steps: make(REPLAY_JSON_MAX_STEPS + 1) }))).toBe("This replay file is too big to open.");
    expect(parseDuosReplayJson(fileJson({ steps: make(REPLAY_JSON_MAX_STEPS) }))).toHaveLength(REPLAY_JSON_MAX_STEPS);
  });

  it("R1242000 a co-op file over REPLAY_JSON_MAX_BYTES is refused unread", async () => {
    const big = "This replay file is too big to open.";
    const unread = new File(["{}"], "jackioh-duos-replay.json");
    Object.defineProperty(unread, "size", { value: REPLAY_JSON_MAX_BYTES + 1 });
    Object.defineProperty(unread, "text", { value: () => Promise.reject(new Error("an oversized file was read")) });
    const failure: unknown = await readReplayJsonFile(unread).then(
      () => null,
      (cause: unknown) => cause,
    );
    if (!(failure instanceof ReplayFailure)) throw new Error("the file was not refused");
    expect(failure.reason).toBe("error");
    expect(failure.message).toBe(big);
  });
});
