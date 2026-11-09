// R3342000: tile aura component (issue #3342).
// Renders an aura under a Legendary or Mythic permanent, responding to touches,
// damage and targets, respecting reduced motion and intensity settings.

import {
  useEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
  type CSSProperties,
  type ReactElement,
} from "react";

import type { PlayerView, Row } from "@jackioh/shared";

import { reducedMotionNow } from "../../game/animations.ts";
import type { AnimatingMap, Highlight, Side } from "../../game/contract.ts";
import { useSetting } from "../../settings/store.ts";
import { getAudioEngine } from "../../audio/engine.ts";
import { touchHaptic } from "../../haptics/haptics.ts";
import { FX_INTENSITY_SCALE } from "../constants.ts";
import { useFxSettings } from "../settings.ts";
import { shedAuraScale, shedLevel, subscribeShed } from "../shed.ts";
import {
  TILE_EDGE,
  TILE_FADE_OUT_MS,
  TILE_FLICKER_MS,
  TILE_GLOW_PX,
  TILE_LIT_BOOST,
  TILE_PATTERN_OPACITY,
  TILE_PULSE_MS,
  TILE_RIPPLE_MS,
  TILE_TOUCH_GAP_MS,
} from "./config.ts";
import {
  planTileAura,
  planTileAuras,
  tileMoteBudget,
  tileMotes,
  tileResponse,
  tileTestid,
  TILE_PATTERN_SHAPES,
} from "./tile.ts";
import "./tile.css";

export type TileAuraProps = {
  view: PlayerView;
  side: Side;
  row: Row;
  lane: number;
  highlight?: Highlight;
  animating?: AnimatingMap;
};

export function TileAura({
  view,
  side,
  row,
  lane,
  highlight,
  animating,
}: TileAuraProps): ReactElement | null {
  const [settings] = useFxSettings();
  const panelReduces = useSetting("reduceMotion");
  const still = panelReduces || reducedMotionNow(settings);
  const spec = settings.intensity === "off" ? null : planTileAura(view, side, row, lane);

  const rootRef = useRef<HTMLDivElement>(null);
  const [touches, setTouches] = useState(0);
  const [ripple, setRipple] = useState(0);
  const lastTouchRef = useRef(-TILE_TOUCH_GAP_MS);

  useEffect(() => {
    if (spec === null) return undefined;
    const element = rootRef.current;
    if (element === null) return undefined;
    const zone = element.closest(".zone");
    if (zone === null) return undefined;

    const handleTouch = (event: Event, isDown: boolean): void => {
      const pointerEvent = event as PointerEvent;
      const pointerType = pointerEvent.pointerType || "mouse";
      if (!isDown && pointerType !== "mouse" && pointerType !== "pen") return;

      const now = performance.now();
      if (now - lastTouchRef.current < TILE_TOUCH_GAP_MS) return;
      lastTouchRef.current = now;

      setTouches((t) => t + 1);
      if (!still) {
        setRipple((r) => r + 1);
      }

      try {
        getAudioEngine().playSfx("auraTouch", { hue: spec.hue });
      } catch {
        // Audio errors must not break interaction.
      }

      if (isDown && pointerType !== "mouse") {
        touchHaptic();
      }
    };

    const onPointerEnter = (e: Event): void => handleTouch(e, false);
    const onPointerDown = (e: Event): void => handleTouch(e, true);

    zone.addEventListener("pointerenter", onPointerEnter);
    zone.addEventListener("pointerdown", onPointerDown);

    return () => {
      zone.removeEventListener("pointerenter", onPointerEnter);
      zone.removeEventListener("pointerdown", onPointerDown);
    };
  }, [spec?.key, spec?.hue, still]);

  const tiles = useMemo(() => (spec === null ? 0 : planTileAuras(view).length), [spec, view]);

  // R3351000: at shed stage 2 the tile halves its motes with the far auras (it measures no box to
  // tell near from far by, so it sheds with the far set).
  const shed = useSyncExternalStore(subscribeShed, shedLevel, () => 0);

  const motes = useMemo(() => {
    if (spec === null || still) return [];
    const viewportWidth = typeof window !== "undefined" ? window.innerWidth : 1280;
    const budget = tileMoteBudget(spec.tier, tiles, FX_INTENSITY_SCALE[settings.intensity] * shedAuraScale(shed), viewportWidth);
    return tileMotes(spec, budget);
  }, [spec, still, tiles, settings.intensity, shed]);

  if (spec === null) return null;

  const response = tileResponse(spec, animating, highlight);
  const state = response.fading ? "out" : spec.dim ? "dim" : "standing";
  const edge = Math.min(1, TILE_EDGE[spec.tier] + (response.lit ? TILE_LIT_BOOST : 0));
  const flip = spec.variant % 2 === 0 ? 1 : -1;

  const style: CSSProperties = {
    "--tile-core": spec.colors[0],
    "--tile-body": spec.colors[1],
    "--tile-glow": spec.colors[2],
    "--tile-glow-px": `${TILE_GLOW_PX[spec.tier]}px`,
    "--tile-edge": edge,
    "--tile-pattern-opacity": TILE_PATTERN_OPACITY,
    "--tile-flip": flip,
    "--tile-phase": `-${spec.variant * 500}ms`,
    "--tile-pulse-ms": `${TILE_PULSE_MS}ms`,
    "--tile-ripple-ms": `${TILE_RIPPLE_MS}ms`,
    "--tile-flicker-ms": `${TILE_FLICKER_MS}ms`,
    "--tile-out-ms": `${TILE_FADE_OUT_MS}ms`,
  } as CSSProperties;

  return (
    <div
      ref={rootRef}
      className="tile-aura"
      aria-hidden="true"
      data-testid={tileTestid(side, row, lane)}
      data-tile-tier={spec.tier}
      data-tile-look={spec.look}
      data-tile-pattern={spec.pattern ?? undefined}
      data-tile-state={state}
      data-tile-lit={response.lit ? "true" : undefined}
      data-tile-hit={response.hit ? "true" : undefined}
      data-tile-motion={still ? "still" : "on"}
      data-tile-touches={touches > 0 ? touches : undefined}
      style={style}
    >
      {spec.tier === "Mythic" && spec.pattern !== null && (
        <svg className="tile-aura-pattern" viewBox="0 0 100 100" aria-hidden="true">
          {TILE_PATTERN_SHAPES[spec.pattern].map((d, index) => (
            <path key={index} d={d} />
          ))}
        </svg>
      )}
      {motes.map((mote, index) => (
        <span
          key={index}
          className="tile-aura-mote"
          data-mote-shape={spec.moteShape}
          style={
            {
              "--mx": `${mote.x}%`,
              "--my": `${mote.y}%`,
              "--mdx": `${mote.dx}px`,
              "--mdy": `${mote.dy}px`,
              "--msize": `${mote.size}px`,
              "--mdelay": `${mote.delayMs}ms`,
              "--mdur": `${mote.durMs}ms`,
            } as CSSProperties
          }
        />
      ))}
      {ripple > 0 && !still && (
        <span key={ripple} className="tile-aura-ripple" onAnimationEnd={() => {}} />
      )}
    </div>
  );
}
export default TileAura;
