import { fusedIdParts, legalActions, subsystems } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario } from "./_harness";

const MUTATE = "classic-078";
const THRIVE = "classic-081";
const PUNISH = "classic-020";
const FILLER = "core-005";
const X = "core-020";
const lib = (n: number) => Array.from({ length: n }, () => X);
const at = (id: string): Selection[] => [{ pick: "instance", instanceId: id }];

describe("scratch: fused card with two Activate abilities", () => {
  it("Thrive + Punish (fused by Radiant Mutate Spell): each 'Activate' once per turn?", () => {
    const s = scenario({
      p1: { hand: [FILLER], backrow: [{ def: MUTATE, radiant: true }, THRIVE], library: lib(5) },
      p2: { hand: [FILLER, FILLER, FILLER], backrow: [{ def: PUNISH, counters: { plague: 1 } }], library: lib(5) },
    });
    const thrive = s.card(THRIVE).id;
    s.activate(MUTATE, { targets: at(s.card(PUNISH).id) });
    s.answer(thrive);
    const fused = s.card(thrive);
    console.log("parts", fusedIdParts(fused.defId));
    console.log("abilities", subsystems.abilitiesOf?.(s.state, fused)?.map((d: { id: string }) => d.id));
    s.activate(thrive, { ability: "thrive", modes: ["draw"] });
    console.log("after thrive: punish listed?", legalActions(s.state, "p1").filter((a) => a.type === "activate" && a.instanceId === thrive));
    console.log("why punish:", subsystems.whyCannotActivateAbility?.(s.state, "p1", thrive, "punish"));
    console.log("view:", JSON.stringify(s.view("p1")).match(/"activations":\[[^\]]*\]/g));
  });

  it("Mutate + Thrive: using the ♾️ ability first blocks the once-per-turn one that was never used", () => {
    const s = scenario({
      p1: { hand: [FILLER], backrow: [{ def: MUTATE, radiant: true, counters: { plague: 2 } }], library: lib(8) },
      p2: { hand: [FILLER], backrow: [{ def: THRIVE, counters: { plague: 1 } }], library: lib(5) },
    });
    const mutate = s.card(MUTATE).id;
    s.activate(MUTATE, { targets: at(s.card(THRIVE).id) });
    s.answer(mutate);
    const fused = s.card(mutate);
    console.log("parts", fusedIdParts(fused.defId), "memory", JSON.stringify(fused.memory));
    // Fresh turn so the use from the fuse activation doesn't count.
    s.endTurn();
    s.endTurn();
    const f2 = s.card(mutate);
    console.log("turn", s.state.turn, "active", s.state.active, "plague", f2.counters.plague, "mem", JSON.stringify(f2.memory));
    s.activate(mutate, { ability: "mutate", targets: at(mutate) });
    console.log("why thrive after one mutate use:", subsystems.whyCannotActivateAbility?.(s.state, "p1", mutate, "thrive"));
  });
});
