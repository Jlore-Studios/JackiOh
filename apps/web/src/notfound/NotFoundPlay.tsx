// #3119, R3119000: the 404 panel's play (main.tsx `NotFound`). Three face-down cards deal in and each
// turns over into a way out; a lost card picked by the path says its line when tapped; the path the
// panel names glitches and settles. Presentation only (CLAUDE.md rule 7): it reads the bundle's
// catalog, its flavour and its voice lines, and sends nothing but a move to another screen.

import { useEffect, useRef, useState, type KeyboardEvent, type MouseEvent, type ReactElement } from "react";

import { CATALOG } from "@jackioh/cards";

import { VOICE_PRIORITY } from "../audio/constants.ts";
import { getAudioEngine } from "../audio/engine.ts";
import { CardBack } from "../cards/CardBack.tsx";
import { CardFace } from "../cards/CardFace.tsx";
import { readerName } from "../cards/chinese.ts";
import { flavourFor } from "../cards/flavour.ts";
import { GlitchBlob } from "../cards/GlitchBlob.tsx";
import { faceModel } from "../cards/model.ts";
import { ChoiceCard } from "../game/ChoiceCard.tsx";
import { t } from "../i18n/i18n.ts";
import { followInApp } from "../routes/nav.tsx";
import { reducedMotionNow } from "../settings/motion.ts";
import { GLITCH_SETTLE_STEP_MS, glitchedPath, lostCardFor, lostCardLine, WAYS_OUT, type WayOut } from "./lost.ts";

import "./notfound.css";

export const notFoundTestid = {
  path: "not-found-path",
  lost: "not-found-lost",
  ways: "not-found-ways",
  way: (key: WayOut["key"]): string => `not-found-way-${key}`,
} as const;

/** How long a tapped lost card stays lit after its line starts. */
const LOST_CARD_SPEAKS_MS = 1200;

/**
 * The path, glitched, settling one character at a time from the left; a pointer over it glitches it
 * again. The words are the path itself, so a reader, a copy and `textContent` get it as it is; the
 * glyphs are drawn over it by `notfound.css` from `data-glitch`. Settled at once under Reduce motion.
 */
export function GlitchPath({ path }: { path: string }): ReactElement {
  const length = [...path].length;
  const [settled, setSettled] = useState(() => (reducedMotionNow() ? length : 0));
  const settling = settled < length;
  useEffect(() => {
    if (!settling) return undefined;
    const timer = window.setInterval(() => {
      setSettled((count) => Math.min(count + 1, length));
    }, GLITCH_SETTLE_STEP_MS);
    return () => {
      window.clearInterval(timer);
    };
  }, [settling, length]);
  return (
    <code
      className="not-found-path"
      data-testid={notFoundTestid.path}
      data-settling={settling ? "true" : "false"}
      data-glitch={settling ? glitchedPath(path, settled) : undefined}
      onPointerEnter={() => {
        if (!settling && !reducedMotionNow()) setSettled(0);
      }}
    >
      {path}
    </code>
  );
}

/** Focus the keyboard brought (a tap's focus is not), read in a try/catch for an engine without `:focus-visible`. */
function focusedByKeyboard(element: Element): boolean {
  try {
    return element.matches(":focus-visible");
  } catch {
    return false;
  }
}

/** One way out: a face-down card that turns over (on a tap, a mouse over it or keyboard focus) into where it leads. */
function WayOutCard({ way, faceUp }: { way: WayOut; faceUp: boolean }): ReactElement {
  const [up, setUp] = useState(faceUp);
  // A mouse turns a card over as it arrives, a moment before its click: the click reads this, not a render's `up`.
  const upNow = useRef(faceUp);
  const turnOver = (): void => {
    upNow.current = true;
    setUp(true);
  };
  const label = t(way.label);
  const plain = (event: MouseEvent<HTMLAnchorElement>): boolean =>
    event.button === 0 && !event.metaKey && !event.ctrlKey && !event.shiftKey && !event.altKey;
  return (
    <a
      className="not-found-way"
      href={way.to}
      data-testid={notFoundTestid.way(way.key)}
      data-face={up ? "up" : "down"}
      aria-label={label}
      onPointerEnter={(event) => {
        if (event.pointerType === "mouse") turnOver();
      }}
      onFocus={(event) => {
        if (focusedByKeyboard(event.currentTarget)) turnOver();
      }}
      onClick={(event) => {
        // A plain tap on a card still face down turns it over; the next one follows it.
        if (!upNow.current && plain(event)) {
          event.preventDefault();
          turnOver();
        }
        followInApp(way.to)(event);
      }}
    >
      <span className="not-found-way__card">
        <span className="not-found-way__side not-found-way__side--back" aria-hidden="true">
          <CardBack />
        </span>
        <span className="not-found-way__side not-found-way__side--face" aria-hidden="true">
          <ChoiceCard seed={`not-found:${way.key}`} label={label} detail={t(way.detail)} />
        </span>
      </span>
    </a>
  );
}

/** The lost card: tapped, it says its line through the audio engine (muting is the engine's, R630). */
function LostCard({ path }: { path: string }): ReactElement | null {
  const defId = lostCardFor(path);
  const def = CATALOG[defId];
  const [speaking, setSpeaking] = useState(false);
  useEffect(() => {
    if (!speaking) return undefined;
    const timer = window.setTimeout(() => {
      setSpeaking(false);
    }, LOST_CARD_SPEAKS_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [speaking]);
  if (def === undefined) return null;
  const line = lostCardLine(defId);
  const flavour = flavourFor(defId)?.flavour;
  const speak = (): void => {
    if (line === null) return;
    const engine = getAudioEngine();
    engine.unlock();
    engine.playVoice(defId, line, 0, VOICE_PRIORITY.summon);
    setSpeaking(true);
  };
  return (
    <figure className="not-found-lost" data-testid={notFoundTestid.lost} data-def-id={defId}>
      <div
        className="not-found-lost__card"
        role="button"
        tabIndex={0}
        aria-label={t("Hear {name}", { name: readerName(def) })}
        data-speaking={speaking ? "true" : undefined}
        onClick={speak}
        onKeyDown={(event: KeyboardEvent<HTMLDivElement>) => {
          if (event.key !== "Enter" && event.key !== " ") return;
          event.preventDefault();
          speak();
        }}
      >
        <GlitchBlob />
        <CardFace face={faceModel({ defId, def, radiant: false })} />
      </div>
      {flavour === undefined ? null : <figcaption className="not-found-lost__flavour">{flavour}</figcaption>}
    </figure>
  );
}

/** The lost card beside the three ways out, dealt in; under Reduce motion they lie dealt and face up. */
export function NotFoundPlay({ path }: { path: string }): ReactElement {
  const [faceUp] = useState(() => reducedMotionNow());
  return (
    <div className="not-found-play">
      <LostCard path={path} />
      <nav className="not-found-ways" data-testid={notFoundTestid.ways} aria-label={t("Ways out")}>
        <p className="not-found-ways__hint">{t("Pick a card, any card.")}</p>
        <div className="not-found-ways__hand">
          {WAYS_OUT.map((way) => (
            <WayOutCard key={way.key} way={way} faceUp={faceUp} />
          ))}
        </div>
      </nav>
    </div>
  );
}
