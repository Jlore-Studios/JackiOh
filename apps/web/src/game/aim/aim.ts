// The aim this seat shows the opponent (SPEC §9.5, R738).
// Only declared targets use public handles, preserving hidden information (CLAUDE.md rule 7).

import type { ActionBody, Aim, AimEnd, PlayerView, Row } from "@jackioh/shared";

import { highlightFor, selectionTestid, type Interaction } from "../actions.ts";
import { laneIndex, LANES, playerOf, testid, type ClickTarget, type Side } from "../contract.ts";

/** Narrows board highlights to the interaction's declared targets. */
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

/** Gives the opponent a public source handle, or hides an unrepresentable source. */
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

/** An aim requires a declared target and a public source. */
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

/** Maps a public end to the receiver's board (`view.viewer`). */
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
