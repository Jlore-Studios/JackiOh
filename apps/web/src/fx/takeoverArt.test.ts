// R3402000: each Core Mythic's takeover draws its own full-screen art (takeoverArt.ts): built as an svg into
// the takeover's layers over the dark (dom.ts), painted by part in fx.css, and shown at rest under Reduce
// Motion (TakeoverStill.test.tsx, board.css). A scene is shapes only: no text, no card's id or name (R202),
// and at most TAKEOVER_ART_MAX_SHAPES of them.

import { CATALOG } from "@jackioh/cards";
import { afterEach, describe, expect, it } from "vitest";

import { TAKEOVER_ART_MAX_SHAPES } from "./constants.ts";
import { mountDomEffect } from "./dom.ts";
import { TAKEOVER_TINTS, takeoverLookOf } from "./takeover.ts";
import { ART_ATTRS, TAKEOVER_ART, TAKEOVER_ART_ASPECT, TAKEOVER_ART_VIEWBOX, buildTakeoverArt } from "./takeoverArt.ts";
import type { FxDomCue, TakeoverLook } from "./types.ts";
import { readSheet } from "../test/setup.ts";

/** The four Core Mythics of #3402 and the looks of their own: #96 My Pawn, #97 Zephrys, #98 Heroic Power, #99 Craft a Card. */
const OWN: ReadonlyArray<readonly [string, TakeoverLook]> = [
  ["core-096", "pawn"],
  ["core-097", "genie"],
  ["core-098", "rotunda"],
  ["core-099", "forge"],
];
/** The looks that keep their takeover as it was: #100 Ceaseless Void's, YileGPT's and the shared one. */
const NONE: readonly TakeoverLook[] = ["void", "ai", "prismatic"];

afterEach(() => {
  document.body.innerHTML = "";
});

describe("R3402000 a Core Mythic's takeover art", () => {
  it("R3402000 each Core Mythic's takeover draws its own art", () => {
    const allowed = new Set<string>(ART_ATTRS);
    for (const [defId, look] of OWN) {
      expect(takeoverLookOf(defId), defId).toBe(look);
      const shapes = TAKEOVER_ART[look] ?? [];
      for (const shape of shapes) {
        for (const name of Object.keys(shape.attrs)) expect(allowed.has(name), `${look} ${shape.part}: ${name}`).toBe(true);
      }
      const svg = buildTakeoverArt(document, look);
      expect(svg, look).not.toBeNull();
      if (svg === null) continue;
      expect(svg.namespaceURI).toBe("http://www.w3.org/2000/svg");
      expect(svg.getAttribute("viewBox")).toBe(TAKEOVER_ART_VIEWBOX);
      expect(svg.getAttribute("preserveAspectRatio")).toBe(TAKEOVER_ART_ASPECT);
      expect(svg.getAttribute("aria-hidden")).toBe("true");
      expect(svg.children.length, look).toBe(shapes.length);
      expect(svg.children.length, look).toBeGreaterThanOrEqual(1);
      expect(svg.children.length, look).toBeLessThanOrEqual(TAKEOVER_ART_MAX_SHAPES);
      // Shapes only, each one classed by its part: no text at all.
      expect(svg.querySelector("text")).toBeNull();
      expect(svg.textContent).toBe("");
      expect([...svg.children].map((child) => child.getAttribute("class"))).toEqual(shapes.map((shape) => `fx-takeover-art-${shape.part}`));
      // It names no card (R202): no card's id, nor the Mythic's own name.
      const html = svg.outerHTML;
      expect(html).not.toContain("core-");
      expect(html.toLowerCase()).not.toContain((CATALOG[defId]?.name ?? defId).toLowerCase());
    }
    // Each look's scene is its own.
    expect(new Set(OWN.map(([, look]) => JSON.stringify(TAKEOVER_ART[look]))).size).toBe(OWN.length);
    for (const look of NONE) {
      expect(TAKEOVER_ART[look], look).toBeUndefined();
      expect(buildTakeoverArt(document, look), look).toBeNull();
    }
  });

  it("R3402000 Heroic Power's rotunda has thirteen doors, a hero in one, each door's place among them for its turn to light", () => {
    const svg = buildTakeoverArt(document, "rotunda");
    const doors = [...(svg?.querySelectorAll<SVGElement>(".fx-takeover-art-door") ?? [])];
    expect(doors).toHaveLength(13);
    expect(doors.map((door) => door.style.getPropertyValue("--fx-art-i").trim())).toEqual(doors.map((_, k) => String(k)));
    expect(doors.every((door) => door.style.getPropertyValue("--fx-art-n").trim() === "13")).toBe(true);
    expect(svg?.querySelectorAll(".fx-takeover-art-hero")).toHaveLength(1);
    expect(svg?.querySelector(".fx-takeover-art-floor")?.tagName).toBe("ellipse");
  });

  it("R3402000 the takeover builds its art over the dark and under the sweep; a look with none builds none", () => {
    const root = document.createElement("div");
    document.body.appendChild(root);
    const at = { x: 100, y: 200, width: 80, height: 120 };
    const cue = (look: TakeoverLook): FxDomCue => ({
      kind: "takeover",
      look,
      tint: TAKEOVER_TINTS[look],
      at: { kind: "testid", testid: "card-u1" },
      landMs: 1200,
      delayMs: 0,
      durationMs: 2400,
    });
    const rotunda = mountDomEffect(root, cue("rotunda"), { at })?.el;
    expect([...(rotunda?.children ?? [])].map((child) => child.className)).toEqual([
      "fx-takeover-dark",
      "fx-takeover-art",
      "fx-takeover-sweep",
      "fx-takeover-rays",
      "fx-takeover-wave",
    ]);
    const svg = rotunda?.querySelector(".fx-takeover-art > svg");
    expect(svg?.children.length).toBe(TAKEOVER_ART.rotunda?.length);
    expect(rotunda?.getAttribute("data-look")).toBe("rotunda");
    expect(rotunda?.style.getPropertyValue("--fx-tint-glow")).toBe(TAKEOVER_TINTS.rotunda.glow);
    expect(rotunda?.textContent).toBe("");
    expect(rotunda?.outerHTML).not.toMatch(/card-u1|core-/);
    rotunda?.remove();

    for (const look of NONE) {
      const plain = mountDomEffect(root, cue(look), { at })?.el;
      expect([...(plain?.children ?? [])].map((child) => child.className), look).toEqual(["fx-takeover-dark", "fx-takeover-sweep", "fx-takeover-rays", "fx-takeover-wave"]);
      expect(plain?.querySelector("svg"), look).toBeNull();
      plain?.remove();
    }
  });

  it("R3402000 fx.css and board.css paint every part a scene draws, and only fx.css moves them", () => {
    const fx = readSheet("src/fx/fx.css").replace(/\/\*[\s\S]*?\*\//g, "");
    const board = readSheet("src/game/board.css").replace(/\/\*[\s\S]*?\*\//g, "");
    const parts = new Set(OWN.flatMap(([, look]) => (TAKEOVER_ART[look] ?? []).map((shape) => shape.part)));
    expect(parts.size).toBeGreaterThan(OWN.length);
    for (const part of parts) {
      expect(fx, `.fx-takeover-art-${part}`).toMatch(new RegExp(`\\.fx-takeover-art-${part}(?![\\w-])`));
      expect(board, `.takeover-still-art-${part}`).toMatch(new RegExp(`\\.takeover-still-art-${part}(?![\\w-])`));
    }
    // The still holds still: no rule of board.css's that names its art animates it.
    const stillRules = [...board.matchAll(/([^{}]*\.takeover-still-art[^{}]*)\{([^{}]*)\}/g)];
    expect(stillRules.length).toBeGreaterThan(0);
    for (const [, selector, body] of stillRules) expect(body, selector).not.toMatch(/(?<![\w-])(animation|transition)(-[\w-]+)?\s*:/);
  });
});
