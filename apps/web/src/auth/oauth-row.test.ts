// R3217000: Google's and Roblox's sign-in buttons are one size because ONE rule in auth.css sizes
// them, and that size is the one Google Identity Services is drawn at. jsdom lays nothing out, so this
// reads the sheet; `e2e/cypress/component/oauth-row.cy.tsx` measures the boxes.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { describe, expect, it } from "vitest";

import { GIS_BUTTON, OAUTH_BUTTON_HEIGHT_PX, OAUTH_BUTTON_WIDTH_PX } from "./googleIdentity.ts";

const CSS = readFileSync(join(dirname(fileURLToPath(import.meta.url)), "auth.css"), "utf8").replace(
  /\/\*[\s\S]*?\*\//g,
  " ",
);

type Block = { selector: string; declarations: Map<string, string> };

/** Every `selector { declarations }` pair; the sheet's at-rules nest only rules these tests do not name. */
function blocks(): Block[] {
  return [...CSS.matchAll(/([^{}]+)\{([^{}]*)\}/g)].map((match) => {
    const declarations = new Map<string, string>();
    for (const line of (match[2] ?? "").split(";")) {
      const colon = line.indexOf(":");
      if (colon > 0) declarations.set(line.slice(0, colon).trim(), line.slice(colon + 1).trim());
    }
    return { selector: (match[1] ?? "").trim(), declarations };
  });
}

/** What sets a button's size and the look of its text, as opposed to its colors. */
const SIZING = [
  "display",
  "flex",
  "width",
  "height",
  "min-height",
  "padding",
  "gap",
  "border",
  "border-radius",
  "font-size",
  "font-weight",
  "font-family",
] as const;

describe("R3217000 one size for Google's and Roblox's buttons", () => {
  it("R3217000 auth.css repeats the constants GIS is drawn at", () => {
    const screen = blocks().find((block) => block.selector === ".auth-screen");
    expect(screen?.declarations.get("--auth-oauth-width")).toBe(`${String(OAUTH_BUTTON_WIDTH_PX)}px`);
    expect(screen?.declarations.get("--auth-oauth-height")).toBe(`${String(OAUTH_BUTTON_HEIGHT_PX)}px`);
    expect(GIS_BUTTON.width).toBe(OAUTH_BUTTON_WIDTH_PX);
    expect(GIS_BUTTON.size).toBe("large");
  });

  it("R3217000 one rule sets both buttons' size", () => {
    const all = blocks();
    const branded = all.filter(
      (block) => /\.auth-oauth-branded(?![\w-])/.test(block.selector) && !block.selector.includes(":disabled"),
    );
    expect(branded).toHaveLength(1);
    for (const property of SIZING) {
      expect(branded[0]?.declarations.has(property), `the shared rule sets ${property}`).toBe(true);
    }

    for (const brand of ["auth-oauth-google", "auth-oauth-roblox"]) {
      const own = all.filter((block) => block.selector.includes(brand));
      expect(own.length, `${brand} has a rule of its colors`).toBeGreaterThan(0);
      for (const block of own) {
        for (const property of SIZING) {
          expect(block.declarations.has(property), `${brand}'s rule leaves ${property} to the shared rule`).toBe(false);
        }
      }
    }
  });

  it("R3217000 neither brand's rules set a focus ring or move the button", () => {
    for (const block of blocks().filter((candidate) => /auth-oauth-(google|roblox)/.test(candidate.selector))) {
      expect(block.declarations.has("outline"), "the focus ring is tavern.css's").toBe(false);
      expect(block.declarations.has("transform"), "no state moves the button").toBe(false);
      expect(block.declarations.has("margin"), "no state shifts the layout").toBe(false);
    }
  });

  it("R3217000 the slot GIS draws in is as wide as its button", () => {
    const slot = blocks().find((block) => block.selector === ".auth-screen .auth-oauth .auth-google");
    expect(slot?.declarations.get("width")).toBe("var(--auth-oauth-width)");
  });
});
