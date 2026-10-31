// R2170000: the Stinkyscape set's own art (issue #2170, stinkyscape.ts). Each family draws one look,
// each weapon the weapon it names, both faces share their geometry, and no other set's card reaches
// any of it, so every picture R503 pinned stays as it was.

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import STINKYSCAPE_V050 from "./fixtures/stinkyscape-art-v0.5.0.json";
import { CardArt } from "./CardArt.tsx";
import { hashId } from "./hash.ts";
import { isStinkyscapeArt, STARSHIP_PIECES, stinkyscapeSvg, stinkySubject, tierOf, type StinkyFamily } from "./stinkyscape.ts";
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
    const ships = SET.filter((card) => keywordsOf(card).some((kind) => kind === "Starship Piece" || kind === "Starship"));
    for (const card of ships) expect(FLEET_FAMILIES, card.name).toContain(subjectOf(card).family);
    expect(subjectOf(byName("Dormant Starship"))).toMatchObject({ family: "fleet", kind: "dormant" });
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

describe("R503: a shipped Stinkyscape card keeps the picture v0.5.0 shipped it with", () => {
  // The hash of `stinkyscapeSvg`'s output on each face, recorded as the set shipped (v0.5.0, #1884),
  // as `core-art-v0.1.json` records Core's: a change to the set's art fails here, entry by entry.
  // #2521 recorded S #176 Preemptive Strike and S #177 Cloaking Module as they ship, and S #137 again:
  // renamed Starfleet Commander, a Human, it draws from its new name.
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
