// R3572001: the held Stinkyscape+ set's own art (#3572, stinkyscapeplus.ts). Every entry of the set is routed to it;
// the ten Terminus Starships and the five Automa-ζ draw a subject of their own, every other card Stinkyscape's painter
// under the Mindustrial overlay; each picture draws at least 60 shapes and stays under the ceiling; both faces share
// their geometry; no base face of the set's own parts paints in Radiant's gold window; and no other set's card reaches
// any of it. Its Legendaries and Mythics draw scenes first (stinkyscapePlusScenes.test.ts).

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { RADIANT_GOLD_HUE_MAX_DEG, RADIANT_GOLD_HUE_MIN_DEG, RADIANT_GOLD_LIGHTNESS_MAX, RADIANT_GOLD_LIGHTNESS_MIN, RADIANT_GOLD_SATURATION_MIN } from "../constants.ts";
import { CardArt } from "./CardArt.tsx";
import { proceduralCardArtUri } from "../../patches/cardArt.ts";
import { artPlaneUri } from "./pictures.ts";
import { ART_DETAIL_MAX_BYTES, artElements } from "./richness.ts";
import { stinkyscapeSvg } from "./stinkyscape.ts";
import {
  AUTOMA_ZETA,
  isStinkyscapePlusArt,
  mindustrialOverlay,
  OVERLAY_SHAPES_MIN,
  STINKYSCAPE_PLUS_FAMILY,
  STINKYSCAPE_PLUS_PREFIX,
  stinkyscapePlusColors,
  stinkyscapePlusSvg,
  TERMINUS_SHIPS,
} from "./stinkyscapeplus.ts";
import { artMotionFor } from "./motion.ts";
import { stinkyscapePlusArtUri } from "./svg.ts";

afterEach(cleanup);

const SET: readonly CardDef[] = Object.values(CATALOG).filter((card) => card.set === "Stinkyscape+");
const OTHERS: readonly CardDef[] = Object.values(CATALOG).filter((card) => card.set !== "Stinkyscape+");
const SVG_PREFIX = "data:image/svg+xml,";

function svgOf(card: CardDef, radiant: boolean): string {
  return stinkyscapePlusSvg(card.id, card.name, card.tags, card.type, radiant);
}

function byId(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`the catalog has no ${id}`);
  return found;
}

/** A hex colour's hue in degrees and saturation and lightness from 0 to 1. */
function hsl(hex: string): { h: number; s: number; l: number } {
  const digits = hex.replace("#", "");
  const [r, g, b] = [0, 2, 4].map((at) => parseInt(digits.slice(at, at + 2), 16) / 255) as [number, number, number];
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  const d = max - min;
  if (d === 0) return { h: 0, s: 0, l };
  const s = d / (1 - Math.abs(2 * l - 1));
  const sector = max === r ? ((g - b) / d + 6) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return { h: sector * 60, s, l };
}

function inRadiantGold(hex: string): boolean {
  const { h, s, l } = hsl(hex);
  return (
    h >= RADIANT_GOLD_HUE_MIN_DEG && h <= RADIANT_GOLD_HUE_MAX_DEG && s >= RADIANT_GOLD_SATURATION_MIN && l >= RADIANT_GOLD_LIGHTNESS_MIN && l <= RADIANT_GOLD_LIGHTNESS_MAX
  );
}

/** Every hex colour a picture paints. */
function colorsOf(svg: string): string[] {
  return [...new Set(svg.match(/#[0-9a-fA-F]{6}\b/g) ?? [])];
}

const SHIPS = SET.filter((card) => card.tags.includes("Starship"));
const ZETAS = SET.filter((card) => card.tags.includes("Automa-ζ"));

describe("R3572001: the Stinkyscape+ set draws its own art", () => {
  it("R3572001 every entry of the set is routed to its art, and no other set's card is", () => {
    expect(SET).toHaveLength(204);
    expect(SET.every((card) => card.id.startsWith(STINKYSCAPE_PLUS_PREFIX))).toBe(true);
    expect(SET.every((card) => isStinkyscapePlusArt(card.id))).toBe(true);
    expect(OTHERS.some((card) => isStinkyscapePlusArt(card.id))).toBe(false);
    // Stinkyscape's own prefix (`stinkyscape-`) is not the set's.
    expect(isStinkyscapePlusArt("stinkyscape-001")).toBe(false);
    expect(isStinkyscapePlusArt("stinkyscapeplus-001")).toBe(true);
  });

  it("R3572001 the two tables are the catalog's ten Terminus Starships and five Automa-ζ, each its own look", () => {
    expect(Object.keys(TERMINUS_SHIPS).sort()).toEqual(SHIPS.map((card) => card.id).sort());
    expect(SHIPS).toHaveLength(10);
    expect(SHIPS.every((card) => card.id.startsWith("stinkyscapeplus-t-terminus-starship-") && card.type === "Unit" && card.token)).toBe(true);
    expect(Object.keys(AUTOMA_ZETA).sort()).toEqual(ZETAS.map((card) => card.id).sort());
    expect(ZETAS).toHaveLength(5);
    expect(ZETAS.every((card) => card.tags.includes("Mindustrial") && card.type === "Unit")).toBe(true);
    // A ship is a hull in a livery no other ship wears; the Automa grow each rung bigger and better armed.
    const looks = Object.values(TERMINUS_SHIPS).map((ship) => JSON.stringify([ship.hull, ship.livery]));
    expect(new Set(looks).size).toBe(10);
    expect(new Set(Object.values(TERMINUS_SHIPS).map((ship) => ship.hull)).size).toBe(10);
    const rungs = ZETAS.map((card) => AUTOMA_ZETA[card.id]).sort((a, b) => (a?.scale ?? 0) - (b?.scale ?? 0));
    const byCost = [...ZETAS].sort((a, b) => Number(a.cost) - Number(b.cost)).map((card) => AUTOMA_ZETA[card.id]);
    expect(rungs).toEqual(byCost);
    for (const [k, look] of byCost.entries()) {
      if (look === undefined) throw new Error("no look");
      const next = byCost[k + 1];
      if (next !== undefined) {
        expect(next.scale).toBeGreaterThan(look.scale);
        expect(next.guns + next.wings + next.pods).toBeGreaterThan(look.guns + look.wings + look.pods);
      }
    }
  });

  it("R3572001 the ships and the fusions draw subjects of their own, no two alike, none the painter's", () => {
    const seen = new Set<string>();
    for (const card of [...SHIPS, ...ZETAS]) {
      const svg = svgOf(card, false);
      expect(svg, card.id).not.toBe(stinkyscapeSvg(card.id, card.name, card.tags, card.type, false));
      expect(seen.has(svg), card.id).toBe(false);
      seen.add(svg);
      // Each wears its own plating: a ship its livery's hull, a fusion its look's.
      const hull = TERMINUS_SHIPS[card.id]?.livery.hull ?? AUTOMA_ZETA[card.id]?.hull;
      expect(svg, card.id).toContain(`stop-color="${hull}"`);
    }
    // A hull is its design's: two ships of one livery on two hulls differ.
    const [first, second] = SHIPS;
    if (first === undefined || second === undefined) throw new Error("no ships");
    expect(svgOf(first, false)).not.toBe(svgOf(second, false));
  });
});

describe("R3572001: the set's pictures are clean, detailed and under the ceiling", () => {
  it("R3572001 every entry, both faces: one <svg viewBox=\"0 0 100 100\">, no NaN, undefined, <text, <title or external reference", () => {
    for (const card of SET) {
      for (const radiant of [false, true]) {
        const svg = svgOf(card, radiant);
        expect(svg.startsWith('<svg viewBox="0 0 100 100"'), card.id).toBe(true);
        expect(svg.match(/<svg/g), card.id).toHaveLength(1);
        expect(svg, card.id).not.toMatch(/NaN|undefined|Infinity|<text|<title|href|<image/);
        expect(svg.replace('xmlns="http://www.w3.org/2000/svg"', ""), card.id).not.toMatch(/https?:/);
      }
    }
  });

  it("R3572001 every picture, base and Radiant, draws at least 60 shapes and stays under ART_DETAIL_MAX_BYTES", () => {
    for (const card of SET) {
      for (const radiant of [false, true]) {
        expect(artElements(svgOf(card, radiant)), `${card.id} ${radiant ? "radiant" : "base"}`).toBeGreaterThanOrEqual(60);
        const length = proceduralCardArtUri(card, radiant).length;
        expect(length, `${card.id} ${radiant ? "radiant" : "base"}`).toBeLessThanOrEqual(ART_DETAIL_MAX_BYTES);
      }
    }
  });

  it("R3572001 a picture is a pure function of the card: drawn twice, it is the same, and no two entries share one", () => {
    const seen = new Set<string>();
    for (const card of SET) {
      const base = svgOf(card, false);
      expect(svgOf(card, false), card.id).toBe(base);
      expect(seen.has(base), card.id).toBe(false);
      seen.add(base);
    }
  });

  it("R3572001 the Radiant face keeps the base geometry, gilds its paint and adds the rays", () => {
    const shapes = (svg: string): string[] => [...svg.matchAll(/(?:points|d)="([^"]*)"/g)].map((match) => match[1] ?? "");
    for (const card of SET) {
      const base = svgOf(card, false);
      const radiant = svgOf(card, true);
      expect(radiant, card.id).not.toBe(base);
      const radiantShapes = shapes(radiant);
      for (const d of shapes(base)) expect(radiantShapes, card.id).toContain(d);
      expect(radiantShapes.length, card.id).toBeGreaterThan(shapes(base).length);
    }
  });

  it("R3572001 no ship, fusion or overlay paints in Radiant's gold window on a base face", () => {
    for (const card of [...SHIPS, ...ZETAS]) {
      const own = colorsOf(svgOf(card, false)).filter((hex) => !["#ffffff", "#000000"].includes(hex));
      for (const hex of own) expect(inRadiantGold(hex), `${card.id}: ${hex}`).toBe(false);
    }
    for (const id of ["stinkyscapeplus-001", "stinkyscapeplus-100", "stinkyscapeplus-t-whatever"]) {
      for (const hex of colorsOf(mindustrialOverlay(id, false))) expect(inRadiantGold(hex), `${id}: ${hex}`).toBe(false);
    }
  });
});

describe("R3572001: the Mindustrial overlay", () => {
  const PLAIN: readonly CardDef[] = SET.filter((card) => !Object.hasOwn(TERMINUS_SHIPS, card.id) && !Object.hasOwn(AUTOMA_ZETA, card.id));

  it("R3572001 the overlay draws at least 30 shapes from the id's hash, and is the same drawn twice", () => {
    expect(OVERLAY_SHAPES_MIN).toBe(30);
    const seen = new Set<string>();
    for (const card of SET) {
      const overlay = mindustrialOverlay(card.id, false);
      expect(artElements(overlay), card.id).toBeGreaterThanOrEqual(OVERLAY_SHAPES_MIN);
      expect(mindustrialOverlay(card.id, false), card.id).toBe(overlay);
      seen.add(overlay);
    }
    // A hash varies it: not every card draws the same girders and gears.
    expect(seen.size).toBeGreaterThan(SET.length / 2);
  });

  it("R3572001 every card but a ship and a fusion draws the painter's picture under the overlay, none when it draws the subject alone", () => {
    expect(PLAIN).toHaveLength(204 - 15);
    for (const card of PLAIN) {
      for (const radiant of [false, true]) {
        const whole = stinkyscapePlusSvg(card.id, card.name, card.tags, card.type, radiant);
        const painter = stinkyscapeSvg(card.id, card.name, card.tags, card.type, radiant);
        const overlay = mindustrialOverlay(card.id, radiant);
        expect(whole, card.id).toContain(overlay);
        expect(artElements(whole) - artElements(painter), card.id).toBe(artElements(overlay));
        // The subject alone is the painter's, with no overlay.
        const subject = stinkyscapePlusSvg(card.id, card.name, card.tags, card.type, radiant, null, card.id, "subject");
        expect(subject, card.id).toBe(stinkyscapeSvg(card.id, card.name, card.tags, card.type, radiant, null, card.id, "subject"));
        expect(subject, card.id).not.toContain(overlay);
        // The backdrop keeps it.
        expect(stinkyscapePlusSvg(card.id, card.name, card.tags, card.type, radiant, null, card.id, "backdrop"), card.id).toContain(overlay);
      }
    }
    for (const card of [...SHIPS, ...ZETAS]) expect(svgOf(card, false), card.id).not.toContain(mindustrialOverlay(card.id, false));
  });

  it("R3572001 the overlay is a layer of the picture's props when the picture moves, and a depth plane's middle", () => {
    const card = byId("stinkyscapeplus-001");
    const motion = artMotionFor(card.id);
    if (motion === null) throw new Error("no motion");
    const withProps = { ...motion, props: { move: "float", seconds: 3.5 } } as const;
    const moving = stinkyscapePlusSvg(card.id, card.name, card.tags, card.type, false, withProps);
    expect(moving).toContain('<g class="art-m-props">');
    expect(moving).toContain(mindustrialOverlay(card.id, false));
    // Still, it draws no wrapper; and each plane draws what its role says.
    expect(stinkyscapePlusSvg(card.id, card.name, card.tags, card.type, false)).not.toContain("art-m-props");
    const planes = (["far", "middle", "near", "nearest"] as const).map((plane) => decodeURIComponent(artPlaneUri(card.id, plane).slice(SVG_PREFIX.length)));
    const overlay = mindustrialOverlay(card.id, false);
    expect(planes.map((svg) => svg.includes(overlay))).toEqual([false, true, false, false]);
  });
});

describe("R3572001: CardArt draws the set's art and leaves every other card's", () => {
  it("R3572001 a Stinkyscape+ face carries its family and the set's picture", () => {
    for (const card of [byId("stinkyscapeplus-001"), ...SHIPS.slice(0, 1), ...ZETAS.slice(0, 1)]) {
      const { container } = render(<CardArt defId={card.id} radiant={false} tags={card.tags} type={card.type} name={card.name} shape="portrait" />);
      const art = container.querySelector<HTMLElement>(".cf-art");
      expect(art?.dataset.artFamily, card.id).toBe(STINKYSCAPE_PLUS_FAMILY);
      expect(art?.dataset.artMotif, card.id).toBeUndefined();
      expect(art?.style.backgroundImage, card.id).toContain(stinkyscapePlusArtUri(card.id, card.name, card.tags, card.type, false));
      cleanup();
    }
  });

  it("R3572001 a shipped card draws as before, with no Stinkyscape+ family", () => {
    for (const card of OTHERS.slice(0, 40)) {
      const { container } = render(<CardArt defId={card.id} radiant={false} tags={card.tags} type={card.type} shape="portrait" />);
      expect(container.querySelector<HTMLElement>(".cf-art")?.dataset.artFamily, card.id).not.toBe(STINKYSCAPE_PLUS_FAMILY);
      cleanup();
    }
  });

  it("R3572001 a tile's aura takes the picture's own colours: a ship's livery, a fusion's paint, else the painter's", () => {
    for (const card of SET) {
      const colors = stinkyscapePlusColors(card);
      expect(colors, card.id).toHaveLength(3);
      for (const hex of colors) expect(hex, card.id).toMatch(/^#[0-9a-f]{6}$/i);
    }
    const ship = SHIPS[0];
    if (ship === undefined) throw new Error("no ships");
    expect(stinkyscapePlusColors(ship)).toEqual([TERMINUS_SHIPS[ship.id]?.livery.trim, TERMINUS_SHIPS[ship.id]?.livery.hull, TERMINUS_SHIPS[ship.id]?.livery.glow]);
  });
});
