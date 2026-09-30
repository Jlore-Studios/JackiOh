// R438: keyword visuals on the board (issue #40). Every keyword kind has a treatment on the board
// minion, each drawn by shape and found by `data-keyword-fx`; several compose inside their layers'
// caps with at most AMBIENT_MAX loops; reduced motion (the media query and the settings panel's
// attribute) stops every loop and leaves the still mark; a Vanilla unit shows only what its view
// lists. The pixel half (sizes, overlaps, the picture) is e2e/cypress/component/keyword-visuals.cy.tsx.
//
// Vitest stubs CSS imports, so keywords.css is read as text and parsed the way fx/css.test.ts reads
// fx.css: into rules (selectors, declarations, the @media they sit in) and keyframes bodies.

import { existsSync, readFileSync } from "node:fs";
import { resolve } from "node:path";

import { cleanup, render } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import { CATALOG } from "@jackioh/cards";
import { KEYWORD_KINDS, type Keyword, type KeywordKind, type UnitView } from "@jackioh/shared";

import { unit } from "../test/fixtures.ts";
import { DRAWN_BY_MINION } from "./KeywordFx.tsx";
import {
  AMBIENT_MAX,
  BRITTLE_CRACK_STAGES,
  KEYWORD_VISUALS,
  LAYER_CAP,
  brittleStage,
  keywordFxPlan,
  treatedKinds,
  type KeywordLayer,
} from "./keywordVisuals.ts";
import { MinionFace } from "./MinionFace.tsx";
import { faceModel } from "./model.ts";

afterEach(() => {
  cleanup();
});

/* ------------------------------------------------------------------------------------------- *
 * Rendering
 * ------------------------------------------------------------------------------------------- */

const DEF_ID = "core-004";

/** A keyword of `kind`, with a number where the kind carries one. */
function keywordOf(kind: KeywordKind, n = 2): Keyword {
  switch (kind) {
    case "Armor":
    case "Lucky":
    case "Brittle":
    case "Spell Damage":
      return { kind, n };
    default:
      return { kind } as Keyword;
  }
}

function renderUnit(over: Partial<UnitView>): HTMLElement {
  const u = unit("p1", { defId: DEF_ID, ...over });
  const face = faceModel({
    defId: DEF_ID,
    def: CATALOG[DEF_ID],
    radiant: false,
    live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords: u.keywords },
  });
  const { container } = render(<MinionFace face={face} unit={u} />);
  const cf = container.querySelector<HTMLElement>(".cf");
  if (cf === null) throw new Error("MinionFace rendered no .cf");
  return cf;
}

function treatments(cf: HTMLElement): HTMLElement[] {
  return [...cf.querySelectorAll<HTMLElement>("[data-keyword-fx]")];
}

function fxOf(cf: HTMLElement, kind: KeywordKind): HTMLElement | null {
  return cf.querySelector<HTMLElement>(`[data-keyword-fx="${kind}"]`);
}

const EVERY_KEYWORD: Keyword[] = KEYWORD_KINDS.map((kind) => keywordOf(kind));

/* ------------------------------------------------------------------------------------------- *
 * keywords.css, parsed
 * ------------------------------------------------------------------------------------------- */

function readSheet(fromWeb: string): string {
  for (const candidate of [fromWeb, `apps/web/${fromWeb}`]) {
    const path = resolve(process.cwd(), candidate);
    if (existsSync(path)) return readFileSync(path, "utf8");
  }
  throw new Error(`${fromWeb} not found from ${process.cwd()}`);
}

type Rule = { selectors: string[]; decls: Map<string, string>; media: string | null };

function closingBrace(src: string, open: number): number {
  let depth = 0;
  for (let i = open; i < src.length; i += 1) {
    if (src[i] === "{") depth += 1;
    else if (src[i] === "}") {
      depth -= 1;
      if (depth === 0) return i;
    }
  }
  return src.length;
}

function splitTop(text: string, sep: string): string[] {
  const out: string[] = [];
  let depth = 0;
  let start = 0;
  for (let i = 0; i < text.length; i += 1) {
    const ch = text[i];
    if (ch === "(" || ch === "[") depth += 1;
    else if (ch === ")" || ch === "]") depth -= 1;
    else if (ch === sep && depth === 0) {
      out.push(text.slice(start, i));
      start = i + 1;
    }
  }
  out.push(text.slice(start));
  return out.map((part) => part.trim()).filter((part) => part !== "");
}

function parseDecls(text: string): Map<string, string> {
  const decls = new Map<string, string>();
  for (const part of splitTop(text, ";")) {
    const colon = part.indexOf(":");
    if (colon > 0) decls.set(part.slice(0, colon).trim().toLowerCase(), part.slice(colon + 1).trim().replace(/\s+/g, " "));
  }
  return decls;
}

type Sheet = { rules: Rule[]; keyframes: Map<string, string>; text: string };

function walk(src: string, media: string | null, sheet: Sheet): void {
  let i = 0;
  while (i < src.length) {
    const open = src.indexOf("{", i);
    if (open < 0) return;
    const head = src.slice(i, open);
    const prelude = head.slice(Math.max(head.lastIndexOf(";"), head.lastIndexOf("}")) + 1).trim();
    const close = closingBrace(src, open);
    const body = src.slice(open + 1, close);
    if (prelude.startsWith("@keyframes")) sheet.keyframes.set(prelude.replace(/^@keyframes\s+/, "").trim(), body);
    else if (prelude.startsWith("@media")) walk(body, prelude, sheet);
    else if (prelude.startsWith("@")) walk(body, media, sheet);
    else sheet.rules.push({ selectors: splitTop(prelude, ","), decls: parseDecls(body), media });
    i = close + 1;
  }
}

function parseSheet(css: string): Sheet {
  const text = css.replace(/\/\*[\s\S]*?\*\//g, "");
  const sheet: Sheet = { rules: [], keyframes: new Map(), text };
  walk(text, null, sheet);
  return sheet;
}

const norm = (selector: string): string => selector.replace(/\s+/g, " ").trim();
const REDUCED = /prefers-reduced-motion\s*:\s*reduce/;

const css = parseSheet(readSheet("src/cards/keywords.css"));
const cardsCss = parseSheet(readSheet("src/cards/cards.css"));

/** The property: value pairs a keyframes body animates, every stop together. */
function animatedProperties(body: string): string[] {
  const out: string[] = [];
  for (const match of body.matchAll(/\{([^{}]*)\}/g)) for (const name of parseDecls(match[1] ?? "").keys()) out.push(name);
  return out;
}

/* ------------------------------------------------------------------------------------------- *
 * The map
 * ------------------------------------------------------------------------------------------- */

describe("R438 the keyword visuals map", () => {
  it("R438 every KeywordKind has a visual: a total map with a layer, a shape and a unique priority", () => {
    expect(new Set(Object.keys(KEYWORD_VISUALS))).toEqual(new Set(KEYWORD_KINDS));
    const priorities = KEYWORD_KINDS.map((kind) => KEYWORD_VISUALS[kind].priority).sort((a, b) => a - b);
    expect(priorities).toEqual(KEYWORD_KINDS.map((_, index) => index + 1));
    for (const kind of KEYWORD_KINDS) {
      const visual = KEYWORD_VISUALS[kind];
      expect(Object.keys(LAYER_CAP), kind).toContain(visual.layer);
      expect(visual.shape.length, kind).toBeGreaterThan(0);
    }
  });

  it("R438 every keyword but Taunt, Divine Shield and Armor loops, each on keyframes of its own", () => {
    const still: KeywordKind[] = ["Taunt", "Divine Shield", "Armor"];
    const names = new Set<string>();
    for (const kind of KEYWORD_KINDS) {
      const motion = KEYWORD_VISUALS[kind].motion;
      if (still.includes(kind)) {
        expect(motion, kind).toBeNull();
        continue;
      }
      expect(motion, kind).not.toBeNull();
      if (motion === null) continue;
      expect(names.has(motion.keyframes), `${kind} shares ${motion.keyframes}`).toBe(false);
      names.add(motion.keyframes);
    }
    // Only the two speed keywords wait on `canAct`.
    const onCanAct = KEYWORD_KINDS.filter((kind) => KEYWORD_VISUALS[kind].motion?.when === "canAct");
    expect(onCanAct.sort()).toEqual(["Charge", "Rush"]);
  });

  it("R438 the numbered keywords print their number: Armor, Brittle, Spell Damage and Lucky", () => {
    const numbered = KEYWORD_KINDS.filter((kind) => KEYWORD_VISUALS[kind].numbered);
    expect(numbered.sort()).toEqual(["Armor", "Brittle", "Lucky", "Spell Damage"]);
  });

  it("R438 Divine Shield and Armor are MinionFace's own elements; every other kind is drawn by KeywordFx", () => {
    expect([...DRAWN_BY_MINION].sort()).toEqual(["Armor", "Divine Shield"]);
  });
});

/* ------------------------------------------------------------------------------------------- *
 * One keyword at a time
 * ------------------------------------------------------------------------------------------- */

describe("R438 a unit with each keyword renders its treatment", () => {
  for (const kind of KEYWORD_KINDS) {
    it(`R438 ${kind}: one [data-keyword-fx] on its layer, drawn by keywords.css`, () => {
      const keyword = keywordOf(kind, 3);
      const cf = renderUnit({ keywords: [keyword], armor: kind === "Armor" ? 3 : 0 });
      const all = treatments(cf);
      expect(all.map((el) => el.getAttribute("data-keyword-fx"))).toEqual([kind]);
      const fx = all[0] as HTMLElement;
      const visual = KEYWORD_VISUALS[kind];
      expect(fx.getAttribute("data-kw-layer")).toBe(visual.layer);

      // Where it sits: the bubble and the plate are the minion's own elements, glyphs sit in their row.
      if (kind === "Divine Shield") expect(fx.classList.contains("shield-icon")).toBe(true);
      else if (kind === "Armor") expect(fx.classList.contains("stat-armor")).toBe(true);
      else {
        expect(fx.classList.contains("kw-fx")).toBe(true);
        expect(fx.parentElement?.classList.contains(visual.layer === "glyph" ? "kw-glyphs" : "cf-scale")).toBe(true);
        // A face is span, strong, img and aria-hidden SVG (B12), and the treatment adds no text: the
        // chip still names the keyword, and the unit's text content is what it was.
        for (const el of [fx, ...fx.querySelectorAll("*")]) {
          if (el.namespaceURI === "http://www.w3.org/1999/xhtml") expect(el.tagName.toLowerCase()).toBe("span");
          else {
            expect(el.closest("svg")?.getAttribute("aria-hidden")).toBe("true");
            expect(["text", "title"]).not.toContain(el.tagName.toLowerCase());
          }
          expect(el.hasAttribute("data-keyword"), "no second [data-keyword]").toBe(false);
        }
        expect(fx.textContent).toBe("");
      }

      // Its number, from the view.
      if (visual.numbered) expect(fx.getAttribute("data-n")).toBe("3");
      else expect(fx.hasAttribute("data-n")).toBe(false);

      // It moves (or not) as its map entry says; a single keyword is always within the cap.
      if (visual.motion === null) expect(fx.hasAttribute("data-kw-motion")).toBe(false);
      else expect(fx.getAttribute("data-kw-motion")).toBe("on");

      // The chip is still there, one per keyword.
      expect(cf.querySelectorAll(`.keywords > [data-keyword="${kind}"]`)).toHaveLength(1);

      // keywords.css (or, for the bubble and the plate, cards.css) draws it.
      const sheet = DRAWN_BY_MINION.includes(kind) ? cardsCss : css;
      const hook = kind === "Divine Shield" ? ".shield-icon" : kind === "Armor" ? ".stat-armor" : `[data-keyword-fx="${kind}"]`;
      const drawn = sheet.rules.some((rule) => rule.selectors.some((s) => s.includes(hook)));
      const partsDrawn = [...fx.querySelectorAll(".kw-part")].every((part) =>
        [...part.classList]
          .filter((c) => c !== "kw-part" && c !== "kw-anim")
          .some((c) => sheet.rules.some((rule) => rule.selectors.some((s) => s.includes(`.${c}`)))),
      );
      expect(drawn || partsDrawn, `${kind} has its own rule`).toBe(true);
    });
  }

  it("R438 Taunt's shield is the Taunt treatment, and the steel ring still keys on data-taunt", () => {
    const cf = renderUnit({ keywords: [{ kind: "Taunt" }] });
    expect(cf.getAttribute("data-taunt")).toBe("true");
    expect(fxOf(cf, "Taunt")?.querySelector(".kw-taunt-shield")).not.toBeNull();
    expect(cardsCss.rules.some((rule) => rule.selectors.some((s) => norm(s) === '.cf[data-taunt="true"] .cf-portrait'))).toBe(true);
    // The shield moved: nothing draws it twice.
    expect(cardsCss.text).not.toMatch(/data-taunt="true"\]\s*\.cf-portrait::before/);
    const none = renderUnit({ keywords: [] });
    expect(none.hasAttribute("data-taunt")).toBe(false);
    expect(fxOf(none, "Taunt")).toBeNull();
  });

  it("R438 the Divine Shield bubble sits under the name plate, and the chip column stops above it", () => {
    // Measured in a browser by keyword-visuals.cy.tsx; here, the two rules that make it so.
    const zOf = (selector: string): number =>
      Number(
        cardsCss.rules
          .filter((rule) => rule.media === null && rule.decls.has("z-index") && rule.selectors.some((s) => norm(s) === selector))
          .at(-1)
          ?.decls.get("z-index"),
      );
    expect(zOf(".cf .shield-icon")).toBeLessThan(zOf(".cf .card-name"));
    const chips = cardsCss.rules.find((rule) => rule.selectors.some((s) => norm(s) === ".cf .keywords"));
    expect(chips?.decls.get("flex-wrap")).toBe("wrap");
    expect(chips?.decls.get("max-height")).toBe("calc(100% - 24cqmin - 2px - var(--mf-stat, 28cqmin) - var(--mf-plate, 11cqh))");
  });

  it("R438 Armor's plate carries its treatment only while the view's armor is above 0", () => {
    const plated = renderUnit({ keywords: [{ kind: "Armor", n: 2 }], armor: 2 });
    expect(fxOf(plated, "Armor")?.getAttribute("data-n")).toBe("2");
    const bare = renderUnit({ keywords: [{ kind: "Armor", n: 2 }], armor: 0 });
    expect(fxOf(bare, "Armor")).toBeNull();
    expect(bare.querySelector(".stat-armor")).toBeNull();
  });
});

/* ------------------------------------------------------------------------------------------- *
 * Composition
 * ------------------------------------------------------------------------------------------- */

describe("R438 several keywords compose", () => {
  it("R438 a unit with every keyword draws each layer within its cap, in priority order, with at most AMBIENT_MAX loops", () => {
    const cf = renderUnit({ keywords: EVERY_KEYWORD, armor: 2 });
    const drawn = treatments(cf);
    const byLayer = (layer: KeywordLayer): KeywordKind[] =>
      drawn.filter((el) => el.getAttribute("data-kw-layer") === layer).map((el) => el.getAttribute("data-keyword-fx") as KeywordKind);

    for (const layer of Object.keys(LAYER_CAP) as KeywordLayer[]) {
      const kinds = KEYWORD_KINDS.filter((kind) => KEYWORD_VISUALS[kind].layer === layer).sort(
        (a, b) => KEYWORD_VISUALS[a].priority - KEYWORD_VISUALS[b].priority,
      );
      // Each layer draws exactly its first LAYER_CAP kinds by priority.
      expect(new Set(byLayer(layer)), layer).toEqual(new Set(kinds.slice(0, LAYER_CAP[layer])));
    }
    // The keywords that decide what the opponent may do are always drawn.
    for (const kind of ["Taunt", "Divine Shield", "Can't attack", "Immune to Spells", "Poisonous"] as const) {
      expect(fxOf(cf, kind), kind).not.toBeNull();
    }

    const looping = drawn.filter((el) => el.getAttribute("data-kw-motion") === "on");
    expect(looping.length).toBeLessThanOrEqual(AMBIENT_MAX);
    expect(looping.length).toBe(AMBIENT_MAX);
    // The loops that run are the highest-priority ones that loop.
    const loopers = drawn
      .filter((el) => el.hasAttribute("data-kw-motion"))
      .map((el) => el.getAttribute("data-keyword-fx") as KeywordKind)
      .sort((a, b) => KEYWORD_VISUALS[a].priority - KEYWORD_VISUALS[b].priority);
    expect(looping.map((el) => el.getAttribute("data-keyword-fx")).sort()).toEqual(loopers.slice(0, AMBIENT_MAX).sort());

    // Every keyword still has its chip badge, so nothing a capped layer leaves out is lost.
    expect(cf.querySelectorAll(".keywords > [data-keyword]")).toHaveLength(KEYWORD_KINDS.length);
  });

  it("R438 frames and veils draw in priority order, so a later one paints over an earlier one", () => {
    const plan = keywordFxPlan({ keywords: EVERY_KEYWORD, canAct: true, armor: 2 });
    const priorities = plan.map((entry) => entry.visual.priority);
    expect(priorities).toEqual([...priorities].sort((a, b) => a - b));
  });

  it("R438 Charge and Rush stream only while the view's canAct is true, and a still streak takes no loop", () => {
    const keywords: Keyword[] = [{ kind: "Charge" }, { kind: "Rush" }, { kind: "First Strike" }];
    const acting = renderUnit({ keywords, canAct: true });
    expect(fxOf(acting, "Charge")?.getAttribute("data-kw-motion")).toBe("on");
    expect(fxOf(acting, "Rush")?.getAttribute("data-kw-motion")).toBe("on");
    // Two loops are running, so First Strike's glint waits.
    expect(fxOf(acting, "First Strike")?.getAttribute("data-kw-motion")).toBe("off");
    cleanup();

    const resting = renderUnit({ keywords, canAct: false });
    expect(fxOf(resting, "Charge")?.getAttribute("data-kw-motion")).toBe("off");
    expect(fxOf(resting, "Rush")?.getAttribute("data-kw-motion")).toBe("off");
    expect(fxOf(resting, "First Strike")?.getAttribute("data-kw-motion")).toBe("on");
    // Still drawn: the streaks are the mark, only their motion follows canAct.
    expect(fxOf(resting, "Charge")?.querySelector(".kw-streaks")).not.toBeNull();
  });

  it("R438 Brittle reads the view's count, the keyword's number otherwise, and its cracks deepen as it falls", () => {
    expect(brittleStage(BRITTLE_CRACK_STAGES + 2)).toBe(1);
    expect(brittleStage(3)).toBe(1);
    expect(brittleStage(2)).toBe(2);
    expect(brittleStage(1)).toBe(3);
    expect(brittleStage(0)).toBe(BRITTLE_CRACK_STAGES);

    const printed = renderUnit({ keywords: [{ kind: "Brittle", n: 3 }] });
    expect(fxOf(printed, "Brittle")?.getAttribute("data-n")).toBe("3");
    expect(fxOf(printed, "Brittle")?.getAttribute("data-stage")).toBe("1");
    cleanup();
    const counted = renderUnit({ keywords: [{ kind: "Brittle", n: 3 }], brittle: 1 });
    expect(fxOf(counted, "Brittle")?.getAttribute("data-n")).toBe("1");
    expect(fxOf(counted, "Brittle")?.getAttribute("data-stage")).toBe("3");
    // Every stage's cracks are in the DOM; keywords.css shows the first `stage` of them.
    expect(fxOf(counted, "Brittle")?.querySelectorAll(".kw-crack")).toHaveLength(BRITTLE_CRACK_STAGES);
    const hides = (stage: number, crack: number): boolean =>
      css.rules.some(
        (rule) =>
          rule.decls.get("display") === "none" &&
          rule.selectors.some((s) => s.includes(`[data-stage="${String(stage)}"]`) && s.includes(`.kw-crack-${String(crack)}`)),
      );
    for (let stage = 1; stage <= BRITTLE_CRACK_STAGES; stage += 1) {
      for (let crack = 1; crack <= BRITTLE_CRACK_STAGES; crack += 1) expect(hides(stage, crack), `stage ${String(stage)} crack ${String(crack)}`).toBe(crack > stage);
    }
    expect(css.rules.some((rule) => rule.selectors.some((s) => s.includes('[data-keyword-fx="Brittle"]::after')) && rule.decls.get("content") === "attr(data-n)")).toBe(true);
  });

  it("R438 Lucky shows its stacked X and Spell Damage its bonus", () => {
    const cf = renderUnit({ keywords: [{ kind: "Lucky", n: 2 }, { kind: "Lucky", n: 1 }, { kind: "Spell Damage", n: 1 }] });
    expect(fxOf(cf, "Lucky")?.getAttribute("data-n")).toBe("3");
    expect(cf.querySelectorAll('[data-keyword-fx="Lucky"]')).toHaveLength(1);
    expect(fxOf(cf, "Spell Damage")?.getAttribute("data-n")).toBe("1");
    const bonus = css.rules.find((rule) => rule.selectors.some((s) => s.includes('[data-keyword-fx="Spell Damage"]::after')));
    expect(bonus?.decls.get("content")).toBe('"+" attr(data-n)');
  });

  it("R438 a backrow card standing as a Unit wears the Animated treatment from the view's animated", () => {
    expect(treatedKinds({ keywords: [], canAct: true, armor: 0, animated: {} })).toEqual(["Animated"]);
    expect(treatedKinds({ keywords: [], canAct: true, armor: 0, animated: { home: 2 } })).toEqual(["Animated on your turn"]);
    // Its own keyword already names it: drawn once.
    expect(treatedKinds({ keywords: [{ kind: "Animated on your turn" }], canAct: true, armor: 0, animated: { home: 2 } })).toEqual([
      "Animated on your turn",
    ]);
    const cf = renderUnit({ keywords: [], animated: {} });
    expect(fxOf(cf, "Animated")?.querySelectorAll(".kw-cog")).toHaveLength(2);
  });
});

/* ------------------------------------------------------------------------------------------- *
 * Vanilla
 * ------------------------------------------------------------------------------------------- */

describe("R438 a Vanilla unit shows only what its view lists", () => {
  it("R438 a Vanilla unit with no keywords draws no treatment, not even from animated", () => {
    const cf = renderUnit({ vanilla: true, keywords: [], animated: {} });
    expect(treatments(cf)).toHaveLength(0);
    expect(cf.querySelector(".kw-fx, .kw-glyphs")).toBeNull();
  });

  it("R438 a Vanilla unit that kept a keyword draws that one and only that one", () => {
    const cf = renderUnit({ vanilla: true, keywords: [{ kind: "Poisonous" }] });
    expect(treatments(cf).map((el) => el.getAttribute("data-keyword-fx"))).toEqual(["Poisonous"]);
  });
});

/* ------------------------------------------------------------------------------------------- *
 * Motion and reduced motion (keywords.css)
 * ------------------------------------------------------------------------------------------- */

describe("R438 motion is cheap, capped and stops under reduced motion", () => {
  /** Every rule that starts a loop: any animation declaration but the reduced-motion blocks' `none`. */
  const animationRules = css.rules.filter(
    (rule) =>
      ["animation", "animation-name", "animation-duration"].some((name) => rule.decls.has(name)) &&
      !(rule.decls.get("animation") ?? "").startsWith("none"),
  );

  it("R438 each loop's keyframes exist, move only transform and opacity, and run only on data-kw-motion=on", () => {
    for (const kind of KEYWORD_KINDS) {
      const motion = KEYWORD_VISUALS[kind].motion;
      if (motion === null) continue;
      const body = css.keyframes.get(motion.keyframes);
      expect(body, `@keyframes ${motion.keyframes}`).toBeDefined();
      const moved = animatedProperties(body ?? "");
      expect(moved.length, motion.keyframes).toBeGreaterThan(0);
      for (const property of moved) expect(["transform", "opacity"], `${motion.keyframes} animates ${property}`).toContain(property);
      const runs = animationRules.some(
        (rule) =>
          (rule.decls.get("animation") ?? "").startsWith(motion.keyframes) &&
          rule.selectors.some((s) => s.includes(`[data-keyword-fx="${kind}"]`) && s.includes('[data-kw-motion="on"]')),
      );
      expect(runs, `${kind} runs ${motion.keyframes} under [data-kw-motion="on"]`).toBe(true);
    }
    // Nothing loops outside that gate, and every loop is a named period, never a bare number.
    for (const rule of animationRules) {
      for (const selector of rule.selectors) expect(selector, "gated on data-kw-motion").toContain('[data-kw-motion="on"]');
      const value = rule.decls.get("animation") ?? rule.decls.get("animation-duration") ?? "";
      expect(value, rule.selectors.join(", ")).toMatch(/var\(--kw-period-[a-z-]+\)/);
    }
  });

  it("R438 the media query stops every loop in a treatment", () => {
    const reduced = css.rules.filter((rule) => rule.media !== null && REDUCED.test(rule.media));
    const stops = reduced.find((rule) => (rule.decls.get("animation") ?? "").startsWith("none"));
    expect(stops).toBeDefined();
    expect(stops?.selectors.map(norm)).toEqual(expect.arrayContaining([".cf .kw-fx", ".cf .kw-fx *", ".cf .kw-fx::after"]));
  });

  it('R438 the settings panel\'s Reduce motion (<html data-reduce-motion="true">) stops them too', () => {
    const attribute = css.rules.find(
      (rule) => rule.media === null && rule.selectors.every((s) => s.startsWith(':root[data-reduce-motion="true"]')) && (rule.decls.get("animation") ?? "").startsWith("none"),
    );
    expect(attribute).toBeDefined();
    expect(attribute?.selectors.map(norm)).toEqual(
      expect.arrayContaining([
        ':root[data-reduce-motion="true"] .cf .kw-fx',
        ':root[data-reduce-motion="true"] .cf .kw-fx *',
        ':root[data-reduce-motion="true"] .cf .kw-fx::after',
      ]),
    );
  });

  it("R438 a still mark stays: no treatment is drawn only by parts that are invisible at rest", () => {
    // A part whose resting rule hides it (Immutable's glint) is no mark; each kind needs another part.
    const hidden = (className: string): boolean =>
      css.rules.some(
        (rule) =>
          rule.media === null &&
          rule.selectors.some((s) => norm(s) === `.cf .${className}`) &&
          (rule.decls.get("opacity") === "0" || rule.decls.get("display") === "none"),
      );
    for (const kind of KEYWORD_KINDS) {
      if (DRAWN_BY_MINION.includes(kind)) continue;
      const one = renderUnit({ keywords: [keywordOf(kind)] });
      const parts = [...(fxOf(one, kind)?.querySelectorAll<HTMLElement>(".kw-part") ?? [])];
      expect(parts.length, kind).toBeGreaterThan(0);
      const visible = parts.filter((part) => ![...part.classList].some((c) => c !== "kw-part" && c !== "kw-anim" && hidden(c)));
      expect(visible.length, `${kind} keeps a mark at rest`).toBeGreaterThan(0);
      cleanup();
    }
  });

  it("R438 a treatment never takes a pointer event", () => {
    const rule = css.rules.find((r) => r.selectors.some((s) => norm(s) === ".cf .kw-fx"));
    expect(rule?.decls.get("pointer-events")).toBe("none");
    const row = css.rules.find((r) => r.selectors.some((s) => norm(s) === ".cf .kw-glyphs"));
    expect(row?.decls.get("pointer-events")).toBe("none");
  });
});
