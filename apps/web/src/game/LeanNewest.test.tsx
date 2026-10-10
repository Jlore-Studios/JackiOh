// "More cards from the newest set" (R1371, R1372, R1373): the switch names the newest set that ships
// from `newestShippedSet()` and the deck builder's own share, and the online pick's store survives a
// refused `localStorage`.

import { cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import { AI_DECK } from "@jackioh/ai";
import { DECK_SIZE } from "@jackioh/engine/config";
import { SHIPPED_SETS, newestShippedSet, setShips } from "@jackioh/shared";

import {
  LeanNewestToggle,
  PLAY_LEAN_NEWEST_KEY,
  leanNewestFloor,
  leanNewestLabel,
  leanNewestNote,
  readPlayLeanNewest,
  writePlayLeanNewest,
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

  it("R1371 the switch names it, and the share it asks for is the deck builder's own number", () => {
    expect(AI_DECK.leanMinShare).toBe(0.5);
    expect(leanNewestFloor()).toBe(Math.ceil(DECK_SIZE * AI_DECK.leanMinShare));
    expect(leanNewestFloor(25)).toBe(13);
    expect(leanNewestLabel()).toBe(`More cards from the newest set (${newestShippedSet()})`);
    expect(leanNewestNote()).toBe(`At least 10 of your ${String(DECK_SIZE)} cards come from ${newestShippedSet()}.`);

    const changes: boolean[] = [];
    render(<LeanNewestToggle checked={false} testid="lean" onChange={(on) => changes.push(on)} />);
    const box = screen.getByTestId("lean");
    expect(box).not.toBeChecked();
    expect(box.closest("label")).toHaveTextContent(leanNewestLabel());
    expect(box).toHaveAccessibleDescription(leanNewestNote());
    fireEvent.click(box);
    expect(changes).toEqual([true]);
  });
});

describe("R1372 the online pick on this device", () => {
  it("R1372 reads back what was written, and off when nothing was or storage refuses", () => {
    expect(readPlayLeanNewest()).toBe(false);
    writePlayLeanNewest(true);
    expect(window.localStorage.getItem(PLAY_LEAN_NEWEST_KEY)).toBe("true");
    expect(readPlayLeanNewest()).toBe(true);
    window.localStorage.setItem(PLAY_LEAN_NEWEST_KEY, "junk");
    expect(readPlayLeanNewest()).toBe(false);

    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("blocked");
    });
    expect(readPlayLeanNewest()).toBe(false);
    expect(() => {
      writePlayLeanNewest(true);
    }).not.toThrow();
  });
});
