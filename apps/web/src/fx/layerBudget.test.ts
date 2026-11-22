// R4493000, R4493001 (issue #4493): every DOM cue's layers on a phone. Each kind is mounted through `mountDomEffect` at
// its largest case (the diorama with the most slices merged as a phone's stage merges it, every set piece shape, every
// takeover look, every figure, each Starship size) at 390x844, and its promoted parts are estimated from fx.css
// (src/test/layerEstimate.ts): no part may cover more than FX_LAYER_MAX_SCREENS at its largest scale, no cue may hold
// more than FX_PHONE_LAYER_BUDGET_MB at 3x, and the director's tables (layers.ts) must never count a cue short. A new
// cue, or a new full-screen part, fails here before it ships. The daily component spec (fx-phone-layers.cy.tsx) measures
// the same in a real browser.

import { afterEach, describe, expect, it } from "vitest";

import { ANOMALY_CODES } from "../cards/anomalies.ts";
import { DIORAMAS } from "../cards/art/dioramas/index.ts";
import { ART_PLANES } from "../cards/art/pictures.ts";
import { anomalyMarkup } from "../cards/art/svg.ts";
import { KEYWORD_CALLOUT_ICONS } from "../cards/keywordVisuals.ts";
import { keyframeBodies, parseDecls, parseSheet, type Sheet } from "../test/cssSheet.ts";
import { estimateLayers, layerProblems, type LayerPart, type LayerView } from "../test/layerEstimate.ts";
import { readSheet } from "../test/setup.ts";
import { mountDomEffect, type DomEffectBoxes } from "./dom.ts";
import { FX_FIGURES, type FxFigure } from "./figures.ts";
import {
  cueScreens,
  FX_BOARD_CUE_LAYER_SCREENS,
  FX_CUE_LAYER_SCREENS,
  FX_SET_PIECE_LAYER_SCREENS,
  FX_TAKEOVER_DOLLY_SCALES,
  FX_TAKEOVER_FLY_REACH,
  mergeStage,
  takeoverForm,
  TAKEOVER_ARTLESS,
} from "./layers.ts";
import { FX_PHONE_STAGE_LAYERS } from "./constants.ts";
import { STARSHIP_CINEMATIC, STARSHIP_TUNING } from "./starship.ts";
import { TAKEOVER_ART_LOOKS, TAKEOVER_TINTS } from "./takeover.ts";
import { STINKYSCAPE_PLUS_TERMINUS_IDS, TERMINUS_IDS } from "./terminus/ids.ts";
import type { FxAnchor, FxBox, FxDomCue, FxTakeoverCue, FxTakeoverStage, SetPieceShape, StarshipSize, TakeoverLook } from "./types.ts";

/** A phone's window, the one R2803000's scenes play at, at an iPhone's 3x. */
const VIEW: LayerView = { width: 390, height: 844 };
const DPR = 3;
/** Card-sized boxes on that window: the one a cue lands on, and the two a flight goes between. */
const AT: FxBox = { x: 161, y: 470, width: 68, height: 94 };
const FROM: FxBox = { x: 20, y: 140, width: 68, height: 94 };
const TO: FxBox = { x: 300, y: 620, width: 68, height: 94 };
/** The board, for a cue over all of it (Crushing Walls, a set piece, a fan or a sheen over the whole board). */
const BOARD: FxBox = { x: 0, y: 96, width: 390, height: 620 };
const BOXES: DomEffectBoxes = { at: AT, from: FROM, to: TO };

const TINT = { rim: "#ffb35c", core: "#fff1c2", glow: "#ff6a2b" };
const card: FxAnchor = { kind: "testid", testid: "card-u1" };
const other: FxAnchor = { kind: "testid", testid: "card-u2" };
const board: FxAnchor = { kind: "testid", testid: "board" };
const LOOKS = Object.keys(TAKEOVER_TINTS) as TakeoverLook[];
const SIZES: readonly StarshipSize[] = ["small", "medium", "large"];
/** The most beats a Terminus set piece marks: a star for every Terminus card. */
const MOST_BEATS = TERMINUS_IDS.length + STINKYSCAPE_PLUS_TERMINUS_IDS.length;
/** Each shape's beats at their most: the Terminus shapes (R3338000) mark them, the board clears none. */
const SHAPE_BEATS: Readonly<Record<SetPieceShape, number>> = {
  vortex: 0,
  fold: 0,
  rift: MOST_BEATS,
  lamp: MOST_BEATS,
  chalk: MOST_BEATS,
  bend: MOST_BEATS,
  freeze: MOST_BEATS,
  abyss: MOST_BEATS,
  crunch: MOST_BEATS,
  doom: MOST_BEATS,
};
const SHAPES = Object.keys(SHAPE_BEATS) as SetPieceShape[];
/** The figures a recipe lands on the whole board rather than a card (fx/cards: cnFleaMarket, chaosCalls, megabase, seance). */
const BOARD_FIGURES: readonly FxFigure[] = ["lanterns", "dusk"];

/** The diorama with the most slices, each its own picture. */
const DEEPEST = [...DIORAMAS.values()].reduce((a, b) => (b.slices.length > a.slices.length ? b : a));
const DIORAMA_STAGE: FxTakeoverStage = {
  planes: [],
  name: "Ceaseless Void",
  face: "script",
  slices: DEEPEST.slices.map((slice, i) => ({ image: `url(s${String(i)})`, plane: slice.plane, depth: slice.depth })),
};
const PLANE_STAGE: FxTakeoverStage = { planes: ART_PLANES.map((plane) => `url(${plane})`), name: "Prismatic Mythic", face: "script" };

const takeover = (look: TakeoverLook, extra: Partial<FxTakeoverCue>): FxTakeoverCue => ({
  kind: "takeover",
  stage: null,
  look,
  tint: TAKEOVER_TINTS[look],
  at: card,
  landMs: 1240,
  delayMs: 0,
  durationMs: 2600,
  ...extra,
});

/**
 * A takeover's four cases: bare of its art, as it plays without a set piece, as a phone draws it holding the deepest
 * diorama, and its dark and its sweep alone, the least the budget leaves it.
 */
const takeoverCases = (look: TakeoverLook): FxTakeoverCue[] => [
  takeover(look, { artless: true }),
  takeover(look, {}),
  takeover(look, { stage: mergeStage(DIORAMA_STAGE, FX_PHONE_STAGE_LAYERS), ...(TAKEOVER_ART_LOOKS.has(look) ? { art: "data:," } : {}) }),
  takeover(look, { artless: true, sweepOnly: true }),
];

const cinematic = (size: StarshipSize, still: boolean): Extract<FxDomCue, { kind: "cinematic" }> => ({
  kind: "cinematic",
  art: "data:,",
  name: "The Ninth Life of the Long Voyage",
  lettering: { font: "signature", weight: 400, tracking: 0.08, italic: true },
  letters: { fill: "#ffeecc", edge: "#332244" },
  size,
  tint: TINT,
  at: card,
  cameraFrom: { x: 0.2, y: 0.5 },
  cameraTo: { x: 0.8, y: 0.45 },
  panAtMs: 144,
  panEndMs: 1320,
  // R4493000: a phone's camera draws no closer than phoneZoom (starship.ts).
  zoom: Math.min(STARSHIP_TUNING[size].zoom, STARSHIP_CINEMATIC.phoneZoom),
  stops: Array.from({ length: STARSHIP_TUNING[size].pieces + 2 }, (_, k) => ({ x: 0.1 + k * 0.1, y: 0.45, atMs: 400 + k * 100 })),
  engines: Array.from({ length: STARSHIP_CINEMATIC.enginesMax }, (_, k) => ({ x: 0.1, y: 0.3 + k * 0.1, atMs: 1320 + k * 50 })),
  sparks: STARSHIP_TUNING[size].sparks,
  stampMs: 1632,
  climbMs: 2064,
  delayMs: 0,
  durationMs: 2700,
  ...(still ? { still: true as const } : {}),
});

/** Every DOM cue kind, at its largest cases. */
const CASES: { readonly [K in FxDomCue["kind"]]: readonly Extract<FxDomCue, { kind: K }>[] } = {
  splat: (["damage", "heal", "loss", "absorb"] as const).map((tone) => ({ kind: "splat", tone, amount: 88, at: card, delayMs: 0, durationMs: 950 })),
  // A fan lands on a card, or on the whole board (a set piece's centre, a radiant set with no tracker), its largest case.
  rays: (["legendary", "mythic", "radiant", "holy", "victory"] as const).flatMap((tone) => [card, board].map((at) => ({ kind: "rays" as const, tone, at, delayMs: 0, durationMs: 850 }))),
  sheen: [card, board].map((at) => ({ kind: "sheen", at, delayMs: 0, durationMs: 400 })),
  ghost: [
    { kind: "ghost", from: card, to: other, delayMs: 0, durationMs: 400 },
    { kind: "ghost", from: card, to: other, delayMs: 0, durationMs: 400, spin: true },
  ],
  callout: (Object.keys(KEYWORD_CALLOUT_ICONS) as (keyof typeof KEYWORD_CALLOUT_ICONS)[]).flatMap((keyword) => [
    { kind: "callout" as const, keyword, at: card, delayMs: 0, durationMs: 700 },
    { kind: "callout" as const, keyword, at: card, delayMs: 0, durationMs: 700, still: true as const },
  ]),
  arrows: (["up", "down"] as const).map((direction) => ({ kind: "arrows", direction, at: card, delayMs: 0, durationMs: 550 })),
  banner: (["you", "opponent", "muted"] as const).map((tone) => ({ kind: "banner", text: "Opponent's turn", tone, delayMs: 0, durationMs: 1500 })),
  result: (["victory", "defeat", "draw"] as const).map((outcome) => ({ kind: "result", outcome, text: "Victory", delayMs: 0, durationMs: 3200 })),
  fracture: [{ kind: "fracture", at: { kind: "crystal", side: "you", index: 2 }, delayMs: 0, durationMs: 910 }],
  walls: [{ kind: "walls", at: board, reach: 0.5, delayMs: 0, durationMs: 1250 }],
  brand: [{ kind: "brand", at: card, tint: TINT, delayMs: 0, durationMs: 1100 }],
  shield: (["small", "full"] as const).map((size) => ({ kind: "shield", size, at: card, delayMs: 0, durationMs: 700 })),
  chaos: [
    {
      kind: "chaos",
      title: "Call to Chaos:",
      lines: Array.from({ length: 3 }, (_, k) => ({ text: "Summon a Chaos Golem", reel: ["Heal the caster's hero 30", "Cast a random Call to Chaos", "Summon a Chaos Golem"], landMs: 360 + k * 180 })),
      delayMs: 0,
      durationMs: 1800,
    },
  ],
  roll: [
    { kind: "roll", title: "Coin flips", die: "coin", items: Array.from({ length: 6 }, () => ({ faces: ["Heads", "Tails"], kept: 0, tumble: ["Tails", "Heads"], landMs: 400 })), delayMs: 0, durationMs: 1400 },
    { kind: "roll", title: "Dice of Doom", die: "die", items: [{ faces: ["67"], kept: 0, tumble: ["12", "99", "67"], landMs: 900 }], delayMs: 0, durationMs: 1800, look: "doom" },
  ],
  fog: (["damage", "heal"] as const).map((tone) => ({ kind: "fog", tone, from: card, to: other, tint: TINT, icon: { d: "M0 0h24v24H0z", rule: "nonzero" }, delayMs: 0, durationMs: 1100 })),
  zone: (["up", "down", "none"] as const).map((direction) => ({ kind: "zone", at: card, tint: TINT, text: "x12", direction, delayMs: 0, durationMs: 1100 })),
  takeover: [...LOOKS.flatMap(takeoverCases), takeover("prismatic", { stage: mergeStage(PLANE_STAGE, FX_PHONE_STAGE_LAYERS) })],
  setPiece: SHAPES.flatMap((shape) =>
    (["you", "opponent", null] as const).map((side) => ({
      kind: "setPiece" as const,
      shape,
      tint: TINT,
      spin: 1 as const,
      side,
      at: board,
      holdMs: 2000,
      delayMs: 0,
      durationMs: 3200,
      ...(SHAPE_BEATS[shape] > 0 ? { beats: Array.from({ length: SHAPE_BEATS[shape] }, (_, k) => 200 + k * 120) } : {}),
    })),
  ),
  launch: SIZES.flatMap((size) => [
    { kind: "launch" as const, at: card, tint: TINT, size, countsMs: [0, 600, 1200], igniteMs: 1800, delayMs: 0, durationMs: 3400 },
    { kind: "launch" as const, at: card, tint: TINT, size, countsMs: [], igniteMs: 0, delayMs: 0, durationMs: 3400, still: true as const },
  ]),
  ship: [
    { kind: "ship", move: "plates", at: card, tint: TINT, count: 8, lit: 4, delayMs: 0, durationMs: 900 },
    { kind: "ship", move: "bay", at: card, tint: TINT, count: 8, lit: 0, delayMs: 0, durationMs: 1200 },
  ],
  rift: (["you", "opponent"] as const).map((side) => ({ kind: "rift", at: card, side, tint: TINT, tearMs: 280, flareMs: 1540, delayMs: 0, durationMs: 4300 })),
  anomalyTitle: ANOMALY_CODES.flatMap((code) => [
    { kind: "anomalyTitle" as const, at: card, text: "Wormhole of the Forgotten", picture: anomalyMarkup(code, false), tint: TINT, delayMs: 0, durationMs: 1800 },
    { kind: "anomalyTitle" as const, at: card, text: "Wormhole", picture: anomalyMarkup(code, false), tint: TINT, delayMs: 0, durationMs: 1800, still: true as const },
  ]),
  radar: (["you", "opponent"] as const).map((side) => ({ kind: "radar", at: card, side, tint: TINT, delayMs: 0, durationMs: 900 })),
  module: (["dock", "recoil", "blowOff"] as const).map((motion) => ({ kind: "module", uri: "data:,", motion, from: card, to: other, share: 0.4, landMs: 240, spin: 2, delayMs: 0, durationMs: 560 })),
  weld: [{ kind: "weld", at: card, hum: 1, delayMs: 0, durationMs: 320 }],
  cinematic: SIZES.flatMap((size) => [cinematic(size, false), cinematic(size, true)]),
  doctrine: (["land", "sea", "air", "space"] as const).map((doctrine) => ({ kind: "doctrine", doctrine, text: "Space Doctrine", tint: TINT, at: card, delayMs: 0, durationMs: 1100 })),
  wreck: [{ kind: "wreck", art: "data:,", at: card, breakMs: 210, turn: 24, delayMs: 0, durationMs: 1150 }],
  scorch: [{ kind: "scorch", at: card, delayMs: 0, durationMs: 885 }],
  figure: [
    ...FX_FIGURES.flatMap((figure) => [
      { kind: "figure" as const, figure, at: card, tint: TINT, delayMs: 0, durationMs: 700 },
      { kind: "figure" as const, figure, at: card, tint: TINT, delayMs: 0, durationMs: 700, side: "opponent" as const },
    ]),
    // The figures a card's recipe draws over the whole board (CN Flea Market's lanterns, the dusk of a starfield).
    ...BOARD_FIGURES.map((figure) => ({ kind: "figure" as const, figure, at: board, tint: TINT, delayMs: 0, durationMs: 700 })),
  ],
};

const fx = parseSheet(readSheet("src/fx/fx.css"));

/** The design tokens (`:root`'s custom properties) the board's sheets set, on the document, as the app's stylesheets set them. */
function installTokens(): void {
  for (const path of ["src/index.css", "src/typescale.css", "src/surfaces.css"]) {
    for (const rule of parseSheet(readSheet(path)).rules) {
      if (rule.media !== null || !rule.selectors.includes(":root")) continue;
      for (const [name, value] of rule.decls) if (name.startsWith("--")) document.documentElement.style.setProperty(name, value);
    }
  }
}

afterEach(() => {
  document.body.innerHTML = "";
});

/** `cue` mounted in an effects layer as FxLayer renders it, its promoted parts estimated from `sheet`. */
function layersOf(cue: FxDomCue, sheet: Sheet = fx): LayerPart[] {
  installTokens();
  const layer = document.createElement("div");
  layer.className = "fx-layer";
  layer.setAttribute("data-fx", "on");
  const dom = document.createElement("div");
  dom.className = "fx-dom";
  layer.appendChild(dom);
  document.body.appendChild(layer);
  // A cue on the board lands on the board's box, every other on a card's.
  const effect = mountDomEffect(dom, cue, "at" in cue && cue.at === board ? { ...BOXES, at: BOARD } : BOXES);
  if (effect === null) throw new Error(`${cue.kind} mounted nothing`);
  const parts = estimateLayers(sheet, effect.el, VIEW);
  layer.remove();
  return parts;
}

const screensOf = (parts: readonly LayerPart[]): number => parts.reduce((sum, part) => sum + part.screens, 0);

/** A case's name in a failure: its kind and what tells it from its kind's other cases. */
function nameOf(cue: FxDomCue): string {
  const facts = Object.entries(cue)
    .filter(([key, value]) => key !== "kind" && (typeof value === "string" || typeof value === "boolean") && !String(value).startsWith("data:") && !String(value).startsWith("<"))
    .map(([key, value]) => `${key}=${String(value)}`);
  const stage = cue.kind === "takeover" && cue.stage !== null ? [`stage=${String(cue.stage.slices?.length ?? cue.stage.planes.filter((p) => p !== "").length)}`] : [];
  return [cue.kind, ...facts, ...stage].join(" ");
}

const ALL: readonly FxDomCue[] = Object.values(CASES).flat();
const ESTIMATES = new Map<FxDomCue, LayerPart[]>();
function estimate(cue: FxDomCue): LayerPart[] {
  const known = ESTIMATES.get(cue);
  if (known !== undefined) return known;
  const parts = layersOf(cue);
  ESTIMATES.set(cue, parts);
  return parts;
}

describe("R4493000 R4493001 every DOM cue's layers on a phone (390x844 at 3x)", () => {
  it("R4493000 no element of any DOM cue covers more than FX_LAYER_MAX_SCREENS", () => {
    const problems = ALL.flatMap((cue) => layerProblems(nameOf(cue), estimate(cue), VIEW, DPR)).filter((p) => p.endsWith("FX_LAYER_MAX_SCREENS"));
    expect(problems).toEqual([]);
  });

  it("R4493001 every DOM cue's largest case at 390x844 and 3x stays within FX_PHONE_LAYER_BUDGET_MB", () => {
    const problems = ALL.flatMap((cue) => layerProblems(nameOf(cue), estimate(cue), VIEW, DPR)).filter((p) => p.endsWith("FX_PHONE_LAYER_BUDGET_MB"));
    expect(problems).toEqual([]);
  });

  it("R4493001 the director's tables never under-count a cue", () => {
    const wrong: string[] = [];
    // A takeover's count is its look's, its art's and its stage's: each case within rounding of its estimate.
    for (const cue of CASES.takeover) {
      const est = screensOf(estimate(cue));
      const counted = cueScreens(cue);
      if (!(counted >= est - 1e-9 && counted < est + 0.05)) wrong.push(`${nameOf(cue)}: counted ${counted.toFixed(3)}, estimated ${est.toFixed(3)}`);
    }
    // Every other kind's (and shape's) count is its largest case's, rounded up.
    const held = (label: string, table: number, cues: readonly FxDomCue[]): void => {
      const most = Math.max(...cues.map((cue) => screensOf(estimate(cue))));
      if (!(table >= most - 1e-9 && table < most + 0.05)) wrong.push(`${label}: counted ${table.toFixed(3)}, estimated ${most.toFixed(3)}`);
    };
    for (const shape of SHAPES) held(`setPiece ${shape}`, FX_SET_PIECE_LAYER_SCREENS[shape], CASES.setPiece.filter((cue) => cue.shape === shape));
    // A kind the board table counts there is held to its board cases there and to the rest elsewhere; every case counts as the director counts it.
    for (const kind of Object.keys(FX_CUE_LAYER_SCREENS) as (keyof typeof FX_CUE_LAYER_SCREENS)[]) {
      const cases: readonly FxDomCue[] = CASES[kind];
      const onBoard = (cue: FxDomCue): boolean => "at" in cue && cue.at === board;
      const there = FX_BOARD_CUE_LAYER_SCREENS[kind];
      if (there === undefined) {
        held(kind, FX_CUE_LAYER_SCREENS[kind], cases);
        continue;
      }
      held(`${kind} on the board`, there, cases.filter(onBoard));
      held(kind, FX_CUE_LAYER_SCREENS[kind], cases.filter((cue) => !onBoard(cue)));
      for (const cue of cases) expect(cueScreens(cue), nameOf(cue)).toBe(onBoard(cue) ? there : FX_CUE_LAYER_SCREENS[kind]);
    }
    // Only a kind that lands on the board has a board count.
    for (const kind of Object.keys(FX_BOARD_CUE_LAYER_SCREENS) as (keyof typeof FX_CUE_LAYER_SCREENS)[]) {
      expect(CASES[kind].some((cue) => "at" in cue && cue.at === board), kind).toBe(true);
    }
    expect(wrong).toEqual([]);
  });

  it("R4493001 a takeover the budget leaves artless and bare counts less, and the phone's merged stage draws FX_PHONE_STAGE_LAYERS pictures", () => {
    for (const look of LOOKS) {
      const [artless, plain, phone, dark] = takeoverCases(look).map((cue) => screensOf(estimate(cue)));
      expect(dark!, look).toBeLessThan(artless!);
      expect(artless!, look).toBeLessThanOrEqual(plain! + 1e-9);
      expect(plain!, look).toBeLessThan(phone!);
      // The darkest form draws no rays and no wave.
      expect(estimate(takeoverCases(look)[3]!).map((part) => part.label).filter((label) => /fx-takeover-(rays|wave)/.test(label)), look).toEqual([]);
    }
    const phone = takeoverCases("void")[2]!;
    expect(phone.stage?.slices).toHaveLength(FX_PHONE_STAGE_LAYERS);
    expect(estimate(phone).filter((part) => part.label.includes("fx-takeover-plane"))).toHaveLength(FX_PHONE_STAGE_LAYERS);
    // The form the budget's art step leaves is the artless case's: no art of its look and no picture.
    expect(screensOf(layersOf(takeoverForm(takeoverCases("freshJack")[2]!, TAKEOVER_ARTLESS)))).toBeLessThan(screensOf(estimate(takeoverCases("freshJack")[2]!)));
  });

  it("R4493001 a fixture cue too big fails", () => {
    const huge = parseSheet(".fx-huge{position:absolute;inset:-50%;animation:fx-huge 1s both}@keyframes fx-huge{to{transform:scale(3)}}");
    const layer = document.createElement("div");
    layer.className = "fx-dom";
    const el = document.createElement("div");
    el.className = "fx-huge";
    layer.appendChild(el);
    document.body.appendChild(layer);
    const parts = estimateLayers(huge, el, VIEW);
    // Twice the window each way at three times its size: 36 screens, past the element's cap and the cue's budget both.
    expect(parts).toHaveLength(1);
    expect(parts[0]?.screens).toBeCloseTo(36, 6);
    const problems = layerProblems("huge", parts, VIEW, DPR);
    expect(problems).toHaveLength(2);
    expect(problems[0]).toMatch(/FX_LAYER_MAX_SCREENS$/);
    expect(problems[1]).toMatch(/FX_PHONE_LAYER_BUDGET_MB$/);
  });

  it("R4493000 .fx-takeover-plane carries no will-change and no overscan past its largest dolly", () => {
    const plane = fx.rules.filter((rule) => rule.media === null && rule.selectors.some((s) => /^\.fx-takeover-plane(\[[^\]]+\])?$/.test(s.trim())));
    expect(plane.length).toBeGreaterThan(0);
    for (const rule of plane) {
      expect(rule.decls.has("will-change"), rule.selectors.join(", ")).toBe(false);
      const inset = rule.decls.get("inset");
      if (inset !== undefined) expect(inset, rule.selectors.join(", ")).toBe("0");
    }
    // Every plane of a mounted stage is the window itself, which its dolly only grows about its middle.
    for (const part of estimate(takeover("prismatic", { stage: PLANE_STAGE })).filter((p) => p.label.includes("fx-takeover-plane"))) {
      expect([part.width, part.height], part.label).toEqual([VIEW.width, VIEW.height]);
    }
    const bodies = keyframeBodies(fx);
    for (const plane of ART_PLANES) {
      const body = bodies.get(`fx-takeover-dolly-${plane}`) ?? "";
      const stops = [...body.matchAll(/\{([^{}]*)\}/g)].map((m) => parseDecls(m[1]!).get("transform") ?? "none");
      expect(stops.length, plane).toBeGreaterThan(0);
      let largest = 1;
      for (const transform of stops) {
        const read = /^(?:scale\(([\d.]+)\))?\s*(?:translate\((-?[\d.]+)vw,\s*(-?[\d.]+)vh\)|translate\(0,\s*0\))?$/.exec(transform.trim());
        expect(read, `${plane}: ${transform}`).not.toBeNull();
        const s = Number(read?.[1] ?? 1);
        const tx = Math.abs(Number(read?.[2] ?? 0)) / 100;
        const ty = Math.abs(Number(read?.[3] ?? 0)) / 100;
        largest = Math.max(largest, s);
        // A dolly only grows the plane, and never shifts it farther than it grew, so no edge comes into the window.
        expect(s, `${plane}: ${transform}`).toBeGreaterThanOrEqual(1);
        expect(s * tx, `${plane}: ${transform}`).toBeLessThanOrEqual((s - 1) / 2 + 1e-9);
        expect(s * ty, `${plane}: ${transform}`).toBeLessThanOrEqual((s - 1) / 2 + 1e-9);
      }
      // The director's dolly scales are the sheet's.
      expect(largest, plane).toBe(FX_TAKEOVER_DOLLY_SCALES[plane]);
    }
    // A diorama's slices fly toward the camera by their depth alone, the director's reach the sheet's.
    const fly = [...(bodies.get("fx-takeover-fly") ?? "").matchAll(/\{([^{}]*)\}/g)].map((m) => parseDecls(m[1]!).get("transform") ?? "");
    expect(fly).toEqual(["scale(1)", `scale(calc(1 + var(--fx-slice-depth, 0) * ${String(FX_TAKEOVER_FLY_REACH)}))`]);
  });
});
