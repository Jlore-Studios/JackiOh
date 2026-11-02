// The custom cards the Sandbox's pool takes (DR1, B21, SPEC §12.9): the creator's cards are kept on
// the account, and practice reads an active account's as a game's own cards (`routes/practice.tsx`).
// The Sandbox reads the same list the same way, so a card made in the creator joins the run's pool;
// a visitor with no account has none. The card builder's model is the creator's alone and is dropped
// (R915006's game card).

import { useMemo } from "react";

import type { CustomCard } from "@jackioh/shared";

import { gameCustomCards, useCustomCards } from "../creator/useCustomCards.ts";
import { getCustomCards } from "../net/customCards.ts";
import type { Account } from "../net/gate.ts";

/** One list for every render that has no cards, so a pool built from it is built once. */
const NONE: CustomCard[] = [];

/**
 * The custom cards a Sandbox run's pool takes: an active account's, read once per token as practice
 * reads them (`load` is the test's seam), otherwise none. The same array while nothing changes.
 */
export function useSandboxCustomCards(account: Account, load = getCustomCards): CustomCard[] {
  const token = account.kind === "ready" && account.me.profile.status === "active" ? account.token : null;
  const { state } = useCustomCards(token, load);
  const cards = state.kind === "ready" ? state.cards : null;
  return useMemo(() => (cards === null || cards.length === 0 ? NONE : gameCustomCards(cards)), [cards]);
}
