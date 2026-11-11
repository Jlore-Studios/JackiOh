// The pack as a physical object (#1663, part of #1651, SPEC §14.3, §14.6) for the shelf and the
// pack-opening scene. PRESENTATION ONLY (CLAUDE.md rule 7): `state` is the caller's, and this decides
// nothing about when. Every animation moves transform and opacity alone (`pack-object.css`). REDUCED
// MOTION holds the tilt, the sheen and the tear still and sets `data-motion=reduce`.

import { useRef, type CSSProperties, type PointerEvent, type ReactElement, type ReactNode } from "react";

import { glyphUri, setMarkOf } from "../cards/setMark.ts";
import { useReducedMotion } from "../economy/FillBar.tsx";
import {
  COSMETIC_PACK,
  DUNGEON_BOSS_PACK,
  DUNGEON_ENEMY_PACK,
  DUNGEON_FILTER_PACK,
  DUNGEON_RANDOM_PACK,
  PACK_TEAR_MS,
  PACK_TILT_DEG,
  packName,
} from "./packs.ts";
import "./pack-object.css";

export type PackObjectState = "idle" | "pressed" | "tearing" | "open";

export type PackObjectProps = {
  /** A shipped set's name, `global` or `cosmetic`. */
  pack: string;
  size: "shelf" | "scene";
  state?: PackObjectState;
  testid?: string;
  /** Drawn on the face under the name: a hint, a count. */
  children?: ReactNode;
};

const INK = "#1d1530";

/** The two emblems `setMark.ts` has no glyph for: a ringed globe and a four-point star, drawn as it draws its own. */
const OWN_EMBLEMS: Readonly<Record<string, string>> = {
  global: glyphUri(
    `<circle cx='12' cy='12' r='9.6' fill='#bba8dc' stroke='${INK}' stroke-width='1.6'/>` +
      `<ellipse cx='12' cy='12' rx='4.2' ry='9.6' fill='none' stroke='${INK}' stroke-width='1.2'/>` +
      `<path d='M2.6 12H21.4M4.4 7.2H19.6M4.4 16.8H19.6' stroke='${INK}' stroke-width='1.2' fill='none'/>`,
  ),
  [COSMETIC_PACK]: glyphUri(
    `<path d='M12 1.4L14.6 9.4L22.6 12L14.6 14.6L12 22.6L9.4 14.6L1.4 12L9.4 9.4Z' fill='#ff8ad0' stroke='#2a0f21' stroke-width='1.4' stroke-linejoin='round'/>`,
  ),
  /** The dungeon run's reward packs (#3588): crossed swords for the beaten enemy's deck. */
  [DUNGEON_ENEMY_PACK]: glyphUri(
    `<path d='M6 18L18 6M18 18L6 6' stroke='${INK}' stroke-width='3.4' stroke-linecap='round'/>` +
      `<path d='M6 18L18 6M18 18L6 6' stroke='#dfe6f2' stroke-width='1.7' stroke-linecap='round'/>` +
      `<path d='M3.4 16.6L8.6 18.4M15.4 18.4L20.6 16.6' stroke='${INK}' stroke-width='1.6' stroke-linecap='round'/>` +
      `<circle cx='6' cy='20' r='1.6' fill='#e6b35c' stroke='${INK}' stroke-width='1.1'/>` +
      `<circle cx='18' cy='20' r='1.6' fill='#e6b35c' stroke='${INK}' stroke-width='1.1'/>`,
  ),
  /** A horned crown for the boss's deck. */
  [DUNGEON_BOSS_PACK]: glyphUri(
    `<path d='M5 12C3 9.5 3 6.5 4.5 4C5.5 6.5 7 8.5 9 9.5L5 12Z' fill='#f2f4fa' stroke='${INK}' stroke-width='1.2' stroke-linejoin='round'/>` +
      `<path d='M19 12C21 9.5 21 6.5 19.5 4C18.5 6.5 17 8.5 15 9.5L19 12Z' fill='#f2f4fa' stroke='${INK}' stroke-width='1.2' stroke-linejoin='round'/>` +
      `<path d='M5 20V12L8.5 15.5L12 8.5L15.5 15.5L19 12V20Z' fill='#e6b35c' stroke='${INK}' stroke-width='1.4' stroke-linejoin='round'/>` +
      `<circle cx='12' cy='16.3' r='1.6' fill='#c8322b' stroke='${INK}' stroke-width='1'/>`,
  ),
  /** A die showing three for the random pack. */
  [DUNGEON_RANDOM_PACK]: glyphUri(
    `<rect x='4.5' y='4.5' width='15' height='15' rx='3' fill='#f2f4fa' stroke='${INK}' stroke-width='1.6'/>` +
      `<circle cx='9' cy='9' r='1.9' fill='${INK}'/>` +
      `<circle cx='12' cy='12' r='1.9' fill='#c8322b'/>` +
      `<circle cx='15' cy='15' r='1.9' fill='${INK}'/>`,
  ),
  /** A tag for the type, tag or rarity pack. */
  [DUNGEON_FILTER_PACK]: glyphUri(
    `<path d='M4 4H12.5L20 11.5L11.5 20L4 12.5Z' fill='#9fb4e8' stroke='${INK}' stroke-width='1.4' stroke-linejoin='round'/>` +
      `<circle cx='8.5' cy='8.5' r='1.8' fill='#f2f4fa' stroke='${INK}' stroke-width='1.2'/>`,
  ),
};

/** The emblem's `src`: the pack's own, else the set's mark (a set this client has no glyph for gets the diamond). */
function emblemOf(pack: string): string {
  return OWN_EMBLEMS[pack] ?? setMarkOf(pack).src;
}

const clamp01 = (value: number): number => Math.min(1, Math.max(0, value));

export default function PackObject({ pack, size, state = "idle", testid, children }: PackObjectProps): ReactElement {
  const reduced = useReducedMotion();
  const root = useRef<HTMLSpanElement | null>(null);

  // The pointer sets two custom properties and the stylesheet moves the sheen and the tilt by
  // transform from them; nothing re-renders for a move.
  const follow = (event: PointerEvent<HTMLSpanElement>): void => {
    if (reduced) return;
    const box = event.currentTarget.getBoundingClientRect();
    if (!(box.width > 0) || !(box.height > 0)) return;
    root.current?.style.setProperty("--pack-px", String(clamp01((event.clientX - box.left) / box.width)));
    root.current?.style.setProperty("--pack-py", String(clamp01((event.clientY - box.top) / box.height)));
  };

  const rest = (): void => {
    root.current?.style.removeProperty("--pack-px");
    root.current?.style.removeProperty("--pack-py");
  };

  const part = (name: string): string | undefined => (testid === undefined ? undefined : `${testid}-${name}`);
  const style = { "--pack-tear-ms": PACK_TEAR_MS, "--pack-tilt-deg": PACK_TILT_DEG } as CSSProperties;

  return (
    <span
      ref={root}
      className="pack-object"
      data-testid={testid}
      data-pack={pack}
      data-size={size}
      data-state={state}
      data-motion={reduced ? "reduce" : "full"}
      style={style}
      onPointerMove={follow}
      onPointerLeave={rest}
      onPointerCancel={rest}
    >
      <span className="pack-object__tilt">
        <span className="pack-object__wrap">
          <span className="pack-object__face">
            <img className="pack-object__emblem" src={emblemOf(pack)} alt="" aria-hidden="true" data-testid={part("emblem")} />
            <span className="pack-object__name" data-testid={part("name")}>
              {packName(pack)}
            </span>
            {children}
          </span>
          <span className="pack-object__seal pack-object__seal--bottom" aria-hidden="true" />
          <span className="pack-object__seal pack-object__seal--top" aria-hidden="true" />
          <span className="pack-object__sheen" aria-hidden="true" />
          <span className="pack-object__flash" aria-hidden="true" />
        </span>
      </span>
    </span>
  );
}
