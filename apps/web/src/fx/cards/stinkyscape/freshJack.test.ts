import { it } from "vitest";
import type { GameEvent } from "@jackioh/shared";
import { describeRun, planWithOffsets, latestEnd } from "../../../test/signaturePass.ts";
import { fullBoardView } from "../../../test/fixtures.ts";

it("probe", () => {
  const out: string[] = [];
  const view = fullBoardView();
  const fj: GameEvent[] = [
    { type: "cardPlayed", player: "p1", instanceId: "fj", defId: "stinkyscape-109", costPaid: 4 },
    { type: "summoned", player: "p1", instanceId: "fj", defId: "stinkyscape-109", row: "units", lane: 3 },
    { type: "hookResolving", sourceId: "fj", defId: "stinkyscape-109", controller: "p1", hook: "cry" },
    { type: "promptOpened", player: "p1", kind: "discover" } as GameEvent,
    { type: "promptAnswered", player: "p1" },
    { type: "cardAnnounced", player: "p1", instanceId: "bx", defId: "core-050", cardType: "Unit", costPaid: 0, targets: [] },
    { type: "cardPlayed", player: "p1", instanceId: "bx", defId: "core-050", costPaid: 0 },
    { type: "cardResolved", player: "p1", instanceId: "bx", defId: "core-050", permanent: false, costPaid: 0 },
    { type: "cardResolved", player: "p1", instanceId: "fj", defId: "stinkyscape-109", permanent: true, costPaid: 4 },
  ];
  const r1 = planWithOffsets(fj, view);
  out.push("FJ live", String(latestEnd(r1)), describeRun(r1));
  const sw: GameEvent[] = [
    { type: "cardPlayed", player: "p1", instanceId: "sw", defId: "stinkyscape-111", costPaid: 4 },
    { type: "summoned", player: "p1", instanceId: "sw", defId: "stinkyscape-111", row: "units", lane: 3 },
    ...["h1", "h2", "h3", "h4"].map((id): GameEvent => ({ type: "enteredGraveyard", instanceId: id, defId: "core-002", owner: "p1" })),
    ...["n1", "n2", "n3", "n4"].map((id): GameEvent => ({ type: "addedToHand", player: "p1", instanceId: id, defId: "core-003" })),
    ...["o1", "o2", "o3"].map((id): GameEvent => ({ type: "enteredGraveyard", instanceId: id, defId: "core-002", owner: "p2" })),
    ...[1, 2, 3].map((): GameEvent => ({ type: "addedToHand", player: "p2", instanceId: "hidden", defId: "hidden" })),
    { type: "cardResolved", player: "p1", instanceId: "sw", defId: "stinkyscape-111", permanent: true, costPaid: 4 },
  ];
  const r2 = planWithOffsets(sw, view);
  out.push("SW live", String(latestEnd(r2)), describeRun(r2));
  const ik: GameEvent[] = [
    { type: "turnStarted", player: "p1", turn: 5 },
    { type: "hookResolving", sourceId: "ik", defId: "stinkyscape-110", controller: "p1", hook: "startOfTurn" },
    { type: "radiantSet", instanceId: view.you.units[0]?.instanceId ?? "", defId: "core-004", zone: { z: "field", player: "p1", row: "units", lane: 1 } },
  ];
  const r3 = planWithOffsets(ik, view);
  out.push("IK bloom", String(latestEnd(r3)), describeRun(r3));
  for (const { entry, cues } of [...r1, ...r2, ...r3]) out.push(JSON.stringify({ type: entry.type, ev: entry.events.map((e) => e.type), D: entry.durationMs }), JSON.stringify(cues));
  throw new Error(out.join("\n"));
});
