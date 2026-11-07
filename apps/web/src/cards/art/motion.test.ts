// R2715001: every full-art card's moving art. Each card of the cosmetics registry's full-art list has a
// motion of its own, whose picture moves two of its elements or more on beats of their own, by transform
// and opacity alone inside the reduced-motion gate; without one the picture is the still one. The pixels
// moving are e2e's cypress/component/animated-art.cy.tsx; this file holds the documents. R3015002 (#3015): a card has
// one motion, which every animated and signed level of every variant it is offered moves its picture by (R2949000's
// Signature motion is gone); R3015000: every card but a Legacy one is offered the Default's animated level.

import { createElement } from "react";
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { COSMETIC_OFFERS } from "../../wire/generated/cosmetics.ts";
import { FULL_ART_CARDS, FULL_ART_TOKEN_PARENT, artMotionOf } from "../fullArt.ts";
import { ART_WEAR_LEVELS } from "../constants.ts";
import { VERSION_ALT_FULL_ART_SIGNED_ID, VERSION_FULL_ART_SIGNED_ID, VERSION_SKETCH_FULL_ART_SIGNED_ID } from "../versions.ts";
import { CardArt } from "./CardArt.tsx";
import {
  ART_MOVES,
  ART_ROLES,
  FULL_ART_MOTION,
  GOLD_PARTICLES,
  PARTICLE_COUNT,
  artMotionFor,
  motionStyle,
  particleMarkup,
  particleStyle,
  type ArtMotion,
} from "./motion.ts";
import { particleLayerUri } from "./svg.ts";
import { crackPaths } from "./wear.ts";

afterEach(cleanup);

const DATA_PREFIX = "data:image/svg+xml,";

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`the catalog has no ${id}`);
  return found;
}

/** The SVG document `id`'s art draws as a full-art face's would, moving as `motion` says. */
function artDocument(id: string, motion: ArtMotion | null): { text: string; doc: Document } {
  const card = def(id);
  const { container } = render(
    createElement(CardArt, { defId: id, radiant: false, tags: card.tags, type: card.type, shape: "full", name: card.name, motion }),
  );
  const art = container.querySelector<HTMLElement>(".cf-art");
  const background = art?.style.backgroundImage ?? "";
  const uri = /^url\("(.*)"\)$/u.exec(background)?.[1] ?? "";
  expect(uri.startsWith(DATA_PREFIX), id).toBe(true);
  const text = decodeURIComponent(uri.slice(DATA_PREFIX.length));
  cleanup();
  return { text, doc: new DOMParser().parseFromString(text, "image/svg+xml") };
}

/** The properties a keyframes text sets. */
function properties(keyframes: string): string[] {
  return [...keyframes.matchAll(/([a-z-]+)\s*:/gu)].map((match) => match[1] ?? "");
}

describe("R2715001 the full-art cards' moving art", () => {
  it("R2715001 every full-art card in the registry moves at least two of its art's own elements", () => {
    expect(FULL_ART_CARDS.size).toBe(21);
    for (const id of FULL_ART_CARDS) {
      const motion = FULL_ART_MOTION[id];
      expect(motion, `${id} has no motion in FULL_ART_MOTION`).toBeDefined();
      if (motion === undefined) continue;
      const { text, doc } = artDocument(id, motion);
      expect(doc.querySelector("parsererror"), id).toBeNull();
      const beats = new Set<string>();
      for (const role of ART_ROLES) {
        const step = motion[role];
        if (step === undefined) continue;
        expect(doc.querySelector(`.art-m-${role}`), `${id}: ${role}`).not.toBeNull();
        beats.add(`${step.move}|${step.seconds}`);
      }
      expect(doc.querySelectorAll(".art-m-particle").length, id).toBe(PARTICLE_COUNT);
      beats.add(`${motion.particles.beat.move}|${motion.particles.beat.seconds}`);
      // Two elements or more, each on a beat of its own, beyond the whole picture panning.
      expect(beats.size, id).toBeGreaterThanOrEqual(2);
      const style = doc.querySelector("style")?.textContent ?? "";
      for (const move of new Set([...ART_ROLES.flatMap((role) => motion[role]?.move ?? []), motion.particles.beat.move])) {
        expect(style, `${id}: ${move}`).toContain(`@keyframes art-${move}{`);
      }
      // No wrapper carries a transform of its own, which the animation would replace.
      for (const tagged of doc.querySelectorAll('[class^="art-m-"]')) expect(tagged.hasAttribute("transform"), id).toBe(false);
      expect(text.startsWith("<svg"), id).toBe(true);
    }
  });

  it("R2715001 no two full-art cards share a motion", () => {
    const seen = new Map<string, string>();
    for (const [id, motion] of Object.entries(FULL_ART_MOTION)) {
      const key = JSON.stringify(motion);
      expect(seen.get(key), `${id} moves as ${seen.get(key) ?? ""} does`).toBeUndefined();
      seen.set(key, id);
    }
    // R3340000: every full-art card keeps an entry, beside the recipes of the Legendaries and Mythics.
    for (const id of FULL_ART_CARDS) expect(FULL_ART_MOTION[id], id).toBeDefined();
  });

  it("R2715001 the keyframes use transform and opacity alone inside the reduced-motion gate", () => {
    for (const [move, spec] of Object.entries(ART_MOVES)) {
      const set = properties(spec.keyframes);
      expect(set.length, move).toBeGreaterThan(0);
      for (const property of set) expect(["transform", "opacity"], `${move}: ${property}`).toContain(property);
    }
    for (const [id, motion] of Object.entries(FULL_ART_MOTION)) {
      const style = motionStyle(motion, "50px 50px");
      expect(style.startsWith("<style>@media (prefers-reduced-motion: no-preference){"), id).toBe(true);
      expect(style.endsWith("}}</style>"), id).toBe(true);
      // Inside the gate, each rule moves by its animation alone: a box, an origin and the loop.
      const inner = style.slice(style.indexOf("{") + 1, style.indexOf("@keyframes"));
      for (const property of properties(inner)) expect(["transform-box", "transform-origin", "animation"], `${id}: ${property}`).toContain(property);
    }
    expect(motionStyle(null, "50px 50px")).toBe("");
  });

  it("R2715001 without a motion the art has no style and no tagged layer", () => {
    for (const id of FULL_ART_CARDS) {
      const { text } = artDocument(id, null);
      expect(text, id).not.toContain("<style");
      expect(text, id).not.toContain("art-m-");
    }
  });

  it("R2715001 R3015000 a listed token takes its parent's motion", () => {
    expect(FULL_ART_TOKEN_PARENT.size).toBeGreaterThan(0);
    for (const [token, parent] of FULL_ART_TOKEN_PARENT) expect(artMotionOf(token), token).toBe(FULL_ART_MOTION[parent]);
    for (const id of FULL_ART_CARDS) expect(artMotionOf(id), id).toBe(FULL_ART_MOTION[id]);
    // R3015000: any other card offered the Default's animated level, every one but a Legacy card, borrows one; a Legacy
    // card's picture never moves (R2724000).
    expect(artMotionOf("core-002")).toBe(artMotionFor("core-002"));
    expect(artMotionOf("traininggrounds-044")).toBeNull();
  });
});

describe("R3015002 a signed level's motion", () => {
  it("R3015002 a signed level moves as every animated level of its card does: the card's one motion, none of the Signature look's own", () => {
    // Every card offered a signed level moves by its own full-art motion or a borrowed one, which draws no gold particle.
    const signed = new Set([VERSION_FULL_ART_SIGNED_ID, VERSION_ALT_FULL_ART_SIGNED_ID, VERSION_SKETCH_FULL_ART_SIGNED_ID].flatMap((id) => COSMETIC_OFFERS[id] ?? []));
    expect(signed.size).toBeGreaterThan(0);
    for (const id of signed) {
      const motion = artMotionOf(id);
      expect(motion, id).not.toBeNull();
      expect(motion, id).toBe(FULL_ART_MOTION[id] ?? artMotionFor(id));
      if (FULL_ART_MOTION[id] === undefined && motion !== null) expect(GOLD_PARTICLES.has(motion.particles.kind), id).toBe(false);
    }
    // Its picture tags that motion's subject and back where it moves them (every painter draws both) and draws its
    // particles, its keyframes inside the gate.
    for (const id of ["core-022", "stinkyscape-001", "traininggrounds-016"]) {
      const motion = artMotionOf(id);
      expect(motion, id).not.toBeNull();
      if (motion === null) continue;
      const { doc } = artDocument(id, motion);
      expect(doc.querySelector("parsererror"), id).toBeNull();
      expect(doc.querySelector('[class^="art-m-"]'), id).not.toBeNull();
      for (const role of ["subject", "back"] as const) if (motion[role] !== undefined) expect(doc.querySelector(`.art-m-${role}`), `${id}: ${role}`).not.toBeNull();
      expect(doc.querySelectorAll(".art-m-particle").length, id).toBe(PARTICLE_COUNT);
      expect(motionStyle(motion, "50px 50px").startsWith("<style>@media (prefers-reduced-motion: no-preference){"), id).toBe(true);
    }
  });
});

describe("R3340000 art that lives in play", () => {
  /** The Legendaries and Mythics of the catalog with no recipe of their own: their art would borrow a motion by id. */
  const borrowing = Object.values(CATALOG)
    .filter((card) => (card.rarity === "Legendary" || card.rarity === "Mythic") && FULL_ART_MOTION[card.id] === undefined)
    .map((card) => card.id)
    .sort();

  it("R3340000 no Legendary or Mythic borrows a motion by hash: the list is empty and may only stay so", () => {
    const BORROWING: readonly string[] = [];
    expect(borrowing).toEqual([...BORROWING]);
    const rare = Object.values(CATALOG).filter((card) => card.rarity === "Legendary" || card.rarity === "Mythic");
    expect(rare.length).toBeGreaterThan(100);
    for (const card of rare) expect(artMotionOf(card.id), card.id).toBe(FULL_ART_MOTION[card.id]);
  });

  it("R3340000 every recipe's JSON is its own, and a rare recipe moves three roles on beats of their own", () => {
    const seen = new Map<string, string>();
    for (const [id, motion] of Object.entries(FULL_ART_MOTION)) {
      const key = JSON.stringify(motion);
      expect(seen.get(key), `${id} moves as ${seen.get(key) ?? ""} does`).toBeUndefined();
      seen.set(key, id);
      expect(motion.detached, id).toBeUndefined();
    }
    for (const card of Object.values(CATALOG)) {
      if ((card.rarity !== "Legendary" && card.rarity !== "Mythic") || FULL_ART_CARDS.has(card.id)) continue;
      const motion = FULL_ART_MOTION[card.id];
      expect(motion, card.id).toBeDefined();
      if (motion === undefined) continue;
      expect(ART_ROLES.filter((role) => motion[role] !== undefined).length, card.id).toBeGreaterThanOrEqual(3);
    }
  });

  it("R3340000 R2725004 no rare recipe's particles are drawn in gold, and every one's keyframes are transform and opacity alone", () => {
    for (const card of Object.values(CATALOG)) {
      if ((card.rarity !== "Legendary" && card.rarity !== "Mythic") || FULL_ART_CARDS.has(card.id)) continue;
      const motion = FULL_ART_MOTION[card.id];
      expect(motion, card.id).toBeDefined();
      if (motion === undefined) continue;
      expect(GOLD_PARTICLES.has(motion.particles.kind), card.id).toBe(false);
      const style = motionStyle(motion, "50px 50px");
      expect(style.startsWith("<style>@media (prefers-reduced-motion: no-preference){"), card.id).toBe(true);
      const frames = style.split("@keyframes").slice(1);
      expect(frames.length, card.id).toBeGreaterThan(0);
      for (const text of frames) for (const property of properties(text)) expect(["transform", "opacity"], `${card.id}: ${property}`).toContain(property);
    }
  });

  it("R3340000 a detached motion draws no particles and no rule for them, and its particle layer draws them alone", () => {
    const motion = FULL_ART_MOTION["core-004"];
    expect(motion).toBeDefined();
    if (motion === undefined) return;
    const detached: ArtMotion = { ...motion, detached: true };
    expect(particleMarkup(motion, "core-004")).not.toBe("");
    expect(particleMarkup(detached, "core-004")).toBe("");
    expect(motionStyle(motion, "50px 50px")).toContain(".art-m-particle{");
    expect(motionStyle(detached, "50px 50px")).not.toContain(".art-m-particle");
    expect(motionStyle(detached, "50px 50px")).not.toContain(`@keyframes art-${motion.particles.beat.move}{`);
    expect(particleStyle(motion).startsWith("<style>@media (prefers-reduced-motion: no-preference){.art-m-particle{")).toBe(true);
    const uri = particleLayerUri(motion, "core-004");
    expect(uri.startsWith(DATA_PREFIX)).toBe(true);
    const doc = new DOMParser().parseFromString(decodeURIComponent(uri.slice(DATA_PREFIX.length)), "image/svg+xml");
    expect(doc.querySelector("parsererror")).toBeNull();
    expect(doc.querySelectorAll(".art-m-particle").length).toBe(PARTICLE_COUNT);
    expect(doc.querySelector("style")?.textContent ?? "").toContain(`@keyframes art-${motion.particles.beat.move}{`);
    expect(particleLayerUri(motion, "core-004")).toBe(uri);
  });

  it("R3340000 a painter's detached backdrop and its subject layer split the picture's layers between them", () => {
    for (const id of ["core-004", "stinkyscape-006", "traininggrounds-016"]) {
      const motion = artMotionOf(id);
      expect(motion, id).not.toBeNull();
      if (motion === null) continue;
      const card = def(id);
      const draw = (part: "backdrop" | "subject", m: ArtMotion): Document => {
        const { container } = render(
          createElement(CardArt, { defId: id, radiant: false, tags: card.tags, type: card.type, shape: "full", name: card.name, motion: m, part }),
        );
        const uri = /^url\("(.*)"\)$/u.exec(container.querySelector<HTMLElement>(".cf-art")?.style.backgroundImage ?? "")?.[1] ?? "";
        cleanup();
        return new DOMParser().parseFromString(decodeURIComponent(uri.slice(DATA_PREFIX.length)), "image/svg+xml");
      };
      const backdrop = draw("backdrop", { ...motion, detached: true });
      const subject = draw("subject", motion);
      expect(backdrop.querySelectorAll(".art-m-particle").length, `${id}: backdrop`).toBe(0);
      expect(backdrop.querySelector(".art-m-subject"), `${id}: backdrop's subject`).toBeNull();
      expect(subject.querySelector(".art-m-subject"), `${id}: subject`).not.toBeNull();
      expect(subject.querySelectorAll(".art-m-particle").length, `${id}: subject's particles`).toBe(0);
    }
  });

  it("R3340000 a crack at one level begins the same crack at the next, and the levels are ART_WEAR_LEVELS", () => {
    for (let level = 1; level < ART_WEAR_LEVELS; level += 1) {
      const lower = crackPaths("core-004", level);
      const higher = crackPaths("core-004", level + 1);
      expect(higher.length).toBe(level + 1);
      lower.forEach((path, k) => expect(higher[k]?.startsWith(path)).toBe(true));
    }
  });
});
