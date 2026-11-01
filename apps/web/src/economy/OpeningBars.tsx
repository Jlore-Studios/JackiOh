// The two bars of an opening (#1658, part of #1651, SPEC §14.5, R1305000, R1953000): the card token
// bar and the cosmetic token bar, pinned across the top of the viewport (the HUD) while the opening is
// on screen. Each slot's points (R1999000) fly from the slot into the HUD, tinted by rarity.
// THE SERVER DECIDES (CLAUDE.md rule 7). `bars` is the server's answer, what the cosmetic bar paid
// (R1305001) and each slot's points included; nothing here counts a point or picks a reward, and both
// bars are drawn whether or not the opening moved them.
// REDUCED MOTION, or the effects' intensity off, flies, sweeps and pulses nothing (`still` carries
// the intensity off to FillBar).

import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type ReactElement } from "react";

import { useFxSettings } from "../fx/settings.ts";
import { t } from "../i18n/i18n.ts";
import type { OpeningBarsAnswer } from "../net/api.ts";
import {
  BAR_FLIGHT_MS,
  BAR_FLIGHT_STAGGER_MS,
  BURST_MS,
  fillReward,
  flightsMs,
  type BarFlight,
  type OpeningBar,
  type RevealItem,
} from "./bars.ts";
import FillBar, { useReducedMotion } from "./FillBar.tsx";
import RewardReveal from "./RewardReveal.tsx";
import "./economy.css";

export type OpeningBarsProps = {
  bars: OpeningBarsAnswer;
  /** The slots' points, flown in before the bars fill (`bars.ts::flightsOf`); with none they fill at once. */
  flights?: readonly BarFlight[];
  /** Whether the opening plays yet; until it does, both bars rest at their points before. */
  play?: boolean;
  testid?: string;
};

/** Where an opening is: held back, its points in flight, or its bars filling. */
type Phase = "waiting" | "flights" | "fill";

/** Where a flight takes off and how far it travels to the HUD's center, in viewport pixels. */
type Course = { x: number; y: number; dx: number; dy: number };

/** The center of an element's box on the viewport. */
function centerOf(element: Element): { x: number; y: number } {
  const box = element.getBoundingClientRect();
  return { x: box.left + box.width / 2, y: box.top + box.height / 2 };
}

/** The bars of one opening; a new answer plays from its start, its rewards shown afresh. */
export default function OpeningBars(props: OpeningBarsProps): ReactElement {
  return <Opening key={JSON.stringify(props.bars)} {...props} />;
}

function Opening({ bars, flights = [], play = true, testid }: OpeningBarsProps): ReactElement {
  const [fx] = useFxSettings();
  const moving = !useReducedMotion() && fx.intensity !== "off";
  const next: Phase = moving && flights.length > 0 ? "flights" : "fill";
  const [phase, setPhase] = useState<Phase>(play ? next : "waiting");
  const [courses, setCourses] = useState<Readonly<Record<string, Course>> | null>(null);
  const [rewards, setRewards] = useState<readonly RevealItem[]>([]);
  const hud = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    if (phase === "waiting" && play) setPhase(next);
  }, [phase, play, next]);

  // The bars fill once the last flight has landed; motion turned off mid-flight lands them all at once.
  useEffect(() => {
    if (phase !== "flights") return undefined;
    if (!moving) {
      setPhase("fill");
      return undefined;
    }
    const timer = window.setTimeout(() => {
      setPhase("fill");
    }, flightsMs(flights.length));
    return () => {
      window.clearTimeout(timer);
    };
  }, [phase, moving, flights.length]);

  // Each flight takes off from its slot's center (the HUD's, when the slot is not on the page) for the HUD's.
  useLayoutEffect(() => {
    if (phase !== "flights" || hud.current === null) return;
    const to = centerOf(hud.current);
    const measured: Record<string, Course> = {};
    for (const flight of flights) {
      const source = document.querySelector(`[data-testid="${flight.source}"]`);
      const from = source === null ? to : centerOf(source);
      measured[flight.key] = { x: from.x, y: from.y, dx: to.x - from.x, dy: to.y - from.y };
    }
    setCourses(measured);
  }, [phase, flights]);

  const reveal = (bar: "card" | "cosmetic", fill: number): void => {
    const item = fillReward(bars, bar, fill);
    if (item === null) return;
    setRewards((before) => (before.some((shown) => shown.key === item.key) ? before : [...before, item]));
  };

  const part = (name: string): string | undefined => (testid === undefined ? undefined : `${testid}-${name}`);
  // Until the bars fill, each rests where the opening found it.
  const drawBar = (name: "card" | "cosmetic", label: string, bar: OpeningBar): ReactElement =>
    phase === "fill" ? (
      <FillBar
        label={label}
        size={bar.size}
        value={bar.after}
        from={bar.before}
        fills={bar.pays}
        still={!moving}
        onFill={(fill) => {
          reveal(name, fill);
        }}
        testid={part(name)}
      />
    ) : (
      <FillBar label={label} size={bar.size} value={bar.before} testid={part(name)} />
    );

  const style = { "--flight-ms": `${String(BAR_FLIGHT_MS)}ms`, "--pulse-ms": `${String(BURST_MS)}ms` } as CSSProperties;
  return (
    <div
      ref={hud}
      className="opening-bars opening-hud"
      data-testid={testid}
      data-motion={moving ? "full" : "still"}
      data-phase={phase}
      style={style}
    >
      {drawBar("card", t("Card token bar"), bars.card)}
      {drawBar("cosmetic", t("Cosmetic token bar"), bars.cosmetic)}
      <RewardReveal items={rewards} testid={part("rewards")} />
      {phase === "flights"
        ? flights.map((flight, index) => {
            const course = courses?.[flight.key];
            if (course === undefined) return null;
            const at = {
              "--flight-x": `${String(course.x)}px`,
              "--flight-y": `${String(course.y)}px`,
              "--flight-dx": `${String(course.dx)}px`,
              "--flight-dy": `${String(course.dy)}px`,
              "--flight-delay": `${String(index * BAR_FLIGHT_STAGGER_MS)}ms`,
            } as CSSProperties;
            return (
              <span
                key={flight.key}
                className="bar-flight"
                data-testid={part(flight.key)}
                data-rarity={flight.rarity}
                aria-hidden="true"
                style={at}
              >
                {`+${String(flight.points)}`}
              </span>
            );
          })
        : null}
    </div>
  );
}
