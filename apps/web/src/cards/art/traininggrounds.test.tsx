// R2202000: the Training Grounds set's own art (issue #2202, traininggrounds.ts). No other set's card
// reaches it, so every picture R503 pinned stays as it was.

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { CardArt } from "./CardArt.tsx";
import { proceduralArtUri, trainingGroundsArtUri } from "./svg.ts";
import { compositionFor, themeFor } from "./themes.ts";
import { isTrainingGroundsArt, SNAPSHOT_ART, snapshotArtOf, trainingGroundsSvg, yardSubject, type YardFamily } from "./traininggrounds.ts";

afterEach(cleanup);

const SET: readonly CardDef[] = Object.values(CATALOG).filter((card) => isTrainingGroundsArt(card.id));
const OTHERS: readonly CardDef[] = Object.values(CATALOG).filter((card) => !card.id.startsWith("traininggrounds-"));

function byName(name: string): CardDef {
  const found = SET.find((card) => card.name === name);
  if (found === undefined) throw new Error(`the set has no ${name}`);
  return found;
}

function subjectOf(card: CardDef): ReturnType<typeof yardSubject> {
  return yardSubject(card.id, card.name, card.tags, card.type);
}

function svgOf(card: CardDef, radiant: boolean): string {
  return trainingGroundsSvg(card.id, card.name, card.tags, card.type, radiant);
}

function core(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`no ${id}`);
  return found;
}

describe("R2202000: the Training Grounds set draws its own families", () => {
  it("R2202000 the set is all there, and no other set's card is routed to its art", () => {
    expect(SET.length).toBeGreaterThanOrEqual(41);
    expect(SET.every((card) => card.set === "Training Grounds")).toBe(true);
    expect(OTHERS.some((card) => isTrainingGroundsArt(card.id))).toBe(false);
  });

  it("R2202000 the brief's motifs: dummies, ogres, knives, totems, Felinors, runes, posters and the Golem", () => {
    const expected: readonly (readonly [string, YardFamily, string])[] = [
      ["Training Dummy", "dummy", "dummy"],
      ["Boulder Fist Ogre", "ogre", "boulder"],
      ["Clobberer", "ogre", "club"],
      ["Ravager", "ogre", "ravager"],
      ["Goliath Situation", "ogre", "goliath"],
      ["Fan of Knives", "knife", "fan"],
      ["Knife Juggler", "knife", "juggler"],
      ["Flame Tongue Totem", "totem", "flame"],
      ["Flaming Felinor", "felinor", "flaming"],
      ["Right-Hand Felinor", "felinor", "paw"],
      ["Quadrupling Felinors", "felinor", "four"],
      ["Guerilla Felinor", "felinor", "guerilla"],
      ["Explosive Runes", "rune", "runes"],
      ["Explosive Trap", "rune", "plate"],
      ["Recruitment Posters", "poster", "recruit"],
      ["Extra Rations", "poster", "rations"],
      ["Level Up!", "poster", "level"],
      ["Corpse Recycler", "poster", "recycle"],
      ["Training-Grounds Golem", "golem", "golem"],
      ["Call to Chaos (Training-Grounds Edition)", "chaos", "die"],
      ["Chosen by the Sun", "sky", "sun"],
      ["Chosen by the Moon", "sky", "moon"],
      ["Celestial Caller", "sky", "both"],
      ["Seance", "seance", "seance"],
      ["Hopeful Jelly Bean", "bean", "bean"],
      ["Floodgates", "gate", "gate"],
      ["Spikey Steven", "recruit", "spiky"],
      ["Tutorial Master", "recruit", "master"],
      ["Trigor", "recruit", "three"],
      ["Oriental", "recruit", "east"],
      ["Occidental", "recruit", "west"],
    ];
    for (const [name, family, kind] of expected) expect(subjectOf(byName(name)), name).toEqual({ family, kind });
  });

  it("R2202000 every Unit the names leave is a recruit, every other Spell a target wearing its name's motif", () => {
    for (const card of SET) {
      const { family } = subjectOf(card);
      if (card.type === "Unit") expect(family, card.name).not.toBe("target");
      if (family === "target") expect(card.type, card.name).toBe("Spell");
    }
  });

  it("R2202000 the snapshots #41–#60 keep their Core card's picture, by TG2's table", () => {
    expect(Object.keys(SNAPSHOT_ART)).toHaveLength(20);
    expect(SNAPSHOT_ART["traininggrounds-041"]).toBe("core-002");
    expect(SNAPSHOT_ART["traininggrounds-060"]).toBe("core-058");
    for (const [snapshot, of] of Object.entries(SNAPSHOT_ART)) {
      expect(isTrainingGroundsArt(snapshot), snapshot).toBe(false);
      expect(snapshotArtOf(snapshot)).toBe(of);
      const card = core(of);
      // A Legacy card is its Core card's name after "Legacy: " and its type (R2511000).
      const landed = CATALOG[snapshot];
      if (landed !== undefined) expect([landed.name, landed.type], snapshot).toEqual([`Legacy: ${card.name}`, card.type]);
    }
    expect(snapshotArtOf("traininggrounds-001")).toBeNull();
    expect(snapshotArtOf("core-002")).toBeNull();
  });
});

describe("R2202000: the set's art is clean, deterministic and gilded on the Radiant face", () => {
  it("R2202000 every entry, both faces: one <svg viewBox=\"0 0 100 100\">, no NaN, undefined, <text, <title or external reference", () => {
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

  it("R2202000 a picture is a pure function of the card: drawn twice, it is the same, and no two entries share one", () => {
    const seen = new Set<string>();
    for (const card of SET) {
      const base = svgOf(card, false);
      expect(svgOf(card, false)).toBe(base);
      expect(seen.has(base), card.id).toBe(false);
      seen.add(base);
    }
  });

  it("R2202000 the Radiant face keeps the base geometry, gilds its paint and adds the rays", () => {
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

describe("R2202000: CardArt draws the set's art and leaves every other card's", () => {
  it("R2202000 a Training Grounds face carries its family and the set's picture, read from its name", () => {
    const card = byName("Training Dummy");
    const { container } = render(<CardArt defId={card.id} radiant={false} tags={card.tags} type={card.type} name={card.name} shape="portrait" />);
    const art = container.querySelector<HTMLElement>(".cf-art");
    expect(art?.dataset.artFamily).toBe("dummy");
    expect(art?.dataset.artMotif).toBeUndefined();
    expect(art?.style.backgroundImage).toContain(trainingGroundsArtUri(card.id, card.name, card.tags, card.type, false));
  });

  it("R2202000 R2511000 a Legacy card draws its Core card's picture, base and Radiant, under its own name and tags", () => {
    const card = core("core-002");
    const legacy = CATALOG["traininggrounds-041"];
    if (legacy === undefined) throw new Error("no traininggrounds-041");
    for (const radiant of [false, true]) {
      const snapshot = render(<CardArt defId={legacy.id} radiant={radiant} tags={legacy.tags} type={legacy.type} name={legacy.name} shape="portrait" />);
      const drawn = snapshot.container.querySelector<HTMLElement>(".cf-art");
      cleanup();
      const original = render(<CardArt defId={card.id} radiant={radiant} tags={card.tags} type={card.type} name={card.name} shape="portrait" />);
      const own = original.container.querySelector<HTMLElement>(".cf-art");
      expect(drawn?.dataset.artFamily).toBeUndefined();
      expect(drawn?.style.backgroundImage).toBe(own?.style.backgroundImage);
      cleanup();
    }
  });

  it("R2202000 a shipped card draws R503's art as before, with no family", () => {
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
