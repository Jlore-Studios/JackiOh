import { it } from "vitest";
import { writeFileSync } from "node:fs";
import { describeRun, planWithOffsets } from "../../../test/signaturePass.ts";
import { fullBoardView } from "../../../test/fixtures.ts";
import type { GameEvent } from "@jackioh/shared";
it("scratch", () => {
  const view = fullBoardView();
  const live: GameEvent[] = [
    { type: "cardPlayed", player: "p1", instanceId: "fj", defId: "stinkyscape-109", costPaid: 4 },
    { type: "summoned", player: "p1", instanceId: "fj", defId: "stinkyscape-109", row: "units", lane: 3 },
    { type: "hookResolving", sourceId: "fj", defId: "stinkyscape-109", controller: "p1", hook: "cry" },
    { type: "cardAnnounced", player: "p1", instanceId: "bx", defId: "core-050", cardType: "Unit", costPaid: 0, targets: [] },
    { type: "cardPlayed", player: "p1", instanceId: "bx", defId: "core-050", costPaid: 0 },
    { type: "cardResolved", player: "p1", instanceId: "bx", defId: "core-050", permanent: false, costPaid: 0 },
    { type: "cardResolved", player: "p1", instanceId: "fj", defId: "stinkyscape-109", permanent: true, costPaid: 4 },
  ];
  writeFileSync("/tmp/fj-run.txt", describeRun(planWithOffsets(live, view)) + "\n");
});
