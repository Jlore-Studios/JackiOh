// #1443: where a non-combat damage's source card is, read off the viewer's `PlayerView` only
// (CLAUDE.md rule 7). A hit counts only when the view shows the source face-up with a `defId` the
// bundled catalog or the match's own definitions know; a hidden card, an unknown id, a hero power
// or no source fires nothing, and the damage animates as it does today.

import { hasKeyword, type Flavor, type PlayerId, type PlayerView } from "@jackioh/shared";

import { flavorOfId } from "../cards/flavor.ts";
import { sideOf, testid, type Side } from "../game/contract.ts";
import { CATALOG } from "../wire/cards.ts";
import { flavorLook, type FlavorLook } from "./flavors.ts";
import type { FxAnchor } from "./types.ts";

const HIDDEN_ID = "hidden";

/** Where the source card is and what it throws, as the view shows it. */
export type FlavorSource = {
  /** Where its projectile flies from: its zone on the field, or its controller's hero. */
  anchor: FxAnchor;
  /** The flavor whose look its projectile draws. */
  flavor: Flavor;
  /** The seat whose hero stands in when the card itself is off the field. */
  controller: PlayerId;
};

function heroAnchor(view: PlayerView, side: Side): FxAnchor {
  return { kind: "testid", testid: testid.hero(side) };
}

function readable(defId: string, view: PlayerView): boolean {
  if (defId === HIDDEN_ID) return false;
  return (CATALOG[defId] ?? view.defs?.[defId]) !== undefined;
}

function sourceOf(defId: string, controller: PlayerId, view: PlayerView, anchor: FxAnchor): FlavorSource | null {
  if (!readable(defId, view)) return null;
  return { anchor, flavor: flavorOfId(defId, view.defs), controller };
}

/**
 * #1444: the look `defId` draws for the card `instanceId`, or `null` when the view hides it.
 * A hidden id or one neither the bundled catalog nor the match's own definitions know is never
 * looked up (CLAUDE.md rule 7); Chaos picks per instance id.
 */
export function flavorLookOf(defId: string, instanceId: string, view: PlayerView): FlavorLook | null {
  if (!readable(defId, view)) return null;
  return flavorLook(flavorOfId(defId, view.defs), instanceId);
}

/** #1444: where the attacker `instanceId` stands and as what, or `null` when the view hides it. */
export function attackerOf(instanceId: string, view: PlayerView): { defId: string; ranged: boolean } | null {
  if (instanceId === HIDDEN_ID) return null;
  for (const side of [view.you, view.opponent]) {
    for (const unit of side.units) {
      if (unit !== null && unit.instanceId === instanceId) {
        return { defId: unit.defId, ranged: hasKeyword(unit.keywords, "Ranged") };
      }
    }
    for (const slot of side.backrow) {
      if (slot === null || slot.faceDown !== false || slot.unit === undefined) continue;
      if (slot.unit.instanceId === instanceId) {
        return { defId: slot.unit.defId, ranged: hasKeyword(slot.unit.keywords, "Ranged") };
      }
    }
  }
  return null;
}

/**
 * The card `sourceId` names, as the view shows it, or `null` when no projectile flies. Field hits
 * (a Unit, or a face-up backrow card — R1081000: a Unit standing in a backrow zone wins over the
 * slot's own `defId`) anchor on the source's zone with the object's own `defId` and controller;
 * anywhere else (resolving, graveyard, exile, the viewer's hand) anchors on the seat's hero with
 * that seat as controller. `null` and R97's sentinel never fly.
 */
export function flavorSourceOf(sourceId: string | null, view: PlayerView): FlavorSource | null {
  if (sourceId === null || sourceId === HIDDEN_ID) return null;
  const seats: readonly Side[] = ["you", "opponent"];
  for (const side of seats) {
    const sv = side === "you" ? view.you : view.opponent;
    for (const unit of sv.units) {
      if (unit !== null && unit.instanceId === sourceId) {
        return sourceOf(unit.defId, unit.controller, view, { kind: "testid", testid: testid.card(sourceId) });
      }
    }
    for (const carried of sv.carried ?? []) {
      if (carried !== null && carried.instanceId === sourceId) {
        return sourceOf(carried.defId, carried.controller, view, { kind: "testid", testid: testid.card(sourceId) });
      }
    }
    for (const slot of sv.backrow) {
      if (slot === null || slot.faceDown !== false) continue;
      if (slot.unit !== undefined && slot.unit.instanceId === sourceId) {
        return sourceOf(slot.unit.defId, slot.unit.controller, view, { kind: "testid", testid: testid.card(sourceId) });
      }
      if (slot.instanceId === sourceId) {
        return sourceOf(slot.defId, slot.controller, view, { kind: "testid", testid: testid.card(sourceId) });
      }
    }
  }
  for (const side of seats) {
    const sv = side === "you" ? view.you : view.opponent;
    for (const card of sv.resolving) {
      if (card.instanceId === sourceId) {
        return sourceOf(card.defId, sv.player, view, heroAnchor(view, sideOf(view, sv.player)));
      }
    }
    for (const card of sv.graveyard) {
      if (card.instanceId === sourceId) {
        return sourceOf(card.defId, sv.player, view, heroAnchor(view, sideOf(view, sv.player)));
      }
    }
    for (const card of sv.exile) {
      if (card.instanceId === sourceId) {
        return sourceOf(card.defId, sv.player, view, heroAnchor(view, sideOf(view, sv.player)));
      }
    }
  }
  const hand = view.you.hand;
  if (Array.isArray(hand)) {
    for (const card of hand) {
      if (card.instanceId === sourceId) {
        return sourceOf(card.defId, view.you.player, view, heroAnchor(view, "you"));
      }
    }
  }
  return null;
}
