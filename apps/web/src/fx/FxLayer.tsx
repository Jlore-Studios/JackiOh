// The fixed effects overlay (docs/polish/1-animations.md, S9) decorates the runner's in-flight entry
// but never paces it (R200). `--anim-squeeze` preserves the runner's actual duration (R201). Its
// layout-effect subscription exists before `Game` enqueues the first entry; it reads only the redacted
// stream (R202). R502: `latest` supplies #21 Hinder's `env.next` and marks the next refresh's empty crystals.

import {
  useContext,
  useEffect,
  useLayoutEffect,
  useRef,
  type ReactElement,
} from "react";

import type { PlayerId, PlayerView } from "@jackioh/shared";

import {
  animationRow,
  leadMs,
  reducedMotionNow,
  type AnimationEntry,
  type AnimationQueue,
  type RunnerSignal,
} from "../game/animations.ts";
import { CatalogContext, type CardLookup } from "../game/catalog.ts";
import { rootVars, type RootVars } from "../game/rootVars.ts";
import { boardShakeSink, resolveAnchor } from "./anchors.ts";
import { FX_DEFAULT_SEED, FX_INTENSITY_SCALE, FX_MOBILE_WIDTH, FX_NEXT_REFRESH_MODIFIER_ID } from "./constants.ts";
import { delayCues, planFx, planHandover, planLethal, planResult } from "./cues.ts";
import { planStillCallouts } from "./motifs/beats.ts";
import { planAnomalyStill } from "./anomaly.ts";
import { planStarshipStill } from "./starship.ts";
import { planTerminusStill } from "./terminus/index.ts";
import { planStage } from "./stage.ts";
import { capacityFor, createFxDirector, type FxDirector } from "./director.ts";
import { createFxMemory } from "./memory.ts";
import { useFxSettings } from "./settings.ts";
import { setShedPointer } from "./shed.ts";
import { useSetting } from "../settings/store.ts";
import { createSurface, type FxSurface } from "./surface.ts";
import { applyManaMarks, manaMarks } from "./manaMarks.ts";
import { sideOf, type Side } from "../game/contract.ts";
import type {
  FxAnchor,
  FxBox,
  FxCardFacts,
  FxFrameSource,
  FxMemory,
  FxPlanEnv,
  FxShakeSink,
  FxVisibility,
} from "./types.ts";
import "./fx.css";

export type FxSeams = {
  now: () => number;
  frames: FxFrameSource;
  visibility: FxVisibility;
  measure: (anchor: FxAnchor) => FxBox | null;
  surface: (canvas: HTMLCanvasElement) => FxSurface | null;
  shakeSink: FxShakeSink;
  seed: number;
  /** Overrides the CatalogContext lookup. */
  catalog: (defId: string) => FxCardFacts | undefined;
  viewportWidth: () => number;
  /** The board element a stage cue acts on (stage.ts), by testid. */
  element: (testid: string) => HTMLElement | null;
};

export type FxLayerProps = {
  queue: AnimationQueue;
  /** The view the board is SHOWING (Game's `shown`), not the newest one. */
  view: PlayerView;
  /** The newest view (Game's `view`), which the burst in flight is heading to. Absent: `view`. */
  latest?: PlayerView;
  /** Game-owned visual hit-stop. It never pauses the runner, game clock, input, or audio. */
  paused?: boolean;
  /** Test seams; production passes none. */
  seams?: Partial<FxSeams>;
};

const SQUEEZE = "--anim-squeeze";
const SCALE = "--anim-scale";

/**
 * #3633: what the layer marks on its parent, the game, while it is so: its effects on, a turn banner
 * up, a stand-in held. animations.css, fx.css and drag.css key off these where they read
 * `.game:has(> .fx-layer[data-fx="on"])`, `.game:has(> .fx-layer [data-fx="banner"])` and
 * `.game:has(.fx-hold)`: a `:has()` on the game made every change under it restyle the board, a
 * hundred milliseconds a frame on a throttled phone (R3351000).
 */
export const FX_LAYER_MARK = "data-fx-layer";
export const FX_BANNER_MARK = "data-fx-banner";
export const FX_HOLD_MARK = "data-fx-hold";

export function defaultNow(): number {
  return performance.now();
}

/** `requestAnimationFrame`, or a source that never fires where there is none (no timers: R200). */
export function defaultFrames(): FxFrameSource {
  if (typeof window.requestAnimationFrame !== "function") {
    return { request: () => 0, cancel: () => undefined };
  }
  return {
    request: (callback) => window.requestAnimationFrame(callback),
    cancel: (handle) => window.cancelAnimationFrame(handle),
  };
}

export function defaultVisibility(): FxVisibility {
  return {
    hidden: () => document.hidden === true,
    subscribe: (listener) => {
      document.addEventListener("visibilitychange", listener);
      return () => document.removeEventListener("visibilitychange", listener);
    },
  };
}


/** The public catalog facts an effect may use: rarity and printed stats of the base face. */
export function catalogFacts(lookup: CardLookup | null, defId: string): FxCardFacts | undefined {
  if (defId === "hidden" || lookup === null) return undefined;
  const info = lookup(defId, false);
  if (info === undefined) return undefined;
  // The card's family lends its look to the effects (looks.ts).
  return { rarity: info.rarity, attack: info.attack, health: info.health, type: info.type, tags: info.tags };
}

/** R502: the sides whose next-refresh rider this entry lays or spends. */
function riderSides(entry: AnimationEntry): Side[] {
  return entry.events.flatMap((event) =>
    event.type === "modifierChanged" && event.modifierId === FX_NEXT_REFRESH_MODIFIER_ID ? [sideOf(entry.view, event.player)] : [],
  );
}


export function FxLayer({ queue, view, latest, paused = false, seams }: FxLayerProps): ReactElement {
  const [settings] = useFxSettings();
  // The panel's "Reduce motion" is read through its hook so a change re-renders the layer.
  const panelReduces = useSetting("reduceMotion");
  const reduces = panelReduces || reducedMotionNow(settings);
  const enabled = !reduces && settings.intensity !== "off";
  // R2789000: under reduced motion the layer draws a keyword's still callout and nothing else.
  const still = reduces && settings.intensity !== "off";
  const settingReduces = settings.motion === "reduce" || panelReduces;
  const intensity: number = FX_INTENSITY_SCALE[settings.intensity];
  const lookup = useContext(CatalogContext);

  const rootRef = useRef<HTMLDivElement | null>(null);
  // #3633: `--anim-squeeze` goes onto the elements that read it (game/rootVars.ts), not onto the parent's
  // own style: there each entry's start restyled the whole game.
  const squeeze = useRef<RootVars | null>(null);
  useLayoutEffect(() => {
    const parent = rootRef.current?.parentElement ?? null;
    if (parent === null) return undefined;
    const owned = rootVars(parent);
    squeeze.current = owned;
    return () => {
      owned.release();
      if (squeeze.current === owned) squeeze.current = null;
    };
  }, []);
  const removeSqueeze = (): void => squeeze.current?.remove(SQUEEZE);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const domRef = useRef<HTMLDivElement | null>(null);
  const director = useRef<FxDirector | null>(null);
  const memory = useRef<FxMemory | null>(null);
  if (memory.current === null) memory.current = createFxMemory();

  // What the runner's listener reads: a ref, so the subscription is made once per queue and never
  // torn down between two entries of one burst.
  const bigMoments = settings.bigMoments;
  const live = useRef({ enabled, still, intensity, lookup, seams, view, latest: latest ?? view, bigMoments });
  live.current = { enabled, still, intensity, lookup, seams, view, latest: latest ?? view, bigMoments };

  const planEnv = (): FxPlanEnv => {
    const current = live.current;
    const override = current.seams?.catalog;
    return {
      intensity: current.intensity,
      card: override ?? ((defId: string) => catalogFacts(live.current.lookup, defId)),
      memory: memory.current!,
      next: current.latest,
      // R3334001: the Starship's cinematic plays at Big moments "full" only, and a phone's throws fewer particles.
      bigMoments: current.bigMoments,
      compact: (current.seams?.viewportWidth?.() ?? window.innerWidth) < FX_MOBILE_WIDTH,
    };
  };

  /** R502: the sides whose next-refresh rider the runner has reached in this burst. */
  const riderReached = useRef(new Set<Side>());
  const markMana = (): void => {
    const current = live.current;
    const sides = riderReached.current;
    applyManaMarks(document, manaMarks(current.view, sides.size > 0 ? { view: current.latest, sides } : undefined));
  };

  // The director lives exactly as long as the canvas and the DOM root it draws into.
  useLayoutEffect(() => {
    if (!enabled && !still) return undefined;
    const canvas = canvasRef.current;
    const domRoot = domRef.current!;
    const s = live.current.seams ?? {};
    // A still layer has no canvas: its callouts are DOM.
    const surface = enabled && canvas !== null ? (s.surface ?? createSurface)(canvas) : null;
    const created = createFxDirector({
      surface,
      domRoot,
      now: s.now ?? defaultNow,
      frames: s.frames ?? defaultFrames(),
      visibility: s.visibility ?? defaultVisibility(),
      measure: s.measure ?? resolveAnchor,
      shakeSink: s.shakeSink ?? boardShakeSink(document),
      seed: s.seed ?? FX_DEFAULT_SEED,
      capacity: capacityFor(s.viewportWidth?.() ?? window.innerWidth),
      element: s.element,
    });
    director.current = created;
    return () => {
      if (director.current === created) director.current = null;
      created.dispose();
      surface?.dispose();
    };
  }, [enabled, still]);

  useLayoutEffect(() => {
    if (paused) director.current?.pause();
    else director.current?.resume();
  }, [paused]);

  // #3633: the marks above. The banner and the holds come and go inside the effects' own root, so they
  // are read off it as it changes; a mutation's record is read before the frame it lands in is drawn.
  useLayoutEffect(() => {
    const parent = rootRef.current?.parentElement ?? null;
    if (parent === null || !enabled) return undefined;
    parent.setAttribute(FX_LAYER_MARK, "on");
    return () => parent.removeAttribute(FX_LAYER_MARK);
  }, [enabled]);
  useLayoutEffect(() => {
    const parent = rootRef.current?.parentElement ?? null;
    const domRoot = domRef.current;
    if (parent === null || domRoot === null) return undefined;
    const mark = (): void => {
      parent.toggleAttribute(FX_BANNER_MARK, domRoot.querySelector('[data-fx="banner"]') !== null);
      parent.toggleAttribute(FX_HOLD_MARK, domRoot.querySelector(".fx-hold") !== null);
    };
    mark();
    const observer = new MutationObserver(mark);
    observer.observe(domRoot, { childList: true, subtree: true });
    return () => {
      observer.disconnect();
      parent.removeAttribute(FX_BANNER_MARK);
      parent.removeAttribute(FX_HOLD_MARK);
    };
  }, [enabled, still]);

  // The reduce setting acts like the media query (R200): index.css zeroes --anim-scale on :root,
  // this zeroes it on the game root.
  useLayoutEffect(() => {
    const parent = rootRef.current?.parentElement ?? null;
    if (parent === null || !settingReduces) return undefined;
    parent.style.setProperty(SCALE, "0");
    return () => {
      parent.style.removeProperty(SCALE);
    };
  }, [settingReduces]);

  /** Entries started since the runner last went idle, and the ones a drain cut short (for planLethal). */
  const played = useRef<AnimationEntry[]>([]);
  const drained = useRef<readonly AnimationEntry[]>([]);

  // Runner signals. Declared after the director so a first `start` already has one to play into.
  useLayoutEffect(() => {
    const onSignal = (signal: RunnerSignal): void => {
      const root = rootRef.current;
      switch (signal.kind) {
        case "start": {
          const entry = signal.entry;
          played.current.push(entry);
          drained.current = [];
          const table = animationRow(entry.type)?.durationMs ?? 0;
          const parent = root?.parentElement ?? null;
          if (parent !== null) {
            // A slam's anticipation and a set piece's hold (R2722001, R2785000) are a wait before the motion, not a slower motion.
            const motionMs = entry.durationMs - leadMs(entry);
            // An event type this build does not know has no row (issue #769): no squeeze.
            if (table > 0) squeeze.current?.set(SQUEEZE, (motionMs / table).toFixed(3));
            else removeSqueeze();
          }
          const reached = riderSides(entry);
          if (reached.length > 0) {
            for (const side of reached) riderReached.current.add(side);
            markMana();
          }
          const target = director.current;
          if (!live.current.enabled || target === null) return;
          const env = planEnv();
          env.memory.remember(entry.events);
          target.play([...planFx(entry, entry.view, env), ...planStage(entry, entry.view, env)]);
          return;
        }
        case "still": {
          // R2789000: an entry of no duration (reduced motion): its keyword callouts, held still;
          // R2801000: and a Starship's launch, one still frame of it; R3338000: and a Terminus set piece's;
          // R3337003: and an Anomaly's still title card.
          const target = director.current;
          if (!live.current.still || target === null) return;
          const env = planEnv();
          env.memory.remember(signal.entry.events);
          target.play([
            ...planStillCallouts(signal.entry, signal.entry.view, env),
            ...planStarshipStill(signal.entry, signal.entry.view, env),
            ...planTerminusStill(signal.entry, signal.entry.view, env),
            ...planAnomalyStill(signal.entry, signal.entry.view, env),
          ]);
          return;
        }
        case "pass":
          // R3407002: a hook's marker plays nothing, but its entry opens for the events after it (R2797000).
          if ((!live.current.enabled && !live.current.still) || director.current === null) return;
          planEnv().memory.remember(signal.entry.events);
          return;
        case "idle":
          removeSqueeze();
          played.current = [];
          return;
        case "drain": {
          removeSqueeze();
          director.current?.clear();
          memory.current?.clear();
          const cut = signal.entries.filter((entry) => !played.current.includes(entry));
          drained.current = [...played.current, ...cut];
          played.current = [];
          return;
        }
        case "skip":
          // R2785000: a skipped set piece cuts to its end; the next entry's start plays on as ever.
          removeSqueeze();
          director.current?.cut();
          return;
        case "reset":
          removeSqueeze();
          director.current?.clear();
          memory.current?.clear();
          played.current = [];
          drained.current = [];
          return;
      }
    };
    const unsubscribe = queue.subscribeSignals(onSignal);
    return () => {
      unsubscribe();
      removeSqueeze();
    };
    // `planEnv` reads refs only, so the subscription depends on the queue alone.
  }, [queue]);

  // A newer view on the board ends every stage effect (stage.ts). A LAYOUT effect, so the swap and
  // the release land in the same paint and no frame shows both, or neither.
  const releasedFor = useRef<PlayerView | null>(null);
  useLayoutEffect(() => {
    if (releasedFor.current !== null && releasedFor.current !== view) director.current?.release();
    releasedFor.current = view;
  }, [view]);

  // R502: the crystals the next refresh will not fill, from the shown view. A LAYOUT effect, after
  // the board has drawn that view's trays, so the mark and the crystals land together.
  useLayoutEffect(() => {
    riderReached.current.clear();
    markMana();
    // `markMana` reads refs only, so the effect depends on the view alone.
  }, [view]);
  useLayoutEffect(
    () => () => {
      applyManaMarks(document, [
        { side: "you", run: { from: 0, count: 0 } },
        { side: "opponent", run: { from: 0, count: 0 } },
      ]);
    },
    [],
  );

  // The turn banner stays until the player acts: the first pointer down, or a prompt opening for the
  // viewer, takes it away, so it never sits over a prompt's zones or behind a Discover sheet.
  useEffect(() => {
    const onDown = (): void => {
      director.current?.dismissBanner();
    };
    document.addEventListener("pointerdown", onDown, true);
    return () => {
      document.removeEventListener("pointerdown", onDown, true);
    };
  }, []);
  // R3351000: the pointer the shed order measures far auras by (shed.ts). One passive listener for
  // the whole layer; the aura layer reads the same store. There was no layer-wide pointer position
  // before: tilt tracks each face on its own (cards/tilt.ts, R3344001).
  useEffect(() => {
    const onMove = (event: PointerEvent): void => {
      setShedPointer({ x: event.clientX, y: event.clientY });
    };
    window.addEventListener("pointermove", onMove, { passive: true });
    return () => {
      window.removeEventListener("pointermove", onMove);
      setShedPointer(null);
    };
  }, []);
  const promptForViewer = view.pending !== null && view.pending.forYou;
  useEffect(() => {
    if (promptForViewer) director.current?.dismissBanner();
  }, [promptForViewer]);

  // The game-over sequence and the hand-over banner run off the shown view, not an entry: `gameOver`
  // is a zero-duration row the runner never plays. `undefined` = no view seen yet.
  const lastResult = useRef<PlayerView["result"] | undefined>(undefined);
  const lastViewer = useRef<PlayerId | undefined>(undefined);
  useEffect(() => {
    const previousResult = lastResult.current;
    const previousViewer = lastViewer.current;
    lastResult.current = view.result;
    lastViewer.current = view.viewer;

    const cut = drained.current;
    drained.current = [];
    const target = director.current;
    if (!live.current.enabled || target === null) return;
    const env = planEnv();
    if (previousResult === null && view.result !== null) {
      // The killing blow first (the drain cleared it before it drew), then the result on its beat.
      for (const entry of cut) env.memory.remember(entry.events);
      const lethal = planLethal(cut, view, env);
      target.play([...lethal.cues, ...delayCues(planResult(view, env), lethal.leadMs)]);
    }
    if (previousViewer !== undefined && previousViewer !== view.viewer) target.play(planHandover(view, env));
  }, [view]);

  return (
    <div ref={rootRef} className="fx-layer" data-testid="fx-layer" data-fx={enabled ? "on" : "off"} data-paused={paused ? "true" : undefined} aria-hidden="true">
      {enabled ? <canvas ref={canvasRef} className="fx-canvas" data-testid="fx-canvas" /> : null}
      {enabled || still ? <div ref={domRef} className="fx-dom" data-testid="fx-dom" data-still={still ? "true" : undefined} /> : null}
    </div>
  );
}

export default FxLayer;
