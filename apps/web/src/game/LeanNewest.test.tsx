// "More cards from" (R1371, R1372, R1373, R924000): the picker names the newest set that ships from
// `newestShippedSet()`, offers every set that ships and no other, asks the deck builder's own share,
// and the online pick's store reads R1372's switch and survives a refused `localStorage`.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { AI_DECK } from "@jackioh/ai";
import { DECK_SIZE } from "@jackioh/engine/config";
import { LEAN_NEWEST, SHIPPED_SETS, isLeanPick, leanPicks, leanSetOf, newestShippedSet, setShips, type LeanPick } from "@jackioh/shared";

import {
  LeanSetPicker,
  PLAY_LEAN_NEWEST_KEY,
  PLAY_LEAN_SET_KEY,
  leanChoiceLabel,
  leanFloor,
  leanLabel,
  leanNote,
  readPlayLean,
  writePlayLean,
} from "./LeanNewest.tsx";

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
  try {
    window.localStorage.clear();
  } catch {
    // nothing to clear
  }
});

describe("R1371 the newest set that ships", () => {
  it("R1371 newestShippedSet is SHIPPED_SETS' last entry, Classic+ until the Meditative set ships", () => {
    expect(newestShippedSet()).toBe(SHIPPED_SETS[SHIPPED_SETS.length - 1]);
    expect(setShips(newestShippedSet())).toBe(true);
    // Meditative from the patch that ships it (issue #553), with no other line to change.
    expect(newestShippedSet()).toBe(setShips("Meditative") ? "Meditative" : "Classic+");
  });

  it("R1371 the picker's newest choice names it, and the share it asks for is the deck builder's own number", () => {
    expect(AI_DECK.leanMinShare).toBe(0.5);
    expect(leanFloor()).toBe(Math.ceil(DECK_SIZE * AI_DECK.leanMinShare));
    expect(leanFloor(25)).toBe(13);
    expect(leanChoiceLabel(LEAN_NEWEST)).toBe(`The newest set (${newestShippedSet()})`);
    expect(leanNote(LEAN_NEWEST)).toBe(`At least 10 of your ${String(DECK_SIZE)} cards come from ${newestShippedSet()}.`);
  });
});

describe("R924000 the picker's choices", () => {
  it("R924000 offers none, newest, then every set that ships in catalog order, and nothing else", () => {
    expect(leanPicks()).toEqual([LEAN_NEWEST, ...SHIPPED_SETS]);
    for (const pick of leanPicks()) expect(isLeanPick(pick)).toBe(true);
    for (const junk of ["Boss", "Boss-X", "NEWEST", "", null, 3, true]) expect(isLeanPick(junk)).toBe(false);
    expect(leanSetOf(LEAN_NEWEST)).toBe(newestShippedSet());
    expect(leanSetOf("Classic")).toBe("Classic");

    const changes: (LeanPick | null)[] = [];
    render(<LeanSetPicker value={null} testid="lean" onChange={(pick) => changes.push(pick)} />);
    const picker = screen.getByTestId("lean") as HTMLSelectElement;
    expect(picker.closest("label")).toHaveTextContent(leanLabel());
    const options = [...picker.options].map((option) => [option.value, option.textContent]);
    expect(options).toEqual([
      ["", leanChoiceLabel(null)],
      [LEAN_NEWEST, `The newest set (${newestShippedSet()})`],
      ...SHIPPED_SETS.map((set) => [set, set]),
    ]);
    expect(picker.value).toBe("");
    // No pick, no note.
    expect(picker).not.toHaveAccessibleDescription();

    fireEvent.change(picker, { target: { value: "Classic" } });
    fireEvent.change(picker, { target: { value: "" } });
    expect(changes).toEqual(["Classic", null]);
  });

  it("R924000 a pick shows the note naming the set it leans on", () => {
    render(<LeanSetPicker value="Core" testid="lean" onChange={() => undefined} />);
    const picker = screen.getByTestId("lean") as HTMLSelectElement;
    expect(picker.value).toBe("Core");
    expect(picker).toHaveAccessibleDescription(leanNote("Core"));
    expect(leanNote("Core")).toBe(`At least 10 of your ${String(DECK_SIZE)} cards come from Core.`);
  });
});

describe("R924000 the online pick on this device", () => {
  it("R924000 reads back what was written, and none when nothing was, junk was or storage refuses", () => {
    expect(readPlayLean()).toBeNull();
    writePlayLean("Classic+");
    expect(window.localStorage.getItem(PLAY_LEAN_SET_KEY)).toBe("Classic+");
    expect(readPlayLean()).toBe("Classic+");
    writePlayLean(null);
    expect(readPlayLean()).toBeNull();
    window.localStorage.setItem(PLAY_LEAN_SET_KEY, "Boss");
    expect(readPlayLean()).toBeNull();

    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    expect(readPlayLean()).toBeNull();
    expect(() => {
      writePlayLean(LEAN_NEWEST);
    }).not.toThrow();
  });

  it("R1372 R924000 a device that kept the switch on reads it as newest until a pick is written", () => {
    window.localStorage.setItem(PLAY_LEAN_NEWEST_KEY, "true");
    expect(readPlayLean()).toBe(LEAN_NEWEST);
    window.localStorage.setItem(PLAY_LEAN_NEWEST_KEY, "false");
    expect(readPlayLean()).toBeNull();
    window.localStorage.setItem(PLAY_LEAN_NEWEST_KEY, "true");
    writePlayLean("Core");
    expect(window.localStorage.getItem(PLAY_LEAN_NEWEST_KEY)).toBeNull();
    expect(readPlayLean()).toBe("Core");
  });
});
