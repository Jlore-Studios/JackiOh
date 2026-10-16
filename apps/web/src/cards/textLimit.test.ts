// R1017000: every catalog face fits the measured text limit (constants.ts FACE_TEXT_LIMIT), or prints
// a short form that does (shortText.ts SHORT_FORMS), so no face in the Almanac or the deck builder
// shrinks its art for its words. The pixel half, that no face takes the long layout at 170 px, is the
// component spec's (e2e/cypress/component/card-faces.cy.tsx B15).

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";

import { FACE_TEXT_LIMIT, PREVIEW_TEXT_LIMIT } from "./constants.ts";
import { faceModel } from "./model.ts";
import {
  MORE,
  SHORT_FORMS,
  SUMMIT_DEF_ID,
  cutText,
  faceLines,
  overLimit,
  shortFormOf,
  summitShortText,
  type ShortForm,
} from "./shortText.ts";

const DEFS = Object.values(CATALOG);
const FACES = [false, true] as const;

/** What a collection face prints: its whole text, or its short form. */
function printed(defId: string, radiant: boolean): { full: string; form: ShortForm | null; shown: string } {
  const def = CATALOG[defId];
  if (def === undefined) throw new Error(`no ${defId}`);
  const full = faceModel({ defId, def, radiant }).text.full;
  const form = shortFormOf(defId, radiant);
  return { full, form, shown: (form === null ? null : cutText(full, form)) ?? full };
}

describe("R1017000 the face text limit", () => {
  it("R1017000 faceLines wraps by words at a width, each line of the text starting a new one", () => {
    expect(faceLines("", 28)).toBe(1);
    expect(faceLines("Deal 2 damage.", 28)).toBe(1);
    expect(faceLines("Taunt\nDeal 2 damage.", 28)).toBe(2);
    // 28 characters fill a line; the 29th word-character wraps the word.
    expect(faceLines("aaaa bbbb cccc dddd eeee fff", 28)).toBe(1);
    expect(faceLines("aaaa bbbb cccc dddd eeee ffff", 28)).toBe(2);
    expect(overLimit("a\nb\nc\nd\ne\nf", FACE_TEXT_LIMIT)).toBe(false);
    expect(overLimit("a\nb\nc\nd\ne\nf\ng", FACE_TEXT_LIMIT)).toBe(true);
  });

  it("R1017000 cutText keeps a short form's first lines, or the text up to its first colon, \"…\" in place of its stop", () => {
    expect(cutText("One.\nTwo.\nThree.", 2)).toBe(`One.\nTwo${MORE}`);
    expect(cutText("One random effect: A; B.", "colon")).toBe(`One random effect${MORE}`);
    expect(cutText("随机一个效果：甲；乙。", "colon")).toBe(`随机一个效果${MORE}`);
    expect(cutText("每张牌。\n攻击。\n防御。", 1)).toBe(`每张牌${MORE}`);
    // A line that ends in no full stop keeps its last character.
    expect(cutText("Reach for the stars!\nThen more.", 1)).toBe(`Reach for the stars!${MORE}`);
    // Nothing to cut: a text no longer than its form, one with no colon, and a clamp.
    expect(cutText("One.\nTwo.", 2)).toBeNull();
    expect(cutText("???", "colon")).toBeNull();
    expect(cutText("One.\nTwo.\nThree.", "clamp")).toBeNull();
  });

  it("R1017000 every catalog face fits the limit, or its short form does", () => {
    const over: string[] = [];
    for (const def of DEFS) {
      for (const radiant of FACES) {
        const { form, shown } = printed(def.id, radiant);
        // A clamp prints its whole text, cut by the rules box: the faces still a line over after the cut.
        if (form === "clamp") continue;
        if (overLimit(shown, FACE_TEXT_LIMIT)) {
          over.push(`${def.id} ${def.name} ${radiant ? "radiant" : "base"}: ${String(faceLines(shown, FACE_TEXT_LIMIT.charsPerLine))} lines`);
        }
      }
    }
    expect(over).toEqual([]);
  });

  it("R1017000 a clamp is only for a face over the limit, and each cut short form shortens its face", () => {
    for (const [defId, forms] of Object.entries(SHORT_FORMS)) {
      for (const radiant of FACES) {
        const form = radiant ? forms.radiant : forms.base;
        if (form === undefined) continue;
        const { full, shown } = printed(defId, radiant);
        const where = `${defId} ${radiant ? "radiant" : "base"}`;
        // A clamp is for a face over the limit; a cut short form may also hide a reminder or the
        // designer's flavour at the end of a face that fits, which the click-in carries.
        if (form === "clamp") expect(overLimit(full, FACE_TEXT_LIMIT), `${where} fits the limit whole`).toBe(true);
        else {
          expect(shown, `${where} has nothing to cut`).not.toBe(full);
          expect(shown.endsWith(MORE), where).toBe(true);
          expect(full.startsWith(shown.slice(0, -MORE.length)), `${where} is not its text's start`).toBe(true);
        }
      }
    }
  });

  it("R1017000 a Summit in play prints its header and each next Ascent line within the hover preview's limit", () => {
    const def = CATALOG[SUMMIT_DEF_ID];
    if (def === undefined) throw new Error("no Summit");
    for (const radiant of FACES) {
      const full = faceModel({ defId: SUMMIT_DEF_ID, def, radiant }).text.full;
      for (let level = 0; level <= 10; level += 1) {
        const short = summitShortText(SUMMIT_DEF_ID, full, level);
        expect(short, `level ${String(level)}`).not.toBeNull();
        expect(overLimit(short ?? full, PREVIEW_TEXT_LIMIT), `level ${String(level)}`).toBe(false);
      }
    }
  });
});
