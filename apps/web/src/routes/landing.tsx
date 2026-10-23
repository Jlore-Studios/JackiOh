// Landing's front door (B37-B39): a fan of real cards and one call to action; CSS stays scoped to
// `.landing` so it cannot affect the board layout. After `GATE_SLOW_NOTICE_SECONDS`, local session
// state avoids flashing Sign in while the account read is slow. R765 shows an active online game.

import { Suspense, lazy, useEffect, useRef, useState, type CSSProperties, type ReactElement } from "react";

import { GATE_SLOW_NOTICE_SECONDS } from "@jackioh/server-config";

import type { CardDef } from "@jackioh/shared";

import { landingFanCardTestid, landingTestid } from "../auth/testids.ts";
import { CardBack } from "../cards/CardBack.tsx";
import { CardFace } from "../cards/CardFace.tsx";
import { useInspectTrigger } from "../cards/inspect/useInspectTrigger.tsx";
import { faceModel } from "../cards/model.ts";
import { readerName } from "../cards/chinese.ts";
import { useCardSettings } from "../cards/settings.ts";
import JlorePill from "../economy/JlorePill.tsx";
import LanguageButton from "../i18n/LanguageButton.tsx";
import { t } from "../i18n/i18n.ts";
import { useAccount, type Account } from "../net/gate.ts";
import { useLiveGame } from "../net/liveGame.ts";
import { useSettingsAccountSync } from "../settings/accountSync.ts";
import { paths } from "../net/navigate.ts";
import { readSession } from "../net/session.ts";
import { SettingsButton } from "../settings/index.ts";
import { useSetting } from "../settings/store.ts";
import { ROTATION_INTERVAL_MS, ROTATION_MIN_GAMES, ROTATION_SWAP_MS } from "../stats/config.ts";
import { PlayerStatsCard } from "../stats/PlayerStatsCard.tsx";
import { readPlayerStats, usePlayerStats } from "../stats/store.ts";
import {
  EVEN,
  dealLandingFan,
  fanPool,
  favouringFullArt,
  featureWeight,
  rotateFan,
  rotationPool,
  type FanFace,
  type RandomSource,
} from "./landingFan.ts";
import { LiveGameBanner } from "./GameBanner.tsx";
import { followInApp } from "./nav.tsx";
import { SiteFooter } from "./SiteFooter.tsx";

import "../auth/tavern.css";
import "./landing.css";

const DEV_ONLY = import.meta.env.MODE !== "production";

/** R639: the card's detail dialog, loaded when a fan card is first opened, so the first screen stays light. */
const CardDetail = lazy(() => import("../cards/inspect/CardDetail.tsx").then((module) => ({ default: module.CardDetail })));

const REDUCED_MOTION_QUERY = "(prefers-reduced-motion: reduce)";

type LandingMotion = "full" | "reduced";
type LandingAccountState = "loading" | "anonymous" | "signed-in";

function prefersReducedMotion(): boolean {
  if (typeof window.matchMedia !== "function") return false;
  return window.matchMedia(REDUCED_MOTION_QUERY).matches;
}

/** `data-motion` exposes the media query or setting to tests; CSS also honours both directly. */
function useMotion(): LandingMotion {
  const [reduced, setReduced] = useState(prefersReducedMotion);
  const settingReduces = useSetting("reduceMotion");

  useEffect(() => {
    if (typeof window.matchMedia !== "function") return undefined;
    const query = window.matchMedia(REDUCED_MOTION_QUERY);
    const onChange = (): void => {
      setReduced(query.matches);
    };
    onChange();
    if (typeof query.addEventListener !== "function") return undefined;
    query.addEventListener("change", onChange);
    return () => {
      query.removeEventListener("change", onChange);
    };
  }, []);

  return reduced || settingReduces ? "reduced" : "full";
}

function accountState(account: Account): LandingAccountState {
  if (account.kind === "loading") return "loading";
  if (account.kind === "ready") return "signed-in";
  // A failed read does not override a local session: offer Account, not Sign in.
  if (account.kind === "error" && readSession() !== null) return "signed-in";
  return "anonymous";
}

// The hand of cards

/** R374 deals four real faces and a back; R639 makes each face a readable control and holds the fan while it is read. R704 owns its animation. */
function CardFan({
  hand,
  onOpen,
  onHold,
}: {
  hand: readonly FanFace[];
  onOpen: (def: CardDef) => void;
  onHold: (held: boolean) => void;
}): ReactElement {
  return (
    <div
      className="landing-fan"
      data-testid={landingTestid.fan}
      style={{ "--fan-swap": `${String(ROTATION_SWAP_MS)}ms` } as CSSProperties}
      onMouseEnter={() => {
        onHold(true);
      }}
      onMouseLeave={() => {
        onHold(false);
      }}
      onFocus={() => {
        onHold(true);
      }}
      onBlur={() => {
        onHold(false);
      }}
    >
      {/* R704: a slot is keyed by its place, not its card, so it stays mounted through a swap and the card going out can fizzle away in it while the new one fades in. */}
      {hand.map((face, index) => (
        <LandingFanSlot key={index} face={face} index={index} onOpen={onOpen} onHold={onHold} />
      ))}
      <div className="landing-fan-slot" aria-hidden="true">
        <div
          className="landing-fan-card landing-fan-card--back"
          data-testid={landingFanCardTestid(hand.length)}
          data-face="down"
        >
          <CardBack />
        </div>
      </div>
    </div>
  );
}

/** R704 keeps the outgoing card as a decorative, inert ghost for `ROTATION_SWAP_MS`. */
function LandingFanSlot({ face, index, onOpen, onHold }: {
  face: FanFace;
  index: number;
  onOpen: (def: CardDef) => void;
  onHold: (held: boolean) => void;
}): ReactElement {
  // Rendering notices a new card so its ghost and replacement first paint together.
  const [shown, setShown] = useState(face);
  const [leaving, setLeaving] = useState<FanFace | null>(null);
  if (shown.def.id !== face.def.id) {
    setShown(face);
    setLeaving(shown);
  }
  useEffect(() => {
    if (leaving === null) return undefined;
    const timer = window.setTimeout(() => {
      setLeaving(null);
    }, ROTATION_SWAP_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [leaving]);
  return (
    <div className="landing-fan-slot">
      <LandingFanCard key={face.def.id} def={face.def} radiant={face.radiant} index={index} swapped={leaving !== null} onOpen={onOpen} onHold={onHold} />
      {leaving === null ? null : (
        <div
          key={leaving.def.id}
          className="landing-fan-card landing-fan-card--leaving"
          data-testid={landingTestid.fanLeaving}
          data-def-id={leaving.def.id}
          data-radiant={leaving.radiant ? "true" : undefined}
          aria-hidden="true"
        >
          <CardFace face={faceModel({ defId: leaving.def.id, def: leaving.def, radiant: leaving.radiant })} />
        </div>
      )}
    </div>
  );
}

/** A touch hold previews a face and holds the fan; mouse clicks still open its detail dialog. */
function LandingFanCard({
  def,
  radiant,
  index,
  swapped,
  onOpen,
  onHold,
}: {
  def: CardDef;
  radiant: boolean;
  index: number;
  swapped: boolean;
  onOpen: (def: CardDef) => void;
  onHold: (held: boolean) => void;
}): ReactElement {
  const face = faceModel({ defId: def.id, def, radiant });
  const inspect = useInspectTrigger(
    { key: `landing-fan-${def.id}`, face },
    { hover: false, touchHold: "preview", prefer: "above" },
  );
  // R704 fixes this mount's opening-deal or swap apparition.
  const [entry] = useState<"deal" | "swap">(swapped ? "swap" : "deal");
  const held = inspect.open !== null;
  useEffect(() => {
    if (!held) return undefined;
    onHold(true);
    return () => {
      onHold(false);
    };
  }, [held, onHold]);
  return (
    <>
      <div
        className="landing-fan-card landing-fan-card--face"
        data-testid={landingFanCardTestid(index)}
        data-face="up"
        data-entry={entry}
        data-def-id={def.id}
        data-radiant={radiant ? "true" : undefined}
        role="button"
        tabIndex={0}
        aria-label={t("Read {name}", { name: readerName(def) })}
        {...inspect.handlers}
        onClick={() => {
          onOpen(def);
        }}
        onKeyDown={(event) => {
          if (event.key !== "Enter" && event.key !== " ") return;
          event.preventDefault();
          onOpen(def);
        }}
      >
        <CardFace face={face} />
      </div>
      {inspect.overlay}
    </>
  );
}

const EMBER_COUNT = 10;

function Backdrop(): ReactElement {
  return (
    <div className="landing-backdrop" aria-hidden="true">
      <div className="landing-hearth" />
      <div className="landing-table" />
      <div className="landing-embers">
        {Array.from({ length: EMBER_COUNT }, (_, index) => (
          <span key={index} className="landing-ember" />
        ))}
      </div>
      <div className="landing-vignette" />
    </div>
  );
}

// The corner slot and the calls to action

/** True once the account has been loading for `GATE_SLOW_NOTICE_SECONDS`. */
function useSlowLoading(account: Account): boolean {
  const loading = account.kind === "loading";
  const [slow, setSlow] = useState(false);
  useEffect(() => {
    if (!loading) {
      setSlow(false);
      return;
    }
    const timer = window.setTimeout(() => {
      setSlow(true);
    }, GATE_SLOW_NOTICE_SECONDS * 1000);
    return () => {
      window.clearTimeout(timer);
    };
  }, [loading]);
  return loading && slow;
}

function Corner({ account }: { account: Account }): ReactElement | null {
  const slow = useSlowLoading(account);
  if (account.kind === "loading" && !slow) return null;
  // A slow or failed read with a local session still offers Account.
  const holdsSession = (account.kind === "loading" || account.kind === "error") && readSession() !== null;
  if (account.kind === "ready" || holdsSession) {
    return (
      <a
        className="landing-corner-link"
        href={paths.account}
        data-testid={landingTestid.account}
        onClick={followInApp(paths.account)}
      >
        {t("Account")}
      </a>
    );
  }
  return (
    <a
      className="landing-corner-link"
      href={paths.login}
      data-testid={landingTestid.signIn}
      onClick={followInApp(paths.login)}
    >
      {t("Sign in")}
    </a>
  );
}

function Actions(): ReactElement {
  // #1577, R1578000: the three doors. Play leads to the Play hub, which holds every way to play,
  // online first; the invite-only sentence lives on `/invite` and `/play`, where the player decides.
  return (
    <nav className="landing-ctas" aria-label={t("Play")}>
      <a
        className="landing-cta landing-cta--primary"
        href={paths.play}
        data-testid={landingTestid.play}
        onClick={followInApp(paths.play)}
      >
        {t("Play")}
      </a>
      <p className="landing-cta-note">{t("Online first: every way to play, from one screen.")}</p>
      <div className="landing-ctas-secondary">
        <a
          className="landing-cta landing-cta--secondary"
          href={paths.collection}
          data-testid={landingTestid.collection}
          onClick={followInApp(paths.collection)}
        >
          {t("Collection")}
        </a>
        <a
          className="landing-cta landing-cta--secondary"
          href={paths.shop}
          data-testid={landingTestid.shop}
          onClick={followInApp(paths.shop)}
        >
          {t("Shop")}
        </a>
      </div>
      {DEV_ONLY ? (
        // Production has no hotseat route; it boots from this full page load's query.
        <a
          className="landing-dev-link"
          href={`${paths.hotseat}?seed=42&a=first20&b=first20`}
          data-testid={landingTestid.hotseat}
        >
          {t("Hotseat: both seats on this device")}
        </a>
      ) : null}
    </nav>
  );
}

// The page

export type LandingRouteProps = {
  /** R374: where the fan's deal draws its randomness; `Math.random` on the page, a seeded source in tests. */
  random?: RandomSource;
};

export default function LandingRoute({ random = Math.random }: LandingRouteProps = {}): ReactElement {
  const account = useAccount();
  // R634: the first screen most visits see keeps an active account's settings level with this device's.
  useSettingsAccountSync(account);
  // R765: the player's live online game, for the banner at the top.
  const liveGame = useLiveGame(account);
  const motion = useMotion();
  // R374 deals once per mount; R639 uses all shipped cards after enough games; R704 rotates that pool.
  // R1017002: from the cards that print whole on a fan card, a hero's full-art card favoured.
  const fullPool = usePlayerStats().games >= ROTATION_MIN_GAMES;
  const { fullArt } = useCardSettings();
  const [hand, setHand] = useState(() =>
    readPlayerStats().games >= ROTATION_MIN_GAMES
      ? dealLandingFan(random, rotationPool(fullArt), favouringFullArt(featureWeight, fullArt))
      : dealLandingFan(random, fanPool(fullArt), favouringFullArt(EVEN, fullArt)),
  );
  const [open, setOpen] = useState<CardDef | null>(null);
  const [held, setHeld] = useState(false);
  const nextSlot = useRef(0);

  // R639/R704 rotate left to right, but never while read, hidden, or under reduced motion.
  useEffect(() => {
    if (motion === "reduced" || open !== null || held) return undefined;
    const timer = window.setInterval(() => {
      if (document.visibilityState === "hidden") return;
      const at = nextSlot.current;
      nextSlot.current += 1;
      setHand((current) =>
        fullPool
          ? rotateFan(current, at, random, rotationPool(fullArt), favouringFullArt(featureWeight, fullArt))
          : rotateFan(current, at, random, fanPool(fullArt), favouringFullArt(EVEN, fullArt)),
      );
    }, ROTATION_INTERVAL_MS);
    return () => {
      window.clearInterval(timer);
    };
  }, [fullPool, fullArt, motion, open, held, random]);

  return (
    <div
      className="landing tavern"
      data-testid={landingTestid.root}
      data-motion={motion}
      data-account={accountState(account)}
    >
      <section className="landing-hero" aria-labelledby="landing-title">
        <Backdrop />

        <header className="landing-topbar">
          {/* The gear beside the account pill, as every other screen keeps it top right: mute,
              volume and reduced motion are reachable from the first screen too. A signed-in
              account's Jlore pill leads the language button and the gear, as in every top bar. */}
          <div className="landing-corner">
            <Corner account={account} />
            <JlorePill />
            <LanguageButton placement="nav" />
            <SettingsButton placement="nav" />
          </div>
        </header>

        {liveGame === null ? null : (
          <div className="landing-banners">
            <LiveGameBanner game={liveGame} />
          </div>
        )}

        <div className="landing-hero-inner">
          <div className="landing-title-block">
            <h1 id="landing-title" className="landing-wordmark">
              JackiOh
            </h1>
          </div>

          <CardFan hand={hand} onOpen={setOpen} onHold={setHeld} />

          <Actions />
        </div>
      </section>

      {/* R1584000: the stats and the footer are one screen below the hero, when there are stats. */}
      <div className="landing-floor">
        <PlayerStatsCard reduced={motion === "reduced"} />

        {open === null ? null : (
          <Suspense fallback={null}>
            <CardDetail
              def={open}
              onClose={() => {
                setOpen(null);
              }}
            />
          </Suspense>
        )}

        <SiteFooter />
      </div>
    </div>
  );
}
