// DOM effects for S10/B39: data-only, aria-hidden elements painted by fx.css; no text nodes become Cypress or screen-reader content.
// Missing boxes mount nothing. No timers: the director removes effects (S8 step 5); idempotent removal makes clear/expiry safe.
// Ghosts are card BACK only (R202); supports R502 fracture, R437 brand, R436 chaos, R2652001 rolls, and R1363 shields.

import { FX_FOG_ICONS, FX_FOG_PAD, FX_FOG_PUFFS } from "./constants.ts";
import type { FxBox, FxChaosCue, FxDomCue, FxHoldCue, FxIcon, FxRollCue, FxTint } from "./types.ts";
import { t } from "../i18n/i18n.ts";

export type DomEffectBoxes = { at?: FxBox | null; from?: FxBox | null; to?: FxBox | null };
export type DomEffect = { readonly el: HTMLElement; remove(): void };

function px(value: number): string {
  return `${value}px`;
}

function centerOf(box: FxBox): { x: number; y: number } {
  return { x: box.x + box.width / 2, y: box.y + box.height / 2 };
}

function placeAtCenter(el: HTMLElement, box: FxBox): void {
  const center = centerOf(box);
  el.style.setProperty("--fx-x", px(center.x));
  el.style.setProperty("--fx-y", px(center.y));
  el.style.setProperty("--fx-w", px(box.width));
  el.style.setProperty("--fx-h", px(box.height));
}

function cover(el: HTMLElement, box: FxBox): void {
  el.style.setProperty("--fx-x", px(box.x));
  el.style.setProperty("--fx-y", px(box.y));
  el.style.setProperty("--fx-w", px(box.width));
  el.style.setProperty("--fx-h", px(box.height));
}

function tint(el: HTMLElement, colors: FxTint): void {
  el.style.setProperty("--fx-mark-rim", colors.rim);
  el.style.setProperty("--fx-mark-core", colors.core);
  el.style.setProperty("--fx-mark-glow", colors.glow);
}

function tintVars(el: HTMLElement, colors: FxTint): void {
  el.style.setProperty("--fx-tint-rim", colors.rim);
  el.style.setProperty("--fx-tint-core", colors.core);
  el.style.setProperty("--fx-tint-glow", colors.glow);
}

function spanOf(a: FxBox, b: FxBox, pad: number): FxBox {
  const x = Math.min(a.x, b.x);
  const y = Math.min(a.y, b.y);
  const right = Math.max(a.x + a.width, b.x + b.width);
  const bottom = Math.max(a.y + a.height, b.y + b.height);
  const grow = (bottom - y) * pad;
  return { x: x - grow, y: y - grow, width: right - x + 2 * grow, height: bottom - y + 2 * grow };
}

const SVG_NS = "http://www.w3.org/2000/svg";

function fogParts(doc: Document, el: HTMLElement, icon: FxIcon | null): void {
  el.style.setProperty("--fx-puffs", String(FX_FOG_PUFFS));
  for (let i = 0; i < FX_FOG_PUFFS; i += 1) {
    const puff = doc.createElement("div");
    puff.className = "fx-fog-puff";
    puff.style.setProperty("--fx-puff", String(i));
    el.appendChild(puff);
  }
  if (icon === null) return;
  el.style.setProperty("--fx-icons", String(FX_FOG_ICONS));
  for (let i = 0; i < FX_FOG_ICONS; i += 1) {
    const svg = doc.createElementNS(SVG_NS, "svg");
    svg.setAttribute("class", "fx-fog-icon");
    svg.setAttribute("viewBox", "0 0 24 24");
    svg.setAttribute("aria-hidden", "true");
    svg.setAttribute("style", `--fx-icon: ${String(i)}`);
    const path = doc.createElementNS(SVG_NS, "path");
    path.setAttribute("d", icon.d);
    path.setAttribute("fill-rule", icon.rule);
    svg.appendChild(path);
    el.appendChild(svg);
  }
}

function chaosLines(doc: Document, el: HTMLElement, lines: FxChaosCue["lines"]): void {
  el.style.setProperty("--fx-lines", String(lines.length));
  lines.forEach((line, index) => {
    const row = doc.createElement("div");
    row.className = "fx-chaos-line";
    row.setAttribute("data-landed-text", line.text);
    row.style.setProperty("--fx-line", String(index));
    row.style.setProperty("--fx-land-ms", `${Math.max(0, line.landMs)}ms`);
    row.style.setProperty("--fx-steps", String(Math.max(0, line.reel.length - 1)));
    const reel = doc.createElement("div");
    reel.className = "fx-chaos-reel";
    line.reel.forEach((name, at) => {
      const cell = doc.createElement("span");
      cell.className = "fx-chaos-cell";
      cell.setAttribute("data-text", t(name));
      if (at === line.reel.length - 1) cell.setAttribute("data-landed", "true");
      reel.appendChild(cell);
    });
    row.appendChild(reel);
    el.appendChild(row);
  });
}

/**
 * R2652001: one item per coin or die, all at once. Each is a window one face tall over a reel of its
 * tumble faces, which spins (a coin) or tumbles (a die) and lands on the kept face at `--fx-land-ms`;
 * under it, when Lucky drew more than one face, every face it drew, the kept one marked. The words
 * are already the reader's (fx/rolls.ts).
 */
function rollItems(doc: Document, el: HTMLElement, cue: FxRollCue): void {
  el.setAttribute("data-kind", cue.die);
  el.style.setProperty("--fx-items", String(cue.items.length));
  const tray = doc.createElement("div");
  tray.className = "fx-roll-tray";
  cue.items.forEach((item, index) => {
    const one = doc.createElement("div");
    one.className = "fx-roll-item";
    one.setAttribute("data-kind", cue.die);
    one.setAttribute("data-landed-text", item.tumble[item.tumble.length - 1] ?? "");
    one.style.setProperty("--fx-item", String(index));
    one.style.setProperty("--fx-land-ms", `${Math.max(0, item.landMs)}ms`);
    one.style.setProperty("--fx-steps", String(Math.max(0, item.tumble.length - 1)));
    const face = doc.createElement("div");
    face.className = "fx-roll-face";
    const reel = doc.createElement("div");
    reel.className = "fx-roll-reel";
    item.tumble.forEach((text, at) => {
      const cell = doc.createElement("span");
      cell.className = "fx-roll-cell";
      cell.setAttribute("data-text", text);
      if (at === item.tumble.length - 1) cell.setAttribute("data-landed", "true");
      reel.appendChild(cell);
    });
    face.appendChild(reel);
    one.appendChild(face);
    if (item.faces.length > 1) {
      const drawn = doc.createElement("div");
      drawn.className = "fx-roll-draws";
      item.faces.forEach((text, at) => {
        const chip = doc.createElement("span");
        chip.className = at === item.kept ? "fx-roll-kept" : "fx-roll-extra";
        chip.setAttribute("data-text", text);
        drawn.appendChild(chip);
      });
      one.appendChild(drawn);
    }
    tray.appendChild(one);
  });
  el.appendChild(tray);
}

function signedAmount(tone: "damage" | "heal" | "loss", amount: number): string {
  return tone === "heal" ? `+${amount}` : `-${amount}`;
}

export function mountDomEffect(root: HTMLElement, cue: FxDomCue, boxes: DomEffectBoxes): DomEffect | null {
  const doc = root.ownerDocument;
  const el = doc.createElement("div");

  switch (cue.kind) {
    case "splat": {
      const at = boxes.at ?? null;
      if (at === null) return null;
      el.setAttribute("data-tone", cue.tone);
      el.setAttribute("data-amount", signedAmount(cue.tone, cue.amount));
      placeAtCenter(el, at);
      break;
    }
    case "rays": {
      const at = boxes.at ?? null;
      if (at === null) return null;
      el.setAttribute("data-tone", cue.tone);
      placeAtCenter(el, at);
      break;
    }
    case "sheen": {
      const at = boxes.at ?? null;
      if (at === null) return null;
      cover(el, at);
      break;
    }
    case "ghost": {
      const from = boxes.from ?? null;
      const to = boxes.to ?? null;
      if (from === null || to === null) return null;
      const start = centerOf(from);
      const end = centerOf(to);
      el.style.setProperty("--fx-x", px(start.x));
      el.style.setProperty("--fx-y", px(start.y));
      el.style.setProperty("--fx-dx", px(end.x - start.x));
      el.style.setProperty("--fx-dy", px(end.y - start.y));
      break;
    }
    case "arrows": {
      const at = boxes.at ?? null;
      if (at === null) return null;
      el.setAttribute("data-direction", cue.direction);
      cover(el, at);
      break;
    }
    case "banner": {
      el.setAttribute("data-tone", cue.tone);
      el.setAttribute("data-text", t(cue.text));
      break;
    }
    case "result": {
      el.setAttribute("data-outcome", cue.outcome);
      el.setAttribute("data-text", t(cue.text));
      break;
    }
    case "fracture": {
      const at = boxes.at ?? null;
      if (at === null) return null;
      cover(el, at);
      break;
    }
    case "walls": {
      const at = boxes.at ?? null;
      if (at === null) return null;
      cover(el, at);
      el.style.setProperty("--fx-reach", px(at.width * cue.reach));
      break;
    }
    case "brand": {
      const at = boxes.at ?? null;
      if (at === null) return null;
      placeAtCenter(el, at);
      tint(el, cue.tint);
      break;
    }
    case "shield": {
      const at = boxes.at ?? null;
      if (at === null) return null;
      placeAtCenter(el, at);
      el.setAttribute("data-size", cue.size);
      break;
    }
    case "chaos": {
      el.setAttribute("data-text", t(cue.title));
      chaosLines(doc, el, cue.lines);
      break;
    }
    case "roll": {
      el.setAttribute("data-text", t(cue.title));
      rollItems(doc, el, cue);
      break;
    }
    case "fog": {
      const from = boxes.from ?? null;
      const to = boxes.to ?? null;
      if (from === null || to === null) return null;
      cover(el, spanOf(from, to, FX_FOG_PAD));
      el.setAttribute("data-tone", cue.tone);
      tintVars(el, cue.tint);
      fogParts(doc, el, cue.icon);
      break;
    }
    case "zone": {
      const at = boxes.at ?? null;
      if (at === null) return null;
      cover(el, at);
      el.setAttribute("data-direction", cue.direction);
      el.setAttribute("data-text", cue.text);
      tintVars(el, cue.tint);
      break;
    }
  }

  el.className = `fx-${cue.kind}`;
  el.setAttribute("data-fx", cue.kind);
  el.setAttribute("aria-hidden", "true");
  el.style.setProperty("--fx-ms", `${Math.max(0, cue.durationMs)}ms`);

  root.appendChild(el);

  return {
    el,
    remove(): void {
      if (el.parentNode !== null) el.parentNode.removeChild(el);
    },
  };
}

/* Stand-ins (B46) carry rendered cards to the next view while stripping identity, interaction, and text. */

function strippable(name: string): boolean {
  return (
    name === "id" ||
    name === "tabindex" ||
    name === "role" ||
    name === "title" ||
    name === "draggable" ||
    name === "disabled" ||
    name === "data-testid" ||
    name === "data-legal" ||
    name === "data-selected" ||
    name === "data-animating" ||
    name === "data-fx-concealed" ||
    name.startsWith("aria-") ||
    name.startsWith("on")
  );
}

export function standInCopy(source: Element): HTMLElement {
  const copy = source.cloneNode(true) as HTMLElement;
  const doc = copy.ownerDocument;
  for (const node of [copy, ...Array.from(copy.querySelectorAll("*"))]) {
    for (const name of node.getAttributeNames()) {
      if (strippable(name)) node.removeAttribute(name);
    }
  }
  const texts: Node[] = [];
  const walker = doc.createTreeWalker(copy, 4 /* NodeFilter.SHOW_TEXT */);
  while (walker.nextNode() !== null) texts.push(walker.currentNode);
  for (const text of texts) {
    const span = doc.createElement("span");
    span.className = "fx-hold-text";
    span.setAttribute("data-text", text.nodeValue ?? "");
    text.parentNode?.replaceChild(span, text);
  }
  copy.style.removeProperty("--fx-lunge-x");
  copy.style.removeProperty("--fx-lunge-y");
  copy.classList.add("fx-hold-card");
  return copy;
}

export function landingBox(zone: FxBox, source: FxBox | null): FxBox {
  const height = Math.max(1, zone.height - 2);
  const aspect = source !== null && source.height > 0 ? source.width / source.height : CARD_ASPECT;
  const width = Math.max(1, Math.min(zone.width - 2, height * aspect));
  return { x: zone.x + (zone.width - width) / 2, y: zone.y + (zone.height - height) / 2, width, height };
}

const CARD_ASPECT = 0.74;
const DROP_PX = -14;
const DROP_SCALE = 1.3;
const FLIGHT_PEAK_SCALE = 1.3;
const HOLD_ORIGIN = { x: 0.5, y: 0.6 } as const;

/** Keeps the scaled stand-in inside the viewport. */
export function holdOrigin(land: FxBox, scale: number, view: { width: number; height: number }): { x: number; y: number } {
  const axis = (start: number, size: number, room: number, rest: number): number => {
    const grow = size * (scale - 1);
    if (!(grow > 0)) return rest;
    const most = start / grow;
    const least = 1 - (room - start - size) / grow;
    return Math.min(1, Math.max(0, Math.min(Math.max(rest, least), most)));
  };
  return {
    x: axis(land.x, land.width, view.width, HOLD_ORIGIN.x),
    y: axis(land.y, land.height, view.height, HOLD_ORIGIN.y),
  };
}

export type HoldEffect = DomEffect & {
  place(land: FxBox): void;
  shift(dx: number, dy: number): void;
};

export function mountHold(
  root: HTMLElement,
  cue: FxHoldCue,
  parts: { source: Element | null; from: FxBox | null; land: FxBox; font: string | null },
): HoldEffect {
  const doc = root.ownerDocument;
  const el = doc.createElement("div");
  el.className = "fx-hold";
  el.setAttribute("data-fx", "hold");
  el.setAttribute("data-look", parts.source !== null ? "card" : "glow");
  // #2111, R2111000: a set lands flat and heavy (fx.css's fx-hold-set); `data-landing` is the drag layer's.
  if (cue.set === true) el.setAttribute("data-land", "set");
  el.setAttribute("aria-hidden", "true");
  el.setAttribute("inert", "");
  el.style.setProperty("--fx-ms", `${Math.max(0, cue.durationMs)}ms`);
  el.style.setProperty("--fx-land-ms", `${Math.max(0, cue.landMs)}ms`);
  if (parts.font !== null && parts.font !== "") el.style.setProperty("font-size", parts.font);

  const place = (land: FxBox): void => {
    el.style.setProperty("--fx-x", px(land.x));
    el.style.setProperty("--fx-y", px(land.y));
    el.style.setProperty("--fx-w", px(land.width));
    el.style.setProperty("--fx-h", px(land.height));
  };
  place(parts.land);

  const landCenter = centerOf(parts.land);
  const startScale = parts.from !== null ? parts.from.height / Math.max(1, parts.land.height) : DROP_SCALE;
  const view = doc.defaultView;
  if (view !== null && view.innerWidth > 0) {
    const origin = holdOrigin(parts.land, Math.max(FLIGHT_PEAK_SCALE, startScale), {
      width: view.innerWidth,
      height: view.innerHeight,
    });
    if (origin.x !== HOLD_ORIGIN.x || origin.y !== HOLD_ORIGIN.y) {
      el.style.setProperty("transform-origin", `${(origin.x * 100).toFixed(1)}% ${(origin.y * 100).toFixed(1)}%`);
    }
  }
  if (parts.from !== null) {
    const start = centerOf(parts.from);
    el.style.setProperty("--fx-dx", px(start.x - landCenter.x));
    el.style.setProperty("--fx-dy", px(start.y - landCenter.y));
    el.style.setProperty("--fx-s", (parts.from.height / Math.max(1, parts.land.height)).toFixed(3));
    el.style.setProperty("--fx-o0", "1");
  } else {
    el.style.setProperty("--fx-dx", px(0));
    el.style.setProperty("--fx-dy", px(DROP_PX));
    el.style.setProperty("--fx-s", String(DROP_SCALE));
    el.style.setProperty("--fx-o0", "0");
  }

  if (parts.source !== null) {
    el.appendChild(standInCopy(parts.source));
  } else {
    const glow = doc.createElement("div");
    glow.className = "fx-hold-glow";
    el.appendChild(glow);
  }

  root.appendChild(el);
  return {
    el,
    place,
    shift(dx: number, dy: number): void {
      if (dx === 0 && dy === 0) el.style.removeProperty("translate");
      else el.style.setProperty("translate", `${px(dx)} ${px(dy)}`);
    },
    remove(): void {
      if (el.parentNode !== null) el.parentNode.removeChild(el);
    },
  };
}
