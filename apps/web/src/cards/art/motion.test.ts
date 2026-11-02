// R2715001: every full-art card's moving art. Each card of the cosmetics registry's full-art list has a
// motion of its own, whose picture moves two of its elements or more on beats of their own, by transform
// and opacity alone inside the reduced-motion gate; without one the picture is the still one. The pixels
// moving are e2e's cypress/component/animated-art.cy.tsx; this file holds the documents.

import { createElement } from "react";
import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import type { CardDef } from "@jackioh/shared";

import { FULL_ART_CARDS, FULL_ART_TOKEN_PARENT, artMotionOf } from "../fullArt.ts";
import { CardArt } from "./CardArt.tsx";
import { ART_MOVES, ART_ROLES, FULL_ART_MOTION, PARTICLE_COUNT, motionStyle, type ArtMotion } from "./motion.ts";

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
    expect(Object.keys(FULL_ART_MOTION).sort()).toEqual([...FULL_ART_CARDS].sort());
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

  it("R2715001 a listed token takes its parent's motion", () => {
    expect(FULL_ART_TOKEN_PARENT.size).toBeGreaterThan(0);
    for (const [token, parent] of FULL_ART_TOKEN_PARENT) expect(artMotionOf(token), token).toBe(FULL_ART_MOTION[parent]);
    for (const id of FULL_ART_CARDS) expect(artMotionOf(id), id).toBe(FULL_ART_MOTION[id]);
    expect(artMotionOf("core-002")).toBeNull();
  });
});
