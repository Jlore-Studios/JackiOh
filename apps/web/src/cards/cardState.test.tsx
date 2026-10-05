// Patch v0.2.0's per-card states, drawn wherever the view lets the viewer read the card (SPEC §10.8):
// a Brittle count (R385), what Degrade and Upgrade changed (R386, R513), the enchantments riding a
// card (B5 E39), a backrow pile's depth (E21), an Animated card's stats and its standing as a Unit
// (R383), a face's own type (B2.7) and a card's lines of code (E36), which a match hides (R693).
//
// Every view here is a fixture shaped as `viewFor` builds it (src/test/fixtures.ts, the view types in
// packages/shared/src/view.ts), rendered on the board — a hand card (the tall face), a unit (the
// minion), a face-up backrow card (the compact face), a face-down one (a back) — and in the inspect
// overlays a hover and a long-press open. Each state is checked on each surface, and its absence
// draws nothing.

import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { CATALOG } from "@jackioh/cards";
import { fillParams, type BackrowView, type CardDef, type PlayerView, type Tuning } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { resetFxSettingsForTests } from "../fx/settings.ts";
import Board from "../game/Board.tsx";
import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import { testid } from "../game/contract.ts";
import { liveFace } from "../game/faces.ts";
import { __resetSettingsForTests } from "../settings/store.ts";
import { baseView, card, emptySide, faceUpBackrow, fusedDef, unit } from "../test/fixtures.ts";
import { CardFace } from "./CardFace.tsx";
import { BERSERK_WORDS, PILE_WORDS, animatedWords, brittleWords, distinctEnchantments, enchantmentWords, stateBadges } from "./cardState.ts";
import { markColorOf, markWords } from "./marks.ts";
import { STATE_BADGES_SMALL_MAX } from "./constants.ts";
import { closeInspect } from "./inspect/store.ts";
import { HOVER_DELAY_MS, LONG_PRESS_MS } from "./inspect/constants.ts";
import {
  INSPECT_GLOSSARY,
  INSPECT_HOVER,
  INSPECT_LOC,
  INSPECT_PRINTED,
  INSPECT_SHEET,
  INSPECT_STATES,
  INSPECT_TUNED,
} from "./inspect/testids.ts";
import { faceModel, type FaceModel } from "./model.ts";
import { glossaryFor } from "./rules.ts";
import {
  LESS_IS_BETTER,
  VERDICT_WORD,
  changeWords,
  faceTuning,
  filledText,
  keyWords,
  tunedRangeWords,
  tuningSummary,
} from "./tuning.ts";

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
  vi.useRealTimers();
  resetFxSettingsForTests();
  __resetSettingsForTests();
});

/* -------------------------------------------------------------------------------------- helpers */

const lookup = lookupFromDefs(CATALOG);

function def(id: string): CardDef {
  const found = CATALOG[id];
  if (found === undefined) throw new Error(`the catalog has no ${id}`);
  return found;
}

function withCatalog(node: ReactElement): ReactElement {
  return <CatalogContext.Provider value={lookup}>{node}</CatalogContext.Provider>;
}

function renderBoard(view: PlayerView): void {
  render(withCatalog(<Board view={view} />));
}

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`expected ${what}`);
  return value;
}

/** The `.cf` face inside a board card's root. */
function faceOf(root: HTMLElement): HTMLElement {
  return must(root.querySelector<HTMLElement>(".cf"), "a face inside the card");
}

/** Rests a mouse on `element` until its hover preview is up, and returns the preview. */
function hover(element: HTMLElement): HTMLElement {
  fireEvent.pointerEnter(element, { pointerType: "mouse" });
  act(() => {
    vi.advanceTimersByTime(HOVER_DELAY_MS * 2);
  });
  return screen.getByTestId(INSPECT_HOVER);
}

/** A touch held on `element` until its inspect sheet opens, and the sheet. */
function longPress(element: HTMLElement): HTMLElement {
  fireEvent.pointerDown(element, { pointerType: "touch", clientX: 10, clientY: 10 });
  act(() => {
    vi.advanceTimersByTime(LONG_PRESS_MS * 2);
  });
  fireEvent.pointerUp(element, { pointerType: "touch", clientX: 10, clientY: 10 });
  return screen.getByTestId(INSPECT_SHEET);
}

function handView(over: Parameters<typeof card>[0]): PlayerView {
  return baseView({ you: emptySide("p1", { hand: [card({ instanceId: "h1", ...over })] }) });
}

function handRoot(): HTMLElement {
  return screen.getByTestId(testid.handCard("h1"));
}

function inPlay(id: string, over: Partial<Parameters<typeof faceModel>[0]["inPlay"]> = {}, radiant = false): FaceModel {
  return faceModel({ defId: id, def: def(id), radiant, inPlay: { ...over } });
}

/* ------------------------------------------------------------------------------- R385 Brittle */

describe("R385 Brittle: a cracked-glass badge with the count, on every face that shows one", () => {
  it("R385 the words: \"Brittle 2: crumbles at 0\"", () => {
    expect(brittleWords(2)).toBe("Brittle 2: crumbles at 0");
    expect(brittleWords(0)).toBe("Brittle 0: crumbles at 0");
  });

  it("R385 a hand card the view gives a count wears the badge: the count over a cracked pane, its words as tooltip and name", () => {
    renderBoard(handView({ defId: "classicplus-074", cost: 2, brittle: 2 }));
    const badge = must(handRoot().querySelector<HTMLElement>('.cf-state[data-state="brittle"]'), "the Brittle badge");
    expect(badge.getAttribute("data-brittle")).toBe("2");
    expect(badge.textContent).toBe("2");
    expect(badge.getAttribute("title")).toBe("Brittle 2: crumbles at 0");
    expect(badge.getAttribute("aria-label")).toBe("Brittle 2: crumbles at 0");
    expect(badge.getAttribute("role")).toBe("img");
    expect(badge.querySelector("img.cf-icon--brittle")).not.toBeNull();
  });

  it("R385 a face-up backrow card wears it on its compact face", () => {
    const entry = faceUpBackrow("p1", { instanceId: "b1", defId: "classicplus-074", type: "Field Trap", brittle: 3 });
    renderBoard(baseView({ you: emptySide("p1", { backrow: [entry, null, null, null, null] }) }));
    const face = faceOf(screen.getByTestId(testid.card("b1")));
    expect(face.getAttribute("data-layout")).toBe("compact");
    expect(face.querySelector('.cf-state[data-state="brittle"]')?.getAttribute("data-brittle")).toBe("3");
  });

  it("R385 the board minion draws the count as its cracks (the keyword treatment), even with no Brittle keyword listed, and not twice", () => {
    renderBoard(
      baseView({
        you: emptySide("p1", {
          units: [unit("p1", { instanceId: "u1", defId: "core-004", keywords: [], brittle: 1 }), null, null, null, null],
        }),
      }),
    );
    const root = screen.getByTestId(testid.card("u1"));
    const cracks = must(root.querySelector('[data-keyword-fx="Brittle"]'), "the Brittle treatment");
    expect(cracks.getAttribute("data-n")).toBe("1");
    expect(root.querySelector('.cf-state[data-state="brittle"]')).toBeNull();
  });

  it("R385 the inspect overlays spell it out beside the face, and the glossary explains Brittle", () => {
    vi.useFakeTimers();
    renderBoard(handView({ defId: "core-004", cost: 1, attack: 1, health: 1, brittle: 2 }));
    const preview = hover(handRoot());
    expect(preview.querySelector('.cf-state[data-state="brittle"]')).not.toBeNull();
    const states = within(preview).getByTestId(INSPECT_STATES);
    expect(states.textContent).toContain("Brittle 2: crumbles at 0");
    expect(within(preview).getByTestId(INSPECT_GLOSSARY).querySelector('[data-glossary-term="Brittle"]')).not.toBeNull();
    fireEvent.pointerLeave(handRoot(), { pointerType: "mouse" });

    const sheet = longPress(handRoot());
    expect(within(sheet).getByTestId(INSPECT_STATES).textContent).toContain("Brittle 2: crumbles at 0");
  });

  it("R385 a card with no count has no badge, no line and no Brittle entry it did not print", () => {
    vi.useFakeTimers();
    renderBoard(handView({ defId: "core-004", cost: 1, attack: 1, health: 1 }));
    expect(handRoot().querySelector(".cf-states")).toBeNull();
    const preview = hover(handRoot());
    expect(within(preview).queryByTestId(INSPECT_STATES)).toBeNull();
    expect(preview.querySelector('[data-glossary-term="Brittle"]')).toBeNull();
  });
});

/* -------------------------------------------------------------------- R386 Degrade / Upgrade */

describe("R386 R513 Degrade and Upgrade: what changed, marked better or worse", () => {
  const BOOK = "classic-012"; // Book of Blood: "Deal {damage} damage to a Unit.", damage 5, better up.

  it("R513 each change is better or worse for the card's controller, and the card as a whole Upgraded, Degraded or Tuned", () => {
    const book = def(BOOK);
    expect(faceTuning(book, false, undefined, undefined)).toBeNull();
    expect(faceTuning(book, false, undefined, { damage: 5 })).toBeNull();

    const up = must(faceTuning(book, false, { numbers: { damage: 1 } }, { damage: 6 }), "a change");
    expect(up.verdict).toBe("upgraded");
    expect(up.changes).toEqual([{ kind: "number", key: "damage", printed: 5, value: 6, way: "better" }]);

    const down = must(faceTuning(book, false, { numbers: { damage: -1 } }, { damage: 4 }), "a change");
    expect(down.verdict).toBe("degraded");

    // A number better lower (Twice Forward's "every {plays} cards", better down).
    const twice = def("classicplus-074");
    expect(faceTuning(twice, false, undefined, { plays: 3, brittleGain: 1 })?.changes).toEqual([
      { kind: "number", key: "plays", printed: 2, value: 3, way: "worse" },
    ]);

    // Stats, keywords, numbered keywords and X: up is better, except Tribute, where less is.
    const mixed: Tuning = {
      attack: 2,
      health: -2,
      addKeywords: [{ kind: "Rush" }],
      removeKeywords: ["Taunt"],
      x: { Armor: -1, Tribute: -1, X: 1 },
    };
    const tuned = must(faceTuning(def("core-004"), false, mixed, undefined), "changes");
    expect(tuned.verdict).toBe("tuned");
    expect(tuned.attack).toBe("better");
    expect(tuned.health).toBe("worse");
    expect(tuned.added).toEqual([{ kind: "Rush" }]);
    expect(tuned.removed).toEqual(["Taunt"]);
    expect(tuned.changes.map(changeWords)).toEqual([
      "+2 Attack",
      "−2 Health",
      "Gained Rush",
      "Lost Taunt",
      "Armor −1",
      "Tribute −1",
      "X +1",
    ]);
    expect(tuned.changes.filter((change) => change.kind === "count").map((change) => change.kind === "count" && change.way)).toEqual([
      "worse",
      "better",
      "better",
    ]);
    expect(LESS_IS_BETTER).toEqual(["Tribute"]);

    // KY's Constant's "to N" on something that is not a declared number is neither better nor worse.
    const set = must(faceTuning(def("core-004"), false, { set: { Armor: 3 } }, undefined), "a set");
    expect(set.verdict).toBe("tuned");
    expect(set.changes.map(changeWords)).toEqual(["Armor set to 3"]);
    expect(tuningSummary(up)).toBe("Upgraded: Damage 5 → 6");
    expect(keyWords("drawLimit")).toBe("Draw limit");
    expect(VERDICT_WORD).toEqual({ upgraded: "Upgraded", degraded: "Degraded", tuned: "Tuned" });
  });

  it("R386 filledText fills every catalog face exactly as fillParams does, and boxes only the numbers that moved", () => {
    let moved = 0;
    for (const card of Object.values(CATALOG)) {
      for (const face of ["base", "radiant"] as const) {
        expect(filledText(card, face).text, `${card.id} ${face}`).toBe(fillParams(card, face));
        expect(filledText(card, face).tuned, `${card.id} ${face}`).toEqual([]);
        const values = Object.fromEntries((card.params ?? []).map((param) => [param.key, param[face] + 2]));
        const filled = filledText(card, face, values);
        expect(filled.text, `${card.id} ${face} moved`).toBe(fillParams(card, face, values));
        for (const range of filled.tuned) {
          expect(filled.text.slice(range.start, range.end), `${card.id} ${face} ${range.key}`).toBe(String(range.value));
          moved += 1;
        }
      }
    }
    expect(moved).toBeGreaterThan(50);
  });

  it("R386 a hand card whose number moved boxes it with ▲ or ▼, keeps the number as its text, and says what was printed", () => {
    renderBoard(handView({ defId: BOOK, cost: 1, params: { damage: 6 }, tuning: { numbers: { damage: 1 } } }));
    const face = faceOf(handRoot());
    expect(face.getAttribute("data-tuned")).toBe("upgraded");
    const number = must(face.querySelector<HTMLElement>(".cf-tuned"), "the boxed number");
    expect(number.textContent).toBe("6");
    expect(number.getAttribute("data-way")).toBe("better");
    expect(number.getAttribute("data-printed")).toBe("5");
    expect(number.getAttribute("title")).toBe(tunedRangeWords({ printed: 5, way: "better" }));
    expect(face.querySelector(".card-text")?.textContent).toContain("Deal 6 damage to a Unit.");
    const mark = must(face.querySelector('.cf-state[data-state="tuned"]'), "the tuned mark");
    expect(mark.getAttribute("data-tuned")).toBe("upgraded");
    expect(mark.textContent).toBe("▲");
    expect(mark.getAttribute("title")).toBe("Upgraded: Damage 5 → 6");
  });

  it("R386 a number moved the worse way is marked ▼ and the card Degraded", () => {
    renderBoard(handView({ defId: BOOK, cost: 1, params: { damage: 4 }, tuning: { numbers: { damage: -1 } } }));
    const face = faceOf(handRoot());
    expect(face.getAttribute("data-tuned")).toBe("degraded");
    expect(face.querySelector(".cf-tuned")?.getAttribute("data-way")).toBe("worse");
    expect(face.querySelector('.cf-state[data-state="tuned"]')?.textContent).toBe("▼");
  });

  it("R386 a tuned unit's stats carry the way they went beside their tones, on the hand face and on the minion", () => {
    const tuning: Tuning = { attack: 3, health: -1 };
    renderBoard(
      baseView({
        you: emptySide("p1", {
          hand: [card({ instanceId: "h1", defId: "core-004", cost: 1, attack: 4, health: 1, tuning })],
          units: [unit("p1", { instanceId: "u1", defId: "core-004", attack: 4, maxHealth: 1, health: 1, tuning }), null, null, null, null],
        }),
      }),
    );
    const hand = faceOf(handRoot());
    expect(hand.querySelector(".cf-atk")?.getAttribute("data-tuned")).toBe("better");
    expect(hand.querySelector(".cf-atk")?.getAttribute("data-tone")).toBe("buffed");
    expect(hand.querySelector(".cf-hp")?.getAttribute("data-tuned")).toBe("worse");
    const minion = screen.getByTestId(testid.card("u1"));
    expect(minion.querySelector(".stat-attack")?.getAttribute("data-tuned")).toBe("better");
    expect(minion.querySelector(".stat-health")?.getAttribute("data-tuned")).toBe("worse");
    // The numbers the tests and e2e read stay the numbers.
    expect(minion.querySelector(".stat-attack")?.textContent).toBe("4");
    expect(faceOf(minion).getAttribute("data-tuned")).toBe("tuned");
    expect(minion.querySelector('.cf-state[data-state="tuned"]')?.getAttribute("data-tuned")).toBe("tuned");
  });

  it("R386 keywords Upgrade added are \"+\" chips and keywords Degrade removed are struck \"−\" chips, and an added one is not also listed as gained", () => {
    const tuning: Tuning = { addKeywords: [{ kind: "Rush" }], removeKeywords: ["Taunt"] };
    renderBoard(
      baseView({
        you: emptySide("p1", {
          hand: [card({ instanceId: "h1", defId: "core-004", cost: 1, attack: 1, health: 1, tuning })],
          units: [unit("p1", { instanceId: "u1", defId: "core-004", keywords: [{ kind: "Rush" }, { kind: "Lifesteal" }], tuning }), null, null, null, null],
        }),
      }),
    );
    const chips = Array.from(faceOf(handRoot()).querySelectorAll<HTMLElement>(".cf-text-tuning .cf-kw-chip"));
    expect(chips.map((chip) => [chip.getAttribute("data-tuned"), chip.textContent])).toEqual([
      ["added", "+Rush"],
      ["removed", "−Taunt"],
    ]);
    expect(chips.map((chip) => chip.getAttribute("title"))).toEqual(["Gained Rush", "Lost Taunt"]);
    const field = liveFace(must(lookup("core-004", false), "Gary"), unit("p1", { defId: "core-004", keywords: [{ kind: "Rush" }, { kind: "Lifesteal" }], tuning }));
    expect(field.gained.map((keyword) => keyword.kind)).toEqual(["Lifesteal"]);
  });

  it("R386 the hover preview gives the ribbon, every change in words, and the printed text beside a face whose numbers moved", () => {
    vi.useFakeTimers();
    renderBoard(handView({ defId: BOOK, cost: 1, params: { damage: 6 }, tuning: { numbers: { damage: 1 }, attack: 0 } }));
    const preview = hover(handRoot());
    const ribbon = within(preview).getByTestId(INSPECT_TUNED);
    expect(ribbon.getAttribute("data-tuned")).toBe("upgraded");
    expect(ribbon.querySelector(".inspect-tuned-ribbon")?.textContent).toBe("▲Upgraded");
    const lines = Array.from(ribbon.querySelectorAll<HTMLElement>(".inspect-tuned-change"));
    expect(lines.map((line) => line.getAttribute("data-way"))).toEqual(["better"]);
    expect(lines[0]?.textContent).toBe("▲Damage 5 → 6 (better)");
    expect(within(preview).getByTestId(INSPECT_PRINTED).textContent).toContain("Deal 5 damage to a Unit.");
    expect(preview.querySelector('[data-glossary-term="Upgrade"]')).not.toBeNull();
    fireEvent.pointerLeave(handRoot(), { pointerType: "mouse" });

    const sheet = longPress(handRoot());
    expect(within(sheet).getByTestId(INSPECT_TUNED).textContent).toContain("Damage 5 → 6");
    expect(within(sheet).getByTestId(INSPECT_PRINTED)).toBeInTheDocument();
  });

  it("R386 an untuned card, or one whose numbers stand at their printed values, carries no mark", () => {
    vi.useFakeTimers();
    renderBoard(handView({ defId: BOOK, cost: 1, params: { damage: 5 } }));
    const face = faceOf(handRoot());
    expect(face.getAttribute("data-tuned")).toBeNull();
    expect(face.querySelector(".cf-tuned, .cf-states, .cf-text-tuning")).toBeNull();
    const preview = hover(handRoot());
    expect(within(preview).queryByTestId(INSPECT_TUNED)).toBeNull();
    expect(within(preview).queryByTestId(INSPECT_PRINTED)).toBeNull();
    expect(preview.querySelector('[data-glossary-term="Upgrade"], [data-glossary-term="Degrade"]')).toBeNull();
  });

  it("R386 a fused card's text is filled from the view's numbers, never shows a brace, and marks nothing as moved", () => {
    const fused = fusedDef([def(BOOK), def("classic-020")], 3);
    expect(fused.params).toBeUndefined();
    expect(fused.base.text).toContain("{damage}");
    const view = baseView({
      you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: fused.id, cost: 3, params: { damage: 7, discards: 1 } })] }),
      defs: { [fused.id]: fused },
    });
    renderBoard(view);
    const text = faceOf(handRoot()).querySelector(".card-text")?.textContent ?? "";
    expect(text).not.toContain("{");
    expect(text).toContain("Deal 7 damage");
    expect(faceOf(handRoot()).querySelector(".cf-tuned")).toBeNull();
  });
});

/* ------------------------------------------------------------------------- E39 enchantments */

describe("E39 enchantments: a glyph and words on every face that shows the card", () => {
  it("E39 R432 the words: \"Returns to hand · can't cost less than (2)\", \"Cast on draw\", \"Targets enemies\"", () => {
    expect(enchantmentWords({ kind: "returnAfterResolve", floor: 2 })).toBe("Returns to hand · can't cost less than (2)");
    expect(enchantmentWords({ kind: "castOnDraw" })).toBe("Cast on draw");
    expect(enchantmentWords({ kind: "targetEnemies" })).toBe("Targets enemies");
    expect(
      distinctEnchantments([{ kind: "castOnDraw" }, { kind: "targetEnemies" }, { kind: "castOnDraw" }]).map((entry) => entry.kind),
    ).toEqual(["castOnDraw", "targetEnemies"]);
  });

  it("E39 a hand card, a unit and a face-up backrow card each wear a badge per enchantment", () => {
    const enchantments = [{ kind: "returnAfterResolve", floor: 2 } as const, { kind: "castOnDraw" } as const, { kind: "targetEnemies" } as const];
    renderBoard(
      baseView({
        you: emptySide("p1", {
          hand: [card({ instanceId: "h1", defId: "classic-012", cost: 2, enchantments: [...enchantments] })],
          units: [unit("p1", { instanceId: "u1", defId: "core-004", enchantments: [{ kind: "returnAfterResolve", floor: 1 }] }), null, null, null, null],
          backrow: [faceUpBackrow("p1", { instanceId: "b1", defId: "classic-020", type: "Field Spell", enchantments: [{ kind: "castOnDraw" }] }), null, null, null, null],
        }),
      }),
    );
    const hand = Array.from(handRoot().querySelectorAll<HTMLElement>(".cf-state"));
    expect(hand.map((badge) => badge.getAttribute("data-enchantment"))).toEqual(["returnAfterResolve", "castOnDraw", "targetEnemies"]);
    expect(hand.map((badge) => badge.getAttribute("title"))).toEqual([
      "Returns to hand · can't cost less than (2)",
      "Cast on draw",
      "Targets enemies",
    ]);
    expect(hand[0]?.getAttribute("data-floor")).toBe("2");
    for (const badge of hand) expect(badge.querySelector("img.cf-icon"), badge.getAttribute("data-state") ?? "").not.toBeNull();
    // On a small face the first two show and the rest fold into "+n" (cardstate.css draws the fold).
    expect(STATE_BADGES_SMALL_MAX).toBe(2);
    expect(handRoot().querySelector(".cf-states-more")?.textContent).toBe("+1");

    const minion = screen.getByTestId(testid.card("u1"));
    expect(minion.querySelector('.cf-state[data-enchantment="returnAfterResolve"]')?.getAttribute("title")).toBe(
      "Returns to hand · can't cost less than (1)",
    );
    const backrow = screen.getByTestId(testid.card("b1"));
    expect(backrow.querySelector('.cf-state[data-enchantment="castOnDraw"]')).not.toBeNull();
  });

  it("E39 the sheet lists each in words, and Cast on draw's glossary entry rides along", () => {
    vi.useFakeTimers();
    renderBoard(handView({ defId: "classic-012", cost: 2, enchantments: [{ kind: "castOnDraw" }, { kind: "targetEnemies" }] }));
    const sheet = longPress(handRoot());
    const lines = Array.from(within(sheet).getByTestId(INSPECT_STATES).querySelectorAll("li"));
    expect(lines.map((line) => line.getAttribute("data-state"))).toEqual(["castOnDraw", "targetEnemies"]);
    expect(lines.map((line) => line.textContent)).toEqual(["Cast on draw", "Targets enemies"]);
    expect(sheet.querySelector('[data-glossary-term="Cast on draw"]')).not.toBeNull();
  });

  it("E39 a card with none has no badge and no line", () => {
    renderBoard(handView({ defId: "classic-012", cost: 2, enchantments: [] }));
    expect(handRoot().querySelector(".cf-states")).toBeNull();
    expect(stateBadges(inPlay("classic-012"))).toEqual([]);
  });
});

/* ------------------------------------------------------------------------- E21 backrow piles */

describe("E21 a backrow pile shows its depth, as a unit pile does", () => {
  it("E21 a face-up top with cards under it, and a face-down one, each show the count in their zone", () => {
    const faceDownPile = { faceDown: true, cost: 2, buried: 1 } as unknown as BackrowView;
    renderBoard(
      baseView({
        you: emptySide("p1", {
          backrow: [faceUpBackrow("p1", { instanceId: "b1", defId: "classic-020", type: "Field Spell", buried: 2 }), faceDownPile, null, null, null],
        }),
      }),
    );
    const first = screen.getByTestId(testid.zone("you", "backrow", 1));
    const pile = must(first.querySelector<HTMLElement>(":scope > .backrow-pile"), "the first zone's depth");
    // A press opens the pile as a wheel (issue #124), so the depth is a button, not a span.
    expect(pile.tagName).toBe("BUTTON");
    expect(pile.getAttribute("data-buried")).toBe("2");
    expect(pile.textContent).toBe("2");
    expect(pile.getAttribute("title")).toBe(PILE_WORDS);
    expect(pile.classList.contains("buried-badge")).toBe(true);
    // Beside the card, not inside it: the card's own root is untouched.
    expect(screen.getByTestId(testid.card("b1")).querySelector(".backrow-pile")).toBeNull();

    const second = screen.getByTestId(testid.zone("you", "backrow", 2));
    expect(second.querySelector(":scope > .backrow-pile")?.getAttribute("data-buried")).toBe("1");
  });

  it("E21 a backrow card with nothing under it shows no depth", () => {
    renderBoard(
      baseView({
        you: emptySide("p1", {
          backrow: [faceUpBackrow("p1", { instanceId: "b1" }), { faceDown: true, cost: 1 }, null, null, null],
        }),
      }),
    );
    expect(document.querySelector(".backrow-pile")).toBeNull();
  });
});

describe("E21 the pile badge's stylesheets", () => {
  function sheet(fromWeb: string): string {
    for (const candidate of [fromWeb, `apps/web/${fromWeb}`]) {
      const path = resolve(process.cwd(), candidate);
      if (existsSync(path)) return readFileSync(path, "utf8");
    }
    throw new Error(`${fromWeb} not found from ${process.cwd()}`);
  }

  function block(css: string, selector: string): string {
    const at = css.indexOf(`${selector} {`);
    expect(at, `a rule for ${selector}`).toBeGreaterThanOrEqual(0);
    return css.slice(at, css.indexOf("}", at));
  }

  it("E21 the badge reads as the badge, not an app-shell button: board.css outranks index.css's padding", () => {
    const rule = block(sheet("src/game/board.css"), ".board button.buried-badge");
    expect(rule).toMatch(/padding:\s*0 3px/);
    expect(rule).toMatch(/border:\s*0/);
  });

  it("E21 a backrow pile's badge takes a press: its rule sets no pointer-events: none", () => {
    const rule = block(sheet("src/cards/cardstate.css"), ".zone > .backrow-pile");
    expect(rule).not.toMatch(/pointer-events:\s*none/);
  });
});

/* -------------------------------------------------------------------------------- R383 Animated */

describe("R383 Animated: its face prints the Unit it becomes, and a card standing as a Unit says so", () => {
  it("R383 an Animated Field Trap in hand and in the collection shows its attack and health; another Field Trap shows none", () => {
    renderBoard(handView({ defId: "classic-005", cost: 2 }));
    const face = faceOf(handRoot());
    expect(face.getAttribute("data-card-type")).toBe("Field Trap");
    expect(face.querySelector(".cf-atk")?.getAttribute("data-face-attack")).toBe("1");
    expect(face.querySelector(".cf-hp")?.getAttribute("data-face-health")).toBe("4");
    cleanup();

    const collection = render(<CardFace face={faceModel({ defId: "classicplus-012-8", def: def("classicplus-012-8"), radiant: true })} />);
    expect(collection.container.querySelector(".cf-atk")?.getAttribute("data-face-attack")).toBe("20");
    expect(collection.container.querySelector(".cf-hp")?.getAttribute("data-face-health")).toBe("6");
    cleanup();

    const plain = render(<CardFace face={faceModel({ defId: "classicplus-074", def: def("classicplus-074"), radiant: false })} />);
    expect(plain.container.querySelector(".cf-stats")).toBeNull();
  });

  it("R383 a card standing in a unit zone: the minion's cog treatment, and its preview's words and glossary say where it goes back", () => {
    vi.useFakeTimers();
    renderBoard(
      baseView({
        you: emptySide("p1", {
          units: [
            unit("p1", { instanceId: "u1", defId: "classicplus-012-8", attack: 10, maxHealth: 3, health: 3, keywords: [{ kind: "Rush" }], animated: { home: 2 } }),
            null,
            null,
            null,
            null,
          ],
          reserved: { units: [false, false, false, false, false], backrow: [false, true, false, false, false] },
        }),
      }),
    );
    const root = screen.getByTestId(testid.card("u1"));
    expect(root.querySelector('[data-keyword-fx="Animated on your turn"]')).not.toBeNull();
    // The rail leaves the cog to the treatment.
    expect(root.querySelector('.cf-state[data-state="animated"]')).toBeNull();
    const preview = hover(root);
    expect(preview.querySelector('.cf-state[data-state="animated"]')?.getAttribute("data-animated")).toBe("2");
    expect(within(preview).getByTestId(INSPECT_STATES).textContent).toBe(animatedWords({ home: 2 }, "Field Spell"));
    expect(animatedWords({ home: 2 }, "Field Spell")).toBe(
      "Animated on your turn: back to its backrow (lane 2) at the end of its controller's turn",
    );
    expect(animatedWords({}, "Field Trap")).toBe("Animated: this Field Trap stands in a unit zone as a Unit");
    expect(preview.querySelector('[data-glossary-term="Animated on your turn"]')).not.toBeNull();
  });
});

/* ----------------------------------------------------------------------------- B5 E35 Berserk */

describe("B5 E35 Berserk: a sword badge on the unit, in its glossary row's words", () => {
  it("E35 a Berserk unit wears the badge on the board and says why in its preview; a calm one wears none", () => {
    vi.useFakeTimers();
    renderBoard(
      baseView({
        you: emptySide("p1", {
          units: [unit("p1", { instanceId: "u1", defId: "classicplus-019-5", berserk: true }), unit("p1", { instanceId: "u2", defId: "classicplus-019-5" }), null, null, null],
        }),
      }),
    );
    const root = screen.getByTestId(testid.card("u1"));
    expect(root.querySelector('.cf-state[data-state="berserk"]')?.getAttribute("aria-label")).toBe(BERSERK_WORDS);
    expect(screen.getByTestId(testid.card("u2")).querySelector('.cf-state[data-state="berserk"]')).toBeNull();
    const preview = hover(root);
    expect(within(preview).getByTestId(INSPECT_STATES).textContent).toBe(BERSERK_WORDS);
    // The same words announce it (`marked`, "berserk"), in the crimson the engine's "red" names.
    expect(markWords("berserk").text).toBe(BERSERK_WORDS);
    expect(markColorOf("red")).toBe("red");
  });
});

/* ----------------------------------------------------------------------------- B2.7 face type */

describe("B2.7 a card the view gives a type of its own draws as that type", () => {
  it("B2.7 Blood Moon's Radiant face is a Field Trap: in hand by the view's type, in the collection by its face's", () => {
    renderBoard(handView({ defId: "classicplus-022", radiant: true, cost: 1, type: "Field Trap" }));
    const face = faceOf(handRoot());
    expect(face.getAttribute("data-card-type")).toBe("Field Trap");
    expect(face.querySelector(".card-type")?.textContent).toBe("Field Trap");
    expect(face.querySelector(".cf-art")?.className).toContain("cf-art--notched");
    expect(faceModel({ defId: "classicplus-022", def: def("classicplus-022"), radiant: true }).type).toBe("Field Trap");
    expect(faceModel({ defId: "classicplus-022", def: def("classicplus-022"), radiant: false }).type).toBe("Trap");
  });

  it("B2.7 a face-up backrow card draws as the type its view names", () => {
    const entry = faceUpBackrow("p1", { instanceId: "b1", defId: "classicplus-022", radiant: true, type: "Field Trap" });
    renderBoard(baseView({ you: emptySide("p1", { backrow: [entry, null, null, null, null] }) }));
    const face = faceOf(screen.getByTestId(testid.card("b1")));
    expect(face.getAttribute("data-card-type")).toBe("Field Trap");
    expect(face.querySelector(".card-type")?.textContent).toBe("Field Trap");
  });
});

/* ------------------------------------------------------------------------- E36 lines of code */

// R693 (#88): lines of code is a hidden stat in a match. The view still carries every count (§5) and
// the face model reads it, which is what the collection's overlays print (inspect.test.tsx), but no
// overlay opened from the board ends with it.
describe("E36 a card's lines of code in the inspect overlays", () => {
  it("E36 R693 the board's hover preview and sheet end with no lines of code, though the face carries them", () => {
    vi.useFakeTimers();
    const gary = def("core-004");
    const loc = must(gary.loc, "Gary's count");
    expect(faceModel({ defId: "core-004", def: gary, radiant: false }).loc).toBe(loc);
    renderBoard(handView({ defId: "core-004", cost: 1, attack: 1, health: 1 }));
    expect(within(hover(handRoot())).queryByTestId(INSPECT_LOC)).toBeNull();
    fireEvent.pointerLeave(handRoot(), { pointerType: "mouse" });
    expect(within(longPress(handRoot())).queryByTestId(INSPECT_LOC)).toBeNull();
  });

  it("E36 a fused card's count is the sum its definition carries (the engine's Fuse adds its ingredients' up), and the board hides it (R693)", () => {
    vi.useFakeTimers();
    const first = def("core-004");
    const second = def("core-019");
    const sum = must(first.loc, "a count") + must(second.loc, "a count");
    const fused = { ...fusedDef([first, second], 4), loc: sum };
    expect(faceModel({ defId: fused.id, def: fused, radiant: false }).loc).toBe(sum);
    renderBoard(
      baseView({
        you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: fused.id, cost: 4, attack: 2, health: 2 })] }),
        defs: { [fused.id]: fused },
      }),
    );
    expect(within(hover(handRoot())).queryByTestId(INSPECT_LOC)).toBeNull();
  });

  it("E36 a card nobody counted has no line", () => {
    vi.useFakeTimers();
    // A match-made definition the engine wrote no count on.
    const uncounted = { ...fusedDef([def("core-004"), def("core-019")], 4) };
    delete uncounted.loc;
    renderBoard(
      baseView({
        you: emptySide("p1", { hand: [card({ instanceId: "h1", defId: uncounted.id, cost: 4, attack: 2, health: 2 })] }),
        defs: { [uncounted.id]: uncounted },
      }),
    );
    expect(within(hover(handRoot())).queryByTestId(INSPECT_LOC)).toBeNull();
  });
});

/* --------------------------------------------------------------------- the glossary of states */

describe("R512 the glossary beside a face explains the states it wears", () => {
  it("R512 Brittle, Upgrade, Degrade, an added keyword, Cast on draw and Animated join the terms its text names", () => {
    const face = inPlay("core-004", {
      brittle: 1,
      tuning: { attack: 1, health: -1, addKeywords: [{ kind: "Lifesteal" }] },
      enchantments: [{ kind: "castOnDraw" }],
      animated: {},
    });
    const ids = glossaryFor(face).map((entry) => entry.id);
    expect(ids).toEqual(expect.arrayContaining(["Cry", "Brittle", "Upgrade", "Degrade", "Lifesteal", "Cast on draw", "Animated"]));
    expect(glossaryFor(inPlay("core-004")).map((entry) => entry.id)).toEqual(["Cry"]);
  });
});
