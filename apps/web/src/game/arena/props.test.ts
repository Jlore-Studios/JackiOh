// R3348001 (#3348): the ten playmats' corner props: two each, one by each corner, on the field taps'
// own sounds, and four hidden sequences.

import type { ReactElement } from "react";
import { describe, expect, it } from "vitest";

import { RAPID_SFX } from "../../audio/constants.ts";
import { cosmeticsOf } from "../../cards/cosmetics.ts";
import { PROP_ART } from "./PropArt.tsx";
import { CORNER_PROPS, CORNER_SEQUENCES, propsOf, sequenceOf } from "./props.ts";

const MATS = cosmeticsOf("table").map((cosmetic) => cosmetic.id);

describe("R3348001 the corner props", () => {
  it("R3348001 each of the ten playmats has two props, one by each corner, and no other mat has any", () => {
    expect(MATS).toHaveLength(10);
    expect(Object.keys(CORNER_PROPS).sort()).toEqual([...MATS].sort());
    for (const mat of MATS) {
      const props = propsOf(mat);
      expect(props, mat).toHaveLength(2);
      expect(props.map((prop) => prop.corner), mat).toEqual(["left", "right"]);
    }
    expect(propsOf("table-unknown")).toEqual([]);
    expect(propsOf("constructor")).toEqual([]);
  });

  it("R3348001 prop ids are unique and begin with a name of their mat's own, and each has its picture", () => {
    const ids = MATS.flatMap((mat) => propsOf(mat).map((prop) => prop.id));
    expect(new Set(ids).size).toBe(20);
    expect(Object.keys(PROP_ART).sort()).toEqual([...ids].sort());
    for (const id of ids) {
      const art = PROP_ART[id] as ReactElement<{ children: ReactElement<{ className: string }>[] }> | undefined;
      expect(art, id).toBeDefined();
      expect(art?.type).toBe("svg");
      const shapes = art?.props.children ?? [];
      expect(shapes.length, `${id} draws at most six shapes`).toBeLessThanOrEqual(6);
      for (const shape of shapes) expect(shape.props.className, id).toMatch(/^p[abc]$/);
    }
  });

  it("R3348001 every prop sounds on a field tap's recipe, so it never spends the game's voices", () => {
    for (const mat of MATS) {
      for (const prop of propsOf(mat)) expect(RAPID_SFX.has(prop.sfx), `${prop.id}: ${prop.sfx}`).toBe(true);
    }
  });

  it("R3348001 within a mat the two props differ in sound and in motion", () => {
    for (const mat of MATS) {
      const [left, right] = propsOf(mat);
      if (left === undefined || right === undefined) throw new Error("two props");
      expect(`${left.sfx}/${left.variant}`, mat).not.toBe(`${right.sfx}/${right.variant}`);
      expect(left.move, mat).not.toBe(right.move);
      for (const prop of [left, right]) {
        expect(prop.pitch).toBeGreaterThanOrEqual(0);
        expect(prop.pitch).toBeLessThanOrEqual(1);
      }
    }
  });

  it("R3348001 four mats hide a sequence of their own props, right, left, right", () => {
    expect(Object.keys(CORNER_SEQUENCES).sort()).toEqual(["table-ancient-map", "table-astral", "table-chessboard", "table-tavern"]);
    for (const [mat, sequence] of Object.entries(CORNER_SEQUENCES)) {
      const [left, right] = propsOf(mat);
      expect(sequence, mat).toEqual([right?.id, left?.id, right?.id]);
      expect(sequenceOf(mat)).toBe(sequence);
    }
    expect(sequenceOf("table-wood")).toBeUndefined();
    expect(sequenceOf("constructor")).toBeUndefined();
  });
});
