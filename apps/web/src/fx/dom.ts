// DOM effects for S10/B39: data-only, aria-hidden elements painted by fx.css; no text nodes become Cypress or screen-reader content.
// No timers: the director removes effects (S8 step 5); idempotent removal makes clear/expiry safe.
// Ghosts are card BACK only (R202); supports R502 fracture, R437 brand, R436 chaos, R2652001 rolls, R1363 shields, R2722001 takeovers, R2801000 launches,
// R3336000 ships, R2800000 set pieces, and R3337002's radar and R3337003's rift and title card. R3400000: a
// takeover's look may draw its art (takeoverArt.ts; R3399002: its parts or a drawing of its own); R3400001: a
// signature's figures (figures.ts), each a shape over its box, never a card.

import { KEYWORD_CALLOUT_ICONS } from "../cards/keywordVisuals.ts";
import { ART_PLANES } from "../cards/art/pictures.ts";
import { fragmentRef, signatureLength, signatureSize } from "../cards/FullArtSignature.tsx";
import { SIGNATURE_FACES } from "../cards/signatureFaces.ts";
import { FX_FOG_ICONS, FX_FOG_PAD, FX_FOG_PUFFS, FX_HOLD_TURN_DEG, FX_HOLD_TURN_PX } from "./constants.ts";
import { atOf, pointIn } from "./anchors.ts";
import { TAKEOVER_ART_DRAWINGS, takeoverArtParts, type TakeoverDrawing } from "./takeoverArt.ts";
import type { FxBox, FxChaosCue, FxDomCue, FxHoldCue, FxIcon, FxRollCue, FxSplatTone, FxTakeoverStage, FxTimedPoint, FxTint, SetPieceShape } from "./types.ts";
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
/** The takeovers mounted so far: each signature clip path an id of its own. */
let takeoverSigns = 0;

/**
 * R3345000: the full-window stage of a takeover that holds a set piece: the card's art in its depth planes
 * and its name signing across the bottom.
 */
function takeoverStage(doc: Document, card: FxTakeoverStage): HTMLElement {
  const stage = doc.createElement("div");
  stage.className = "fx-takeover-stage";

  const planes = doc.createElement("div");
  planes.className = "fx-takeover-planes";
  ART_PLANES.forEach((plane, i) => {
    const p = doc.createElement("div");
    p.className = "fx-takeover-plane";
    p.setAttribute("data-plane", plane);
    p.style.backgroundImage = card.planes[i] ?? "";
    planes.appendChild(p);
  });
  stage.appendChild(planes);

  const signatureFace = card.face;
  // R2715002, R3345000: the name in its set's hand (`.cf-signature[data-signature-face]` sets the face),
  // uncovered left to right as a pen would write it: a clip path whose rect slides in (fx.css).
  const signature = doc.createElement("div");
  signature.className = "fx-takeover-signature";
  const hand = doc.createElement("span");
  hand.className = "cf-signature";
  hand.setAttribute("data-signature-face", signatureFace);
  hand.setAttribute("aria-hidden", "true");
  signature.appendChild(hand);

  const sigSvg = doc.createElementNS(SVG_NS, "svg");
  sigSvg.setAttribute("viewBox", "0 0 200 48");
  sigSvg.setAttribute("preserveAspectRatio", "xMidYMid meet");
  sigSvg.setAttribute("focusable", "false");

  const clipId = `fx-takeover-sign-${takeoverSigns++}`;
  const defs = doc.createElementNS(SVG_NS, "defs");
  const clip = doc.createElementNS(SVG_NS, "clipPath");
  clip.setAttribute("id", clipId);
  const reveal = doc.createElementNS(SVG_NS, "rect");
  reveal.setAttribute("class", "fx-takeover-sign-reveal");
  reveal.setAttribute("x", "-10");
  reveal.setAttribute("y", "-10");
  reveal.setAttribute("width", "220");
  reveal.setAttribute("height", "68");
  clip.appendChild(reveal);
  defs.appendChild(clip);
  sigSvg.appendChild(defs);

  const ink = doc.createElementNS(SVG_NS, "g");
  ink.setAttribute("clip-path", fragmentRef(clipId));
  const text = doc.createElementNS(SVG_NS, "text");
  text.setAttribute("class", "cf-signature-text");
  text.setAttribute("x", "100");
  text.setAttribute("y", "34");
  text.setAttribute("text-anchor", "middle");
  const { charEm } = SIGNATURE_FACES[signatureFace];
  const size = signatureSize(card.name, charEm);
  const length = signatureLength(card.name, size, charEm);
  text.setAttribute("font-size", String(size));
  text.setAttribute("textLength", String(length));
  text.setAttribute("lengthAdjust", "spacingAndGlyphs");
  text.textContent = card.name;
  ink.appendChild(text);
  sigSvg.appendChild(ink);

  hand.appendChild(sigSvg);
  stage.appendChild(signature);
  return stage;
}

/** R2722001: a takeover's layers, bottom to top (fx.css). */
const TAKEOVER_PARTS = ["dark", "sweep", "rays", "wave"] as const;

/** R2800000: each set piece shape's layers, bottom to top (fx.css). */
const SET_PIECE_PARTS: Readonly<Record<SetPieceShape, readonly string[]>> = {
  vortex: ["dark", "spiral", "core"],
  fold: ["edges", "light"],
  // R3338000: the Terminus cards' set pieces (fx/terminus/).
  rift: ["dark", "tear", "stars"],
  lamp: ["dark", "cone", "glow"],
  chalk: ["slate", "scrawl", "crack", "stamp"],
  bend: ["lens", "clock"],
  freeze: ["frost", "cracks"],
  abyss: ["dark", "chains"],
  crunch: ["collapse", "point"],
  doom: ["dark", "stars"],
};

/** R2801000: a Starship launch's layers, above its counts (fx.css). */
const LAUNCH_PARTS = ["flame", "trail"] as const;

/** A share as a CSS percentage. */
const PERCENT = 100;

/** R3334000: a weld's parts: a clamp either side of the module and the weld's flash between them. */
const WELD_PARTS = ["clamp", "clamp", "flash"] as const;

/** R3334003: a wreck's two halves, fore and aft. */
const WRECK_HALVES = ["fore", "aft"] as const;

function ms(value: number): string {
  return `${Math.max(0, Math.round(value))}ms`;
}

function placeAtPoint(el: HTMLElement, point: { x: number; y: number }): void {
  el.style.setProperty("--fx-x", px(point.x));
  el.style.setProperty("--fx-y", px(point.y));
}

/**
 * R3334000: a Starship's own picture (or one of its modules'), as its card draws it (cards/art, the same
 * procedural SVG the board's face shows): never an image asset (B27), never a card's face (R202).
 */
function picture(doc: Document, className: string, src: string): HTMLImageElement {
  const img = doc.createElement("img");
  img.className = className;
  img.alt = "";
  img.draggable = false;
  img.decoding = "async";
  img.src = src;
  return img;
}

/** R3334001: one of a cinematic's points, at its share of the picture, lit at its time. */
function timedPoint(doc: Document, className: string, point: FxTimedPoint): HTMLElement {
  const el = doc.createElement("div");
  el.className = className;
  el.style.setProperty("--fx-px", `${(point.x * PERCENT).toFixed(2)}%`);
  el.style.setProperty("--fx-py", `${(point.y * PERCENT).toFixed(2)}%`);
  el.style.setProperty("--fx-at", ms(point.atMs));
  return el;
}

/** R3336000: a Battleship bay's two doors, left to right (fx.css). */
const SHIP_DOOR_SIDES = ["left", "right"] as const;

/** R3337003: an Anomaly's rift's layers, bottom to top (fx.css). */
const RIFT_PARTS = ["tear", "flare"] as const;

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

/** R3399002: a takeover's drawing in its art layer, one path a shape, each moved by fx.css under the live takeover as its `part` says. */
function takeoverDrawing(doc: Document, drawing: TakeoverDrawing): SVGSVGElement {
  const svg = doc.createElementNS(SVG_NS, "svg");
  svg.setAttribute("class", "takeover-art-drawing");
  svg.setAttribute("viewBox", drawing.viewBox);
  svg.setAttribute("preserveAspectRatio", "xMidYMid slice");
  svg.setAttribute("aria-hidden", "true");
  for (const shape of drawing.paths) {
    const path = doc.createElementNS(SVG_NS, "path");
    path.setAttribute("class", `takeover-drawing-${shape.part}`);
    path.setAttribute("d", shape.d);
    path.setAttribute("fill", shape.fill);
    svg.appendChild(path);
  }
  return svg;
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
 * R2652001: one item per coin or die, all at once: a window one face tall over a reel that lands on the
 * kept face at `--fx-land-ms`; under it, when Lucky drew more than one face, every face drawn, the kept
 * one marked. The words are already the reader's (fx/rolls.ts).
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

function signedAmount(tone: FxSplatTone, amount: number): string {
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
      if (cue.spin === true) el.setAttribute("data-spin", "true");
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
    case "callout": {
      // R2789000: a keyword's icon over the unit, one svg of one path. `data-callout`, not `data-keyword`, which the
      // card's rules text matches; it names the keyword, never the card (R202).
      const at = boxes.at ?? null;
      const icon = KEYWORD_CALLOUT_ICONS[cue.keyword];
      if (at === null || icon === undefined) return null;
      placeAtCenter(el, at);
      el.setAttribute("data-callout", cue.keyword);
      if (cue.still === true) el.setAttribute("data-still", "true");
      const svg = doc.createElementNS(SVG_NS, "svg");
      svg.setAttribute("class", "fx-callout-icon");
      svg.setAttribute("viewBox", "0 0 100 100");
      svg.setAttribute("aria-hidden", "true");
      const path = doc.createElementNS(SVG_NS, "path");
      path.setAttribute("d", icon.d);
      if (icon.line === true) path.setAttribute("data-line", "true");
      svg.appendChild(path);
      el.appendChild(svg);
      break;
    }
    case "chaos": {
      el.setAttribute("data-text", t(cue.title));
      chaosLines(doc, el, cue.lines);
      break;
    }
    case "roll": {
      el.setAttribute("data-text", t(cue.title));
      // R3338000: Dice of Doom's d100 wears its own look (fx.css).
      if (cue.look !== undefined) el.setAttribute("data-look", cue.look);
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
    case "takeover": {
      // R2722001, R3345000: the board's darkening, the card's full art in 4 depth planes,
      // dolly camera, set signature hand signing across the bottom, and rays/shockwave.
      // R3399002: a look drawn as a picture of its own paints it in its art layer, keyed by the look alone.
      const at = boxes.at ?? null;
      if (at === null) return null;
      placeAtCenter(el, at);
      el.setAttribute("data-look", cue.look);
      tintVars(el, cue.tint);
      el.style.setProperty("--fx-land", `${Math.max(0, cue.landMs)}ms`);

      const stage = cue.stage === null ? null : takeoverStage(doc, cue.stage);

      for (const part of TAKEOVER_PARTS) {
        const child = doc.createElement("div");
        child.className = `fx-takeover-${part}`;
        el.appendChild(child);
        // R3345000: the card's own full-art stage sits over the dark, under the sweep, rays and wave.
        if (part === "dark" && stage !== null) el.appendChild(stage);
        // R3407000: a look of its own may draw its card's picture filling the screen, over the dark and under the sweep.
        if (part === "dark" && cue.art !== undefined) {
          const picture = doc.createElement("img");
          picture.className = "fx-takeover-art";
          picture.alt = "";
          picture.draggable = false;
          picture.decoding = "async";
          picture.src = cue.art;
          el.appendChild(picture);
        }
        // R3400000: a look with art of its own draws it over the dark, under the sweep, rays and wave, one element a part;
        // R3399002: or its drawing, one path a shape.
        const parts = part === "dark" ? takeoverArtParts(cue.look) : [];
        const drawing = part === "dark" ? TAKEOVER_ART_DRAWINGS[cue.look] : undefined;
        if (parts.length === 0 && drawing === undefined) continue;
        const art = doc.createElement("div");
        art.className = "takeover-art";
        art.setAttribute("data-look", cue.look);
        for (const name of parts) {
          const piece = doc.createElement("div");
          piece.className = `takeover-art-${name}`;
          art.appendChild(piece);
        }
        if (drawing !== undefined) art.appendChild(takeoverDrawing(doc, drawing));
        el.appendChild(art);
      }
      break;
    }
    case "figure": {
      // R3400001: a signature's figure over its box, in its tint; mirrored for the other seat, smaller for a second card.
      const at = boxes.at ?? null;
      if (at === null) return null;
      cover(el, at);
      el.setAttribute("data-figure", cue.figure);
      tintVars(el, cue.tint);
      if (cue.side !== undefined) el.setAttribute("data-side", cue.side);
      if (cue.size !== undefined) el.setAttribute("data-size", cue.size);
      break;
    }
    case "setPiece": {
      // R2800000: a board clear's vortex or fold, centred on `at`; the cards it swallows are the stage's (`pull`).
      const at = boxes.at ?? null;
      if (at === null) return null;
      placeAtCenter(el, at);
      el.setAttribute("data-shape", cue.shape);
      el.setAttribute("data-side", cue.side ?? "both");
      tintVars(el, cue.tint);
      el.style.setProperty("--fx-spin", String(cue.spin));
      el.style.setProperty("--fx-hold", `${Math.max(0, cue.holdMs)}ms`);
      // R3338000: one still frame under Reduce Motion, holding nothing.
      if (cue.still === true) el.setAttribute("data-still", "true");
      for (const part of SET_PIECE_PARTS[cue.shape]) {
        const child = doc.createElement("div");
        child.className = `fx-setPiece-${part}`;
        el.appendChild(child);
      }
      // R3338000: each beat a mark of its own (a star lighting, a lane chaining shut), lit at its time.
      const beats = cue.beats ?? [];
      if (beats.length > 0) el.style.setProperty("--fx-beats", String(beats.length));
      beats.forEach((beatMs, index) => {
        const beat = doc.createElement("div");
        beat.className = "fx-setPiece-beat";
        beat.style.setProperty("--fx-beat", `${String(Math.max(0, beatMs))}ms`);
        beat.style.setProperty("--fx-beat-index", String(index));
        el.appendChild(beat);
      });
      break;
    }
    case "launch": {
      // R2801000: a Starship's launch over it: its countdown, each count popping in at its time (3, 2, 1),
      // then its flame and its contrail from the ignition on. It names no card (R202).
      const at = boxes.at ?? null;
      if (at === null) return null;
      placeAtCenter(el, at);
      el.setAttribute("data-size", cue.size);
      if (cue.still === true) el.setAttribute("data-still", "true");
      tintVars(el, cue.tint);
      el.style.setProperty("--fx-ignite", `${Math.max(0, cue.igniteMs)}ms`);
      cue.countsMs.forEach((countMs, k) => {
        const count = doc.createElement("div");
        count.className = "fx-launch-count";
        count.setAttribute("data-text", String(cue.countsMs.length - k));
        count.style.setProperty("--fx-count-at", `${Math.max(0, countMs)}ms`);
        el.appendChild(count);
      });
      for (const part of LAUNCH_PARTS) {
        const child = doc.createElement("div");
        child.className = `fx-launch-${part}`;
        el.appendChild(child);
      }
      break;
    }
    case "module": {
      // R3334000: a Starship module (its picture, never a card's face, R202) flying from `from`'s point to
      // `to`'s: docking on its hardpoint, kicking back where it stands, or torn off the hull and tumbling.
      const from = boxes.from ?? null;
      const to = boxes.to ?? null;
      if (from === null || to === null) return null;
      const start = pointIn(from, atOf(cue.from));
      const end = pointIn(to, atOf(cue.to));
      el.style.setProperty("--fx-x", px(start.x));
      el.style.setProperty("--fx-y", px(start.y));
      el.style.setProperty("--fx-dx", px(end.x - start.x));
      el.style.setProperty("--fx-dy", px(end.y - start.y));
      el.style.setProperty("--fx-size", px(to.width * cue.share));
      el.style.setProperty("--fx-spin", String(cue.spin));
      el.style.setProperty("--fx-land", ms(cue.landMs));
      el.setAttribute("data-motion", cue.motion);
      el.appendChild(picture(doc, "fx-module-img", cue.uri));
      break;
    }
    case "weld": {
      // R3334000: the clamps closing on a docked module and its weld's flash, humming `--fx-hum` of full.
      const at = boxes.at ?? null;
      if (at === null) return null;
      placeAtPoint(el, pointIn(at, atOf(cue.at)));
      el.style.setProperty("--fx-hum", cue.hum.toFixed(3));
      WELD_PARTS.forEach((part, index) => {
        const child = doc.createElement("div");
        child.className = `fx-weld-${part}`;
        if (part === "clamp") child.setAttribute("data-side", String(index));
        el.appendChild(child);
      });
      break;
    }
    case "cinematic": {
      // R3334001: a Starship's launch as a cinematic over the whole layer: the board dims, its picture fills
      // the screen, the camera pans along its hull past each module, its engines light, its public name
      // stamps across it, and it climbs out toward its tile (`at`). `still`: the title card.
      const at = boxes.at ?? null;
      if (at === null) return null;
      placeAtCenter(el, at);
      el.setAttribute("data-size", cue.size);
      if (cue.still === true) el.setAttribute("data-still", "true");
      tintVars(el, cue.tint);
      el.style.setProperty("--fx-stamp", ms(cue.stampMs));
      el.style.setProperty("--fx-climb", ms(cue.climbMs));
      el.style.setProperty("--fx-climb-ms", ms(cue.durationMs - cue.climbMs));
      el.style.setProperty("--fx-pan-at", ms(cue.panAtMs));
      el.style.setProperty("--fx-pan-ms", ms(cue.panEndMs - cue.panAtMs));
      el.style.setProperty("--fx-pull-ms", ms(cue.stampMs - cue.panEndMs));
      const dim = doc.createElement("div");
      dim.className = "fx-cinematic-dim";
      const climb = doc.createElement("div");
      climb.className = "fx-cinematic-climb";
      const camera = doc.createElement("div");
      camera.className = "fx-cinematic-camera";
      camera.appendChild(picture(doc, "fx-cinematic-art", cue.art));
      camera.style.setProperty("--fx-cam-x0", cue.cameraFrom.x.toFixed(4));
      camera.style.setProperty("--fx-cam-y0", cue.cameraFrom.y.toFixed(4));
      camera.style.setProperty("--fx-cam-x1", cue.cameraTo.x.toFixed(4));
      camera.style.setProperty("--fx-cam-y1", cue.cameraTo.y.toFixed(4));
      camera.style.setProperty("--fx-zoom", String(cue.zoom));
      for (const stop of cue.stops) camera.appendChild(timedPoint(doc, "fx-cinematic-stop", stop));
      for (const engine of cue.engines) {
        const lit = timedPoint(doc, "fx-cinematic-engine", engine);
        lit.style.setProperty("--fx-rest", ms(cue.durationMs - engine.atMs));
        for (let k = 0; k < cue.sparks; k += 1) {
          const spark = doc.createElement("div");
          spark.className = "fx-cinematic-spark";
          spark.style.setProperty("--fx-spark", String(k));
          spark.style.setProperty("--fx-sparks", String(cue.sparks));
          lit.appendChild(spark);
        }
        camera.appendChild(lit);
      }
      const name = doc.createElement("div");
      name.className = "fx-cinematic-name";
      name.setAttribute("data-text", cue.name);
      name.setAttribute("data-font", cue.lettering.font);
      name.setAttribute("data-italic", String(cue.lettering.italic));
      name.style.setProperty("--fx-letter-weight", String(cue.lettering.weight));
      name.style.setProperty("--fx-letter-tracking", `${cue.lettering.tracking}em`);
      name.style.setProperty("--fx-letter-fill", cue.letters.fill);
      name.style.setProperty("--fx-letter-edge", cue.letters.edge);
      climb.appendChild(camera);
      climb.appendChild(name);
      el.appendChild(dim);
      el.appendChild(climb);
      break;
    }
    case "doctrine": {
      // R3334001: a War Doctrine announced over its Starship, in its own look; its name the card's public one.
      const at = boxes.at ?? null;
      if (at === null) return null;
      placeAtPoint(el, pointIn(at, atOf(cue.at)));
      el.setAttribute("data-doctrine", cue.doctrine);
      el.setAttribute("data-text", cue.text);
      tintVars(el, cue.tint);
      const insignia = doc.createElement("div");
      insignia.className = "fx-doctrine-insignia";
      el.appendChild(insignia);
      break;
    }
    case "wreck": {
      // R3334003: a Starship's hull breaking in two over its art, each half its picture's fore or aft half.
      const at = boxes.at ?? null;
      if (at === null) return null;
      cover(el, at);
      el.style.setProperty("--fx-break", ms(cue.breakMs));
      el.style.setProperty("--fx-break-ms", ms(cue.durationMs - cue.breakMs));
      el.style.setProperty("--fx-turn", `${cue.turn}deg`);
      for (const half of WRECK_HALVES) {
        const child = picture(doc, "fx-wreck-half", cue.art);
        child.setAttribute("data-half", half);
        el.appendChild(child);
      }
      break;
    }
    case "scorch": {
      // R3334003: a brief scorch settling over the zone a Starship fell in.
      const at = boxes.at ?? null;
      if (at === null) return null;
      cover(el, at);
      break;
    }
    case "ship": {
      // R3336000: a Drone Battleship's layer over it: `count` armor plates, the first `lit` glowing down as the Shield
      // took the hit ("plates"), or a bay's two doors sliding apart over `count` lights ("bay"). It names no card (R202).
      const at = boxes.at ?? null;
      if (at === null) return null;
      placeAtCenter(el, at);
      el.setAttribute("data-move", cue.move);
      tintVars(el, cue.tint);
      el.style.setProperty("--fx-count", String(cue.count));
      el.style.setProperty("--fx-lit", String(cue.lit));
      if (cue.move === "plates") {
        for (let i = 0; i < cue.count; i += 1) {
          const plate = doc.createElement("div");
          plate.className = "fx-ship-plate";
          plate.style.setProperty("--fx-plate", String(i));
          if (i < cue.lit) plate.setAttribute("data-lit", "true");
          el.appendChild(plate);
        }
      } else {
        for (const side of SHIP_DOOR_SIDES) {
          const door = doc.createElement("div");
          door.className = "fx-ship-door";
          door.setAttribute("data-side", side);
          el.appendChild(door);
        }
        for (let i = 0; i < cue.count; i += 1) {
          const light = doc.createElement("i");
          light.className = "fx-ship-light";
          el.appendChild(light);
        }
      }
      break;
    }
    case "rift": {
      // R3337003: an Anomaly's rift down the middle of the caster's half, tearing open at `--fx-tear` and
      // flaring white at `--fx-flare`; the cards it takes are the stage's (`pull`). It names no card (R202).
      const at = boxes.at ?? null;
      if (at === null) return null;
      placeAtCenter(el, at);
      el.setAttribute("data-side", cue.side);
      tintVars(el, cue.tint);
      el.style.setProperty("--fx-tear", `${Math.max(0, cue.tearMs)}ms`);
      el.style.setProperty("--fx-flare", `${Math.max(0, cue.flareMs)}ms`);
      for (const part of RIFT_PARTS) {
        const child = doc.createElement("div");
        child.className = `fx-rift-${part}`;
        el.appendChild(child);
      }
      break;
    }
    case "anomalyTitle": {
      // R3337003: the Anomaly's title card, its public name (`data-text`, no text node) over its picture.
      const at = boxes.at ?? null;
      if (at === null) return null;
      placeAtCenter(el, at);
      if (cue.still === true) el.setAttribute("data-still", "true");
      tintVars(el, cue.tint);
      const picture = doc.createElement("div");
      picture.className = "fx-anomalyTitle-picture";
      const drawn = drawnSvg(doc, cue.picture);
      if (drawn !== null) picture.appendChild(drawn);
      const name = doc.createElement("div");
      name.className = "fx-anomalyTitle-name";
      name.setAttribute("data-text", t(cue.text));
      el.append(picture, name);
      break;
    }
    case "radar": {
      // R3337002: a Scanning Probe's sweep, one wedge turning from `at`. It names no card (R202).
      const at = boxes.at ?? null;
      if (at === null) return null;
      placeAtCenter(el, at);
      el.setAttribute("data-side", cue.side);
      tintVars(el, cue.tint);
      const wedge = doc.createElement("div");
      wedge.className = "fx-radar-wedge";
      el.appendChild(wedge);
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

/**
 * R3337003: SVG markup the art painter drew (cards/art `anomalyMarkup`) as elements of `doc`, drawn in place as the
 * layer's own icons are, never loaded as an image (B27); markup that is no well-formed SVG draws nothing.
 */
function drawnSvg(doc: Document, markup: string): Element | null {
  const Parser = doc.defaultView?.DOMParser;
  if (Parser === undefined) return null;
  const drawn = new Parser().parseFromString(markup, "image/svg+xml").documentElement;
  if (drawn.namespaceURI !== SVG_NS || drawn.localName !== "svg" || drawn.getElementsByTagName("parsererror").length > 0) return null;
  const svg = doc.importNode(drawn, true);
  svg.setAttribute("aria-hidden", "true");
  return svg;
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
  copy.style.removeProperty("--fx-lunge-lift");
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
    // #3347: it leans the way it travels (toward its zone) as it leaves, and straightens in flight.
    const lean = Math.max(-1, Math.min(1, (start.x - landCenter.x) / FX_HOLD_TURN_PX)) * -FX_HOLD_TURN_DEG;
    el.style.setProperty("--fx-rot", `${lean.toFixed(2)}deg`);
  } else {
    el.style.setProperty("--fx-dx", px(0));
    el.style.setProperty("--fx-dy", px(DROP_PX));
    el.style.setProperty("--fx-s", String(DROP_SCALE));
    el.style.setProperty("--fx-o0", "0");
    el.style.setProperty("--fx-rot", "0deg");
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
