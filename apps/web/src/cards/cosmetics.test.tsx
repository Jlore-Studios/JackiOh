// R1295000: the card cosmetics a player's own cards wear: the store, how a face wears them, the cap
// on moving foils, the Collection's account tiles (R1579000), and the stylesheet's two promises — no
// border in the Radiant gold, and every foil still under reduced motion. Vitest stubs CSS imports, so
// the sheets are read as text (foil.test.ts does the same for cards.css). R1953001: a face wears its
// card's own picks or its kinds' defaults, never the account's, and every chase moves wherever it is
// drawn.

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { CATALOG } from "@jackioh/cards";

import AccountCosmeticTiles from "../collection/AccountCosmeticTiles.tsx";
import { CosmeticLook } from "../collection/CosmeticLook.tsx";
import { collectionTestid } from "../collection/collection.ts";
import { SETTINGS_SLOTS } from "../settings/slots.ts";
import { resetSettings } from "../settings/store.ts";
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
  CARD_PICKS_SETTINGS_KEY,
  CARD_SLOT_KINDS,
  COSMETICS_SETTINGS_KEY,
  DECK_SLOT_KINDS,
  DEFAULT_CARD_COSMETICS,
  MOVING_FINISHES_AND_BORDERS,
  cosmeticsOf,
  deckCosmeticPatch,
  deckCosmeticsOf,
  isChaseCosmetic,
  normalizeCosmetics,
  pickedDeckCosmetics,
  readCardPicks,
  readCosmetics,
  resetCosmeticsForTests,
  resolveCosmetic,
  resolveDeckCosmetic,
  useCosmetics,
  wornCosmetics,
  wornDeckCosmetics,
  writeCardFullArt,
  writeCardPick,
  writeCosmetics,
  type CardSlotKind,
  type CosmeticId,
  type CosmeticKind,
} from "./cosmetics.ts";
import { FullArtOwned, OwnCopy } from "./fullArt.ts";
import { CardDetail } from "./inspect/CardDetail.tsx";
import { HoverPreview } from "./inspect/HoverPreview.tsx";
import { InspectSheet } from "./inspect/InspectSheet.tsx";
import { closeInspect } from "./inspect/store.ts";
import { INSPECT_FACE, INSPECT_FACE_BASE, INSPECT_FACE_RADIANT } from "./inspect/testids.ts";
import { MinionFace } from "./MinionFace.tsx";
import { faceModel, type FaceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import { COSMETICS } from "../wire/generated/cosmetics.ts";

const HERE = dirname(fileURLToPath(import.meta.url));

/** A stylesheet as text, comments stripped, so a comment naming a selector proves nothing. */
function sheet(name: string): string {
  return readFileSync(join(HERE, name), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
}

const cosmeticsCss = sheet("cosmetics.css");
const cardsCss = sheet("cards.css");
const tableCss = readFileSync(join(HERE, "..", "game", "table.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");
const collectionCss = readFileSync(join(HERE, "..", "collection", "collection.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

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

/** The rules of `css` inside its motion gates (the Collection's sheet has two). */
function gatedRules(css: string): { selector: string; body: string }[] {
  const gates = [...css.matchAll(/@media \(prefers-reduced-motion: no-preference\)\s*\{/g)];
  expect(gates.length).toBeGreaterThan(0);
  return gates.flatMap((gate) => rules(blockFrom(css, (gate.index ?? 0) + gate[0].length - 1)));
}

/** A hex color's hue in degrees and saturation and lightness from 0 to 1. */
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

/** Every `#…` color in `text`, without its `#`; a length no color has fails the test. */
function hexes(text: string): string[] {
  const found = [...text.matchAll(/#([0-9a-fA-F]+)\b/g)].map((match) => match[1] ?? "");
  for (const hex of found) expect([3, 4, 6, 8], `#${hex}`).toContain(hex.length);
  return found;
}

function face(defId: string, radiant = false): FaceModel {
  return faceModel({ defId, def: CATALOG[defId], radiant });
}

/** Every pick other than its kind's default: the registry's first non-default entry. */
function firstItem(kind: CosmeticKind): CosmeticId {
  const item = cosmeticsOf(kind).find((cosmetic) => !cosmetic.default);
  if (item === undefined) throw new Error(`no ${kind} item`);
  return item.id;
}

/** The registry's second entry of `kind` that is not its default. */
function secondItem(kind: CosmeticKind): CosmeticId {
  const item = cosmeticsOf(kind).filter((cosmetic) => !cosmetic.default)[1];
  if (item === undefined) throw new Error(`no second ${kind} item`);
  return item.id;
}

const PICKS = { foil: firstItem("foil"), finish: firstItem("finish"), border: firstItem("border") } as const;

/** R1953001: `defId`'s own pick of each kind `picks` names, which is how a card wears a foil, finish or border. */
function wearOn(defId: string, picks: Partial<Record<CardSlotKind, CosmeticId>>): void {
  for (const kind of CARD_SLOT_KINDS) {
    const id = picks[kind];
    if (id !== undefined) writeCardPick(defId, kind, id);
  }
}

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
    expect(resolveCosmetic("foil", "foil-unknown")).toBe(DEFAULT_CARD_COSMETICS.foil);
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
    writeCosmetics({ foil: "foil-unknown", border: PICKS.border });
    expect(readCosmetics()).toEqual({ ...DEFAULT_CARD_COSMETICS, foil: "foil-unknown", border: PICKS.border });
    expect(JSON.parse(localStorage.getItem(COSMETICS_SETTINGS_KEY) ?? "{}")).toMatchObject({ foil: "foil-unknown" });
    // Read back from storage on a fresh page, it is still there.
    resetCosmeticsForTests();
    expect(readCosmetics().foil).toBe("foil-unknown");

    // A card's own pick of an id this client does not know draws as the default, its known one as itself.
    wearOn("core-004", { foil: "foil-unknown" as CosmeticId, border: PICKS.border });
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
    wearOn("core-004", PICKS);
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
    wearOn("core-004", PICKS);
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
    wearOn("core-096", PICKS);
    wearOn("core-004", PICKS);
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
    wearOn("core-004", { foil: PICKS.foil });
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
    wearOn("core-004", { foil: PICKS.foil });
    writeCardSettings({ animatedFoil: false });
    const { container } = render(<CardFace face={face("core-004")} worn />);
    expect(root(container).getAttribute("data-cosmetic-foil")).toBe(PICKS.foil);
    expect(root(container).getAttribute("data-cosmetic-motion")).toBe("static");
  });
});

describe("R1295000 the cosmetics stylesheet", () => {
  it("R1295000 no border's declared colors fall in the Radiant gold range", () => {
    // The control: the window holds every color the Radiant face declares.
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
      // Every color a border declares is a hex this test can read.
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

describe("R1297000 the deck slots", () => {
  const TABLE = firstItem("table");
  const FIELD = firstItem("field");
  const BACK = firstItem("cardBack");
  const DECK_PICKS = deckCosmeticsOf({ tableCosmetic: TABLE, fieldCosmetic: FIELD, cardBackCosmetic: BACK });

  it("R1297000 the deck's choice beats the account's, which beats the default", () => {
    expect(DECK_PICKS).toEqual({ table: TABLE, field: FIELD, cardBack: BACK });
    const otherTable = cosmeticsOf("table").filter((cosmetic) => !cosmetic.default)[1]?.id ?? "";
    expect(otherTable).not.toBe(TABLE);
    const account = { ...DEFAULT_CARD_COSMETICS, table: otherTable, cardBack: otherTable };

    // The deck's pick wins over the account's.
    expect(resolveDeckCosmetic("table", DECK_PICKS, account)).toBe(TABLE);
    // A deck with no pick for a kind leaves it to the account's.
    const partial = deckCosmeticsOf({ tableCosmetic: TABLE });
    expect(partial).toEqual({ table: TABLE, field: null, cardBack: null });
    expect(resolveDeckCosmetic("field", partial, { ...account, field: FIELD })).toBe(FIELD);
    expect(resolveDeckCosmetic("table", partial, account)).toBe(TABLE);
    // Neither: the kind's default; a deck with no picks at all is null.
    expect(resolveDeckCosmetic("field", null, DEFAULT_CARD_COSMETICS)).toBe(DEFAULT_CARD_COSMETICS.field);
    expect(resolveDeckCosmetic("cardBack", partial, DEFAULT_CARD_COSMETICS)).toBe(DEFAULT_CARD_COSMETICS.cardBack);
    expect(deckCosmeticsOf({})).toBeNull();
    expect(deckCosmeticsOf({ tableCosmetic: null, fieldCosmetic: null })).toBeNull();

    // A side that is not the viewer's wears the defaults whatever the deck and the account say.
    expect(wornDeckCosmetics(false, DECK_PICKS, account)).toEqual({
      table: DEFAULT_CARD_COSMETICS.table,
      field: DEFAULT_CARD_COSMETICS.field,
      cardBack: DEFAULT_CARD_COSMETICS.cardBack,
    });
    expect(wornDeckCosmetics(true, DECK_PICKS, account)).toEqual({ table: TABLE, field: FIELD, cardBack: BACK });
  });

  it("R1297000 an unknown id falls back to its kind's default, from the deck or the account", () => {
    const unknown = deckCosmeticsOf({ tableCosmetic: "table-unknown", cardBackCosmetic: "card-back-gold" });
    // The deck's word is final: its unknown pick draws as the default, and the account's does not step in.
    const account = { ...DEFAULT_CARD_COSMETICS, table: TABLE, cardBack: BACK };
    expect(resolveDeckCosmetic("table", unknown, account)).toBe(DEFAULT_CARD_COSMETICS.table);
    expect(resolveDeckCosmetic("cardBack", unknown, account)).toBe(DEFAULT_CARD_COSMETICS.cardBack);
    // An unknown account id, with no deck pick, draws as the default too.
    expect(resolveDeckCosmetic("field", null, { ...DEFAULT_CARD_COSMETICS, field: "field-unknown" })).toBe(DEFAULT_CARD_COSMETICS.field);
    // An id of another kind is no id of this one.
    expect(resolveCosmetic("table", FIELD)).toBe(DEFAULT_CARD_COSMETICS.table);
    expect(resolveCosmetic("cardBack", TABLE)).toBe(DEFAULT_CARD_COSMETICS.cardBack);
  });

  it("R1297000 the store keeps the deck slots beside the card slots", () => {
    expect([...CARD_SLOT_KINDS, ...DECK_SLOT_KINDS]).toEqual(["foil", "finish", "border", "table", "field", "cardBack"]);
    expect(readCosmetics()).toEqual(DEFAULT_CARD_COSMETICS);
    expect(Object.keys(DEFAULT_CARD_COSMETICS)).toHaveLength(6);

    writeCosmetics({ table: TABLE, cardBack: BACK, border: PICKS.border });
    expect(readCosmetics()).toEqual({ ...DEFAULT_CARD_COSMETICS, table: TABLE, cardBack: BACK, border: PICKS.border });
    // An id this client does not know is kept in the deck slots too.
    writeCosmetics({ field: "field-unknown" });
    expect(readCosmetics().field).toBe("field-unknown");
    resetCosmeticsForTests();
    expect(readCosmetics().field).toBe("field-unknown");
    expect(normalizeCosmetics({ table: "", cardBack: 3, field: "f".repeat(41) })).toEqual(DEFAULT_CARD_COSMETICS);

    // A deck's picks read as the deck fields keep them: texts of a pick's length, and nothing else.
    expect(pickedDeckCosmetics({ tableCosmetic: TABLE, fieldCosmetic: null, cardBackCosmetic: "", extra: 1 })).toEqual({ tableCosmetic: TABLE });
    expect(pickedDeckCosmetics({ fieldCosmetic: "f".repeat(41) })).toEqual({});
    expect(pickedDeckCosmetics("junk")).toEqual({});
    expect(deckCosmeticPatch("table", TABLE)).toEqual({ tableCosmetic: TABLE });
    expect(deckCosmeticPatch("field", null)).toEqual({ fieldCosmetic: null });
    expect(deckCosmeticPatch("cardBack", BACK)).toEqual({ cardBackCosmetic: BACK });
  });

  it("R1297000 the collection's tiles write the account's playmat, backdrop and card back", () => {
    render(<AccountCosmeticTiles />);
    const tile = (id: string): HTMLElement => screen.getByTestId(collectionTestid.tile(id));
    for (const kind of DECK_SLOT_KINDS) {
      const row = within(screen.getByTestId(collectionTestid.tiles(kind)));
      expect(row.getAllByRole("button").map((button) => button.getAttribute("aria-label"))).toEqual(
        cosmeticsOf(kind).map((cosmetic) => cosmetic.name),
      );
      expect(tile(DEFAULT_CARD_COSMETICS[kind])).toHaveAttribute("data-equipped", "true");
    }
    for (const id of [TABLE, FIELD, BACK]) fireEvent.click(within(tile(id)).getByRole("button"));
    expect(readCosmetics()).toEqual({ ...DEFAULT_CARD_COSMETICS, table: TABLE, field: FIELD, cardBack: BACK });
    expect(tile(BACK)).toHaveAttribute("data-equipped", "true");

    // An id this client does not know shows as the default, which is what the board draws.
    act(() => {
      writeCosmetics({ table: "table-unknown" });
    });
    expect(tile(DEFAULT_CARD_COSMETICS.table)).toHaveAttribute("data-equipped", "true");
  });

  it("R1579000 no settings reset touches the equipped cosmetics", () => {
    writeCosmetics({ foil: PICKS.foil, finish: PICKS.finish, border: PICKS.border, table: TABLE, field: FIELD, cardBack: BACK });
    const equipped = { ...readCosmetics() };
    for (const slot of SETTINGS_SLOTS) slot.reset?.();
    resetSettings();
    expect(readCosmetics()).toEqual(equipped);
  });

  it("R1297000 every backdrop has a still version and moves only inside the motion gate", () => {
    const gate = tableCss.indexOf("@media (prefers-reduced-motion: no-preference)");
    expect(gate).toBeGreaterThan(0);
    const gated = blockFrom(tableCss, tableCss.indexOf("{", gate));
    const gateEnd = tableCss.indexOf(gated) + gated.length;

    for (const field of cosmeticsOf("field").filter((cosmetic) => !cosmetic.default)) {
      // The still version: a tint on the layer and a look for its particles, both before the gate.
      const tint = tableCss.indexOf(`.board-backdrop[data-backdrop="${field.id}"] {`);
      expect(tint, `${field.id}'s tint`).toBeGreaterThanOrEqual(0);
      expect(tint).toBeLessThan(gate);
      expect(blockFrom(tableCss, tableCss.indexOf("{", tint))).toMatch(/background\s*:/);
      const particle = tableCss.indexOf(`.board-backdrop[data-backdrop="${field.id}"] .board-backdrop-particle {`);
      expect(particle, `${field.id}'s particles`).toBeGreaterThanOrEqual(0);
      expect(particle).toBeLessThan(gate);
      const still = blockFrom(tableCss, tableCss.indexOf("{", particle));
      expect(still).toMatch(/background\s*:/);
      expect(still).not.toMatch(/animation/);

      const moving = rules(gated).find((rule) =>
        rule.selector.includes(`:root:not([data-reduce-motion="true"]) .board-backdrop[data-backdrop="${field.id}"] .board-backdrop-particle`),
      );
      expect(moving, `${field.id}'s moving rule`).toBeDefined();
      expect(moving?.body).toMatch(/animation\s*:\s*backdrop-/);
    }
    // The base particle sits at its own place, so the still field shows them where they would be.
    expect(ruleBody(tableCss, /\.board-backdrop-particle\s*\{/)).toMatch(/left:\s*var\(--x\)/);

    // Nothing animates outside the gate.
    for (const match of tableCss.matchAll(/animation\s*:/g)) {
      expect(match.index).toBeGreaterThan(gate);
      expect(match.index).toBeLessThan(gateEnd);
    }
    // Each loop moves by transform or opacity alone.
    const frames = [...tableCss.matchAll(/@keyframes\s+(backdrop-[a-z-]+)\s*\{/g)];
    expect(frames.length).toBe(cosmeticsOf("field").filter((cosmetic) => !cosmetic.default).length);
    for (const frame of frames) {
      const body = blockFrom(tableCss, (frame.index ?? 0) + frame[0].length - 1);
      const properties = [...body.matchAll(/([a-z-]+)\s*:/g)].map((match) => match[1]);
      expect(properties.length, frame[1]).toBeGreaterThan(0);
      for (const property of properties) expect(["transform", "opacity"], `${frame[1] ?? ""}: ${property ?? ""}`).toContain(property);
    }
    expect(tableCss).not.toContain("prefers-reduced-motion: reduce");
  });

  it("R1297000 the backdrop never takes input", () => {
    const layer = ruleBody(tableCss, /\.board-backdrop\s*\{/);
    expect(layer).toMatch(/pointer-events:\s*none/);
    expect(layer).toMatch(/z-index:\s*-1\b/);
    // The board makes no stacking context for it: that would put the cards mid-animation (z-index
    // 40 to 50) under the effects layer (38), which fx/fx.css documents above them (R200).
    expect(ruleBody(tableCss, /\.board\[data-backdrop\]\s*\{/)).not.toMatch(/isolation|z-index/);
    // So the board's opaque surface would paint over a negative layer: the board hands it to its
    // own `::before`, a negative layer ahead of the backdrop in tree order (the pixels are
    // e2e/cypress/component/deck-cosmetics.cy.tsx's).
    expect(ruleBody(tableCss, /\.board\[data-backdrop\]\s*\{/)).toMatch(/background:\s*none/);
    const surface = ruleBody(tableCss, /\.board\[data-backdrop\]::before\s*\{/);
    expect(surface).toMatch(/z-index:\s*-1\b/);
    expect(surface).toMatch(/background:\s*var\(--board-surface\)/);
    expect(surface).toMatch(/pointer-events:\s*none/);
    // The mat's surface is no hit target either, and each look names every table the registry lists.
    expect(ruleBody(tableCss, /\.field\[data-table\]::before\s*\{/)).toMatch(/pointer-events:\s*none/);
    for (const table of cosmeticsOf("table").filter((cosmetic) => !cosmetic.default)) {
      expect(tableCss, table.id).toContain(`.field[data-table="${table.id}"]::before`);
    }
    // A card back is a look, never text: no content, and a look for each non-default back.
    for (const back of cosmeticsOf("cardBack").filter((cosmetic) => !cosmetic.default)) {
      expect(cosmeticsCss, back.id).toContain(`[data-card-back="${back.id}"]`);
    }
    const backs = cosmeticsCss.indexOf('.cf-back[data-card-back="');
    expect(backs).toBeGreaterThan(0);
    expect(cosmeticsCss.slice(backs)).not.toMatch(/content\s*:/);
  });
});

describe("R1655000 the picks of each card", () => {
  const ANCHOR = { left: 600, top: 200, width: 100, height: 140 };
  /** Two cards, the one with picks of its own and the one without. */
  const OWN = "core-004";
  const OTHER = "core-005";
  /** A second border, apart from the account's `PICKS.border`. */
  const CARD_BORDER = secondItem("border");

  afterEach(() => {
    closeInspect();
  });

  it("R1655000 the picks are stored per card on the device, in a store of their own", () => {
    expect(CARD_BORDER).not.toBe(PICKS.border);
    expect(readCardPicks(OWN)).toEqual({});
    writeCardPick(OWN, "foil", PICKS.foil);
    writeCardPick(OWN, "border", CARD_BORDER);
    writeCardFullArt(OWN, true);
    expect(readCardPicks(OWN)).toEqual({ foil: PICKS.foil, border: CARD_BORDER, fullArt: true });
    expect(readCardPicks(OTHER)).toEqual({});
    expect(JSON.parse(localStorage.getItem(CARD_PICKS_SETTINGS_KEY) ?? "null")).toEqual({
      [OWN]: { foil: PICKS.foil, border: CARD_BORDER, fullArt: true },
    });
    // The account's store, which syncs as the `cosmetics` group, is not touched.
    expect(localStorage.getItem(COSMETICS_SETTINGS_KEY)).toBeNull();
    expect(readCosmetics()).toEqual(DEFAULT_CARD_COSMETICS);

    // A new page reads them back; null clears a kind to its default, and a card with none leaves the store.
    resetCosmeticsForTests();
    expect(readCardPicks(OWN)).toEqual({ foil: PICKS.foil, border: CARD_BORDER, fullArt: true });
    writeCardPick(OWN, "foil", null);
    writeCardPick(OWN, "border", null);
    expect(readCardPicks(OWN)).toEqual({ fullArt: true });
    writeCardFullArt(OWN, null);
    expect(readCardPicks(OWN)).toEqual({});
    expect(localStorage.getItem(CARD_PICKS_SETTINGS_KEY)).toBe("{}");
    // Off is a pick of its own, which beats a Full-art cards setting that is on.
    writeCardFullArt(OWN, false);
    expect(readCardPicks(OWN)).toEqual({ fullArt: false });
  });

  it("R1655000 the store keeps an id it does not know, which reads as its kind's default, and drops what is no pick", () => {
    localStorage.setItem(
      CARD_PICKS_SETTINGS_KEY,
      `{"${OWN}":{"foil":"foil-unknown","finish":7,"border":"${"b".repeat(41)}","fullArt":"yes"},"${OTHER}":"junk","__proto__":{"foil":"${PICKS.foil}"}}`,
    );
    expect(readCardPicks(OWN)).toEqual({ foil: DEFAULT_CARD_COSMETICS.foil });
    expect(readCardPicks(OTHER)).toEqual({});
    expect(({} as Record<string, unknown>)["foil"]).toBeUndefined();

    // A write of another kind never overwrites the id a newer client wrote.
    writeCardPick(OWN, "border", CARD_BORDER);
    expect(JSON.parse(localStorage.getItem(CARD_PICKS_SETTINGS_KEY) ?? "{}")[OWN]).toEqual({ foil: "foil-unknown", border: CARD_BORDER });
    resetCosmeticsForTests();
    expect(readCardPicks(OWN)).toEqual({ foil: DEFAULT_CARD_COSMETICS.foil, border: CARD_BORDER });

    // Unparsable storage is no picks.
    localStorage.setItem(CARD_PICKS_SETTINGS_KEY, "{not json");
    resetCosmeticsForTests();
    expect(readCardPicks(OWN)).toEqual({});
  });

  it("R1655000 a write starts from what storage holds now: another tab's picks and a newer client's kinds stand", () => {
    expect(readCardPicks(OWN)).toEqual({});
    // Another tab, maybe a newer client's, wrote after this page read.
    localStorage.setItem(CARD_PICKS_SETTINGS_KEY, JSON.stringify({ [OTHER]: { border: CARD_BORDER }, [OWN]: { sleeve: "sleeve-x" } }));

    writeCardPick(OWN, "foil", PICKS.foil);

    expect(JSON.parse(localStorage.getItem(CARD_PICKS_SETTINGS_KEY) ?? "{}")).toEqual({
      [OTHER]: { border: CARD_BORDER },
      [OWN]: { sleeve: "sleeve-x", foil: PICKS.foil },
    });
    expect(readCardPicks(OTHER)).toEqual({ border: CARD_BORDER });
    expect(readCardPicks(OWN)).toEqual({ foil: PICKS.foil });
  });

  it("R1655000 a pick storage cannot keep holds in memory for the page, past the next write", () => {
    const setItem = vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new DOMException("full", "QuotaExceededError");
    });
    writeCardPick(OWN, "foil", PICKS.foil);
    writeCardPick(OTHER, "border", CARD_BORDER);
    expect(readCardPicks(OWN)).toEqual({ foil: PICKS.foil });
    expect(readCardPicks(OTHER)).toEqual({ border: CARD_BORDER });
    setItem.mockRestore();

    // Storage takes the next write, and it holds both.
    writeCardFullArt(OWN, true);
    expect(JSON.parse(localStorage.getItem(CARD_PICKS_SETTINGS_KEY) ?? "{}")).toEqual({
      [OWN]: { foil: PICKS.foil, fullArt: true },
      [OTHER]: { border: CARD_BORDER },
    });
  });

  it("R1655000 a card wears its own picks on the board's faces while another card draws the defaults", () => {
    writeCosmetics({ border: PICKS.border });
    writeCardPick(OWN, "foil", PICKS.foil);
    writeCardPick(OWN, "border", CARD_BORDER);
    const { container } = render(
      <>
        <CardFace face={face(OWN)} worn />
        <MinionFace face={face(OWN)} unit={unit("p1")} worn />
        <CardFace face={face(OTHER)} worn />
        <MinionFace face={face(OTHER)} unit={unit("p1", { defId: OTHER })} worn />
      </>,
    );
    const [own, ownMinion, other, otherMinion] = [...container.querySelectorAll<HTMLElement>(".cf")];
    for (const element of [own, ownMinion]) {
      expect(element?.getAttribute("data-cosmetic-foil")).toBe(PICKS.foil);
      expect(element?.getAttribute("data-frame")).toBe(CARD_BORDER);
      expect(element?.getAttribute("data-finish")).toBeNull();
    }
    // R1953001: the account's border is not worn; the card with no pick of its own draws the default.
    for (const element of [other, otherMinion]) {
      expect(element?.getAttribute("data-cosmetic-foil")).toBeNull();
      expect(element?.getAttribute("data-frame")).toBeNull();
    }
  });

  it("R1655000 a pick of a kind's default is a pick, and clearing it leaves the kind to its default, live", () => {
    writeCosmetics({ border: PICKS.border });
    writeCardPick(OWN, "border", DEFAULT_CARD_COSMETICS.border);
    const { container } = render(
      <>
        <CardFace face={face(OWN)} worn />
        <CardFace face={face(OTHER)} worn />
      </>,
    );
    const [own, other] = [...container.querySelectorAll<HTMLElement>(".cf")];
    expect(readCardPicks(OWN)).toEqual({ border: DEFAULT_CARD_COSMETICS.border });
    expect(own?.getAttribute("data-frame")).toBeNull();
    expect(other?.getAttribute("data-frame")).toBeNull();
    // A write re-renders the faces that wear it.
    act(() => {
      writeCardPick(OWN, "border", CARD_BORDER);
    });
    expect(own?.getAttribute("data-frame")).toBe(CARD_BORDER);
    act(() => {
      writeCardPick(OWN, "border", null);
    });
    expect(readCardPicks(OWN)).toEqual({});
    expect(own?.getAttribute("data-frame")).toBeNull();
    expect(other?.getAttribute("data-frame")).toBeNull();
  });

  it("R1655000 a face that is not the viewer's own draws the defaults, whatever its card's picks", () => {
    writeCosmetics(PICKS);
    writeCardPick(OWN, "foil", PICKS.foil);
    writeCardPick(OWN, "border", CARD_BORDER);
    const { container } = render(
      <>
        <CardFace face={face(OWN)} />
        <MinionFace face={face(OWN)} unit={unit("p2")} />
      </>,
    );
    const faces = [...container.querySelectorAll<HTMLElement>(".cf")];
    expect(faces).toHaveLength(2);
    for (const element of faces) {
      for (const attribute of ATTRIBUTES) expect(element.getAttribute(attribute), attribute).toBeNull();
    }
  });

  it("R1655000 a Mythic face wears no cosmetic foil and a Radiant face nothing, whatever its card's picks", () => {
    for (const defId of ["core-096", OWN]) {
      writeCardPick(defId, "foil", PICKS.foil);
      writeCardPick(defId, "finish", PICKS.finish);
      writeCardPick(defId, "border", CARD_BORDER);
    }
    const { container } = render(
      <>
        <CardFace face={face("core-096")} worn />
        <CardFace face={face(OWN, true)} worn />
      </>,
    );
    const [mythic, radiant] = [...container.querySelectorAll<HTMLElement>(".cf")];
    expect(mythic?.getAttribute("data-cosmetic-foil")).toBeNull();
    expect(mythic?.getAttribute("data-finish")).toBe(PICKS.finish);
    expect(mythic?.getAttribute("data-frame")).toBe(CARD_BORDER);
    for (const attribute of ATTRIBUTES) expect(radiant?.getAttribute(attribute), attribute).toBeNull();
  });

  it("R1655000 a pick the viewer does not own on its card is not worn: the kind's default shows", () => {
    // A face passes the kind defaults under the card's picks (R1953001).
    const card = { border: CARD_BORDER };
    expect(wornCosmetics(face(OWN), DEFAULT_CARD_COSMETICS, card).border).toBe(CARD_BORDER);
    expect(wornCosmetics(face(OWN), DEFAULT_CARD_COSMETICS, card, (id) => id === CARD_BORDER).border).toBe(CARD_BORDER);
    expect(wornCosmetics(face(OWN), DEFAULT_CARD_COSMETICS, card, (id) => id !== CARD_BORDER).border).toBe(DEFAULT_CARD_COSMETICS.border);
    // Not the viewer's own: the defaults, picks owned or not.
    expect(wornCosmetics(face(OWN), null, card).border).toBe(DEFAULT_CARD_COSMETICS.border);
  });

  it("R1655000 the viewer's own card's hover preview, touch sheet and detail view wear its picks; another's draws the defaults", () => {
    writeCosmetics({ finish: PICKS.finish });
    writeCardPick(OWN, "border", CARD_BORDER);
    const def = CATALOG[OWN];
    if (def === undefined) throw new Error("no card");
    const frames = (own: boolean): (string | null)[] => {
      const found: (string | null)[] = [];
      const wrap = (child: ReactElement) => <OwnCopy own={own}>{child}</OwnCopy>;
      render(wrap(<HoverPreview face={face(OWN)} anchor={ANCHOR} />));
      found.push(screen.getByTestId(INSPECT_FACE).querySelector(".cf")?.getAttribute("data-frame") ?? null);
      cleanup();
      render(wrap(<InspectSheet face={face(OWN)} onClose={() => {}} />));
      found.push(screen.getByTestId(INSPECT_FACE).querySelector(".cf")?.getAttribute("data-frame") ?? null);
      cleanup();
      closeInspect();
      render(wrap(<CardDetail def={def} onClose={() => {}} />));
      found.push(screen.getByTestId(INSPECT_FACE_BASE).querySelector(".cf")?.getAttribute("data-frame") ?? null);
      // The Radiant face's gold is its own.
      found.push(screen.getByTestId(INSPECT_FACE_RADIANT).querySelector(".cf")?.getAttribute("data-frame") ?? null);
      cleanup();
      closeInspect();
      return found;
    };
    expect(frames(true)).toEqual([CARD_BORDER, CARD_BORDER, CARD_BORDER, null]);
    expect(frames(false)).toEqual([null, null, null, null]);

    // Another card in the same preview wears none of the first card's picks, nor the account's (R1953001).
    render(
      <OwnCopy own>
        <HoverPreview face={face(OTHER)} anchor={ANCHOR} />
      </OwnCopy>,
    );
    const preview = screen.getByTestId(INSPECT_FACE).querySelector(".cf");
    expect(preview?.getAttribute("data-frame")).toBeNull();
    expect(preview?.getAttribute("data-finish")).toBeNull();
  });
});

describe("R1956000 R1956001 picks across tabs, and a Created card's generator", () => {
  /** A card with picks of its own, and a second border apart from the account's `PICKS.border`. */
  const OWN = "core-004";
  const CARD_BORDER = secondItem("border");
  const TABLE = firstItem("table");

  /** Another tab's write: storage changes there, and this window hears of it. */
  function anotherTabWrites(key: string | null, value?: unknown): void {
    if (key !== null && value !== undefined) localStorage.setItem(key, JSON.stringify(value));
    window.dispatchEvent(new StorageEvent("storage", { key }));
  }

  /** The account store's playmat as a board reads it (R1297000), mounted beside a face. */
  function AccountTable(): ReactElement {
    return <span data-testid="account-table">{useCosmetics().table}</span>;
  }

  it("R1956000 a pick another tab stores reaches what this tab draws at once, the card's and the account's", () => {
    const { container } = render(
      <>
        <CardFace face={face(OWN)} worn />
        <AccountTable />
      </>,
    );
    const shown = root(container);
    const table = screen.getByTestId("account-table");
    expect(shown.getAttribute("data-frame")).toBeNull();

    act(() => {
      anotherTabWrites(CARD_PICKS_SETTINGS_KEY, { [OWN]: { border: CARD_BORDER } });
    });
    // The same face, re-rendered with no remount.
    expect(root(container)).toBe(shown);
    expect(shown.getAttribute("data-frame")).toBe(CARD_BORDER);
    expect(readCardPicks(OWN)).toEqual({ border: CARD_BORDER });

    act(() => {
      anotherTabWrites(COSMETICS_SETTINGS_KEY, { table: TABLE, border: PICKS.border });
    });
    expect(table.textContent).toBe(TABLE);
    expect(readCosmetics().border).toBe(PICKS.border);
    // R1953001: the account's border dresses no card, so the face keeps its own.
    expect(shown.getAttribute("data-frame")).toBe(CARD_BORDER);

    // Another key is none of these stores'; a cleared storage (a null key) is both.
    act(() => {
      localStorage.clear();
      anotherTabWrites("jackioh.something-else", 1);
    });
    expect(shown.getAttribute("data-frame")).toBe(CARD_BORDER);
    expect(table.textContent).toBe(TABLE);
    act(() => {
      localStorage.clear();
      anotherTabWrites(null);
    });
    expect(shown.getAttribute("data-frame")).toBeNull();
    expect(table.textContent).toBe(DEFAULT_CARD_COSMETICS.table);
    expect(readCosmetics()).toEqual(DEFAULT_CARD_COSMETICS);
    expect(readCardPicks(OWN)).toEqual({});
  });

  it("R1956000 a face mounted after another tab's pick draws it, though none was mounted to hear it", () => {
    render(
      <>
        <CardFace face={face(OWN)} worn />
        <AccountTable />
      </>,
    );
    cleanup();
    // Nothing is mounted while the other tab writes.
    anotherTabWrites(CARD_PICKS_SETTINGS_KEY, { [OWN]: { border: CARD_BORDER, finish: PICKS.finish } });
    anotherTabWrites(COSMETICS_SETTINGS_KEY, { table: TABLE });
    const { container } = render(
      <>
        <CardFace face={face(OWN)} worn />
        <AccountTable />
      </>,
    );
    expect(root(container).getAttribute("data-frame")).toBe(CARD_BORDER);
    expect(root(container).getAttribute("data-finish")).toBe(PICKS.finish);
    expect(screen.getByTestId("account-table").textContent).toBe(TABLE);
  });

  it("R1956001 a Created card wears its generator's foil, finish and border, owned on the generator, never its full art", () => {
    const GENERATOR = OWN;
    // Big D-fender, a hero portrait's card, which prints full art.
    const CREATED = "core-001";
    const created = (defId: string = CREATED, radiant = false): FaceModel =>
      faceModel({ defId, def: CATALOG[defId], radiant, inPlay: { created: true, createdBy: GENERATOR } });
    writeCardPick(GENERATOR, "foil", PICKS.foil);
    writeCardPick(GENERATOR, "border", "border-stone");
    writeCardPick(GENERATOR, "finish", "finish-parchment");
    // The generator's own full art is off; the Created card's is its own card's, the setting's.
    writeCardFullArt(GENERATOR, false);
    // The Created card's own picks are not what it wears.
    writeCardPick(CREATED, "border", CARD_BORDER);

    let { container } = render(<CardFace face={created()} worn />);
    let shown = root(container);
    expect(shown.getAttribute("data-frame")).toBe("border-stone");
    expect(shown.getAttribute("data-finish")).toBe("finish-parchment");
    expect(shown.getAttribute("data-cosmetic-foil")).toBe(PICKS.foil);
    expect(shown.getAttribute("data-full-art")).toBe("true");
    cleanup();

    // Ownership is the generator's: what the ledger lists on it is worn, what it does not is not.
    ({ container } = render(
      <FullArtOwned owned={new Map([[GENERATOR, new Set(["border-stone"])]])}>
        <CardFace face={created()} worn />
      </FullArtOwned>,
    ));
    shown = root(container);
    expect(shown.getAttribute("data-frame")).toBe("border-stone");
    expect(shown.getAttribute("data-finish")).toBeNull();
    cleanup();
    // A ledger listing them on the Created card's own definition owns nothing on the generator.
    ({ container } = render(
      <FullArtOwned owned={new Map([[CREATED, new Set(["border-stone", "finish-parchment"])]])}>
        <CardFace face={created()} worn />
      </FullArtOwned>,
    ));
    shown = root(container);
    expect(shown.getAttribute("data-frame")).toBeNull();
    expect(shown.getAttribute("data-finish")).toBeNull();
    cleanup();

    // The Created card's own rarity holds: a Mythic face wears no cosmetic foil, a Radiant face nothing.
    ({ container } = render(
      <>
        <CardFace face={created("core-096")} worn />
        <CardFace face={created(CREATED, true)} worn />
        <CardFace face={created()} />
      </>,
    ));
    const [mythic, radiant, unworn] = [...container.querySelectorAll<HTMLElement>(".cf")];
    expect(mythic?.getAttribute("data-cosmetic-foil")).toBeNull();
    expect(mythic?.getAttribute("data-frame")).toBe("border-stone");
    for (const attribute of ATTRIBUTES) {
      expect(radiant?.getAttribute(attribute), attribute).toBeNull();
      // Not the viewer's own: the defaults, whatever its generator wears.
      expect(unworn?.getAttribute(attribute), attribute).toBeNull();
    }
  });
});

describe("R1833000 the 36 earned cosmetics' looks", () => {
  /** Whether some rule of `css` whose selector holds `needle` declares something. */
  function styles(css: string, needle: string): boolean {
    return rules(css).some((rule) => rule.selector.includes(needle) && /[a-z0-9-]+\s*:/.test(rule.body));
  }

  /** A selector list's selectors: split at its commas, those inside `:not(…)` and `:is(…)` kept. */
  function selectorsOf(list: string): string[] {
    const out: string[] = [];
    let depth = 0;
    let start = 0;
    for (let i = 0; i < list.length; i += 1) {
      if (list[i] === "(") depth += 1;
      else if (list[i] === ")") depth -= 1;
      else if (list[i] === "," && depth === 0) {
        out.push(list.slice(start, i).trim());
        start = i + 1;
      }
    }
    out.push(list.slice(start).trim());
    return out;
  }

  /** The non-default entries of the kinds a player picks, as `[id, kind]`. */
  const PICKED = COSMETICS.filter((cosmetic) => !cosmetic.default && cosmetic.kind !== "fullArt" && cosmetic.kind !== "portrait");

  it("R1833000 every non-default foil, finish, border, table, field and card back has a rule that styles it", () => {
    expect(PICKED.length).toBe(6 * 9);
    for (const { id, kind } of PICKED) {
      switch (kind) {
        case "foil":
          expect(styles(cosmeticsCss, `[data-cosmetic-foil="${id}"]`), id).toBe(true);
          break;
        case "finish":
          expect(styles(cosmeticsCss, `.cf[data-finish="${id}"]`), id).toBe(true);
          break;
        case "border":
          expect(styles(cosmeticsCss, `.cf[data-frame="${id}"]`), id).toBe(true);
          break;
        case "cardBack":
          // A back draws on the viewer's face-down cards and on their deck pile alike.
          expect(styles(cosmeticsCss, `.cf-back[data-card-back="${id}"]`), id).toBe(true);
          expect(styles(cosmeticsCss, `.pile[data-card-back="${id}"]`), id).toBe(true);
          break;
        case "table":
          expect(styles(tableCss, `.field[data-table="${id}"]`), id).toBe(true);
          // And the Collection's tile draws it at tile size.
          expect(styles(collectionCss, `.collection-miniboard[data-table="${id}"]`), id).toBe(true);
          break;
        case "field":
          expect(styles(tableCss, `.board-backdrop[data-backdrop="${id}"]`), id).toBe(true);
          break;
      }
    }
  });

  it("R1833000 every new chase moves only inside its sheet's gate by transform or opacity, and MOVING_FINISHES_AND_BORDERS names the ones that move", () => {
    const gated = [...gatedRules(cosmeticsCss), ...gatedRules(tableCss), ...gatedRules(collectionCss)];
    // Every moving rule waits on the settings' Reduce motion, and a card's on its motion slot.
    for (const rule of gated) {
      for (const part of selectorsOf(rule.selector)) {
        expect(part, rule.selector).toMatch(/^:root:not\(\[data-reduce-motion="true"\]\) /);
        if (/\[data-(finish|frame|cosmetic-foil)=/.test(part)) expect(part, rule.selector).toContain('[data-cosmetic-motion="animated"]');
      }
    }
    const moves = (needle: string): boolean => gated.some((rule) => rule.selector.includes(needle) && /animation\s*:/.test(rule.body));
    const needleOf = (id: string, kind: string): string => {
      switch (kind) {
        case "foil":
          return `[data-cosmetic-foil="${id}"]`;
        case "finish":
          return `[data-finish="${id}"]`;
        case "border":
          return `[data-frame="${id}"]`;
        case "cardBack":
          return `[data-card-back="${id}"]`;
        case "table":
          return `[data-table="${id}"]`;
        default:
          return `[data-backdrop="${id}"]`;
      }
    };
    const chases = PICKED.filter((cosmetic) => cosmetic.chase && !cosmetic.starter);
    expect(chases.length).toBe(15);
    for (const { id, kind } of chases) expect(moves(needleOf(id, kind)), `${id} moves`).toBe(true);
    // The finishes and borders that move are exactly the set a face's motion slot counts.
    const moving = PICKED.filter((cosmetic) => (cosmetic.kind === "finish" || cosmetic.kind === "border") && moves(needleOf(cosmetic.id, cosmetic.kind)));
    expect(new Set(moving.map((cosmetic) => cosmetic.id))).toEqual(MOVING_FINISHES_AND_BORDERS);
    // Every loop of each sheet moves by transform or opacity alone.
    for (const css of [cosmeticsCss, tableCss, collectionCss]) {
      const frames = [...css.matchAll(/@keyframes\s+([a-z-]+)\s*\{/g)];
      expect(frames.length).toBeGreaterThan(0);
      for (const frame of frames) {
        const body = blockFrom(css, (frame.index ?? 0) + frame[0].length - 1);
        for (const property of [...body.matchAll(/([a-z-]+)\s*:/g)].map((match) => match[1])) {
          expect(["transform", "opacity"], `${frame[1] ?? ""}: ${property ?? ""}`).toContain(property);
        }
      }
    }
    // R1953001: the Collection's sheet moves a chase playmat's pads on its mini board, inside its gate alone.
    const movesMini = (rule: { selector: string; body: string }): boolean => rule.selector.includes(".collection-miniboard") && /animation\s*:/.test(rule.body);
    const miniGated = gatedRules(collectionCss).filter(movesMini);
    expect(rules(collectionCss).filter(movesMini).map((rule) => rule.selector)).toEqual(miniGated.map((rule) => rule.selector));
    expect(miniGated.map((rule) => /\[data-table="([a-z-]+)"\]/.exec(rule.selector)?.[1])).toEqual(
      PICKED.filter((cosmetic) => cosmetic.kind === "table" && cosmetic.chase).map((cosmetic) => cosmetic.id),
    );
  });

  it("R1833000 a face wearing a moving finish or border claims a motion slot, and a still one claims none", () => {
    wearOn("core-004", { finish: "finish-starlit" });
    const once = (): HTMLElement => root(render(<CardFace face={face("core-004")} worn />).container);
    let cf = once();
    expect(cf.getAttribute("data-finish")).toBe("finish-starlit");
    expect(cf.getAttribute("data-cosmetic-foil")).toBeNull();
    expect(cf.getAttribute("data-cosmetic-motion")).toBe("animated");
    cleanup();

    wearOn("core-004", { finish: "finish-velvet", border: "border-celestial" });
    expect(once().getAttribute("data-cosmetic-motion")).toBe("animated");
    cleanup();
    wearOn("core-004", { border: "border-runes" });
    cf = once();
    expect(cf.getAttribute("data-frame")).toBe("border-runes");
    expect(cf.getAttribute("data-cosmetic-motion")).toBeNull();
    cleanup();

    // The cap is the foils': past COSMETIC_FOIL_ANIMATED_MAX faces a moving border holds still.
    wearOn("core-004", { border: "border-filigree" });
    const { container } = render(
      <>
        {Array.from({ length: COSMETIC_FOIL_ANIMATED_MAX + 1 }, (_unused, index) => (
          <CardFace key={index} face={face("core-004")} worn />
        ))}
      </>,
    );
    const motions = [...container.querySelectorAll<HTMLElement>(".cf")].map((element) => element.getAttribute("data-cosmetic-motion"));
    expect(motions).toEqual([...Array<string>(COSMETIC_FOIL_ANIMATED_MAX).fill("animated"), "static"]);
    cleanup();

    // With Animated foil off it holds still too.
    writeCardSettings({ animatedFoil: false });
    wearOn("core-004", { finish: "finish-obsidian", border: DEFAULT_CARD_COSMETICS.border });
    expect(once().getAttribute("data-cosmetic-motion")).toBe("static");
  });
});

describe("R2655003 the full-art tiers' motion", () => {
  const GATE = ':root:not([data-reduce-motion="true"]) ';

  it("R2655003 the full-art tiers move only inside the motion gate by transform or opacity, and stand still outside it", () => {
    const gated = gatedRules(cosmeticsCss);
    const moving = gated.filter((rule) => rule.selector.includes("cf-full-art--") && /animation\s*:/.test(rule.body));
    // The art's drift and the band's sweep on both moving tiers; on the Mythic, R2715002's signing: the
    // wipe, the pen, the landing's burst and flourish, and the shine.
    expect(moving.map((rule) => rule.selector)).toEqual([
      `${GATE}.cf.cf-full-art--animated[data-full-art-motion="animated"] .cf-art-frame > .cf-art`,
      `${GATE}.cf.cf-full-art--animated[data-full-art-motion="animated"] .cf-full-art-sweep`,
      `${GATE}.cf.cf-full-art--signed[data-full-art-motion="animated"] .cf-signature-wipe`,
      `${GATE}.cf.cf-full-art--signed[data-full-art-motion="animated"] .cf-signature-pen`,
      `${GATE}.cf.cf-full-art--signed[data-full-art-motion="animated"] .cf-signature-burst`,
      `${GATE}.cf.cf-full-art--signed[data-full-art-motion="animated"] .cf-signature-flourish`,
      `${GATE}.cf.cf-full-art--signed[data-full-art-motion="animated"] .cf-signature-shine`,
    ]);
    // Outside the gate nothing of a full art's tier animates: the still frame and the whole name.
    const all = rules(cosmeticsCss);
    const ungated = all.filter(
      (rule) => /cf-full-art|cf-signature/.test(rule.selector) && !gated.some((inner) => inner.selector === rule.selector && inner.body === rule.body),
    );
    expect(ungated.length).toBeGreaterThan(0);
    for (const rule of ungated) expect(rule.body, rule.selector).not.toMatch(/animation|transition|transform\s*:/);
    // Each loop moves by transform or opacity alone, and the name signs once and holds, but its shine.
    for (const name of [
      "full-art-drift",
      "full-art-sweep",
      "full-art-sign-wipe",
      "full-art-sign-pen",
      "full-art-sign-burst",
      "full-art-sign-flourish",
      "full-art-sign-shine",
    ]) {
      const at = cosmeticsCss.indexOf(`@keyframes ${name} {`);
      expect(at, name).toBeGreaterThanOrEqual(0);
      const body = blockFrom(cosmeticsCss, cosmeticsCss.indexOf("{", at));
      const properties = [...body.matchAll(/([a-z-]+)\s*:/g)].map((match) => match[1]);
      expect(properties.length, name).toBeGreaterThan(0);
      for (const property of properties) expect(["transform", "opacity"], `${name}: ${property ?? ""}`).toContain(property);
    }
    const signing = moving.filter((rule) => rule.selector.includes("cf-signature") && !rule.selector.endsWith(".cf-signature-shine"));
    for (const rule of signing) expect(rule.body, rule.selector).toMatch(/\bboth\b/);
    for (const rule of signing) expect(rule.body, rule.selector).not.toMatch(/\binfinite\b/);
    for (const rule of moving.filter((rule) => !signing.includes(rule))) expect(rule.body, rule.selector).toMatch(/\binfinite\b/);
    // The signature is set in the self-hosted script face, in the palette's colors.
    const text = ruleBody(cosmeticsCss, /\.cf-signature-text\s*\{/);
    expect(text).toMatch(/font-family:\s*var\(--font-signature\)/);
    for (const rule of all.filter((rule) => /cf-full-art|cf-signature/.test(rule.selector))) expect(hexes(rule.body), rule.selector).toEqual([]);
  });
});

describe("R1953001 the chases move everywhere, and a card wears its own picks or the defaults", () => {
  /** Every Legendary and Mythic entry of the registry, the starters among them, so a new one is walked too. */
  const CHASES = COSMETICS.filter((cosmetic) => cosmetic.rarity === "Legendary" || cosmetic.rarity === "Mythic");
  /** A Common card the catalog has, which wears every cosmetic foil (no rarity foil of its own). */
  const CARD = "core-004";
  const GATE = ':root:not([data-reduce-motion="true"]) ';

  /** Whether a rule in one of `css`'s motion gates, its selector opening with `selector`, animates. */
  function animates(css: string, selector: string): boolean {
    return gatedRules(css).some((rule) => rule.selector.startsWith(`${GATE}${selector}`) && /animation\s*:/.test(rule.body));
  }

  /** The `data-cosmetic-motion` of the face `element` draws. */
  function motion(element: ReactElement): string | null {
    const found = render(element).container.querySelector(".cf")?.getAttribute("data-cosmetic-motion") ?? null;
    cleanup();
    return found;
  }

  it("R1953001 every Legendary and Mythic foil, finish, border, table, card back and field moves in a game, in the Collection's samples and on its mini boards", () => {
    // The chase mark is exactly Legendary and Mythic (R1333000), and every kind a player equips has
    // some, a card's full art among them since #2655 (R2655000).
    expect(CHASES.map((cosmetic) => cosmetic.id)).toEqual(COSMETICS.filter((cosmetic) => cosmetic.chase).map((cosmetic) => cosmetic.id));
    expect(new Set(CHASES.map((cosmetic) => cosmetic.kind))).toEqual(new Set([...CARD_SLOT_KINDS, ...DECK_SLOT_KINDS, "fullArt"]));
    for (const { id } of COSMETICS) expect(isChaseCosmetic(id), id).toBe(CHASES.some((chase) => chase.id === id));
    expect(isChaseCosmetic("no-such-cosmetic")).toBe(false);

    for (const { id, kind } of CHASES) {
      switch (kind) {
        case "foil":
          expect(animates(cosmeticsCss, `.cf[data-cosmetic-foil="${id}"][data-cosmetic-motion="animated"]`), id).toBe(true);
          break;
        case "finish":
          expect(animates(cosmeticsCss, `.cf[data-finish="${id}"][data-cosmetic-motion="animated"]`), id).toBe(true);
          expect(MOVING_FINISHES_AND_BORDERS.has(id), `${id} takes a motion slot`).toBe(true);
          break;
        case "border":
          expect(animates(cosmeticsCss, `.cf[data-frame="${id}"][data-cosmetic-motion="animated"]`), id).toBe(true);
          expect(MOVING_FINISHES_AND_BORDERS.has(id), `${id} takes a motion slot`).toBe(true);
          break;
        case "cardBack":
          // Scoped to no screen, so the back moves in a game, on a tile and in a reveal alike.
          expect(animates(cosmeticsCss, `.cf-back[data-card-back="${id}"] `), id).toBe(true);
          break;
        case "table":
          expect(animates(tableCss, `.field[data-table="${id}"] `), `${id} in a game`).toBe(true);
          expect(animates(collectionCss, `:is(.tavern.app-shell, .cosmetic-look) .collection-miniboard[data-table="${id}"] `), `${id}'s mini board`).toBe(true);
          break;
        case "field":
          // Scoped to no board, so the mini board's backdrop (game/Backdrop.tsx) moves as the game's does.
          expect(animates(tableCss, `.board-backdrop[data-backdrop="${id}"] `), id).toBe(true);
          break;
        case "fullArt":
          // R2655000: every Legendary and Mythic full art moves its art wherever its card prints it, by
          // its tier's class, which CardFace sets on every full face of that tier (fullArtTiers.test.tsx).
          expect(animates(cosmeticsCss, '.cf.cf-full-art--animated[data-full-art-motion="animated"] '), id).toBe(true);
          break;
        default:
          throw new Error(`${id} is a chase of a kind no player equips`);
      }
    }

    // A tile that is not the equipped one, hovered or focused holds its pads still, as it does its
    // particles: its rule weighs what the pads' motion rules do, so it comes after them.
    const all = rules(collectionCss);
    const pause = all.find((rule) => rule.selector.includes(".collection-tile:not") && rule.selector.endsWith(".collection-miniboard__pads > span"));
    expect(pause?.body).toMatch(/animation-play-state:\s*paused/);
    const padMotions = all.filter((rule) => rule.selector.includes(".collection-miniboard[data-table=") && /animation\s*:/.test(rule.body));
    expect(padMotions.length).toBeGreaterThan(0);
    for (const rule of padMotions) expect(rule.at).toBeLessThan(pause?.at ?? 0);

    // On a face in a game and on a sample (a reveal's, the Collection's), each chase foil, finish and
    // border moves while Animated foil is on, and holds still while it is off.
    for (const animatedFoil of [true, false]) {
      writeCardSettings({ animatedFoil });
      const expected = animatedFoil ? "animated" : "static";
      for (const { id, kind } of CHASES) {
        if (kind !== "foil" && kind !== "finish" && kind !== "border") continue;
        writeCardPick(CARD, kind, id);
        expect(motion(<CardFace face={face(CARD)} worn />), `${id} in a game`).toBe(expected);
        writeCardPick(CARD, kind, null);
        expect(motion(<CosmeticLook id={id} cardId={CARD} />), `${id}'s sample`).toBe(expected);
      }
    }
    // A sample of what is no chase holds still.
    writeCardSettings({ animatedFoil: true });
    expect(motion(<CosmeticLook id="foil-holo-sweep" cardId={CARD} />)).toBe("static");
  });

  it("R1953001 a card with no pick of its own draws its kind's default though the synced settings hold another foil, finish and border", () => {
    const synced = { foil: "foil-void-shimmer", finish: "finish-starlit", border: "border-celestial" } as const;
    writeCosmetics(synced);
    // The synced group keeps them, as R1655000 froze its shape.
    expect(readCosmetics()).toMatchObject(synced);

    const { container } = render(
      <>
        <CardFace face={face(CARD)} worn />
        <MinionFace face={face(CARD)} unit={unit("p1", { defId: CARD })} worn />
        <OwnCopy own>
          <HoverPreview face={face(CARD)} anchor={{ left: 600, top: 200, width: 100, height: 140 }} />
        </OwnCopy>
      </>,
    );
    const faces = [...container.querySelectorAll<HTMLElement>(".cf"), ...screen.getByTestId(INSPECT_FACE).querySelectorAll<HTMLElement>(".cf")];
    expect(faces.length).toBeGreaterThanOrEqual(3);
    for (const element of faces) {
      for (const attribute of ATTRIBUTES) expect(element.getAttribute(attribute), attribute).toBeNull();
    }
    cleanup();
    closeInspect();

    // A pick of one kind is worn; the kinds it has no pick of still draw their defaults.
    writeCardPick(CARD, "border", PICKS.border);
    const cf = root(render(<CardFace face={face(CARD)} worn />).container);
    expect(cf.getAttribute("data-frame")).toBe(PICKS.border);
    expect(cf.getAttribute("data-cosmetic-foil")).toBeNull();
    expect(cf.getAttribute("data-finish")).toBeNull();
  });
});
