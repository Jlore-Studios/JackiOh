// C+ #77 Anti-Softlock — SPEC §8.7 row 77, E20, E21, E38, R81, R97, R346, R386, R440, BUILD M9 row C+ 77.

import { beneathAt, isLocked, lockZone, stepParam, type CardInstance } from "@jackioh/engine";
import type { KeywordKind } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { base, def, radiant } from "../../src/scripts/classic-plus/077-anti-softlock";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const SOFTLOCK = "classicplus-077";
const VANILLA = "core-008";
const TIMMY = "core-011";
const ARMORED = "core-025"; // 7/7, Armor 7
const ECLIPSE = "core-035"; // (1) Spell: deal 3 damage to a target
const MANA_WELL = "core-006"; // a Field Spell
const CONJURE_KY = "core-057"; // (2) Spell: add 3 random KY cards to your hand
const FILLER = "core-005";
const ALL = "All cards";
const YOURS = "Only yours";

function softlock(opts: { radiant?: boolean; p1?: SideSetup; p2?: SideSetup } = {}): Scenario {
  return scenario({
    p1: { hand: [{ def: SOFTLOCK, radiant: opts.radiant === true }, VANILLA], library: [TIMMY, FILLER, FILLER], ...opts.p1 },
    p2: { hand: [VANILLA], field: [TIMMY], library: [FILLER], ...opts.p2 },
  });
}

/** The keywords a card was granted, wherever it is (E38). */
function granted(s: Scenario, card: string | CardInstance): KeywordKind[] {
  return s.card(card).grantedKeywords.map((keyword) => keyword.kind);
}

describe("C+ #77 Anti-Softlock", () => {
  it("draws its declared `draw`; the Radiant face declares the choice", () => {
    expect(def.id).toBe(SOFTLOCK);
    expect(def.params?.map((entry) => [entry.key, entry.base, entry.radiant])).toEqual([["draw", 1, 2]]);
    expect(base.modes).toBeUndefined();
    expect(radiant.modes).toEqual([{ kind: "mode", options: [ALL, YOURS] }]);
  });

  describe("base", () => {
    it("E38 draws 1, then every card on the field, in both hands and in both decks gains Stack and Pierce — the drawn card too", () => {
      const s = softlock({ p1: { field: [ARMORED], backrow: [MANA_WELL] } }).play(SOFTLOCK);
      expect(s.hand("p1").map((card) => card.defId)).toEqual([VANILLA, TIMMY]);
      const everywhere = [
        ...s.hand("p1"),
        ...s.pile("p1", "library"),
        ...s.hand("p2"),
        ...s.pile("p2", "library"),
        s.unit("p1", 1),
        s.backrow("p1", 1),
        s.unit("p2", 1),
      ];
      for (const card of everywhere) expect(granted(s, card as CardInstance)).toEqual(["Stack", "Pierce"]);
      expect(s.stats(s.unit("p2", 1) as CardInstance).keywords.map((keyword) => keyword.kind)).toEqual(
        expect.arrayContaining(["Stack", "Pierce"]),
      );
    });

    it("this card, resolving, gains nothing; a card dormant under a Stack is not on the field", () => {
      const s = softlock({ p2: { hand: [VANILLA], field: [VANILLA, { def: "core-092", stack: true }], library: [FILLER] } });
      // The pile reads top first: the Fiender on top, the Vanilla dormant beneath it.
      const buried = s.state.players.p2.units[0]?.[1] as CardInstance;
      s.play(SOFTLOCK);
      expect(granted(s, SOFTLOCK)).toEqual([]);
      expect(granted(s, buried)).toEqual([]);
    });

    it("E38 a hand Unit carries them onto the field, and is played onto an occupied unit zone (Stack)", () => {
      const s = softlock({ p1: { field: [ARMORED] } }).play(SOFTLOCK);
      s.play(VANILLA, { zone: 1 });
      expect(s.unit("p1", 1)?.defId).toBe(VANILLA);
      expect(s.state.players.p1.units[0]?.map((card) => card.defId)).toEqual([VANILLA, ARMORED]);
      expect(s.stats(s.unit("p1", 1) as CardInstance).keywords.map((keyword) => keyword.kind)).toEqual(
        expect.arrayContaining(["Stack", "Pierce"]),
      );
    });

    it("E21 a backrow card with Stack is played onto an occupied backrow zone: a pile whose top acts", () => {
      const s = softlock({ p1: { hand: [{ def: SOFTLOCK }, MANA_WELL], backrow: [MANA_WELL], mana: 5 } });
      const under = s.backrow("p1", 1) as CardInstance;
      s.play(SOFTLOCK);
      const top = s.hand("p1").find((card) => card.defId === MANA_WELL) as CardInstance;
      s.play(top, { zone: 1 });
      expect(s.backrow("p1", 1)?.id).toBe(top.id);
      expect(beneathAt(s.state, { player: "p1", row: "backrow", lane: 1 }).map((card) => card.id)).toEqual([under.id]);
    });

    it("R346 a Spell's Pierce skips Armor, and its Stack does nothing", () => {
      const s = softlock({ p1: { hand: [{ def: SOFTLOCK }, ECLIPSE] }, p2: { field: [ARMORED] } }).play(SOFTLOCK);
      s.play(ECLIPSE, { targets: [{ pick: "instance", instanceId: s.unit("p2", 1)?.id ?? "" }] });
      s.expectStats(ARMORED, { health: 4 });
      s.expectInZone(ECLIPSE, "graveyard");
    });

    it("a card created later lacks both", () => {
      const s = softlock({ p1: { hand: [{ def: SOFTLOCK }, CONJURE_KY], library: [FILLER] } }).play(SOFTLOCK);
      const before = new Set(s.hand("p1").map((card) => card.id));
      s.play(CONJURE_KY);
      const made = s.hand("p1").filter((card) => !before.has(card.id));
      expect(made).toHaveLength(3);
      for (const card of made) expect(granted(s, card)).toEqual([]);
    });

    it("E20 unlocks every Locked zone of both players (`unlocked`); a reserved zone stays reserved", () => {
      const s = softlock();
      lockZone(s.state, { player: "p1", row: "units", lane: 2 });
      lockZone(s.state, { player: "p2", row: "backrow", lane: 3 });
      s.state.reserved.push({ player: "p2", row: "units", lane: 4 });
      s.play(SOFTLOCK);
      expect(isLocked(s.state, { player: "p1", row: "units", lane: 2 })).toBe(false);
      expect(isLocked(s.state, { player: "p2", row: "backrow", lane: 3 })).toBe(false);
      expect(s.events.filter((event) => event.type === "unlocked")).toHaveLength(2);
      expect(s.state.reserved).toEqual([{ player: "p2", row: "units", lane: 4 }]);
    });

    it("R97 R440 the grants in the opponent's hand and deck name no card", () => {
      const s = softlock().play(SOFTLOCK);
      const hidden = [...s.hand("p2"), ...s.pile("p2", "library"), ...s.pile("p1", "library")].map((card) => card.id);
      for (const event of s.view("p1").events) {
        if (event.type === "keywordGranted") expect(hidden).not.toContain(event.instanceId);
      }
      expect(JSON.stringify(s.view("p1"))).not.toContain(s.hand("p2")[0]?.id ?? "-");
    });

    it("R386 the draw reads through param(): an Upgrade draws 2", () => {
      const s = softlock();
      stepParam(s.card(SOFTLOCK), "draw", 1);
      s.play(SOFTLOCK);
      expect(s.hand("p1")).toHaveLength(3);
    });
  });

  describe("radiant", () => {
    it("draws 2; all cards: both sides gain Stack and Pierce", () => {
      const s = softlock({ radiant: true }).play(SOFTLOCK, { modes: [ALL] });
      expect(s.hand("p1")).toHaveLength(3);
      expect(granted(s, s.unit("p2", 1) as CardInstance)).toEqual(["Stack", "Pierce"]);
      expect(granted(s, s.hand("p2")[0] as CardInstance)).toEqual(["Stack", "Pierce"]);
    });

    it("R81 only yours: your cards gain them, the opponent's do not, and every zone is still unlocked", () => {
      const s = softlock({ radiant: true });
      lockZone(s.state, { player: "p2", row: "units", lane: 5 });
      s.play(SOFTLOCK, { modes: [YOURS] });
      for (const card of [...s.hand("p1"), ...s.pile("p1", "library")]) expect(granted(s, card)).toEqual(["Stack", "Pierce"]);
      for (const card of [...s.hand("p2"), ...s.pile("p2", "library"), s.unit("p2", 1) as CardInstance]) {
        expect(granted(s, card)).toEqual([]);
      }
      expect(isLocked(s.state, { player: "p2", row: "units", lane: 5 })).toBe(false);
    });
  });
});
