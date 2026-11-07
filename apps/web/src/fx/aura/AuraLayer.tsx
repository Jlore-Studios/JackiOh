// R2787000: the aura layer, the standing look of every Field Spell, Field Trap and lasting modifier the
// shown view holds (plan.ts), over the zone, lane, side or board it covers: a tint and an edge glow per
// aura, and the motes of all of them on one canvas driven by one frame loop (loop.ts) that runs only
// while an aura shows and never while the page is hidden. Each aura begins and ends with the shown view,
// so nothing begins at mount. Reduced motion leaves the tint and the edge still, with no canvas and no
// loop; the effects intensity `off` draws nothing. Components take the view and nothing else (CLAUDE.md
// rule 7): no definition reaches the page (R202).
//
// R3337003: an Anomaly's backdrop is a side aura that also carries its board's code (`data-anomaly`) and
// picture (`--aura-picture`), both public (R202); reduced motion plans none.

import { useLayoutEffect, useRef, useState, type CSSProperties, type ReactElement } from "react";

import type { PlayerId, PlayerView } from "@jackioh/shared";

import { ANOMALIES, type AnomalyCode } from "../../cards/anomalies.ts";
import { anomalyArtUri } from "../../cards/art/svg.ts";
import { newEventsSince, reducedMotionNow } from "../../game/animations.ts";
import { byTestid } from "../../game/contract.ts";
import { useSetting } from "../../settings/store.ts";
import { FX_CARD_TABLE, type FxCardEntry } from "../cards/tables.ts";
import { AURA_FADE_IN_MS, AURA_FADE_OUT_MS, FX_DEFAULT_SEED, FX_INTENSITY_SCALE } from "../constants.ts";
import { capacityFor } from "../director.ts";
import { defaultFrames, defaultNow, defaultVisibility } from "../FxLayer.tsx";
import { createFrameLoop } from "../loop.ts";
import { createFxMemory } from "../memory.ts";
import { createRng } from "../rng.ts";
import { useFxSettings } from "../settings.ts";
import { createSurface, type FxSurface } from "../surface.ts";
import type { FxBox, FxFrameSource, FxMemory, FxVisibility } from "../types.ts";
import { createAuraDrift, type AuraDrift } from "./drift.ts";
import { AURA_LOOKS } from "./looks.ts";
import {
  attributeModifiers,
  auraBudget,
  auraTestid,
  auraZones,
  NO_AURAS,
  noteAnomalies,
  planAuras,
  reconcileAuras,
  settleAuras,
  stackAuras,
  type AuraSources,
  type AuraTrack,
  type TrackedAura,
} from "./plan.ts";
import "./aura.css";

export type AuraSeams = {
  now: () => number;
  frames: FxFrameSource;
  visibility: FxVisibility;
  surface: (canvas: HTMLCanvasElement) => FxSurface | null;
  /** The box that spans the zones `testids` names, or null while one is missing. */
  measure: (testids: readonly string[]) => FxBox | null;
  viewportWidth: () => number;
  seed: number;
  table: ReadonlyMap<string, FxCardEntry>;
};

export type AuraLayerProps = {
  /** The view the board is SHOWING (Game's `shown`): every aura begins and ends with it. */
  view: PlayerView;
  /** The newest view (Game's `view`), whose events say which play added a modifier. Absent: `view`. */
  latest?: PlayerView;
  /** Test seams; production passes none. */
  seams?: Partial<AuraSeams>;
};

export const AURA_LAYER_TESTID = "aura-layer";
export const AURA_CANVAS_TESTID = "aura-canvas";

/** The union of the zones' boxes, or null while one of them is not on the page or it is empty. */
export function measureZones(testids: readonly string[]): FxBox | null {
  let left = Infinity;
  let top = Infinity;
  let right = -Infinity;
  let bottom = -Infinity;
  for (const id of testids) {
    const zone = byTestid(id);
    if (zone === null) return null;
    const rect = zone.getBoundingClientRect();
    left = Math.min(left, rect.left);
    top = Math.min(top, rect.top);
    right = Math.max(right, rect.right);
    bottom = Math.max(bottom, rect.bottom);
  }
  if (!(right > left && bottom > top)) return null;
  return { x: left, y: top, width: right - left, height: bottom - top };
}

function measureAll(auras: readonly TrackedAura[], measure: AuraSeams["measure"]): Map<string, FxBox> {
  const out = new Map<string, FxBox>();
  for (const aura of auras) {
    const box = measure(auraZones(aura.spec));
    if (box !== null) out.set(aura.spec.key, box);
  }
  return out;
}

function sameBoxes(a: ReadonlyMap<string, FxBox>, b: ReadonlyMap<string, FxBox>): boolean {
  if (a.size !== b.size) return false;
  for (const [key, box] of a) {
    const other = b.get(key);
    if (other === undefined || other.x !== box.x || other.y !== box.y || other.width !== box.width || other.height !== box.height) return false;
  }
  return true;
}

type Engine = { drift: AuraDrift; wake: () => void };

export function AuraLayer({ view, latest, seams }: AuraLayerProps): ReactElement | null {
  const [settings] = useFxSettings();
  // The panel's "Reduce motion" is read through its hook so a change re-renders the layer, as FxLayer's.
  const panelReduces = useSetting("reduceMotion");
  const off = settings.intensity === "off";
  const still = panelReduces || reducedMotionNow(settings);
  const intensity: number = FX_INTENSITY_SCALE[settings.intensity];

  // What the effects read: a ref, so a seam object made each render restarts nothing.
  const live = useRef(seams ?? {});
  live.current = seams ?? {};
  const now = (): number => (live.current.now ?? defaultNow)();
  const measure = (testids: readonly string[]): FxBox | null => (live.current.measure ?? measureZones)(testids);

  // Which play added each modifier, and (R3337003) which Anomaly each side stands as, read off the newest
  // view's events as they arrive. A viewer change starts over from the new viewer's window, as the
  // board's runner does (Game.tsx).
  const memory = useRef<FxMemory | null>(null);
  if (memory.current === null) memory.current = createFxMemory();
  const sources = useRef<AuraSources>(new Map());
  const anomalies = useRef(new Map<PlayerId, AnomalyCode>());
  const seenLatest = useRef<PlayerView | null>(null);
  const newest = latest ?? view;
  useLayoutEffect(() => {
    const previous = seenLatest.current;
    if (previous === newest) return;
    seenLatest.current = newest;
    const remembered = memory.current!;
    if (previous === null || previous.viewer !== newest.viewer) {
      remembered.clear();
      sources.current.clear();
      anomalies.current.clear();
      attributeModifiers(newest.events, remembered, sources.current);
      noteAnomalies(newest.events, anomalies.current);
    } else {
      const events = newEventsSince(previous.events, newest.events);
      attributeModifiers(events, remembered, sources.current);
      noteAnomalies(events, anomalies.current);
    }
  }, [newest]);

  // The auras of the shown view, against the ones already up. The effects off forget them all, so turning
  // the effects back on begins nothing.
  const [track, setTrack] = useState<AuraTrack>(NO_AURAS);
  const trackRef = useRef(track);
  trackRef.current = track;
  useLayoutEffect(() => {
    if (off) {
      setTrack(NO_AURAS);
      return;
    }
    // R3337003: an Anomaly's backdrop is off under reduced motion.
    const planned = planAuras(view, sources.current, live.current.table ?? FX_CARD_TABLE, still ? new Map() : anomalies.current);
    const t = now();
    setTrack((prev) => reconcileAuras(settleAuras(prev, t), planned, t, !still));
    // `now` reads a ref only, so the plan depends on the view and the two switches alone.
  }, [view, still, off]);

  const shown = off ? [] : stackAuras(track.auras);
  const budget = auraBudget(shown.length, intensity, capacityFor((live.current.viewportWidth ?? (() => window.innerWidth))()));

  // The board can move under the auras without this layer rendering (a resize, an orientation change, a
  // scroll), so the boxes are measured after every render and on those events, and set only when one moved.
  const [boxes, setBoxes] = useState<ReadonlyMap<string, FxBox>>(() => new Map());
  const shownRef = useRef(shown);
  shownRef.current = shown;
  useLayoutEffect(() => {
    const next = shown.length === 0 ? new Map<string, FxBox>() : measureAll(shown, measure);
    setBoxes((prev) => (sameBoxes(prev, next) ? prev : next));
  });
  const measuring = shown.length > 0;
  useLayoutEffect(() => {
    if (!measuring) return undefined;
    const update = (): void => {
      const next = measureAll(shownRef.current, measure);
      setBoxes((prev) => (sameBoxes(prev, next) ? prev : next));
    };
    window.addEventListener("resize", update);
    window.addEventListener("scroll", update, true);
    return () => {
      window.removeEventListener("resize", update);
      window.removeEventListener("scroll", update, true);
    };
    // `measure` reads a ref only.
  }, [measuring]);

  // The motes and their loop live exactly as long as the canvas, which stands while an aura shows and
  // motion is on. Each frame steps and draws them, and settles a begun or ended aura's phase.
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const engine = useRef<Engine | null>(null);
  const moving = !off && !still && shown.length > 0;
  useLayoutEffect(() => {
    if (!moving) return undefined;
    const s = live.current;
    const surface = (s.surface ?? createSurface)(canvasRef.current!);
    const drift = createAuraDrift({ rng: createRng(s.seed ?? FX_DEFAULT_SEED) });
    // Motes to draw, or a phase to settle. Without a canvas (no 2D context) only the phases are work.
    const busy = (current: AuraTrack): boolean => (surface !== null && drift.count() > 0) || current.auras.some((aura) => aura.phase !== "standing");
    const loop = createFrameLoop({
      frames: s.frames ?? defaultFrames(),
      visibility: s.visibility ?? defaultVisibility(),
      now: s.now ?? defaultNow,
      onFrame: ({ dt, now: t }) => {
        drift.step(dt);
        if (surface !== null) {
          surface.clear();
          drift.draw(surface.ctx);
        }
        const current = trackRef.current;
        const settled = settleAuras(current, t);
        if (settled !== current) {
          trackRef.current = settled;
          setTrack((prev) => settleAuras(prev, t));
        }
        return busy(settled);
      },
      onResume: () => created.wake(),
    });
    const created: Engine = {
      drift,
      wake: () => {
        if (busy(trackRef.current)) loop.wake();
      },
    };
    engine.current = created;
    return () => {
      if (engine.current === created) engine.current = null;
      loop.dispose();
      drift.clear();
      surface?.dispose();
    };
  }, [moving]);

  // What the motes cover follows every render: the auras that show, their boxes, budget and phase.
  useLayoutEffect(() => {
    const current = engine.current;
    if (current === null) return;
    current.drift.sync(
      shown.flatMap((aura) => {
        const box = boxes.get(aura.spec.key);
        return box === undefined ? [] : [{ key: aura.spec.key, box, look: aura.spec.look, colors: aura.spec.colors, budget, phase: aura.phase }];
      }),
    );
    current.wake();
  });

  if (off || shown.length === 0) return null;
  return (
    <div className="aura-layer" data-testid={AURA_LAYER_TESTID} data-aura={still ? "still" : "on"} data-aura-motes={still ? 0 : budget * shown.length} aria-hidden="true">
      {still ? null : <canvas ref={canvasRef} className="aura-canvas" data-testid={AURA_CANVAS_TESTID} />}
      {[...shown].reverse().map((aura) => {
        const box = boxes.get(aura.spec.key);
        if (box === undefined) return null;
        const look = AURA_LOOKS[aura.spec.look];
        const [, body, glow] = aura.spec.colors;
        const { anomaly } = aura.spec;
        const style = {
          ...(anomaly === undefined ? {} : { "--aura-picture": `url("${anomalyArtUri(anomaly, ANOMALIES[anomaly].rich)}")` }),
          left: box.x,
          top: box.y,
          width: box.width,
          height: box.height,
          "--aura-body": body,
          "--aura-glow": glow ?? body,
          "--aura-tint": String(look.tint),
          "--aura-edge": String(look.edge),
          "--aura-glow-px": `${String(look.glow)}px`,
          "--aura-in-ms": `${String(AURA_FADE_IN_MS)}ms`,
          "--aura-out-ms": `${String(AURA_FADE_OUT_MS)}ms`,
        } as CSSProperties;
        return (
          <div
            key={aura.spec.key}
            className="aura"
            data-testid={auraTestid(aura.spec.key)}
            data-aura-look={aura.spec.look}
            data-aura-scope={aura.spec.scope}
            data-aura-phase={aura.phase}
            data-anomaly={anomaly}
            style={style}
          />
        );
      })}
    </div>
  );
}

export default AuraLayer;
