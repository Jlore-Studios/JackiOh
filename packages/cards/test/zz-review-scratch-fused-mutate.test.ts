import { fusedIdParts, legalActions } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario } from "./_harness";

const MUTATE = "classic-078";
const THRIVE = "classic-081";
const WELL = "core-006";
const VANILLA = "core-008";
const FILLER = "core-005";
const X = "core-020";
const lib = (n: number) => Array.from({ length: n }, () => X);
const at = (id: string): Selection[] => [{ pick: "instance", instanceId: id }];

describe("scratch: fused Radiant Mutate Spell", () => {
  it("fuse branch prompt on a fused Mutate", () => {
    const s = scenario({
      p1: { hand: [FILLER], backrow: [{ def: MUTATE, radiant: true, counters: { plague: 1 } }], library: lib(8) },
      p2: { hand: [FILLER], backrow: [{ def: THRIVE, counters: { plague: 1 } }, { def: WELL, counters: { plague: 1 } }], library: lib(5) },
    });
    const mutate = s.card(MUTATE).id;
    s.activate(MUTATE, { targets: at(s.card(THRIVE).id) });
    s.answer(mutate);
    expect(fusedIdParts(s.card(mutate).defId)).toEqual([THRIVE, MUTATE]);
    const well = s.card(WELL).id;
    s.activate(mutate, { ability: "mutate", targets: at(well) });
    console.log("pending", JSON.stringify(s.state.pending?.options?.map((o) => o.key)));
    s.answer(mutate);
    const after = s.card(mutate);
    console.log("parts after", JSON.stringify(fusedIdParts(after.defId)), "well exists?", s.state.players.p2.backrow?.length);
    expect(s.state.pending).toBeNull();
  });

  it("forced attack x2 on a fused Mutate", () => {
    const s = scenario({
      p1: { hand: [FILLER], field: [{ def: VANILLA, counters: { plague: 1 } }], backrow: [{ def: MUTATE, radiant: true, counters: { plague: 1 } }], library: lib(8) },
      p2: { hand: [FILLER], backrow: [{ def: THRIVE, counters: { plague: 1 } }], library: lib(5) },
    });
    const mutate = s.card(MUTATE).id;
    const unit = s.card(VANILLA).id;
    s.activate(MUTATE, { targets: at(s.card(THRIVE).id) });
    s.answer(mutate);
    s.activate(mutate, { ability: "mutate", targets: at(unit) });
    const attacks = s.lastEvents.filter((e) => e.type === "attackDeclared");
    console.log("attacks", attacks.length, "p2 hp", s.state.players.p2.hero.health);
    expect(attacks.length).toBe(2);
  });
});
