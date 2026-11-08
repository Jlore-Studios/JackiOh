import { describe, expect, it } from "vitest";

import { describeRun, galleryRun, planWithOffsets, latestEnd } from "../../../test/signaturePass.ts";
import { fullBoardView } from "../../../test/fixtures.ts";

describe("scratch", () => {
  it("prints", () => {
    for (const id of ["stinkyscape-112", "stinkyscape-112-1", "stinkyscape-115", "stinkyscape-116"]) {
      const { run, events } = galleryRun(id);
      console.log(id, latestEnd(run), events.map((e) => e.type).join(","), "\n" + describeRun(run));
    }
    const view = fullBoardView();
    const pc = "stinkyscape-112-1";
    for (const x of [{ type: "manaChanged", player: "p1", current: 4, max: 4 }, { type: "marketCreditsChanged", player: "p1", value: 20 }] as const) {
      const run = planWithOffsets([{ type: "cardPlayed", player: "p1", instanceId: "pc", defId: pc, costPaid: 0 }, x, { type: "cardResolved", player: "p1", instanceId: "pc", defId: pc, permanent: false, costPaid: 0 }], view);
      console.log(x.type, latestEnd(run), "\n" + describeRun(run));
    }
    expect(1).toBe(1);
  });
});
