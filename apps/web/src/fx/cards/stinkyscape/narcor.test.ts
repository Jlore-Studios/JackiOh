import { it } from "vitest";
import type { GameEvent } from "@jackioh/shared";
import { describeRun, planWithOffsets, latestEnd, galleryRun } from "../../../test/signaturePass.ts";
import { fullBoardView } from "../../../test/fixtures.ts";

it("probe", () => {
  const out: string[] = [];
  const view = fullBoardView();
  for (const id of ["stinkyscape-t-narcor", "stinkyscape-t-red-narcor", "stinkyscape-153"]) {
    const play = (x: GameEvent[]): GameEvent[] => [{ type: "cardPlayed", player: "p1", instanceId: "pc", defId: id, costPaid: 0 }, ...x, { type: "cardResolved", player: "p1", instanceId: "pc", defId: id, permanent: false, costPaid: 0 }];
    const run = planWithOffsets(play([{ type: "manaChanged", player: "p1", current: 6, max: 4 }, { type: "drawn", player: "p1", instanceId: "d1", defId: "core-003" }, { type: "addedToHand", player: "p1", instanceId: "a1", defId: "core-003" }]), view);
    out.push(id, String(latestEnd(run)), describeRun(run));
    for (const { entry } of run) out.push(JSON.stringify({ ev: entry.events.map((e) => e.type), D: entry.durationMs, sp: entry.setPiece, ent: entry.entrance }));
  }
  throw new Error(out.join("\n"));
});
