// R3331000: a card's picture composed as a scene (ART.md, "Scene recipes and kits"). A scene recipe
// (sceneRecipes.ts) is data per card: a sky, a light, an emblem and layers from the back of the
// picture to its front (sky, far, mid, subject, near, light, particles), each a list of placements of
// kit parts (sceneKit.ts, figureKit.ts). `composeScene` draws the parts from one stream seeded by the
// card's id (or a picture's own seed, `core-002#alt`), so a placement's copies vary card by card and
// the Radiant face, which only repaints, keeps every shape. `withScene` lays a composed scene into the
// procedural painter's `ArtSpec`, so every painter that hands it a recipe card prints, moves, splits
// (Out of frame) and restyles it as it does any other picture. A card with no recipe never reaches
// here and draws its picture byte for byte. Presentation only (CLAUDE.md rule 7): the art reads a
// card's id, never anything hidden.

import { hashId, round2, seededRandom } from "./hash.ts";
import { drawFigure, type FigureLook, type PoseId, type FigurePartId } from "./figureKit.ts";
import type { ShowcaseFamily } from "./pictures.ts";
import type { ArtSpec, RidgeRole } from "./procedural.ts";
import { SCENE_KIT, type SceneryPartId } from "./sceneKit.ts";
import { SCENE_RECIPES } from "./sceneRecipes.ts";
import { mixHex, RADIANT_MIX, RADIANT_PALETTE } from "./themes.ts";

export type Rng = () => number;

export { SCENE_BOX } from "./sceneKit.ts";

/** A scene's layers, back to front. */
export type SceneLayer = "sky" | "far" | "mid" | "subject" | "near" | "light" | "particles";
export const SCENE_LAYERS: readonly SceneLayer[] = ["sky", "far", "mid", "subject", "near", "light", "particles"];

/** R2715001: the part of the picture each layer is, which a moving picture moves on its own beat. */
export const SCENE_LAYER_ROLES: Readonly<Record<SceneLayer, RidgeRole>> = {
  sky: "back",
  far: "back",
  mid: "back",
  subject: "subject",
  near: "over",
  light: "light",
  particles: "motes",
};

/** palette.css's colours by name, without the `--pal-` prefix: the only colours a kit paints with. */
export const SCENE_PALETTE = {
  black: "#000000",
  white: "#ffffff",
  "grey-500": "#6b7280",
  "stone-500": "#8a8577",
  "wood-950": "#0e0905",
  "wood-900": "#120b07",
  "wood-800": "#2b170c",
  "wood-700": "#5a341b",
  "wood-600": "#713f12",
  "brass-950": "#2b1800",
  "brass-800": "#4a3c1a",
  "brass-600": "#a86a1f",
  "brass-500": "#c9953f",
  "gold-400": "#f0c04a",
  "gold-300": "#ffd24a",
  "gold-200": "#fde68a",
  "yellow-400": "#facc15",
  parchment: "#efe0bf",
  "red-300": "#fca5a5",
  "red-400": "#f87171",
  "red-500": "#ef4444",
  "red-600": "#c0453f",
  "red-900": "#7f1d1d",
  "ember-500": "#ff5a36",
  "orange-400": "#fb923c",
  "lime-200": "#d9f99d",
  "lime-400": "#a3e635",
  "green-200": "#bbf7d0",
  "green-400": "#4ade80",
  "jade-400": "#4ec98a",
  "green-500": "#22c55e",
  "green-700": "#15803d",
  "green-900": "#14532d",
  "cyan-400": "#22d3ee",
  "sky-300": "#7dd3fc",
  "sky-400": "#38bdf8",
  "blue-200": "#bfdbfe",
  "blue-600": "#3d7bd4",
  "blue-700": "#1d4ed8",
  "blue-800": "#2f5d8c",
  "blue-900": "#1e3a8a",
  "slate-100": "#e8eaf2",
  "slate-400": "#9aa1b8",
  "slate-700": "#3f4964",
  "slate-800": "#2b3349",
  "slate-900": "#141a2e",
  "violet-300": "#c4b5fd",
  "violet-500": "#8b5cf6",
  "violet-700": "#6d28d9",
  "purple-400": "#c084fc",
} as const;

export type PaletteName = keyof typeof SCENE_PALETTE;

/** What a kit part is: six kinds of scenery and the five of a figure. */
export type KitCategory =
  | "terrain"
  | "architecture"
  | "plants"
  | "creatures"
  | "objects"
  | "weather"
  | "pose"
  | "head"
  | "hair"
  | "outfit"
  | "item";

/**
 * One filled shape of a part. A line is drawn as a thin band. `inked` shapes take their family's
 * outline (`SCENE_LINES`); a placement's `paint` repaints the `main` ones, a figure's look its `main`
 * (cloth) and `skin` ones.
 */
export type KitShape = {
  readonly d: string;
  readonly paint: PaletteName;
  readonly opacity: number;
  readonly inked?: boolean;
  readonly main?: boolean;
  readonly skin?: boolean;
};

/** Where a part is drawn: its anchor, its scale and which way it faces. */
export type PartPlace = { readonly x: number; readonly y: number; readonly s: number; readonly flip: 1 | -1 };

export type KitPart = { readonly category: KitCategory; draw(at: PartPlace, rng: Rng): readonly KitShape[] };

/**
 * A part in a layer: `count` copies, the first at x, y and each other offset by up to `spread`, every
 * copy's scale jittered by `SCENE_SCALE_JITTER`; `paint` repaints the part's `main` shapes.
 */
export type ScenePlacement = {
  readonly part: SceneryPartId;
  readonly x: number;
  readonly y: number;
  readonly scale?: number;
  readonly flip?: 1 | -1;
  readonly count?: number;
  readonly spread?: readonly [number, number];
  readonly paint?: PaletteName;
};

/** The figure a recipe's subject layer draws in front of its own parts, standing on x, y. */
export type FigureRecipe = {
  readonly x: number;
  readonly y: number;
  readonly scale: number;
  readonly flip?: 1 | -1;
  readonly pose: PoseId;
  readonly look: FigureLook;
};

export type SceneRecipe = {
  readonly family: ShowcaseFamily;
  /** The sky's gradient, top to bottom. */
  readonly sky: readonly [PaletteName, PaletteName];
  /** The glow behind every layer. */
  readonly light: { readonly x: number; readonly y: number; readonly r: number; readonly paint: PaletteName; readonly opacity: number };
  /** Where the card's emblem stands in the scene. */
  readonly emblem: { readonly x: number; readonly y: number; readonly size: number; readonly rotate: number };
  readonly figure?: FigureRecipe;
  readonly layers: Partial<Record<SceneLayer, readonly ScenePlacement[]>>;
};

/** A composed shape: the layer it lies in and the part it came from. */
export type SceneShape = KitShape & { readonly layer: SceneLayer; readonly part: SceneryPartId | FigurePartId };

/** Salts the scene's stream, so it shares no draw with the painter's own. */
export const SCENE_SALT = 0x5ce7e331;
/** What ends a card's id in a picture's own seed (`core-002#alt`, pictures.ts `artSeedOf`). */
export const SCENE_SEED_SEPARATOR = "#";
/** How far a Radiant face mixes each layer's paint toward its gold. */
export const SCENE_RADIANT_MIX = 0.5;
/** How much brighter a Radiant face's glow is, as the painter's is. */
export const SCENE_RADIANT_GLOW_BOOST = 0.1;
/** The fewest parts the kits hold, the figure kit's included. */
export const SCENE_KIT_MIN_PARTS = 60;
/** The fewest parts each kind of scenery holds. */
export const SCENE_KIT_MIN_PER_CATEGORY = 6;
/** How far each copy's scale strays from its placement's, either way. */
export const SCENE_SCALE_JITTER = 0.08;

/** Each layer's gold, which a Radiant face mixes its paint toward. */
export const SCENE_RADIANT_GOLD: Readonly<Record<SceneLayer, string>> = {
  sky: RADIANT_PALETTE.sky[0],
  far: RADIANT_PALETTE.ridges[0],
  mid: RADIANT_PALETTE.ridges[1],
  subject: RADIANT_PALETTE.emblem.fill,
  near: RADIANT_PALETTE.ridges[2],
  light: RADIANT_PALETTE.glow,
  particles: RADIANT_PALETTE.mote,
};

/** Each family's outline on its `inked` shapes (ART.md's art bible), or none. */
export const SCENE_LINES: Readonly<Record<ShowcaseFamily, { readonly paint: PaletteName; readonly width: number } | null>> = {
  core: null,
  classic: { paint: "slate-900", width: 0.5 },
  meditative: { paint: "slate-900", width: 0.9 },
  stinkyscape: { paint: "black", width: 0.4 },
  "training-grounds": { paint: "wood-950", width: 0.7 },
};

function between(rng: Rng, low: number, high: number): number {
  return low + (high - low) * rng();
}

/** The card's recipe, read by its id alone (a picture's own seed keeps the card's), or null. */
export function sceneRecipeOf(seedId: string): SceneRecipe | null {
  const cut = seedId.indexOf(SCENE_SEED_SEPARATOR);
  const id = cut < 0 ? seedId : seedId.slice(0, cut);
  return Object.hasOwn(SCENE_RECIPES, id) ? (SCENE_RECIPES[id] ?? null) : null;
}

/** Every shape of the scene, back to front, drawn from `seedId`'s scene stream; the same on both faces. */
export function composeScene(recipe: SceneRecipe, seedId: string): SceneShape[] {
  const rng = seededRandom((hashId(seedId) ^ SCENE_SALT) >>> 0);
  const out: SceneShape[] = [];
  for (const layer of SCENE_LAYERS) {
    for (const placed of recipe.layers[layer] ?? []) {
      const kit = SCENE_KIT[placed.part];
      const copies = placed.count ?? 1;
      const [dx, dy] = placed.spread ?? [0, 0];
      for (let copy = 0; copy < copies; copy += 1) {
        const x = copy === 0 ? placed.x : placed.x + between(rng, -dx, dx);
        const y = copy === 0 ? placed.y : placed.y + between(rng, -dy, dy);
        const s = (placed.scale ?? 1) * (1 + between(rng, -SCENE_SCALE_JITTER, SCENE_SCALE_JITTER));
        for (const shape of kit.draw({ x, y, s, flip: placed.flip ?? 1 }, rng)) {
          const paint = shape.main === true && placed.paint !== undefined ? placed.paint : shape.paint;
          out.push({ ...shape, paint, layer, part: placed.part });
        }
      }
    }
    // The figure stands in front of the subject layer's own parts (Ceaseless Void's void behind it).
    if (layer === "subject" && recipe.figure !== undefined) {
      for (const shape of drawFigure(recipe.figure, rng)) out.push({ ...shape, layer });
    }
  }
  return out;
}

/** How many layers and how many distinct parts a scene draws (R3331001's measure). */
export function sceneUse(shapes: readonly SceneShape[]): { layers: number; parts: number } {
  return { layers: new Set(shapes.map((shape) => shape.layer)).size, parts: new Set(shapes.map((shape) => shape.part)).size };
}

/**
 * `spec` with its picture replaced by the recipe's scene: its ridges are the scene's shapes, its sky,
 * glow and emblem's place the recipe's, and the painter's accent, motes and motif glyphs gone. Its
 * theme, layout, backdrop, sky scheme, emblem glyph, motif and rays stay, so R503's variety and the
 * Radiant rays read as before. A Radiant face mixes each shape toward its layer's gold.
 */
export function withScene(spec: ArtSpec, recipe: SceneRecipe, seedId: string, radiant: boolean): ArtSpec {
  const shapes = composeScene(recipe, seedId);
  const line = SCENE_LINES[recipe.family];
  const paint = (name: PaletteName, layer: SceneLayer): string =>
    radiant ? mixHex(SCENE_PALETTE[name], SCENE_RADIANT_GOLD[layer], SCENE_RADIANT_MIX) : SCENE_PALETTE[name];
  const ridges: ArtSpec["ridges"][number][] = shapes.map((shape) => {
    const ridge = { d: shape.d, fill: paint(shape.paint, shape.layer), opacity: round2(shape.opacity), role: SCENE_LAYER_ROLES[shape.layer] };
    return shape.inked === true && line !== null ? { ...ridge, stroke: SCENE_PALETTE[line.paint], strokeWidth: line.width } : ridge;
  });
  const subjectFrom = shapes.findIndex((shape) => shape.layer === "subject");
  const subjectTo = subjectFrom < 0 ? 0 : subjectFrom + shapes.filter((shape) => shape.layer === "subject").length;
  const [skyTop, skyBottom] = recipe.sky;
  const { light, emblem } = recipe;
  return {
    ...spec,
    sky: {
      from: radiant ? mixHex(SCENE_PALETTE[skyTop], RADIANT_PALETTE.sky[0], RADIANT_MIX) : SCENE_PALETTE[skyTop],
      to: radiant ? mixHex(SCENE_PALETTE[skyBottom], RADIANT_PALETTE.sky[1], RADIANT_MIX) : SCENE_PALETTE[skyBottom],
      angle: spec.sky.angle,
    },
    glow: {
      cx: light.x,
      cy: light.y,
      r: light.r,
      color: radiant ? RADIANT_PALETTE.glow : SCENE_PALETTE[light.paint],
      opacity: round2(radiant ? Math.min(1, light.opacity + SCENE_RADIANT_GLOW_BOOST) : light.opacity),
    },
    ridges,
    emblem: { ...spec.emblem, x: emblem.x, y: emblem.y, size: emblem.size, rotate: emblem.rotate },
    accent: null,
    motes: [],
    motifGlyphs: [],
    subjectRidges: subjectFrom < 0 ? [0, 0] : [subjectFrom, subjectTo],
  };
}
