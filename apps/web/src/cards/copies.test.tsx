// Classic #57 Echo's copied text (B5 E14, R399, R545–R547, R511): the view carries `copies` (the
// Spell whose text a copier has, on the face it was played on, with that definition's numbers as they
// read on the card) on its owner's hand view and on both views while it resolves. The face prints that
// text, filled through the face's own text path (`fillParams`), in place of its copying sentence, and
// keeps its own name, cost, type and art; the inspect notes say what it copies.
//
// The fixtures are shaped as `viewFor` builds the view; the last block plays the real engine.

import { CATALOG } from "@jackioh/cards";
import type { CardView } from "@jackioh/shared";
import { HUMAN_HANDICAP } from "@jackioh/engine/config";
import { act, cleanup, fireEvent, render, screen, within } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { devDeck, playOf, realGame } from "../audio/test/realGame.ts";
import Board from "../game/Board.tsx";
import { CatalogContext, lookupFromDefs } from "../game/catalog.ts";
import { testid } from "../game/contract.ts";
import { listedFace } from "../game/faces.ts";
import { baseView, card, emptySide } from "../test/fixtures.ts";
import { HOVER_DELAY_MS, LONG_PRESS_MS } from "./inspect/constants.ts";
import { copyingWords } from "./inspect/StateNotes.tsx";
import { closeInspect } from "./inspect/store.ts";
import { INSPECT_HOVER, INSPECT_PRINTED, INSPECT_SHEET, INSPECT_STATES } from "./inspect/testids.ts";

afterEach(() => {
  act(() => {
    closeInspect();
  });
  cleanup();
  vi.useRealTimers();
});

const ECHO = "classic-057";
/** (1) Spell: "Draw {draw}." — 3 on its base face, 6 on its Radiant face. */
const KNOWLEDGE = "classic-024";
/** (1) Spell, no declared numbers: "Draw 2. Heal your hero 2." */
const STOCKPILE = "core-005";
const ECHO_SENTENCE = "This has the text of the last Spell either player played.";

const lookup = lookupFromDefs(CATALOG);

function must<T>(value: T | null | undefined, what: string): T {
  if (value === null || value === undefined) throw new Error(`expected ${what}`);
  return value;
}

function withCatalog(node: ReactElement, catalog = lookup): ReactElement {
  return <CatalogContext.Provider value={catalog}>{node}</CatalogContext.Provider>;
}

function echo(over: Partial<CardView> = {}): CardView {
  return card({ instanceId: "e1", defId: ECHO, cost: 1, ...over });
}

function renderHand(cards: CardView[]): void {
  render(withCatalog(<Board view={baseView({ you: emptySide("p1", { hand: cards }) })} />));
}

function face(root: HTMLElement): HTMLElement {
  return must(root.querySelector<HTMLElement>(".cf"), "a face");
}

function rulesText(root: HTMLElement): string {
  return must(root.querySelector(".cf-text-base"), "the rules box").textContent ?? "";
}

function hover(element: HTMLElement): HTMLElement {
  fireEvent.pointerEnter(element, { pointerType: "mouse" });
  act(() => {
    vi.advanceTimersByTime(HOVER_DELAY_MS * 2);
  });
  return screen.getByTestId(INSPECT_HOVER);
}

function longPress(element: HTMLElement): HTMLElement {
  fireEvent.pointerDown(element, { pointerType: "touch", clientX: 10, clientY: 10 });
  act(() => {
    vi.advanceTimersByTime(LONG_PRESS_MS * 2);
  });
  fireEvent.pointerUp(element, { pointerType: "touch", clientX: 10, clientY: 10 });
  return screen.getByTestId(INSPECT_SHEET);
}

describe("R399 R511 Echo prints the text it copies", () => {
  it("R511 the words of the note: \"Copying: Book of Knowledge\", \"(Radiant)\" on a Radiant copy", () => {
    expect(copyingWords({ name: "Book of Knowledge", radiant: false })).toBe("Copying: Book of Knowledge");
    expect(copyingWords({ name: "Book of Knowledge", radiant: true })).toBe("Copying: Book of Knowledge (Radiant)");
  });

  it("R511 in hand it prints the copied face's text with the numbers it reads, under its own name, cost and type", () => {
    renderHand([echo({ copies: { defId: KNOWLEDGE, radiant: false, params: { draw: 3 } } })]);
    const root = screen.getByTestId(testid.handCard("e1"));
    expect(rulesText(root)).toBe("Draw 3.");
    expect(root.querySelector(".card-name")?.textContent).toBe("Echo");
    expect(root.querySelector(".cost-gem")?.getAttribute("data-cost")).toBe("1");
    expect(face(root).getAttribute("data-card-type")).toBe("Spell");
    expect(root.textContent).not.toContain("{");
  });

  it("R511 a number the copy reads off its printed value is drawn as it reads, marked as moved", () => {
    renderHand([echo({ copies: { defId: KNOWLEDGE, radiant: false, params: { draw: 5 } } })]);
    const root = screen.getByTestId(testid.handCard("e1"));
    expect(rulesText(root)).toBe("Draw 5.");
    const moved = must(root.querySelector<HTMLElement>(".cf-tuned"), "the moved number");
    expect(moved.textContent).toBe("5");
    expect(moved.getAttribute("data-way")).toBe("better");
  });

  it("R511 a copy of a Spell with no declared numbers is its text as printed, on the face it was played on", () => {
    renderHand([echo({ copies: { defId: STOCKPILE, radiant: true } })]);
    expect(rulesText(screen.getByTestId(testid.handCard("e1")))).toBe("Draw 5. Heal your hero 5.");
  });

  it("R511 a Radiant Echo keeps its own Echo 1, marked as its Radiant face marks it, over the text it copies", () => {
    renderHand([echo({ radiant: true, copies: { defId: KNOWLEDGE, radiant: true, params: { draw: 6 } } })]);
    const root = screen.getByTestId(testid.handCard("e1"));
    expect(rulesText(root)).toBe("Echo 1\nDraw 6.");
    const marks = Array.from(root.querySelectorAll(".cf-text-base .cf-mark")).map((mark) => mark.textContent ?? "");
    expect(marks.join(" ")).toContain("Echo 1");
    // Book of Knowledge's Radiant face draws 6 where its base face draws 3: the copy marks it too.
    expect(marks).toContain("6");
  });

  it("R399 an Echo that copies nothing prints its own text and has no note", () => {
    vi.useFakeTimers();
    renderHand([echo()]);
    const root = screen.getByTestId(testid.handCard("e1"));
    expect(rulesText(root)).toBe(ECHO_SENTENCE);
    const preview = hover(root);
    expect(preview.querySelector('[data-state="copies"]')).toBeNull();
    expect(within(preview).queryByTestId(INSPECT_PRINTED)).toBeNull();
  });

  it("R511 the hover preview and the sheet say what it copies, and give Echo's own printed text beside the copy", () => {
    vi.useFakeTimers();
    renderHand([echo({ copies: { defId: KNOWLEDGE, radiant: false, params: { draw: 3 } } })]);
    const root = screen.getByTestId(testid.handCard("e1"));
    const preview = hover(root);
    const note = must(within(preview).getByTestId(INSPECT_STATES).querySelector<HTMLElement>('[data-state="copies"]'), "the note");
    expect(note.getAttribute("data-copies")).toBe(KNOWLEDGE);
    expect(note.querySelector(".inspect-state-words")?.textContent).toBe("Copying: Book of Knowledge");
    expect(within(preview).getByTestId(INSPECT_PRINTED).textContent).toContain(ECHO_SENTENCE);
    fireEvent.pointerLeave(root, { pointerType: "mouse" });

    const sheet = longPress(root);
    expect(within(sheet).getByTestId(INSPECT_STATES).textContent).toContain("Copying: Book of Knowledge");
  });

  it("R399 while it resolves both players' boards draw the copy in the resolving strip, and the play reveal's face does too", () => {
    const resolving = echo({ instanceId: "r1", copies: { defId: KNOWLEDGE, radiant: false, params: { draw: 3 } } });
    const view = baseView({ opponent: emptySide("p2", { resolving: [resolving], hand: { count: 3 } }) });
    render(withCatalog(<Board view={view} />));
    const root = within(screen.getByTestId("resolving-opponent")).getByTestId(testid.card("r1"));
    expect(rulesText(root)).toBe("Draw 3.");
    expect(root.querySelector(".card-name")?.textContent).toBe("Echo");
    // The showcase and the log draw a card the view lists through listedFace.
    const listed = must(listedFace(lookup, view, resolving), "the listed face");
    expect(listed.text.full).toBe("Draw 3.");
    expect(listed.copying).toEqual({ defId: KNOWLEDGE, name: "Book of Knowledge", radiant: false });
  });
});

describe("R399 R511 Echo through the real engine", () => {
  it("R399 R511 once a Spell is played, Echo in its owner's hand prints that Spell's text, and the other seat sees no hand card", () => {
    // A four-card deck all dealt at once (R182, R184), so Echo and Book of Knowledge are both in hand.
    const rest = devDeck("cheap20").filter((id) => id !== ECHO && id !== KNOWLEDGE);
    const deck = [ECHO, KNOWLEDGE, ...rest.slice(0, 2)];
    const handicap = { ...HUMAN_HANDICAP, deckSize: deck.length, extraOpeningCards: 1 };
    const game = realGame("echo-copies", [deck, devDeck("first20")], { p1: handicap });
    const catalog = lookupFromDefs(game.catalog);
    const hand = (): CardView[] => game.view("p1").you.hand as CardView[];
    const echoCard = must(hand().find((entry) => entry.defId === ECHO), "Echo in hand");
    const knowledge = must(hand().find((entry) => entry.defId === KNOWLEDGE), "Book of Knowledge in hand");
    expect(echoCard.copies).toBeUndefined();

    game.act("p1", must(playOf(game, "p1", knowledge.instanceId), "a legal play of Book of Knowledge"));
    const copied = must(hand().find((entry) => entry.instanceId === echoCard.instanceId), "Echo still in hand");
    expect(copied.copies).toEqual({ defId: KNOWLEDGE, radiant: false, params: { draw: 3 } });
    expect(Array.isArray(game.view("p2").opponent.hand)).toBe(false);

    vi.useFakeTimers();
    render(withCatalog(<Board view={game.view("p1")} />, catalog));
    const root = screen.getByTestId(testid.handCard(echoCard.instanceId));
    expect(rulesText(root)).toBe("Draw 3.");
    expect(root.querySelector(".card-name")?.textContent).toBe("Echo");
    const preview = hover(root);
    expect(within(preview).getByTestId(INSPECT_STATES).textContent).toContain("Copying: Book of Knowledge");
  });
});
