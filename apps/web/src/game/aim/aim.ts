// The aim this seat shows the opponent (SPEC §9.5, R660), as data.
//
// While a play, an Activate or an attack is being aimed — by drag or by click-select, both of which
// are the board's `interaction` — the opponent's board draws an arrow from its source to the
// target under the pointer. This module turns that interaction and the hovered spot into the wire's
// `Aim`, whose ends are public handles only (`@jackioh/shared` `aim.ts`): a hero by its seat, a
// field card by its zone (never its instance id, so a face-down card is only ever a zone), and a
// hand card by its position, which the opponent sees as the card back there.
//
// Only an aim at a declared target is shown — never a zone a card is being placed in, a Tribute or
// a discard — so the arrow says no more about a hand card than Hearthstone's does: that it is being
// aimed, and at what. Nothing here is a rule (CLAUDE.md rule 7): the targets are the glow the board
// already lights from `legal`.
//
// No React; only `aimEndElement` reads the DOM.

import type { ActionBody, Aim, AimEnd, PlayerView, Row } from "@jackioh/shared";

import { highlightFor, selectionTestid, type Interaction } from "../actions.ts";
import { laneIndex, LANES, playerOf, testid, type ClickTarget, type Side } from "../contract.ts";

/**
 * The testids an aim may land on now: the board's glow, narrowed to what the interaction aims at —
 * an attack's targets, or the declared targets (`targets`) of a play or an activation still in the
 * running. Empty when nothing is being aimed.
 */
export function aimTargets(view: PlayerView, legal: readonly ActionBody[], interaction: Interaction): ReadonlySet<string> {
  if (interaction.stage === "idle") return new Set();
  const glow = highlightFor(view, legal, interaction).glow ?? new Set<string>();
  if (interaction.stage === "attacking") return glow;
  const declared = new Set<string>();
  for (const candidate of interaction.candidates) {
    if (!("targets" in candidate) || candidate.targets === undefined) continue;
    for (const selection of candidate.targets) {
      const where = selectionTestid(view, selection);
      if (where !== null && glow.has(where)) declared.add(where);
    }
  }
  return declared;
}

function zoneOf(view: PlayerView, side: Side, instanceId: string): AimEnd | null {
  const seat = view[side];
  for (const lane of LANES) {
    const unit = seat.units[laneIndex(lane)];
    if (unit != null && unit.instanceId === instanceId) return { at: "zone", player: seat.player, row: "units", lane };
    const card = seat.backrow[laneIndex(lane)];
    if (card != null && "instanceId" in card && card.instanceId === instanceId) {
      return { at: "zone", player: seat.player, row: "backrow", lane };
    }
  }
  return null;
}

/** Where a card the viewer controls stands, by zone, on either side (a stolen card may, R33). */
function fieldEnd(view: PlayerView, instanceId: string): AimEnd | null {
  return zoneOf(view, "you", instanceId) ?? zoneOf(view, "opponent", instanceId);
}

/**
 * Where the aim starts, as the opponent can see it: the hand card's position, the zone of the unit
 * attacking or the card activating, or the hero whose Heroic Power it is. Null when the source is
 * somewhere the arrow cannot start from (a play from the graveyard pile), and nothing is shown.
 */
export function aimSource(view: PlayerView, interaction: Interaction): AimEnd | null {
  switch (interaction.stage) {
    case "idle":
      return null;
    case "attacking":
      return fieldEnd(view, interaction.attackerId);
    case "playing": {
      const hand = view.you.hand;
      if (!Array.isArray(hand)) return null;
      const index = hand.findIndex((card) => card.instanceId === interaction.instanceId);
      return index < 0 ? null : { at: "hand", player: view.viewer, index };
    }
    case "activating": {
      const onField = fieldEnd(view, interaction.instanceId);
      if (onField !== null) return onField;
      for (const side of ["you", "opponent"] as const) {
        const seat = view[side];
        if (seat.hero.powers.some((power) => power.instanceId === interaction.instanceId)) {
          return { at: "hero", player: seat.player };
        }
      }
      return null;
    }
  }
}

/** The hovered spot as a public handle: a hero, or the zone a card or an empty zone stands for. */
export function aimEndOf(view: PlayerView, target: ClickTarget): AimEnd | null {
  switch (target.on) {
    case "hero":
      return { at: "hero", player: playerOf(view, target.side) };
    case "unit":
    case "backrow": {
      const row: Row = target.on === "unit" ? "units" : "backrow";
      return { at: "zone", player: playerOf(view, target.side), row, lane: target.lane };
    }
    case "zone":
      return { at: "zone", player: playerOf(view, target.side), row: target.row, lane: target.lane };
    default:
      return null;
  }
}

/**
 * The aim to show the opponent: null while nothing is aimed (idle, a placement, a source the arrow
 * cannot start from, or `targets` — `aimTargets` — empty), else the source and the hovered target,
 * which `hovered` gives only when it is one of `targets` (the caller picks it off the DOM with them).
 */
export function aimFor(
  view: PlayerView,
  interaction: Interaction,
  targets: ReadonlySet<string>,
  hovered: ClickTarget | null,
): Aim | null {
  if (targets.size === 0) return null;
  const source = aimSource(view, interaction);
  if (source === null) return null;
  return { source, target: hovered === null ? null : aimEndOf(view, hovered) };
}

/**
 * The element an end names on the RECEIVER's board (`view.viewer` is the receiver, so the sender's
 * seat is "opponent"): its hero, its zone, or the card back at that position in the hand.
 */
export function aimEndElement(root: ParentNode, view: PlayerView, end: AimEnd): Element | null {
  const side: Side = end.player === view.viewer ? "you" : "opponent";
  switch (end.at) {
    case "hero":
      return root.querySelector(`[data-testid="${testid.hero(side)}"]`);
    case "zone":
      return root.querySelector(`[data-testid="${testid.zone(side, end.row, end.lane)}"]`);
    case "hand":
      return root.querySelectorAll(`[data-testid="hand-${side}"] .hand-slot`)[end.index] ?? null;
  }
}
