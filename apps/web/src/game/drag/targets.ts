// DOM target reader (S9): board identity comes from data-testid and zone data attributes, not React or rules.

import type { Row } from "@jackioh/shared";

import { ACTIVATE_ABILITY_ATTRIBUTE, ACTIVATE_FOR_ATTRIBUTE } from "../ActivateControl.tsx";
import type { ClickTarget, Side } from "../contract.ts";
import type { DragPlan, DropSpot } from "./model.ts";

const HAND_CARD = /^hand-card-(.+)$/;
const CARD = /^card-(.+)$/;
const ZONE = /^zone-(you|opponent)-(units|backrow)-(\d+)$/;
const POWER = /^power(?:-.+)?$/;

/** R384, R510: activation controls and Heroic Powers read their attributes, never their testids. */
function activationOf(element: Element, id: string): ClickTarget | null {
  const card = element.getAttribute(ACTIVATE_FOR_ATTRIBUTE);
  if (card !== null && card !== "") {
    const ability = element.getAttribute(ACTIVATE_ABILITY_ATTRIBUTE);
    return ability === null || ability === ""
      ? { on: "activate", instanceId: card }
      : { on: "activate", instanceId: card, ability };
  }
  const power = POWER.test(id) ? element.getAttribute("data-instance-id") : null;
  return power === null || power === "" ? null : { on: "activate", instanceId: power };
}

type Place = { side: Side; row: Row; lane: number };

function asSide(value: string | null): Side | null {
  return value === "you" || value === "opponent" ? value : null;
}

function asRow(value: string | null): Row | null {
  return value === "units" || value === "backrow" ? value : null;
}

/** Zone.tsx writes a zone's side, row and lane as data attributes. */
function placeOf(zone: Element): Place | null {
  const side = asSide(zone.getAttribute("data-side"));
  const row = asRow(zone.getAttribute("data-row"));
  const lane = Number(zone.getAttribute("data-lane"));
  if (side !== null && row !== null && Number.isInteger(lane) && lane > 0) return { side, row, lane };
  return null;
}

function enclosingZone(element: Element): Element | null {
  for (let at = element.parentElement; at !== null; at = at.parentElement) {
    if (ZONE.test(at.getAttribute("data-testid") ?? "")) return at;
  }
  return null;
}

/** Controls are pressed, never dragged. */
function isControl(element: Element): boolean {
  const tag = element.tagName.toLowerCase();
  return tag === "button" || tag === "input" || element.getAttribute("role") === "button";
}

/** Maps DOM testids to ClickTargets. Nested controls are presses, not drag sources; activation wins first (R384). */
export function targetFromElement(element: Element): { target: ClickTarget; testid: string } | null {
  let throughControl = false;
  for (let at: Element | null = element; at !== null; at = at.parentElement) {
    const id = at.getAttribute("data-testid");
    if (id !== null) {
      const activation = activationOf(at, id);
      if (activation !== null) return { target: activation, testid: id };

      const hand = HAND_CARD.exec(id);
      if (hand !== null) {
        if (throughControl) return null;
        return { target: { on: "hand", instanceId: hand[1] as string }, testid: id };
      }

      const card = CARD.exec(id);
      if (card !== null) {
        if (throughControl) return null;
        const zone = enclosingZone(at);
        const place = zone === null ? null : placeOf(zone);
        if (place === null) return null;
        const instanceId = card[1] as string;
        return {
          target:
            place.row === "units"
              ? { on: "unit", instanceId, side: place.side, lane: place.lane }
              : { on: "backrow", instanceId, side: place.side, lane: place.lane },
          testid: id,
        };
      }

      if (id === "hero-you" || id === "hero-opponent") {
        return { target: { on: "hero", side: id === "hero-you" ? "you" : "opponent" }, testid: id };
      }

      if (ZONE.test(id)) {
        // Controls in a zone are presses, not drag sources.
        if (throughControl) return null;
        const place = placeOf(at);
        if (place === null) return null;
        return { target: { on: "zone", side: place.side, row: place.row, lane: place.lane }, testid: id };
      }
    }
    if (isControl(at)) throughControl = true;
  }
  return null;
}

export function pickDropSpot(stack: readonly Element[], allowed: ReadonlySet<string>): DropSpot {
  for (const element of stack) {
    const hit = targetFromElement(element);
    if (hit !== null && allowed.has(hit.testid)) return { at: "target", target: hit.target, testid: hit.testid };
  }
  const top = stack[0];
  if (top === undefined) return { at: "outside" };
  if (top.closest('[data-testid="board"]') === null) return { at: "outside" };
  if (top.closest('[data-testid="hand-you"]') !== null) return { at: "outside" };
  return { at: "board" };
}

/** Zone.tsx exposes view locks as data-locked; report a locked play zone only if another zone in its row is offered. */
export function lockedZoneAt(
  stack: readonly Element[],
  plan: Pick<DragPlan, "kind" | "dropTestids">,
): { testid: string; zone: Element } | null {
  if (plan.kind !== "play") return null;
  for (const element of stack) {
    const hit = targetFromElement(element);
    if (hit === null || hit.target.on !== "zone") continue;
    const { side, row } = hit.target;
    if (side !== "you" || plan.dropTestids.has(hit.testid)) return null;
    const zone = element.closest(`[data-testid="${hit.testid}"]`);
    if (zone === null || zone.getAttribute("data-locked") !== "true") return null;
    const offered = [...plan.dropTestids].some((id) => id.startsWith(`zone-you-${row}-`));
    return offered ? { testid: hit.testid, zone } : null;
  }
  return null;
}
