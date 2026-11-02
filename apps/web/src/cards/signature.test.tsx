// R2715002: the Mythic's signature signs fast, lands hard and shines, in a hand that suits its card.
// The signing's timings are named constants held under a second, every Mythic full art of the registry
// signs in a face whose file and licence are in public/fonts, and the face draws the name to a known
// length, with its pen, burst, flourish and shine. The pixels (a mid-signing frame, a landed one) are
// e2e's cypress/component/signature.cy.tsx; R2655003's gate is cosmetics.test.tsx's.

import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { cleanup, render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { COSMETICS } from "../wire/generated/cosmetics.ts";
import { CardFace } from "./CardFace.tsx";
import { resetCosmeticsForTests } from "./cosmetics.ts";
import { FULL_ARTS_OF_CARD, FULL_ART_CARDS, FullArtShown, OwnCopy, fullArtPickOf } from "./fullArt.ts";
import {
  SIGNATURE_LAND_MS,
  SIGNATURE_SHINE_PERIOD_MS,
  SIGNATURE_SIGNED_MS,
  SIGNATURE_START_MS,
  SIGNATURE_WIDTH,
  SIGNATURE_WRITE_MS,
} from "./FullArtSignature.tsx";
import { faceModel } from "./model.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "./settings.ts";
import {
  DEFAULT_SIGNATURE_FACE,
  SIGNATURE_FACES,
  SIGNATURE_FACE_OF_CARD,
  SIGNATURE_FACE_OF_SET,
  signatureFaceOf,
  type SignatureFace,
} from "./signatureFaces.ts";

const HERE = dirname(fileURLToPath(import.meta.url));
const FONTS = join(HERE, "..", "..", "public", "fonts");
const fontsCss = readFileSync(join(HERE, "..", "fonts.css"), "utf8");
const cosmeticsCss = readFileSync(join(HERE, "cosmetics.css"), "utf8").replace(/\/\*[\s\S]*?\*\//g, "");

/** The issue's limit on the signing: one second. */
const ONE_SECOND_MS = 1000;

/** The custom property a face's family is named by in fonts.css. */
function familyVariable(face: SignatureFace): string {
  return face === DEFAULT_SIGNATURE_FACE ? "--font-signature" : `--font-signature-${face}`;
}

/** The Mythic full face of `id`, the viewer's own copy shown at that tier. */
function mythic(id: string): HTMLElement {
  const mythicId = FULL_ARTS_OF_CARD.get(id)?.get("Mythic");
  if (mythicId === undefined) throw new Error(`no Mythic of ${id}`);
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  const { container } = render(
    <OwnCopy own>
      <FullArtShown defId={id} pick={fullArtPickOf(id, mythicId)}>
        <CardFace face={faceModel({ defId: id, def, radiant: false })} layout="full" />
      </FullArtShown>
    </OwnCopy>,
  );
  const signature = container.querySelector<HTMLElement>(".cf-signature");
  if (signature === null) throw new Error(`${id}'s Mythic signs nothing`);
  return signature;
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

describe("R2715002 the signature's speed", () => {
  it("R2715002 the signing finishes within SIGNATURE_SIGNED_MS, under a second, and starts at once", () => {
    expect(SIGNATURE_SIGNED_MS).toBe(SIGNATURE_START_MS + SIGNATURE_WRITE_MS + SIGNATURE_LAND_MS);
    expect(SIGNATURE_SIGNED_MS).toBeLessThan(ONE_SECOND_MS);
    // No slow start: the pen touches down within a fifth of the writing.
    expect(SIGNATURE_START_MS).toBeLessThan(SIGNATURE_WRITE_MS / 5);
    // The shine comes back now and then, never as a constant flicker.
    expect(SIGNATURE_SHINE_PERIOD_MS).toBeGreaterThan(SIGNATURE_SIGNED_MS * 2);
  });

  it("R2715002 the stylesheet takes every signing duration from the face's custom properties, eased out", () => {
    const signing = [".cf-signature-wipe", ".cf-signature-pen", ".cf-signature-burst", ".cf-signature-flourish", ".cf-signature-shine"];
    for (const part of signing) {
      const at = cosmeticsCss.indexOf(`[data-full-art-motion="animated"] ${part} {`);
      expect(at, part).toBeGreaterThanOrEqual(0);
      const body = cosmeticsCss.slice(at, cosmeticsCss.indexOf("}", at));
      const animation = /animation:([^;]+);/u.exec(body)?.[1] ?? "";
      expect(animation, part).toMatch(/var\(--cf-sign-(write|land|shine)\)/u);
      expect(animation, part).toMatch(/var\(--cf-sign-(start|landed)\)/u);
      // No duration or delay is written into the sheet itself.
      expect(animation.replace(/cubic-bezier\([^)]*\)/gu, ""), part).not.toMatch(/\d(ms|s)\b/u);
    }
    // The pen writes with no slow start: its curve leaves faster than it lands.
    const wipe = cosmeticsCss.slice(cosmeticsCss.indexOf('[data-full-art-motion="animated"] .cf-signature-wipe {'));
    const [x1, y1] = (/cubic-bezier\(([^)]*)\)/u.exec(wipe)?.[1] ?? "").split(",").map(Number);
    expect((y1 ?? 0) / (x1 ?? 1)).toBeGreaterThan(1);
  });

  it("R2715002 a Mythic face carries its timings as the constants say", () => {
    const signature = mythic("core-001");
    expect(signature.style.getPropertyValue("--cf-sign-start")).toBe(`${SIGNATURE_START_MS}ms`);
    expect(signature.style.getPropertyValue("--cf-sign-write")).toBe(`${SIGNATURE_WRITE_MS}ms`);
    expect(signature.style.getPropertyValue("--cf-sign-land")).toBe(`${SIGNATURE_LAND_MS}ms`);
    expect(signature.style.getPropertyValue("--cf-sign-landed")).toBe(`${SIGNATURE_START_MS + SIGNATURE_WRITE_MS}ms`);
    expect(signature.style.getPropertyValue("--cf-sign-shine")).toBe(`${SIGNATURE_SHINE_PERIOD_MS}ms`);
  });
});

describe("R2715002 the signature's faces", () => {
  it("R2715002 every Mythic full art in the registry signs in a face whose file and licence are in public/fonts", () => {
    const mythics = COSMETICS.flatMap((cosmetic) => (cosmetic.kind === "fullArt" && cosmetic.rarity === "Mythic" ? [cosmetic.card] : []));
    expect(mythics.length).toBe(FULL_ART_CARDS.size);
    for (const card of mythics) {
      const face = signatureFaceOf(card);
      const { family, file, licence } = SIGNATURE_FACES[face];
      expect(existsSync(join(FONTS, file)), `${card}: ${file}`).toBe(true);
      expect(existsSync(join(FONTS, licence)), `${card}: ${licence}`).toBe(true);
      expect(readFileSync(join(FONTS, licence), "utf8"), licence).toMatch(/SIL Open Font License|Apache License/u);
      // fonts.css declares the face from that file, self-hosted, and names it by its custom property.
      const declared = [...fontsCss.matchAll(/@font-face\s*\{([^}]*)\}/gu)].map((match) => match[1] ?? "");
      const own = declared.find((body) => body.includes(`font-family: "${family}"`));
      expect(own, `${card}: ${family}'s @font-face`).toBeDefined();
      expect(own, family).toContain(`src: url("/fonts/${file}") format("woff2")`);
      expect(fontsCss, family).toMatch(new RegExp(`${familyVariable(face)}: "${family}"`, "u"));
      // The face's rule sets the name in it (the default's is the signature text's own rule).
      if (face !== DEFAULT_SIGNATURE_FACE) {
        expect(cosmeticsCss, face).toContain(
          `.cf-signature[data-signature-face="${face}"] .cf-signature-text {\n  font-family: var(${familyVariable(face)});`,
        );
      }
    }
    // Nothing is fetched from anywhere but the site.
    expect(fontsCss).not.toMatch(/url\("?https?:/u);
  });

  it("R2715002 each set signs in a face of its own, and the map's cards in theirs", () => {
    const sets = new Map<string, SignatureFace>();
    for (const card of FULL_ART_CARDS) {
      const set = card.slice(0, card.indexOf("-"));
      expect(SIGNATURE_FACE_OF_SET[set], `${set} has a face`).toBeDefined();
      sets.set(set, SIGNATURE_FACE_OF_SET[set] ?? DEFAULT_SIGNATURE_FACE);
      expect(signatureFaceOf(card), card).toBe(SIGNATURE_FACE_OF_CARD[card] ?? SIGNATURE_FACE_OF_SET[set]);
    }
    // The six sets of the full-art cards, each in its own hand, none of them Dancing Script.
    expect([...sets.keys()].sort()).toEqual(["classic", "classicplus", "core", "meditative", "stinkyscape", "traininggrounds"]);
    expect(new Set(sets.values()).size).toBe(sets.size);
    expect([...sets.values()]).not.toContain(DEFAULT_SIGNATURE_FACE);
    // The map's cards, a listed token (its parent's) and a set with no face of its own.
    expect(signatureFaceOf("core-008")).toBe("script");
    expect(signatureFaceOf("core-086")).toBe("copperplate");
    expect(signatureFaceOf("classicplus-019-1")).toBe(SIGNATURE_FACE_OF_SET["classicplus"]);
    expect(signatureFaceOf("custom-1")).toBe(DEFAULT_SIGNATURE_FACE);
  });

  it("R2715002 a Mythic face signs its name to a known length in its face, with its pen, burst, flourish and shine", () => {
    for (const card of ["core-001", "classic-090", "stinkyscape-154", "traininggrounds-040", "core-008"]) {
      const signature = mythic(card);
      const face = signatureFaceOf(card);
      expect(signature.getAttribute("data-signature-face"), card).toBe(face);
      const texts = [...signature.querySelectorAll<SVGTextElement>("text.cf-signature-text")];
      // The name, and its copy that clips the shine to the ink.
      expect(texts.map((text) => text.textContent), card).toEqual([CATALOG[card]?.name, CATALOG[card]?.name]);
      for (const text of texts) {
        expect(text.getAttribute("lengthAdjust"), card).toBe("spacingAndGlyphs");
        expect(Number(text.getAttribute("textLength")), card).toBeGreaterThan(0);
        expect(Number(text.getAttribute("textLength")), card).toBeLessThanOrEqual(SIGNATURE_WIDTH);
      }
      // The written name shows through the mask the wipe slides in, and the shine through the ink.
      const ink = signature.querySelector(".cf-signature-ink");
      const maskId = /^url\(#(.+)\)$/u.exec(ink?.getAttribute("mask") ?? "")?.[1] ?? "";
      expect(signature.querySelector(`mask[id="${maskId}"] .cf-signature-wipe`), card).not.toBeNull();
      const clip = ink?.querySelector("[clip-path]")?.getAttribute("clip-path") ?? "";
      expect(signature.querySelector(`clipPath[id="${/^url\(#(.+)\)$/u.exec(clip)?.[1] ?? ""}"] text`), card).not.toBeNull();
      expect(ink?.querySelector(".cf-signature-shine"), card).not.toBeNull();
      for (const part of [".cf-signature-pen", ".cf-signature-burst", ".cf-signature-flourish"]) {
        expect(signature.querySelector(part), `${card}: ${part}`).not.toBeNull();
      }
      cleanup();
    }
  });

  it("R2715002 two signatures in one document name masks and clips of their own", () => {
    const { container } = render(
      <OwnCopy own>
        {["core-001", "core-004"].map((id) => (
          <FullArtShown key={id} defId={id} pick={fullArtPickOf(id, FULL_ARTS_OF_CARD.get(id)?.get("Mythic") ?? "")}>
            <CardFace face={faceModel({ defId: id, def: CATALOG[id], radiant: false })} layout="full" />
          </FullArtShown>
        ))}
      </OwnCopy>,
    );
    const ids = [...container.querySelectorAll(".cf-signature [id]")].map((element) => element.id);
    expect(ids.length).toBe(6);
    expect(new Set(ids).size).toBe(ids.length);
  });
});
