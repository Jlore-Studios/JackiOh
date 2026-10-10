// UX support contract: S6 attributes, S8 settings testids, and S9 drag overlay. Specs use these
// constants rather than raw selectors.
//
// Drag events use viewport coordinates; drops are hit-tested by `elementsFromPoint`, so Cypress
// scrolling stays disabled after a point is read.
// S9 events set `pointerId`, `clientX`, and `clientY`, leaving `isPrimary` and `pointerType` at defaults.

import { timeouts } from "./config.ts";

// S9: Drag overlay

/** Overlay root; `data-kind="play" | "attack"`. */
export const DRAG_LAYER = "drag-layer";
/** Placed card; `data-instance-id`. */
export const DRAG_GHOST = "drag-ghost";
/** Targeting arrow; `data-from` and `data-valid="true" | "false"`. */
export const DRAG_ARROW = "drag-arrow";
/** Valid-target reticle; `data-target`. */
export const DRAG_RETICLE = "drag-reticle";

export const DRAG_KIND_ATTR = "data-kind";
export const DRAG_INSTANCE_ATTR = "data-instance-id";
export const DRAG_FROM_ATTR = "data-from";
export const DRAG_VALID_ATTR = "data-valid";
export const DRAG_TARGET_ATTR = "data-target";

export const DRAG_THRESHOLD_PX = 8;

// S6: Attributes

/** `"ready"` marks a legal card, zone, hero, power, or end turn. */
export const GLOW_ATTR = "data-glow";
export const GLOW_READY = "ready";
/** Face-up cards set this to `"true"` for the yellow glow. */
export const CONDITION_ATTR = "data-condition-active";
/** Board value: `"on" | "off"` from the Drag to play setting. */
export const DRAG_ATTR = "data-drag";
/** `<html>` value: `"play" | "attack"` while dragging. */
export const DRAGGING_ATTR = "data-dragging";
export const ROOT = "html";
/** End-turn value: `"armed"` while confirmation is pending. */
export const CONFIRM_ATTR = "data-confirm";

// S8: Settings panel

export const SETTINGS_OPEN_GAME = "settings-open-game";
export const SETTINGS_OPEN_NAV = "settings-open-nav";
export const SETTINGS_PANEL = "settings-panel";
export const SETTINGS_SCRIM = "settings-scrim";
export const SETTINGS_CLOSE = "settings-close";
export const SETTINGS_RESET = "settings-reset";
/** Reset only the open tab. */
export const SETTINGS_RESET_TAB = "settings-reset-tab";
export const SETTINGS_TABLIST = "settings-tablist";

export type SettingKey = "dragToPlay" | "confirmEndTurn" | "hoverPreviews" | "reduceMotion";
export type SettingsSection = "gameplay" | "visuals" | "audio" | "account";

export function settingId(key: SettingKey): string {
  return `setting-${key}`;
}

/** All tab panels stay mounted; only the open panel is shown. */
export function settingsSectionId(section: SettingsSection): string {
  return `settings-section-${section}`;
}

export function settingsTabId(section: SettingsSection): string {
  return `settings-tab-${section}`;
}

// Gesture

export type Point = { x: number; y: number };

const POINTER_ID = 1;

/** Twice the threshold guarantees that the first move starts a drag. */
const LIFT_PX = DRAG_THRESHOLD_PX * 2;

function centreOf(element: HTMLElement): Point {
  const win = element.ownerDocument.defaultView;
  let box = element.getBoundingClientRect();
  if (win !== null && (box.top < 0 || box.left < 0 || box.bottom > win.innerHeight || box.right > win.innerWidth)) {
    element.scrollIntoView({ block: "center", inline: "center" });
    box = element.getBoundingClientRect();
  }
  return { x: Math.round(box.left + box.width / 2), y: Math.round(box.top + box.height / 2) };
}

/** A point in `.app-shell--wide`'s left padding (apps/web/src/index.css), never under the board. */
function outsidePoint(win: Window): Point {
  return { x: 2, y: Math.round(win.innerHeight / 2) };
}

function pointer(at: Point, phase: "down" | "move" | "up") {
  return {
    eventConstructor: "PointerEvent",
    pointerId: POINTER_ID,
    clientX: at.x,
    clientY: at.y,
    button: 0,
    buttons: phase === "up" ? 0 : 1,
    bubbles: true,
    cancelable: true,
    force: true,
    scrollBehavior: false as const,
  };
}

function moveTo(at: Point): void {
  cy.get("body", { log: false }).trigger("pointermove", pointer(at, "move"));
}

export function pressAndLift(source: string): Cypress.Chainable<Point> {
  return cy
    .get(source, { timeout: timeouts.view })
    .should("be.visible")
    .then(($source) => {
      const element = $source[0] as HTMLElement;
      const from = centreOf(element);
      cy.wrap($source, { log: false }).trigger("pointerdown", pointer(from, "down"));
      const lifted = { x: from.x, y: from.y - LIFT_PX };
      moveTo(lifted);
      return cy.wrap(lifted, { log: false });
    });
}

export function hoverOver(target: string | "outside"): Cypress.Chainable<Point> {
  if (target === "outside") {
    return cy.window({ log: false }).then((win) => {
      const at = outsidePoint(win);
      moveTo(at);
      return cy.wrap(at, { log: false });
    });
  }
  return cy.get(target, { timeout: timeouts.view }).then(($target) => {
    const at = centreOf($target[0] as HTMLElement);
    // Avoid teleporting directly onto the target.
    moveTo({ x: at.x - 24, y: at.y + 24 });
    moveTo(at);
    return cy.wrap(at, { log: false });
  });
}

export function releaseOver(target: string | "outside"): void {
  if (target === "outside") {
    cy.window({ log: false }).then((win) => {
      cy.get("body", { log: false }).trigger("pointerup", pointer(outsidePoint(win), "up"));
    });
    return;
  }
  cy.get(target, { timeout: timeouts.view }).then(($target) => {
    const at = centreOf($target[0] as HTMLElement);
    cy.wrap($target, { log: false }).trigger("pointerup", pointer(at, "up"));
  });
}

export function dragTo(source: string, target: string | "outside"): void {
  pressAndLift(source);
  hoverOver(target);
  releaseOver(target);
  cy.settled();
}
