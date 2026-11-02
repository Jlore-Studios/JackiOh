// B21 (DR14 part D, #2705): the Sandbox's pool takes the custom cards the creator saved, read as practice
// reads them (SPEC §12.9): an active account's, as a game carries them, and none for anyone else.

import { cleanup, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { CustomCard } from "@jackioh/shared";

import type { CustomCardsResponse } from "../net/customCards.ts";
import type { Account } from "../net/gate.ts";
import { useSandboxCustomCards } from "./customCards.ts";

/** A card as the account keeps it, with the creator's builder model. */
const WITH_BUILDER: CustomCard = {
  id: "custom-ember-imp",
  version: 2,
  frame: { name: "Ember Imp", type: "Unit", cost: 1, rarity: "Common", attack: 1, health: 2, text: "" },
  source: "",
  builder: { blocks: [] },
};
/** The card as a game carries it: the builder's model is the creator's alone. */
const IN_GAME: CustomCard = { id: WITH_BUILDER.id, version: WITH_BUILDER.version, frame: WITH_BUILDER.frame, source: "" };

function account(status: "active" | "pending"): Account {
  return {
    kind: "ready",
    token: "tok-1",
    me: { profile: { id: "u1", status }, needsInviteCode: false, emailVerified: true, currentMatchId: null, email: "player@example.com" },
  };
}

const answer = (cards: CustomCard[]): CustomCardsResponse => ({ cards, limits: { cards: 50 } });

afterEach(() => {
  cleanup();
});

describe("B21 the custom cards the Sandbox takes", () => {
  it("B21 an active account contributes builder-free custom cards", async () => {
    const load = vi.fn((_token: string) => Promise.resolve(answer([WITH_BUILDER])));
    const ready = account("active");
    const { result, rerender } = renderHook(() => useSandboxCustomCards(ready, load));

    expect(result.current).toEqual([]);
    await waitFor(() => {
      expect(result.current).toEqual([IN_GAME]);
    });
    expect(result.current[0]).not.toHaveProperty("builder");
    expect(load).toHaveBeenCalledTimes(1);
    expect(load).toHaveBeenCalledWith("tok-1");

    // The same list while nothing changes, so the pool built from it is built once.
    const cards = result.current;
    rerender();
    expect(result.current).toBe(cards);
  });

  it("B21 an anonymous account reads none", () => {
    const load = vi.fn((_token: string) => Promise.resolve(answer([WITH_BUILDER])));
    const { result, rerender } = renderHook(() => useSandboxCustomCards({ kind: "anonymous" }, load));

    expect(result.current).toEqual([]);
    const none = result.current;
    rerender();
    expect(result.current).toBe(none);
    expect(load).not.toHaveBeenCalled();
  });

  it("B21 an account that is not active, one still loading and a read that fails have none", async () => {
    const load = vi.fn((_token: string) => Promise.reject(new Error("offline")));
    for (const who of [account("pending"), { kind: "loading" } as const]) {
      const { result } = renderHook(() => useSandboxCustomCards(who, load));
      expect(result.current).toEqual([]);
    }
    expect(load).not.toHaveBeenCalled();

    const { result } = renderHook(() => useSandboxCustomCards(account("active"), load));
    await waitFor(() => {
      expect(load).toHaveBeenCalledTimes(1);
    });
    expect(result.current).toEqual([]);
  });
});
