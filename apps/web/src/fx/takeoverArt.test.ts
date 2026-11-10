// R3402000: each Core Mythic's takeover draws its own full-screen art (takeoverArt.ts), a scene of shapes among
// R3399002's drawings (`TAKEOVER_ART_DRAWINGS`): built as an svg into the takeover's art layer over the dark
// (dom.ts), painted and moved by part in fx.css from the look's tint, and shown at rest under Reduce Motion
// (TakeoverStill.test.tsx). A scene is shapes only: no text, no card's id or name (R202), and at most
// TAKEOVER_ART_MAX_SHAPES of them.

import { CATALOG } from "@jackioh/cards";
import { afterEach, describe, expect, it } from "vitest";

import { ART_PLANES } from "../cards/art/pictures.ts";
import { TAKEOVER_ART_MAX_SHAPES } from "./constants.ts";
import { mountDomEffect } from "./dom.ts";
import { TAKEOVER_TINTS, takeoverLookOf } from "./takeover.ts";
import { ART_ATTRS, TAKEOVER_ART_DRAWINGS, takeoverArtParts, type ArtShape } from "./takeoverArt.ts";
import type { FxDomCue, TakeoverLook } from "./types.ts";
import { readSheet } from "../test/setup.ts";

/** The four Core Mythics of #3402 and the looks of their own: #96 My Pawn, #97 Zephrys, #98 Heroic Power, #99 Craft a Card. */
const OWN: ReadonlyArray<readonly [string, TakeoverLook]> = [
  ["core-096", "pawn"],
  ["core-097", "genie"],
  ["core-098", "rotunda"],
  ["core-099", "crystalForge"],
];
/** The looks that keep their takeover as it was: #100 Ceaseless Void's and the shared one (R3405000: YileGPT's `ai` draws its parts now). */
const NONE: readonly TakeoverLook[] = ["void", "prismatic"];

/** `look`'s scene's shapes, or none when it draws no scene. */
function sceneOf(look: TakeoverLook): readonly ArtShape[] {
  const drawing = TAKEOVER_ART_DRAWINGS[look];
  return drawing !== undefined && "shapes" in drawing ? drawing.shapes : [];
}

const at = { x: 100, y: 200, width: 80, height: 120 };
const cue = (look: TakeoverLook): Extract<FxDomCue, { kind: "takeover" }> => ({
  kind: "takeover",
  stage: null,
  look,
  tint: TAKEOVER_TINTS[look],
  at: { kind: "testid", testid: "card-u1" },
  landMs: 1200,
  delayMs: 0,
  durationMs: 2400,
});

afterEach(() => {
  document.body.innerHTML = "";
});

describe("R3402000 a Core Mythic's takeover art", () => {
  it("R3402000 each Core Mythic's takeover draws its own scene", () => {
    const root = document.createElement("div");
    document.body.appendChild(root);
    const allowed = new Set<string>(ART_ATTRS);
    for (const [defId, look] of OWN) {
      expect(takeoverLookOf(defId), defId).toBe(look);
      const drawing = TAKEOVER_ART_DRAWINGS[look];
      expect(drawing !== undefined && "shapes" in drawing, look).toBe(true);
      const shapes = sceneOf(look);
      expect(drawing?.viewBox, look).toBe("0 0 100 100");
      // A scene draws no parts beside it (R3400000's), as no drawing does.
      expect(takeoverArtParts(look), look).toEqual([]);
      for (const shape of shapes) {
        for (const name of Object.keys(shape.attrs)) expect(allowed.has(name), `${look} ${shape.part}: ${name}`).toBe(true);
      }
      expect(shapes.length, look).toBeGreaterThanOrEqual(1);
      expect(shapes.length, look).toBeLessThanOrEqual(TAKEOVER_ART_MAX_SHAPES);
      const el = mountDomEffect(root, cue(look), { at })?.el;
      const svg = el?.querySelector(`.takeover-art[data-look="${look}"] > svg.takeover-art-drawing`) ?? null;
      expect(svg, look).not.toBeNull();
      if (svg === null) continue;
      expect(svg.namespaceURI).toBe("http://www.w3.org/2000/svg");
      expect(svg.getAttribute("viewBox")).toBe(drawing?.viewBox);
      expect(svg.getAttribute("preserveAspectRatio")).toBe("xMidYMid slice");
      expect(svg.getAttribute("aria-hidden")).toBe("true");
      expect(svg.children.length, look).toBe(shapes.length);
      // Shapes only, each one classed by its part and carrying no paint of its own: no text at all.
      expect(svg.querySelector("text")).toBeNull();
      expect(svg.textContent).toBe("");
      expect([...svg.children].map((child) => child.getAttribute("class"))).toEqual(shapes.map((shape) => `takeover-drawing-${shape.part}`));
      expect([...svg.children].map((child) => child.tagName)).toEqual(shapes.map((shape) => shape.tag));
      expect(svg.querySelector("[fill]")).toBeNull();
      // It names no card (R202): no card's id, nor the Mythic's own name.
      const html = svg.outerHTML;
      expect(html).not.toContain("core-");
      expect(html.toLowerCase()).not.toContain((CATALOG[defId]?.name ?? defId).toLowerCase());
      el?.remove();
    }
    // Each look's scene is its own.
    expect(new Set(OWN.map(([, look]) => JSON.stringify(sceneOf(look)))).size).toBe(OWN.length);
    for (const look of NONE) {
      expect(TAKEOVER_ART_DRAWINGS[look], look).toBeUndefined();
      expect(takeoverArtParts(look), look).toEqual([]);
    }
  });

  it("R3402000 Heroic Power's rotunda has thirteen doors, a hero in one, each door's place among them for its turn to light", () => {
    const root = document.createElement("div");
    document.body.appendChild(root);
    const svg = mountDomEffect(root, cue("rotunda"), { at })?.el.querySelector(".takeover-art-drawing");
    const doors = [...(svg?.querySelectorAll<SVGElement>(".takeover-drawing-door") ?? [])];
    expect(doors).toHaveLength(13);
    expect(doors.map((door) => door.style.getPropertyValue("--fx-art-i").trim())).toEqual(doors.map((_, k) => String(k)));
    expect(doors.every((door) => door.style.getPropertyValue("--fx-art-n").trim() === "13")).toBe(true);
    expect(svg?.querySelectorAll(".takeover-drawing-hero")).toHaveLength(1);
    expect(svg?.querySelector(".takeover-drawing-floor")?.tagName).toBe("ellipse");
  });

  it("R3402000 the takeover builds its scene in its art layer over the dark and under the sweep; a look with none builds none", () => {
    const root = document.createElement("div");
    document.body.appendChild(root);
    const rotunda = mountDomEffect(root, cue("rotunda"), { at })?.el;
    expect([...(rotunda?.children ?? [])].map((child) => child.className)).toEqual([
      "fx-takeover-dark",
      "takeover-art",
      "fx-takeover-sweep",
      "fx-takeover-rays",
      "fx-takeover-wave",
    ]);
    const svg = rotunda?.querySelector(".takeover-art > svg.takeover-art-drawing");
    expect(svg?.children.length).toBe(sceneOf("rotunda").length);
    expect(rotunda?.getAttribute("data-look")).toBe("rotunda");
    // The takeover carries its look's tint, which fx.css paints the scene's parts from.
    expect(rotunda?.style.getPropertyValue("--fx-tint-glow")).toBe(TAKEOVER_TINTS.rotunda.glow);
    expect(rotunda?.textContent).toBe("");
    expect(rotunda?.outerHTML).not.toMatch(/card-u1|core-/);
    rotunda?.remove();

    // R3345000: a set piece's full-art stage sits over the dark and under the scene, as under every look's own art.
    const staged = mountDomEffect(root, { ...cue("rotunda"), stage: { planes: ART_PLANES.map(() => "none"), name: "", face: "ink" } }, { at })?.el;
    expect([...(staged?.children ?? [])].map((child) => child.className).slice(0, 3)).toEqual(["fx-takeover-dark", "fx-takeover-stage", "takeover-art"]);
    staged?.remove();

    for (const look of NONE) {
      const plain = mountDomEffect(root, cue(look), { at })?.el;
      expect([...(plain?.children ?? [])].map((child) => child.className), look).toEqual(["fx-takeover-dark", "fx-takeover-sweep", "fx-takeover-rays", "fx-takeover-wave"]);
      expect(plain?.querySelector("svg"), look).toBeNull();
      plain?.remove();
    }
  });

  it("R3402000 fx.css paints every part a scene draws, at rest as the still shows it, and moves them only under the live takeover", () => {
    const fx = readSheet("src/fx/fx.css").replace(/\/\*[\s\S]*?\*\//g, "");
    const parts = new Set(OWN.flatMap(([, look]) => sceneOf(look).map((shape) => shape.part)));
    expect(parts.size).toBeGreaterThan(OWN.length);
    // R3399002's drawings' paths move by these names, so no scene's part takes one.
    for (const motion of ["back", "subject", "spin", "drift"]) expect(parts.has(motion), motion).toBe(false);
    for (const part of parts) expect(fx, `.takeover-drawing-${part}`).toMatch(new RegExp(`\\.takeover-drawing-${part}(?![\\w-])`));
    // Every rule that names a drawing's part and moves it is the live takeover's alone, so the still stands still.
    const rules = [...fx.matchAll(/([^{}]*\.takeover-drawing-[\w-]+[^{}]*)\{([^{}]*)\}/g)];
    expect(rules.length).toBeGreaterThan(0);
    for (const [, selector, body] of rules) {
      if (!/(?<![\w-])(animation|transition)(-[\w-]+)?\s*:/.test(body ?? "")) continue;
      // Each selector of the list (a comma inside `:is(…)` is the selector's own).
      for (const one of (selector ?? "").split(/,(?![^(]*\))/)) expect(one, one).toMatch(/\.fx-takeover(?![\w-])/);
    }
  });
});
