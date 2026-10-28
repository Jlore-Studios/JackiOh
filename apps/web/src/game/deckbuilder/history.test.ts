// #2099: the open deck's Undo and Redo history, each step an ordinary `updateDeck` edit.

import { describe, expect, it } from "vitest";

import {
  emptyHistory,
  historyKey,
  inTextField,
  recorded,
  redone,
  sealed,
  undone,
  type DeckHistory,
} from "./history.ts";
import type { DeckItem } from "./sync.ts";

const DECK: DeckItem = { id: "a", name: "Aggro", cards: ["c1"], portrait: null, createdAt: 1, updatedAt: 1 };

function withCards(deck: DeckItem, cards: readonly string[]): DeckItem {
  return { ...deck, cards };
}

describe("issue #2099 the deck's undo history", () => {
  it("issue #2099 an edit is one step holding what it overwrote, and a new edit after Undo empties Redo", () => {
    const afterAdd = withCards(DECK, ["c1", "c2"]);
    const history = recorded(emptyHistory("a"), DECK, { cards: ["c1", "c2"] }, false);
    expect(history.past).toEqual([{ before: { cards: ["c1"] }, after: { cards: ["c1", "c2"] }, naming: false }]);

    const back = undone(history);
    expect(back?.patch).toEqual({ cards: ["c1"] });
    expect(back?.history.future).toHaveLength(1);

    expect(back === null).toBe(false);
    if (back === null) return;
    const next = recorded(back.history, afterAdd, { portrait: "gary" }, false);
    expect(next.future).toEqual([]);
  });

  it("issue #2099 an edit that changes nothing is no step", () => {
    const history = emptyHistory("a");
    expect(recorded(history, DECK, { name: "Aggro" }, false)).toBe(history);
    expect(recorded(history, DECK, { cards: ["c1"] }, false)).toBe(history);
    expect(recorded(history, DECK, { fieldCosmetic: null }, false)).toBe(history);
  });

  it("issue #2099 a rename's keys fold into one step until sealed, and a rename back to where it began is none", () => {
    const naming = (deck: DeckItem, name: string, history: DeckHistory): DeckHistory => recorded(history, deck, { name }, true);
    const one = naming(DECK, "A", emptyHistory("a"));
    const two = naming({ ...DECK, name: "A" }, "Ab", one);
    expect(two.past).toEqual([{ before: { name: "Aggro" }, after: { name: "Ab" }, naming: true }]);

    const afterSeal = sealed(two);
    expect(afterSeal.past).toEqual([{ before: { name: "Aggro" }, after: { name: "Ab" }, naming: false }]);
    const three = naming({ ...DECK, name: "Ab" }, "Abc", afterSeal);
    expect(three.past).toHaveLength(2);

    const back = naming({ ...DECK, name: "Agg" }, "Aggro", naming(DECK, "Agg", emptyHistory("a")));
    expect(back.past).toEqual([]);

    const cardsAfterRename = recorded(two, { ...DECK, name: "Ab" }, { cards: ["c1", "c2"] }, false);
    expect(cardsAfterRename.past).toHaveLength(2);
    expect(cardsAfterRename.past[0]).toMatchObject({ naming: false });
  });

  it("issue #2099 undone and redone move one step each way, and are null with nothing to move", () => {
    expect(undone(emptyHistory("a"))).toBeNull();
    expect(redone(emptyHistory("a"))).toBeNull();

    const history = recorded(emptyHistory("a"), DECK, { tableCosmetic: "table-felt" }, false);
    const back = undone(history);
    expect(back?.patch).toEqual({ tableCosmetic: null });
    expect(back === null).toBe(false);
    if (back === null) return;
    const forward = redone(back.history);
    expect(forward?.patch).toEqual({ tableCosmetic: "table-felt" });
    expect(forward?.history).toEqual(history);
  });

  it("issue #2099 an edit of another deck starts its own history", () => {
    const history = recorded(emptyHistory("a"), DECK, { cards: ["c1", "c2"] }, false);
    const other = recorded(history, { ...DECK, id: "b" }, { cards: ["c1", "c2"] }, false);
    expect(other.deckId).toBe("b");
    expect(other.past).toHaveLength(1);
    expect(other.past[0]).toMatchObject({ before: { cards: ["c1"] } });
  });

  it('issue #2099 historyKey reads Ctrl/Cmd+Z as undo and Ctrl/Cmd+Shift+Z (key "Z" or "z") as redo, nothing else', () => {
    const key = (overrides: Partial<Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "shiftKey" | "altKey">>): Pick<KeyboardEvent, "key" | "ctrlKey" | "metaKey" | "shiftKey" | "altKey"> => ({
      key: "z",
      ctrlKey: false,
      metaKey: false,
      shiftKey: false,
      altKey: false,
      ...overrides,
    });
    expect(historyKey(key({ ctrlKey: true }))).toBe("undo");
    expect(historyKey(key({ metaKey: true }))).toBe("undo");
    expect(historyKey(key({ metaKey: true, key: "Z", shiftKey: true }))).toBe("redo");
    expect(historyKey(key({ ctrlKey: true, shiftKey: true, key: "Z" }))).toBe("redo");
    expect(historyKey(key({}))).toBeNull();
    expect(historyKey(key({ ctrlKey: true, key: "y" }))).toBeNull();
    expect(historyKey(key({ ctrlKey: true, altKey: true }))).toBeNull();
    expect(historyKey(key({ metaKey: true, altKey: true, shiftKey: true }))).toBeNull();
  });

  it("issue #2099 inTextField is a text input, a search box or a textarea, never a checkbox, a select, a button, the body or null", () => {
    const input = (type: string): HTMLInputElement => {
      const field = document.createElement("input");
      field.type = type;
      return field;
    };
    expect(inTextField(input("text"))).toBe(true);
    expect(inTextField(input("search"))).toBe(true);
    expect(inTextField(document.createElement("textarea"))).toBe(true);
    expect(inTextField(input("checkbox"))).toBe(false);
    expect(inTextField(input("button"))).toBe(false);
    expect(inTextField(input("range"))).toBe(false);
    expect(inTextField(document.createElement("select"))).toBe(false);
    expect(inTextField(document.createElement("button"))).toBe(false);
    expect(inTextField(document.body)).toBe(false);
    expect(inTextField(null)).toBe(false);
  });
});
