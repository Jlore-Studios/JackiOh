// R195, R658: reading the yellow glow (`conditionActive`) off a viewer's own `viewFor`, for the card
// tests that prove it in their own file (README §5). The key is present and `true`, or absent: a key
// present with any other value fails here.

import type { PlayerId } from "@jackioh/shared";
import { expect } from "vitest";
import type { Scenario } from "./_harness";

/** `true` when the view carries the key (which must then be exactly `true`), `false` when absent. */
export function glows(card: object | null | undefined): boolean {
  if (card === null || card === undefined) throw new Error("no card at that place in the view");
  if (!("conditionActive" in card)) return false;
  expect((card as { conditionActive?: unknown }).conditionActive).toBe(true);
  return true;
}

/** Does this hand card glow in its holder's own view right now? */
export function handGlows(s: Scenario, instanceId: string, viewer: PlayerId = "p1"): boolean {
  const hand = s.view(viewer).you.hand;
  if (!Array.isArray(hand)) throw new Error("the viewer's own hand must travel in full (§10.8)");
  return glows(hand.find((card) => card.instanceId === instanceId));
}

/** Does the card in this backrow lane (1-based) glow in its controller's own view right now? */
export function backrowGlows(s: Scenario, lane: number, viewer: PlayerId = "p1"): boolean {
  return glows(s.view(viewer).you.backrow[lane - 1]);
}

/** The other seat's view of that backrow lane: a face-down trap is a bare back, with no glow on it. */
export function opponentSeesGlow(s: Scenario, lane: number, owner: PlayerId = "p1"): boolean {
  const other: PlayerId = owner === "p1" ? "p2" : "p1";
  const zone = s.view(other).opponent.backrow[lane - 1];
  if (zone === null || zone === undefined) throw new Error("no card at that place in the view");
  return "conditionActive" in zone;
}
