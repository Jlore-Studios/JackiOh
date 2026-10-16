// R1017001: the hero portraits' cards print full art; no other card does, and a full face is the
// only layout that does. The pixels are cards.css's ("full art"), seen in the Almanac.

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { PORTRAITS } from "@jackioh/shared";

import { CardFace } from "./CardFace.tsx";
import { FULL_ART_NAMES, isFullArt } from "./fullArt.ts";
import { faceModel } from "./model.ts";

const DEFS = Object.values(CATALOG);

function faceOf(id: string, radiant: boolean, layout: "full" | "compact" = "full"): HTMLElement {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  const { container } = render(<CardFace face={faceModel({ defId: id, def, radiant })} layout={layout} />);
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error("no face");
  return cf;
}

afterEach(() => {
  cleanup();
});

describe("R1017001 full-art faces", () => {
  it("R1017001 the full-art cards are the hero portraits' cards, each one in the catalog", () => {
    const names = Object.values(PORTRAITS).map((portrait) => portrait.cardName);
    expect([...FULL_ART_NAMES].sort()).toEqual([...new Set(names)].sort());
    for (const name of names) expect(DEFS.some((def) => def.name === name), name).toBe(true);
    expect(isFullArt("Reach the Summit")).toBe(false);
  });

  it("R1017001 a full face of one of them, base or Radiant, fills itself with its art; other cards keep their windows", () => {
    // The six heroes' cards, and a card of every type besides (isFullArt decides the rest by name).
    const heroes = DEFS.filter((def) => FULL_ART_NAMES.has(def.name));
    const others = (["Unit", "Spell", "Field Spell", "Trap", "Field Trap"] as const).flatMap((type) =>
      DEFS.filter((def) => def.type === type && !FULL_ART_NAMES.has(def.name)).slice(0, 2),
    );
    expect(heroes).toHaveLength(FULL_ART_NAMES.size);
    for (const def of [...heroes, ...others]) {
      for (const radiant of [false, true]) {
        const cf = faceOf(def.id, radiant);
        const where = `${def.id} ${radiant ? "radiant" : "base"}`;
        const full = FULL_ART_NAMES.has(def.name);
        expect(cf.getAttribute("data-full-art"), where).toBe(full ? "true" : null);
        expect(cf.querySelector(".cf-art-frame > .cf-art")?.classList.contains("cf-art--full"), where).toBe(full);
        cleanup();
      }
    }
  });

  it("R1017001 a compact face (a face-up backrow card) keeps its window", () => {
    const def = DEFS.find((entry) => FULL_ART_NAMES.has(entry.name));
    if (def === undefined) throw new Error("no full-art card");
    expect(faceOf(def.id, false, "compact").hasAttribute("data-full-art")).toBe(false);
  });
});
