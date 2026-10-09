// The effects layer (docs/polish/1-animations.md, S9): a fixed, click-through overlay that decorates
// the animation runner's in-flight entry. It paces nothing (R200): it only hears `subscribeSignals`
// and never calls `schedule` or `drain`. Besides the board shake it writes `--anim-squeeze` on its
// parent: the duration the runner gave an entry (speed setting, burst budget, R201) over the table's.
// The subscription is a LAYOUT effect: `Game` enqueues events in its own layout effect and child
// layout effects run first, so the listener exists before the first entry starts.
// It reads only the redacted stream (R202). Under reduced motion or intensity "off" it renders the
// empty root (`data-fx="off"`) but keeps `--anim-squeeze`; the reduce setting also zeroes `--anim-scale`.
// A drain clears the killing blow before it draws, so the layer keeps the cut entries and replays the
// lethal one ahead of game-over (`planLethal`).
// R502: `latest` is the newest view, read for what events do not carry (#21 Hinder's `env.next`) and
// to mark the crystals the next refresh will not fill (`manaMarks.ts`), even under reduced motion.

import {
  useContext,
  useEffect,
  useLayoutEffect,
  useRef,
  type ReactElement,
} from "react";

import type { PlayerId, PlayerView } from "@jackioh/shared";

import {
  ANIMATIONS,
  reducedMotionNow,
  type AnimationEntry,
  type AnimationQueue,
  type RunnerSignal,
} from "../game/animations.ts";
import { CatalogContext, type CardLookup } from "../game/catalog.ts";
import { boardShakeSink, resolveAnchor } from "./anchors.ts";
import { FX_DEFAULT_SEED, FX_INTENSITY_SCALE, FX_NEXT_REFRESH_MODIFIER_ID } from "./constants.ts";
import { delayCues, planFx, planHandover, planLethal, planResult } from "./cues.ts";
import { planStage } from "./stage.ts";
import { capacityFor, createFxDirector, type FxDirector } from "./director.ts";
import { createFxMemory } from "./memory.ts";
import { useFxSettings } from "./settings.ts";
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

function defaultNow(): number {
  return performance.now();
}

/** `requestAnimationFrame`, or a source that never fires where there is none (no timers: R200). */
function defaultFrames(): FxFrameSource {
  if (typeof window.requestAnimationFrame !== "function") {
    return { request: () => 0, cancel: () => undefined };
  }
  return {
    request: (callback) => window.requestAnimationFrame(callback),
    cancel: (handle) => window.cancelAnimationFrame(handle),
  };
}

function defaultVisibility(): FxVisibility {
  return {
    hidden: () => document.hidden === true,
    subscribe: (listener) => {
      document.addEventListener("visibilitychange", listener);
      return () => document.removeEventListener("visibilitychange", listener);
    },
  };
}


/** The public catalog facts an effect may use: rarity and printed stats of the base face. */
function catalogFacts(lookup: CardLookup | null, defId: string): FxCardFacts | undefined {
  if (defId === "hidden" || lookup === null) return undefined;
  const info = lookup(defId, false);
  if (info === undefined) return undefined;
  // The card's family lends its look to the effects (looks.ts); the catalog is public.
  return { rarity: info.rarity, attack: info.attack, health: info.health, type: info.type, tags: info.tags };
}

/** R502: the sides whose next-refresh rider this entry lays or spends. */
function riderSides(entry: AnimationEntry): Side[] {
  return entry.events.flatMap((event) =>
    event.type === "modifierChanged" && event.modifierId === FX_NEXT_REFRESH_MODIFIER_ID ? [sideOf(entry.view, event.player)] : [],
  );
}

function removeSqueeze(root: HTMLElement | null): void {
  root?.parentElement?.style.removeProperty(SQUEEZE);
}

export function FxLayer({ queue, view, latest, paused = false, seams }: FxLayerProps): ReactElement {
  const [settings] = useFxSettings();
  // The panel's "Reduce motion" is read through its hook so a change re-renders the layer.
  const panelReduces = useSetting("reduceMotion");
  const enabled = !panelReduces && !reducedMotionNow(settings) && settings.intensity !== "off";
  const settingReduces = settings.motion === "reduce" || panelReduces;
  const intensity: number = FX_INTENSITY_SCALE[settings.intensity];
  const lookup = useContext(CatalogContext);

  const rootRef = useRef<HTMLDivElement | null>(null);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const domRef = useRef<HTMLDivElement | null>(null);
  const director = useRef<FxDirector | null>(null);
  const memory = useRef<FxMemory | null>(null);
  if (memory.current === null) memory.current = createFxMemory();

  // What the runner's listener reads: a ref, so the subscription is made once per queue and never
  // torn down between two entries of one burst.
  const live = useRef({ enabled, intensity, lookup, seams, view, latest: latest ?? view });
  live.current = { enabled, intensity, lookup, seams, view, latest: latest ?? view };

  const planEnv = (): FxPlanEnv => {
    const current = live.current;
    const override = current.seams?.catalog;
    return {
      intensity: current.intensity,
      card: override ?? ((defId: string) => catalogFacts(live.current.lookup, defId)),
      memory: memory.current!,
      next: current.latest,
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
    if (!enabled) return undefined;
    const canvas = canvasRef.current!;
    const domRoot = domRef.current!;
    const s = live.current.seams ?? {};
    const surface = (s.surface ?? createSurface)(canvas);
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
  }, [enabled]);

  useLayoutEffect(() => {
    if (paused) director.current?.pause();
    else director.current?.resume();
  }, [paused]);

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
          const table = ANIMATIONS[entry.type].durationMs;
          const parent = root?.parentElement ?? null;
          if (parent !== null) {
            // A slam's anticipation is a wait before the motion, not a slower motion.
            const motionMs = entry.durationMs - (entry.slam?.anticipationMs ?? 0);
            parent.style.setProperty(SQUEEZE, (motionMs / table).toFixed(3));
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
        case "idle":
          removeSqueeze(root);
          played.current = [];
          return;
        case "drain": {
          removeSqueeze(root);
          director.current?.clear();
          memory.current?.clear();
          const cut = signal.entries.filter((entry) => !played.current.includes(entry));
          drained.current = [...played.current, ...cut];
          played.current = [];
          return;
        }
        case "reset":
          removeSqueeze(root);
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
      removeSqueeze(rootRef.current);
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
    // `planEnv` reads refs only, so the effect depends on the view alone.
  }, [view]);

  return (
    <div ref={rootRef} className="fx-layer" data-testid="fx-layer" data-fx={enabled ? "on" : "off"} data-paused={paused ? "true" : undefined} aria-hidden="true">
      {enabled ? <canvas ref={canvasRef} className="fx-canvas" data-testid="fx-canvas" /> : null}
      {enabled ? <div ref={domRef} className="fx-dom" data-testid="fx-dom" /> : null}
    </div>
  );
}

export default FxLayer;
