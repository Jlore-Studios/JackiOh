// The opening deal (#3347): the opening hands are dealt one card at a time from each deck to its hand,
// after the decks riffle. The engine sends no event for it, and the first view a board sees never animates
// (Game.tsx), so the deal is read off the view itself: a game whose first view is in the mulligan phase
// is a game being dealt. Only counts and places are read, never a card (the opponent's hand is backs,
// CLAUDE.md rule 7). Pure plan here; Game.tsx runs it, board.css draws it from the properties `aimDeal` writes.

import type { PlayerView } from "@jackioh/shared";

import { normalizeSpeed } from "../fx/settings.ts";
import { DEAL_FLIGHT_MS, DEAL_STAGGER_MS, RIFFLE_MS } from "./config.ts";
import type { Side } from "./contract.ts";

export type OpeningPlan = {
  /** How long the decks riffle before the first card is dealt, at the viewer's speed (R201). */
  riffleMs: number;
  /** How far apart the cards are dealt. */
  staggerMs: number;
  /** How long one card takes to fly from its deck to its hand. */
  flightMs: number;
  /** Every card in the order it is dealt, you first, alternating, with when it leaves the deck. */
  deals: { side: Side; atMs: number }[];
  /** When the last card has landed. */
  totalMs: number;
};

const scaled = (ms: number, speed: number): number => Math.round(ms / normalizeSpeed(speed));

function handSize(hand: PlayerView["you"]["hand"]): number {
  return Array.isArray(hand) ? hand.length : hand.count;
}

/** The deal of a view in the mulligan phase, or null: no other view is a deal, and a game with no cards dealt has none. */
export function planOpening(view: PlayerView, speed: number): OpeningPlan | null {
  if (view.phase !== "mulligan") return null;
  const left: Record<Side, number> = { you: handSize(view.you.hand), opponent: handSize(view.opponent.hand) };
  if (left.you <= 0 && left.opponent <= 0) return null;
  const riffleMs = scaled(RIFFLE_MS, speed);
  const staggerMs = scaled(DEAL_STAGGER_MS, speed);
  const flightMs = scaled(DEAL_FLIGHT_MS, speed);
  const deals: OpeningPlan["deals"] = [];
  while (left.you > 0 || left.opponent > 0) {
    for (const side of ["you", "opponent"] as const) {
      if (left[side] <= 0) continue;
      left[side] -= 1;
      deals.push({ side, atMs: riffleMs + deals.length * staggerMs });
    }
  }
  const last = deals[deals.length - 1];
  return { riffleMs, staggerMs, flightMs, deals, totalMs: (last?.atMs ?? riffleMs) + flightMs };
}

const SIDES: readonly Side[] = ["you", "opponent"];
const DEAL_PROPERTIES = ["--deal-dx", "--deal-dy", "--deal-at"] as const;

function centerOf(element: Element): { x: number; y: number } {
  const rect = element.getBoundingClientRect();
  return { x: rect.left + rect.width / 2, y: rect.top + rect.height / 2 };
}

/**
 * Gives each `.hand-slot` of both hands the three properties board.css's `hand-deal` reads: how far the
 * slot's deck is from it (`--deal-dx`, `--deal-dy`, in pixels) and when its card leaves the deck
 * (`--deal-at`). The slot's place in its hand is the order it is dealt in.
 */
export function aimDeal(root: HTMLElement, plan: OpeningPlan): void {
  for (const side of SIDES) {
    const deck = root.querySelector(`[data-testid="library-${side}"]`);
    const from = deck === null ? null : centerOf(deck);
    const slots = root.querySelectorAll<HTMLElement>(`[data-testid="hand-${side}"] .hand-slot`);
    const times = plan.deals.filter((deal) => deal.side === side).map((deal) => deal.atMs);
    slots.forEach((slot, i) => {
      const at = times[i];
      if (at === undefined) return;
      const to = centerOf(slot);
      slot.style.setProperty("--deal-dx", `${String(Math.round(from === null ? 0 : from.x - to.x))}px`);
      slot.style.setProperty("--deal-dy", `${String(Math.round(from === null ? 0 : from.y - to.y))}px`);
      slot.style.setProperty("--deal-at", `${String(at)}ms`);
    });
  }
}

/** Takes the properties `aimDeal` wrote off every slot, so a hand dealt once is an ordinary hand. */
export function clearDeal(root: HTMLElement): void {
  for (const slot of root.querySelectorAll<HTMLElement>(".hand-slot")) {
    for (const property of DEAL_PROPERTIES) slot.style.removeProperty(property);
  }
}
