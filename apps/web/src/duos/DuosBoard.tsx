// The Duos co-op board (#936 S8, B61–B64): Game.tsx for the seat's own duel, and around it the life
// strip, the teammate strip, the portal, the pass control and the pings.
//
// GAME.TSX STAYS THE ONE BOARD (CLAUDE.md). The seat's own duel is `<Game view={view.own}>` with the
// duel moves `legal` lists unwrapped from their `{kind: "duel"}` envelopes, and every move it reports
// goes back out wrapped in one. The portal (`duos-portal`) flips the board to `<Game view={view.teammate}>`
// — the teammate's own view, their hand and face-down cards included, which is the team's to see
// (B23) — with no legal action and a no-op `onAction` (`replay`, so it raises no prompt and lifts no
// card): read-only (B63). Each has its own key, so neither board animates the other duel's events.
//
// CLAUDE.md rule 7: nothing here decides a rule. The pass control is live exactly when `legal` lists
// that card's `pass` (B19), the concede offer when it lists `concede` (B11), and a duel's own concede
// is never sent: the board's Concede means the team's (B10), so it goes out as `{kind: "concede"}`.
// Game's R345 preference is pinned off, as every Duos duel already holds it (B9), so the board never
// asks for it. Draw offers are refused in Duos (B10): `legal` never lists one, so those controls stay
// dark, and duos.css hides them with the board's own Concede, which the rail's offer stands in for.
//
// PASSING (B62): the pass control on a hand card of the seat's own (`CardActions.tsx`), or a hand card
// dragged onto the portal, sends that card's listed `pass`; anything not listed sends nothing.
//
// PINGS (B64): the ping wheel on a hero, zone or hand card of either board sends a ping (`onPing`); a
// ping marks its place (`duos-ping-marker`) for PING_SHOW_MS on whichever board shows its duel, the
// teammate's ping plays its sound once, and one on the board not showing flashes the portal.

import {
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactElement,
} from "react";

import type { ActionBody } from "@jackioh/shared";
import { PING_SHOW_MS } from "@jackioh/server-config";

import type { DuosActionBody } from "../wire/generated/DuosActionBody.ts";
import type { DuosResult } from "../wire/generated/DuosResult.ts";
import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import type { Team } from "../wire/generated/Team.ts";
import Game from "../game/Game.tsx";
import { DRAG_THRESHOLD_PX } from "../game/drag/model.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import type { PingKind, PingTarget, ReceivedPing } from "../net/duos.ts";
import { CardActions, type PingMarker } from "./CardActions.tsx";
import { DuosChromeContext } from "./context.ts";
import { LifeStrip, TeammateStrip } from "./LifeStrip.tsx";
import { PING_LABEL } from "./pings.ts";
import { duelOf, seatLabel, teamOf, teammateOf } from "./seats.ts";
import { playPingSound } from "./sounds.ts";
import { spotOfElement, spotOfTarget } from "./spots.ts";
import { duosTestid } from "./testids.ts";
import "./duos.css";

export type { ReceivedPing } from "../net/duos.ts";

export type DuosBoardProps = {
  view: DuosView;
  legal: readonly DuosActionBody[];
  onAction: (body: DuosActionBody) => void;
  pings: readonly ReceivedPing[];
  onPing: (ping: PingKind, target: PingTarget) => void;
  error?: string | null;
};

/** Which duel the board shows: the seat's own, or the teammate's through the portal. */
type Showing = "own" | "teammate";

/** What the teammate's board is offered: nothing (B63). */
const NOTHING: readonly ActionBody[] = [];

function ignore(): void {}

/**
 * `Date.now()` as of this render, re-rendered when the next live ping's PING_SHOW_MS runs out, so a
 * marker and the portal's flash go when their time is up with nothing else happening.
 */
function usePingNow(pings: readonly ReceivedPing[]): number {
  const [tick, setTick] = useState(0);
  const now = Date.now();
  useEffect(() => {
    const current = Date.now();
    const ends = pings.map((ping) => ping.at + PING_SHOW_MS).filter((end) => end > current);
    if (ends.length === 0) return undefined;
    const timer = window.setTimeout(() => {
      setTick((count) => count + 1);
    }, Math.min(...ends) - current);
    return () => {
      window.clearTimeout(timer);
    };
  }, [pings, tick]);
  return now;
}

/** The teammate's pings play their sound once each, as they land (B64); the seat's own are silent. */
function usePingSounds(pings: readonly ReceivedPing[], seat: DuosSeat): void {
  const heard = useRef(new Set<ReceivedPing>());
  useEffect(() => {
    const seen = heard.current;
    for (const ping of pings) {
      if (seen.has(ping)) continue;
      seen.add(ping);
      if (ping.from !== seat && Date.now() - ping.at < PING_SHOW_MS) playPingSound(ping.ping);
    }
    for (const old of [...seen]) {
      if (!pings.includes(old)) seen.delete(old);
    }
  }, [pings, seat]);
}

/** A Duos result in a player's words, from this seat's team's side. */
function resultWords(result: DuosResult, team: Team): { outcome: "win" | "loss" | "draw"; heading: string; reason: string } {
  const outcome = result.outcome === "draw" ? "draw" : result.winner === team ? "win" : "loss";
  const heading = outcome === "win" ? t("Your team won") : outcome === "loss" ? t("Your team lost") : t("Draw");
  switch (result.reason) {
    case "poolEmptied":
      return {
        outcome,
        heading,
        reason:
          outcome === "draw"
            ? t("Both pools ran out together.")
            : outcome === "win"
              ? t("The enemy team's pool ran out.")
              : t("Your team's pool ran out."),
      };
    case "concede":
      return { outcome, heading, reason: outcome === "win" ? t("The enemy team conceded.") : t("Your team conceded.") };
    case "turnCap":
      return { outcome, heading, reason: t("The game reached its turn limit.") };
    case "duelResult":
      return { outcome, heading, reason: t("One of the duels decided the game.") };
    case "ceiling":
      return { outcome, heading, reason: t("The game ran out of time.") };
    default:
      return { outcome, heading, reason: "" };
  }
}

/** B11: the team's concede, offered by one seat and agreed by the other; agreeing asks twice. */
function ConcedeControl({ view, live, onConcede }: { view: DuosView; live: boolean; onConcede: () => void }): ReactElement | null {
  const [armedFor, setArmedFor] = useState<DuosSeat | null>(null);
  if (view.result !== null) return null;
  const offeredBy = view.concedeOfferedBy;
  if (offeredBy === view.seat) {
    return (
      <p className="duos-concede__status" data-testid={duosTestid.concedeStatus} role="status">
        {t("You offered to concede. If your teammate agrees, the game ends.")}
      </p>
    );
  }
  const offered = offeredBy !== null;
  const armed = offered && armedFor === offeredBy;
  return (
    <div className="duos-concede">
      {offered ? (
        <p className="duos-concede__status" data-testid={duosTestid.concedeStatus} role="status">
          {t("Your teammate offers to concede.")}
        </p>
      ) : null}
      <button
        type="button"
        className="duos-concede__button"
        data-testid={duosTestid.concede}
        data-armed={armed ? "true" : undefined}
        disabled={!live}
        onClick={() => {
          if (offered && !armed) {
            setArmedFor(offeredBy);
            return;
          }
          setArmedFor(null);
          onConcede();
        }}
      >
        {!offered ? t("Offer to concede") : armed ? t("Confirm: concede the game") : t("Concede with your teammate")}
      </button>
    </div>
  );
}

function ResultPanel({
  view,
  onShowBoard,
  onLeave,
}: {
  view: DuosView;
  onShowBoard: () => void;
  onLeave: (() => void) | undefined;
}): ReactElement | null {
  if (view.result === null) return null;
  const words = resultWords(view.result, teamOf(view.seat));
  return (
    <div className="duos-result" data-testid={duosTestid.result} data-outcome={words.outcome} role="dialog" aria-label={words.heading}>
      <h2 className="duos-result__heading">{words.heading}</h2>
      {words.reason === "" ? null : <p className="duos-result__reason">{words.reason}</p>}
      <div className="row duos-result__actions">
        <button type="button" data-testid={duosTestid.resultBoard} onClick={onShowBoard}>
          {t("View the board")}
        </button>
        {onLeave === undefined ? null : (
          <button type="button" className="button-primary" data-testid={duosTestid.resultBack} onClick={onLeave}>
            {t("Back to Play together")}
          </button>
        )}
      </div>
    </div>
  );
}

export function DuosBoard({ view, legal, onAction, pings, onPing, error }: DuosBoardProps): ReactElement {
  useLocale();
  const chrome = useContext(DuosChromeContext);
  const container = useRef<HTMLDivElement>(null);
  const portal = useRef<HTMLButtonElement>(null);
  const [showing, setShowing] = useState<Showing>("own");
  const [boardAfterResult, setBoardAfterResult] = useState(false);

  const seat = view.seat;
  const mate = teammateOf(seat);
  const slot = duelOf(seat);
  const shownDuel = showing === "own" ? slot : duelOf(mate);
  const shownView = showing === "own" ? view.own : view.teammate;
  const over = view.result !== null;

  // The duel moves `legal` lists, out of their envelopes, for the own board (rule 7).
  const duelLegal = useMemo(() => legal.flatMap((body) => (body.kind === "duel" ? [body.body] : [])), [legal]);
  const passLegal = useMemo(
    () => new Set(legal.flatMap((body) => (body.kind === "pass" ? [body.instanceId] : []))),
    [legal],
  );
  const concedeLegal = legal.some((body) => body.kind === "concede");

  const sendDuel = useCallback(
    (body: ActionBody) => {
      // B10: a duel's own concede is refused in Duos; the board's Concede means the team's.
      onAction(body.type === "concede" ? { kind: "concede" } : { kind: "duel", body });
    },
    [onAction],
  );
  const sendPass = useCallback(
    (instanceId: string) => {
      const body = legal.find((entry) => entry.kind === "pass" && entry.instanceId === instanceId);
      if (body !== undefined) onAction(body);
    },
    [legal, onAction],
  );
  const sendConcede = useCallback(() => {
    const body = legal.find((entry) => entry.kind === "concede");
    if (body !== undefined) onAction(body);
  }, [legal, onAction]);

  // B62: a hand card of the seat's own dropped on the portal is passed, when `legal` lists it.
  const passLatest = useRef(sendPass);
  passLatest.current = sendPass;
  useEffect(() => {
    if (showing !== "own") return undefined;
    let press: { pointerId: number; instanceId: string; x: number; y: number } | null = null;
    const onDown = (event: PointerEvent): void => {
      press = null;
      const host = container.current;
      const target = event.target;
      if (host === null || !(target instanceof Element) || !host.contains(target)) return;
      const spot = spotOfElement(target);
      if (spot?.at !== "hand") return;
      press = { pointerId: event.pointerId, instanceId: spot.instanceId, x: event.clientX, y: event.clientY };
    };
    const onUp = (event: PointerEvent): void => {
      const held = press;
      press = null;
      if (held === null || held.pointerId !== event.pointerId) return;
      if (Math.hypot(event.clientX - held.x, event.clientY - held.y) < DRAG_THRESHOLD_PX) return;
      if (typeof document.elementFromPoint !== "function") return;
      // The dragged card holds the pointer's capture, so the drop is read from where it was let go.
      const under = document.elementFromPoint(event.clientX, event.clientY);
      if (under === null || portal.current === null || !portal.current.contains(under)) return;
      passLatest.current(held.instanceId);
    };
    const onCancel = (): void => {
      press = null;
    };
    window.addEventListener("pointerdown", onDown, true);
    window.addEventListener("pointerup", onUp, true);
    window.addEventListener("pointercancel", onCancel, true);
    return () => {
      window.removeEventListener("pointerdown", onDown, true);
      window.removeEventListener("pointerup", onUp, true);
      window.removeEventListener("pointercancel", onCancel, true);
    };
  }, [showing]);

  // B64: the pings still up, marked on the board that shows their duel.
  usePingSounds(pings, seat);
  const now = usePingNow(pings);
  const livePings = pings.filter((ping) => now - ping.at < PING_SHOW_MS);
  const markers: PingMarker[] = livePings.flatMap((ping, index) => {
    const spot = spotOfTarget(ping.target, shownView, shownDuel);
    return spot === null
      ? []
      : [{ key: `${ping.from}-${String(ping.at)}-${String(index)}`, spot, ping: ping.ping, from: ping.from, mine: ping.from === seat }];
  });
  const flash = livePings.some((ping) => ping.from !== seat && ping.target.duel !== shownDuel);
  const heard = [...livePings].reverse().find((ping) => ping.from !== seat);

  const ended = view.team.ended[slot] === true;
  const ourTurn = view.own.active === view.own.viewer;

  const rail = (
    <div className="duos-rail">
      {chrome?.back}
      <p className="duos-round" data-testid={duosTestid.round} data-round={view.round}>
        {view.round === 0 ? t("Mulligan") : t("Round {n}", { n: view.round })}
        {view.round === 0 || over ? null : <> · {ourTurn ? t("Your team's turn") : t("Enemy team's turn")}</>}
        {" · "}
        {t("You are {seat}", { seat: seatLabel(seat) })}
      </p>
      <LifeStrip view={view} />
      <TeammateStrip view={view} link={chrome?.teammate ?? null} />
      <div className="duos-rail__row">
        <button
          ref={portal}
          type="button"
          className="duos-portal"
          data-testid={duosTestid.portal}
          data-showing={showing}
          data-flash={flash ? "ping" : undefined}
          aria-pressed={showing === "teammate"}
          onClick={() => {
            setShowing((current) => (current === "own" ? "teammate" : "own"));
          }}
        >
          {showing === "own" ? t("Teammate's board") : t("Back to your board")}
        </button>
        <ConcedeControl view={view} live={concedeLegal} onConcede={sendConcede} />
      </div>
      {!over && ourTurn && ended ? (
        <p className="duos-waiting" data-testid={duosTestid.waiting} role="status">
          {t("Waiting for your teammate")}
        </p>
      ) : null}
    </div>
  );

  return (
    <div
      className="duos-board"
      ref={container}
      data-testid={duosTestid.board}
      data-seat={seat}
      data-showing={showing}
      data-duel={shownDuel}
    >
      {showing === "own" ? (
        <Game
          key="own"
          view={view.own}
          legal={duelLegal}
          onAction={sendDuel}
          error={error ?? null}
          autoEndTurn={false}
          resultForm="chip"
          turnClock={chrome?.clock}
          matchStatus={chrome?.status}
          boardRail={rail}
        />
      ) : (
        <Game
          key="teammate"
          view={view.teammate}
          legal={NOTHING}
          onAction={ignore}
          replay
          resultForm="chip"
          turnClock={chrome?.clock}
          matchStatus={chrome?.status}
          boardRail={rail}
        />
      )}
      <CardActions
        key={showing}
        container={container}
        view={shownView}
        duel={shownDuel}
        enabled={!over}
        pass={showing === "own" ? { legal: passLegal, cost: view.team.passCost, left: view.team.passesLeft[slot] ?? 0 } : null}
        onPass={sendPass}
        onPing={onPing}
        markers={markers}
      />
      <p className="duos-sr-only" role="status" aria-live="polite">
        {heard === undefined ? "" : t("{seat}: {ping}", { seat: seatLabel(heard.from), ping: t(PING_LABEL[heard.ping]) })}
      </p>
      {over && !boardAfterResult ? (
        <ResultPanel
          view={view}
          onShowBoard={() => {
            setBoardAfterResult(true);
          }}
          onLeave={chrome?.onLeave}
        />
      ) : null}
    </div>
  );
}

export default DuosBoard;
