// C #43 Plague Nuke — SPEC §8.6 row 43, BUILD M9 Classic row C 43: "Counts the Plague Tokens on every
// Unit first, then destroys all Units in one state check (§4.5), then gives 1 mana this turn per token
// counted, an Indestructible survivor's tokens included; its preview is that mana (R280); radiant:
// after that check, each non-token Unit card that had a token and now lies in a graveyard is summoned
// to your side under your control, its owner unchanged, into your leftmost open zones in lane order,
// without a Cry; a Reborn Unit already back is not summoned again; tokens are gone (R11); a full board
// leaves the rest; a Unit exiled instead of dying into a graveyard (C #50) is not summoned; its tuned
// number (mana per token) reads through `param()` (R386)".
//
// The preview's proofs (R280) are in `../preview.test.ts`, with the other cards'. The C #50 case waits
// for C #50 Voidwalker's script (cards-classic-b), whose Aura exiles what would go to a graveyard.

import { stepParam } from "@jackioh/engine";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic/043-plague-nuke";

const NUKE = "classic-043";
const STATE = "classic-041"; // State of the Game: 3/3 Indestructible.
const VOIDWALKER = "classic-050"; // Aura: cards that would go to a graveyard are exiled instead.
const VANILLA = "core-008"; // (1) Unit 4/4.
const MENACE = "core-019"; // (3) Unit 9/9.
const FELINORS = "core-012"; // Cry: summon a copy of this.
const DEFENDER = "core-003"; // Right-house defender: Taunt, Divine Shield, Reborn.
const RUSH_TOKEN = "core-t-rush";
const ANCHOR = "core-010";

function plagued(defId: string, n: number, extra: Record<string, unknown> = {}): { def: string; counters: { plague: number } } {
  return { def: defId, counters: { plague: n }, ...extra };
}

function manaGained(s: Scenario): number {
  // The cast paid 3 from 4; anything above 1 is the Spell's gain.
  return s.state.players.p1.mana.current - 1;
}

function unitDefs(s: Scenario, player: "p1" | "p2"): (string | null)[] {
  return [1, 2, 3, 4, 5].map((lane) => s.unit(player, lane)?.defId ?? null);
}

describe("C #43 Plague Nuke", () => {
  it("has a script per face, each with a preview", () => {
    expect(def.id).toBe(NUKE);
    expect(base.preview).toBeTypeOf("function");
    expect(radiant.preview).toBeTypeOf("function");
  });

  describe("base", () => {
    it("destroys all Units on both sides, then gains 1 mana per Plague Token that was on them", () => {
      const s = scenario({
        p1: { hand: [NUKE, ANCHOR], field: [plagued(VANILLA, 2)] },
        p2: { hand: [ANCHOR], field: [plagued(MENACE, 3), VANILLA] },
      });

      s.play(NUKE);

      expect(unitDefs(s, "p1")).toEqual([null, null, null, null, null]);
      expect(unitDefs(s, "p2")).toEqual([null, null, null, null, null]);
      expect(manaGained(s)).toBe(5);
    });

    it("§4.5 one state check: every death comes first, and the mana after them", () => {
      const s = scenario({
        p1: { hand: [NUKE, ANCHOR], field: [plagued(VANILLA, 1)] },
        p2: { hand: [ANCHOR], field: [plagued(MENACE, 1), VANILLA] },
      });

      s.play(NUKE);

      const kinds = s.lastEvents.map((event) => event.type);
      const destroyed = kinds.flatMap((kind, at) => (kind === "destroyed" ? [at] : []));
      expect(destroyed).toHaveLength(3);
      const gain = kinds.lastIndexOf("manaChanged");
      for (const at of destroyed) expect(at).toBeLessThan(gain);
    });

    it("no tokens on the board: it destroys all and gives nothing", () => {
      const s = scenario({ p1: { hand: [NUKE, ANCHOR], field: [VANILLA] }, p2: { hand: [ANCHOR], field: [MENACE] } });

      s.play(NUKE);

      expect(manaGained(s)).toBe(0);
      s.expectInZone(VANILLA, "graveyard").expectInZone(MENACE, "graveyard");
    });

    it("R46 an Indestructible Unit survives, and its tokens count all the same", () => {
      const s = scenario({ p1: { hand: [NUKE, ANCHOR] }, p2: { hand: [ANCHOR], field: [plagued(STATE, 2), plagued(VANILLA, 1)] } });

      s.play(NUKE);

      s.expectInZone(STATE, "field").expectInZone(VANILLA, "graveyard");
      expect(manaGained(s)).toBe(3);
    });

    it("the base face summons nothing back", () => {
      const s = scenario({ p1: { hand: [NUKE, ANCHOR] }, p2: { hand: [ANCHOR], field: [plagued(VANILLA, 1)] } });

      s.play(NUKE);

      s.expectInZone(VANILLA, "graveyard");
      expect(unitDefs(s, "p1")).toEqual([null, null, null, null, null]);
    });

    it("R386 an Upgrade gives 2 mana per token", () => {
      const s = scenario({ p1: { hand: [NUKE, ANCHOR] }, p2: { hand: [ANCHOR], field: [plagued(VANILLA, 2)] } });
      stepParam(s.card(NUKE), "mana", 1);

      s.play(NUKE);

      expect(manaGained(s)).toBe(4);
    });
  });

  describe("radiant", () => {
    it("summons each Unit that had a token from its owner's graveyard under your control, owner unchanged, in lane order", () => {
      const s = scenario({
        p1: { hand: [{ def: NUKE, radiant: true }, ANCHOR], field: [plagued(VANILLA, 1)] },
        p2: { hand: [ANCHOR], field: [MENACE, plagued(MENACE, 2)] },
      });
      const mine = s.card(VANILLA);
      const theirs = s.unit("p2", 2);
      if (theirs === null) throw new Error("a plagued Menace in lane 2");

      s.play(NUKE);

      expect(manaGained(s)).toBe(3);
      expect(unitDefs(s, "p1")).toEqual([VANILLA, MENACE, null, null, null]);
      expect(s.unit("p1", 1)?.id).toBe(mine.id);
      expect(s.unit("p1", 2)?.id).toBe(theirs.id);
      expect(s.card(theirs.id).owner).toBe("p2");
      expect(s.card(theirs.id).controller).toBe("p1");
      // The Menace with no token stays in its owner's graveyard.
      expect(s.pile("p2", "graveyard").map((card) => card.defId)).toEqual([MENACE]);
    });

    it("R1 R78 a summoned Unit returns reset and fires no Cry: a Duplicating Felinors makes no copy, and its tokens are gone", () => {
      const s = scenario({ p1: { hand: [{ def: NUKE, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR], field: [plagued(FELINORS, 1)] } });

      s.play(NUKE);

      expect(unitDefs(s, "p1")).toEqual([FELINORS, null, null, null, null]);
      expect(s.unit("p1", 1)?.counters.plague ?? 0).toBe(0);
    });

    it("R64 R83 a Reborn Unit the check already put back is not summoned again", () => {
      const s = scenario({ p1: { hand: [{ def: NUKE, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR], field: [plagued(DEFENDER, 1)] } });
      const defender = s.card(DEFENDER);

      s.play(NUKE);

      expect(s.card(defender.id).controller).toBe("p2");
      expect(s.unit("p2", 1)?.id).toBe(defender.id);
      expect(unitDefs(s, "p1")).toEqual([null, null, null, null, null]);
    });

    it("R11 a token that had a token is gone, not summoned", () => {
      const s = scenario({ p1: { hand: [{ def: NUKE, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR], field: [plagued(RUSH_TOKEN, 2)] } });
      const token = s.unit("p2", 1);
      if (token === null) throw new Error("a token");

      s.play(NUKE);

      s.expectInZone(token, "gone");
      expect(unitDefs(s, "p1")).toEqual([null, null, null, null, null]);
      expect(manaGained(s)).toBe(2);
    });

    it("an Indestructible survivor with tokens stays where it is, with its controller", () => {
      const s = scenario({ p1: { hand: [{ def: NUKE, radiant: true }, ANCHOR] }, p2: { hand: [ANCHOR], field: [plagued(STATE, 1)] } });

      s.play(NUKE);

      expect(s.card(STATE).controller).toBe("p2");
      expect(unitDefs(s, "p1")).toEqual([null, null, null, null, null]);
    });

    it("a full board leaves the rest in the graveyard", () => {
      const s = scenario({
        p1: { hand: [{ def: NUKE, radiant: true }, ANCHOR], field: [STATE, STATE, STATE, STATE] },
        p2: { hand: [ANCHOR], field: [plagued(VANILLA, 1), plagued(MENACE, 1)] },
      });

      s.play(NUKE);

      expect(unitDefs(s, "p1")).toEqual([STATE, STATE, STATE, STATE, VANILLA]);
      expect(s.pile("p2", "graveyard").map((card) => card.defId)).toEqual([MENACE]);
    });

    it("C #50 a Unit exiled instead of dying into a graveyard is not summoned", () => {
      // C #50 Voidwalker's base Aura: "Cards that would go to a graveyard are exiled instead" — every
      // card, so the Voidwalker's own death and the Vanilla's both go to exile.
      const s = scenario({
        p1: { hand: [{ def: NUKE, radiant: true }, ANCHOR] },
        p2: { hand: [ANCHOR], field: [plagued(VANILLA, 1), VOIDWALKER] },
      });

      s.play(NUKE);

      expect(s.pile("p2", "exile").map((card) => card.defId)).toContain(VANILLA);
      expect(unitDefs(s, "p1")).toEqual([null, null, null, null, null]);
    });
  });
});
