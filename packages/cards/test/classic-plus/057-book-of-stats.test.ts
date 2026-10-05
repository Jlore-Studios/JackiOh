// C+ #57 Book of Stats — SPEC §8.7 row 57, BUILD M9 Classic+ row C+ 57: "A target Unit on either side
// gets +5/+5 permanently, kept until it leaves the field; an Immune to Spells Unit is no legal target;
// the buff reads through `param()`; radiant +10/+10".

import { legalActions, stepParam } from "@jackioh/engine";
import type { Selection } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { scenario, type Scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/057-book-of-stats";

const BOOK = "classicplus-057";
const VANILLA = "core-008"; // Mr. Vanilla 4/4
const FLOOD = "core-017"; // (4) Spell: bounce all Units
const FILLER = "core-005";

function stats(opts: { radiant?: boolean } = {}): Scenario {
  return scenario({
    p1: { hand: [{ def: BOOK, ...(opts.radiant === true ? { radiant: true } : {}) }, FLOOD, FILLER], field: [VANILLA] },
    p2: { hand: [FILLER], field: [VANILLA] },
  });
}

function at(s: Scenario, player: "p1" | "p2"): Selection[] {
  const unit = s.unit(player, 1);
  if (unit === null) throw new Error(`no unit for ${player}`);
  return [{ pick: "instance", instanceId: unit.id }];
}

/** The instance ids `legalActions` offers Book of Stats as targets. */
function offered(s: Scenario): string[] {
  const book = s.card(BOOK);
  return legalActions(s.state, "p1").flatMap((action) =>
    action.type === "play" && action.instanceId === book.id
      ? (action.targets ?? []).flatMap((pick) => (pick.pick === "instance" ? [pick.instanceId] : []))
      : [],
  );
}

describe("C+ #57 Book of Stats", () => {
  it("is a (1) Spell, Book that declares one Unit target on either side", () => {
    expect(def.id).toBe(BOOK);
    expect(base.targets).toEqual([
      { kind: "target", min: 1, max: 1, aim: "help", filter: { side: "any", of: ["unit"] } },
    ]);
    expect(radiant).toBe(base);
  });

  describe("base", () => {
    it("§10.4 your Unit gets +5/+5", () => {
      const s = stats();
      s.play(BOOK, { targets: at(s, "p1") });
      s.expectStats(s.unit("p1", 1) ?? "", { attack: 9, health: 9, maxHealth: 9 });
    });

    it("R81 an enemy Unit is a legal target too", () => {
      const s = stats();
      s.play(BOOK, { targets: at(s, "p2") });
      s.expectStats(s.unit("p2", 1) ?? "", { attack: 9, health: 9 });
    });

    it("§10.4 the buff is permanent through the turns, and is lost when the Unit leaves the field (R78)", () => {
      const s = stats();
      s.play(BOOK, { targets: at(s, "p1") });
      const unit = s.unit("p1", 1);
      s.endTurn().endTurn();
      s.expectStats(unit ?? "", { attack: 9, health: 9 });
      s.play(FLOOD);
      expect(s.card(unit ?? "").zone.z).toBe("hand");
      expect(s.card(unit ?? "").buffs).toEqual({ attack: 0, health: 0 });
    });

    it("R23 an Immutable Unit still takes the buff: a buff is no change to its text", () => {
      const s = scenario({
        p1: { hand: [BOOK, FILLER], field: [{ def: "core-019", radiant: true }] },
        p2: { hand: [FILLER] },
      });
      s.play(BOOK, { targets: at(s, "p1") });
      s.expectStats(s.unit("p1", 1) ?? "", { attack: 23, health: 23 });
    });

    it("E35 an Immune to Spells Unit is no legal target", () => {
      const s = stats();
      const immune = s.unit("p2", 1);
      if (immune === null) throw new Error("no unit");
      s.card(immune).grantedKeywords.push({ kind: "Immune to Spells" });
      expect(offered(s)).not.toContain(immune.id);
      expect(offered(s)).toContain(s.unit("p1", 1)?.id);
      expect(() => s.play(BOOK, { targets: at(s, "p2") })).toThrow();
    });

    it("R386 an Upgrade gives +6/+6; a Degrade +4/+4", () => {
      const up = stats();
      stepParam(up.card(BOOK), "buff", 1);
      up.play(BOOK, { targets: at(up, "p1") });
      up.expectStats(up.unit("p1", 1) ?? "", { attack: 10, health: 10 });

      const down = stats();
      stepParam(down.card(BOOK), "buff", -1);
      down.play(BOOK, { targets: at(down, "p1") });
      down.expectStats(down.unit("p1", 1) ?? "", { attack: 8, health: 8 });
    });
  });

  describe("radiant", () => {
    it("§10.4 the Unit gets +10/+10", () => {
      const s = stats({ radiant: true });
      s.play(BOOK, { targets: at(s, "p2") });
      s.expectStats(s.unit("p2", 1) ?? "", { attack: 14, health: 14, maxHealth: 14 });
    });

    it("R386 the Radiant buff steps by its declared step: an Upgrade gives +11/+11", () => {
      const s = stats({ radiant: true });
      stepParam(s.card(BOOK), "buff", 1);
      s.play(BOOK, { targets: at(s, "p1") });
      s.expectStats(s.unit("p1", 1) ?? "", { attack: 15, health: 15 });
    });
  });
});
