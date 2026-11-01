// The account's custom cards (`GET /api/custom-cards`) as a screen reads them (SPEC §12.9, R915008),
// each drawn from its frame's definition (`customDef`: no Luau is loaded to draw, R915009).

import { useCallback, useEffect, useRef, useState } from "react";

import { CUSTOM_ID_PREFIX } from "@jackioh/engine/config";
import type { CardDef, CardDefs, CustomCard } from "@jackioh/shared";

import { readFailureOf } from "../net/gate.ts";
import { getCustomCards, type CustomCardsResponse } from "../net/customCards.ts";
import { customDef } from "../wire/validator.ts";

export type CustomCardsState =
  | { kind: "none" }
  | { kind: "loading" }
  | { kind: "ready"; cards: CustomCard[]; limit: number }
  /** The read failed; `retryAfterMs` is the wait a rate limit stated (R192), which Try again counts down (#1691). */
  | { kind: "failed"; retryAfterMs?: number };

/**
 * An active account's custom cards, read once per token and again on `reload`; `token` null reads
 * nothing. `load` is the route's, so a test hands its own.
 */
export function useCustomCards(
  token: string | null,
  load: (token: string) => Promise<CustomCardsResponse> = getCustomCards,
): { state: CustomCardsState; reload: () => void; replace: (cards: CustomCard[]) => void } {
  const [loaded, setLoaded] = useState<{ token: string; state: CustomCardsState } | null>(null);
  const [generation, setGeneration] = useState(0);
  const loader = useRef(load);
  loader.current = load;

  useEffect(() => {
    if (token === null) return;
    let canceled = false;
    let pending: Promise<CustomCardsResponse>;
    try {
      pending = loader.current(token);
    } catch (cause) {
      pending = Promise.reject(cause instanceof Error ? cause : new Error(String(cause)));
    }
    pending.then(
      (response) => {
        if (canceled) return;
        const cards = Array.isArray(response.cards) ? response.cards : [];
        setLoaded({ token, state: { kind: "ready", cards, limit: response.limits.cards } });
      },
      (cause: unknown) => {
        if (canceled) return;
        const { retryAfterMs } = readFailureOf(cause);
        setLoaded({ token, state: retryAfterMs === undefined ? { kind: "failed" } : { kind: "failed", retryAfterMs } });
      },
    );
    return () => {
      canceled = true;
    };
  }, [token, generation]);

  const reload = useCallback(() => {
    // A failed read shows loading until the answer, so Try again is not offered twice (and counts its own wait).
    setLoaded((current) => (current !== null && current.state.kind === "failed" ? null : current));
    setGeneration((value) => value + 1);
  }, []);
  /** After a save or a delete: the list as the server now holds it, without a second read. */
  const replace = useCallback(
    (cards: CustomCard[]) => {
      if (token === null) return;
      setLoaded((current) =>
        current !== null && current.token === token && current.state.kind === "ready"
          ? { token, state: { ...current.state, cards } }
          : current,
      );
    },
    [token],
  );

  const state: CustomCardsState =
    token === null ? { kind: "none" } : loaded !== null && loaded.token === token ? loaded.state : { kind: "loading" };
  return { state, reload, replace };
}

/** Each card's definition by its id (`customDef`); a card whose frame the engine refuses is left out. */
export function customDefsOf(cards: readonly CustomCard[]): CardDefs {
  const defs: Record<string, CardDef> = {};
  for (const card of cards) {
    try {
      defs[card.id] = customDef(card);
    } catch {
      // A card saved under other limits: it is still listed, but cannot be drawn or played.
    }
  }
  return defs;
}

/** The custom cards a deck names, in the order it first names them; ids it names that are gone are `missing`. */
export function customCardsOfDeck(
  deck: readonly string[],
  cards: readonly CustomCard[],
): { cards: CustomCard[]; missing: string[] } {
  const byId = new Map(cards.map((card) => [card.id, card]));
  const found: CustomCard[] = [];
  const missing: string[] = [];
  const seen = new Set<string>();
  for (const id of deck) {
    if (!id.startsWith(CUSTOM_ID_PREFIX) || seen.has(id)) continue;
    seen.add(id);
    const card = byId.get(id);
    if (card === undefined) missing.push(id);
    else found.push(card);
  }
  return { cards: found, missing };
}

/** Whether a deck names a custom card: R915005's practice deck, which no online path offers. */
export function namesCustomCard(deck: readonly string[]): boolean {
  return deck.some((id) => id.startsWith(CUSTOM_ID_PREFIX));
}

/** The cards as a game carries them (R915006): the builder's model is the creator's alone, and is dropped. */
export function gameCustomCards(cards: readonly CustomCard[]): CustomCard[] {
  return cards.map(({ builder: _builder, ...card }) => card);
}
