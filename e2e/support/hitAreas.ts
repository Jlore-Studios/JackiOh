// R2122000 (#2122): the hit-area measure, shared by spec 47's per-screen smoke and the component specs that hold one
// screen to it (friends-screen.cy.tsx, #3955): every control under a finger takes a tap across 44x44 px and its tap
// area covers no neighbour.

import { COARSE_POINTER_QUERY, upTo } from "../../apps/web/src/breakpoints.ts";
import type { MenuSize, MenuViewport } from "./config.ts";

/** R2122000: the least a control's hit area spans in either dimension; index.css's `--hit-min`. */
export const HIT_MIN_PX = 44;
/** A box may fall this far short (a sub-pixel border, a rounded width) and still be at its mark. */
const SLACK_PX = 0.5;
/** A failure names a control with no testid by this much of its text, and sizes in this many decimals. */
const NAME_CHARS = 30;
export const PX_DIGITS = 1;
/** The interactive elements #2122 names. */
const CONTROLS = 'button, a[href], [role="button"], input, select, [tabindex="0"]';
/** R2122000's exemptions, never measured but still neighbours: a link in running text, and a card. */
const EXEMPT = ":is(p, label) a, .card";
/** One pass over a screen of several hundred controls, retried until the layout has settled. */
const MEASURE_TIMEOUT = 10_000;
/** R2122000's media, index.css's: where the bands are drawn. */
export const HIT_AREA_MEDIA = `${COARSE_POINTER_QUERY}, ${upTo("phone")}`;
/** An iPad's `navigator.maxTouchPoints`, for the touch the wider sizes emulate. */
const TOUCH_POINTS = 5;

type Box = { left: number; top: number; right: number; bottom: number };
type Control = { el: Element; name: string; own: Box; shown: Box; hit: Box; exempt: boolean };

const boxOf = (el: Element): Box => {
  const { left, top, right, bottom } = el.getBoundingClientRect();
  return { left, top, right, bottom };
};
const union = (a: Box, b: Box): Box => ({ left: Math.min(a.left, b.left), top: Math.min(a.top, b.top), right: Math.max(a.right, b.right), bottom: Math.max(a.bottom, b.bottom) });
/** How far two boxes overlap across and down (negative: the gap between them). */
const overlap = (a: Box, b: Box) => ({ x: Math.min(a.right, b.right) - Math.max(a.left, b.left), y: Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) });

/** A checkbox's or a radio's labels, which toggle it. The tag, not `instanceof`: the page is another window. */
function labelsOf(el: Element): Element[] {
  const input = el as HTMLInputElement;
  return el.tagName === "INPUT" && (input.type === "checkbox" || input.type === "radio") && input.labels !== null ? Array.from(input.labels) : [];
}

/** `el`'s box, its labels' and each ::before and ::after that takes taps where its overflow leaves room; one with no width or no height takes none. */
function hitBox(win: Window, el: Element): Box {
  const style = win.getComputedStyle(el);
  let hit = boxOf(el);
  const origin = { x: hit.left + parseFloat(style.borderLeftWidth), y: hit.top + parseFloat(style.borderTopWidth) };
  for (const which of ["::before", "::after"]) {
    const p = win.getComputedStyle(el, which);
    if (p.content === "none" || p.content === "normal" || p.position !== "absolute" || style.position === "static" || p.pointerEvents === "none") continue;
    // A centering translate (inspectable.css's pile band) shifts the box; the rest of a transform is ignored.
    const shift = new DOMMatrixReadOnly(p.transform === "none" ? undefined : p.transform);
    const [x, y, w, h] = [parseFloat(p.left) + shift.e, parseFloat(p.top) + shift.f, parseFloat(p.width), parseFloat(p.height)];
    if (![x, y, w, h].every(Number.isFinite) || w <= SLACK_PX || h <= SLACK_PX || style.clipPath !== "none") continue;
    if (style.overflowX === "visible") hit = { ...hit, left: Math.min(hit.left, origin.x + x), right: Math.max(hit.right, origin.x + x + w) };
    if (style.overflowY === "visible") hit = { ...hit, top: Math.min(hit.top, origin.y + y), bottom: Math.max(hit.bottom, origin.y + y + h) };
  }
  return labelsOf(el).reduce((sum, label) => union(sum, hitBox(win, label)), hit);
}

/** The part of `el` the window and its clipping ancestors show; null when none is (scrolled out of its region). */
function shownBox(win: Window, el: Element): Box | null {
  let shown = { ...boxOf(el) };
  shown = { left: Math.max(shown.left, 0), top: Math.max(shown.top, 0), right: Math.min(shown.right, win.innerWidth), bottom: Math.min(shown.bottom, win.innerHeight) };
  for (let up = el.parentElement; up !== null; up = up.parentElement) {
    const style = win.getComputedStyle(up);
    const clip = boxOf(up);
    if (style.overflowX !== "visible") shown = { ...shown, left: Math.max(shown.left, clip.left), right: Math.min(shown.right, clip.right) };
    if (style.overflowY !== "visible") shown = { ...shown, top: Math.max(shown.top, clip.top), bottom: Math.min(shown.bottom, clip.bottom) };
  }
  return shown.right - shown.left > SLACK_PX && shown.bottom - shown.top > SLACK_PX ? shown : null;
}

function nameOf(el: Element): string {
  const own = el.getAttribute("data-testid");
  if (own !== null) return own;
  const what = `<${el.tagName.toLowerCase()}> "${(el.getAttribute("aria-label") ?? el.textContent ?? "").trim().slice(0, NAME_CHARS)}"`;
  const holder = el.parentElement?.closest("[data-testid]")?.getAttribute("data-testid");
  return holder === undefined || holder === null ? what : `${what} in ${holder}`;
}

/** The controls a tap reaches: shown, taking pointer events, inside the topmost open modal if any. */
function controlsOf(win: Window): Control[] {
  const modal = Array.from(win.document.querySelectorAll('[aria-modal="true"]')).filter((m) => m.getClientRects().length > 0).at(-1);
  const controls: Control[] = [];
  for (const el of win.document.querySelectorAll(CONTROLS)) {
    const style = win.getComputedStyle(el);
    const shown = shownBox(win, el);
    if ((modal !== undefined && !modal.contains(el)) || style.visibility === "hidden" || style.pointerEvents === "none" || shown === null) continue;
    const own = labelsOf(el).reduce((sum, label) => union(sum, boxOf(label)), boxOf(el));
    controls.push({ el, name: nameOf(el), own, shown, hit: hitBox(win, el), exempt: el.matches(EXEMPT) });
  }
  return controls;
}

/**
 * R2122000: a hit area reaches `HIT_MIN_PX / 2` past its center each way, or halfway to a nearer
 * neighbour (another control, neither holding the other, not overlapping it), and covers no neighbour.
 */
export function hitAreaProblems(win: Window): string[] {
  const controls = controlsOf(win);
  const problems: string[] = [];
  for (const c of controls) {
    const others = controls.filter((n) => {
      const o = overlap(c.own, n.own);
      return n !== c && !n.el.contains(c.el) && !c.el.contains(n.el) && (o.x <= SLACK_PX || o.y <= SLACK_PX);
    });
    const half = HIT_MIN_PX / 2;
    const [cx, cy] = [(c.own.left + c.own.right) / 2, (c.own.top + c.own.bottom) / 2];
    const reach = { left: cx - half, top: cy - half, right: cx + half, bottom: cy + half };
    for (const n of others) {
      const o = overlap(c.own, n.own);
      if (o.y > 0 && n.own.right <= c.own.left + SLACK_PX) reach.left = Math.max(reach.left, (n.own.right + c.own.left) / 2);
      if (o.y > 0 && n.own.left >= c.own.right - SLACK_PX) reach.right = Math.min(reach.right, (n.own.left + c.own.right) / 2);
      if (o.x > 0 && n.own.bottom <= c.own.top + SLACK_PX) reach.top = Math.max(reach.top, (n.own.bottom + c.own.top) / 2);
      if (o.x > 0 && n.own.top >= c.own.bottom - SLACK_PX) reach.bottom = Math.min(reach.bottom, (n.own.top + c.own.bottom) / 2);
      const covered = overlap(c.hit, n.shown);
      if (covered.x > SLACK_PX && covered.y > SLACK_PX) problems.push(`${c.name}: its hit area covers ${n.name}`);
    }
    const size = `${(c.hit.right - c.hit.left).toFixed(PX_DIGITS)}x${(c.hit.bottom - c.hit.top).toFixed(PX_DIGITS)} px`;
    const short = (axis: string) => problems.push(`${c.name}: its hit area is ${size}, short of ${String(HIT_MIN_PX)} px ${axis}`);
    if (!c.exempt && (c.hit.left > reach.left + SLACK_PX || c.hit.right < reach.right - SLACK_PX)) short("across");
    if (!c.exempt && (c.hit.top > reach.top + SLACK_PX || c.hit.bottom < reach.bottom - SLACK_PX)) short("down");
  }
  return problems;
}

/** R2122000 at `size`, under its media, retried until the screen has settled. */
export function expectHitAreas(size: MenuSize | MenuViewport): void {
  cy.window({ timeout: MEASURE_TIMEOUT }).should((win) => {
    expect([win.innerWidth, win.innerHeight], "the viewport asked for").to.deep.eq([size.width, size.height]);
    expect(win.matchMedia(HIT_AREA_MEDIA).matches, `${HIT_AREA_MEDIA} holds`).to.eq(true);
    expect(hitAreaProblems(win), "controls whose hit area falls short or covers another").to.deep.eq([]);
  });
}

/** A coarse pointer (and no hover) for every page until turned off: the wider sizes are held under a finger. */
export function emulateTouch(enabled: boolean): void {
  cy.then(() => Cypress.automation("remote:debugger:protocol", { command: "Emulation.setTouchEmulationEnabled", params: { enabled, maxTouchPoints: TOUCH_POINTS } }));
}
