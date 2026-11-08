// R3342000: tile aura planner and pure color/mote math (issue #3342).
// Every Legendary and Mythic permanent in play marks its tile with an aura
// in its own picture's colors and family pattern (CLAUDE.md rule 7, R202).

import type { CardDef, Flavor, PlayerView, Row } from "@jackioh/shared";

import { hashId } from "../../cards/art/hash.ts";
import { motifFor } from "../../cards/art/motifs.ts";
import { showcaseFamilyOf, type ShowcaseFamily } from "../../cards/art/pictures.ts";
import { artSpec } from "../../cards/art/procedural.ts";
import { FAMILY_PAINTS as STINKY_PAINTS, isStinkyscapeArt, stinkySubject } from "../../cards/art/stinkyscape.ts";
import { compositionFor, themeFor, THEME_MOTES, turnHue, type MoteShape } from "../../cards/art/themes.ts";
import { FAMILY_PAINTS as YARD_PAINTS, isTrainingGroundsArt, LEGACY_PAINT, snapshotArtOf, yardSubject } from "../../cards/art/traininggrounds.ts";
import { LANES, laneIndex, sideView, testid, type AnimatingMap, type Highlight, type Side } from "../../game/contract.ts";
import { CATALOG } from "../../wire/cards.ts";
import type { PublicBackrowView } from "../../wire/generated/PublicBackrowView.ts";
import type { UnitView } from "../../wire/generated/UnitView.ts";
import { entryAura, FX_CARD_TABLE, type FxCardEntry } from "../cards/tables.ts";
import { capacityFor } from "../director.ts";
import { createRng } from "../rng.ts";
import {
  TILE_HUE_SPREAD_DEG,
  TILE_MOTES_LEGENDARY,
  TILE_MOTES_MYTHIC,
  TILE_MOTE_TRAVEL_PX,
  TILE_PARTICLE_SHARE,
  TILE_VARIANTS,
} from "./config.ts";
import { auraLookSpec, type AuraLook } from "./looks.ts";

export type TileTier = "Legendary" | "Mythic";

export type TilePattern = "circuitry" | "runes" | "ink" | "starfield" | "roots";

export const TILE_PATTERN_OF_FAMILY: Readonly<Record<ShowcaseFamily, TilePattern>> = {
  core: "circuitry",
  classic: "runes",
  meditative: "ink",
  stinkyscape: "starfield",
  "training-grounds": "roots",
};

export const TILE_LOOK_OF_FLAVOR: Readonly<Record<Flavor, AuraLook>> = {
  Physical: "storm",
  Brute: "void",
  Beast: "ember",
  Fire: "ember",
  Dark: "void",
  Holy: "holy",
  Nerd: "arcane",
  Tech: "storm",
  Nature: "bloom",
  Toxic: "toxic",
  Snack: "bloom",
  Hustle: "holy",
  Clout: "arcane",
  Spirit: "frost",
  Chaos: "arcane",
};

export const TILE_PATTERN_SHAPES: Readonly<Record<TilePattern, readonly string[]>> = {
  runes: [
    "M 50 15 A 35 35 0 0 1 85 50",
    "M 85 50 A 35 35 0 0 1 50 85",
    "M 50 85 A 35 35 0 0 1 15 50",
    "M 15 50 A 35 35 0 0 1 50 15",
    "M 50 25 L 50 40 M 40 32 L 60 32",
  ],
  starfield: [
    "M 25 20 L 25 30 M 20 25 L 30 25",
    "M 75 35 L 75 45 M 70 40 L 80 40",
    "M 35 70 L 35 80 M 30 75 L 40 75",
    "M 70 75 L 70 85 M 65 80 L 75 80",
    "M 50 48 A 2 2 0 1 0 50 52 A 2 2 0 1 0 50 48",
  ],
  roots: [
    "M 50 100 Q 45 70 30 50 Q 20 35 15 20",
    "M 50 100 Q 55 70 70 50 Q 80 35 85 20",
    "M 40 65 Q 40 45 50 30 Q 55 20 50 10",
    "M 60 65 Q 65 50 60 35",
  ],
  circuitry: [
    "M 10 20 H 40 V 50 H 60",
    "M 90 20 H 70 V 40",
    "M 20 80 H 50 V 60 H 80",
    "M 80 60 V 85 H 90",
    "M 45 45 H 55 V 55 H 45 Z",
  ],
  ink: [
    "M 20 30 Q 50 15 80 35 Q 50 25 20 30",
    "M 25 70 Q 55 85 85 65 Q 55 75 25 70",
    "M 48 45 C 45 50 42 55 45 60 C 48 65 54 65 57 60 C 60 55 57 50 54 45 Z",
    "M 30 50 Q 40 40 50 50 T 70 50",
  ],
};

const HEX_MAX = 255;
const HUE_TURN = 360;
const HUE_SEXTANT = 60;

export function hueOf(hex: string): number {
  const clean = hex.startsWith("#") ? hex.slice(1) : hex;
  const [r, g, b] =
    clean.length === 3
      ? [
          parseInt(clean[0]! + clean[0]!, 16) / HEX_MAX,
          parseInt(clean[1]! + clean[1]!, 16) / HEX_MAX,
          parseInt(clean[2]! + clean[2]!, 16) / HEX_MAX,
        ]
      : [
          parseInt(clean.slice(0, 2), 16) / HEX_MAX,
          parseInt(clean.slice(2, 4), 16) / HEX_MAX,
          parseInt(clean.slice(4, 6), 16) / HEX_MAX,
        ];
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const chroma = max - min;
  if (chroma === 0) return 0;
  let hue: number;
  if (max === r) hue = ((g - b) / chroma) % 6;
  else if (max === g) hue = (b - r) / chroma + 2;
  else hue = (r - g) / chroma + 4;
  return (((hue * HUE_SEXTANT) % HUE_TURN) + HUE_TURN) % HUE_TURN;
}

export function tileHueShift(defId: string): number {
  const hash = hashId(defId);
  const span = TILE_HUE_SPREAD_DEG * 2 + 1;
  return (hash % span) - TILE_HUE_SPREAD_DEG;
}

export function tierOf(def: Pick<CardDef, "rarity" | "printedRarity">): TileTier | null {
  const rarity = def.printedRarity ?? def.rarity;
  return rarity === "Legendary" || rarity === "Mythic" ? rarity : null;
}

const pictureColorsCache = new Map<string, readonly [string, string, string]>();

export function pictureColors(
  defId: string,
  def: CardDef,
  radiant: boolean = false,
): readonly [string, string, string] {
  const cacheKey = `${defId}:${radiant ? "r" : "b"}`;
  const cached = pictureColorsCache.get(cacheKey);
  if (cached !== undefined) return cached;

  let colors: readonly [string, string, string];
  if (isStinkyscapeArt(defId)) {
    const subject = stinkySubject(defId, def.name, def.tags, def.type);
    const paint = STINKY_PAINTS[subject.family];
    colors = [paint.light, paint.nebula[0], paint.glow];
  } else if (snapshotArtOf(defId) !== null || isTrainingGroundsArt(defId)) {
    const paint = snapshotArtOf(defId) !== null
      ? LEGACY_PAINT
      : YARD_PAINTS[yardSubject(defId, def.name, def.tags, def.type).family];
    colors = [paint.light, paint.body, paint.glow];
  } else {
    const theme = themeFor(def.tags, def.type);
    const motif = motifFor(def.name, theme);
    const spec = artSpec(defId, theme, compositionFor(def.type), radiant, motif);
    colors = [spec.emblem.fill, spec.ridges[0]?.fill ?? spec.sky.from, spec.glow.color];
  }

  pictureColorsCache.set(cacheKey, colors);
  return colors;
}

export type TileAuraSpec = {
  key: string;
  defId: string;
  instanceId: string;
  side: Side;
  row: Row;
  lane: number;
  tier: TileTier;
  colors: readonly [string, string, string];
  look: AuraLook;
  moteShape: MoteShape;
  pattern: TilePattern | null;
  variant: number;
  dim: boolean;
  hue: number;
};

export function planTileAura(
  view: PlayerView,
  side: Side,
  row: Row,
  lane: number,
  table: ReadonlyMap<string, FxCardEntry> = FX_CARD_TABLE,
): TileAuraSpec | null {
  const seat = sideView(view, side);
  const card = row === "units" ? seat.units[laneIndex(lane)] : seat.backrow[laneIndex(lane)];
  if (card === null || card === undefined) return null;

  // Face-down or unrevealed cards wear no tile aura (R202).
  if ("faceDown" in card && card.faceDown) return null;
  if ("unrevealed" in card && (card as { unrevealed?: boolean }).unrevealed === true) return null;

  const live = card as UnitView | PublicBackrowView;
  if (live.defId === "hidden") return null;

  const def = CATALOG[live.defId] ?? view.defs?.[live.defId];
  if (def === undefined) return null;

  const tier = tierOf(def);
  if (tier === null) return null;

  // Signature aura replaces the default (R3389000: a signature bespoke's too).
  if (entryAura(table.get(live.defId)) !== undefined) return null;

  const radiant = live.radiant ?? false;
  const rawColors = pictureColors(live.defId, def, radiant);
  const shift = tileHueShift(live.defId);
  const colors: readonly [string, string, string] = [
    turnHue(rawColors[0], shift),
    turnHue(rawColors[1], shift),
    turnHue(rawColors[2], shift),
  ];

  const look = (def.flavor ? TILE_LOOK_OF_FLAVOR[def.flavor] : undefined) ?? "arcane";
  const moteShape = THEME_MOTES[themeFor(def.tags, def.type)];
  const family = showcaseFamilyOf(def.set);
  const pattern = tier === "Mythic" ? (family !== null ? TILE_PATTERN_OF_FAMILY[family] : "runes") : null;
  const variant = hashId(`${live.defId}:tile`) % TILE_VARIANTS;
  const hue = hueOf(colors[1]);

  const dim =
    row === "units"
      ? (live as UnitView).slumber !== undefined ||
        (live as UnitView).frozen === true ||
        (live as UnitView).frozenForever === true ||
        (live as UnitView).vanilla === true
      : ((live as PublicBackrowView).unit !== undefined &&
          ((live as PublicBackrowView).unit!.slumber !== undefined ||
            (live as PublicBackrowView).unit!.frozen === true ||
            (live as PublicBackrowView).unit!.frozenForever === true ||
            (live as PublicBackrowView).unit!.vanilla === true)) ||
        (live as { vanilla?: boolean }).vanilla === true;

  const key = `tile-${side}-${row}-${lane}-${live.instanceId}`;
  return {
    key,
    defId: live.defId,
    instanceId: live.instanceId,
    side,
    row,
    lane,
    tier,
    colors,
    look,
    moteShape,
    pattern,
    variant,
    dim,
    hue,
  };
}

const SIDES: readonly Side[] = ["you", "opponent"];
const ROWS: readonly Row[] = ["units", "backrow"];

export function planTileAuras(
  view: PlayerView,
  table: ReadonlyMap<string, FxCardEntry> = FX_CARD_TABLE,
): TileAuraSpec[] {
  const out: TileAuraSpec[] = [];
  for (const side of SIDES) {
    for (const row of ROWS) {
      for (const lane of LANES) {
        const spec = planTileAura(view, side, row, lane, table);
        if (spec !== null) out.push(spec);
      }
    }
  }
  return out;
}

export function tileTestid(side: Side, row: Row, lane: number): string {
  return `tile-aura-${side}-${row}-${lane}`;
}

export function tileResponse(
  spec: TileAuraSpec,
  animating?: AnimatingMap,
  highlight?: Highlight | { selected?: string | ReadonlySet<string>; legal?: ReadonlySet<string> },
): { fading: boolean; hit: boolean; lit: boolean } {
  const cardTestid = testid.card(spec.instanceId);
  const zoneTestid = testid.zone(spec.side, spec.row, spec.lane);
  const anim = animating?.get(cardTestid);
  const fading = anim === "destroyed" || anim === "exiled";
  const hit = anim === "damage" || anim === "divineShieldLost";
  const isSelected =
    highlight?.selected !== undefined &&
    (typeof (highlight.selected as unknown as { has?: unknown }).has === "function"
      ? (highlight.selected as ReadonlySet<string>).has(cardTestid) || (highlight.selected as ReadonlySet<string>).has(zoneTestid)
      : (highlight.selected as unknown) === cardTestid || (highlight.selected as unknown) === zoneTestid);
  return { fading, hit, lit: Boolean(isSelected) };
}

export function tileMoteBudget(
  tier: TileTier,
  tiles: number,
  intensity: number,
  viewportWidth: number,
): number {
  const base = tier === "Legendary" ? TILE_MOTES_LEGENDARY : TILE_MOTES_MYTHIC;
  return Math.max(
    0,
    Math.min(
      Math.floor(base * intensity),
      Math.floor((TILE_PARTICLE_SHARE * capacityFor(viewportWidth)) / Math.max(1, tiles)),
    ),
  );
}

export type TileMote = {
  x: number;
  y: number;
  dx: number;
  dy: number;
  size: number;
  delayMs: number;
  durMs: number;
};

export function tileMotes(spec: TileAuraSpec, count: number): readonly TileMote[] {
  if (count <= 0) return [];
  const rng = createRng(hashId(spec.defId));
  const look = auraLookSpec(spec.look);
  const out: TileMote[] = [];
  for (let i = 0; i < count; i += 1) {
    const x = Math.round(rng() * 100);
    const y = Math.round(rng() * 100);
    const durMs = Math.round(look.life[0] + rng() * (look.life[1] - look.life[0]));
    const lifeSec = durMs / 1000;
    const vx = look.vx[0] + rng() * (look.vx[1] - look.vx[0]);
    const vy = look.vy[0] + rng() * (look.vy[1] - look.vy[0]);
    const rawDx = vx * lifeSec;
    const rawDy = vy * lifeSec;
    const dx = Math.max(-TILE_MOTE_TRAVEL_PX, Math.min(TILE_MOTE_TRAVEL_PX, Math.round(rawDx)));
    const dy = Math.max(-TILE_MOTE_TRAVEL_PX, Math.min(TILE_MOTE_TRAVEL_PX, Math.round(rawDy)));
    const size = Math.round((look.size[0] + rng() * (look.size[1] - look.size[0])) * 10) / 10;
    const delayMs = Math.round(rng() * durMs);
    out.push({ x, y, dx, dy, size, delayMs, durMs });
  }
  return out;
}
