// R2202000: the Training Grounds set's own art (issue #2202, traininggrounds.ts). Each family draws
// one look, both faces share their geometry, the Legacy cards draw their Core card's subject in the
// yard with a touch of their own (R2724000), and no other set's card reaches any of it, so every
// picture R503 pinned stays as it was. R3393000, R3393001: every card of the set now draws its scene
// recipe first, so the yard painter and `legacySvg` tested here draw only a card with none.

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { EMBLEMS } from "./emblems.ts";
import { CardArt } from "./CardArt.tsx";
import ART_BEFORE_2724 from "./fixtures/art-before-2724.json";
import { hashId } from "./hash.ts";
import { motifFor } from "./motifs.ts";
import { artSpec } from "./procedural.ts";
import { sceneRecipeOf } from "./scene.ts";
import { legacyArtUri, proceduralArtUri, trainingGroundsArtUri } from "./svg.ts";
import { compositionFor, compositionOf, themeFor } from "./themes.ts";
import {
  isTrainingGroundsArt,
  LEGACY_PAINT,
  LEGACY_PREFIX,
  LEGACY_TOUCHES,
  legacySvg,
  SNAPSHOT_ART,
  snapshotArtOf,
  TOUCH_DRAWINGS,
  trainingGroundsSvg,
  yardSubject,
  type YardFamily,
} from "./traininggrounds.ts";

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
      ["Easterner", "recruit", "east"],
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

  it("R2511000 R2724000 a Legacy card draws its own yard picture, base and Radiant, never its Core card's", () => {
    const card = core("core-002");
    const legacy = CATALOG["traininggrounds-041"];
    if (legacy === undefined) throw new Error("no traininggrounds-041");
    for (const radiant of [false, true]) {
      const snapshot = render(<CardArt defId={legacy.id} radiant={radiant} tags={legacy.tags} type={legacy.type} name={legacy.name} shape="portrait" />);
      const drawn = snapshot.container.querySelector<HTMLElement>(".cf-art");
      cleanup();
      const original = render(<CardArt defId={card.id} radiant={radiant} tags={card.tags} type={card.type} name={card.name} shape="portrait" />);
      const own = original.container.querySelector<HTMLElement>(".cf-art");
      expect(drawn?.dataset.artFamily).toBe("legacy");
      expect(drawn?.style.backgroundImage).toContain(legacyArtUri(legacy.id, card.id, card.name, legacy.tags, legacy.type, radiant));
      expect(drawn?.style.backgroundImage).not.toBe(own?.style.backgroundImage);
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

const LEGACY = Object.keys(SNAPSHOT_ART).map(core);

/** A Legacy card's own face beside the Core picture it freezes, on both faces. */
function legacyOf(card: CardDef, radiant: boolean): { svg: string; spec: ReturnType<typeof artSpec>; uri: string; coreUri: string } {
  const coreId = snapshotArtOf(card.id) ?? "";
  const name = card.name.replace(LEGACY_PREFIX, "");
  const theme = themeFor(card.tags, card.type);
  const motif = motifFor(name, theme);
  return {
    svg: legacySvg(card.id, coreId, name, card.tags, card.type, radiant),
    // R3331000: the Core card's subject as the Legacy card froze it, before any scene of its own.
    spec: artSpec(coreId, theme, compositionFor(card.type), radiant, motif, null),
    uri: legacyArtUri(card.id, coreId, name, card.tags, card.type, radiant),
    coreUri: proceduralArtUri(coreId, theme, compositionFor(card.type), radiant, motif),
  };
}

describe("R2724000: each Legacy card redraws its Core card's subject in the yard", () => {
  it("R2724000 every Legacy face, base and Radiant, differs from its Core card's and is clean", () => {
    for (const card of LEGACY) {
      for (const radiant of [false, true]) {
        const { svg, uri, coreUri } = legacyOf(card, radiant);
        expect(uri, card.id).not.toBe(coreUri);
        expect(svg.startsWith('<svg viewBox="0 0 100 100"'), card.id).toBe(true);
        expect(svg.match(/<svg/g)).toHaveLength(1);
        expect(svg, card.id).not.toMatch(/NaN|undefined|Infinity|<text|<title|href|<image/);
        expect(svg.replace('xmlns="http://www.w3.org/2000/svg"', ""), card.id).not.toMatch(/https?:/);
      }
    }
  });

  it("R2724000 each still draws its Core card's subject: its composition's shapes, its emblem and its motif's glyphs", () => {
    for (const card of LEGACY) {
      for (const radiant of [false, true]) {
        const { svg, spec } = legacyOf(card, radiant);
        const [from, to] = spec.subjectRidges;
        if (spec.composition !== "landscape") expect(to, card.id).toBeGreaterThan(from);
        for (const ridge of spec.ridges.slice(from, to)) expect(svg, card.id).toContain(`d="${ridge.d}"`);
        expect(svg, card.id).toContain(`d="${EMBLEMS[spec.emblem.glyph].d}"`);
        for (const placed of spec.motifGlyphs) expect(svg, card.id).toContain(`d="${EMBLEMS[placed.glyph].d}"`);
        if (spec.accent !== null) expect(svg, card.id).toContain(`d="${EMBLEMS[spec.accent.glyph].d}"`);
      }
    }
  });

  it("R2724000 each draws the yard's layers: its sky, its palisade and its sand floor", () => {
    for (const card of LEGACY) {
      for (const radiant of [false, true]) {
        const { svg } = legacyOf(card, radiant);
        expect(svg, card.id).toContain('fill="url(#sk)"');
        expect(svg, card.id).toContain('fill="url(#gr)"');
        expect(svg.match(/<polygon/g)?.length ?? 0, card.id).toBeGreaterThanOrEqual(13);
      }
    }
  });

  it("R2724000 every Legacy card has a yard touch of its own, drawn on its face, and no two share one", () => {
    expect(Object.keys(LEGACY_TOUCHES).sort()).toEqual(Object.keys(SNAPSHOT_ART).sort());
    const touches = Object.values(LEGACY_TOUCHES);
    expect(new Set(touches).size).toBe(touches.length);
    const drawings = touches.map((touch) => TOUCH_DRAWINGS[touch](LEGACY_PAINT));
    expect(new Set(drawings).size).toBe(drawings.length);
    for (const card of LEGACY) {
      const touch = LEGACY_TOUCHES[card.id];
      if (touch === undefined) throw new Error(`no touch for ${card.id}`);
      expect(legacyOf(card, false).svg, card.id).toContain(TOUCH_DRAWINGS[touch](LEGACY_PAINT));
    }
  });

  it("R2724000 the Radiant face keeps the base geometry, gilds its paint and adds the rays", () => {
    const shapes = (svg: string): string[] => [...svg.matchAll(/(?:points|d)="([^"]*)"/g)].map((match) => match[1] ?? "");
    for (const card of LEGACY) {
      const base = legacyOf(card, false).svg;
      expect(legacyOf(card, false).svg).toBe(base);
      const radiant = legacyOf(card, true).svg;
      expect(radiant, card.id).not.toBe(base);
      const baseShapes = shapes(base);
      const radiantShapes = shapes(radiant);
      for (const d of baseShapes) expect(radiantShapes, card.id).toContain(d);
      expect(radiantShapes.length).toBeGreaterThan(baseShapes.length);
    }
  });
});

describe("R2724000: no other card's picture moves", () => {
  // R3331002: a record of the pictures before #2724, which pins every card with no scene recipe; a card
  // a set's picture pass gives a recipe draws its scene instead.
  // Each picture as CardArt draws it, its card's own composition included: #3338 recorded Core #100 Ceaseless
  // Void again, now a hole in the shape of a giant (R3338001).
  const recorded = ART_BEFORE_2724 as unknown as Readonly<Record<string, readonly [number, number]>>;
  const KEPT = Object.values(CATALOG).filter((card) => (card.set === "Core" || card.set === "Training Grounds") && snapshotArtOf(card.id) === null);
  const SCENES = KEPT.filter((card) => sceneRecipeOf(card.id) !== null);
  function pictureOf(card: CardDef, radiant: boolean): string {
    if (isTrainingGroundsArt(card.id)) return trainingGroundsArtUri(card.id, card.name, card.tags, card.type, radiant);
    const theme = themeFor(card.tags, card.type);
    return proceduralArtUri(card.id, theme, compositionOf(card.id, card.type), radiant, motifFor(card.name, theme));
  }

  it("R2724000 the fixture covers every Core card and every Training Grounds card but the Legacy ones", () => {
    expect(Object.keys(recorded).sort()).toEqual(KEPT.map((card) => card.id).sort());
  });

  it("R2724000 R3331002 each of them with no scene recipe draws exactly the picture it drew before, on both faces", () => {
    let unchanged = 0;
    for (const card of KEPT) {
      if (sceneRecipeOf(card.id) !== null) continue;
      const pair = recorded[card.id] ?? (() => { throw new Error(`no fixture for ${card.id}`); })();
      expect(hashId(pictureOf(card, false)), card.id).toBe(pair[0]);
      expect(hashId(pictureOf(card, true)), card.id).toBe(pair[1]);
      unchanged += 1;
    }
    expect(unchanged).toBe(KEPT.length - SCENES.length);
  });

  it("R3331002 R3393000 the cards with a scene recipe no longer draw the picture art-before-2724 records", () => {
    // R3393000: the three Core proofs, and every Training Grounds card of the set's own, the Golem token's included.
    const own = Object.values(CATALOG).filter((card) => isTrainingGroundsArt(card.id)).map((card) => card.id);
    expect(own).toHaveLength(41);
    expect(SCENES.map((card) => card.id).sort()).toEqual(["core-002", "core-098", "core-100", ...own].sort());
    for (const card of SCENES) {
      const pair = recorded[card.id] ?? (() => { throw new Error(`no fixture for ${card.id}`); })();
      expect(hashId(pictureOf(card, false)), card.id).not.toBe(pair[0]);
      expect(hashId(pictureOf(card, true)), card.id).not.toBe(pair[1]);
    }
  });
});
