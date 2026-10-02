// C+ #45 Complexity Audit — SPEC §8.7 row 45, E36, R13, R177, R280, R583, BUILD M9 row C+ 45: C+ #44
// with the comparison reversed. The sweep is `subsystems.auditTargets` (packages/engine/test/audit.test.ts);
// its preview is pinned in test/preview.test.ts. Every `loc` is read off the catalog and guarded.

import { createRng, newInstance, subsystems, type CardInstance } from "@jackioh/engine";
import type { CardView, PreviewValue } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { CATALOG, cardDef } from "../../src/index";
import { base, def, radiant } from "../../src/scripts/classic-plus/045-complexity-audit";
import { scenario, type Scenario, type SideSetup } from "../_harness";

const AUDIT = "classicplus-045";
const LOC = cardDef(AUDIT).loc ?? 0;
const LOW = "core-008"; // Mr. Vanilla
const HIGH = "core-022"; // Carnivorous Cube, whose Death an exile never fires
const HIGH_TRAP = "core-060"; // Bear Honeypot
const FIENDER = "core-092"; // Felinor Fiender, Stack
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

function fuseEqual(s: Scenario, lane: number): CardInstance {
  const kept = s.unit("p2", lane);
  if (kept === null) throw new Error("a unit to fuse onto");
  const sink = { state: s.state, events: [], rng: createRng(s.state.seed, s.state.rngCursor) };
  const phantom = newInstance(s.state, EQUAL_PAIR[1], "p2", { z: "gone", player: "p2" });
  const fused = subsystems.fuse(sink, { ingredients: [kept, phantom], target: kept });
  if (fused === null) throw new Error("the fusion");
  return fused;
}

function preview(s: Scenario): PreviewValue[] | undefined {
  const hand = s.view("p1").you.hand as CardView[];
  return hand.find((card) => card.instanceId === s.card(AUDIT).id)?.preview;
}

describe("C+ #45 Complexity Audit", () => {
  it("the catalog's loc guards: the cards below are above and below the Audit's", () => {
    expect(def.id).toBe(AUDIT);
    for (const id of [HIGH, HIGH_TRAP]) expect(cardDef(id).loc ?? 0).toBeGreaterThan(LOC);
    for (const id of [LOW, FIENDER]) expect(cardDef(id).loc ?? 0).toBeLessThan(LOC);
  });

  describe("base", () => {
    it("E36 exiles every permanent on both sides whose card has more lines of code; a lower one stays", () => {
      const s = audit({ p1: { field: [HIGH, LOW] }, p2: { field: [HIGH, LOW] } });
      const [mine, theirs] = [s.unit("p1", 1), s.unit("p2", 1)];
      s.play(AUDIT);
      s.expectInZone(mine as CardInstance, "exile").expectInZone(theirs as CardInstance, "exile");
      expect(s.unit("p1", 2)?.defId).toBe(LOW);
      expect(s.unit("p2", 2)?.defId).toBe(LOW);
      // An exile fires no Death (§6.3): the Cubes' Deaths never ran.
      expect(s.events.some((event) => event.type === "destroyed")).toBe(false);
    });

    it("E36 an equal loc stays: a fused card's loc is its ingredients' sum (R77)", () => {
      const s = audit({ p2: { field: [EQUAL_PAIR[0], HIGH] } });
      const fused = fuseEqual(s, 1);
      expect(subsystems.linesOfCode(s.state, fused.defId)).toBe(LOC);
      s.play(AUDIT);
      expect(s.unit("p2", 1)?.id).toBe(fused.id);
      expect(s.unit("p2", 2)).toBeNull();
    });

    it("§3.2 face-down backrow cards are permanents too, and are exiled", () => {
      const s = audit({ p2: { backrow: [{ def: HIGH_TRAP, faceUp: false }] } });
      s.play(AUDIT);
      s.expectInZone(HIGH_TRAP, "exile");
    });

    it("R13 a card dormant under a Stack is not on the field, and is not exiled", () => {
      const s = audit({ p2: { field: [HIGH, { def: FIENDER, stack: true }] } });
      const buried = s.card(HIGH);
      s.play(AUDIT);
      expect(s.unit("p2", 1)?.defId).toBe(FIENDER);
      expect(s.card(buried).zone.z).toBe("field");
    });

    it("nothing above its loc: nothing is exiled, and the Spell still resolves", () => {
      const s = audit({ p2: { field: [LOW] } });
      s.play(AUDIT);
      expect(s.unit("p2", 1)?.defId).toBe(LOW);
      s.expectInZone(AUDIT, "graveyard");
    });

    it("R280 the base face marks nothing: no preview", () => {
      expect(base.preview).toBeUndefined();
      expect(preview(audit({ p2: { field: [HIGH] } }))).toBeUndefined();
    });
  });

  describe("radiant", () => {
    it("R81 chooses with the play: all permanents, or only the opponent's", () => {
      expect(radiant.modes).toEqual([{ kind: "mode", options: [ALL, THEIRS] }]);
      const theirs = audit({ radiant: true, p1: { field: [HIGH] }, p2: { field: [HIGH, LOW] } });
      theirs.play(AUDIT, { modes: [THEIRS] });
      expect(theirs.unit("p1", 1)?.defId).toBe(HIGH);
      expect(theirs.unit("p2", 1)).toBeNull();
      expect(theirs.unit("p2", 2)?.defId).toBe(LOW);
      const all = audit({ radiant: true, p1: { field: [HIGH] }, p2: { field: [HIGH] } });
      all.play(AUDIT, { modes: [ALL] });
      expect([all.unit("p1", 1), all.unit("p2", 1)]).toEqual([null, null]);
    });

    it("R280 R583 its preview is, for each choice, the permanents it would exile now — and that is what the play exiles", () => {
      const s = audit({ radiant: true, p1: { field: [HIGH, LOW] }, p2: { field: [HIGH, LOW] } });
      const [mine, theirs] = [s.unit("p1", 1)?.id, s.unit("p2", 1)?.id];
      expect(preview(s)).toEqual([
        { label: "all permanents", value: 2, ids: [mine, theirs] },
        { label: "only your opponent's", value: 1, ids: [theirs] },
      ]);
      for (const entry of preview(s) ?? []) expect(cardDef(AUDIT).radiant.text).toContain(entry.label);
      s.play(AUDIT, { modes: [ALL] });
      expect([s.card(mine ?? "").zone.z, s.card(theirs ?? "").zone.z]).toEqual(["exile", "exile"]);
    });

    it("R177 R583 the preview never marks an enemy face-down card, which the exile still takes", () => {
      const s = audit({ radiant: true, p2: { backrow: [{ def: HIGH_TRAP, faceUp: false }] } });
      expect(preview(s)).toEqual([
        { label: "all permanents", value: 0, ids: [] },
        { label: "only your opponent's", value: 0, ids: [] },
      ]);
      s.play(AUDIT, { modes: [THEIRS] });
      s.expectInZone(HIGH_TRAP, "exile");
    });
  });
});
