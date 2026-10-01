// R280 (SPEC §10.8, §10.9, §10.10): the number a Core card's formula comes to now, as `viewFor`
// carries it (`preview`), for the six Core cards whose text computes one from the board by more than
// a plain X. Each test reads the value off `s.view(...)` and then lets the card resolve, so the value
// is proved against what the card's own resolution does, not against a second copy of the formula.
//
// What each formula may read (R280), stated here and proved by the "reads nothing hidden" test at
// the end, which makes every library and every hand throw on access:
//
//   #18 Bread and Butter        the active player's current mana (public, §10.8)
//   #31 KY's Math Equation      its own count of plays (the card's own instance, R429)
//   #38 Quickstriker            its controller's count of plays this turn (public)
//   #40 Echoes of the Forgotten its controller's exile count (public, §3)
//   #70 Spiteful Stab           its controller's hero health and exile count (public)
//   #91 Fed Fauci               its own Plague Tokens (its counters travel on its view, §10.8)
//   C #19 Lizard's Breath       its controller's deck, graveyard and exile SIZES (public, §10.8), never
//                               their contents — so it has its own proof below rather than the fence
//                               at the end, which walls a library off whole
//
// None reads a library's contents or order, or a hand's contents: the value shows to every viewer
// who may read the card — the other seat too, for a card on the field — so it must say nothing more
// than the card and the public board do. And the view shows it only where the viewer may read the
// card: a #31 in the opponent's hand, or the opponent's face-down #18, carries none (§10.8).
//
// Not previewed, on purpose (R280): #92's stats, #100's cost and #89's hand stats are on the face
// already, and #24's and #74's X is chosen at play. The set test pins the six.

import { createRng, stepParam, subsystems, type CardInstance, type ConditionContext, type GameState } from "@jackioh/engine";
import type { CardView, PlayerId, PlayerView, PreviewValue } from "@jackioh/shared";
import { describe, expect, it } from "vitest";
import { CARDS, cardDef } from "../src/index";
import { scenario, type Scenario, type ScenarioOptions } from "./_harness";

const BREAD_AND_BUTTER = "core-018";
const MATH_EQUATION = "core-031";
const QUICKSTRIKER = "core-038";
const ECHOES = "core-040";
const SPITEFUL_STAB = "core-070";
const FED_FAUCI = "core-091";
const COMBO_INDEX = "core-093";

/** R280's six, in index order. */
const PREVIEWED = [BREAD_AND_BUTTER, MATH_EQUATION, QUICKSTRIKER, ECHOES, SPITEFUL_STAB, FED_FAUCI];

/** Patch v0.2.0's Classic cards that declare one (R280), each proved in its own block below. */
const CURSE = "classic-001"; // C #1 Curse of the Forgotten Classic
const PLAGUE_NUKE = "classic-043"; // C #43 Plague Nuke
const LIZARDS_BREATH = "classic-019"; // C #19 Lizard's Breath
const PLAGUE_DOCTOR = "classic-059"; // C #59 Plague Doctor
const SIPHON_SQUAD = "classic-088"; // C #88 Siphon Squad
const CLASSIC_PREVIEWED = [CURSE, LIZARDS_BREATH, PLAGUE_NUKE, PLAGUE_DOCTOR, SIPHON_SQUAD];

const RAPID_REPLENISH = "core-010"; // 0-cost Spell; Combo 3, so nothing at one play — a free anchor
const TEMPO_TIMMY = "core-011"; // 1-cost Unit
const BIG_D_FENDER = "core-001"; // 2-cost Unit
const MENACE = "core-019"; // library filler, and a 9/9 target that survives
const STOCKPILE = "core-005";

const AT_ENEMY_HERO = [{ pick: "hero", player: "p2" }] as const;

type Face = "base" | "radiant";
const FACES: readonly Face[] = ["base", "radiant"];

function ownHand(view: PlayerView): CardView[] {
  const hand = view.you.hand;
  if (!Array.isArray(hand)) throw new Error("the viewer's own hand must travel in full (§10.8)");
  return hand;
}

function handCard(view: PlayerView, instanceId: string): CardView {
  const found = ownHand(view).find((card) => card.instanceId === instanceId);
  if (found === undefined) throw new Error(`${instanceId} is not in the viewer's hand`);
  return found;
}

/** The list the view carries, or null when the key is absent; a key holding `[]` fails here. */
function shown(card: object | null | undefined): PreviewValue[] | null {
  if (card === null || card === undefined) throw new Error("no card at that place in the view");
  if (!("preview" in card)) return null;
  const list = (card as { preview?: unknown }).preview;
  expect(Array.isArray(list) && list.length > 0, "preview is absent rather than empty").toBe(true);
  return list as PreviewValue[];
}

/** The one value a card's view carries, failing unless there is exactly one. */
function valueOf(card: object | null | undefined): number {
  const list = shown(card);
  expect(list, "the card carries a preview").not.toBeNull();
  expect(list).toHaveLength(1);
  return list?.[0]?.value ?? Number.NaN;
}

/** The amounts of every `damage` event on a hero so far, in order: one entry per hit. */
function hitsOn(s: Scenario, player: PlayerId): number[] {
  return s.events.flatMap((event) =>
    event.type === "damage" && event.targetId === `hero-${player}` ? [event.amount] : [],
  );
}

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`missing: ${what}`);
  return value;
}

// =============================================================================================
// The set, and the labels
// =============================================================================================

describe("R280 the Core cards that declare preview", () => {
  // R372 added #93 Combo-Index, whose grade is a counter on the card in play, so its hook answers
  // on the field only; the hand-based tests below keep to the six, and 093-combo-index.test.ts
  // proves its values.
  it("R280 R372 are exactly #18, #31, #38, #40, #70, #91 and #93, and the Classic cards listed, on both faces", () => {
    const hooked = Object.entries(CARDS)
      .filter(([, card]) => card.base.preview !== undefined || card.radiant.preview !== undefined)
      .map(([id]) => id)
      .sort();
    expect(hooked).toEqual([...PREVIEWED, COMBO_INDEX, ...CLASSIC_PREVIEWED].sort());
    for (const id of hooked) {
      expect(CARDS[id]?.base.preview, `${id} base`).toBeTypeOf("function");
      expect(CARDS[id]?.radiant.preview, `${id} radiant`).toBeTypeOf("function");
    }
  });

  it("R280 each label is an exact substring of the running face's catalog text, on both faces", () => {
    for (const id of PREVIEWED) {
      for (const face of FACES) {
        const s = scenario({ p1: { hand: [{ def: id, radiant: face === "radiant" }, RAPID_REPLENISH] } });
        const list = must(shown(handCard(s.view("p1"), s.card(id).id)), `${id} ${face}'s preview in hand`);
        const text = cardDef(id)[face].text;
        for (const entry of list) {
          expect(text, `${id} ${face}: "${entry.label}"`).toContain(entry.label);
        }
      }
    }
  });

  it("R280 the labels are the formulas each face prints", () => {
    const labels = (id: string, face: Face): string[] => {
      const s = scenario({ p1: { hand: [{ def: id, radiant: face === "radiant" }, RAPID_REPLENISH] } });
      return (shown(handCard(s.view("p1"), s.card(id).id)) ?? []).map((entry) => entry.label);
    };
    expect(labels(BREAD_AND_BUTTER, "base")).toEqual(["X = that player's unspent mana"]);
    expect(labels(BREAD_AND_BUTTER, "radiant")).toEqual(["X = 3 × that player's unspent mana"]);
    expect(labels(MATH_EQUATION, "base")).toEqual(["Fib(times played + 1)"]);
    expect(labels(MATH_EQUATION, "radiant")).toEqual(["Fib(times played + 3)"]);
    expect(labels(QUICKSTRIKER, "base")).toEqual(["X = cards you played earlier this turn"]);
    expect(labels(QUICKSTRIKER, "radiant")).toEqual(["X = cards you played earlier this turn"]);
    expect(labels(ECHOES, "base")).toEqual(["the cards in your exile"]);
    expect(labels(ECHOES, "radiant")).toEqual(["twice the cards in your exile"]);
    expect(labels(SPITEFUL_STAB, "base")).toEqual([cardDef(SPITEFUL_STAB).base.text]);
    expect(labels(SPITEFUL_STAB, "radiant")).toEqual([cardDef(SPITEFUL_STAB).radiant.text]);
    expect(labels(FED_FAUCI, "base")).toEqual(["+1 mana per Plague Token"]);
    expect(labels(FED_FAUCI, "radiant")).toEqual(["+2 mana per Plague Token"]);
  });
});

// =============================================================================================
// #18 Bread and Butter: the active player's current mana
// =============================================================================================

describe("#18 Bread and Butter previews the Bread Token's X (R280)", () => {
  function trap(face: Face, opts: { active?: PlayerId; p1Mana?: number; p2Mana?: number; faceUp?: boolean } = {}): Scenario {
    return scenario({
      active: opts.active ?? "p1",
      p1: {
        backrow: [{ def: BREAD_AND_BUTTER, radiant: face === "radiant", ...(opts.faceUp === true ? { faceUp: true } : {}) }],
        field: ["core-012"],
        library: [RAPID_REPLENISH],
        ...(opts.p1Mana === undefined ? {} : { mana: opts.p1Mana }),
      },
      p2: {
        field: ["core-012"],
        library: [RAPID_REPLENISH],
        ...(opts.p2Mana === undefined ? {} : { mana: opts.p2Mana }),
      },
    });
  }

  it("R280 base: 3 unspent mana on its controller's turn previews 3, and the turn's end makes a 3/3", () => {
    const s = trap("base", { p1Mana: 3 });
    expect(valueOf(s.view("p1").you.backrow[0])).toBe(3);

    s.endTurn();

    const token = must(s.unit("p1", 2), "the Bread Token");
    expect(token.defId).toBe("core-t-bread");
    s.expectStats(token, { attack: 3, health: 3 });
  });

  it("R280 radiant: 2 unspent mana previews 3 × 2 = 6, and the turn's end makes a 6/6", () => {
    const s = trap("radiant", { p1Mana: 2 });
    expect(valueOf(s.view("p1").you.backrow[0])).toBe(6);

    s.endTurn();

    s.expectStats(must(s.unit("p1", 2), "the Bread Token"), { attack: 6, health: 6 });
  });

  it("R280 on the opponent's turn it reads the active player's mana, not its controller's", () => {
    const s = trap("base", { active: "p2", p1Mana: 1, p2Mana: 4 });
    expect(valueOf(s.view("p1").you.backrow[0])).toBe(4);

    s.endTurn();

    // R52: p2 ended with 4, and the 4/4 is the trap's controller's.
    s.expectStats(must(s.unit("p1", 2), "the Bread Token"), { attack: 4, health: 4 });
  });

  it("R280 face-down, only its controller sees it; the other seat's view carries nothing of it", () => {
    const s = trap("base", { p1Mana: 3 });

    expect(valueOf(s.view("p1").you.backrow[0])).toBe(3);
    const theirs = s.view("p2");
    expect(theirs.opponent.backrow[0]).toEqual({ faceDown: true, cost: 1 });
    expect(JSON.stringify(theirs)).not.toContain("unspent mana");
  });

  it("R280 face-up (fired), it is public, and both seats see the same number", () => {
    const s = trap("radiant", { p1Mana: 1, faceUp: true });

    expect(valueOf(s.view("p1").you.backrow[0])).toBe(3);
    expect(valueOf(s.view("p2").opponent.backrow[0])).toBe(3);
  });
});

// =============================================================================================
// #31 KY's Math Equation: its own count of plays (R429)
// =============================================================================================

describe("#31 KY's Math Equation previews its damage (R280)", () => {
  function equation(face: Face, timesPlayed: number, costMod = 0): Scenario {
    const s = scenario({
      p1: {
        hand: [{ def: MATH_EQUATION, radiant: face === "radiant", costMod }, RAPID_REPLENISH],
        mana: 10,
      },
      p2: { field: [MENACE], hand: [MATH_EQUATION] },
    });
    // R429: the plays it has had before, which the harness cannot seed.
    must(s.hand("p1").find((c) => c.defId === MATH_EQUATION), "p1's Equation").timesPlayed = timesPlayed;
    return s;
  }

  const CASES: readonly [Face, number, number, number][] = [
    ["base", 0, 0, 1], // its 1st play: Fib(1 + 1)
    ["base", 2, 0, 3], // its 3rd play: Fib(3 + 1)
    ["base", 0, 3, 1], // R67: a (4) Equation's 1st play still deals Fib(1 + 1)
    ["radiant", 0, 0, 3], // Fib(1 + 3)
    ["radiant", 1, 0, 5], // Fib(2 + 3)
  ];

  for (const [face, timesPlayed, costMod, damage] of CASES) {
    it(`R280 R429 ${face}, played ${timesPlayed} times before, costMod ${costMod}: previews ${damage}, and deals exactly that`, () => {
      const s = equation(face, timesPlayed, costMod);
      const card = must(s.hand("p1").find((c) => c.defId === MATH_EQUATION), "p1's Equation");
      expect(valueOf(handCard(s.view("p1"), card.id))).toBe(damage);

      s.play(card, { targets: AT_ENEMY_HERO });

      expect(hitsOn(s, "p2")).toEqual([damage]);
    });
  }

  it("R280 R67 a player discount changes the price, not the preview", () => {
    const s = equation("base", 0);
    const card = must(s.hand("p1").find((c) => c.defId === MATH_EQUATION), "p1's Equation");
    s.state.players.p1.mods.push({
      id: "test-spell-discount",
      kind: "costDiscount",
      amount: 1,
      onlyType: "Spell",
      expiry: { until: "never" },
    });

    expect(valueOf(handCard(s.view("p1"), card.id))).toBe(1);
  });

  it("R280 in the opponent's hand it is hidden: p1's view carries no preview at all, p2's own does", () => {
    const s = equation("base", 2);

    const p1View = s.view("p1");
    expect(p1View.opponent.hand).toEqual({ count: 1 });
    // p1's own Equation carries one; nothing else in p1's view does, least of all p2's hand card.
    expect(JSON.stringify(p1View).match(/"preview"/g)).toHaveLength(1);

    const theirs = must(s.hand("p2")[0], "p2's Equation");
    expect(valueOf(handCard(s.view("p2"), theirs.id))).toBe(1);
  });
});

// =============================================================================================
// #38 Quickstriker: its controller's plays this turn
// =============================================================================================

describe("#38 Quickstriker previews X, the count the next play reads (R280)", () => {
  for (const face of FACES) {
    it(`R280 ${face}: X is 0, then 1, then 2 as the turn goes, and the next play's Combo reads it`, () => {
      const s = scenario({
        p1: {
          backrow: [{ def: QUICKSTRIKER, radiant: face === "radiant" }],
          hand: [RAPID_REPLENISH, TEMPO_TIMMY, BIG_D_FENDER, STOCKPILE],
          library: [MENACE, MENACE, MENACE, MENACE],
          mana: 10,
        },
        p2: { hand: [STOCKPILE], library: [MENACE] },
      });
      const multiple = face === "radiant" ? 2 : 1;

      expect(valueOf(s.view("p1").you.backrow[0])).toBe(0);
      s.play(RAPID_REPLENISH);
      const x1 = valueOf(s.view("p1").you.backrow[0]);
      expect(x1).toBe(1);
      s.play(TEMPO_TIMMY);
      const x2 = valueOf(s.view("p1").you.backrow[0]);
      expect(x2).toBe(2);
      s.play(BIG_D_FENDER);

      // The same X on both faces; the Radiant face deals it twice over, as one hit (R281).
      expect(hitsOn(s, "p2")).toEqual([x1 * multiple, x2 * multiple]);
      // It is public: the other seat sees the same X.
      expect(valueOf(s.view("p2").opponent.backrow[0])).toBe(3);
    });
  }

  it("R280 in hand it previews the X the plays so far give", () => {
    const s = scenario({ p1: { hand: [RAPID_REPLENISH, QUICKSTRIKER, TEMPO_TIMMY], library: [MENACE, MENACE] } });
    const card = s.card(QUICKSTRIKER);
    expect(valueOf(handCard(s.view("p1"), card.id))).toBe(0);
    s.play(RAPID_REPLENISH).play(TEMPO_TIMMY);
    expect(valueOf(handCard(s.view("p1"), card.id))).toBe(2);
  });
});

// =============================================================================================
// #40 Echoes of the Forgotten: its controller's exile count
// =============================================================================================

describe("#40 Echoes of the Forgotten previews its start-of-turn damage (R280)", () => {
  for (const [face, expected] of [
    ["base", 3],
    ["radiant", 6],
  ] as const) {
    it(`R280 ${face}: three cards in your exile preview ${expected}, and the start of your turn deals it`, () => {
      const s = scenario({
        p1: {
          backrow: [{ def: ECHOES, radiant: face === "radiant" }],
          exile: [STOCKPILE, TEMPO_TIMMY, MENACE],
          library: [MENACE, TEMPO_TIMMY],
        },
        p2: { exile: [STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE, STOCKPILE] },
      });

      expect(valueOf(s.view("p1").you.backrow[0])).toBe(expected);
      // A public Field Spell: the other seat sees the same number, and the opponent's pile of 5 is
      // not counted (R72).
      expect(valueOf(s.view("p2").opponent.backrow[0])).toBe(expected);

      s.startTurn();

      expect(hitsOn(s, "p2")).toEqual([expected]);
    });
  }
});

// =============================================================================================
// #70 Spiteful Stab: its controller's hero health and exile count
// =============================================================================================

describe("#70 Spiteful Stab previews its damage (R280)", () => {
  function stab(face: Face, opts: ScenarioOptions["p1"] = {}): Scenario {
    return scenario({
      p1: { ...opts, hand: [{ def: SPITEFUL_STAB, radiant: face === "radiant" }, RAPID_REPLENISH] },
      p2: { field: [MENACE], exile: [STOCKPILE, STOCKPILE, STOCKPILE] },
    });
  }

  const CASES: readonly [Face, number, number, number][] = [
    ["base", 30, 0, 2],
    ["base", 23, 2, 5], // 2 + floor(7/5) + 2
    ["base", 35, 1, 3], // R72: missing counts from 30, floored at 0
    ["radiant", 30, 0, 4],
    ["radiant", 23, 2, 10], // 4 + floor(7/3) + 2 × 2
    ["radiant", 21, 1, 9], // 4 + 3 + 2
  ];

  for (const [face, health, exiled, damage] of CASES) {
    it(`R280 ${face} at ${health} health with ${exiled} exiled previews ${damage}, and deals exactly that`, () => {
      const s = stab(face, { health, exile: Array.from({ length: exiled }, () => STOCKPILE) });
      expect(valueOf(handCard(s.view("p1"), s.card(SPITEFUL_STAB).id))).toBe(damage);

      s.play(SPITEFUL_STAB, { targets: AT_ENEMY_HERO });

      expect(hitsOn(s, "p2")).toEqual([damage]);
    });
  }
});

// =============================================================================================
// #91 Fed Fauci: its own Plague Tokens
// =============================================================================================

describe("#91 Fed Fauci previews the mana its next start of turn gives (R280)", () => {
  for (const [face, expected] of [
    ["base", 2],
    ["radiant", 4],
  ] as const) {
    it(`R280 ${face}: two Plague Tokens preview ${expected}, and its controller's next turn starts with that much more`, () => {
      const s = scenario({
        active: "p2",
        p1: {
          field: [{ def: FED_FAUCI, radiant: face === "radiant", counters: { plague: 2 } }],
          hand: [STOCKPILE],
          library: [MENACE, MENACE],
        },
        p2: { hand: [STOCKPILE], library: [MENACE, MENACE] },
      });

      expect(valueOf(s.view("p1").you.units[0])).toBe(expected);
      // A unit is public: the other seat sees the same number.
      expect(valueOf(s.view("p2").opponent.units[0])).toBe(expected);

      s.endTurn(); // p2 ends; p1's turn starts: the refresh, then the hook (R62)

      s.expectMana("p1", 4 + expected);
    });
  }

  it("R280 in hand it holds no tokens (R78), so it previews 0", () => {
    const s = scenario({ p1: { hand: [FED_FAUCI, RAPID_REPLENISH] } });
    expect(valueOf(handCard(s.view("p1"), s.card(FED_FAUCI).id))).toBe(0);
  });
});

// =============================================================================================
// A fused card's list (R102)
// =============================================================================================

describe("R280 a fused Core card lists its ingredients' previews in order", () => {
  it("R280 a crafted Equation + Stab previews both numbers, and its Cry deals their sum", () => {
    const s = scenario({
      p1: { hand: [MATH_EQUATION, SPITEFUL_STAB, RAPID_REPLENISH], health: 20, exile: [STOCKPILE], mana: 10 },
      p2: { field: [MENACE] },
    });
    const sink = { state: s.state, events: [], rng: createRng(s.state.seed, s.state.rngCursor) };
    const fused = must(
      subsystems.fuse(sink, { ingredients: [s.card(MATH_EQUATION), s.card(SPITEFUL_STAB)], toHand: "p1" }),
      "the crafted card",
    );

    const list = must(shown(handCard(s.view("p1"), fused.id)), "the fused card's preview");
    // #31's half reads the fused card's own count of plays, none yet (R429): its 1st play, Fib(0 + 1 +
    // 1) = 1. #70's half reads 20 health (missing 10: +2) and one exiled card: 2 + 2 + 1 = 5.
    expect(list).toEqual([
      { label: "Fib(times played + 1)", value: 1 },
      { label: cardDef(SPITEFUL_STAB).base.text, value: 5 },
    ]);
    const text = must(s.state.transientDefs[fused.defId], "the fused def").base.text;
    for (const entry of list) expect(text).toContain(entry.label);

    s.play(fused, { targets: [...AT_ENEMY_HERO, ...AT_ENEMY_HERO] });

    expect(hitsOn(s, "p2")).toEqual(list.map((entry) => entry.value));
  });
});

// =============================================================================================
// Classic #19 Lizard's Breath: the pile or piles that count now (R280)
// =============================================================================================

describe("C #19 Lizard's Breath previews the pile or piles that would count now (R280)", () => {
  const X = "core-008";

  function breath(face: Face, deck: number, graveyard: number, exile: number, extra: Partial<ScenarioOptions> = {}): Scenario {
    return scenario({
      ...extra,
      p1: {
        hand: [{ def: LIZARDS_BREATH, radiant: face === "radiant" }, RAPID_REPLENISH],
        library: Array.from({ length: deck }, () => X),
        graveyard: Array.from({ length: graveyard }, () => STOCKPILE),
        exile: Array.from({ length: exile }, () => STOCKPILE),
      },
      p2: { hand: [STOCKPILE] },
    });
  }

  function preview(s: Scenario): PreviewValue[] | null {
    return shown(handCard(s.view("p1"), s.card(LIZARDS_BREATH).id));
  }

  function draws(s: Scenario): number {
    return s.events.filter((event) => event.type === "drawn").length;
  }

  it("R280 base: the Deck largest names the Deck with its size, and the play then draws 1 after a hit of 2", () => {
    const s = breath("base", 5, 2, 1);
    expect(preview(s)).toEqual([{ label: "Your largest pile", value: 5, display: "Deck" }]);

    s.play(LIZARDS_BREATH, { targets: AT_ENEMY_HERO });

    expect(hitsOn(s, "p2")).toEqual([2]);
    expect(draws(s)).toBe(1);
  });

  it("R280 base: the Exile largest names the Exile, and the play then hits for 6", () => {
    const s = breath("base", 1, 2, 3);
    expect(preview(s)).toEqual([{ label: "Your largest pile", value: 3, display: "Exile" }]);

    s.play(LIZARDS_BREATH, { targets: AT_ENEMY_HERO });

    expect(hitsOn(s, "p2")).toEqual([6]);
  });

  it("R280 base: a Graveyard–Exile tie names the Graveyard, listed first, and the play gives 2 mana", () => {
    const s = breath("base", 0, 2, 2);
    expect(preview(s)).toEqual([{ label: "Your largest pile", value: 2, display: "Graveyard" }]);

    s.play(LIZARDS_BREATH, { targets: AT_ENEMY_HERO });

    s.expectMana("p1", 5);
  });

  it("R280 base: three empty piles name the Deck at 0", () => {
    expect(preview(breath("base", 0, 0, 0))).toEqual([{ label: "Your largest pile", value: 0, display: "Deck" }]);
  });

  it("R280 radiant: names the two largest in rank order, and the play then does both", () => {
    const s = breath("radiant", 1, 3, 5);
    expect(preview(s)).toEqual([
      { label: "Your two largest piles", value: 5, display: "Exile" },
      { label: "Your two largest piles", value: 3, display: "Graveyard" },
    ]);

    s.play(LIZARDS_BREATH, { targets: AT_ENEMY_HERO });

    expect(hitsOn(s, "p2")).toEqual([8]);
    s.expectMana("p1", 5);
    expect(draws(s)).toBe(0);
  });

  it("R280 radiant: three equal piles name the Deck and the Graveyard", () => {
    expect((preview(breath("radiant", 2, 2, 2)) ?? []).map((entry) => entry.display)).toEqual(["Deck", "Graveyard"]);
  });

  it("R280 each label is an exact substring of its face's text, with no placeholder in it", () => {
    for (const face of FACES) {
      const text = cardDef(LIZARDS_BREATH)[face].text;
      for (const entry of preview(breath(face, 3, 2, 1)) ?? []) {
        expect(text).toContain(entry.label);
        expect(entry.label).not.toMatch(/[{}]/);
      }
    }
  });

  it("R280 it follows the piles as they change: a pile growing past another moves the preview", () => {
    const s = scenario({
      p1: { hand: [LIZARDS_BREATH, RAPID_REPLENISH, STOCKPILE], library: [X, X, X], graveyard: [STOCKPILE, STOCKPILE] },
      p2: { hand: [STOCKPILE] },
    });
    expect(preview(s)?.[0]?.display).toBe("Deck");

    // Stockpile draws 2 (deck 3 → 1) and lands in the graveyard (2 → 3).
    s.play(STOCKPILE);

    expect(preview(s)).toEqual([{ label: "Your largest pile", value: 3, display: "Graveyard" }]);
  });

  it("R280 in the opponent's hand it is hidden: p2's view carries no preview of it", () => {
    const s = breath("base", 3, 1, 1);

    expect(JSON.stringify(s.view("p2"))).not.toContain("Your largest pile");
  });

  it("R280 it reads only pile sizes: two games whose decks and hands differ but whose sizes match show the same preview", () => {
    const one = scenario({
      p1: { hand: [LIZARDS_BREATH, STOCKPILE], library: [X, X, X], graveyard: [STOCKPILE], exile: [STOCKPILE] },
      p2: { hand: [STOCKPILE], library: [X] },
    });
    const other = scenario({
      p1: { hand: [LIZARDS_BREATH, MENACE], library: [TEMPO_TIMMY, BIG_D_FENDER, RAPID_REPLENISH], graveyard: [STOCKPILE], exile: [STOCKPILE] },
      p2: { hand: [MATH_EQUATION], library: [BIG_D_FENDER] },
    });

    expect(preview(other)).toEqual(preview(one));
  });

  it("R280 the hook is a pure read: no write, no state.active, and of a library or a hand only its size", () => {
    for (const face of FACES) {
      const s = breath(face, 3, 2, 1);
      const card = must(s.hand("p1").find((c) => c.defId === LIZARDS_BREATH), "p1's Breath");
      const scripts = must(CARDS[LIZARDS_BREATH], LIZARDS_BREATH);
      const hook = must((card.radiant ? scripts.radiant : scripts.base).preview, "the Breath's hook");
      const copy = structuredClone(s.state);
      const self = must(copy.players.p1.hand.find((c) => c.id === card.id), "the Breath in the copy");
      // A pile that answers its length and throws on anything else: the sizes are public, the cards are not.
      const sizeOnly = (pile: CardInstance[], what: string): CardInstance[] =>
        new Proxy(pile, {
          get: (target, key) => {
            if (key === "length") return target.length;
            throw new Error(`a preview read ${what}[${String(key)}]`);
          },
        });
      for (const player of ["p1", "p2"] as const) {
        copy.players[player].library = sizeOnly(copy.players[player].library, `${player}'s library`);
        copy.players[player].hand = sizeOnly(copy.players[player].hand, `${player}'s hand`);
      }
      Object.defineProperty(copy, "active", {
        get: () => {
          throw new Error("a preview read state.active");
        },
      });
      deepFreeze(copy);

      const answer = hook({ state: copy, self, controller: "p1", radiant: card.radiant, zone: "hand", yourTurn: true });

      expect(shown(handCard(s.view("p1"), card.id))).toEqual(answer);
      expect(answer.map((entry) => entry.display)).toEqual(face === "base" ? ["Deck"] : ["Deck", "Graveyard"]);
    }
  });
});

// =============================================================================================
// A read of what the controller may read, and nothing else
// =============================================================================================

/** Freeze a JSON tree in place, data properties only (an accessor below is left alone). */
function deepFreeze(value: unknown): void {
  if (typeof value !== "object" || value === null || Object.isFrozen(value)) return;
  for (const descriptor of Object.values(Object.getOwnPropertyDescriptors(value))) {
    if ("value" in descriptor) deepFreeze(descriptor.value);
  }
  Object.freeze(value);
}

/**
 * A copy of `state` that throws on any write, and on any read of a library, a hand or
 * `state.active` (a hook reads `yourTurn` instead, README §1). `self` is found in the copy before
 * its zone is fenced off, so a hand card's hook still has itself.
 */
function guarded(state: GameState, selfId: string): { state: GameState; self: CardInstance } {
  const copy = structuredClone(state);
  const all = (["p1", "p2"] as const).flatMap((player) => [
    ...copy.players[player].hand,
    ...copy.players[player].units.flatMap((pile) => pile ?? []),
    ...copy.players[player].backrow.flatMap((card) => (card === null ? [] : [card])),
  ]);
  const self = must(all.find((card) => card.id === selfId), `${selfId} in the copy`);
  deepFreeze(self);
  for (const player of ["p1", "p2"] as const) {
    for (const zone of ["library", "hand"] as const) {
      Object.defineProperty(copy.players[player], zone, {
        get: () => {
          throw new Error(`a preview read ${player}'s ${zone}`);
        },
      });
    }
  }
  Object.defineProperty(copy, "active", {
    get: () => {
      throw new Error("a preview read state.active");
    },
  });
  deepFreeze(copy);
  return { state: copy, self };
}

describe("R280 each Core hook is a pure read of public facts", () => {
  /** Every previewed card, both faces, in hand and — for a permanent — on the field, on either turn. */
  function placements(): { name: string; opts: ScenarioOptions; id: string; zone: "hand" | "field" }[] {
    const out: { name: string; opts: ScenarioOptions; id: string; zone: "hand" | "field" }[] = [];
    for (const id of PREVIEWED) {
      for (const radiant of [false, true]) {
        for (const active of ["p1", "p2"] as const) {
          const common = {
            active,
            p2: { hand: [MATH_EQUATION, STOCKPILE], library: [MENACE, TEMPO_TIMMY], exile: [STOCKPILE] },
          };
          out.push({
            name: `${id} ${radiant ? "radiant" : "base"} in hand, ${active} active`,
            opts: { ...common, p1: { hand: [{ def: id, radiant }], library: [MENACE], exile: [STOCKPILE], health: 17 } },
            id,
            zone: "hand",
          });
          if (id === MATH_EQUATION || id === SPITEFUL_STAB) continue;
          const permanent = id === FED_FAUCI ? { field: [{ def: id, radiant, counters: { plague: 3 } }] } : { backrow: [{ def: id, radiant }] };
          out.push({
            name: `${id} ${radiant ? "radiant" : "base"} on the field, ${active} active`,
            opts: { ...common, p1: { ...permanent, library: [MENACE], exile: [STOCKPILE], health: 17 } },
            id,
            zone: "field",
          });
        }
      }
    }
    return out;
  }

  it("R280 no hook writes, or reads a library, a hand or state.active, and each answers as the view shows", () => {
    for (const { name, opts, id, zone } of placements()) {
      const s = scenario(opts);
      // p1's copy: p2 holds an Equation too, and `s.card` would look at the active side first.
      const card = must(
        zone === "hand" ? s.hand("p1").find((c) => c.defId === id) : (s.backrow("p1", 1) ?? s.unit("p1", 1)),
        `p1's ${id}`,
      );
      const scripts = must(CARDS[id], id);
      const hook = must((card.radiant ? scripts.radiant : scripts.base).preview, `${id}'s hook`);
      const { state, self } = guarded(s.state, card.id);
      const ctx: ConditionContext = {
        state,
        self,
        controller: card.controller,
        radiant: card.radiant,
        zone,
        yourTurn: s.state.active === card.controller,
      };

      const answer = hook(ctx);

      const view = s.view("p1");
      const place = zone === "hand" ? handCard(view, card.id) : (view.you.backrow[0] ?? view.you.units[0]);
      expect(shown(place), name).toEqual(answer);
    }
  });

  it("R280 two games that differ only in cards a viewer may not read show that viewer the same previews", () => {
    type Hand = NonNullable<NonNullable<ScenarioOptions["p1"]>["hand"]>;
    const board = (library: readonly string[], p1Hand: Hand, p2Hand: Hand): Scenario =>
      scenario({
        p1: {
          hand: [...p1Hand, SPITEFUL_STAB, FED_FAUCI],
          backrow: [BREAD_AND_BUTTER, QUICKSTRIKER, { def: ECHOES, radiant: true }],
          field: [{ def: FED_FAUCI, counters: { plague: 2 } }],
          library,
          exile: [STOCKPILE, STOCKPILE],
          health: 18,
          mana: 3,
        },
        p2: {
          hand: [...p2Hand, MATH_EQUATION],
          library,
          backrow: [{ def: BREAD_AND_BUTTER, radiant: true }, QUICKSTRIKER],
          exile: [STOCKPILE],
        },
      });
    const previews = (s: Scenario, viewer: PlayerId): (PreviewValue[] | null)[] => {
      const view = s.view(viewer);
      return [
        ...ownHand(view).map((card) => shown(card)),
        ...[view.you, view.opponent].flatMap((side) =>
          [...side.units, ...side.backrow].map((card) => (card === null ? null : shown(card))),
        ),
      ];
    };

    const one = board([MENACE, TEMPO_TIMMY, STOCKPILE], [{ def: MATH_EQUATION, costMod: 1 }], [STOCKPILE]);
    // p1 may not read p2's hand or either library: change all three, and p1's view is the same.
    const forP1 = board([BIG_D_FENDER, BIG_D_FENDER, RAPID_REPLENISH], [{ def: MATH_EQUATION, costMod: 1 }], [SPITEFUL_STAB]);
    expect(previews(forP1, "p1")).toEqual(previews(one, "p1"));
    // p2 may not read p1's hand or either library: change all three, and p2's view is the same —
    // however much p1's hidden Equation or Stab would deal.
    const forP2 = board([BIG_D_FENDER, RAPID_REPLENISH, RAPID_REPLENISH], [{ def: MATH_EQUATION, costMod: 4, radiant: true }], [STOCKPILE]);
    expect(previews(forP2, "p2")).toEqual(previews(one, "p2"));

    // Not vacuous: each seat sees previews, and p1's own changed: its hidden Equation is its own.
    expect(previews(one, "p1").filter((list) => list !== null).length).toBeGreaterThanOrEqual(7);
    expect(previews(one, "p2").filter((list) => list !== null).length).toBeGreaterThanOrEqual(5);
    expect(previews(forP2, "p1")).not.toEqual(previews(one, "p1"));
  });
});

// =============================================================================================
// C #1 Curse of the Forgotten Classic: the damage per card times the opponent's exile size
// =============================================================================================

describe("C #1 Curse of the Forgotten Classic previews N, its one hit (R280)", () => {
  function curse(face: Face, theirExile: number): Scenario {
    return scenario({
      p1: { hand: [{ def: CURSE, radiant: face === "radiant" }, RAPID_REPLENISH], library: [MENACE, TEMPO_TIMMY], exile: [STOCKPILE] },
      p2: { hand: [STOCKPILE], exile: Array.from({ length: theirExile }, () => STOCKPILE), library: [MENACE] },
    });
  }

  for (const face of FACES) {
    it(`R280 ${face}: the label is the formula the face prints, an exact substring of its text`, () => {
      const s = curse(face, 2);
      const list = must(shown(handCard(s.view("p1"), s.card(CURSE).id)), "the Curse's preview");
      expect(list.map((entry) => entry.label)).toEqual(["for each card in their exile"]);
      for (const entry of list) expect(cardDef(CURSE)[face].text).toContain(entry.label);
    });

    for (const exiled of [0, 3]) {
      it(`R280 ${face}: ${exiled} cards in their exile preview ${exiled}, and the Spell deals exactly that`, () => {
        const s = curse(face, exiled);
        expect(valueOf(handCard(s.view("p1"), s.card(CURSE).id))).toBe(exiled);

        s.play(CURSE);

        // R63: 0 is no hit at all. Your own exile is not counted.
        expect(hitsOn(s, "p2")).toEqual(exiled === 0 ? [] : [exiled]);
      });
    }
  }

  it("R280 R386 an Upgrade of damage per card doubles the preview, and the hit", () => {
    const s = curse("base", 3);
    stepParam(s.card(CURSE), "damage", 1);
    expect(valueOf(handCard(s.view("p1"), s.card(CURSE).id))).toBe(6);
    s.play(CURSE);
    expect(hitsOn(s, "p2")).toEqual([6]);
  });

  it("R280 in the opponent's hand it is hidden: p2's view carries no preview of it", () => {
    const s = curse("base", 3);
    expect(s.view("p2").opponent.hand).toEqual({ count: 2 });
  });

  it("R280 the hook is a pure read of public facts: no library, no hand, no state.active", () => {
    for (const face of FACES) {
      const s = curse(face, 4);
      const card = must(s.hand("p1").find((c) => c.defId === CURSE), "p1's Curse");
      const scripts = must(CARDS[CURSE], CURSE);
      const hook = must((card.radiant ? scripts.radiant : scripts.base).preview, "the Curse's hook");
      const { state, self } = guarded(s.state, card.id);
      const answer = hook({ state, self, controller: "p1", radiant: card.radiant, zone: "hand", yourTurn: true });
      expect(shown(handCard(s.view("p1"), card.id))).toEqual(answer);
      expect(answer).toEqual([{ label: "for each card in their exile", value: 4 }]);
    }
  });
});

// =============================================================================================
// C #43 Plague Nuke: the mana it would give, the Plague Tokens on the Units on the field
// =============================================================================================

describe("C #43 Plague Nuke previews the mana it would give now (R280)", () => {
  const LABEL = "for each Plague Token that was on them";
  const VANILLA = "core-008";

  function nuke(face: Face, mine: number, theirs: number): Scenario {
    return scenario({
      p1: { hand: [{ def: PLAGUE_NUKE, radiant: face === "radiant" }, RAPID_REPLENISH], field: [{ def: VANILLA, counters: { plague: mine } }] },
      p2: { hand: [STOCKPILE], field: [{ def: MENACE, counters: { plague: theirs } }, TEMPO_TIMMY] },
    });
  }

  function gained(s: Scenario): number {
    // The Spell paid 3 of 4; read off p1's own view (§10.8).
    return s.view("p1").you.mana.current - 1;
  }

  for (const face of FACES) {
    it(`R280 ${face}: the label is the formula's words, an exact substring of its text with no placeholder`, () => {
      const s = nuke(face, 1, 1);
      const list = must(shown(handCard(s.view("p1"), s.card(PLAGUE_NUKE).id)), "the Nuke's preview");
      expect(list.map((entry) => entry.label)).toEqual([LABEL]);
      expect(cardDef(PLAGUE_NUKE)[face].text).toContain(LABEL);
      expect(LABEL).not.toMatch(/[{}]/);
    });

    it(`R280 ${face}: the tokens on every Unit on both sides preview 5, and the Spell then gives exactly 5`, () => {
      const s = nuke(face, 2, 3);
      expect(valueOf(handCard(s.view("p1"), s.card(PLAGUE_NUKE).id))).toBe(5);

      s.play(PLAGUE_NUKE);

      expect(gained(s)).toBe(5);
    });
  }

  it("R280 no tokens on the board preview 0, and the Spell gives nothing", () => {
    const s = nuke("base", 0, 0);
    expect(valueOf(handCard(s.view("p1"), s.card(PLAGUE_NUKE).id))).toBe(0);

    s.play(PLAGUE_NUKE);

    expect(gained(s)).toBe(0);
  });

  it("R280 R386 an Upgrade of mana per token doubles the preview, and the gain", () => {
    const s = nuke("base", 1, 2);
    stepParam(s.card(PLAGUE_NUKE), "mana", 1);
    expect(valueOf(handCard(s.view("p1"), s.card(PLAGUE_NUKE).id))).toBe(6);

    s.play(PLAGUE_NUKE);

    expect(gained(s)).toBe(6);
  });

  it("R280 in the opponent's hand it is hidden: p2's view carries no preview of it", () => {
    const s = nuke("base", 1, 1);
    expect(JSON.stringify(s.view("p2"))).not.toContain(LABEL);
  });

  it("R280 the hook is a pure read of public facts: no library, no hand, no state.active", () => {
    for (const face of FACES) {
      const s = nuke(face, 2, 2);
      const card = must(s.hand("p1").find((c) => c.defId === PLAGUE_NUKE), "p1's Nuke");
      const scripts = must(CARDS[PLAGUE_NUKE], PLAGUE_NUKE);
      const hook = must((card.radiant ? scripts.radiant : scripts.base).preview, "the Nuke's hook");
      const { state, self } = guarded(s.state, card.id);
      const answer = hook({ state, self, controller: "p1", radiant: card.radiant, zone: "hand", yourTurn: true });
      expect(shown(handCard(s.view("p1"), card.id))).toEqual(answer);
      expect(answer).toEqual([{ label: LABEL, value: 4 }]);
    }
  });
});

// =============================================================================================
// C #59 Plague Doctor: N, every Plague Token on the field (and the Radiant face's own placement)
// =============================================================================================

describe("C #59 Plague Doctor previews N, the hit its Cry deals (R280)", () => {
  const DOCTOR_LABEL = "the number of Plague Tokens on the field";
  const PAWN = "core-096";

  /** Tokens on both sides, a face-down trap's included: 2 + 1 + 3 = 6 on the field. */
  function plagued(face: Face, active: PlayerId = "p1"): Scenario {
    return scenario({
      active,
      p1: {
        hand: [{ def: PLAGUE_DOCTOR, radiant: face === "radiant" }, RAPID_REPLENISH],
        field: [{ def: TEMPO_TIMMY, counters: { plague: 2 } }],
        library: [MENACE],
      },
      p2: {
        hand: [STOCKPILE],
        field: [{ def: MENACE, counters: { plague: 1 } }],
        backrow: [{ def: PAWN, faceUp: false, counters: { plague: 3 } }],
        health: 30,
      },
    });
  }

  it("R280 the label is the phrase both faces print, with no placeholder in it", () => {
    for (const face of FACES) {
      const s = plagued(face);
      const list = must(shown(handCard(s.view("p1"), s.card(PLAGUE_DOCTOR).id)), `${face} preview`);
      expect(list.map((entry) => entry.label)).toEqual([DOCTOR_LABEL]);
      expect(cardDef(PLAGUE_DOCTOR)[face].text).toContain(DOCTOR_LABEL);
      expect(DOCTOR_LABEL).not.toMatch(/[{}]/);
    }
  });

  for (const [face, expected] of [
    ["base", 6],
    ["radiant", 8],
  ] as const) {
    it(`R280 ${face}: in hand it previews ${expected}, and its Cry then deals exactly that`, () => {
      const s = plagued(face);
      const value = valueOf(handCard(s.view("p1"), s.card(PLAGUE_DOCTOR).id));
      expect(value).toBe(expected);

      s.play(PLAGUE_DOCTOR, { targets: AT_ENEMY_HERO });

      expect(hitsOn(s, "p2")).toEqual([value]);
    });
  }

  it("R280 radiant: on the field the preview counts its own tokens, plus the 2 another Cry would place", () => {
    const s = plagued("radiant");
    s.play(PLAGUE_DOCTOR, { targets: AT_ENEMY_HERO });
    // 6 on the field, 2 more on the Doctor: 8 tokens now, and a Cry now would add 2 more.
    const doctor = s.card(PLAGUE_DOCTOR);
    const lane = must(doctor.zone.z === "field" ? doctor.zone.lane : null, "the Doctor's lane");
    expect(valueOf(s.view("p1").you.units[lane - 1])).toBe(10);
    // A unit is public: the other seat sees the same number.
    expect(valueOf(s.view("p2").opponent.units[lane - 1])).toBe(10);
  });

  it("R280 base: on the field it previews the tokens there now, for both seats", () => {
    const s = scenario({
      p1: { field: [{ def: PLAGUE_DOCTOR, counters: { plague: 1 } }], hand: [RAPID_REPLENISH] },
      p2: { field: [{ def: MENACE, counters: { plague: 2 } }], hand: [STOCKPILE] },
    });
    expect(valueOf(s.view("p1").you.units[0])).toBe(3);
    expect(valueOf(s.view("p2").opponent.units[0])).toBe(3);
  });

  it("R280 an empty field previews 0 on the base face, and the Cry then deals nothing (R63)", () => {
    const s = scenario({ p1: { hand: [PLAGUE_DOCTOR, RAPID_REPLENISH] }, p2: { hand: [STOCKPILE] } });
    expect(valueOf(handCard(s.view("p1"), s.card(PLAGUE_DOCTOR).id))).toBe(0);

    s.play(PLAGUE_DOCTOR, { targets: AT_ENEMY_HERO });

    expect(hitsOn(s, "p2")).toEqual([]);
  });

  it("R280 R386 radiant: an Upgrade of its tokens moves the preview with the hit", () => {
    const s = plagued("radiant");
    stepParam(s.card(PLAGUE_DOCTOR), "tokens", 1);
    const value = valueOf(handCard(s.view("p1"), s.card(PLAGUE_DOCTOR).id));
    expect(value).toBe(9);

    s.play(PLAGUE_DOCTOR, { targets: AT_ENEMY_HERO });

    expect(hitsOn(s, "p2")).toEqual([9]);
  });

  it("R280 the hook writes nothing and reads no library, no hand and not state.active", () => {
    for (const face of FACES) {
      for (const active of ["p1", "p2"] as const) {
        const s = plagued(face, active);
        const card = s.card(PLAGUE_DOCTOR);
        const scripts = must(CARDS[PLAGUE_DOCTOR], PLAGUE_DOCTOR);
        const hook = must((face === "radiant" ? scripts.radiant : scripts.base).preview, "the Doctor's hook");
        const { state, self } = guarded(s.state, card.id);
        const answer = hook({ state, self, controller: "p1", radiant: card.radiant, zone: "hand", yourTurn: active === "p1" });
        expect(shown(handCard(s.view("p1"), card.id))).toEqual(answer);
      }
    }
  });

  it("R280 the opponent's Doctor in hand carries no preview for you (§10.8)", () => {
    const s = plagued("base");
    const theirs = s.view("p2").opponent.hand;
    expect(JSON.stringify(theirs)).not.toContain(DOCTOR_LABEL);
  });
});

// =============================================================================================
// C #88 Siphon Squad: X, twice the Units its controller's opponent controls (base face only)
// =============================================================================================

describe("C #88 Siphon Squad previews X, the attack its aura takes off each enemy Unit (R280)", () => {
  const SIPHON_LABEL = "−X Attack";

  function siphonBoard(opts: { faceUp?: boolean; radiant?: boolean; enemies?: number } = {}): Scenario {
    const enemies = Array.from({ length: opts.enemies ?? 2 }, (_, at) => ({ def: MENACE, lane: at + 1 }));
    return scenario({
      p1: {
        hand: [RAPID_REPLENISH],
        backrow: [{ def: SIPHON_SQUAD, radiant: opts.radiant === true, ...(opts.faceUp === undefined ? {} : { faceUp: opts.faceUp }) }],
      },
      p2: { hand: [STOCKPILE], field: enemies },
    });
  }

  it("R280 the label is in the base face's text, with no placeholder in it", () => {
    const s = siphonBoard({ faceUp: false });
    const list = must(shown(s.view("p1").you.backrow[0]), "the controller's preview");
    expect(list.map((entry) => entry.label)).toEqual([SIPHON_LABEL]);
    expect(cardDef(SIPHON_SQUAD).base.text).toContain(SIPHON_LABEL);
    expect(SIPHON_LABEL).not.toMatch(/[{}]/);
  });

  it("R280 face-down, its controller sees X, and it is what the aura takes off each enemy Unit", () => {
    const s = siphonBoard({ faceUp: false });
    const value = valueOf(s.view("p1").you.backrow[0]);

    expect(value).toBe(4);
    const menace = must(s.unit("p2", 1), "p2's Menace");
    expect(s.stats(menace).attack).toBe(9 - value);
  });

  it("R280 §10.8 face-down, the other player's view of it carries no preview", () => {
    const s = siphonBoard({ faceUp: false });
    const theirs = s.view("p2").opponent.backrow[0];

    expect(theirs).toMatchObject({ faceDown: true });
    expect(JSON.stringify(theirs)).not.toContain(SIPHON_LABEL);
    expect(JSON.stringify(s.view("p2"))).not.toContain("preview");
  });

  it("R280 face-up, both players see X", () => {
    const s = siphonBoard({ faceUp: true, enemies: 3 });

    expect(valueOf(s.view("p1").you.backrow[0])).toBe(6);
    expect(valueOf(s.view("p2").opponent.backrow[0])).toBe(6);
  });

  it("R280 in hand it previews the X it would take now", () => {
    const s = scenario({ p1: { hand: [SIPHON_SQUAD, RAPID_REPLENISH] }, p2: { field: [MENACE], hand: [STOCKPILE] } });
    expect(valueOf(handCard(s.view("p1"), s.card(SIPHON_SQUAD).id))).toBe(2);
  });

  it("R280 R386 an Upgrade of its multiplier moves the preview with the aura", () => {
    const s = siphonBoard({ faceUp: false });
    stepParam(must(s.backrow("p1", 1), "the Siphon"), "multiplier", 1);

    const value = valueOf(s.view("p1").you.backrow[0]);
    expect(value).toBe(6);
    expect(s.stats(must(s.unit("p2", 1), "p2's Menace")).attack).toBe(9 - value);
  });

  it("R280 the Radiant face has no X, and no preview", () => {
    const s = siphonBoard({ radiant: true, faceUp: true });
    expect(shown(s.view("p1").you.backrow[0])).toBeNull();
    expect(shown(s.view("p2").opponent.backrow[0])).toBeNull();
  });

  it("R280 the hook writes nothing and reads no library, no hand and not state.active", () => {
    for (const active of ["p1", "p2"] as const) {
      const s = scenario({
        active,
        p1: { hand: [RAPID_REPLENISH], backrow: [{ def: SIPHON_SQUAD, faceUp: false }], library: [MENACE] },
        p2: { hand: [STOCKPILE], field: [MENACE], library: [MENACE] },
      });
      const card = must(s.backrow("p1", 1), "the Siphon");
      const hook = must(must(CARDS[SIPHON_SQUAD], SIPHON_SQUAD).base.preview, "its hook");
      const { state, self } = guarded(s.state, card.id);
      const answer = hook({ state, self, controller: "p1", radiant: false, zone: "field", yourTurn: active === "p1" });
      expect(shown(s.view("p1").you.backrow[0])).toEqual(answer);
    }
  });
});
