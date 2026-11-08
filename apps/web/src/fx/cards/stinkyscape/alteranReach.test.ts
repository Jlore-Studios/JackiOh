import { writeFileSync } from "node:fs";
import { describe, it } from "vitest";
const out: string[] = [];
const log = (...a: unknown[]) => out.push(a.join(" "));
import { describeRun, galleryRun, planWithOffsets } from "../../../test/signaturePass.ts";
import { fullBoardView } from "../../../test/fixtures.ts";

describe("probe", () => {
  it("probe", () => {
    for (const id of ["stinkyscape-148", "stinkyscape-148-1", "stinkyscape-153", "stinkyscape-t-narcor", "stinkyscape-t-red-narcor"]) {
      const { run, events } = galleryRun(id);
      log(id, JSON.stringify(events.map((e) => e.type)), "\n" + describeRun(run));
    }
    const v = fullBoardView();
    log(v.viewer, JSON.stringify(Object.keys(v.you)), JSON.stringify(v.you.backrow.map((b) => b && b.instanceId)));
    const run = planWithOffsets([
      { type: "hookResolving", sourceId: "x", defId: "stinkyscape-148", controller: "p1", hook: "activate" },
      { type: "starscapeTravelled", player: "p1", node: 20, reward: "edge" },
      { type: "addedToHand", player: "p1", instanceId: "e", defId: "stinkyscape-148-1" },
      { type: "manaChanged", player: "p1", current: 3, max: 5 } as never,
      { type: "drawn", player: "p1", instanceId: "e2", defId: "core-001" } as never,
    ], v);
    log(describeRun(run));
    writeFileSync("/tmp/probe-3407-mine.txt", out.join("\n"));
  });
});
