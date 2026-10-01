// C+ #44 Simplicity Audit — SPEC §8.7 row 44, E36, R11, R13, R177, R280, R583, BUILD M9 row C+ 44.
// The sweep is `subsystems.auditTargets` (packages/engine/test/audit.test.ts); its preview is pinned
// in test/preview.test.ts with R280's other cards. Every `loc` here is read off the catalog, so a
// script edit that moves one fails the guard that names it rather than a silent comparison.

import { createRng, newInstance, subsystems, type CardInstance } from "@jackioh/engine";
import type { CardView, PreviewValue } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { CATALOG, cardDef } from "../../src/index";
import { base, def, radiant } from "../../src/scripts/classic-plus/044-simplicity-audit";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const AUDIT = "classicplus-044";
const LOC = cardDef(AUDIT).loc ?? 0;
const LOW = "core-008"; // Mr. Vanilla
const HIGH = "core-022"; // Carnivorous Cube
const LOW_TRAP = "core-071"; // Intern Stimmy, a Field Trap
const REBORN = "core-003"; // Right-house defender: Taunt, Divine Shield, Reborn
const RUSH = "core-t-rush";
const FIENDER = "core-092"; // Felinor Fiender, Stack
const INDESTRUCTIBLE = "classic-041"; // State of the Game: Indestructible
const FILLER = "core-005";
const ALL = "All permanents";
const THEIRS = "Only your opponent's";

/** Two Units whose `loc` sums to the Audit's: fused, they are a permanent of exactly its `loc`. */
const EQUAL_PAIR = ((): [string, string] => {
  const units = Object.values(CATALOG).filter((card) => card.type === "Unit" && !card.token && card.loc !== undefined);
  for (const a of units) {
    const b = units.find((other) => other.id !== a.id && (a.loc ?? 0) + (other.loc ?? 0) === LOC);
    if (b !== undefined) return [a.id, b.id];
  }
  throw new Error(`no two Units sum to ${LOC} lines`);
})();

function audit(opts: { radiant?: boolean; p1?: SideSetup; p2?: SideSetup } = {}): Scenario {
  return scenario({
    p1: { hand: [{ def: AUDIT, radiant: opts.radiant === true }, FILLER], ...opts.p1 },
    p2: { hand: [FILLER], ...opts.p2 },
  });
}

/** Fuse the pair onto p2's unit in `lane`, as R77's kept target: a permanent of the Audit's `loc`. */
function fuseEqual(s: Scenario, lane: number): CardInstance {
  const kept = s.unit("p2", lane);
  if (kept === null) throw new Error("a unit to fuse onto");
  const other = s.card(kept).defId === EQUAL_PAIR[0] ? EQUAL_PAIR[1] : EQUAL_PAIR[0];
  const sink = { state: s.state, events: [], rng: createRng(s.state.seed, s.state.rngCursor) };
  const phantom = newInstance(s.state, other, "p2", { z: "gone", player: "p2" });
  const fused = subsystems.fuse(sink, { ingredients: [kept, phantom], target: kept });
  if (fused === null) throw new Error("the fusion");
  return fused;
}

function preview(s: Scenario): PreviewValue[] | undefined {
  const hand = s.view("p1").you.hand as CardView[];
  return hand.find((card) => card.instanceId === s.card(AUDIT).id)?.preview;
}

describe("C+ #44 Simplicity Audit", () => {
  it("the catalog's loc guards: the cards below are below, above and summed to the Audit's", () => {
    expect(def.id).toBe(AUDIT);
    expect(LOC).toBeGreaterThan(0);
    for (const id of [LOW, LOW_TRAP, REBORN, RUSH, FIENDER, INDESTRUCTIBLE]) expect(cardDef(id).loc ?? 0).toBeLessThan(LOC);
    expect(cardDef(HIGH).loc ?? 0).toBeGreaterThan(LOC);
  });

  describe("base", () => {
    it("E36 exiles every permanent on both sides whose card has fewer lines of code; a higher one stays", () => {
      const s = audit({ p1: { field: [LOW, HIGH] }, p2: { field: [LOW, HIGH] } });
      const [mine, theirs] = [s.unit("p1", 1), s.unit("p2", 1)];
      s.play(AUDIT);
      s.expectInZone(mine as CardInstance, "exile").expectInZone(theirs as CardInstance, "exile");
      expect(s.card(HIGH).zone.z).toBe("field");
      expect(s.unit("p2", 2)?.defId).toBe(HIGH);
      expect(s.unit("p1", 2)?.defId).toBe(HIGH);
    });

    it("E36 an equal loc stays: a fused card's loc is its ingredients' sum (R77)", () => {
      const s = audit({ p2: { field: [EQUAL_PAIR[0], LOW] } });
      const fused = fuseEqual(s, 1);
      expect(subsystems.linesOfCode(s.state, fused.defId)).toBe(LOC);
      s.play(AUDIT);
      expect(s.unit("p2", 1)?.id).toBe(fused.id);
      expect(s.unit("p2", 2)).toBeNull();
    });

    it("§3.2 face-down backrow cards are permanents too, and are exiled", () => {
      const s = audit({ p2: { backrow: [{ def: LOW_TRAP, faceUp: false }] } });
      s.play(AUDIT);
      s.expectInZone(LOW_TRAP, "exile");
    });

    it("R11 §6.3 an exile: a token ceases to exist, Reborn never returns, and the exile counter moves", () => {
      const s = audit({ p2: { field: [RUSH, REBORN] } });
      const token = s.unit("p2", 1) as CardInstance;
      const exiled = s.state.counters.exiled;
      s.play(AUDIT);
      s.expectInZone(token, "gone").expectInZone(REBORN, "exile");
      expect(s.unit("p2", 2)).toBeNull();
      expect(s.events.some((event) => event.type === "destroyed")).toBe(false);
      // The token never reached an exile pile (R11), so only the defender counts.
      expect(s.state.counters.exiled).toBe(exiled + 1);
    });

    it("§6.1 an exile is no destroy: an Indestructible permanent is exiled too", () => {
      const s = audit({ p2: { field: [INDESTRUCTIBLE] } });
      s.play(AUDIT);
      s.expectInZone(INDESTRUCTIBLE, "exile");
    });

    it("R13 a card dormant under a Stack is not on the field: the top goes, and the card beneath resumes", () => {
      const s = audit({ p2: { field: [HIGH, { def: FIENDER, stack: true }] } });
      s.play(AUDIT);
      s.expectInZone(FIENDER, "exile");
      expect(s.unit("p2", 1)?.defId).toBe(HIGH);
    });

    it("nothing below its loc: nothing is exiled, and the Spell still resolves", () => {
      const s = audit({ p2: { field: [HIGH] } });
      s.play(AUDIT);
      expect(s.unit("p2", 1)?.defId).toBe(HIGH);
      s.expectInZone(AUDIT, "graveyard");
    });

    it("R280 the base face marks nothing: no preview", () => {
      expect(base.preview).toBeUndefined();
      const s = audit({ p2: { field: [LOW] } });
      expect(preview(s)).toBeUndefined();
    });
  });

  describe("radiant", () => {
    it("R81 chooses with the play: all permanents, or only the opponent's", () => {
      expect(radiant.modes).toEqual([{ kind: "mode", options: [ALL, THEIRS] }]);
      const all = audit({ radiant: true, p1: { field: [LOW] }, p2: { field: [LOW, HIGH] } });
      all.play(AUDIT, { modes: [ALL] });
      expect(all.unit("p1", 1)).toBeNull();
      expect(all.unit("p2", 1)).toBeNull();
      const theirs = audit({ radiant: true, p1: { field: [LOW] }, p2: { field: [LOW, HIGH] } });
      theirs.play(AUDIT, { modes: [THEIRS] });
      expect(theirs.unit("p1", 1)?.defId).toBe(LOW);
      expect(theirs.unit("p2", 1)).toBeNull();
      expect(theirs.unit("p2", 2)?.defId).toBe(HIGH);
    });

    it("R280 R583 its preview is, for each choice, the permanents it would exile now — and that is what the play exiles", () => {
      const s = audit({ radiant: true, p1: { field: [LOW, HIGH] }, p2: { field: [LOW, HIGH] } });
      const [mine, theirs] = [s.unit("p1", 1)?.id, s.unit("p2", 1)?.id];
      expect(preview(s)).toEqual([
        { label: "all permanents", value: 2, ids: [mine, theirs] },
        { label: "only your opponent's", value: 1, ids: [theirs] },
      ]);
      for (const entry of preview(s) ?? []) expect(cardDef(AUDIT).radiant.text).toContain(entry.label);
      s.play(AUDIT, { modes: [ALL] });
      expect([s.card(mine ?? "").zone.z, s.card(theirs ?? "").zone.z]).toEqual(["exile", "exile"]);
    });

    it("R177 R583 the preview never marks an enemy face-down card, which the exile still takes; its own face-down card it marks", () => {
      const s = audit({
        radiant: true,
        p1: { backrow: [{ def: LOW_TRAP, faceUp: false }] },
        p2: { backrow: [{ def: LOW_TRAP, faceUp: false }] },
      });
      const [mine, theirs] = [s.backrow("p1", 1)?.id ?? "", s.backrow("p2", 1)?.id ?? ""];
      expect(preview(s)).toEqual([
        { label: "all permanents", value: 1, ids: [mine] },
        { label: "only your opponent's", value: 0, ids: [] },
      ]);
      s.play(AUDIT, { modes: [THEIRS] });
      s.expectInZone(theirs, "exile");
      expect(s.backrow("p1", 1)?.id).toBe(mine);
    });

    it("R280 the opponent's view of the hand card carries no preview", () => {
      const s = audit({ radiant: true, p2: { field: [LOW] } });
      expect(JSON.stringify(s.view("p2"))).not.toContain("all permanents");
    });
  });
});
