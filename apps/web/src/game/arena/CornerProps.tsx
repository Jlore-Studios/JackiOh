// A playmat's corner props (#3348, R3348001): small cosmetic things to find, tap, and tap again for a
// bigger payoff, with a rare surprise. This layer is the viewer's own and local: it takes no `view`, sends
// nothing, logs nothing and never holds the game up (CLAUDE.md rule 7). The viewer's own half answers
// taps; the opponent's half shows their mat's props still (R1834004), and only yours answer you.

import { useCallback, useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactElement, type RefObject } from "react";

import { getAudioEngine } from "../../audio/engine.ts";
import { tapHaptic, type HapticMoment } from "../../haptics/haptics.ts";
import { t } from "../../i18n/i18n.ts";
import { CORNER_BIG_MS, CORNER_OBSTACLES, CORNER_RARE_MS, CORNER_SEQUENCE_MS, CORNER_STILL_MS, CORNER_TAP_MS } from "./config.ts";
import { emptyCornerState, soundsFor, tapCorner, type CornerOutcome } from "./cornerPlay.ts";
import { propFits, type Box } from "./fit.ts";
import { arenaPlay, arenaRandom } from "./play.ts";
import { PROP_ART } from "./PropArt.tsx";
import { propsOf, sequenceOf, type CornerPropDef } from "./props.ts";

export type CornerPropsProps = {
  /** The seat's playmat id (`table-tavern` for the default). */
  mat: string;
  /** "yours" answers taps; "theirs" is the opponent's mat's props, still. */
  half: "yours" | "theirs";
  /** The animation queue is draining: props still answer, quietly. */
  draining?: boolean;
};

type Fit = "true" | "false";

/** How long an outcome's change stays on a prop. */
const OUTCOME_MS: Readonly<Record<CornerOutcome, number>> = {
  tap: CORNER_TAP_MS,
  escalate: CORNER_BIG_MS,
  rare: CORNER_RARE_MS,
  sequence: CORNER_SEQUENCE_MS,
};

const HAPTIC_OF: Readonly<Record<CornerOutcome, HapticMoment>> = {
  tap: "corner",
  escalate: "cornerBig",
  rare: "cornerRare",
  sequence: "cornerRare",
};

const boxOf = (element: Element): Box => {
  const { left, top, right, bottom } = element.getBoundingClientRect();
  return { left, top, right, bottom };
};

/** Whether each prop of `root` may be seen: it must meet no tile, hand, seat, control or log, and stay on the screen. */
function useFits(root: RefObject<HTMLDivElement | null>, mat: string): Readonly<Record<string, Fit>> {
  const [fits, setFits] = useState<Readonly<Record<string, Fit>>>({});

  const measure = useCallback((): void => {
    const layer = root.current;
    if (layer === null) return;
    const scope: ParentNode = layer.closest(".board") ?? document;
    const obstacles = [...scope.querySelectorAll(CORNER_OBSTACLES)].map(boxOf);
    const viewport = { width: window.innerWidth, height: window.innerHeight };
    const next: Record<string, Fit> = {};
    for (const prop of layer.querySelectorAll<HTMLElement>(".corner-prop")) {
      next[prop.dataset.prop ?? ""] = propFits(boxOf(prop), obstacles, viewport) ? "true" : "false";
    }
    setFits((previous) => {
      const same = Object.keys(next).length === Object.keys(previous).length && Object.entries(next).every(([id, fit]) => previous[id] === fit);
      return same ? previous : next;
    });
  }, [root]);

  useLayoutEffect(() => {
    measure();
    const layer = root.current;
    const board = layer?.closest(".board") ?? null;
    window.addEventListener("resize", measure);
    let observer: ResizeObserver | null = null;
    if (typeof ResizeObserver !== "undefined" && board !== null) {
      observer = new ResizeObserver(measure);
      for (const watched of [board, ...board.querySelectorAll(".field, .control-bar, .hand, .seat, .log")]) observer.observe(watched);
    }
    return () => {
      window.removeEventListener("resize", measure);
      observer?.disconnect();
    };
  }, [measure, mat, root]);

  return fits;
}

type Shown = { state?: CornerOutcome; play?: "move" | "still"; serial: number };

function YourProps({ mat, draining }: { mat: string; draining: boolean }): ReactElement {
  const defs = propsOf(mat);
  const root = useRef<HTMLDivElement>(null);
  const fits = useFits(root, mat);
  const corner = useRef(emptyCornerState());
  const pointer = useRef("");
  const timers = useRef(new Map<string, number>());
  const [shown, setShown] = useState<Readonly<Record<string, Shown>>>({});

  useEffect(() => {
    const live = timers.current;
    return () => {
      for (const timer of live.values()) window.clearTimeout(timer);
      live.clear();
    };
  }, []);

  /** Shows `outcome` on `def` and takes it off after its time. */
  const show = (def: CornerPropDef, outcome: CornerOutcome, play: "move" | "still"): void => {
    setShown((previous) => ({ ...previous, [def.id]: { state: outcome, play, serial: (previous[def.id]?.serial ?? 0) + 1 } }));
    const earlier = timers.current.get(def.id);
    if (earlier !== undefined) window.clearTimeout(earlier);
    timers.current.set(
      def.id,
      window.setTimeout(() => {
        timers.current.delete(def.id);
        setShown((previous) => ({ ...previous, [def.id]: { serial: previous[def.id]?.serial ?? 0 } }));
      }, play === "still" ? CORNER_STILL_MS : OUTCOME_MS[outcome]),
    );
  };

  const press = (def: CornerPropDef): void => {
    const { state, outcome } = tapCorner(corner.current, mat, def.id, performance.now(), arenaRandom);
    corner.current = state;
    // Quiet while the animation queue drains: one soft cue, a still change, no buzz.
    const quiet = draining;
    try {
      for (const cue of soundsFor(def, outcome, quiet)) getAudioEngine().playSfx(cue.sfx, cue.params, cue.delayMs);
    } catch {
      // Web Audio can be unavailable before a gesture unlock; the prop still answers.
    }
    if (pointer.current === "touch" && !quiet) tapHaptic(HAPTIC_OF[outcome]);
    pointer.current = "";
    const play = quiet ? "still" : arenaPlay();
    if (play === "none") return;
    // A finished sequence lights both props; any other outcome is the tapped prop's alone.
    const lit = outcome === "sequence" ? defs.filter((other) => sequenceOf(mat)?.includes(other.id) === true) : [def];
    for (const prop of lit) show(prop, outcome, play);
  };

  return (
    <div className="corner-props" data-half="yours" data-mat={mat} ref={root}>
      {defs.map((def) => {
        const now = shown[def.id];
        return (
          <button
            type="button"
            className="corner-prop"
            key={def.id}
            data-prop={def.id}
            data-corner={def.corner}
            data-fit={fits[def.id] ?? "pending"}
            data-state={now?.state}
            data-play={now?.play}
            data-move={def.move}
            data-payoff={def.payoff}
            aria-label={t("A decoration on your playmat")}
            onPointerDown={(event) => {
              pointer.current = event.pointerType;
            }}
            onClick={() => {
              press(def);
            }}
          >
            <span key={now?.serial ?? 0} className="corner-prop__art" style={{ "--prop-pitch": String(def.pitch) } as CSSProperties}>
              {PROP_ART[def.id]}
            </span>
          </button>
        );
      })}
    </div>
  );
}

function TheirProps({ mat }: { mat: string }): ReactElement {
  const root = useRef<HTMLDivElement>(null);
  const fits = useFits(root, mat);
  return (
    <div className="corner-props" data-half="theirs" data-mat={mat} ref={root}>
      {propsOf(mat).map((def) => (
        <span className="corner-prop" key={def.id} data-prop={def.id} data-corner={def.corner} data-fit={fits[def.id] ?? "pending"} aria-hidden="true">
          <span className="corner-prop__art">{PROP_ART[def.id]}</span>
        </span>
      ))}
    </div>
  );
}

/** The mat's corner props for one half of the field; a new playmat starts its taps afresh. */
export default function CornerProps({ mat, half, draining = false }: CornerPropsProps): ReactElement {
  return half === "yours" ? <YourProps key={mat} mat={mat} draining={draining} /> : <TheirProps key={mat} mat={mat} />;
}
