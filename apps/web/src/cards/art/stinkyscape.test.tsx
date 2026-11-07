// R2170000: the Stinkyscape set's own art (#2170, stinkyscape.ts). Each family draws one look, each
// weapon the weapon it names, both faces share their geometry, and no other set's card reaches any of
// it, so every picture R503 pinned stays.

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { RADIANT_GOLD_HUE_MAX_DEG, RADIANT_GOLD_HUE_MIN_DEG, RADIANT_GOLD_LIGHTNESS_MAX, RADIANT_GOLD_LIGHTNESS_MIN, RADIANT_GOLD_SATURATION_MIN } from "../constants.ts";
import STINKYSCAPE_V050 from "./fixtures/stinkyscape-art-v0.5.0.json";
import { CardArt } from "./CardArt.tsx";
import { hashId } from "./hash.ts";
import { moduleMarkup, modulePaint, STARSHIP_MODULE_IDS, starshipModuleOf } from "./starshipModules.ts";
import {
  isStinkyscapeArt,
  SHIP_DRONE_IDS,
  SHIP_LAYER_IDS,
  SHIP_LAYERS,
  shipArtOf,
  shipDecksOf,
  shipLayerOf,
  STARSHIP_PIECES,
  stinkyscapeSvg,
  stinkySubject,
  tierOf,
  type ShipArt,
  type ShipLayer,
  type StinkyFamily,
} from "./stinkyscape.ts";
import { proceduralArtUri, stinkyscapeArtUri } from "./svg.ts";
import { compositionFor, themeFor } from "./themes.ts";

afterEach(cleanup);

const SET: readonly CardDef[] = Object.values(CATALOG).filter((card) => isStinkyscapeArt(card.id));
const OTHERS: readonly CardDef[] = Object.values(CATALOG).filter((card) => !isStinkyscapeArt(card.id));

function byName(name: string): CardDef {
  const found = SET.find((card) => card.name === name);
  if (found === undefined) throw new Error(`the set has no ${name}`);
  return found;
}

function subjectOf(card: CardDef): ReturnType<typeof stinkySubject> {
  return stinkySubject(card.id, card.name, card.tags, card.type);
}

function svgOf(card: CardDef, radiant: boolean): string {
  return stinkyscapeSvg(card.id, card.name, card.tags, card.type, radiant);
}

function keywordsOf(card: CardDef): string[] {
  return card.base.keywords.map((keyword) => keyword.kind);
}

/** The families a ship's part draws in: never a crew member or a bare planet. */
const FLEET_FAMILIES: readonly StinkyFamily[] = ["fleet", "weapon", "miner", "drone", "ai"];

describe("R2170000: the Stinkyscape set draws its own families", () => {
  it("R2170000 the set is all there, and no other set's card is routed to its art", () => {
    expect(SET.length).toBeGreaterThan(0);
    expect(SET.every((card) => card.set === "Stinkyscape")).toBe(true);
    expect(OTHERS.some((card) => card.set === "Stinkyscape")).toBe(false);
  });

  it("R2170000 STARSHIP_PIECES is exactly the catalog's Starship Piece entries", () => {
    const printed = SET.filter((card) => keywordsOf(card).includes("Starship Piece")).map((card) => card.id);
    expect([...STARSHIP_PIECES].sort()).toEqual(printed.sort());
  });

  it("R3142002 the Stinkyscape Golem is no Starship Piece and is drawn as the fleet's golem, not as a fighter or a crew member", () => {
    const golem = byName("Stinkyscape Golem");
    expect(keywordsOf(golem)).not.toContain("Starship Piece");
    expect(STARSHIP_PIECES.has(golem.id)).toBe(false);
    for (const keyword of ["Flying", "Ranged", "Rush"]) expect(keywordsOf(golem)).toContain(keyword);
    expect(subjectOf(golem)).toMatchObject({ family: "fleet", kind: "golem" });
    // Its own painter: a stone-plated body, a cockpit visor, wing fins and two engine nozzles, so it is not
    // the picture the fighter hull draws for the same seed.
    const fighter = stinkyscapeSvg(golem.id, "Falcon", [], "Unit", false);
    expect(svgOf(golem, false)).not.toBe(fighter);
    expect(svgOf(golem, false)).toContain('fill="url(#vz)"');
    // The Training Grounds Golem is `traininggrounds.ts`'s, never this file's.
    expect(isStinkyscapeArt("traininggrounds-039-1")).toBe(false);
    // The ships that never printed the keyword keep their look.
    expect(subjectOf(byName("Dormant Starship"))).toMatchObject({ family: "fleet", kind: "dormant" });
    expect(subjectOf(byName("Falcon"))).toMatchObject({ family: "fleet", kind: "fighter" });
  });

  it("R3142002 a Human Unit is a crew figure whatever its name holds: the Drone Commander is crew, a Drone Interceptor stays a drone", () => {
    const commander = byName("Drone Commander");
    expect(commander.tags).toContain("Human");
    expect(subjectOf(commander)).toMatchObject({ family: "crew", kind: "commander" });
    for (const name of ["Drone Interceptor", "Drone Corvette", "Drone Hanger", "Drone Battleship (Shield Layer)"]) {
      expect(subjectOf(byName(name)).family, name).toBe("drone");
    }
    // Every other Human Unit is classified as it was: only a name holding "drone" moved.
    const moved = SET.filter((card) => card.type === "Unit" && card.tags.includes("Human") && /(^|[^a-z])drone([^a-z]|$)/i.test(card.name));
    expect(moved.map((card) => card.name)).toEqual(["Drone Commander"]);
    // Both faces draw the officer's drone hovering behind her: a drone's glowing eye and the crew visor.
    for (const radiant of [false, true]) {
      const svg = svgOf(commander, radiant);
      expect(svg).toContain('fill="url(#vz)"');
      expect(svg).toContain('fill="url(#dr)"');
    }
  });

  it("R2170000 every Drone Core is the bare drone core, its mark the count of its nodes, and every drone shares the drone", () => {
    const cores = SET.filter((card) => card.tags.includes("Drone Core"));
    expect(cores).toHaveLength(10);
    for (const card of cores) expect(subjectOf(card)).toMatchObject({ family: "drone", kind: "core" });
    expect(cores.map((card) => subjectOf(card).tier)).toEqual([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    for (const name of ["Drone Interceptor", "Drone Corvette", "Drone Battleship (Shield Layer)", "Drone Battleship (Propagation Layer)", "Drone Battleship (Meltdown Layer)", "Drone Hanger"]) {
      expect(subjectOf(byName(name)).family, name).toBe("drone");
    }
    expect(subjectOf(byName("Drone Battleship (Meltdown Layer)")).kind).toBe("meltdown");
  });

  it("R2170000 each weapon is the weapon its name says", () => {
    const named: Readonly<Record<string, string>> = {
      "Beam v3": "beam",
      "Repeater v4": "repeater",
      "Cannon v5": "cannon",
      "Flak v3": "flak",
      "Rush Token Launcher": "launcher",
      "Fruit Launcher": "launcher",
      "Jade Launcher": "launcher",
      "Book Launcher": "launcher",
      "AP Rounds": "rounds",
      "Vamp Rounds": "rounds",
      "Jlockeed's Yamato Cannon": "cannon",
      "Jlockeed's Missile Pod": "missiles",
      "Jlockeed's Siege Tank": "tank",
      "Freeze Ray": "ray",
      Disintegrex: "ray",
      "RR-5000": "rail",
      "Trila 3": "triple",
      Spacemine: "mine",
      "Tractor Beam": "tractor",
    };
    for (const [name, kind] of Object.entries(named)) expect(subjectOf(byName(name)).kind, name).toBe(kind);
    for (const card of SET.filter((each) => each.tags.includes("Weapon"))) {
      const word = card.name.split(" ")[0]?.toLowerCase() ?? "";
      const kind = subjectOf(card).kind;
      expect(subjectOf(card).family, card.name).toBe("weapon");
      expect(card.name.toLowerCase().includes(kind) || word === kind || kind === "launcher", `${card.name} draws ${kind}`).toBe(true);
    }
  });

  it("R2170000 every Starship Piece and Starship draws as part of the fleet", () => {
    // R3038000: a Starship is the card that carries the Starship tag since #3038, the Dormant Starship.
    const ships = SET.filter((card) => keywordsOf(card).includes("Starship Piece") || card.tags.includes("Starship"));
    for (const card of ships) expect(FLEET_FAMILIES, card.name).toContain(subjectOf(card).family);
    expect(ships.map((card) => card.name)).toContain("Dormant Starship");
    expect(subjectOf(byName("Dormant Starship"))).toMatchObject({ family: "fleet", kind: "dormant" });
    // R3038002: the Drone Battleship's layers, no Starships since #3038, still draw as the fleet's.
    for (const name of ["Drone Battleship (Shield Layer)", "Drone Battleship (Propagation Layer)", "Drone Battleship (Meltdown Layer)"]) {
      expect(ships.map((card) => card.name), name).not.toContain(name);
      expect(FLEET_FAMILIES, name).toContain(subjectOf(byName(name)).family);
    }
    for (const name of ["Paragon-X", "Falcon", "Shield Amplifier", "Poison Capsule", "After Burner", "Magic Module"]) {
      expect(subjectOf(byName(name)).family, name).toBe("fleet");
    }
  });

  it("R2170000 the set's other families: Terminus, the Doctrines, the Swarm, the Acclaimed, Jlockeed, the AI cards, the Resources, the Miners, the Anomalies", () => {
    const tagged: readonly (readonly [string, StinkyFamily])[] = [
      ["Terminus", "terminus"],
      ["War Doctrine", "doctrine"],
      ["Swarm", "swarm"],
      ["Acclaimed", "acclaimed"],
      ["Jlockeed", "jlockeed"],
      ["AI", "ai"],
    ];
    for (const [tag, family] of tagged) {
      const cards = SET.filter((card) => card.tags.includes(tag as CardDef["tags"][number]));
      expect(cards.length, tag).toBeGreaterThan(0);
      for (const card of cards) expect(subjectOf(card).family, card.name).toBe(family);
    }
    for (const name of ["Reknite", "Korrelite", "Gellium", "Axnit", "Narcor", "Red Narcor", "Metal Scraps", "Spice", "Power Cell", "Asteroid Field", "Tiny Asteroid", "Huge Asteroid", "Comet Crash"]) {
      expect(subjectOf(byName(name)).family, name).toBe("resource");
    }
    for (const name of ["Miner v1", "Miner v5", "Ozark", "Orca", "Galaxy"]) expect(subjectOf(byName(name)).family, name).toBe("miner");
    for (const name of ["Anomaly Found!", "Scanning Probe Mk1", "Scanning Probe Mk2"]) expect(subjectOf(byName(name)).family, name).toBe("anomaly");
    expect(subjectOf(byName("Power Word Shield")).family).toBe("space");
  });

  it("R2170000 a name's mark: v3, Mk2, X2, a Roman numeral, else 1", () => {
    expect(tierOf("Beam v3")).toBe(3);
    expect(tierOf("Paragon Mk2")).toBe(2);
    expect(tierOf("Prototype-X2")).toBe(2);
    expect(tierOf("Drone Core IX")).toBe(9);
    expect(tierOf("Drone Core IV")).toBe(4);
    expect(tierOf("Falcon")).toBe(1);
  });
});

describe("R2170000: the set's art is clean, deterministic and gilded on the Radiant face", () => {
  it("R2170000 every entry, both faces: one <svg viewBox=\"0 0 100 100\">, no NaN, undefined, <text, <title or external reference", () => {
    for (const card of SET) {
      for (const radiant of [false, true]) {
        const svg = svgOf(card, radiant);
        expect(svg.startsWith('<svg viewBox="0 0 100 100"'), card.id).toBe(true);
        expect(svg.match(/<svg/g)).toHaveLength(1);
        expect(svg, card.id).not.toMatch(/NaN|undefined|Infinity|<text|<title|href|<image/);
        expect(svg.replace('xmlns="http://www.w3.org/2000/svg"', ""), card.id).not.toMatch(/https?:/);
      }
    }
  });

  it("R2170000 a picture is a pure function of the card: drawn twice, it is the same, and no two entries share one", () => {
    const seen = new Set<string>();
    for (const card of SET) {
      const base = svgOf(card, false);
      expect(svgOf(card, false)).toBe(base);
      expect(seen.has(base), card.id).toBe(false);
      seen.add(base);
    }
  });

  it("R2170000 the Radiant face keeps the base geometry, gilds its paint and adds the rays", () => {
    const shapes = (svg: string): string[] => [...svg.matchAll(/(?:points|d)="([^"]*)"/g)].map((match) => match[1] ?? "");
    for (const card of SET) {
      const base = svgOf(card, false);
      const radiant = svgOf(card, true);
      expect(radiant, card.id).not.toBe(base);
      const baseShapes = shapes(base);
      const radiantShapes = shapes(radiant);
      for (const d of baseShapes) expect(radiantShapes, card.id).toContain(d);
      expect(radiantShapes.length).toBeGreaterThan(baseShapes.length);
    }
  });
});

/** A hex color's hue in degrees and saturation and lightness from 0 to 1 (as cosmetics.test.tsx reads them). */
function hsl(hex: string): { h: number; s: number; l: number } {
  const digits = hex.length <= 4 ? [...hex].map((digit) => digit + digit).join("") : hex;
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

/** R2725004: whether a color is within Radiant's gold window. */
function inRadiantGold(hex: string): boolean {
  const { h, s, l } = hsl(hex);
  return (
    h >= RADIANT_GOLD_HUE_MIN_DEG &&
    h <= RADIANT_GOLD_HUE_MAX_DEG &&
    s >= RADIANT_GOLD_SATURATION_MIN &&
    l >= RADIANT_GOLD_LIGHTNESS_MIN &&
    l <= RADIANT_GOLD_LIGHTNESS_MAX
  );
}

/** Every `#…` color in `text`, without its `#`. */
function hexes(text: string): string[] {
  return [...text.matchAll(/#([0-9a-fA-F]{3,8})\b/g)].map((match) => match[1] ?? "");
}

describe("R3333001: a Starship Piece's own card draws its module", () => {
  it("R3333001 every piece's own card draws its module in its bay, with no Radiant gold on its base face", () => {
    const pictures = new Set<string>();
    for (const id of Object.keys(STARSHIP_MODULE_IDS)) {
      const card = CATALOG[id];
      if (card === undefined) throw new Error(`no ${id}`);
      const m = starshipModuleOf(id);
      const base = svgOf(card, false);
      const radiant = svgOf(card, true);
      // The module, large, in the module's own paint, on the base face.
      expect(base, id).toContain(moduleMarkup(m, modulePaint(m)));
      pictures.add(base);
      expect(hexes(base).filter(inRadiantGold), `${id} base`).toEqual([]);
      expect(hexes(radiant).some(inRadiantGold), `${id} Radiant`).toBe(true);
    }
    expect(pictures.size).toBe(Object.keys(STARSHIP_MODULE_IDS).length);
  });
});

describe("R2170000: CardArt draws the set's art and leaves every other card's", () => {
  it("R2170000 a Stinkyscape face carries its family and the set's picture, read from its name", () => {
    const card = byName("Cannon v3");
    const { container } = render(<CardArt defId={card.id} radiant={false} tags={card.tags} type={card.type} name={card.name} shape="portrait" />);
    const art = container.querySelector<HTMLElement>(".cf-art");
    expect(art?.dataset.artFamily).toBe("weapon");
    expect(art?.dataset.artMotif).toBeUndefined();
    expect(art?.style.backgroundImage).toContain(stinkyscapeArtUri(card.id, card.name, card.tags, card.type, false));
  });

  it("R2170000 a shipped card draws R503's art as before, with no family", () => {
    for (const card of OTHERS.slice(0, 40)) {
      const { container } = render(<CardArt defId={card.id} radiant={false} tags={card.tags} type={card.type} shape="portrait" />);
      const art = container.querySelector<HTMLElement>(".cf-art");
      const theme = themeFor(card.tags, card.type);
      expect(art?.dataset.artFamily, card.id).toBeUndefined();
      expect(art?.style.backgroundImage, card.id).toContain(proceduralArtUri(card.id, theme, compositionFor(card.type), false, null));
      cleanup();
    }
  });
});

describe("R3336000: the Drone Battleship's three layers are the decks of one ship", () => {
  const LAYER_CARDS: Readonly<Record<ShipLayer, CardDef>> = {
    meltdown: byName("Drone Battleship (Meltdown Layer)"),
    propagation: byName("Drone Battleship (Propagation Layer)"),
    shield: byName("Drone Battleship (Shield Layer)"),
  };

  function shipSvg(layer: ShipLayer, radiant: boolean, ship: ShipArt | null): string {
    const card = LAYER_CARDS[layer];
    return stinkyscapeSvg(card.id, card.name, card.tags, card.type, radiant, null, card.id, "whole", ship);
  }

  /** The decks an SVG draws, bottom first as it paints them, and the one marked its own. */
  function decksOf(svg: string): { drawn: string[]; own: string[] } {
    return {
      drawn: [...svg.matchAll(/data-deck="(\w+)"/g)].map((match) => match[1] ?? ""),
      own: [...svg.matchAll(/data-deck="(\w+)" data-own="true"/g)].map((match) => match[1] ?? ""),
    };
  }

  /** The faint decks of a printed card: those drawn at the faint opacity. */
  function faintOf(svg: string): string[] {
    return [...svg.matchAll(/data-deck="(\w+)" opacity="0\.3"/g)].map((match) => match[1] ?? "");
  }

  it("R3336000 the layers are the three tokens, bottom first, and no other card is a layer", () => {
    expect(SHIP_LAYERS).toEqual(["meltdown", "propagation", "shield"]);
    for (const layer of SHIP_LAYERS) {
      expect(shipLayerOf(SHIP_LAYER_IDS[layer])).toBe(layer);
      expect(LAYER_CARDS[layer].id).toBe(SHIP_LAYER_IDS[layer]);
    }
    for (const id of [SHIP_DRONE_IDS.interceptor, SHIP_DRONE_IDS.corvette, "stinkyscape-036", "core-001"]) expect(shipLayerOf(id), id).toBeNull();
    expect(Object.values(SHIP_DRONE_IDS).every((id) => byName(CATALOG[id]?.name ?? "").id === id)).toBe(true);
  });

  it("R3336000 a printed layer draws its own deck large, in the middle, with the other two faint", () => {
    for (const layer of SHIP_LAYERS) {
      const svg = shipSvg(layer, false, null);
      const { drawn, own } = decksOf(svg);
      expect(own, layer).toEqual([layer]);
      expect([...drawn].sort(), layer).toEqual([...SHIP_LAYERS].sort());
      expect(drawn[drawn.length - 1], `${layer} is drawn last, over the others`).toBe(layer);
      expect(faintOf(svg).sort(), layer).toEqual(SHIP_LAYERS.filter((other) => other !== layer).sort());
      expect(shipDecksOf(SHIP_LAYER_IDS[layer], null), layer).toBe(SHIP_LAYERS.length);
    }
    // Each layer's card is its own deck's picture.
    const pictures = new Set(SHIP_LAYERS.map((layer) => shipSvg(layer, false, null)));
    expect(pictures.size).toBe(SHIP_LAYERS.length);
  });

  it("R3336000 the Radiant layer is the same ship gilded inside the Radiant rules: its geometry, its own paint", () => {
    const shapes = (svg: string): string[] => [...svg.matchAll(/(?:points|d)="([^"]*)"/g)].map((match) => match[1] ?? "");
    for (const layer of SHIP_LAYERS) {
      const base = shipSvg(layer, false, null);
      const radiant = shipSvg(layer, true, null);
      expect(radiant, layer).not.toBe(base);
      for (const d of shapes(base)) expect(shapes(radiant), layer).toContain(d);
      expect(decksOf(radiant).own, layer).toEqual([layer]);
    }
  });

  it("R3336000 the top card draws what is left of the ship: three decks, then two, then one", () => {
    const board: readonly (readonly [ShipLayer, number, readonly ShipLayer[]])[] = [
      ["shield", 2, ["meltdown", "propagation", "shield"]],
      ["propagation", 1, ["meltdown", "propagation"]],
      ["meltdown", 0, ["meltdown"]],
    ];
    for (const [layer, buried, decks] of board) {
      const svg = shipSvg(layer, false, { buried });
      const { drawn, own } = decksOf(svg);
      expect(drawn, layer).toEqual(decks);
      expect(own, layer).toEqual([layer]);
      expect(faintOf(svg), layer).toEqual([]);
      expect(shipDecksOf(SHIP_LAYER_IDS[layer], { buried }), layer).toBe(decks.length);
    }
    // A pile no deeper than its ship draws no deck that is not one of the three; a Shield with one card beneath draws two.
    expect(decksOf(shipSvg("shield", false, { buried: 1 })).drawn).toEqual(["propagation", "shield"]);
    expect(decksOf(shipSvg("shield", false, { buried: 9 })).drawn).toEqual(["meltdown", "propagation", "shield"]);
    expect(decksOf(shipSvg("propagation", false, { buried: 0 })).drawn).toEqual(["propagation"]);
    // Other cards have no decks, and a ship changes none of their pictures.
    const cannon = byName("Cannon v3");
    expect(shipDecksOf(cannon.id, { buried: 2 })).toBeNull();
    expect(stinkyscapeSvg(cannon.id, cannon.name, cannon.tags, cannon.type, false, null, cannon.id, "whole", { buried: 2 })).toBe(svgOf(cannon, false));
  });

  it("R3336000 the hangar's lights stay lit for the bays that still hold drones", () => {
    const lit = (bays: number): number => (shipSvg("propagation", false, { buried: 0, bays }).match(/data-bay="lit"/g) ?? []).length;
    const dark = (bays: number): number => (shipSvg("propagation", false, { buried: 0, bays }).match(/data-bay="dark"/g) ?? []).length;
    expect([lit(2), lit(1), lit(0)]).toEqual([2, 1, 0]);
    expect([dark(2), dark(1), dark(0)]).toEqual([0, 1, 2]);
    expect(lit(9)).toBe(2);
    // No count known (the opponent's view carries no Activates): both lit.
    expect((shipSvg("propagation", false, { buried: 0 }).match(/data-bay="lit"/g) ?? []).length).toBe(2);
    expect((shipSvg("propagation", false, null).match(/data-bay="lit"/g) ?? []).length).toBe(2);
  });

  it("R3336000 the reactor's core glows hotter and its cracks spread as the keel melts and its Brittle runs low", () => {
    const cracks = (ship: ShipArt): number => Number(shipSvg("meltdown", false, ship).match(/data-cracks="(\d+)"/)?.[1] ?? 0);
    const core = (heat: number): number => Number(shipSvg("meltdown", false, { buried: 0, heat }).match(/opacity="([\d.]+)" data-core/)?.[1]);
    expect(cracks({ buried: 0, heat: 0, brittle: 3 })).toBe(0);
    const grown = [0, 1, 2, 3, 4, 5, 6].map((heat) => cracks({ buried: 0, heat, brittle: 3 }));
    for (let k = 1; k < grown.length; k += 1) expect(grown[k]).toBeGreaterThanOrEqual(grown[k - 1] ?? 0);
    expect(grown[grown.length - 1]).toBe(6);
    expect(cracks({ buried: 0, heat: 0, brittle: 1 })).toBe(1);
    expect(cracks({ buried: 0, heat: 6, brittle: 1 })).toBe(6);
    expect(cracks({ buried: 0, heat: 3, brittle: 1 })).toBeGreaterThan(cracks({ buried: 0, heat: 3, brittle: 3 }));
    const cores = [0, 2, 4, 6].map(core);
    for (let k = 1; k < cores.length; k += 1) expect(cores[k]).toBeGreaterThan(cores[k - 1] ?? 1);
    // A printed keel is whole, its core a third melted.
    expect(shipSvg("meltdown", false, null)).not.toContain("data-cracks");
  });

  it("R3336000 a layer unit's ship is its pile, its melting, its Brittle and the bays that still hold drones", () => {
    const unit = { buried: 2, health: 6, maxHealth: 12, brittle: 2, activations: undefined };
    expect(shipArtOf(SHIP_LAYER_IDS.meltdown, unit)).toEqual({ buried: 2, heat: 3, brittle: 2, bays: 2 });
    const spent = [
      { ability: "a", label: "Activate 2", usesLeft: 0, usable: false },
      { ability: "b", label: "Activate", usesLeft: null, usable: true },
    ];
    expect(shipArtOf(SHIP_LAYER_IDS.propagation, { ...unit, activations: spent })?.bays).toBe(1);
    expect(shipArtOf(SHIP_LAYER_IDS.propagation, { ...unit, health: 12 })?.heat).toBe(0);
    expect(shipArtOf(SHIP_LAYER_IDS.propagation, { ...unit, health: 0 })?.heat).toBe(6);
    expect(shipArtOf(SHIP_DRONE_IDS.corvette, unit)).toBeNull();
    expect(shipArtOf("core-001", unit)).toBeNull();
  });

  it("R3336000 CardArt reports the decks it draws, and a ship gives the art its own memo", () => {
    for (const [layer, buried, decks] of [["shield", 2, 3], ["shield", 1, 2], ["propagation", 1, 2], ["meltdown", 0, 1]] as const) {
      const card = LAYER_CARDS[layer];
      const { container } = render(<CardArt defId={card.id} radiant={false} tags={card.tags} type={card.type} name={card.name} shape="oval" ship={{ buried }} />);
      expect(container.querySelector<HTMLElement>(".cf-art")?.dataset.artDecks, `${layer} ${buried}`).toBe(String(decks));
      cleanup();
    }
    const printed = LAYER_CARDS.shield;
    const { container } = render(<CardArt defId={printed.id} radiant={false} tags={printed.tags} type={printed.type} name={printed.name} shape="portrait" />);
    const art = container.querySelector<HTMLElement>(".cf-art");
    expect(art?.dataset.artDecks).toBe("3");
    expect(art?.style.backgroundImage).toContain(stinkyscapeArtUri(printed.id, printed.name, printed.tags, printed.type, false));
    expect(stinkyscapeArtUri(printed.id, printed.name, printed.tags, printed.type, false, null, printed.id, null, "whole", { buried: 1 })).not.toBe(
      stinkyscapeArtUri(printed.id, printed.name, printed.tags, printed.type, false),
    );
    const cannon = byName("Cannon v3");
    cleanup();
    const other = render(<CardArt defId={cannon.id} radiant={false} tags={cannon.tags} type={cannon.type} name={cannon.name} shape="portrait" ship={{ buried: 2 }} />);
    expect(other.container.querySelector<HTMLElement>(".cf-art")?.dataset.artDecks).toBeUndefined();
  });
});

describe("R503: a shipped Stinkyscape card keeps the picture v0.5.0 shipped it with", () => {
  // The hash of `stinkyscapeSvg`'s output on each face, recorded as the set shipped (v0.5.0, #1884),
  // as `core-art-v0.1.json` records Core's: a change to the set's art fails here, entry by entry.
  // #2521 recorded S #176 Preemptive Strike and S #177 Cloaking Module as they ship, and S #137 again:
  // renamed Starfleet Commander, a Human, it draws from its new name. #3142 recorded S #45 Drone Commander
  // (a Human officer, no longer a hive ship) and S #150.1 Stinkyscape Golem (no longer a fighter hull)
  // again (R3142002, R2529000). #3333 recorded the fifty Starship Pieces again, each its module in its bay (R3333001).
  // #3336 recorded the three Drone Battleship layers again: each is a deck of one ship now (R3336000).
  const recorded = STINKYSCAPE_V050 as unknown as Readonly<Record<string, readonly [number, number]>>;

  it("R503 the fixture covers every Stinkyscape entry", () => {
    expect(Object.keys(recorded).sort()).toEqual(SET.map((card) => card.id).sort());
  });

  it("R503 every Stinkyscape entry draws exactly the picture it shipped with, on both faces", () => {
    for (const card of SET) {
      const [base, radiant] = recorded[card.id] ?? [];
      expect(hashId(svgOf(card, false)), card.id).toBe(base);
      expect(hashId(svgOf(card, true)), card.id).toBe(radiant);
    }
  });
});
