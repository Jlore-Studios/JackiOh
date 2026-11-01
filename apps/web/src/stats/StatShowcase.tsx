// R1584000: the floor's showcase. One stat at a time: its title, its card as a real card face, the
// player's number for it and the board's own animation for that stat, played on a one-card stage
// (showcaseStage.ts) that carries the board's testids, so the board's keyframes (game/animations.css)
// and effects (fx/) run on it unchanged. No effect is new.
//
// The showcase steps on its own every `SHOWCASE_INTERVAL_MS`, and the player steps it by hand with a
// click, a tap, Enter or Space. It holds like the home hand (R639): under reduced motion (the page's,
// the panel's or the effects setting), a pointer or focus on it, and while the page is hidden. A card
// already moving finishes its animation first. Reduced motion shows the still card and its number,
// and a click still steps it. It reads only what the device logged and the Pocket Pick's public
// rates (CLAUDE.md rule 7).
//
// R1743000: the stats come in a shuffled bag drawn from the page's random source (`nextInBag`): each
// once before any repeats, never the one just shown first in a new round. There is no arrow and no dot
// (a dot for each would give away how many there are), and the showcase is itself the one control.
//
// R2649000: a script's `aura` (Brittle's or Temporary's last count, the board's own) is drawn on the card
// from its way in until the first beat, which waits a beat's gap more so the warning reads first; under
// reduced motion the still card wears it. Each step draws the card afresh, so a beat played twice in a
// row (`stamp`, `windfury`, `thaw`) starts its keyframe again.

import {
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type ReactElement,
} from "react";

import { CATALOG } from "@jackioh/cards";

import { landingTestid } from "../auth/testids.ts";
import CardMarks from "../cards/CardMarks.tsx";
import { CardFace, faceModel } from "../cards/index.ts";
import { markEventOf } from "../cards/marks.ts";
import { resolveAnchor } from "../fx/anchors.ts";
import { FX_DEFAULT_SEED, FX_INTENSITY_SCALE } from "../fx/constants.ts";
import { capacityFor, createFxDirector, type FxDirector } from "../fx/director.ts";
import { defaultFrames, defaultNow, defaultVisibility } from "../fx/FxLayer.tsx";
import { createFxMemory } from "../fx/memory.ts";
import { useFxSettings } from "../fx/settings.ts";
import { createSurface } from "../fx/surface.ts";
import type { FxShakeSink } from "../fx/types.ts";
import { testid } from "../game/contract.ts";
import { useLocale } from "../i18n/i18n.ts";
import type { RandomSource } from "../routes/landingFan.ts";
import { SHOWCASE_BEAT_GAP_MS, SHOWCASE_INTERVAL_MS, SHOWCASE_LEAD_MS } from "./config.ts";
import { EMPTY_BAG, nextInBag, type ShowcaseStat } from "./showcase.ts";
import { OPPONENT_LUNGE_DIR, STAGE_INSTANCE_ID, STAGE_LANE, firstBeatAt, stageCues, stageScript } from "./showcaseStage.ts";
import "../game/animations.css";
import "../fx/fx.css";

/** The showcase shakes nothing: the landing is not the board. */
const NO_SHAKE: FxShakeSink = { apply: () => undefined, clear: () => undefined };

/** Where a stat's card is in its time on screen: coming in, on beat `n` of its animation, or at rest. */
type Step = "enter" | number | "rest";

export type StatShowcaseProps = {
  /** The stats to show, in order; the showcase draws nothing while there are none. */
  stats: readonly ShowcaseStat[];
  /** The page's reduced motion (landing.tsx's); the effects setting's is read here. */
  reduced: boolean;
  /** R1743000: where the order of the stats is drawn from; `Math.random` on the page, a seeded source in tests. */
  random: RandomSource;
};

export function StatShowcase({ stats, reduced, random }: StatShowcaseProps): ReactElement | null {
  useLocale();
  const [fx] = useFxSettings();
  const keys = useMemo(() => stats.map((stat) => stat.key), [stats]);
  const [bag, setBag] = useState(() => nextInBag(EMPTY_BAG, keys, random));
  const [held, setHeld] = useState(false);
  const still = reduced || fx.motion === "reduce";
  const fxOn = !still && fx.intensity !== "off";
  const intensity: number = FX_INTENSITY_SCALE[fx.intensity];

  const advance = (): void => {
    setBag(nextInBag(bag, keys, random));
  };

  // R639's hold: the interval runs only while nothing reads the stat, and a hidden page skips a tick.
  // A step by hand starts it again, so the stat the player moved to gets its whole time.
  useEffect(() => {
    if (still || held || keys.length < 2) return undefined;
    const timer = window.setInterval(() => {
      if (document.visibilityState === "hidden") return;
      setBag(nextInBag(bag, keys, random));
    }, SHOWCASE_INTERVAL_MS);
    return () => {
      window.clearInterval(timer);
    };
  }, [still, held, keys, bag, random]);

  const current = stats.find((stat) => stat.key === bag.shown) ?? stats[0];
  if (current === undefined) return null;
  const steps = stats.length > 1;

  return (
    <div
      className="landing-showcase"
      data-testid={landingTestid.showcase}
      data-stat={current.key}
      style={{ "--showcase-lead": `${String(SHOWCASE_LEAD_MS)}ms` } as CSSProperties}
      role={steps ? "button" : undefined}
      tabIndex={steps ? 0 : undefined}
      onClick={steps ? advance : undefined}
      onKeyDown={
        steps
          ? (event) => {
              if (event.key !== "Enter" && event.key !== " ") return;
              event.preventDefault();
              advance();
            }
          : undefined
      }
      onMouseEnter={() => {
        setHeld(true);
      }}
      onMouseLeave={() => {
        setHeld(false);
      }}
      onFocus={() => {
        setHeld(true);
      }}
      onBlur={() => {
        setHeld(false);
      }}
    >
      <StatStage key={`${current.key}:${current.cardId}`} stat={current} still={still} fxOn={fxOn} intensity={intensity} />
      <div className="landing-showcase-caption">
        <h3 className="landing-showcase-title" data-testid={landingTestid.showcaseTitle}>
          {current.title()}
        </h3>
        <p className="landing-showcase-line" data-testid={landingTestid.showcaseLine}>
          {current.line()}
        </p>
      </div>
    </div>
  );
}

type StatStageProps = { stat: ShowcaseStat; still: boolean; fxOn: boolean; intensity: number };

/** One stat's card and its animation: it comes in, plays each beat of its script in turn, then rests. */
function StatStage({ stat, still, fxOn, intensity }: StatStageProps): ReactElement | null {
  // By what it plays, not by the stat's identity: a Pocket Pick arriving rebuilds every stat and must not restart this one.
  const { animation, cardId: defId, side: stageSide } = stat;
  const script = useMemo(() => stageScript(animation, defId, stageSide), [animation, defId, stageSide]);
  const [step, setStep] = useState<Step>("enter");

  const director = useRef<FxDirector | null>(null);
  const memory = useRef(createFxMemory());
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const domRef = useRef<HTMLDivElement | null>(null);

  // The beats come one after another once the card is in, each held for its animation and a gap.
  useEffect(() => {
    if (still) return undefined;
    const timers: number[] = [];
    let wait = firstBeatAt(script);
    script.beats.forEach((beat, index) => {
      timers.push(
        window.setTimeout(() => {
          setStep(index);
        }, wait),
      );
      wait += beat.entry.durationMs + SHOWCASE_BEAT_GAP_MS;
    });
    timers.push(
      window.setTimeout(() => {
        setStep("rest");
      }, wait),
    );
    return () => {
      for (const timer of timers) window.clearTimeout(timer);
    };
  }, [still, script]);

  // The director lives exactly as long as the canvas and the DOM root it draws into (PackScene's).
  useLayoutEffect(() => {
    if (!fxOn) return undefined;
    const surface = createSurface(canvasRef.current!);
    const created = createFxDirector({
      surface,
      domRoot: domRef.current!,
      now: defaultNow,
      frames: defaultFrames(),
      visibility: defaultVisibility(),
      measure: resolveAnchor,
      shakeSink: NO_SHAKE,
      seed: FX_DEFAULT_SEED,
      capacity: capacityFor(window.innerWidth),
    });
    director.current = created;
    return () => {
      if (director.current === created) director.current = null;
      created.dispose();
      surface?.dispose();
    };
  }, [fxOn]);

  // Each beat's cues are drawn once the card carries that beat's frame (a layout effect runs after the commit).
  useLayoutEffect(() => {
    if (still || typeof step !== "number") return;
    const beat = script.beats[step];
    if (beat !== undefined) director.current?.play(stageCues(beat, intensity, memory.current));
  }, [step, still, script, intensity]);

  const beat = !still && typeof step === "number" ? script.beats[step] : undefined;
  const side = beat?.side ?? script.side;
  const zoneTestid = testid.zone(side, "units", STAGE_LANE);
  const cardTestid = testid.card(STAGE_INSTANCE_ID);
  const entry = !still && step === "enter" ? "enter" : !still && step === "rest" && script.returns ? "return" : undefined;
  const first = beat?.entry.events[0];
  const change = first === undefined ? null : markEventOf(first);
  const mark = change !== null && change.added ? change : null;
  const aura = still || step === "enter" ? script.aura : undefined;
  const def = CATALOG[stat.cardId];
  if (def === undefined) return null;

  return (
    <div className="landing-showcase-stage" data-testid={landingTestid.showcaseStage}>
      <div key={String(step)} className="landing-showcase-zone" data-testid={zoneTestid} data-animating={beat?.entry.frames.get(zoneTestid)}>
        <div
          className="landing-showcase-card"
          data-testid={cardTestid}
          data-def-id={stat.cardId}
          data-entry={entry}
          data-animating={beat?.entry.frames.get(cardTestid)}
          data-field-trap={beat?.entry.type === "trapFired" ? "true" : undefined}
          style={side === "opponent" ? ({ "--lunge-dir": OPPONENT_LUNGE_DIR } as CSSProperties) : undefined}
        >
          <CardFace face={faceModel({ defId: stat.cardId, def, radiant: false })} />
          <CardMarks marks={mark === null ? [] : [{ mark: mark.mark, color: mark.color }]} instanceId={STAGE_INSTANCE_ID} bareAura={aura} />
        </div>
      </div>
      {fxOn ? (
        <div className="fx-layer" data-testid={landingTestid.showcaseFx} data-fx="on" aria-hidden="true">
          <canvas ref={canvasRef} className="fx-canvas" />
          <div ref={domRef} className="fx-dom" />
        </div>
      ) : null}
    </div>
  );
}
