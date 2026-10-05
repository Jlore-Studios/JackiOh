// The Glitch token on the client (SPEC §7, R662): a blank face whose art, a glitching blob, breaks out
// of the frame and never moves under Reduce motion, and a name, rules text and flavour line drawn
// corrupted, the same jumble on every client, wherever they show.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { GLITCH_DEF_ID } from "@jackioh/engine/config";
import { fillParams, type CardDef } from "@jackioh/shared";

import { lookupFromDefs } from "../game/catalog.ts";
import { CardFace } from "./CardFace.tsx";
import { flushFits } from "./fit.ts";
import { CARD_FLAVOUR, flavourFor } from "./flavour.ts";
import { corruptedText, displayName, isGlitch } from "./glitch.ts";
import { faceModel } from "./model.ts";

afterEach(() => {
  cleanup();
});

const HERE = dirname(fileURLToPath(import.meta.url));
/** vitest stubs CSS imports, so the stylesheets are read as text. */
const glitchCss = readFileSync(join(HERE, "glitch.css"), "utf8");
const boardCss = readFileSync(join(HERE, "../game/board.css"), "utf8");

function glitchDef(): CardDef {
  const def = CATALOG[GLITCH_DEF_ID];
  if (def === undefined) throw new Error("the catalog has no Glitch");
  return def;
}

function renderGlitch(radiant: boolean, layout: "full" | "compact" = "full"): HTMLElement {
  const { container } = render(<CardFace face={faceModel({ defId: GLITCH_DEF_ID, def: glitchDef(), radiant })} layout={layout} />);
  flushFits();
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error("no .cf for Glitch");
  return cf;
}

/** Every word of `text` that is three letters or more, which a corrupted string must not spell out. */
function words(text: string): string[] {
  return text.split(/[^A-Za-z]+/).filter((word) => word.length >= 3);
}

describe("R662 Glitch's corrupted text", () => {
  it("R662 isGlitch names Glitch alone", () => {
    expect(isGlitch(GLITCH_DEF_ID)).toBe(true);
    expect(Object.keys(CATALOG).filter(isGlitch)).toEqual([GLITCH_DEF_ID]);
    expect(isGlitch("hidden")).toBe(false);
    expect(isGlitch(undefined)).toBe(false);
  });

  it("R662 a corrupted text is the same jumble every time, keeps its spaces and spells out none of its words", () => {
    const def = glitchDef();
    for (const text of [def.name, fillParams(def, "base"), fillParams(def, "radiant")]) {
      const jumble = corruptedText(text);
      expect(corruptedText(text)).toBe(jumble);
      expect(jumble).not.toBe(text);
      expect(jumble.split(" ").length).toBe(text.split(" ").length);
      for (const word of words(text)) expect(jumble.toLowerCase(), word).not.toContain(word.toLowerCase());
    }
    expect(corruptedText("")).toBe("");
    expect(corruptedText(def.name)).not.toBe(corruptedText(fillParams(def, "base")));
  });

  it("R662 the face model, the game's card lookup and the flavour line all give the jumble, never the words", () => {
    const def = glitchDef();
    for (const radiant of [false, true]) {
      const text = fillParams(def, radiant ? "radiant" : "base");
      const face = faceModel({ defId: GLITCH_DEF_ID, def, radiant });
      expect(face.name).toBe(corruptedText(def.name));
      expect(face.text).toEqual({ full: corruptedText(text), marks: [] });
      const info = lookupFromDefs(CATALOG)(GLITCH_DEF_ID, radiant);
      expect(info?.name).toBe(corruptedText(def.name));
      expect(info?.text).toBe(corruptedText(text));
    }
    const line = CARD_FLAVOUR[GLITCH_DEF_ID]?.flavour ?? "";
    expect(line.length).toBeGreaterThan(0);
    expect(flavourFor(GLITCH_DEF_ID)?.flavour).toBe(corruptedText(line));
    expect(displayName(def)).toBe(corruptedText(def.name));
    // Every other card reads as it is.
    expect(faceModel({ defId: "classic-018", def: CATALOG["classic-018"], radiant: false }).name).toBe("Glitch in the System");
    expect(lookupFromDefs(CATALOG)("classic-018", false)?.name).toBe("Glitch in the System");
    expect(displayName(CATALOG["classic-018"] ?? def)).toBe("Glitch in the System");
  });

  it("R662 a copier that takes Glitch's text (C #57 Echo) prints it corrupted, and names what it copies corrupted", () => {
    const echo = CATALOG["classic-057"];
    if (echo === undefined) throw new Error("the catalog has no Echo");
    const def = glitchDef();
    const face = faceModel({ defId: echo.id, def: echo, radiant: false, inPlay: { copies: { def, radiant: false } } });
    expect(face.name).toBe("Echo");
    expect(face.text.full).toBe(corruptedText(fillParams(def, "base")));
    expect(face.copying?.name).toBe(corruptedText(def.name));
    for (const word of words(fillParams(def, "base"))) expect(face.text.full, word).not.toContain(word);
  });
});

describe("R662 Glitch's face", () => {
  it("R662 is blank: no gem, name, type, rules, tags or set mark, on either face or layout", () => {
    for (const radiant of [false, true]) {
      for (const layout of ["full", "compact"] as const) {
        const cf = renderGlitch(radiant, layout);
        expect(cf).toHaveAttribute("data-glitch", "true");
        expect(cf).toHaveAttribute("data-layout", layout);
        expect(cf.textContent).toBe("");
        // With no words on it, its corrupted name is what a screen reader hears.
        expect(cf).toHaveAttribute("role", "img");
        expect(cf).toHaveAttribute("aria-label", corruptedText(glitchDef().name));
        expect(cf.querySelector(".cost-gem, .card-name, .card-type, .card-text, .cf-tags, .cf-set, .cf-gem, .cf-art")).toBeNull();
        expect(cf.getAttribute("data-radiant-face")).toBe(radiant ? "true" : null);
        cleanup();
      }
    }
  });

  it("R662 draws its blob outside the clipping plate, with no foil to clip it, and only spans (B12)", () => {
    const cf = renderGlitch(false);
    const blob = cf.querySelector(".cf-glitch-blob");
    expect(blob).not.toBeNull();
    expect(blob?.parentElement).toBe(cf);
    expect(blob).toHaveAttribute("aria-hidden", "true");
    expect(cf.querySelector(".cf-scale .cf-glitch-blob")).toBeNull();
    expect(cf.hasAttribute("data-foil")).toBe(false);
    expect([...cf.querySelectorAll("*")].every((el) => el.tagName.toLowerCase() === "span")).toBe(true);
    expect([...cf.querySelectorAll(".cf-glitch-layer")].map((el) => el.getAttribute("data-layer"))).toEqual(["body", "red", "cyan", "slice"]);
  });

  it("R662 breaks out of the frame: neither the face nor the board's card clips it", () => {
    expect(glitchCss).toMatch(/\.cf\[data-glitch\] \{[^}]*overflow: visible/);
    expect(boardCss).toMatch(/\.card\[data-glitch\] \{[^}]*overflow: visible/);
    // The blob reaches past the face's edges.
    expect(glitchCss).toMatch(/\.cf-glitch-blob \{[^}]*inset: [^;]*-\d+%/);
  });

  it("R662 moves by transform and opacity alone, and not at all under Reduce motion", () => {
    const keyframes = [...glitchCss.matchAll(/@keyframes [\w-]+ \{([\s\S]*?)\n\}/g)].map((match) => match[1] ?? "");
    expect(keyframes.length).toBeGreaterThan(0);
    for (const body of keyframes) {
      const properties = [...body.matchAll(/([a-z-]+):/g)].map((match) => match[1]);
      expect(properties.every((property) => property === "transform" || property === "opacity"), body).toBe(true);
    }
    expect(glitchCss).toMatch(/@media \(prefers-reduced-motion: reduce\) \{\s*\.cf\[data-glitch\] \.cf-glitch-layer \{\s*animation: none;/);
    expect(glitchCss).toMatch(/:root\[data-reduce-motion="true"\] \.cf\[data-glitch\] \.cf-glitch-layer \{\s*animation: none;/);
  });
});
