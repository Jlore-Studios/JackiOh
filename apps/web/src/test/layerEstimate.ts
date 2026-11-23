// R4493000, R4493001 (issue #4493): an estimate, from fx.css's text, of the composited layers a mounted DOM cue holds.
// jsdom lays nothing out, so this reads the sheet's rules (cssSheet.ts) the way the browser would cascade them onto each
// element and pseudo-element of the cue: a part is its own layer when it carries `will-change` or runs an animation that
// moves it by transform or opacity, and it covers its box at its largest scale (its static transform or scale, or any
// stop of its keyframes), squared, since a browser rasterizes a layer at the largest scale it animates to. A value it
// cannot read throws, naming it, so a new kind of length is taught here rather than miscounted. The daily component spec
// (`fx-phone-layers.cy.tsx`) measures the same parts in a real browser, so the two cannot drift apart.

import { FX_BYTES_PER_MB, FX_LAYER_BYTES_PER_PX, FX_LAYER_MAX_SCREENS, FX_PHONE_LAYER_BUDGET_MB } from "../fx/constants.ts";
import { keyframeBodies, parseDecls, splitTop, type Sheet } from "./cssSheet.ts";

/** The window the estimate lays a cue out in, CSS px. */
export type LayerView = { width: number; height: number };
/** One promoted part: its box, its largest scale and the screens' worth of the window it covers at that scale. */
export type LayerPart = { label: string; width: number; height: number; scale: number; screens: number };

/** A character of text, as wide as this share of its font size (a mean glyph). */
const TEXT_CHAR_EM = 0.6;
/** A line of text, this many font sizes tall. */
const TEXT_LINE_EM = 1.2;
/** The font size, CSS px, with none declared: the browser's. */
const DEFAULT_FONT_PX = 16;
/** A CSS percentage of its whole. */
const PERCENT = 100;

/** Thrown when a length is a share of a box not known yet: a part whose size hangs on its parent's. */
class Dependent extends Error {}

type Pseudo = "" | "::before" | "::after";
type Decls = Map<string, string>;
type Ctx = { vars: (name: string) => string | undefined; base: number | null; em: number; view: LayerView };
type Val = { n: number; len: boolean };

const REDUCED = /prefers-reduced-motion\s*:\s*reduce/;
const PSEUDO_TAIL = /::?(before|after)$/;
const OTHER_PSEUDO = /::[\w-]+/;
const TRANSFORMING = new Set(["transform", "scale", "translate", "rotate", "opacity"]);
const SIDES = ["top", "right", "bottom", "left"] as const;

/* Values: var() substitution, then calc() arithmetic over lengths */

/** The index of the `)` closing the `(` at `open`. */
function closeParen(text: string, open: number): number {
  let depth = 0;
  for (let i = open; i < text.length; i += 1) {
    if (text[i] === "(") depth += 1;
    else if (text[i] === ")") {
      depth -= 1;
      if (depth === 0) return i;
    }
  }
  throw new Error(`unbalanced parentheses in "${text}"`);
}

/** `text` with every `var(--x, fallback)` replaced by its value, recursively. */
function substitute(text: string, vars: Ctx["vars"]): string {
  let out = text;
  for (let at = out.indexOf("var("); at >= 0; at = out.indexOf("var(")) {
    const close = closeParen(out, at + 3);
    const inner = out.slice(at + 4, close);
    const comma = inner.indexOf(",");
    const name = (comma < 0 ? inner : inner.slice(0, comma)).trim();
    const value = vars(name) ?? (comma < 0 ? undefined : inner.slice(comma + 1).trim());
    if (value === undefined) throw new Error(`no value for ${name} in "${text}"`);
    out = out.slice(0, at) + value + out.slice(close + 1);
  }
  return out;
}

const UNIT = /^(-?(?:\d+\.?\d*|\.\d+)(?:e-?\d+)?)(px|%|vw|vh|vmin|vmax|em|rem)?/i;

/** Evaluates a length or a number expression (calc, min, max, clamp; px, %, vw, vh, vmin, vmax, em, rem). */
function evaluate(source: string, ctx: Ctx): Val {
  const text = substitute(source, ctx.vars).trim();
  let i = 0;
  const fail = (): never => {
    throw new Error(`cannot read "${source}" (as "${text}")`);
  };
  const space = (): void => {
    while (i < text.length && /\s/.test(text.charAt(i))) i += 1;
  };
  const unit = (n: number, u: string | undefined): Val => {
    const { width, height } = ctx.view;
    switch ((u ?? "").toLowerCase()) {
      case "":
        return { n, len: false };
      case "px":
        return { n, len: true };
      case "%":
        if (ctx.base === null) throw new Dependent(source);
        return { n: (n / PERCENT) * ctx.base, len: true };
      case "vw":
        return { n: (n * width) / PERCENT, len: true };
      case "vh":
        return { n: (n * height) / PERCENT, len: true };
      case "vmin":
        return { n: (n * Math.min(width, height)) / PERCENT, len: true };
      case "vmax":
        return { n: (n * Math.max(width, height)) / PERCENT, len: true };
      case "em":
        return { n: n * ctx.em, len: true };
      case "rem":
        return { n: n * DEFAULT_FONT_PX, len: true };
      default:
        return fail();
    }
  };
  const args = (): Val[] => {
    const out: Val[] = [];
    for (;;) {
      out.push(sum());
      space();
      if (text.charAt(i) === ",") i += 1;
      else if (text.charAt(i) === ")") {
        i += 1;
        return out;
      } else fail();
    }
  };
  const primary = (): Val => {
    space();
    const rest = text.slice(i);
    const fn = /^(calc|min|max|clamp)\(/i.exec(rest);
    if (fn !== null) {
      i += fn[0].length;
      const vals = args();
      const pick = (values: Val[], f: (...n: number[]) => number): Val => ({ n: f(...values.map((v) => v.n)), len: values.some((v) => v.len) });
      switch (fn[1]!.toLowerCase()) {
        case "calc":
          return vals.length === 1 ? vals[0]! : fail();
        case "min":
          return pick(vals, Math.min);
        case "max":
          return pick(vals, Math.max);
        default: {
          const [lo, mid, hi] = vals;
          if (lo === undefined || mid === undefined || hi === undefined) return fail();
          return { n: Math.max(lo.n, Math.min(mid.n, hi.n)), len: lo.len || mid.len || hi.len };
        }
      }
    }
    if (rest.startsWith("(")) {
      i += 1;
      const inner = sum();
      space();
      if (text.charAt(i) !== ")") fail();
      i += 1;
      return inner;
    }
    const number = UNIT.exec(rest);
    if (number === null) return fail();
    i += number[0].length;
    return unit(Number(number[1]), number[2]);
  };
  const unary = (): Val => {
    space();
    if (text.charAt(i) === "-" && !UNIT.test(text.slice(i))) {
      i += 1;
      const v = unary();
      return { n: -v.n, len: v.len };
    }
    if (text.charAt(i) === "+") i += 1;
    return primary();
  };
  const product = (): Val => {
    let left = unary();
    for (;;) {
      space();
      const op = text.charAt(i);
      if (op !== "*" && op !== "/") return left;
      i += 1;
      const right = unary();
      left = op === "*" ? { n: left.n * right.n, len: left.len || right.len } : { n: left.n / right.n, len: left.len };
    }
  };
  const sum = (): Val => {
    let left = product();
    for (;;) {
      space();
      const op = text.charAt(i);
      if ((op !== "+" && op !== "-") || !/\s/.test(text.charAt(i + 1))) return left;
      i += 1;
      const right = product();
      left = { n: op === "+" ? left.n + right.n : left.n - right.n, len: left.len || right.len };
    }
  };
  const value = sum();
  space();
  if (i !== text.length) fail();
  return value;
}

/** A length in px, or undefined for `auto`, `none` or nothing. */
function lengthOf(value: string | undefined, ctx: Ctx): number | undefined {
  if (value === undefined) return undefined;
  const v = value.trim().toLowerCase();
  if (v === "auto" || v === "none" || v === "initial" || v === "unset" || v === "fit-content" || v === "max-content" || v === "min-content") return undefined;
  return evaluate(value, ctx).n;
}

/** The widths of a `border` (or `border-width`) value's sides, its first length; `none` and a colour or style alone are 0. */
function borderOf(value: string | undefined, ctx: Ctx): number {
  if (value === undefined) return 0;
  for (const token of splitTop(value, " ")) {
    // A var() may hold the width (or the whole value); a colour's own var()s are never read.
    if (/^var\(/i.test(token)) return borderOf(substitute(token, ctx.vars), ctx);
    if (/^(none|hidden)$/i.test(token)) return 0;
    if (/^(thin|medium|thick)$/i.test(token)) return { thin: 1, medium: 3, thick: 5 }[token.toLowerCase() as "thin" | "medium" | "thick"];
    if (UNIT.test(token) || /^(calc|min|max|clamp)\(/i.test(token)) return evaluate(token, ctx).n;
  }
  return 0;
}

/* Scales: the largest a part is drawn at */

/** The largest absolute factor a `transform` value scales by: its scale functions and matrices multiplied together. */
function transformScale(value: string, ctx: Ctx): number {
  const text = /^var\(/.test(value.trim()) ? substitute(value, ctx.vars) : value;
  if (/^none$/i.test(text.trim())) return 1;
  let factor = 1;
  const call = /\b([a-zA-Z0-9]+)\(/g;
  for (let match = call.exec(text); match !== null; match = call.exec(text)) {
    const open = match.index + match[0].length - 1;
    const close = closeParen(text, open);
    call.lastIndex = close + 1;
    const name = match[1]!.toLowerCase();
    if (!name.startsWith("scale") && !name.startsWith("matrix")) continue;
    const nums = splitTop(text.slice(open + 1, close), ",").map((arg) => {
      const v = evaluate(arg, { ...ctx, base: PERCENT });
      return arg.trim().endsWith("%") ? v.n / PERCENT : v.n;
    });
    const at = (k: number): number => nums[k] ?? 0;
    if (name === "scale") factor *= Math.max(Math.abs(at(0)), Math.abs(nums[1] ?? at(0)));
    else if (name === "scalex" || name === "scaley" || name === "scalez") factor *= Math.abs(at(0));
    else if (name === "scale3d") factor *= Math.max(Math.abs(at(0)), Math.abs(at(1)));
    else if (name === "matrix") factor *= Math.max(Math.hypot(at(0), at(1)), Math.hypot(at(2), at(3)));
    else if (name === "matrix3d") factor *= Math.max(Math.hypot(at(0), at(1), at(2)), Math.hypot(at(4), at(5), at(6)));
  }
  return factor;
}

/** The largest absolute factor a `scale` property value scales by. */
function scaleProperty(value: string, ctx: Ctx): number {
  const text = substitute(value, ctx.vars).trim();
  if (/^none$/i.test(text)) return 1;
  return Math.max(
    ...splitTop(text, " ").map((token) => {
      const v = evaluate(token, { ...ctx, base: PERCENT });
      return Math.abs(token.endsWith("%") ? v.n / PERCENT : v.n);
    }),
  );
}

/** Each stop's declarations of a keyframes body. */
function stopsOf(body: string): Decls[] {
  const out: Decls[] = [];
  const stop = /\{([^{}]*)\}/g;
  for (let match = stop.exec(body); match !== null; match = stop.exec(body)) out.push(parseDecls(match[1]!));
  return out;
}

/* The cascade */

type Selector = { base: string; pseudo: Pseudo };
type Compiled = { selectors: Selector[]; decls: Decls; text: string };
type Computed = { decls: Decls; inline: Decls };
/** A sheet's rules, and by each class a selector names, the rules that name it (in sheet order). */
type Index = { rules: Compiled[]; byClass: Map<string, number[]>; always: number[] };

const INDEXES = new WeakMap<Sheet, Index>();

/**
 * A rule may apply to an element when it names one of the element's classes, a `data-fx` attribute or every child of
 * `.fx-dom`: the sheet's rules indexed that way, read once a sheet.
 */
function indexOf(sheet: Sheet): Index {
  const known = INDEXES.get(sheet);
  if (known !== undefined) return known;
  const rules = compile(sheet);
  const byClass = new Map<string, number[]>();
  const always: number[] = [];
  rules.forEach((rule, k) => {
    if (rule.text.includes("[data-fx") || /\.fx-dom\s*>\s*\*/.test(rule.text)) always.push(k);
    for (const name of new Set([...rule.text.matchAll(/\.([\w-]+)/g)].map((match) => match[1]!))) {
      const list = byClass.get(name) ?? [];
      list.push(k);
      byClass.set(name, list);
    }
  });
  const index = { rules, byClass, always };
  INDEXES.set(sheet, index);
  return index;
}

/** A rule's selectors the estimate can read, split from their pseudo-element; reduced-motion rules are left out. */
function compile(sheet: Sheet): Compiled[] {
  const out: Compiled[] = [];
  for (const rule of sheet.rules) {
    if (rule.media !== null) {
      if (REDUCED.test(rule.media)) continue;
      throw new Error(`the layer estimate reads no @media but reduced motion: ${rule.media}`);
    }
    const selectors: Selector[] = [];
    for (const raw of rule.selectors) {
      const s = raw.trim();
      const tail = PSEUDO_TAIL.exec(s);
      const pseudo: Pseudo = tail === null ? "" : tail[1] === "before" ? "::before" : "::after";
      const base = tail === null ? s : s.slice(0, tail.index).trim() || "*";
      if (OTHER_PSEUDO.test(base)) continue;
      selectors.push({ base, pseudo });
    }
    if (selectors.length > 0) out.push({ selectors, decls: rule.decls, text: rule.selectors.join(", ") });
  }
  return out;
}

/** `decls` with `name: value` applied in order: shorthands to their longhands, `animation` clearing an earlier `animation-name`. */
function apply(decls: Decls, name: string, raw: string): void {
  const important = raw.toLowerCase().lastIndexOf("!important");
  const value = (important < 0 ? raw : raw.slice(0, important)).trim();
  if (name === "inset") {
    const parts = splitTop(value, " ");
    const [t, r = t, b = t, l = r] = parts;
    SIDES.forEach((side, k) => decls.set(side, [t, r, b, l][k] ?? "auto"));
    return;
  }
  if (name === "animation") decls.delete("animation-name");
  if (name === "border") for (const side of SIDES) decls.set(`border-${side}-width`, value);
  if (name === "border-width") {
    const [t, r = t, b = t, l = r] = splitTop(value, " ");
    SIDES.forEach((side, k) => decls.set(`border-${side}-width`, [t, r, b, l][k] ?? "0"));
    return;
  }
  if (name === "padding") {
    const [t, r = t, b = t, l = r] = splitTop(value, " ");
    SIDES.forEach((side, k) => decls.set(`padding-${side}`, [t, r, b, l][k] ?? "0"));
    return;
  }
  if (/^border-(top|right|bottom|left)$/.test(name)) decls.set(`${name}-width`, value);
  decls.set(name, value);
}

function classesOf(el: Element): string[] {
  return (el.getAttribute("class") ?? "").split(/\s+/).filter((name) => name !== "");
}

/** The ancestors-and-self walk, for inheritance. */
function lineage(el: Element): Element[] {
  const out: Element[] = [];
  for (let at: Element | null = el; at !== null; at = at.parentElement) out.push(at);
  return out;
}

/**
 * R4493000: every promoted part of the cue mounted at `root`, laid out in `view`: each element and pseudo-element with
 * `will-change` or an animation moving it by transform or opacity, its box at its largest scale. SVG inside an svg is
 * its svg's picture, and a part under `display: none` draws nothing.
 */
export function estimateLayers(sheet: Sheet, root: HTMLElement, view: LayerView): LayerPart[] {
  const index = indexOf(sheet);
  const bodies = keyframeBodies(sheet);
  const cache = new Map<string, Computed>();
  const ids = new Map<Element, number>();
  const idOf = (el: Element): number => {
    const known = ids.get(el);
    if (known !== undefined) return known;
    ids.set(el, ids.size);
    return ids.size - 1;
  };

  /** The declarations the sheet and the element's own style give `el` (or its `pseudo`), later winning. */
  const computed = (el: Element, pseudo: Pseudo): Computed => {
    const key = `${String(idOf(el))}${pseudo}`;
    const known = cache.get(key);
    if (known !== undefined) return known;
    const decls: Decls = new Map();
    const candidates = new Set(index.always);
    for (const name of classesOf(el)) for (const k of index.byClass.get(name) ?? []) candidates.add(k);
    for (const k of [...candidates].sort((a, b) => a - b)) {
      const rule = index.rules[k]!;
      const hit = rule.selectors.some((s) => {
        if (s.pseudo !== pseudo) return false;
        try {
          return el.matches(s.base);
        } catch {
          throw new Error(`the layer estimate cannot match the selector "${s.base}" (${rule.text})`);
        }
      });
      if (hit) for (const [name, value] of rule.decls) apply(decls, name, value);
    }
    const inline: Decls = new Map();
    if (pseudo === "") for (const [name, value] of parseDecls(el.getAttribute("style") ?? "")) apply(inline, name, value);
    for (const [name, value] of inline) decls.set(name, value);
    const out = { decls, inline };
    cache.set(key, out);
    return out;
  };

  /** A custom property of `el` (or its pseudo-element), inherited from its ancestors. */
  const varsOf =
    (el: Element, pseudo: Pseudo): Ctx["vars"] =>
    (name) => {
      if (pseudo !== "") {
        const own = computed(el, pseudo).decls.get(name);
        if (own !== undefined) return own;
      }
      for (const at of lineage(el)) {
        const value = computed(at, "").decls.get(name);
        if (value !== undefined) return value;
      }
      return undefined;
    };

  /** `el`'s (or its pseudo-element's) font size, px. */
  const fontOf = (el: Element, pseudo: Pseudo): number => {
    const parent = pseudo === "" ? el.parentElement : el;
    const inherited = parent === null || !root.contains(parent) ? DEFAULT_FONT_PX : fontOf(parent, "");
    const value = computed(el, pseudo).decls.get("font-size");
    if (value === undefined || /^inherit$/i.test(value)) return inherited;
    return evaluate(value, { vars: varsOf(el, pseudo), base: inherited, em: inherited, view }).n;
  };

  /** The characters of text `el` (or its pseudo-element) draws: its `content`, or its own text nodes. */
  const textOf = (el: Element, pseudo: Pseudo): number => {
    if (pseudo === "") return Array.from(el.childNodes).reduce((n, node) => n + (node.nodeType === 3 ? (node.textContent ?? "").trim().length : 0), 0);
    const content = computed(el, pseudo).decls.get("content") ?? "";
    let chars = 0;
    for (const token of content.match(/attr\(\s*([\w-]+)\s*\)|"(?:[^"\\]|\\.)*"|'(?:[^'\\]|\\.)*'/g) ?? []) {
      const attr = /^attr\(\s*([\w-]+)\s*\)$/.exec(token);
      chars += attr !== null ? (el.getAttribute(attr[1]!) ?? "").length : token.slice(1, -1).replace(/\\[0-9a-f]{1,6}\s?|\\./gi, "x").length;
    }
    return chars;
  };

  /** Whether `el`'s pseudo-element is drawn at all: a `content` other than none. */
  const hasPseudo = (el: Element, pseudo: Pseudo): boolean => {
    const content = computed(el, pseudo).decls.get("content");
    return content !== undefined && !/^(none|normal)$/i.test(content.trim()) && !/^none$/i.test(computed(el, pseudo).decls.get("display") ?? "");
  };

  const shown = (el: Element): boolean => !/^none$/i.test((computed(el, "").decls.get("display") ?? "").trim());
  const isSvgChild = (el: Element): boolean => el instanceof SVGElement && el.ownerSVGElement !== null;

  type Node = { el: Element; pseudo: Pseudo };
  const childrenOf = (node: Node): Node[] => {
    if (node.pseudo !== "") return [];
    const out: Node[] = [];
    if (hasPseudo(node.el, "::before")) out.push({ el: node.el, pseudo: "::before" });
    for (const child of Array.from(node.el.children)) if (!isSvgChild(child) && shown(child)) out.push({ el: child, pseudo: "" });
    if (hasPseudo(node.el, "::after")) out.push({ el: node.el, pseudo: "::after" });
    return out;
  };

  type Box = { width: number; height: number };
  /** `node`'s border box in `cb` (its containing block, null: unknown, so a share of it throws `Dependent`). */
  const boxOf = (node: Node, cb: Box | null): Box => {
    const { decls } = computed(node.el, node.pseudo);
    const position = (decls.get("position") ?? "static").toLowerCase();
    const block = position === "fixed" ? view : cb;
    const absolute = position === "absolute" || position === "fixed";
    const em = fontOf(node.el, node.pseudo);
    const vars = varsOf(node.el, node.pseudo);
    const along = (base: number | null): Ctx => ({ vars, base, em, view });
    const w0 = block?.width ?? null;
    const h0 = block?.height ?? null;
    let width = lengthOf(decls.get("width"), along(w0));
    let height = lengthOf(decls.get("height"), along(h0));
    // A size declared or drawn as text is the content's, which a content-box adds its border and padding to.
    let ownW = width !== undefined;
    let ownH = height !== undefined;
    if (width === undefined && absolute) {
      const left = lengthOf(decls.get("left"), along(w0));
      const right = lengthOf(decls.get("right"), along(w0));
      if (left !== undefined && right !== undefined) width = (w0 ?? Number.NaN) - left - right;
      if (Number.isNaN(width)) throw new Dependent("insets");
    }
    if (height === undefined && absolute) {
      const top = lengthOf(decls.get("top"), along(h0));
      const bottom = lengthOf(decls.get("bottom"), along(h0));
      if (top !== undefined && bottom !== undefined) height = (h0 ?? Number.NaN) - top - bottom;
      if (Number.isNaN(height)) throw new Dependent("insets");
    }
    const ratio = decls.get("aspect-ratio");
    if (ratio !== undefined && !/^auto$/i.test(ratio)) {
      const [a = "1", b = "1"] = substitute(ratio, vars).split("/").map((part) => part.trim());
      const r = Number(a) / Number(b);
      if (!(r > 0)) throw new Error(`cannot read aspect-ratio "${ratio}"`);
      if (width === undefined && height !== undefined) {
        width = height * r;
        ownW = ownH;
      } else if (height === undefined && width !== undefined) {
        height = width / r;
        ownH = ownW;
      }
    }
    const chars = textOf(node.el, node.pseudo);
    if (chars > 0) {
      if (width === undefined) {
        width = chars * TEXT_CHAR_EM * em;
        ownW = true;
      }
      if (height === undefined) {
        height = TEXT_LINE_EM * em;
        ownH = true;
      }
    }
    if (width === undefined || height === undefined) {
      if (absolute) {
        // Shrink to fit: the largest of its parts that do not hang on its own size.
        let w = 0;
        let h = 0;
        for (const child of childrenOf(node)) {
          try {
            const b = boxOf(child, null);
            w = Math.max(w, b.width);
            h = Math.max(h, b.height);
          } catch (error) {
            if (!(error instanceof Dependent)) throw error;
          }
        }
        if (width === undefined) {
          width = w;
          ownW = true;
        }
        if (height === undefined) {
          height = h;
          ownH = true;
        }
      } else {
        if (block === null) throw new Dependent("a static part");
        width ??= block.width;
        height ??= block.height;
      }
    }
    const clamp = (v: number, lo: string | undefined, hi: string | undefined, base: number | null): number => {
      const max = lengthOf(hi, along(base));
      const min = lengthOf(lo, along(base));
      return Math.max(min ?? 0, max === undefined ? v : Math.min(v, max));
    };
    width = clamp(width, decls.get("min-width"), decls.get("max-width"), w0);
    height = clamp(height, decls.get("min-height"), decls.get("max-height"), h0);
    if (!/^border-box$/i.test(decls.get("box-sizing") ?? "")) {
      const edge = (side: (typeof SIDES)[number]): number =>
        borderOf(decls.get(`border-${side}-width`), along(w0)) + (lengthOf(decls.get(`padding-${side}`), along(w0)) ?? 0);
      if (ownW) width += edge("left") + edge("right");
      if (ownH) height += edge("top") + edge("bottom");
    }
    return { width: Math.max(0, width), height: Math.max(0, height) };
  };

  /** The animations `decls` runs, by name. */
  const animationsOf = (decls: Decls): string[] => {
    const named = decls.get("animation-name");
    if (named !== undefined) return splitTop(named, ",").filter((name) => bodies.has(name));
    const shorthand = decls.get("animation");
    if (shorthand === undefined) return [];
    return splitTop(shorthand, ",").flatMap((one) => splitTop(one, " ").filter((token) => bodies.has(token)));
  };

  const parts: LayerPart[] = [];
  const visit = (node: Node, cb: Box): void => {
    const box = boxOf(node, cb);
    const { decls } = computed(node.el, node.pseudo);
    const ctx: Ctx = { vars: varsOf(node.el, node.pseudo), base: null, em: fontOf(node.el, node.pseudo), view };
    const names = animationsOf(decls);
    const stops = names.flatMap((name) => stopsOf(bodies.get(name) ?? ""));
    const willChange = (decls.get("will-change") ?? "auto").trim().toLowerCase();
    const moving = stops.some((stop) => [...stop.keys()].some((name) => TRANSFORMING.has(name)));
    if (willChange !== "auto" || moving) {
      let scale = 1;
      const own = [{ transform: decls.get("transform"), scale: decls.get("scale") }, ...stops.map((stop) => ({ transform: stop.get("transform"), scale: stop.get("scale") }))];
      for (const at of own) {
        if (at.transform !== undefined) scale = Math.max(scale, transformScale(at.transform, ctx));
        if (at.scale !== undefined) scale = Math.max(scale, scaleProperty(at.scale, ctx));
      }
      const label = `${node.el.localName}${classesOf(node.el).map((name) => `.${name}`).join("")}${node.pseudo}`;
      parts.push({ label, width: box.width, height: box.height, scale, screens: (box.width * scale * box.height * scale) / (view.width * view.height) });
    }
    for (const child of childrenOf(node)) visit(child, box);
  };
  visit({ el: root, pseudo: "" }, view);
  return parts;
}

/**
 * R4493000, R4493001: what is wrong with a cue's `parts` at `dpr`: each part covering more than FX_LAYER_MAX_SCREENS, and
 * the cue's layers passing FX_PHONE_LAYER_BUDGET_MB; none, a cue that fits.
 */
export function layerProblems(name: string, parts: readonly LayerPart[], view: LayerView, dpr: number): string[] {
  const problems: string[] = [];
  for (const part of parts) {
    if (part.screens > FX_LAYER_MAX_SCREENS) {
      problems.push(
        `${name}: ${part.label} covers ${part.screens.toFixed(2)} screens (${part.width.toFixed(0)}x${part.height.toFixed(0)} at x${part.scale.toFixed(2)}), past FX_LAYER_MAX_SCREENS`,
      );
    }
  }
  const screens = parts.reduce((sum, part) => sum + part.screens, 0);
  const bytes = screens * view.width * view.height * dpr * dpr * FX_LAYER_BYTES_PER_PX;
  if (bytes > FX_PHONE_LAYER_BUDGET_MB * FX_BYTES_PER_MB) {
    problems.push(`${name}: ${(bytes / FX_BYTES_PER_MB).toFixed(1)} MB of layers (${screens.toFixed(2)} screens) at x${String(dpr)}, past FX_PHONE_LAYER_BUDGET_MB`);
  }
  return problems;
}
