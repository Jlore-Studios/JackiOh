// #3609: a card is drawn again only when its own slice of the `Highlight` changes. Each case adds one thing
// to a highlight and says whether the card `card-u1` (a unit; hand card `hand-card-u1` and its switch read
// the same slice) would see it.

import type { AttackForecast } from "@jackioh/shared";
import { describe, expect, it } from "vitest";

import { sameHighlightAt, type CardReads } from "./cardHighlight.ts";
import { NO_HIGHLIGHT, testid, type Highlight } from "./contract.ts";

const UNIT: CardReads = { testId: "card-u1", instanceId: "u1" };
const HAND: CardReads = { testId: "hand-card-u1", instanceId: "u1" };

type SetName = "legal" | "selected" | "glow" | "condition" | "counted";

/** The highlight with `id` added to the set `set` names. */
function adding(base: Highlight, set: SetName, id: string): Highlight {
  return { ...base, [set]: new Set([...(base[set] ?? []), id]) };
}

function forecast(dealt: number): AttackForecast {
  return { attackerId: "u1", targetId: "e1", dealt, taken: 0, kills: false, survives: true, pops: false, attackerSize: 1, targetSize: 1 };
}

const BASE: Highlight = { legal: new Set(["card-u2"]), selected: new Set(), glow: new Set(["card-u2"]) };
const SETS: SetName[] = ["legal", "selected", "glow", "condition", "counted"];

describe("#3609 sameHighlightAt", () => {
  it("is true for a highlight against itself, and for undefined against NO_HIGHLIGHT", () => {
    expect(sameHighlightAt(BASE, BASE, UNIT)).toBe(true);
    expect(sameHighlightAt(undefined, undefined, UNIT)).toBe(true);
    expect(sameHighlightAt(undefined, NO_HIGHLIGHT, UNIT)).toBe(true);
    expect(sameHighlightAt(NO_HIGHLIGHT, undefined, UNIT)).toBe(true);
    expect(sameHighlightAt(undefined, { legal: new Set(), selected: new Set() }, UNIT)).toBe(true);
  });

  it("is false for undefined against a highlight that lights the card", () => {
    expect(sameHighlightAt(undefined, adding(NO_HIGHLIGHT, "legal", "card-u1"), UNIT)).toBe(false);
    expect(sameHighlightAt(adding(NO_HIGHLIGHT, "glow", "card-u1"), undefined, UNIT)).toBe(false);
  });

  for (const set of SETS.filter((name) => name === "legal" || name === "selected" || name === "glow")) {
    it(`is true when an unrelated id joins ${set}`, () => {
      expect(sameHighlightAt(BASE, adding(BASE, set, "card-u3"), UNIT)).toBe(true);
      expect(sameHighlightAt(adding(BASE, set, "card-u3"), BASE, UNIT)).toBe(true);
      expect(sameHighlightAt(BASE, adding(BASE, set, "zone-you-units-3"), UNIT)).toBe(true);
    });
  }

  for (const set of SETS) {
    it(`is false when the card's own testid joins or leaves ${set}`, () => {
      expect(sameHighlightAt(BASE, adding(BASE, set, "card-u1"), UNIT)).toBe(false);
      expect(sameHighlightAt(adding(BASE, set, "card-u1"), BASE, UNIT)).toBe(false);
    });

    it(`is false when a hand card's testid joins ${set}, and true for another hand card's`, () => {
      expect(sameHighlightAt(BASE, adding(BASE, set, "hand-card-u1"), HAND)).toBe(false);
      expect(sameHighlightAt(BASE, adding(BASE, set, "hand-card-h2"), HAND)).toBe(true);
    });

    it(`is false when the card's switch joins ${set}`, () => {
      expect(sameHighlightAt(BASE, adding(BASE, set, testid.switchPosition("u1")), UNIT)).toBe(false);
      expect(sameHighlightAt(BASE, adding(BASE, set, testid.switchPosition("u2")), UNIT)).toBe(true);
    });
  }

  it("is false when one of the card's Activate controls joins legal, selected or glow", () => {
    for (const set of ["legal", "selected", "glow"] as const) {
      expect(sameHighlightAt(BASE, adding(BASE, set, "activate-u1-x"), UNIT)).toBe(false);
      expect(sameHighlightAt(BASE, adding(BASE, set, testid.activate("u1")), UNIT)).toBe(false);
      expect(sameHighlightAt(adding(BASE, set, "activate-u1-x"), BASE, UNIT)).toBe(false);
      expect(sameHighlightAt(BASE, adding(BASE, set, "activate-u2-x"), UNIT)).toBe(true);
    }
  });

  it("is false when the card's Activate ability changes within a set that already holds another", () => {
    const one = adding(BASE, "legal", "activate-u1-x");
    const other = adding(BASE, "legal", "activate-u1-y");
    expect(sameHighlightAt(one, other, UNIT)).toBe(false);
    expect(sameHighlightAt(one, adding(BASE, "legal", "activate-u1-x"), UNIT)).toBe(true);
  });

  it("is true for a hovered attack that names other cards, and false for one that names this card", () => {
    const elsewhere: Highlight = { ...BASE, hoveredAttack: { attacker: "card-u2", target: "card-e1", strikesBack: true } };
    expect(sameHighlightAt(BASE, elsewhere, UNIT)).toBe(true);
    expect(sameHighlightAt(elsewhere, BASE, UNIT)).toBe(true);

    const target: Highlight = { ...BASE, hoveredAttack: { attacker: "card-u2", target: "card-u1", strikesBack: true } };
    expect(sameHighlightAt(BASE, target, UNIT)).toBe(false);
    expect(sameHighlightAt(target, BASE, UNIT)).toBe(false);

    const attacker: Highlight = { ...BASE, hoveredAttack: { attacker: "card-u1", target: "card-e1", strikesBack: true } };
    expect(sameHighlightAt(BASE, attacker, UNIT)).toBe(false);
    expect(sameHighlightAt(attacker, BASE, UNIT)).toBe(false);
  });

  it("is false when the hovered attack on this card changes who strikes it or whether it strikes back", () => {
    const first: Highlight = { ...BASE, hoveredAttack: { attacker: "card-u1", target: "card-e1", strikesBack: true } };
    const moved: Highlight = { ...BASE, hoveredAttack: { attacker: "card-u1", target: "card-e2", strikesBack: true } };
    const quiet: Highlight = { ...BASE, hoveredAttack: { attacker: "card-u1", target: "card-e1", strikesBack: false } };
    expect(sameHighlightAt(first, moved, UNIT)).toBe(false);
    expect(sameHighlightAt(first, quiet, UNIT)).toBe(false);
    expect(sameHighlightAt(first, { ...first, hoveredAttack: { ...first.hoveredAttack! } }, UNIT)).toBe(true);
  });

  it("is true when the hovered attack moves between two other cards", () => {
    const a: Highlight = { ...BASE, hoveredAttack: { attacker: "card-u2", target: "card-e1", strikesBack: true } };
    const b: Highlight = { ...BASE, hoveredAttack: { attacker: "card-u2", target: "card-e2", strikesBack: false } };
    expect(sameHighlightAt(a, b, UNIT)).toBe(true);
  });

  it("is false when the forecast at the card's testid changes, and true for another's", () => {
    const own = forecast(3);
    const withOwn: Highlight = { ...BASE, outcomes: new Map([["card-u1", own]]) };
    const elsewhere: Highlight = { ...BASE, outcomes: new Map([["card-e1", forecast(2)]]) };
    expect(sameHighlightAt(BASE, elsewhere, UNIT)).toBe(true);
    expect(sameHighlightAt(BASE, withOwn, UNIT)).toBe(false);
    expect(sameHighlightAt(withOwn, { ...BASE, outcomes: new Map([["card-u1", forecast(3)]]) }, UNIT)).toBe(false);
    expect(sameHighlightAt(withOwn, { ...BASE, outcomes: new Map([["card-u1", own], ["card-e1", forecast(1)]]) }, UNIT)).toBe(true);
  });

  it("is false when the blow the hovering attacker takes back changes", () => {
    const hover = { attacker: "card-u1", target: "card-e1", strikesBack: true };
    const before: Highlight = { ...BASE, hoveredAttack: hover, outcomes: new Map([["card-e1", forecast(1)]]) };
    const after: Highlight = { ...BASE, hoveredAttack: hover, outcomes: new Map([["card-e1", forecast(2)]]) };
    expect(sameHighlightAt(before, after, UNIT)).toBe(false);
    const other: Highlight = { ...BASE, hoveredAttack: { attacker: "card-u2", target: "card-e1", strikesBack: true }, outcomes: after.outcomes! };
    expect(sameHighlightAt({ ...other, outcomes: before.outcomes! }, other, UNIT)).toBe(true);
  });

  it("counts a card with no testid and no instance id (a back) as seeing nothing", () => {
    const back: CardReads = { testId: undefined, instanceId: undefined };
    const busy: Highlight = { ...adding(adding(BASE, "legal", "card-u1"), "selected", "card-u1"), hoveredAttack: { attacker: "card-u1", target: "card-e1", strikesBack: true } };
    expect(sameHighlightAt(BASE, busy, back)).toBe(true);
  });
});
