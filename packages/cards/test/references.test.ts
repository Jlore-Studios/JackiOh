// R279: a card's text links the cards and tokens it names. The catalog lists them per card
// (`refs`, by id), and this file proves the list against the texts both ways, from the catalog
// alone: every card or token a text names is in that card's `refs`, and every id in `refs` is named
// in one of the card's two texts. It also proves that every token is named by some card, except The
// Coin, which §2.1's setup deals rather than a card (R244).
//
// What "names" means is written once, here, and the client's renderer (apps/web/src/cards/refs.ts)
// reads the same rule: a card's name, or its name before a parenthesis ("Call to Chaos (Core
// Edition)" is named as "Call to Chaos"), alone or with a plural "s", standing as whole words — the
// characters either side are not letters, digits, an apostrophe or a hyphen, so "CN-Viral" does not
// name "CN-Virus" and "Mr. Vanilla" is not named by #61's "Vanilla copy".
//
// Patch v0.2.0 adds two things to the rule. R381 (B2.8): four Classic cards are named like rules
// words — #10 Exile, #36 Burn, #57 Echo and #30 Recycle — so a text that says "Exile …" or "Echo 1"
// names none of them unless its `refs` lists the card (a card's `refs` stays curated). R480: the
// eight Pancake tokens and the ten AI generated cards reach play only through a pool a card names by
// its tag ("a random Pancake token", "a random AI generated card"), so a text naming that pool names
// each of its members for the rule that every token is named by some card.

import { describe, expect, it } from "vitest";
import type { CardDef } from "@jackioh/shared";
import { CATALOG } from "../src/catalog-data";

const ENTRIES: readonly CardDef[] = Object.values(CATALOG);

/** §2.1, R244: dealt by a rule, so no card's text names it — The Coin, and Glitch, which R672's roll deals. */
const DEALT_BY_A_RULE: readonly string[] = ["core-t-coin", "classic-t-glitch"];

/**
 * R381 (B2.8): card names that are also rules words. A text using one names that card only when the
 * card's `refs` lists it: Exile the verb and the pile, Burn at the hand cap, Echo the keyword,
 * Recycle the verb.
 */
const RULES_WORDS: readonly string[] = ["Exile", "Burn", "Echo", "Recycle"];

/** R480: the token pools a card names by tag, and the phrase that names each. */
const TOKEN_POOLS: readonly { tag: "Pancake" | "AI"; phrase: string }[] = [
  { tag: "Pancake", phrase: "Pancake token" },
  { tag: "AI", phrase: "AI generated card" },
];

/** The names a text may call a card by: its name, and its name before a parenthesis. */
function namesOf(def: CardDef): string[] {
  const bare = def.name.replace(/\s*\(.*\)\s*$/, "");
  return bare === def.name ? [def.name] : [def.name, bare];
}

const WORD = /[A-Za-z0-9'-]/;

/** Whether `text` names `name` as whole words, alone or plural. */
function names(text: string, name: string): boolean {
  let from = 0;
  for (;;) {
    const at = text.indexOf(name, from);
    if (at < 0) return false;
    const before = text[at - 1];
    let end = at + name.length;
    if (text[end] === "s") end += 1;
    const after = text[end];
    if ((before === undefined || !WORD.test(before)) && (after === undefined || !WORD.test(after))) return true;
    from = at + 1;
  }
}

/** Every catalog id a card's base or Radiant text names, in catalog order. */
function namedBy(card: CardDef): string[] {
  const texts = [card.base.text, card.radiant.text];
  return ENTRIES.filter((other) => namesOf(other).some((name) => texts.some((text) => names(text, name)))).map(
    (other) => other.id,
  );
}

/** Whether an id names a card whose name is a rules word (R381). */
const isRulesWord = (id: string): boolean => RULES_WORDS.includes(CATALOG[id]?.name ?? "");

describe("R279 the reference map (SPEC §5, §7, §10.10)", () => {
  it("R279 lists in every card's refs exactly the cards and tokens its texts name", () => {
    const wrong: string[] = [];
    for (const card of ENTRIES) {
      const named = namedBy(card);
      const listed = [...(card.refs ?? [])].sort();
      // A rules-word name is required of no text (R381), and may be listed where a text means it.
      const expected = [...named.filter((id) => !isRulesWord(id) || listed.includes(id))].sort();
      if (JSON.stringify(expected) !== JSON.stringify(listed)) {
        wrong.push(`${card.id} ${card.name}: texts name [${expected.join(", ")}], refs list [${listed.join(", ")}]`);
      }
    }
    expect(wrong).toEqual([]);
  });

  it("R279 has every token named by at least one card, except what a rule deals (The Coin, which Classic+ #42 KY's Test also names)", () => {
    const named = new Set(ENTRIES.flatMap((card) => card.refs ?? []));
    // R480: a card whose text names a token pool by its tag names every member of it.
    for (const pool of TOKEN_POOLS) {
      const namedByTag = ENTRIES.some((card) => [card.base.text, card.radiant.text].some((text) => text.includes(pool.phrase)));
      if (!namedByTag) continue;
      for (const member of ENTRIES.filter((card) => card.token && card.tags.includes(pool.tag))) named.add(member.id);
    }
    const unnamed = ENTRIES.filter((card) => card.token && !named.has(card.id)).map((card) => card.id);
    expect(unnamed.filter((id) => !DEALT_BY_A_RULE.includes(id))).toEqual([]);
  });

  it("R480 names the eight Pancake tokens and the ten AI generated cards by their tags, from the cards that make them", () => {
    const pancakes = ENTRIES.filter((card) => card.token && card.tags.includes("Pancake")).map((card) => card.index);
    const ai = ENTRIES.filter((card) => card.token && card.tags.includes("AI")).map((card) => card.index);
    expect(pancakes).toEqual(["12.1", "12.2", "12.3", "12.4", "12.5", "12.6", "12.7", "12.8"]);
    expect(ai).toEqual(["T-AI-1", "T-AI-2", "T-AI-3", "T-AI-4", "T-AI-5", "T-AI-6", "T-AI-7", "T-AI-8", "T-AI-9", "T-AI-10"]);
    const naming = (phrase: string): string[] =>
      ENTRIES.filter((card) => [card.base.text, card.radiant.text].some((text) => text.includes(phrase))).map((card) => card.id);
    expect(naming("Pancake token")).toEqual(["classicplus-012", "classicplus-013"]);
    // AI Slop and Claude's Datacenter make them; Scaling Law counts them.
    expect(naming("AI generated card")).toEqual(["classicplus-043", "classicplus-078", "classicplus-t-ai-02"]);
  });

  it("R381 reads Exile, Burn, Echo and Recycle as rules words: a text that says one names no card unless its refs list it", () => {
    for (const word of RULES_WORDS) {
      const card = ENTRIES.find((entry) => entry.name === word);
      expect(card?.set, `${word} is a Classic card`).toBe("Classic");
    }
    // They are all over the texts as rules words — and no card lists them.
    const usesExile = ENTRIES.filter((card) => names(card.base.text, "Exile") || names(card.radiant.text, "Exile"));
    const usesEcho = ENTRIES.filter((card) => names(card.base.text, "Echo") || names(card.radiant.text, "Echo"));
    expect(usesExile.length).toBeGreaterThan(10);
    expect(usesEcho.length).toBeGreaterThan(2);
    expect(ENTRIES.filter((card) => (card.refs ?? []).some(isRulesWord)).map((card) => card.id)).toEqual([]);
    // "Recycler" is a longer word, so Malzahar's Recycler never names Recycle at all.
    expect(names("Malzahar's Recycler", "Recycle")).toBe(false);
  });

  it("R279 names a card by its name before a parenthesis, and never by a longer word", () => {
    const chaos = CATALOG["core-095"];
    expect(chaos?.refs).toContain("core-095");
    expect(names("Shuffle a CN-Viral Injection", "CN-Virus")).toBe(false);
    expect(names("summon a Vanilla copy", "Mr. Vanilla")).toBe(false);
    expect(names("fill your board with Rush Tokens; if", "Rush Token")).toBe(true);
    expect(names("your units other than Spikey Pillows have", "Spikey Pillow")).toBe(true);
  });

  it("R279 keeps an empty list out of the catalog: a card that names nothing has no refs", () => {
    const empty = ENTRIES.filter((card) => card.refs !== undefined && card.refs.length === 0).map((card) => card.id);
    expect(empty).toEqual([]);
  });
});
