// R1017000: every catalog face fits the measured text limit (constants.ts FACE_TEXT_LIMIT), or prints
// a short form that does (shortText.ts SHORT_FORMS), so no face in the Almanac or the deck builder
// shrinks its art for its words; the pixel half is e2e/cypress/component/card-faces.cy.tsx's (B15).
// R2450000: a staged Quest is measured by the one stage it prints (questFace.ts).

import { describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { GLITCH_DEF_ID } from "@jackioh/engine/config";

import { FACE_TEXT_LIMIT, PREVIEW_TEXT_LIMIT, TEXT_TIER_MAX } from "./constants.ts";
import { faceModel } from "./model.ts";
import { questFaceText } from "./questFace.ts";
import {
  LONG_FACES,
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

/** What a collection face prints: its whole text (a staged Quest's first stage, R2450000), or its short form. */
function printed(defId: string, radiant: boolean): { full: string; form: ShortForm | null; shown: string } {
  const def = CATALOG[defId];
  if (def === undefined) throw new Error(`no ${defId}`);
  const face = faceModel({ defId, def, radiant });
  const full = questFaceText(face)?.full ?? face.text.full;
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

  it("B15 R1017000 the premise: the longest texts (Core's #93, #95 and #98, Classic+ #73, five Meditative cards, two Stinkyscape rows and the held Training Grounds #39) each print a short form", () => {
    const long = DEFS.flatMap((def) =>
      FACES.filter((face) => printed(def.id, face).full.length > TEXT_TIER_MAX.xl).map(
        (face) => `${def.id} ${face ? "radiant" : "base"}`,
      ),
    );
    expect(TEXT_TIER_MAX.xl).toBe(260);
    for (const entry of long) {
      const [id, label] = entry.split(" ");
      expect(shortFormOf(id ?? "", label === "radiant"), entry).not.toBeNull();
    }
    expect([...long].sort()).toEqual([...LONG_FACES].sort());
  });

  it("R2450000 a staged Quest is measured by the stage it prints: no cut short form, Heist's base face whole, the rest clamped and whole in the hover preview", () => {
    expect(shortFormOf("stinkyscape-007", false)).toBeNull();
    for (const [defId, radiant] of [["stinkyscape-007", true], ["stinkyscape-112", false], ["stinkyscape-112", true]] as const) {
      expect(shortFormOf(defId, radiant), `${defId} ${String(radiant)}`).toBe("clamp");
    }
    for (const [defId, radiant] of [["stinkyscape-007", false], ["stinkyscape-007", true], ["stinkyscape-112", false], ["stinkyscape-112", true]] as const) {
      const { full } = printed(defId, radiant);
      expect(full, `${defId} ${String(radiant)} prints one stage`).toContain("Stage 1 of 3, ");
      expect(full).not.toContain("Stage 2");
      expect(overLimit(full, PREVIEW_TEXT_LIMIT), `${defId} ${String(radiant)}`).toBe(false);
    }
    expect(overLimit(printed("stinkyscape-007", false).full, FACE_TEXT_LIMIT)).toBe(false);
    expect(LONG_FACES.filter((entry) => entry.startsWith("stinkyscape-007 ") || entry.startsWith("stinkyscape-112 "))).toEqual([]);
  });

  it("MN09 the premise: the set is in the catalog with a Chinese face for every card, and its longest faces are named", () => {
    const MEDITATIVE = DEFS.filter((def) => def.set === "Meditative" && def.id !== GLITCH_DEF_ID);
    const VARIANTS = [
      { label: "base", radiant: false, chinese: false },
      { label: "radiant", radiant: true, chinese: false },
      { label: "Chinese base", radiant: false, chinese: true },
      { label: "Chinese radiant", radiant: true, chinese: true },
    ] as const;
    expect(MEDITATIVE.length, "Meditative entries in the catalog").toBeGreaterThan(0);
    for (const def of MEDITATIVE) {
      const chinese = faceModel({ defId: def.id, def, radiant: false, chinese: true });
      expect(chinese.chinese, `${def.id} has a Chinese face (#516)`).toBe(true);
      expect(chinese.name, `${def.id}'s Chinese name`).not.toBe("");
    }
    // No Chinese face is among the longest: a Chinese face prints fewer characters for the
    // same words, so the pinned list names only the underlying base and radiant faces.
    const long = MEDITATIVE.flatMap((def) =>
      VARIANTS.filter(
        (face) =>
          faceModel({ defId: def.id, def, radiant: face.radiant, ...(face.chinese ? { chinese: true } : {}) }).text.full
            .length > TEXT_TIER_MAX.xl,
      ).map((face) => `${def.id} ${face.label}`),
    );
    expect([...long].sort()).toEqual([...LONG_FACES.filter((entry) => entry.startsWith("meditative-"))].sort());
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
