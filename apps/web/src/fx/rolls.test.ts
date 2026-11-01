// R2652001: the client's reading of a `rolled` batch (rolls.ts): its one adapter, the words the log and
// the live region say, the tumble each reel shows, and the panel the effects layer plans and mounts.

import { ROLL_HEADS, ROLL_TAILS } from "@jackioh/engine/config";
import type { GameEvent } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { FX_BANNER_TAIL_MS, FX_ROLL_LAND_AT, FX_ROLL_LAND_LAST, FX_ROLL_TUMBLE_FACES, FX_TEXT } from "./constants.ts";
import { mountDomEffect } from "./dom.ts";
import { extraDraws, isHeads, rollBatchOf, rollCues, rollLandAt, rollTumble, rollWords } from "./rolls.ts";
import type { FxRollCue } from "./types.ts";

const D = 1_200;

function rolled(kind: "coin" | "die", sides: number, rolls: { faces: number[]; kept: number }[]): GameEvent {
  return { type: "rolled", player: "p1", instanceId: "c4", defId: "core-004", kind, sides, rolls };
}

function panelOf(event: GameEvent): FxRollCue {
  const panel = rollCues(event, D, 1).find((cue): cue is FxRollCue => cue.kind === "roll");
  if (panel === undefined) throw new Error("no roll panel planned");
  return panel;
}

describe("R2652001 the batch as words", () => {
  it("R2652001 the adapter reads only `rolled`, every face and the one kept", () => {
    const event = rolled("coin", 2, [{ faces: [ROLL_TAILS, ROLL_HEADS], kept: 1 }]);
    expect(rollBatchOf(event)).toEqual({
      player: "p1",
      instanceId: "c4",
      defId: "core-004",
      kind: "coin",
      sides: 2,
      rolls: [{ faces: [ROLL_TAILS, ROLL_HEADS], kept: 1 }],
    });
    expect(rollBatchOf({ type: "boardReplaced", player: "p1" })).toBeNull();
    expect(isHeads(ROLL_HEADS)).toBe(true);
    expect(isHeads(ROLL_TAILS)).toBe(false);
  });

  it("R2652001 coins and dice say what landed, and how many more Lucky rolled", () => {
    const say = (event: GameEvent): string => {
      const batch = rollBatchOf(event);
      if (batch === null) throw new Error("not a batch");
      return rollWords(batch, "Gary the Gambler");
    };
    expect(say(rolled("coin", 2, [{ faces: [ROLL_HEADS], kept: 0 }]))).toBe("Gary the Gambler flipped a coin: heads");
    const seven = [1, 2, 1, 1, 2, 1, 2].map((face) => ({ faces: [face], kept: 0 }));
    expect(say(rolled("coin", 2, seven))).toBe("Gary the Gambler flipped 7 coins: 4 heads, 3 tails");
    expect(say(rolled("die", 100, [{ faces: [67], kept: 0 }]))).toBe("Gary the Gambler rolled a 100-sided die: 67");
    expect(say(rolled("die", 80, [{ faces: [3], kept: 0 }, { faces: [67], kept: 0 }]))).toBe("Gary the Gambler rolled 2 80-sided dice: 3; 67");
    // Lucky's extra draws: the kept face is what the sentence counts, the extras are told after it.
    const lucky = rolled("coin", 2, [{ faces: [ROLL_TAILS, ROLL_HEADS], kept: 1 }, { faces: [ROLL_TAILS, ROLL_TAILS], kept: 0 }]);
    expect(say(lucky)).toBe("Gary the Gambler flipped 2 coins: 1 heads, 1 tails (Lucky rolled 2 more)");
    const batch = rollBatchOf(lucky);
    expect(batch === null ? -1 : extraDraws(batch)).toBe(2);
  });
});

describe("R2652001 the panel", () => {
  it("R2652001 every item lands inside the entry, the last by FX_ROLL_LAND_LAST, and the panel is gone by D + FX_BANNER_TAIL_MS", () => {
    const ten = Array.from({ length: 10 }, (_, i) => ({ faces: [i % 2 === 0 ? ROLL_HEADS : ROLL_TAILS], kept: 0 }));
    const panel = panelOf(rolled("coin", 2, ten));
    expect(panel.title).toBe(FX_TEXT.coinsFlipped);
    expect(panel.die).toBe("coin");
    expect(panel.items).toHaveLength(10);
    expect(panel.durationMs).toBe(D + FX_BANNER_TAIL_MS);
    expect(panel.items[0]?.landMs).toBe(Math.round(FX_ROLL_LAND_AT * D));
    const lands = panel.items.map((item) => item.landMs);
    expect([...lands].sort((a, b) => a - b)).toEqual(lands);
    expect(Math.max(...lands)).toBeLessThanOrEqual(Math.round(FX_ROLL_LAND_LAST * D));
    expect(rollLandAt(0, 1)).toBe(FX_ROLL_LAND_AT);
    // One spark burst as each lands.
    expect(rollCues(rolled("coin", 2, ten), D, 1).filter((cue) => cue.kind === "burst")).toHaveLength(10);
    expect(rollCues(rolled("coin", 2, []), D, 1)).toEqual([]);
  });

  it("R2652001 the kept face lands last in its tumble, a coin alternating and a die never showing it early", () => {
    for (const face of [ROLL_HEADS, ROLL_TAILS]) {
      const tumble = rollTumble("coin", 2, face, 0);
      expect(tumble).toHaveLength(FX_ROLL_TUMBLE_FACES + 1);
      expect(tumble[tumble.length - 1]).toBe(face);
      expect(tumble[tumble.length - 2]).not.toBe(face);
      for (let k = 1; k < tumble.length; k += 1) expect(tumble[k]).not.toBe(tumble[k - 1]);
    }
    for (const [sides, face] of [[100, 67], [80, 1], [6, 6]] as const) {
      for (const i of [0, 1, 2]) {
        const tumble = rollTumble("die", sides, face, i);
        expect(tumble[tumble.length - 1]).toBe(face);
        for (const shown of tumble.slice(0, -1)) {
          expect(shown).not.toBe(face);
          expect(shown).toBeGreaterThanOrEqual(1);
          expect(shown).toBeLessThanOrEqual(sides);
        }
      }
    }
    // The same roll tumbles the same every time.
    expect(rollTumble("die", 100, 67, 1)).toEqual(rollTumble("die", 100, 67, 1));
    const dice = panelOf(rolled("die", 100, [{ faces: [41], kept: 0 }]));
    expect(dice.title).toBe(FX_TEXT.diceRolled);
    expect(dice.items[0]?.tumble[FX_ROLL_TUMBLE_FACES]).toBe("41");
  });

  it("R2652001 Lucky's extra draws are drawn beside the kept face, which is marked", () => {
    const panel = panelOf(rolled("coin", 2, [{ faces: [ROLL_TAILS, ROLL_HEADS], kept: 1 }, { faces: [ROLL_HEADS], kept: 0 }]));
    expect(panel.items[0]).toMatchObject({ faces: ["tails", "heads"], kept: 1 });
    const root = document.createElement("div");
    document.body.appendChild(root);
    const effect = mountDomEffect(root, panel, {});
    expect(effect).not.toBeNull();
    const el = effect?.el;
    expect(el?.className).toBe("fx-roll");
    expect(el?.getAttribute("aria-hidden")).toBe("true");
    expect(el?.getAttribute("data-text")).toBe(FX_TEXT.coinsFlipped);
    const items = Array.from(el?.querySelectorAll<HTMLElement>(".fx-roll-item") ?? []);
    expect(items).toHaveLength(2);
    expect(items[0]?.style.getPropertyValue("--fx-land-ms")).toBe(`${String(panel.items[0]?.landMs ?? 0)}ms`);
    expect(items[0]?.querySelector(".fx-roll-cell[data-landed]")?.getAttribute("data-text")).toBe("heads");
    expect(Array.from(items[0]?.querySelectorAll(".fx-roll-kept, .fx-roll-extra") ?? []).map((chip) => [chip.className, chip.getAttribute("data-text")])).toEqual([
      ["fx-roll-extra", "tails"],
      ["fx-roll-kept", "heads"],
    ]);
    // A coin that drew one face shows no draws row.
    expect(items[1]?.querySelector(".fx-roll-draws")).toBeNull();
    effect?.remove();
    root.remove();
  });
});
