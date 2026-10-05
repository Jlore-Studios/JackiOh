// Glitch's face (issue #170): a blank card whose name, type, text and cost are corrupted, drawn the
// same on every client, with a blob that spills over the frame and stands still under reduced motion.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { GLITCH_DEF_ID } from "@jackioh/engine/config";

import { lookupFromDefs } from "../game/catalog.ts";
import { CardFace } from "./CardFace.tsx";
import { GLITCH_GLYPHS, GLITCH_SOURCE, GLITCH_WORDS, corrupt, isGlitch } from "./glitch.ts";
import { faceModel } from "./model.ts";

afterEach(cleanup);

const glitchCss = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "glitch.css"), "utf8");

function glitchFace(liveCost?: number): HTMLElement {
  const info = lookupFromDefs(CATALOG)(GLITCH_DEF_ID, false);
  if (info === undefined) throw new Error("no Glitch in the catalog");
  const face = faceModel({ defId: GLITCH_DEF_ID, ...(info.def === undefined ? {} : { def: info.def }), radiant: false, ...(liveCost === undefined ? {} : { liveCost }) });
  const { container } = render(<CardFace face={face} />);
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error("no .cf");
  return cf;
}

describe("Glitch's corrupted words", () => {
  it("are the same on every call: a fixed source and seed, no randomness", () => {
    expect(corrupt(GLITCH_SOURCE.name)).toBe(corrupt(GLITCH_SOURCE.name));
    expect(GLITCH_WORDS.name).toBe(corrupt(GLITCH_SOURCE.name));
    expect(GLITCH_WORDS.text).toBe(corrupt(GLITCH_SOURCE.text));
    // Pinned, so a change to the jumble is a change someone meant.
    expect(GLITCH_WORDS.name).toBe("Øl̢i̴▒ch̡");
  });

  it("never read as the card's own words, and the gem holds no digit", () => {
    expect(GLITCH_WORDS.name).not.toBe(GLITCH_SOURCE.name);
    expect(GLITCH_WORDS.type).not.toBe(GLITCH_SOURCE.type);
    expect(GLITCH_WORDS.text).not.toBe(GLITCH_SOURCE.text);
    expect(GLITCH_WORDS.cost).not.toMatch(/\d/);
    for (const char of GLITCH_WORDS.cost) expect(GLITCH_GLYPHS).toContain(char);
    // No brace, so no corrupted text reads as a `{key}` placeholder (B3.4).
    expect(Object.values(GLITCH_WORDS).join("")).not.toMatch(/[{}]/);
  });

  it("are what every lookup answers for Glitch: name, text and the def it carries", () => {
    const info = lookupFromDefs(CATALOG)(GLITCH_DEF_ID, false);
    expect(info?.name).toBe(GLITCH_WORDS.name);
    expect(info?.text).toBe(GLITCH_WORDS.text);
    expect(info?.def?.name).toBe(GLITCH_WORDS.name);
    expect(info?.def?.base.text).toBe(GLITCH_WORDS.text);
    expect(lookupFromDefs(CATALOG)(GLITCH_DEF_ID, true)?.text).toBe(GLITCH_WORDS.text);
    // Any other card is untouched.
    expect(lookupFromDefs(CATALOG)("core-001", false)?.name).toBe(CATALOG["core-001"]?.name);
  });

  it("are Glitch's alone: the hidden sentinel and every other id are not Glitch", () => {
    expect(isGlitch(GLITCH_DEF_ID)).toBe(true);
    expect(isGlitch("hidden")).toBe(false);
    expect(isGlitch("classic-018")).toBe(false);
    expect(isGlitch(undefined)).toBe(false);
  });
});

describe("Glitch's face", () => {
  it("is blank: corrupted name, type, text and gem, no picture, set mark or tags", () => {
    const cf = glitchFace(0);
    expect(cf).toHaveAttribute("data-glitch", "true");
    expect(cf.querySelector(".card-name")?.textContent).toBe(GLITCH_WORDS.name);
    expect(cf.querySelector(".card-type")?.textContent).toBe(GLITCH_WORDS.type);
    expect(cf.querySelector(".cf-text-base")?.textContent).toBe(GLITCH_WORDS.text);
    expect(cf.querySelector(".cost-gem")?.textContent).toBe(GLITCH_WORDS.cost);
    expect(cf.textContent).not.toContain("Glitch");
    expect(cf.textContent).not.toContain("Spell");
    expect(cf.textContent).not.toContain("Token");
    expect(cf.querySelector(".cf-set")).toBeNull();
    expect(cf.querySelector(".cf-tags")).toBeNull();
    expect(cf.querySelector(".cf-art-frame .cf-glitch-void")).not.toBeNull();
    expect(cf.querySelector(".cf-art-frame img")).toBeNull();
  });

  it("wears its blob outside the clipped .cf-scale, so it can overflow the frame, hidden from assistive tech", () => {
    const cf = glitchFace();
    const blob = cf.querySelector(".cf-glitch");
    expect(blob).not.toBeNull();
    expect(blob?.parentElement).toBe(cf);
    expect(blob?.closest(".cf-scale")).toBeNull();
    expect(blob).toHaveAttribute("aria-hidden", "true");
    expect(blob?.textContent).toBe("");
    expect(glitchCss).toMatch(/\.cf-glitch \{[^}]*inset: -/);
  });

  it("stands still under reduced motion: the setting's attribute and the OS preference both stop the blob", () => {
    expect(glitchCss).toMatch(/@media \(prefers-reduced-motion: reduce\) \{[^@]*\.cf-glitch[^@]*animation: none/);
    expect(glitchCss).toMatch(/:root\[data-reduce-motion="true"\] \.cf-glitch[^{]*\{\s*animation: none !important;/);
  });

  it("any other card keeps its own face", () => {
    const def = CATALOG["core-001"];
    if (def === undefined) throw new Error("no core-001");
    const { container } = render(<CardFace face={faceModel({ defId: def.id, def, radiant: false })} />);
    expect(container.querySelector(".cf")).not.toHaveAttribute("data-glitch");
    expect(container.querySelector(".cf-glitch")).toBeNull();
    expect(container.querySelector(".card-name")?.textContent).toBe(def.name);
  });
});
