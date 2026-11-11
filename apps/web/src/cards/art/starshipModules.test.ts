// R3333001: each Starship Piece's module (starshipModules.ts), the part its own card, its Starship's hull and its icon
// draw: one for each of the fifty piece definitions, each apart from every other, each tier of a weapon's or a miner's
// ladder bigger and busier than the one below it, each on the hardpoint its kind takes.

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import {
  LADDERED_KINDS,
  MODULE_KINDS,
  MODULE_TIER_MAX,
  moduleMarkup,
  modulePaint,
  moduleScale,
  moduleTierScale,
  STARSHIP_MODULE_IDS,
  starshipModuleOf,
  type ModuleKind,
  type StarshipModule,
} from "./starshipModules.ts";
import { STARSHIP_PIECES, tierOf } from "./stinkyscape.ts";

/** The three Jlockeed weapons that are Starship Pieces only when Embiggened (#127 to #129). */
const EMBIGGENED = ["stinkyscape-127", "stinkyscape-128", "stinkyscape-129"] as const;
const IDS = Object.keys(STARSHIP_MODULE_IDS);

function drawn(m: StarshipModule): string {
  return moduleMarkup(m, modulePaint(m));
}

/** A kind's module at `tier`, as a piece of that tier would draw it. */
function atTier(kind: ModuleKind, tier: number): StarshipModule {
  const { group, mount, hangar } = MODULE_KINDS[kind];
  return { defId: `test-${kind}-${String(tier)}`, kind, tier, group, mount, hangar };
}

function elements(svg: string): number {
  return (svg.match(/<(?!\/)[a-z]+/g) ?? []).length;
}

describe("R3333001: a module for every Starship Piece", () => {
  it("R3333001 the modules cover the fifty piece definitions, each tier its name's", () => {
    expect(IDS.length).toBe(50);
    expect([...IDS].sort()).toEqual([...STARSHIP_PIECES, ...EMBIGGENED].sort());
    for (const id of IDS) {
      const card = CATALOG[id];
      expect(card, id).toBeDefined();
      expect(starshipModuleOf(id).tier, id).toBe(tierOf(card?.name ?? ""));
    }
  });

  it("R3333001 every module is drawn apart from every other", () => {
    const pictures = IDS.map((id) => drawn(starshipModuleOf(id)));
    expect(new Set(pictures).size).toBe(IDS.length);
    // Every kind but the spare plate is some piece's, and no two pieces share a kind and a tier.
    const kinds = new Set(IDS.map((id) => starshipModuleOf(id).kind));
    expect([...kinds].sort()).toEqual(Object.keys(MODULE_KINDS).filter((kind) => kind !== "spare").sort());
    expect(new Set(IDS.map((id) => `${starshipModuleOf(id).kind}${String(starshipModuleOf(id).tier)}`)).size).toBe(IDS.length);
  });

  it("R3333001 each tier of a weapon or a miner is bigger and busier than the one below it", () => {
    expect([...LADDERED_KINDS].sort()).toEqual(["beam", "cannon", "drill", "flak", "repeater"]);
    for (const kind of LADDERED_KINDS) {
      for (let tier = 2; tier <= MODULE_TIER_MAX; tier += 1) {
        const below = atTier(kind, tier - 1);
        const here = atTier(kind, tier);
        expect(drawn(here), `${kind} v${String(tier)}`).not.toBe(drawn(below));
        expect(elements(drawn(here)), `${kind} v${String(tier)}`).toBeGreaterThan(elements(drawn(below)));
        expect(moduleTierScale(tier)).toBeGreaterThan(moduleTierScale(tier - 1));
        expect(moduleScale(here)).toBeGreaterThan(moduleScale(below));
      }
    }
    // The pieces themselves: Cannon v3, v4 and v5 each outgrow the last; Mining Drill v1 to v5 too.
    const cannons = ["stinkyscape-052", "stinkyscape-056", "stinkyscape-060"].map((id) => drawn(starshipModuleOf(id)));
    const drills = ["stinkyscape-062", "stinkyscape-063", "stinkyscape-064", "stinkyscape-065", "stinkyscape-066"].map((id) => drawn(starshipModuleOf(id)));
    for (const ladder of [cannons, drills]) {
      ladder.slice(1).forEach((svg, k) => {
        expect(elements(svg)).toBeGreaterThan(elements(ladder[k] ?? ""));
      });
    }
  });

  it("R3333001 each module mounts where its kind does", () => {
    const mountOf = (id: string): string => starshipModuleOf(id).mount;
    // The weapons on the bow, the drills on the keel, the engines at the stern, the launchers and pods on the flanks.
    for (const id of ["stinkyscape-050", "stinkyscape-055", "stinkyscape-060", "stinkyscape-061", "stinkyscape-100", "stinkyscape-122"]) expect(mountOf(id), id).toBe("bow");
    for (const id of ["stinkyscape-062", "stinkyscape-066"]) expect(mountOf(id), id).toBe("keel");
    for (const id of ["stinkyscape-090", "stinkyscape-101", "stinkyscape-106"]) expect(mountOf(id), id).toBe("stern");
    for (const id of ["stinkyscape-140", "stinkyscape-141", "stinkyscape-142", "stinkyscape-143", "stinkyscape-043", "stinkyscape-134", "stinkyscape-177"]) {
      expect(mountOf(id), id).toBe("flank");
    }
    // The defenses on the spine behind the bow's guns.
    for (const id of ["stinkyscape-038", "stinkyscape-104", "stinkyscape-132", "stinkyscape-133"]) expect(mountOf(id), id).toBe("spine");
    for (const id of IDS) {
      const m = starshipModuleOf(id);
      expect(m, id).toMatchObject({ group: MODULE_KINDS[m.kind].group, mount: MODULE_KINDS[m.kind].mount, hangar: MODULE_KINDS[m.kind].hangar });
    }
    // The Embiggened Jlockeed weapons keep their blueprint's bay.
    for (const id of EMBIGGENED) expect(starshipModuleOf(id).hangar, id).toBe("blueprint");
  });

  it("R3333001 an id that is no piece draws the spare module", () => {
    for (const id of ["stinkyscape-t-dormant-starship", "core-008", "no-such-card"]) {
      expect(starshipModuleOf(id), id).toMatchObject({ kind: "spare", tier: 1, mount: "flank" });
    }
    expect(drawn(starshipModuleOf("core-008"))).toBe(drawn(starshipModuleOf("stinkyscape-t-dormant-starship")));
  });

  it("R3333001 a module is clean: flat fills, no text, nothing undefined", () => {
    for (const id of IDS) {
      const svg = drawn(starshipModuleOf(id));
      for (const banned of ["NaN", "undefined", "url(", "<text", "Infinity"]) expect(svg, `${id} ${banned}`).not.toContain(banned);
    }
  });
});
