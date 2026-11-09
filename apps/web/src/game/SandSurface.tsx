// A cosmetic, input-transparent fidget layer for the playmat. It reads neither rules nor hidden
// state: the board owns the cards above it, and this layer only records pointer marks where a
// press landed on neither a card nor a board control. #1954: one press is one tap, in the look and
// sound of the playmat's family; holding or dragging adds nothing. R3348000 (#3348): every tile
// answers a press too, legal or not, with a ripple in its half's playmat family, that family's
// sound and, on touch, a short haptic: it never stops or changes what the press does in the game.

import { useEffect, useRef, useState, type CSSProperties, type ReactElement, type RefObject } from "react";

import type { SfxId } from "../audio/types.ts";
import { getAudioEngine } from "../audio/engine.ts";
import type { Cosmetic } from "../cards/cosmetics.ts";
import { tapHaptic } from "../haptics/haptics.ts";
import { TILE_FEEL } from "./arena/config.ts";
import { arenaPlay } from "./arena/play.ts";

/** Tunables kept together so the surface can be retuned without changing input code. */
export const SAND_FEEL = {
  fadeMs: 8_000,
  maximumMarks: 180,
  settleMs: 650,
  maximumBuild: 6,
  baseGrains: 4,
  grainVariation: 3,
  grainSpreadPercent: 2,
  crunchVariants: 4,
} as const;

/** A playmat's tap family: playmats that feel alike share one mark style and one SFX recipe. */
export type TapFamily = "sand" | "wood" | "felt" | "stone" | "neon" | "parchment" | "astral";

/** A playmat id (the registry's `kind: "table"` ids, wired/generated/cosmetics.ts). */
export type TableId = Extract<Cosmetic, { kind: "table" }>["id"];

/** #1954: each playmat's tap family; an unknown or new playmat taps as sand. */
export const TAP_FAMILY: Readonly<Record<TableId, TapFamily>> = {
  "table-tavern": "sand",
  "table-desert-sand": "sand",
  "table-wood": "wood",
  "table-chessboard": "wood",
  "table-felt": "felt",
  "table-marble": "stone",
  "table-obsidian-slab": "stone",
  "table-neon-grid": "neon",
  "table-ancient-map": "parchment",
  "table-astral": "astral",
  // R3335001: the Fleet track's playmat, the neon grid recolored, taps as the grid does.
  "table-stinkyscape": "neon",
};

/** The family of `table` (Board's data-table); absent or unknown is the default, which taps as sand. */
export function tapFamilyOf(table: string | undefined): TapFamily {
  return table !== undefined && Object.hasOwn(TAP_FAMILY, table) ? TAP_FAMILY[table as TableId] : "sand";
}

/** #1954: each family's SFX recipe; the sand fallback keeps the issue's table total. */
export const TAP_SFX: Readonly<Record<TapFamily, SfxId>> = {
  sand: "sand",
  wood: "tapWood",
  felt: "tapFelt",
  stone: "tapStone",
  neon: "tapNeon",
  parchment: "tapParchment",
  astral: "tapAstral",
};

/** The feel values the tap marks draw from (board.css reads the tint and flight as CSS vars). */
export const TAP_FEEL = {
  neonColors: ["#22d3ee", "#e879f9", "#a3e635", "#facc15"],
  astralColors: ["#ffffff", "#c4b5fd", "#bae6fd"],
  astralStars: 5,
  astralSpreadPx: 16,
} as const;

export type SandMark = {
  id: number;
  x: number;
  y: number;
  family: TapFamily;
  grain: boolean;
  build: number;
  variation: number;
  /** Neon's ring color, or one astral star's color. */
  tint?: string;
  /** One astral star's flight, in px. */
  dx?: number;
  dy?: number;
  /** #185: a landing Unit's mark on the sand, sized to its zone (`slamSand`). */
  slam?: "cracks" | "crater";
  /** The zone's width, in % of the field, for a slam mark. */
  size?: number;
};

/** R3348000: one tile's answer to a press: a ripple around the pressed point, or round the card an occupied tile holds. Lengths are px in the field. */
export type TileAnswer = {
  id: number;
  /** The box that answers: the card's, or the empty tile's. */
  x: number;
  y: number;
  w: number;
  h: number;
  /** The ripple's center: the press on an empty tile, the card's center on an occupied one. */
  cx: number;
  cy: number;
  family: TapFamily;
  /** A legal tile answers harder. */
  strong: boolean;
  occupied: boolean;
  /** "still" under reduced motion or effects off: the same change with no motion. */
  play: "move" | "still";
  tint?: string;
};

/** #185: what a landing leaves in the sand. Cracks are never drawn under Reduce motion. */
export type SlamSand = { marks: readonly ("cracks" | "crater")[] };

/** The DOM event a landing zone dispatches; it bubbles to the field this surface listens on. */
export const SLAM_SAND_EVENT = "jk-slam-sand";

/** #2015: the field's center line, in % of its height; above it is the opponent's half (table.css's `.field-their-mat`, `inset: 0 0 50%`). */
const FIELD_MIDLINE_PERCENT = 50;

/** #185: dispatch a landing's sand marks at `zone` (the field's surface draws them under the cards). */
export function slamSand(zone: Element, sand: SlamSand): void {
  zone.dispatchEvent(new CustomEvent<SlamSand>(SLAM_SAND_EVENT, { bubbles: true, detail: sand }));
}

export type SandSurfaceProps = {
  field: RefObject<HTMLElement | null>;
  /** The action builder owns this signal: while it has a card lifted or targeted, sand is inert. */
  disabled: boolean;
  /** The worn playmat (Board's data-table). */
  table?: string;
  /** R1834004: the opponent's playmat (Board's data-their-table), under the upper half. */
  theirTable?: string;
};

function isEmptySurfaceTarget(target: EventTarget | null): target is Element {
  if (!(target instanceof Element)) return false;
  // #2332: every bit of the playmat taps, an occupied zone's margin included. Only a card, a board
  // control or a legal zone (a button: the press plays into it) keeps the press.
  return target.closest(".card, button, input, select, textarea, [role=button], .hero, .seat, .control-bar") === null;
}

function variationFor(x: number, y: number, sequence: number): number {
  const value = Math.abs(Math.sin(x * 13.17 + y * 7.31 + sequence * 0.91));
  return value;
}

function pick(from: readonly [string, ...string[]], at: number): string {
  return from[Math.floor(at * from.length) % from.length] ?? from[0];
}

/**
 * #1954: the mark set one press draws, in `family`'s style at (`x`, `y`). The hue and flight come
 * from the same deterministic `variationFor` as every other mark, so tests can check them.
 */
export function tapMarks(family: TapFamily, x: number, y: number, firstId: number, build: number): SandMark[] {
  let id = firstId - 1;
  const mark = (markX: number, markY: number, grain: boolean): SandMark => {
    id += 1;
    return { id, x: markX, y: markY, family, grain, build, variation: variationFor(markX, markY, id) };
  };
  const first = mark(x, y, false);
  if (family === "neon") {
    return [{ ...first, tint: pick(TAP_FEEL.neonColors, first.variation) }];
  }
  if (family === "astral") {
    return Array.from({ length: TAP_FEEL.astralStars }, (_, star) => {
      const angle = variationFor(x, y, first.id + star) * Math.PI * 2;
      const distance = TAP_FEEL.astralSpreadPx * (0.4 + 0.6 * variationFor(y, x, first.id + star + 1));
      return {
        ...(star === 0 ? first : mark(x, y, false)),
        tint: pick(TAP_FEEL.astralColors, variationFor(x + star, y, first.id)),
        dx: Math.cos(angle) * distance,
        dy: Math.sin(angle) * distance,
      };
    });
  }
  if (family === "sand") {
    const grains: SandMark[] = [];
    const grainCount = SAND_FEEL.baseGrains + Math.floor(first.variation * SAND_FEEL.grainVariation);
    for (let grain = 0; grain < grainCount; grain += 1) {
      const angle = variationFor(x, y, first.id + grain) * Math.PI * 2;
      const distance = SAND_FEEL.grainSpreadPercent * (0.35 + variationFor(y, x, first.id + grain + 1));
      grains.push(mark(x + Math.cos(angle) * distance, y + Math.sin(angle) * distance, true));
    }
    return [first, ...grains];
  }
  return [first];
}

/** The `.zone` a press landed in, inside `field`: its own element or anything it holds. */
function zoneOf(target: EventTarget | null, field: HTMLElement): HTMLElement | null {
  if (!(target instanceof Element)) return null;
  const zone = target.closest<HTMLElement>('.zone[data-testid^="zone-"]');
  return zone !== null && field.contains(zone) ? zone : null;
}

export default function SandSurface({ field, disabled, table, theirTable }: SandSurfaceProps): ReactElement {
  const [marks, setMarks] = useState<readonly SandMark[]>([]);
  const [tiles, setTiles] = useState<readonly TileAnswer[]>([]);
  const disabledRef = useRef(disabled);
  const tableRef = useRef(table);
  const theirTableRef = useRef(theirTable);
  const markId = useRef(0);
  const tileId = useRef(0);
  const tileTimers = useRef(new Set<number>());
  const lastTap = useRef<number | null>(null);
  const build = useRef(0);
  disabledRef.current = disabled;
  tableRef.current = table;
  theirTableRef.current = theirTable;

  useEffect(() => {
    const timers = tileTimers.current;
    return () => {
      for (const timer of timers) window.clearTimeout(timer);
      timers.clear();
    };
  }, []);

  useEffect(() => {
    const element = field.current;
    if (element === null) return undefined;

    /** The mat's tap (#1954) at `event`; true when it drew its marks and sounded. */
    const tapMat = (event: PointerEvent, bounds: DOMRect): boolean => {
      if (disabledRef.current || !isEmptySurfaceTarget(event.target)) return false;
      const x = ((event.clientX - bounds.left) / bounds.width) * 100;
      const y = ((event.clientY - bounds.top) / bounds.height) * 100;
      const now = performance.now();
      build.current = lastTap.current === null || now - lastTap.current > SAND_FEEL.settleMs
        ? 1
        : Math.min(SAND_FEEL.maximumBuild, build.current + 1);
      lastTap.current = now;
      // #2015: each half taps in its own playmat, the opponent's above the center line.
      const family = tapFamilyOf(y < FIELD_MIDLINE_PERCENT ? theirTableRef.current : tableRef.current);
      const added = tapMarks(family, x, y, markId.current + 1, build.current);
      markId.current += added.length;
      setMarks((previous) => [...previous, ...added].slice(-SAND_FEEL.maximumMarks));
      const variation = added[0]?.variation ?? 0;
      // A field tap has its own low-gain, polyphonic crunch path: every press sounds, and its
      // variant/build values make a rapid pile sound fuller.
      try {
        getAudioEngine().playSfx(TAP_SFX[family], {
          variation,
          sandVariant: Math.floor(variation * SAND_FEEL.crunchVariants),
          sandBuild: build.current,
        });
      } catch {
        // Web Audio can be unavailable before a gesture unlock; the cosmetic mark still appears.
      }
      return true;
    };

    /**
     * R3348000: the tile `zone` answers a press: a ripple in its half's playmat family, its sound
     * unless the mat's tap just made one, and on touch a haptic. It reads only what the zone already
     * draws (a legal flag, a card) and never stops the press, so a legal tile still plays and an
     * illegal one still does nothing in the game.
     */
    const answerTile = (event: PointerEvent, zone: HTMLElement, bounds: DOMRect, sounded: boolean): void => {
      const legal = zone.dataset.legal === "true";
      const card = zone.querySelector<HTMLElement>(".card");
      const box = (card ?? zone).getBoundingClientRect();
      const family = tapFamilyOf(zone.dataset.side === "opponent" ? theirTableRef.current : tableRef.current);
      // The answers' layer fills the field's padding box, which starts inside the field's border.
      const originX = bounds.left + element.clientLeft;
      const originY = bounds.top + element.clientTop;
      const x = box.left - originX;
      const y = box.top - originY;
      const cx = card === null ? event.clientX - originX : x + box.width / 2;
      const cy = card === null ? event.clientY - originY : y + box.height / 2;
      tileId.current += 1;
      const id = tileId.current;
      const variation = variationFor(cx, cy, id);
      const answer: TileAnswer = {
        id,
        x,
        y,
        w: box.width,
        h: box.height,
        cx,
        cy,
        family,
        strong: legal,
        occupied: card !== null,
        play: arenaPlay() === "move" ? "move" : "still",
        ...(family === "neon" ? { tint: pick(TAP_FEEL.neonColors, variation) } : {}),
      };
      setTiles((previous) => [...previous, answer].slice(-TILE_FEEL.maximum));
      const timer = window.setTimeout(() => {
        tileTimers.current.delete(timer);
        setTiles((previous) => previous.filter((tile) => tile.id !== id));
      }, TILE_FEEL.rippleMs);
      tileTimers.current.add(timer);
      if (!sounded) {
        try {
          getAudioEngine().playSfx(TAP_SFX[family], {
            variation,
            sandVariant: Math.floor(variation * SAND_FEEL.crunchVariants),
            sandBuild: legal ? TILE_FEEL.legalBuild : TILE_FEEL.baseBuild,
          });
        } catch {
          // Web Audio can be unavailable before a gesture unlock; the ripple still shows.
        }
      }
      if (event.pointerType === "touch") tapHaptic(legal ? "tileLegal" : "tile");
    };

    const down = (event: PointerEvent): void => {
      if (document.documentElement.hasAttribute("data-dragging")) return;
      const bounds = element.getBoundingClientRect();
      if (bounds.width === 0 || bounds.height === 0) return;
      const sounded = tapMat(event, bounds);
      const zone = zoneOf(event.target, element);
      if (zone !== null) answerTile(event, zone, bounds, sounded);
    };

    // #185: a slam's cracks and crater, centered on its zone and as wide as it, under the cards.
    const slam = (event: Event): void => {
      const detail = (event as CustomEvent<SlamSand>).detail;
      const zone = event.target;
      if (!(zone instanceof Element) || detail === undefined) return;
      const bounds = element.getBoundingClientRect();
      const box = zone.getBoundingClientRect();
      if (bounds.width === 0 || bounds.height === 0) return;
      const x = ((box.left + box.width / 2 - bounds.left) / bounds.width) * 100;
      const y = ((box.top + box.height / 2 - bounds.top) / bounds.height) * 100;
      const size = (box.width / bounds.width) * 100;
      const added = detail.marks.map((kind): SandMark => {
        markId.current += 1;
        return { id: markId.current, x, y, family: "sand", grain: false, build: 1, variation: variationFor(x, y, markId.current), slam: kind, size };
      });
      setMarks((previous) => [...previous, ...added].slice(-SAND_FEEL.maximumMarks));
    };
    element.addEventListener(SLAM_SAND_EVENT, slam);
    element.addEventListener("pointerdown", down);
    return () => {
      element.removeEventListener(SLAM_SAND_EVENT, slam);
      element.removeEventListener("pointerdown", down);
    };
  }, [field]);

  return (
    <>
      <span className="sand-surface" aria-hidden="true">
        {marks.map((mark) => (
          <span
            className={
              mark.slam !== undefined
                ? `sand-mark sand-mark--${mark.slam}`
                : mark.family === "sand"
                  ? mark.grain
                    ? "sand-mark sand-mark--grain"
                    : "sand-mark"
                  : `tap-mark tap-mark--${mark.family}`
            }
            key={mark.id}
            style={{
              "--sand-x": `${String(mark.x)}%`,
              "--sand-y": `${String(mark.y)}%`,
              "--sand-build": String(mark.build),
              "--sand-variation": String(mark.variation),
              "--sand-fade": `${String(SAND_FEEL.fadeMs)}ms`,
              ...(mark.size === undefined ? {} : { "--sand-zone": `${String(mark.size)}%` }),
              ...(mark.tint === undefined ? {} : { "--tap-tint": mark.tint }),
              ...(mark.dx === undefined ? {} : { "--tap-dx": `${String(mark.dx)}px` }),
              ...(mark.dy === undefined ? {} : { "--tap-dy": `${String(mark.dy)}px` }),
            } as CSSProperties}
          />
        ))}
      </span>
      {tiles.length > 0 && (
        <span className="tile-answers" aria-hidden="true">
          {tiles.map((tile) => (
            <span
              className="tile-answer"
              key={tile.id}
              data-family={tile.family}
              data-strength={tile.strong ? "strong" : "base"}
              data-occupied={tile.occupied ? "true" : undefined}
              data-play={tile.play}
              style={{
                "--tile-x": `${String(tile.x)}px`,
                "--tile-y": `${String(tile.y)}px`,
                "--tile-w": `${String(tile.w)}px`,
                "--tile-h": `${String(tile.h)}px`,
                "--tile-cx": `${String(tile.cx)}px`,
                "--tile-cy": `${String(tile.cy)}px`,
                ...(tile.tint === undefined ? {} : { "--tap-tint": tile.tint }),
              } as CSSProperties}
            />
          ))}
        </span>
      )}
    </>
  );
}
