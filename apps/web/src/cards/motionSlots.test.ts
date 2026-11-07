// R3340000: the motion slots' ranking. A face holds one of COSMETIC_FOIL_ANIMATED_MAX slots by tier (hovered, Mythic,
// Legendary, the rest), the newest first within a tier; a freed slot passes on in the same tick; a re-rank keeps a face's
// age. Faces in the DOM, with the cards that wear them, are artLive.test.tsx's.

import { createElement, type ReactElement } from "react";
import { act, cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { COSMETIC_FOIL_ANIMATED_MAX } from "./constants.ts";
import { REST_RANK, allocate, resetMotionSlotsForTests, slotRarityOf, useMotionSlot, usePageVisible, type SlotRank } from "./motionSlots.ts";

afterEach(() => {
  cleanup();
  resetMotionSlotsForTests();
});

const MAX = COSMETIC_FOIL_ANIMATED_MAX;
const HOVERED: SlotRank = { hovered: true, rarity: null };
const MYTHIC: SlotRank = { hovered: false, rarity: "Mythic" };
const LEGENDARY: SlotRank = { hovered: false, rarity: "Legendary" };

/** A face's claim: `name` is its token's description, so a failure names it. */
const claim = (name: string, rank: SlotRank, seq: number) => ({ token: Symbol(name), rank, seq });

function Slot({ id, wanted = true, rank = REST_RANK }: { id: number; wanted?: boolean; rank?: SlotRank }): ReactElement {
  const holds = useMotionSlot(wanted, rank);
  return createElement("span", { "data-id": id, "data-holds": holds ? "true" : "false" });
}

/** `count` faces, oldest first, each given its own rank by `rankOf`. */
function faces(count: number, rankOf: (id: number) => SlotRank = () => REST_RANK): ReactElement {
  return createElement(
    "div",
    null,
    Array.from({ length: count }, (_unused, id) => createElement(Slot, { key: id, id, rank: rankOf(id) })),
  );
}

/** The ids of the faces holding a slot now, in order. */
const holders = (container: HTMLElement): number[] =>
  [...container.querySelectorAll<HTMLElement>("[data-holds='true']")].map((element) => Number(element.getAttribute("data-id")));

describe("R3340000 the slots are allocated by tier, newest first", () => {
  it("R3340000 hovered faces first, then Mythics (the newest first), Legendaries, then the rest newest first", () => {
    const rest1 = claim("rest1", REST_RANK, 1);
    const legendary = claim("legendary", LEGENDARY, 2);
    const oldMythic = claim("oldMythic", MYTHIC, 3);
    const rest2 = claim("rest2", REST_RANK, 4);
    const hovered = claim("hovered", HOVERED, 5);
    const newMythic = claim("newMythic", MYTHIC, 6);
    const rest3 = claim("rest3", REST_RANK, 7);
    const all = [rest1, legendary, oldMythic, rest2, hovered, newMythic, rest3];
    const names = (max: number): string[] => [...allocate(all, max)].map((token) => token.description ?? "");
    expect(names(1)).toEqual(["hovered"]);
    expect(names(2)).toEqual(["hovered", "newMythic"]);
    expect(names(3)).toEqual(["hovered", "newMythic", "oldMythic"]);
    expect(names(4)).toEqual(["hovered", "newMythic", "oldMythic", "legendary"]);
    expect(names(5)).toEqual(["hovered", "newMythic", "oldMythic", "legendary", "rest3"]);
    expect(names(6)).toEqual(["hovered", "newMythic", "oldMythic", "legendary", "rest3", "rest2"]);
    expect(names(99)).toHaveLength(all.length);
    expect(allocate([], MAX).size).toBe(0);
    // A hovered Mythic is a hovered face, and still outranks a newer Mythic.
    const both = claim("both", { hovered: true, rarity: "Mythic" }, 1);
    expect([...allocate([both, claim("newer", MYTHIC, 9)], 1)][0]).toBe(both.token);
  });

  it("R3340000 a card's rarity is a slot's rank only for a Mythic or a Legendary", () => {
    expect(slotRarityOf("Mythic")).toBe("Mythic");
    expect(slotRarityOf("Legendary")).toBe("Legendary");
    for (const rarity of ["Common", "Rare", "Epic", "Token", null] as const) expect(slotRarityOf(rarity)).toBeNull();
  });

  it("R3340000 faces take the slots newest first and never more than COSMETIC_FOIL_ANIMATED_MAX", () => {
    const { container } = render(faces(MAX + 3));
    expect(holders(container)).toEqual(Array.from({ length: MAX }, (_unused, index) => index + 3));
    // A face that does not want one claims none and takes none from the others.
    const { container: second } = render(createElement("div", null, createElement(Slot, { id: 0, wanted: false })));
    expect(holders(second)).toEqual([]);
    expect(holders(container)).toHaveLength(MAX);
  });

  it("R3340000 a Mythic and a Legendary hold a slot ahead of newer faces, a Mythic first", () => {
    const rankOf = (id: number): SlotRank => (id === 0 ? LEGENDARY : id === 1 ? MYTHIC : REST_RANK);
    const { container } = render(faces(MAX + 3, rankOf));
    // The two oldest faces are the Mythic and the Legendary: both hold, and the newest rest take the others.
    expect(holders(container)).toEqual([0, 1, ...Array.from({ length: MAX - 2 }, (_unused, index) => index + 5)]);
  });
});

describe("R3340000 a freed slot passes on at once", () => {
  it("R3340000 a holder that unmounts hands its slot to the next face in the same tick", () => {
    const count = MAX + 2;
    const { container, rerender } = render(faces(count));
    expect(holders(container)).toEqual(Array.from({ length: MAX }, (_unused, index) => index + 2));
    // The newest holder leaves: the next face in line holds at once, read before any timer or frame runs.
    act(() => {
      rerender(faces(count - 1));
    });
    expect(holders(container)).toEqual(Array.from({ length: MAX }, (_unused, index) => index + 1));
    act(() => {
      rerender(faces(count - 2));
    });
    expect(holders(container)).toEqual(Array.from({ length: MAX }, (_unused, index) => index));
  });

  it("R3340000 a face that stops wanting a slot frees it for the next", () => {
    const wants = (second: boolean): ReactElement =>
      createElement(
        "div",
        null,
        Array.from({ length: MAX + 1 }, (_unused, id) => createElement(Slot, { key: id, id, wanted: id === MAX ? second : true })),
      );
    const { container, rerender } = render(wants(true));
    expect(holders(container)).toEqual(Array.from({ length: MAX }, (_unused, index) => index + 1));
    act(() => {
      rerender(wants(false));
    });
    expect(holders(container)).toEqual(Array.from({ length: MAX }, (_unused, index) => index));
  });
});

describe("R3340000 a re-rank keeps a face's age", () => {
  it("R3340000 a hovered face takes a slot and gives it back to the face it took it from, and is not made the newest", () => {
    const hover = (on: boolean) => (id: number): SlotRank => (id === 0 && on ? HOVERED : REST_RANK);
    const { container, rerender } = render(faces(MAX + 1));
    expect(holders(container)).toEqual(Array.from({ length: MAX }, (_unused, index) => index + 1));
    act(() => {
      rerender(faces(MAX + 1, hover(true)));
    });
    // The oldest face is hovered: it holds, and the oldest of the others (the next in age) gives its slot up.
    expect(holders(container)).toEqual([0, ...Array.from({ length: MAX - 1 }, (_unused, index) => index + 2)]);
    act(() => {
      rerender(faces(MAX + 1, hover(false)));
    });
    // Left again, it is the oldest still: it was not made the newest by being hovered.
    expect(holders(container)).toEqual(Array.from({ length: MAX }, (_unused, index) => index + 1));
  });
});

describe("R3340000 the page's visibility", () => {
  it("R3340000 usePageVisible follows the document's visibilitychange", () => {
    let seen = true;
    function Probe(): null {
      seen = usePageVisible();
      return null;
    }
    const state = (value: DocumentVisibilityState): void => {
      Object.defineProperty(document, "visibilityState", { configurable: true, get: () => value });
      document.dispatchEvent(new Event("visibilitychange"));
    };
    render(createElement(Probe));
    expect(seen).toBe(true);
    act(() => {
      state("hidden");
    });
    expect(seen).toBe(false);
    act(() => {
      state("visible");
    });
    expect(seen).toBe(true);
    delete (document as { visibilityState?: unknown }).visibilityState;
  });
});
