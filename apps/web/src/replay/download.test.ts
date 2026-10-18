// R1141000, issue #1160: a replay file the page wrote opens again. `parseReplayJsonFile` gives back the
// steps `replayJsonFile` was given, in order, and refuses anything but the documented shape of this
// build's catalog, under named caps; `memoryReplaySource` pages them like the source they came from.

import { CATALOG_VERSION } from "@jackioh/cards";
import { describe, expect, it } from "vitest";

import { fakeReplaySource, foldedReplaySteps } from "../test/replay.ts";
import { REPLAY_JSON_MAX_BYTES, REPLAY_JSON_MAX_STEPS } from "./config.ts";
import { REPLAY_JSON_FORMAT, REPLAY_JSON_VERSION, parseReplayJsonFile, practiceGame, replayJsonFile } from "./download.ts";
import { ReplayFailure, memoryReplaySource } from "./source.ts";
import { createStepCache } from "./steps.ts";

const steps = foldedReplaySteps();

/** The file a practice game's download makes, over the folded steps. */
function written(): Promise<File> {
  const game = practiceGame({ game: 1, endedAt: 0, result: "loss", turns: steps.at(-1)?.turn ?? 0, steps: steps.length, portraits: { p1: "vanilla", p2: "vanilla" } });
  return replayJsonFile(game, fakeReplaySource(steps), 0);
}

/** A file of `json` as the page writes it, with `over` set on top. */
function fileOf(over: Record<string, unknown>): File {
  const json = { format: REPLAY_JSON_FORMAT, version: REPLAY_JSON_VERSION, catalogVersion: CATALOG_VERSION, savedAt: 0, game: {}, steps, ...over };
  return new File([JSON.stringify(json)], "replay.json", { type: "application/json" });
}

/** The sentence `file` is refused with. */
async function refusal(file: File): Promise<string> {
  const failure: unknown = await parseReplayJsonFile(file).then(
    () => null,
    (cause: unknown) => cause,
  );
  if (!(failure instanceof ReplayFailure)) throw new Error("the file was not refused");
  expect(failure.reason).toBe("error");
  return failure.message;
}

/** A step of no weight, for a file of many. */
function bare(index: number, turn = 0): unknown {
  return { step: index, turn, view: {} };
}

describe("R1141000 opening a downloaded replay", () => {
  it("R1141000 parses a file replayJsonFile wrote into identical ordered steps", async () => {
    expect(await parseReplayJsonFile(await written())).toEqual([...steps]);
  });

  it("R1141000 the memory source pages verified steps and walks turns like the original", async () => {
    const opened = createStepCache(memoryReplaySource(await parseReplayJsonFile(await written())));
    const original = createStepCache(fakeReplaySource(steps));
    expect(opened.total).toBe(original.total);
    /** The steps a turn move lands on, from `from` until it stops moving. */
    const walk = async (cache: typeof opened, move: "nextTurn" | "previousTurn", from: number): Promise<number[]> => {
      const visited = [from];
      for (;;) {
        const next = await cache[move](visited[visited.length - 1] ?? from);
        if (next === visited[visited.length - 1]) return visited;
        visited.push(next);
      }
    };
    const forward = await walk(original, "nextTurn", 0);
    expect(forward.length).toBeGreaterThan(3);
    expect(await walk(opened, "nextTurn", 0)).toEqual(forward);
    const last = steps.length - 1;
    const backward = await walk(original, "previousTurn", last);
    expect(backward.length).toBeGreaterThan(3);
    expect(await walk(opened, "previousTurn", last)).toEqual(backward);
    // A page holds 1 to `count` steps from `from`, and none past the end.
    const source = memoryReplaySource(steps);
    expect(await source.page(2, 3)).toEqual(steps.slice(2, 5));
    expect(await source.page(steps.length, 3)).toEqual([]);
  });

  it("R1141000 refuses text that is not JSON, and JSON that is not an object", async () => {
    const not = "This file is not a JackiOh replay file.";
    expect(await refusal(new File(["{ not json"], "replay.json"))).toBe(not);
    expect(await refusal(new File(["[1, 2]"], "replay.json"))).toBe(not);
    expect(await refusal(new File(["null"], "replay.json"))).toBe(not);
  });

  it("R1141000 refuses another format or version", async () => {
    const unsupported = "This replay file uses an unsupported format.";
    expect(await refusal(fileOf({ format: "jackioh-deck" }))).toBe(unsupported);
    expect(await refusal(fileOf({ version: 999 }))).toBe(unsupported);
    expect(await refusal(new File(["{}"], "replay.json"))).toBe(unsupported);
  });

  it("R1141000 refuses a file from another catalog version", async () => {
    expect(await refusal(fileOf({ catalogVersion: "v0.0.0" }))).toBe("This replay file is from another patch and cannot be opened here.");
    expect(await refusal(fileOf({ catalogVersion: undefined }))).toBe("This replay file is from another patch and cannot be opened here.");
  });

  it("R1141000 refuses steps that are missing, out of order or malformed", async () => {
    const not = "This file is not a JackiOh replay file.";
    const [first, second, third] = steps;
    if (first === undefined || second === undefined || third === undefined) throw new Error("the folded game is too short");
    expect(await refusal(fileOf({ steps: [] }))).toBe(not);
    expect(await refusal(fileOf({ steps: "none" }))).toBe(not);
    expect(await refusal(fileOf({ steps: [first, third] }))).toBe(not);
    expect(await refusal(fileOf({ steps: [second, first] }))).toBe(not);
    expect(await refusal(fileOf({ steps: [first, { ...second, turn: -1 }] }))).toBe(not);
    expect(await refusal(fileOf({ steps: [first, { ...second, turn: 1.5 }] }))).toBe(not);
    expect(await refusal(fileOf({ steps: [first, { ...second, turn: "1" }] }))).toBe(not);
    expect(await refusal(fileOf({ steps: [bare(0, 2), bare(1, 1)] }))).toBe(not);
    expect(await refusal(fileOf({ steps: [first, { ...second, view: null }] }))).toBe(not);
    expect(await refusal(fileOf({ steps: [first, { ...second, view: [] }] }))).toBe(not);
    expect(await refusal(fileOf({ steps: [first, "second"] }))).toBe(not);
  });

  it("R1141000 refuses more steps than the cap, and takes exactly the cap", async () => {
    const make = (count: number): unknown[] => Array.from({ length: count }, (_, index) => bare(index));
    expect(await refusal(fileOf({ steps: make(REPLAY_JSON_MAX_STEPS + 1) }))).toBe("This replay file is too big to open.");
    expect(await parseReplayJsonFile(fileOf({ steps: make(REPLAY_JSON_MAX_STEPS) }))).toHaveLength(REPLAY_JSON_MAX_STEPS);
  });

  it("R1141000 refuses a file over the byte cap, by its size unread and by its text read", async () => {
    const big = "This replay file is too big to open.";
    const unread = new File(["{}"], "replay.json");
    Object.defineProperty(unread, "size", { value: REPLAY_JSON_MAX_BYTES + 1 });
    Object.defineProperty(unread, "text", { value: () => Promise.reject(new Error("an oversized file was read")) });
    expect(await refusal(unread)).toBe(big);
    // A size that understates its text is not believed.
    const understated = new File(["x".repeat(REPLAY_JSON_MAX_BYTES + 1)], "replay.json");
    Object.defineProperty(understated, "size", { value: 2 });
    expect(await refusal(understated)).toBe(big);
  });

  it("R1141000 opens only what it parsed: a step's extra keys do not come through", async () => {
    const [first] = steps;
    if (first === undefined) throw new Error("the folded game is empty");
    const opened = await parseReplayJsonFile(fileOf({ steps: [{ ...first, seed: "hunter2", log: [] }] }));
    expect(opened).toEqual([first]);
    expect(Object.keys(opened[0] ?? {}).sort()).toEqual(["step", "turn", "view"]);
  });
});
