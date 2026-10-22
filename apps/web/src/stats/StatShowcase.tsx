// R1584000: the floor's showcase. One stat at a time: its title, its card as a real card face, the
// player's number for it and the board's own animation for that stat, played on a one-card stage
// (showcaseStage.ts) that carries the board's testids, so the board's keyframes (game/animations.css)
// and effects (fx/) run on it unchanged. No effect is new.
//
// The showcase steps on its own every `SHOWCASE_INTERVAL_MS`, and the player steps it by hand with
// the arrows or a dot. It holds like the home hand (R639): under reduced motion (the page's, the
// panel's or the effects setting), a pointer or focus on it, and while the page is hidden. A card
// already moving finishes its animation first. Reduced motion shows the still card and its number,
// and the controls still step it. It reads only what the device logged and the Pocket Pick's public
// rates (CLAUDE.md rule 7).

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

import { landingShowcasePickTestid, landingTestid } from "../auth/testids.ts";
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
import { t, useLocale } from "../i18n/i18n.ts";
import { SHOWCASE_BEAT_GAP_MS, SHOWCASE_INTERVAL_MS, SHOWCASE_LEAD_MS } from "./config.ts";
import type { ShowcaseKey, ShowcaseStat } from "./showcase.ts";
import { OPPONENT_LUNGE_DIR, STAGE_INSTANCE_ID, STAGE_LANE, stageCues, stageScript } from "./showcaseStage.ts";
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
};

export function StatShowcase({ stats, reduced }: StatShowcaseProps): ReactElement | null {
  useLocale();
  const [fx] = useFxSettings();
  const [key, setKey] = useState<ShowcaseKey | null>(null);
  const [held, setHeld] = useState(false);
  const still = reduced || fx.motion === "reduce";
  const fxOn = !still && fx.intensity !== "off";
  const intensity: number = FX_INTENSITY_SCALE[fx.intensity];

  // R639's hold: the interval runs only while nothing reads the stat, and a hidden page skips a tick.
  // A step by hand starts it again, so the stat the player picked gets its whole time.
  useEffect(() => {
    if (still || held || stats.length < 2) return undefined;
    const timer = window.setInterval(() => {
      if (document.visibilityState === "hidden") return;
      setKey((from) => stats[(Math.max(0, stats.findIndex((stat) => stat.key === from)) + 1) % stats.length]?.key ?? null);
    }, SHOWCASE_INTERVAL_MS);
    return () => {
      window.clearInterval(timer);
    };
  }, [still, held, stats, key]);

  const at = Math.max(0, stats.findIndex((stat) => stat.key === key));
  const current = stats[at];
  if (current === undefined) return null;

  const step = (to: number): void => {
    setKey(stats[(to + stats.length) % stats.length]?.key ?? null);
  };

  return (
    <div
      className="landing-showcase"
      data-testid={landingTestid.showcase}
      data-stat={current.key}
      style={{ "--showcase-lead": `${String(SHOWCASE_LEAD_MS)}ms` } as CSSProperties}
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
      {stats.length > 1 ? (
        <div className="landing-showcase-controls">
          <button
            type="button"
            className="landing-showcase-step"
            data-testid={landingTestid.showcasePrev}
            aria-label={t("Previous stat")}
            onClick={() => {
              step(at - 1);
            }}
          >
            ‹
          </button>
          <div className="landing-showcase-picks" role="group" aria-label={t("Pick a stat")}>
            {stats.map((stat) => (
              <button
                key={stat.key}
                type="button"
                className="landing-showcase-pick"
                data-testid={landingShowcasePickTestid(stat.key)}
                aria-pressed={stat.key === current.key}
                aria-label={stat.title()}
                onClick={() => {
                  setKey(stat.key);
                }}
              />
            ))}
          </div>
          <button
            type="button"
            className="landing-showcase-step"
            data-testid={landingTestid.showcaseNext}
            aria-label={t("Next stat")}
            onClick={() => {
              step(at + 1);
            }}
          >
            ›
          </button>
        </div>
      ) : null}
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
    let wait = SHOWCASE_LEAD_MS;
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
  const def = CATALOG[stat.cardId];
  if (def === undefined) return null;

  return (
    <div className="landing-showcase-stage" data-testid={landingTestid.showcaseStage}>
      <div className="landing-showcase-zone" data-testid={zoneTestid} data-animating={beat?.entry.frames.get(zoneTestid)}>
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
          {mark === null ? null : <CardMarks marks={[{ mark: mark.mark, color: mark.color }]} instanceId={STAGE_INSTANCE_ID} />}
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
