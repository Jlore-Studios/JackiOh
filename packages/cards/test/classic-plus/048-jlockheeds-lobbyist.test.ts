// C+ #48 Jlockheed's Lobbyist — SPEC §8.7 row 48, BUILD M9 Classic+ row C+ 48: "0/3: cannot be in
// Defense Position (a switch is refused, a switch-all effect leaves it in Attack) and never attacks
// with 0 attack; Death adds a random non-token Jlockeed card that costs (0), the pool exactly Core #13,
// #14 and C+ #51, #52 (one tag, never itself, R387); a full hand burns it; hidden from the opponent
// (R97); radiant 0/6, may go to Defense Position, and the card is Radiant".

import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/048-jlockheeds-lobbyist";

const LOBBYIST = "classicplus-048";
const HIT_JOB = "core-016"; // (3) Spell: destroy target Unit
const SWITCH_ALL = "core-048"; // (0) Spell: switch the position of every Unit
const VANILLA = "core-008"; // Mr. Vanilla 4/4
const FILLER = "core-005";
const POOL = ["classicplus-051", "classicplus-052", "core-013", "core-014"];

function board(opts: { radiant?: boolean; seed?: string; hand?: readonly string[] } = {}): Scenario {
  return scenario({
    seed: opts.seed ?? "lobbyist",
    p1: {
      field: [{ def: LOBBYIST, ...(opts.radiant === true ? { radiant: true } : {}) }, VANILLA],
      hand: opts.hand ?? [HIT_JOB, FILLER],
    },
    p2: { hand: [FILLER], field: [VANILLA] },
  });
}

/** Play Hit Job on the Lobbyist and return the card its Death added (or burned). */
function killLobbyist(s: Scenario): string | null {
  const lobbyist = s.unit("p1", 1);
  if (lobbyist === null) throw new Error("no Lobbyist");
  s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: lobbyist.id }] });
  const added = s.lastEvents.find((event) => event.type === "addedToHand" && event.player === "p1");
  const burned = s.lastEvents.find((event) => event.type === "burned");
  if (added?.type === "addedToHand") return added.instanceId;
  if (burned?.type === "burned") return burned.instanceId;
  return null;
}

describe("C+ #48 Jlockheed's Lobbyist", () => {
  it("is a 0/3 Jlockeed Unit (0/6 Radiant) whose base face alone carries the Defense ban", () => {
    expect(def.id).toBe(LOBBYIST);
    expect(def.tags).toEqual(["Jlockeed"]);
    expect([def.base.attack, def.base.health, def.radiant.attack, def.radiant.health]).toEqual([0, 3, 0, 6]);
    expect(base.staticFlags?.neverDefense).toBe(true);
    expect(radiant.staticFlags?.neverDefense).toBeUndefined();
  });

  describe("base", () => {
    it("§4.1 a switch to Defense Position is refused", () => {
      const s = board();
      expect(() => s.switchPosition(s.unit("p1", 1) ?? "")).toThrow(/Defense Position/);
      expect(s.stats(s.unit("p1", 1) ?? "").position).toBe("ATK");
    });

    it("R20 a switch-all effect leaves it in Attack while every other Unit switches", () => {
      const s = board({ hand: [SWITCH_ALL, FILLER] });
      s.play(SWITCH_ALL);
      expect(s.stats(s.unit("p1", 1) ?? "").position).toBe("ATK");
      expect(s.stats(s.unit("p1", 2) ?? "").position).toBe("DEF");
      expect(s.stats(s.unit("p2", 1) ?? "").position).toBe("DEF");
    });

    it("§4.2 with 0 attack it never attacks", () => {
      const s = board();
      expect(() => s.attack(s.unit("p1", 1) ?? "", "hero")).toThrow(/0 attack/);
    });

    it("§6.2 Death adds a random Jlockeed card to your hand that costs (0)", () => {
      const s = board();
      const id = killLobbyist(s);
      s.expectInZone(LOBBYIST, "graveyard");
      const card = s.card(id ?? "");
      expect(card.zone.z).toBe("hand");
      expect(POOL).toContain(card.defId);
      expect(card.costOverride).toBe(0);
      expect(card.radiant).toBe(false);
      expect(s.view("p1").you.hand).toEqual(expect.arrayContaining([expect.objectContaining({ instanceId: card.id, cost: 0 })]));
    });

    it("§6.2 killed on the opponent's turn, its Death still adds the card to your hand", () => {
      const s = scenario({
        active: "p2",
        p1: { field: [LOBBYIST], hand: [FILLER] },
        p2: { hand: [HIT_JOB, FILLER], field: [VANILLA] },
      });
      s.play(HIT_JOB, { targets: [{ pick: "instance", instanceId: s.card(LOBBYIST).id }] });
      const added = s.lastEvents.flatMap((event) => (event.type === "addedToHand" ? [event] : []));
      expect(added.map((event) => event.player)).toEqual(["p1"]);
      const card = s.card(added[0]?.instanceId ?? "");
      expect(card.zone).toMatchObject({ z: "hand", player: "p1" });
      expect(POOL).toContain(card.defId);
      expect(card.costOverride).toBe(0);
    });

    it("R278 R387 the pool is exactly Core #13, #14 and C+ #51, #52 — one tag, never itself", () => {
      const seen = new Set<string>();
      for (let i = 0; i < 80; i += 1) {
        const s = board({ seed: `lobby-${i}` });
        seen.add(s.card(killLobbyist(s) ?? "").defId);
      }
      expect([...seen].sort()).toEqual(POOL);
    });

    it("§2.4 R4 a full hand burns the card, which keeps no (0) price", () => {
      const s = board({ hand: [HIT_JOB, ...Array.from({ length: 10 }, () => FILLER)] });
      const id = killLobbyist(s);
      expect(s.lastEvents.some((event) => event.type === "burned")).toBe(true);
      const card = s.card(id ?? "");
      expect(card.zone.z).toBe("graveyard");
      expect(card.costOverride).toBeUndefined();
    });

    it("R97 the opponent sees only that a card reached your hand", () => {
      const s = board();
      const id = killLobbyist(s) ?? "";
      const theirs = s.view("p2");
      expect(JSON.stringify(theirs)).not.toContain(`"${id}"`);
      const added = theirs.events.find((event) => event.type === "addedToHand" && event.player === "p1");
      expect(added).toMatchObject({ instanceId: "hidden", defId: "hidden" });
    });
  });

  describe("radiant", () => {
    it("§5.2 0/6 with no Defense ban: it may switch to Defense Position", () => {
      const s = board({ radiant: true });
      s.expectStats(s.unit("p1", 1) ?? "", { attack: 0, health: 6 });
      expect(() => s.attack(s.unit("p1", 1) ?? "", "hero")).toThrow(/0 attack/);
      s.switchPosition(s.unit("p1", 1) ?? "");
      expect(s.stats(s.unit("p1", 1) ?? "").position).toBe("DEF");
    });

    it("R74 Death adds a Radiant Jlockeed card that costs (0), never itself", () => {
      const seen = new Set<string>();
      for (let i = 0; i < 40; i += 1) {
        const s = board({ radiant: true, seed: `rlobby-${i}` });
        const card = s.card(killLobbyist(s) ?? "");
        expect(card.radiant).toBe(true);
        expect(card.costOverride).toBe(0);
        seen.add(card.defId);
      }
      expect(seen.has(LOBBYIST)).toBe(false);
      expect([...seen].every((id) => POOL.includes(id))).toBe(true);
    });
  });
});
