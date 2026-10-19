// R1295000: the card cosmetics a player's own cards wear: the store, how a face wears them, the cap
// on moving foils, the settings rows, and the stylesheet's two promises — no border in the Radiant
// gold, and every foil still under reduced motion. Vitest stubs CSS imports, so the sheets are read
// as text (foil.test.ts does the same for cards.css).

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { CardCosmeticsControls } from "../settings/controls.tsx";
import { unit } from "../test/fixtures.ts";
import { CardFace } from "./CardFace.tsx";
import {
  COSMETIC_FOIL_ANIMATED_MAX,
  RADIANT_GOLD_HUE_MAX_DEG,
  RADIANT_GOLD_HUE_MIN_DEG,
  RADIANT_GOLD_LIGHTNESS_MAX,
  RADIANT_GOLD_LIGHTNESS_MIN,
  RADIANT_GOLD_SATURATION_MIN,
} from "./constants.ts";
import {
  COSMETICS_SETTINGS_KEY,
  DEFAULT_CARD_COSMETICS,
  cosmeticsOf,
  normalizeCosmetics,
  readCosmetics,
  resetCosmeticsForTests,
  resolveCosmetic,
  writeCosmetics,
  type CosmeticKind,
} from "./cosmetics.ts";
import { MinionFace } from "./MinionFace.tsx";
import { faceModel, type FaceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";

const HERE = dirname(fileURLToPath(import.meta.url));

/** A stylesheet as text, comments stripped, so a comment naming a selector proves nothing. */
function sheet(name: string): string {
  return readFileSync(join(HERE, name), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
}

const cosmeticsCss = sheet("cosmetics.css");
const cardsCss = sheet("cards.css");

/** The balanced `{ … }` block of `css` opened at `open`, inner text without the braces. */
function blockFrom(css: string, open: number): string {
  let depth = 0;
  for (let i = open; i < css.length; i += 1) {
    if (css[i] === "{") depth += 1;
    else if (css[i] === "}") {
      depth -= 1;
      if (depth === 0) return css.slice(open + 1, i);
    }
  }
  throw new Error("unbalanced braces");
}

/** The inner text of the first rule of `css` whose selector `pattern` matches. */
function ruleBody(css: string, pattern: RegExp): string {
  const match = pattern.exec(css);
  expect(match, `no rule matches ${String(pattern)}`).not.toBeNull();
  return blockFrom(css, css.indexOf("{", match?.index ?? 0));
}

/** Every rule of `css` (at any depth) as its selector and body, outermost blocks included. */
function rules(css: string): { selector: string; body: string; at: number }[] {
  const out: { selector: string; body: string; at: number }[] = [];
  let start = 0;
  for (let i = 0; i < css.length; i += 1) {
    if (css[i] === "{") {
      out.push({ selector: css.slice(start, i).trim(), body: blockFrom(css, i), at: i });
      start = i + 1;
    } else if (css[i] === "}" || css[i] === ";") start = i + 1;
  }
  return out;
}

/** A hex colour's hue in degrees and saturation and lightness from 0 to 1. */
function hsl(hex: string): { h: number; s: number; l: number } {
  const digits = hex.length <= 4 ? [...hex].map((digit) => digit + digit).join("") : hex;
  const [r, g, b] = [0, 2, 4].map((at) => parseInt(digits.slice(at, at + 2), 16) / 255) as [number, number, number];
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const l = (max + min) / 2;
  const d = max - min;
  if (d === 0) return { h: 0, s: 0, l };
  const s = d / (1 - Math.abs(2 * l - 1));
  const sector = max === r ? ((g - b) / d + 6) % 6 : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
  return { h: sector * 60, s, l };
}

function inRadiantGold(hex: string): boolean {
  const { h, s, l } = hsl(hex);
  return (
    h >= RADIANT_GOLD_HUE_MIN_DEG &&
    h <= RADIANT_GOLD_HUE_MAX_DEG &&
    s >= RADIANT_GOLD_SATURATION_MIN &&
    l >= RADIANT_GOLD_LIGHTNESS_MIN &&
    l <= RADIANT_GOLD_LIGHTNESS_MAX
  );
}

/** Every `#…` colour in `text`, without its `#`; a length no colour has fails the test. */
function hexes(text: string): string[] {
  const found = [...text.matchAll(/#([0-9a-fA-F]+)\b/g)].map((match) => match[1] ?? "");
  for (const hex of found) expect([3, 4, 6, 8], `#${hex}`).toContain(hex.length);
  return found;
}

function face(defId: string, radiant = false): FaceModel {
  return faceModel({ defId, def: CATALOG[defId], radiant });
}

/** Every pick other than its kind's default: the registry's first non-default entry. */
function firstItem(kind: CosmeticKind): string {
  const item = cosmeticsOf(kind).find((cosmetic) => !cosmetic.default);
  if (item === undefined) throw new Error(`no ${kind} item`);
  return item.id;
}

const PICKS = { foil: firstItem("foil"), finish: firstItem("finish"), border: firstItem("border") } as const;

const ATTRIBUTES = ["data-cosmetic-foil", "data-cosmetic-motion", "data-finish", "data-frame"] as const;

function root(container: HTMLElement): HTMLElement {
  const found = container.querySelector<HTMLElement>(".cf");
  if (found === null) throw new Error("no face");
  return found;
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
});

afterEach(() => {
  cleanup();
  resetCosmeticsForTests();
  localStorage.clear();
});

describe("R1295000 the cosmetics store and how a face wears it", () => {
  it("R1295000 an unknown id resolves to its kind's default, never an error", () => {
    expect(resolveCosmetic("foil", "foil-aurora")).toBe(DEFAULT_CARD_COSMETICS.foil);
    // An id of another kind is no pick of this one.
    expect(resolveCosmetic("foil", PICKS.border)).toBe(DEFAULT_CARD_COSMETICS.foil);
    expect(resolveCosmetic("border", 7)).toBe(DEFAULT_CARD_COSMETICS.border);
    expect(resolveCosmetic("finish", undefined)).toBe(DEFAULT_CARD_COSMETICS.finish);
    expect(resolveCosmetic("finish", PICKS.finish)).toBe(PICKS.finish);
    // Each kind has its one default, first in its list.
    for (const kind of ["foil", "finish", "border"] as const) {
      expect(cosmeticsOf(kind)[0]?.id).toBe(DEFAULT_CARD_COSMETICS[kind]);
      expect(cosmeticsOf(kind).filter((cosmetic) => cosmetic.default)).toHaveLength(1);
    }
  });

  it("R1295000 the store keeps an id it does not know and the face draws the default", () => {
    writeCosmetics({ foil: "foil-aurora", border: PICKS.border });
    expect(readCosmetics()).toEqual({ foil: "foil-aurora", finish: DEFAULT_CARD_COSMETICS.finish, border: PICKS.border });
    expect(JSON.parse(localStorage.getItem(COSMETICS_SETTINGS_KEY) ?? "{}")).toMatchObject({ foil: "foil-aurora" });
    // Read back from storage on a fresh page, it is still there.
    resetCosmeticsForTests();
    expect(readCosmetics().foil).toBe("foil-aurora");

    const { container } = render(<CardFace face={face("core-004")} worn />);
    expect(root(container).getAttribute("data-cosmetic-foil")).toBeNull();
    expect(root(container).getAttribute("data-frame")).toBe(PICKS.border);

    // What the settings cannot carry, or is no text, is the default.
    expect(normalizeCosmetics({ foil: "", finish: 3, border: "b".repeat(41) })).toEqual(DEFAULT_CARD_COSMETICS);
    expect(normalizeCosmetics("junk")).toEqual(DEFAULT_CARD_COSMETICS);
  });

  it("R1295000 a face that is not worn renders as before, whatever the picks", () => {
    const before = render(
      <>
        <CardFace face={face("core-004")} />
        <MinionFace face={face("core-004")} unit={unit("p1")} />
      </>,
    ).container.innerHTML;
    cleanup();

    writeCosmetics(PICKS);
    const { container } = render(
      <>
        <CardFace face={face("core-004")} />
        <MinionFace face={face("core-004")} unit={unit("p1")} />
      </>,
    );
    const faces = [...container.querySelectorAll<HTMLElement>(".cf")];
    expect(faces).toHaveLength(2);
    for (const element of faces) {
      for (const attribute of ATTRIBUTES) expect(element.getAttribute(attribute), attribute).toBeNull();
    }
    // Instance ids aside, the markup is the markup it was.
    const strip = (html: string): string => html.replace(/u\d+/g, "u");
    expect(strip(container.innerHTML)).toBe(strip(before));
  });

  it("R1295000 the viewer's own face wears each pick on its own axis, never data-foil", () => {
    writeCosmetics(PICKS);
    const { container } = render(
      <>
        <CardFace face={face("core-004")} worn />
        <MinionFace face={face("core-004")} unit={unit("p1")} worn />
      </>,
    );
    const faces = [...container.querySelectorAll<HTMLElement>(".cf")];
    expect(faces).toHaveLength(2);
    for (const element of faces) {
      expect(element.getAttribute("data-cosmetic-foil")).toBe(PICKS.foil);
      expect(element.getAttribute("data-cosmetic-motion")).toBe("animated");
      expect(element.getAttribute("data-finish")).toBe(PICKS.finish);
      expect(element.getAttribute("data-frame")).toBe(PICKS.border);
      // The rarity foil is untouched: a Common card has none.
      expect(element.getAttribute("data-foil")).toBe("none");
    }
  });

  it("R1295000 a Mythic face wears no cosmetic foil; a Radiant face no foil, finish or border", () => {
    writeCosmetics(PICKS);
    const { container } = render(
      <>
        <CardFace face={face("core-096")} worn />
        <CardFace face={face("core-004", true)} worn />
        <MinionFace face={face("core-004", true)} unit={unit("p1", { radiant: true })} worn />
      </>,
    );
    const [mythic, radiant, radiantMinion] = [...container.querySelectorAll<HTMLElement>(".cf")];
    expect(mythic?.getAttribute("data-foil")).toBe("animated");
    expect(mythic?.getAttribute("data-cosmetic-foil")).toBeNull();
    expect(mythic?.getAttribute("data-finish")).toBe(PICKS.finish);
    expect(mythic?.getAttribute("data-frame")).toBe(PICKS.border);
    for (const element of [radiant, radiantMinion]) {
      expect(element?.getAttribute("data-radiant-face")).toBe("true");
      for (const attribute of ATTRIBUTES) expect(element?.getAttribute(attribute), attribute).toBeNull();
    }
  });

  it("R1295000 at most COSMETIC_FOIL_ANIMATED_MAX faces move their foil, and a freed slot passes on", () => {
    writeCosmetics({ foil: PICKS.foil });
    const count = COSMETIC_FOIL_ANIMATED_MAX + 2;
    const hand = (from: number) => (
      <>
        {Array.from({ length: count - from }, (_unused, index) => (
          <span key={from + index} data-n={from + index}>
            <CardFace face={face("core-004")} worn />
          </span>
        ))}
      </>
    );
    const motions = (container: HTMLElement): string[] =>
      [...container.querySelectorAll<HTMLElement>("[data-n] > .cf")].map((element) => element.getAttribute("data-cosmetic-motion") ?? "");

    const { container, rerender } = render(hand(0));
    expect(motions(container)).toEqual([
      ...Array<string>(COSMETIC_FOIL_ANIMATED_MAX).fill("animated"),
      "static",
      "static",
    ]);
    // Every face wears its foil, moving or not.
    expect(container.querySelectorAll(`[data-cosmetic-foil="${PICKS.foil}"]`)).toHaveLength(count);

    rerender(hand(1));
    expect(motions(container)).toEqual([...Array<string>(COSMETIC_FOIL_ANIMATED_MAX).fill("animated"), "static"]);
  });

  it("R1295000 with Animated foil off a worn foil holds still", () => {
    writeCosmetics({ foil: PICKS.foil });
    writeCardSettings({ animatedFoil: false });
    const { container } = render(<CardFace face={face("core-004")} worn />);
    expect(root(container).getAttribute("data-cosmetic-foil")).toBe(PICKS.foil);
    expect(root(container).getAttribute("data-cosmetic-motion")).toBe("static");
  });

  it("R1295000 the settings selects write the store and show the default for an unknown id", () => {
    render(<CardCosmeticsControls />);
    const foil = screen.getByTestId<HTMLSelectElement>("setting-cosmetic-foil");
    expect(foil.value).toBe(DEFAULT_CARD_COSMETICS.foil);
    expect([...foil.options].map((option) => option.value)).toEqual(cosmeticsOf("foil").map((cosmetic) => cosmetic.id));

    fireEvent.change(screen.getByTestId("setting-cosmetic-border"), { target: { value: PICKS.border } });
    expect(readCosmetics().border).toBe(PICKS.border);
    expect(screen.getByTestId<HTMLSelectElement>("setting-cosmetic-border").value).toBe(PICKS.border);

    act(() => {
      writeCosmetics({ finish: "finish-velvet" });
    });
    expect(screen.getByTestId<HTMLSelectElement>("setting-cosmetic-finish").value).toBe(DEFAULT_CARD_COSMETICS.finish);
    expect(readCosmetics().finish).toBe("finish-velvet");
  });
});

describe("R1295000 the cosmetics stylesheet", () => {
  it("R1295000 no border's declared colours fall in the Radiant gold range", () => {
    // The control: the window holds every colour the Radiant face declares.
    const radiant = hexes(ruleBody(cardsCss, /\.cf\[data-radiant-face="true"\]\s*\{/));
    expect(radiant.length).toBeGreaterThan(0);
    for (const hex of radiant) expect(inRadiantGold(hex), `Radiant's #${hex}`).toBe(true);

    for (const border of cosmeticsOf("border").filter((cosmetic) => !cosmetic.default)) {
      expect(cosmeticsCss, border.id).toContain(`.cf[data-frame="${border.id}"]`);
    }
    const framed = rules(cosmeticsCss).filter((rule) => rule.selector.includes("[data-frame"));
    expect(framed.length).toBeGreaterThan(0);
    for (const rule of framed) {
      for (const hex of hexes(rule.body)) expect(inRadiantGold(hex), `${rule.selector}'s #${hex}`).toBe(false);
      // Every colour a border declares is a hex this test can read.
      for (const way of ["rgb(", "hsl(", "color-mix(", "gold"]) expect(rule.body, rule.selector).not.toContain(way);
    }
  });

  it("R1295000 every animated foil has a still version and moves only inside the motion gate", () => {
    const gate = cosmeticsCss.indexOf("@media (prefers-reduced-motion: no-preference)");
    expect(gate).toBeGreaterThan(0);
    const gated = blockFrom(cosmeticsCss, cosmeticsCss.indexOf("{", gate));
    const gateEnd = cosmeticsCss.indexOf(gated) + gated.length;

    for (const foil of cosmeticsOf("foil").filter((cosmetic) => !cosmetic.default)) {
      const still = cosmeticsCss.indexOf(`.cf[data-cosmetic-foil="${foil.id}"] .cf-art::after {`);
      expect(still, `${foil.id}'s still rule`).toBeGreaterThanOrEqual(0);
      expect(still, `${foil.id}'s still rule comes before the gate`).toBeLessThan(gate);
      const body = blockFrom(cosmeticsCss, cosmeticsCss.indexOf("{", still));
      expect(body).toMatch(/background\s*:/);
      expect(body).not.toMatch(/animation/);

      const moving = rules(gated).find((rule) =>
        rule.selector.includes(`:root:not([data-reduce-motion="true"]) .cf[data-cosmetic-foil="${foil.id}"][data-cosmetic-motion="animated"]`),
      );
      expect(moving, `${foil.id}'s moving rule`).toBeDefined();
      expect(moving?.body).toMatch(/animation\s*:\s*cosmetic-foil-/);
    }

    // Nothing animates outside the gate.
    for (const match of cosmeticsCss.matchAll(/animation\s*:/g)) {
      expect(match.index).toBeGreaterThan(gate);
      expect(match.index).toBeLessThan(gateEnd);
    }
    // Each loop moves by transform or opacity alone, which the compositor runs on its own.
    const frames = [...cosmeticsCss.matchAll(/@keyframes\s+(cosmetic-foil-[a-z-]+)\s*\{/g)];
    expect(frames.length).toBeGreaterThan(0);
    for (const frame of frames) {
      const body = blockFrom(cosmeticsCss, (frame.index ?? 0) + frame[0].length - 1);
      const properties = [...body.matchAll(/([a-z-]+)\s*:/g)].map((match) => match[1]);
      expect(properties.length, frame[1]).toBeGreaterThan(0);
      for (const property of properties) expect(["transform", "opacity"], `${frame[1] ?? ""}: ${property ?? ""}`).toContain(property);
    }
    // Reduced motion is the absence of the gate, never a rule of its own to keep in step.
    expect(cosmeticsCss).not.toContain("prefers-reduced-motion: reduce");
  });
});
