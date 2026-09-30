// B3.4 rule 5: the numbers Degrade, Upgrade and KY's Constant may move are declared in the catalog
// as `params`, and each face's text writes them as `{key}`. `fillParams` (packages/shared) is the one
// function that turns a face's text into what a player reads, from the printed values or from an
// instance's current ones; the client, R277's diff and the text tests all read a face through it.
//
// R482 fixes how: every item of a card's Numbers line (docs/classic-sets.md B6–B8) is one param,
// written `{key}` in each face's text that shows that number and nowhere as a literal, a face's text
// reading right at its own printed value; a numbered keyword (Armor, Lucky, Brittle, Spell Damage)
// and an Echo, Tribute or Activate count is no param, since B3.4's X change tunes those.

import { describe, expect, it } from "vitest";
import { fillParams, type CardDef } from "@jackioh/shared";
import { CATALOG, cardDef } from "../src/catalog-data";

const ENTRIES: readonly CardDef[] = Object.values(CATALOG);
const PLACEHOLDER = /\{([A-Za-z][A-Za-z0-9]*)\}/g;

describe("B3.4 params: the numbers a card declares", () => {
  it("fills each face's text with its own printed values", () => {
    const heal = cardDef("classic-003");
    expect(heal.base.text).toBe("Heal a target {heal}.");
    expect(fillParams(heal, "base")).toBe("Heal a target 9.");
    expect(fillParams(heal, "radiant")).toBe("Heal a target 18.");
  });

  it("fills an instance's current values when given, and leaves a text without params alone", () => {
    const flame = cardDef("classic-016");
    expect(fillParams(flame, "base", { damage: 5 })).toBe("Deal 5 damage.");
    expect(fillParams(flame, "radiant", {})).toBe("Deal 8 damage.");
    const vanilla = cardDef("core-008");
    expect(vanilla.params).toBeUndefined();
    expect(fillParams(vanilla, "base")).toBe(vanilla.base.text);
  });

  it("R482 declares every placeholder a text writes, and writes every param it declares", () => {
    const wrong: string[] = [];
    for (const card of ENTRIES) {
      const keys = new Set((card.params ?? []).map((param) => param.key));
      const written = new Set(
        [card.base.text, card.radiant.text].flatMap((text) => [...text.matchAll(PLACEHOLDER)].map((match) => match[1] ?? "")),
      );
      for (const key of written) if (!keys.has(key)) wrong.push(`${card.id}: {${key}} is not declared`);
      for (const key of keys) if (!written.has(key)) wrong.push(`${card.id}: param ${key} is written in neither face`);
      if (keys.size !== (card.params ?? []).length) wrong.push(`${card.id}: repeats a key`);
    }
    expect(wrong).toEqual([]);
  });

  it("keeps every value inside its bounds, with a direction and a positive step", () => {
    const wrong: string[] = [];
    for (const card of ENTRIES) {
      for (const param of card.params ?? []) {
        const at = `${card.id} ${param.key}`;
        if (param.better !== "up" && param.better !== "down") wrong.push(`${at}: better`);
        if (param.step !== undefined && !(param.step > 0)) wrong.push(`${at}: step`);
        for (const value of [param.base, param.radiant]) {
          if (param.min !== undefined && value < param.min) wrong.push(`${at}: ${value} < min ${param.min}`);
          if (param.max !== undefined && value > param.max) wrong.push(`${at}: ${value} > max ${param.max}`);
        }
      }
    }
    expect(wrong).toEqual([]);
  });

  it("R482 reads the Numbers lines as the brief writes them, on cards that show one of each kind", () => {
    // A Radiant-only number: Cloaked Toe Cracker's "gain 1 mana" exists on its Radiant face only.
    expect(cardDef("classic-006").params).toEqual([{ key: "mana", base: 1, radiant: 1, better: "up", step: 1, min: 1 }]);
    expect(cardDef("classic-006").base.text).not.toContain("{mana}");
    // A stated step and a lower-is-better threshold with a stated floor.
    expect(cardDef("classic-083").params?.[0]).toMatchObject({ key: "damage", base: 11, radiant: 22, step: 2 });
    expect(cardDef("classic-038").params?.[0]).toMatchObject({ key: "plays", base: 3, radiant: 2, better: "down", min: 2 });
    // A percentage: step 10, never above 100.
    expect(cardDef("classicplus-019-2").params?.[0]).toMatchObject({ key: "chance", base: 25, radiant: 50, step: 10, max: 100 });
  });

  it("R482 never makes a numbered keyword, an Echo or an Activate count a param: the X change tunes those", () => {
    const keys = (id: string): string[] => (cardDef(id).params ?? []).map((param) => param.key);
    // Solarius prints Spell Damage +2 and draws 1: only the draw is a param.
    expect(cardDef("classicplus-038").base.keywords).toEqual([{ kind: "Spell Damage", n: 2 }]);
    expect(keys("classicplus-038")).toEqual(["draw"]);
    // Top Loser's Armor, Twice Forward's printed Brittle, Brother Ping's Activate, Echo's Echo 1.
    expect(keys("classicplus-019-1")).toEqual([]);
    expect(keys("classicplus-074")).toEqual(["plays", "brittleGain"]);
    expect(keys("classicplus-076-1")).toEqual(["damage"]);
    expect(cardDef("classic-057").params).toBeUndefined();
    // No param shares its name with a keyword kind.
    for (const card of ENTRIES) {
      for (const param of card.params ?? []) {
        expect(["armor", "lucky", "brittle", "spellDamage", "echo", "activate"].includes(param.key) && card.base.keywords.some((k) => k.kind.toLowerCase() === param.key), `${card.id} ${param.key}`).toBe(false);
      }
    }
  });
});
