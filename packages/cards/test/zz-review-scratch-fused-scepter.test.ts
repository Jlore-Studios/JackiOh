import { fusedIdParts, legalActions } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario } from "./_harness";

const MUTATE = "classic-078";
const SCEPTER = "classic-007";
const STOCKPILE = "core-005";
const WELL = "core-006";
const VANILLA = "core-008";
const at = (id: string): Selection[] => [{ pick: "instance", instanceId: id }];

describe("scratch: InfiniScepter fused in hand, then played", () => {
  it("its Cry remembers in its ingredient's place and its Activate reads it back", () => {
    const s = scenario({
      p1: { hand: [SCEPTER, STOCKPILE], backrow: [{ def: MUTATE, radiant: true }], library: [VANILLA, VANILLA, VANILLA, VANILLA, VANILLA], health: 20, mana: 4 },
      p2: { hand: [STOCKPILE], backrow: [{ def: WELL, counters: { plague: 1 } }], library: [VANILLA, VANILLA] },
    });
    const scepter = s.card(SCEPTER).id;
    s.activate(MUTATE, { targets: at(s.card(WELL).id) });
    console.log("options", JSON.stringify(s.state.pending?.options?.map((o) => o.key)));
    s.answer(scepter);
    console.log("parts", JSON.stringify(fusedIdParts(s.card(scepter).defId)), "zone", JSON.stringify(s.card(scepter).zone));
    const plays = legalActions(s.state, "p1").filter((a) => a.type === "play" && a.instanceId === scepter);
    console.log("plays", JSON.stringify(plays));
    s.play(scepter, { zone: 2, targets: at(s.card(STOCKPILE).id) });
    console.log("memory", JSON.stringify(s.card(scepter).memory));
    const acts = legalActions(s.state, "p1").filter((a) => a.type === "activate" && a.instanceId === scepter);
    console.log("acts", JSON.stringify(acts));
    expect(acts.length).toBe(1);
  });
});
