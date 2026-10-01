// C #27 Pestilent Slime — SPEC §8.6 row 27, BUILD M9 Classic row C 27: "Every placement onto it is
// doubled: "place N Plague Tokens on this" puts 2N, and each one-token placement of a split puts 2,
// still one placement for "whenever Plague Tokens are placed" triggers; `counterChanged` shows the
// count; its tokens reset when it leaves (R78); radiant 2/2: tripled; its tuned number (multiplier)
// reads through `param()` (R386)".
//
// The placements come from C #39 Outbreak ("Place {tokens} Plague Tokens on a permanent", one
// placement) and, for the split, C #70 Book of Plague ("Place {tokens} Plague Tokens": one placement
// per token, each a prompt), whose script (cards-classic-b) that case waits for.

import { stepParam } from "@jackioh/engine";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/027-pestilent-slime";

const SLIME = "classic-027";
const OUTBREAK = "classic-039"; // (1) Spell: place {tokens} on a permanent; own → draw per token.
const BOOK_OF_PLAGUE = "classic-070"; // (1) Spell: Place 5 Plague Tokens (5 placements).
const FLOOD = "core-017"; // (4) Spell: Bounce all Units.
const VANILLA = "core-008";
const ANCHOR = "core-010";
const X = "core-020";

type Placement = Extract<GameEvent, { type: "counterChanged" }>;

function placements(s: Scenario, instanceId: string): Placement[] {
  return s.events.filter(
    (event): event is Placement => event.type === "counterChanged" && event.counter === "plague" && event.instanceId === instanceId,
  );
}

function outbreakOn(s: Scenario, instanceId: string): Scenario {
  return s.play(OUTBREAK, { targets: [{ pick: "instance", instanceId }] });
}

describe("C #27 Pestilent Slime", () => {
  it("declares its multiplier as a placement hook, one script on both faces", () => {
    expect(def.id).toBe(SLIME);
    expect(base.plagueMultiplier).toBeTypeOf("function");
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("is a 1/1", () => {
      const s = scenario({ p1: { field: [SLIME], hand: [ANCHOR] } });
      s.expectStats(SLIME, { attack: 1, health: 1 });
    });

    it("R471 a placement of 1 on it puts 2, as ONE placement: one counterChanged carrying placed 2", () => {
      const s = scenario({ p1: { hand: [OUTBREAK, ANCHOR], field: [SLIME], library: [X, X, X] }, p2: { hand: [ANCHOR] } });
      const slime = s.card(SLIME);

      outbreakOn(s, slime.id);

      expect(s.card(slime.id).counters.plague).toBe(2);
      expect(placements(s, slime.id)).toEqual([
        { type: "counterChanged", instanceId: slime.id, counter: "plague", value: 2, placed: 2 },
      ]);
    });

    it("a placement of N puts 2N: a Radiant Outbreak's 2 put 4, on top of what it had", () => {
      const s = scenario({
        p1: { hand: [{ def: OUTBREAK, radiant: true }, ANCHOR], field: [{ def: SLIME, counters: { plague: 1 } }], library: [X, X, X, X, X, X] },
        p2: { hand: [ANCHOR] },
      });
      const slime = s.card(SLIME);

      outbreakOn(s, slime.id);

      expect(s.card(slime.id).counters.plague).toBe(5);
      expect(placements(s, slime.id).map((event) => [event.value, event.placed])).toEqual([[5, 4]]);
    });

    it("whoever places them: the opponent's placement on it is doubled too", () => {
      const s = scenario({
        active: "p2",
        p1: { hand: [ANCHOR], field: [SLIME] },
        p2: { hand: [OUTBREAK, ANCHOR], library: [X, X] },
      });
      const slime = s.card(SLIME);

      outbreakOn(s, slime.id);

      expect(placements(s, slime.id)[0]?.placed).toBe(2);
    });

    it("R471 each one-token placement of a split puts 2, each its own placement", () => {
      const s = scenario({ p1: { hand: [BOOK_OF_PLAGUE, ANCHOR], field: [SLIME, VANILLA] }, p2: { hand: [ANCHOR] } });
      const slime = s.card(SLIME);
      const vanilla = s.card(VANILLA);

      s.play(BOOK_OF_PLAGUE);
      s.answer([{ pick: "instance", instanceId: slime.id }]);
      s.answer([{ pick: "instance", instanceId: slime.id }]);
      s.answer([{ pick: "instance", instanceId: vanilla.id }]);
      s.answer([{ pick: "instance", instanceId: slime.id }]);
      s.answer([{ pick: "instance", instanceId: vanilla.id }]);

      expect(s.card(slime.id).counters.plague).toBe(6);
      expect(placements(s, slime.id).map((event) => event.placed)).toEqual([2, 2, 2]);
      expect(s.card(vanilla.id).counters.plague).toBe(2);
    });

    it("R78 its tokens reset when it leaves the field", () => {
      const s = scenario({ p1: { hand: [FLOOD, ANCHOR], field: [{ def: SLIME, counters: { plague: 4 } }] }, p2: { hand: [ANCHOR] } });
      const slime = s.card(SLIME);

      s.play(FLOOD);

      s.expectInZone(slime, "hand");
      expect(s.card(slime.id).counters.plague ?? 0).toBe(0);
    });

    it("R386 an Upgrade makes it ×3, a Degrade ×1", () => {
      const up = scenario({ p1: { hand: [OUTBREAK, ANCHOR], field: [SLIME], library: [X, X, X, X] }, p2: { hand: [ANCHOR] } });
      stepParam(up.card(SLIME), "multiplier", 1);
      outbreakOn(up, up.card(SLIME).id);
      expect(up.card(SLIME).counters.plague).toBe(3);

      const down = scenario({ p1: { hand: [OUTBREAK, ANCHOR], field: [SLIME], library: [X, X, X, X] }, p2: { hand: [ANCHOR] } });
      stepParam(down.card(SLIME), "multiplier", -1);
      outbreakOn(down, down.card(SLIME).id);
      expect(down.card(SLIME).counters.plague).toBe(1);
    });
  });

  describe("radiant", () => {
    it("is a 2/2, and a placement of 1 on it puts 3", () => {
      const s = scenario({ p1: { hand: [OUTBREAK, ANCHOR], field: [{ def: SLIME, radiant: true }], library: [X, X, X, X] }, p2: { hand: [ANCHOR] } });
      const slime = s.card(SLIME);
      s.expectStats(slime, { attack: 2, health: 2 });

      outbreakOn(s, slime.id);

      expect(s.card(slime.id).counters.plague).toBe(3);
      expect(placements(s, slime.id).map((event) => event.placed)).toEqual([3]);
    });

    it("R386 an Upgrade on the Radiant face steps ×3 to ×4", () => {
      const s = scenario({ p1: { hand: [OUTBREAK, ANCHOR], field: [{ def: SLIME, radiant: true }], library: [X, X, X, X, X] }, p2: { hand: [ANCHOR] } });
      stepParam(s.card(SLIME), "multiplier", 1);

      outbreakOn(s, s.card(SLIME).id);

      expect(s.card(SLIME).counters.plague).toBe(4);
    });
  });
});
