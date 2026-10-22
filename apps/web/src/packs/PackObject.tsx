// The pack as a physical object (#1663, part of #1651, SPEC §14.3, §14.6): a foil wrapper with a
// crimped seal at each end, the set's emblem and colours, a sheen that follows the pointer, a slight
// tilt toward it, a press, and a tear that bursts it open. The store's shelf and the pack-opening
// scene draw the same object, at `shelf` and `scene` size.
//
// PRESENTATION ONLY (CLAUDE.md rule 7). `state` is the caller's: this draws a pack pressed, tearing
// or open and decides nothing about when. Every animation moves transform and opacity alone
// (`pack-object.css`). REDUCED MOTION (the panel's setting or the system's, read as `PackScene`
// reads them) holds the tilt, the sheen and the tear still and sets `data-motion=reduce`.

import { useRef, type CSSProperties, type PointerEvent, type ReactElement, type ReactNode } from "react";

import { setMarkOf } from "../cards/setMark.ts";
import { useFxSettings } from "../fx/settings.ts";
import { reducedMotionNow } from "../game/animations.ts";
import { useSetting } from "../settings/store.ts";
import { COSMETIC_PACK, PACK_TEAR_MS, PACK_TILT_DEG, packName } from "./packs.ts";
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

const SVG_NS = "http://www.w3.org/2000/svg";
const INK = "#1d1530";

function dataUri(body: string): string {
  return `data:image/svg+xml,${encodeURIComponent(`<svg xmlns='${SVG_NS}' viewBox='0 0 24 24'>${body}</svg>`)}`;
}

/** The two emblems `setMark.ts` has no glyph for: a ringed globe and a four-point star, drawn as it draws its own. */
const OWN_EMBLEMS: Readonly<Record<string, string>> = {
  global: dataUri(
    `<circle cx='12' cy='12' r='9.6' fill='#bba8dc' stroke='${INK}' stroke-width='1.6'/>` +
      `<ellipse cx='12' cy='12' rx='4.2' ry='9.6' fill='none' stroke='${INK}' stroke-width='1.2'/>` +
      `<path d='M2.6 12H21.4M4.4 7.2H19.6M4.4 16.8H19.6' stroke='${INK}' stroke-width='1.2' fill='none'/>`,
  ),
  [COSMETIC_PACK]: dataUri(
    `<path d='M12 1.4L14.6 9.4L22.6 12L14.6 14.6L12 22.6L9.4 14.6L1.4 12L9.4 9.4Z' fill='#ff8ad0' stroke='#2a0f21' stroke-width='1.4' stroke-linejoin='round'/>`,
  ),
};

/** The emblem's `src`: the pack's own, else the set's mark (a set this client has no glyph for gets the diamond). */
function emblemOf(pack: string): string {
  return OWN_EMBLEMS[pack] ?? setMarkOf(pack).src;
}

const clamp01 = (value: number): number => Math.min(1, Math.max(0, value));

export default function PackObject({ pack, size, state = "idle", testid, children }: PackObjectProps): ReactElement {
  const [fx] = useFxSettings();
  const panelReduces = useSetting("reduceMotion");
  const reduced = panelReduces || reducedMotionNow(fx);
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
