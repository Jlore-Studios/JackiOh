// The Duos co-op board (#936 S8, B61–B64; R2095000): Game.tsx for the seat's own duel, and around
// it the life strip, the teammate strip, the portal, the Both boards toggle, the pass control and
// the pings.
//
// GAME.TSX STAYS THE ONE BOARD (CLAUDE.md). The seat's own duel is `<Game view={view.own}>` with the
// duel moves `legal` lists unwrapped from their `{kind: "duel"}` envelopes, and every move it reports
// goes back out wrapped in one. The portal (`duos-portal`) flips the board to `<Game view={view.teammate}>`
// — the teammate's own view, their hand and face-down cards included, which is the team's to see
// (B23) — with no legal action and a no-op `onAction` (`replay`, so it raises no prompt and lifts no
// card): read-only (B63). Each has its own key, so neither board animates the other duel's events.
// On a wide screen R2095000's toggle (`duos-both`) shows that same read-only board beside the own
// one instead, silent (Game's `silent`, so two mounted boards play one duel's cues), in
// `duos-board-teammate` next to `duos-board-own`; the portal stays mounted and CSS hides it there.
// On a narrowed screen CSS hides the toggle and draws only the board the portal names, which
// carries the rail as the single board does, so the visible portal flips again.
//
// CLAUDE.md rule 7: nothing here decides a rule. The pass control is live exactly when `legal` lists
// that card's `pass` (B19), the concede offer when it lists `concede` (B11), and a duel's own concede
// is never sent: the board's Concede means the team's (B10), so it goes out as `{kind: "concede"}`.
// Game's R345 preference is pinned off, as every Duos duel already holds it (B9), so the board never
// asks for it. Draw offers are refused in Duos (B10): `legal` never lists one, so those controls stay
// dark, and duos.css hides them with the board's own Concede, which the rail's offer stands in for.
//
// PASSING (B62; R2095000, #2619): the pass control on a hand card of the seat's own (`CardActions.tsx`)
// sends that card's listed `pass`; anything not listed sends nothing. It is the only way to pass: a
// hand card dropped on the portal or the teammate's board passes nothing. The Trade control
// (R1838000) rides the same bar on a Tradable one, in place of Game's own Trade bar.
//
// PINGS (B64): the ping wheel on a hero, zone or hand card of either board sends a ping (`onPing`); a
// ping marks its place (`duos-ping-marker`) for PING_SHOW_MS on whichever board shows its duel, the
// teammate's ping plays its sound once, and one on the board not showing flashes the portal.
//
// REPLAY (#965): `replay` draws a finished game's step for `DuosReplay.tsx`. The own duel's Game is
// `replay` too, with no legal action and a no-op `onAction`, as the teammate's already is; no card
// bar, concede, waiting line or result panel is drawn, the rail's round line names the team's result,
// and the portal still flips.
//
// THE RESULT (#1755): the team's result panel takes focus on its first way on as it comes in, and
// "View the board" folds it to the rail's "Result", which takes the focus and opens it again, its
// XP reveal and replay door with it, as the 1v1 result folds to its chip (Result.tsx).
//
// PLAY AGAIN (R2092000): the panel draws the chrome's `playAgain` and decides nothing: the route's
// last status read and the last offer's refusal pick the line, and the button only sends the offer.
// A replay or /dev/duos hangs no `playAgain`, so draws no button.

import {
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  type ReactElement,
} from "react";

import type { ActionBody, PlayerView } from "@jackioh/shared";
import { PING_SHOW_MS } from "@jackioh/server-config";

import type { DuosActionBody } from "../wire/generated/DuosActionBody.ts";
import type { DuosResult } from "../wire/generated/DuosResult.ts";
import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import type { Team } from "../wire/generated/Team.ts";
import { announceWallet } from "../economy/wallet.ts";
import XpReveal from "../economy/XpReveal.tsx";
import type { GameEconomy } from "../economy/xp.ts";
import Game from "../game/Game.tsx";
import { ModifierList } from "../game/ModifierList.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import type { PingKind, PingTarget, ReceivedPing } from "../net/duos.ts";
import { useBoardBusy } from "../replay/ReplayPlayer.tsx";
import { tradeOfferFor } from "../game/TradeControl.tsx";
import { CardActions, type PingMarker } from "./CardActions.tsx";
import { DUOS_BOTH_KEY } from "./config.ts";
import { DuosChromeContext, type DuosPlayAgain } from "./context.ts";
import { LifeStrip, TeammateStrip } from "./LifeStrip.tsx";
import { PING_LABEL } from "./pings.ts";
import { duelOf, seatLabel, teamOf, teammateOf } from "./seats.ts";
import { playPingSound } from "./sounds.ts";
import { spotOfTarget } from "./spots.ts";
import { duosTestid } from "./testids.ts";
import "./duos.css";

export type DuosBoardProps = {
  view: DuosView;
  legal: readonly DuosActionBody[];
  onAction: (body: DuosActionBody) => void;
  pings: readonly ReceivedPing[];
  onPing: (ping: PingKind, target: PingTarget) => void;
  error?: string | null;
  /** R1483000: the server's per-seat flag for whether the daily cap held this seat's pay back; dev and replay pass false. */
  jloreCapped: boolean;
  /** #1664: what the result paid this seat's person, from the server's last view; dev, replay and a game it paid nothing pass none. */
  economy?: GameEconomy | null;
  /** #965: a finished game's step to look at (`DuosReplay.tsx`): nothing is offered, and nothing is sent. */
  replay?: boolean;
};

/** Which duel the board shows: the seat's own, or the teammate's through the portal. */
type Showing = "own" | "teammate";

/** What the teammate's board is offered: nothing (B63). */
const NOTHING: readonly ActionBody[] = [];

function ignore(): void {}

/**
 * R2095000: the Both boards choice, kept per device. `"1"` means on, anything else (nothing
 * stored included) off, read and written in `try`/`catch` exactly as `CoopPanel.tsx` keeps its
 * room: blocked storage keeps the choice for the page's life.
 */
function readBoth(): boolean {
  try {
    return window.localStorage.getItem(DUOS_BOTH_KEY) === "1";
  } catch {
    return false;
  }
}

function storeBoth(both: boolean): void {
  try {
    if (both) window.localStorage.setItem(DUOS_BOTH_KEY, "1");
    else window.localStorage.removeItem(DUOS_BOTH_KEY);
  } catch {
    // Blocked storage: the choice lasts as long as the page does.
  }
}

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
    case "voided":
      return { outcome, heading, reason: t("A Glitch voided one of the duels.") };
    default:
      return { outcome, heading, reason: "" };
  }
}

/** R2092000: the Play again line, from the route's last read and the last offer's refusal. */
function againLine(again: DuosPlayAgain | undefined): { words: string; alert: boolean } | null {
  if (again === undefined) return null;
  const status = again.status;
  if (status !== null && !status.teammateHere) return { words: t("Your teammate has left"), alert: false };
  if (again.refusal !== null) return { words: t(again.refusal), alert: true };
  if (status === null || status.matchId !== null) return null;
  if (status.youOffered) return { words: t("Waiting for your teammate…"), alert: false };
  if (status.teammateOffered) return { words: t("Your teammate wants to play again"), alert: false };
  return null;
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
  jloreCapped,
  economy,
  onShowBoard,
  onWatchReplay,
  onLeave,
  leaveRoute,
  playAgain,
}: {
  view: DuosView;
  jloreCapped: boolean;
  economy: GameEconomy | null;
  onShowBoard: () => void;
  onWatchReplay: (() => void) | undefined;
  onLeave: (() => void) | undefined;
  leaveRoute: string | undefined;
  playAgain: DuosPlayAgain | undefined;
}): ReactElement | null {
  // #1755: the panel takes focus as it comes in (and again when the rail's "Result" reopens it,
  // which mounts it afresh), on its first way on, as the 1v1 result does (Result.tsx).
  const actions = useRef<HTMLDivElement>(null);
  useEffect(() => {
    actions.current?.querySelector<HTMLElement>("button, a")?.focus({ preventScroll: true });
  }, []);
  if (view.result === null) return null;
  const words = resultWords(view.result, teamOf(view.seat));
  const line = againLine(playAgain);
  return (
    <div className="duos-result" data-testid={duosTestid.result} data-outcome={words.outcome} role="dialog" aria-label={words.heading}>
      <h2 className="duos-result__heading">{words.heading}</h2>
      {words.reason === "" ? null : <p className="duos-result__reason">{words.reason}</p>}
      {jloreCapped ? (
        <p className="duos-result__note" data-testid={duosTestid.resultJlore}>
          {t("Daily Jlore reached")}
        </p>
      ) : null}
      {/* #1664: the tracks the game moved, level by level with their rewards, and the quests it completed. */}
      {economy === null ? null : <XpReveal economy={economy} />}
      <div className="row duos-result__actions" ref={actions}>
        <button type="button" data-testid={duosTestid.resultBoard} onClick={onShowBoard}>
          {t("View the board")}
        </button>
        {onWatchReplay === undefined ? null : (
          <button type="button" data-testid={duosTestid.resultReplay} onClick={onWatchReplay}>
            {t("Watch the replay")}
          </button>
        )}
        {playAgain === undefined || playAgain.status?.teammateHere !== true ? null : (
          <button
            type="button"
            className="button-primary"
            data-testid={duosTestid.resultAgain}
            disabled={playAgain.offering}
            aria-busy={playAgain.offering}
            onClick={playAgain.offer}
          >
            {t("Play again")}
          </button>
        )}
        {onLeave === undefined ? null : (
          <button type="button" data-testid={duosTestid.resultBack} data-route={leaveRoute} onClick={onLeave}>
            {t("Back to Play together")}
          </button>
        )}
      </div>
      {line === null ? null : (
        <p className="duos-result__note" data-testid={duosTestid.resultAgainStatus} role={line.alert ? "alert" : "status"}>
          {line.words}
        </p>
      )}
    </div>
  );
}

export function DuosBoard({
  view,
  legal,
  onAction,
  pings,
  onPing,
  error,
  jloreCapped,
  economy = null,
  replay = false,
}: DuosBoardProps): ReactElement {
  useLocale();
  // #1664: the result's write paid this seat's person, so the pill reads the wallet again, once
  // however many views carry the answer.
  const paid = economy !== null;
  useEffect(() => {
    if (paid) announceWallet();
  }, [paid]);
  const chrome = useContext(DuosChromeContext);
  // The board's element as state, for the card bar: set as it mounts, so a ping already up is marked
  // on the first render rather than a frame later (CardActions.tsx).
  const [host, setHost] = useState<HTMLDivElement | null>(null);
  // R2095000: the teammate board's own host, a copy of the one above, so its card bar and ping
  // markers are laid over its own board, not the seat's.
  const [mateHost, setMateHost] = useState<HTMLDivElement | null>(null);
  // The team's result waits for the duel's last burst to play, as the 1v1 result does (Game.tsx).
  const boardBusy = useBoardBusy(host);
  const [showing, setShowing] = useState<Showing>("own");
  // R2095000: Both boards, off until the player presses it, never in a replay.
  const [both, setBoth] = useState<boolean>(readBoth);
  const showBoth = both && !replay;
  const [boardAfterResult, setBoardAfterResult] = useState(false);
  // #1755: "View the board" folds the result to the rail's "Result", which takes the focus the panel
  // had and opens the panel again, as the 1v1 result's chip does (Result.tsx).
  const reopen = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (boardAfterResult) reopen.current?.focus({ preventScroll: true });
  }, [boardAfterResult]);

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
  // R1838000: a Trade is a duel move: the listed `trade`, in its envelope, or nothing.
  const sendTrade = useCallback(
    (instanceId: string) => {
      const body = duelLegal.find((entry) => entry.type === "trade" && entry.instanceId === instanceId);
      if (body !== undefined) sendDuel(body);
    },
    [duelLegal, sendDuel],
  );

  // B64: the pings still up, marked on the board that shows their duel.
  usePingSounds(pings, seat);
  const now = usePingNow(pings);
  const livePings = pings.filter((ping) => now - ping.at < PING_SHOW_MS);
  // R2095000: each board marks the pings that name its duel, so a ping shows on whichever board
  // shows its duel; single mode keeps marking the shown board's, as before.
  const markersFor = (boardView: PlayerView, duel: 0 | 1): PingMarker[] =>
    livePings.flatMap((ping, index) => {
      const spot = spotOfTarget(ping.target, boardView, duel);
      return spot === null
        ? []
        : [{ key: `${ping.from}-${String(ping.at)}-${String(index)}`, spot, ping: ping.ping, from: ping.from, mine: ping.from === seat }];
    });
  const markersOwn = markersFor(view.own, slot);
  const markersMate = markersFor(view.teammate, duelOf(mate));
  const markers = showing === "own" ? markersOwn : markersMate;
  // R2095000: the portal flashes for a ping on the duel it does not show. On a wide screen both
  // duels show so the portal (hidden by CSS there) flashes unseen; on a narrowed screen one board
  // is CSS-hidden, so the visible portal flashes exactly as today's single board does.
  const flash = livePings.some((ping) => ping.from !== seat && ping.target.duel !== shownDuel);
  const heard = [...livePings].reverse().find((ping) => ping.from !== seat);

  // B4, R1168001: whose turn the round is on in the seat's own duel. The duel's active player may be
  // a turn ahead of the round there, when its round's turn ended as it began.
  const ended = view.team.ended[slot] === true;
  const ourTurn = view.roundSeats[slot] === seat;

  const rail = (
    <div className="duos-rail">
      {chrome?.back}
      <p className="duos-round" data-testid={duosTestid.round} data-round={view.round}>
        {view.round === 0 ? t("Mulligan") : t("Round {n}", { n: view.round })}
        {view.round === 0 || over ? null : <> · {ourTurn ? t("Your team's turn") : t("Enemy team's turn")}</>}
        {" · "}
        {t("You are {seat}", { seat: seatLabel(seat) })}
        {replay && view.result !== null ? <> · {resultWords(view.result, teamOf(seat)).heading}</> : null}
      </p>
      <LifeStrip view={view} />
      <TeammateStrip view={view} link={chrome?.teammate ?? null} />
      <div className="duos-rail__row">
        <button
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
        {!replay ? (
          <button
            type="button"
            className="duos-both-toggle"
            data-testid={duosTestid.both}
            aria-pressed={both}
            onClick={() => {
              const next = !both;
              setBoth(next);
              storeBoth(next);
              // Both boards open on the own one, which carries the rail.
              if (next) setShowing("own");
            }}
          >
            {t("Both boards")}
          </button>
        ) : null}
        {replay ? null : <ConcedeControl view={view} live={concedeLegal} onConcede={sendConcede} />}
        {/* R169: the seat's own duel's live modifiers in full, whichever board the portal shows. */}
        <ModifierList view={view.own} opponentLabel={t("Opponent")} />
        {over && boardAfterResult && !replay ? (
          <button
            ref={reopen}
            type="button"
            data-testid={duosTestid.resultReopen}
            onClick={() => {
              setBoardAfterResult(false);
            }}
          >
            {t("Result")}
          </button>
        ) : null}
      </div>
      {!replay && !over && ended ? (
        <p className="duos-waiting" data-testid={duosTestid.waiting} role="status">
          {t("Waiting for your teammate")}
        </p>
      ) : null}
    </div>
  );

  return (
    <div
      className="duos-board"
      ref={showBoth ? undefined : setHost}
      data-testid={duosTestid.board}
      data-seat={seat}
      data-showing={showing}
      data-duel={shownDuel}
      data-both={showBoth}
    >
      {/* R2095000: the rail, with the clock and the status, rides the board `showing` names, as
          the single board's does, so a narrowed screen, which draws only that board, keeps the
          portal to flip back. */}
      {showBoth ? (
        <div className="duos-both">
          <div className="duos-board-own" data-testid={duosTestid.boardOwn} ref={setHost}>
            <Game
              key="board-own"
              view={view.own}
              legal={replay ? NOTHING : duelLegal}
              onAction={replay ? ignore : sendDuel}
              replay={replay}
              wearCosmetics
              error={error ?? null}
              autoEndTurn={false}
              resultForm="chip"
              turnClock={showing === "own" ? chrome?.clock : undefined}
              matchStatus={showing === "own" ? chrome?.status : undefined}
              boardRail={showing === "own" ? rail : undefined}
              tradeBar={false}
            />
            <CardActions
              key="actions-own"
              container={host}
              view={view.own}
              duel={slot}
              enabled={!over}
              pass={{ legal: passLegal, cost: view.team.passCost, left: view.team.passesLeft[slot] ?? 0 }}
              onPass={sendPass}
              trade={tradeOfferFor(view.own, duelLegal)}
              onTrade={sendTrade}
              onPing={onPing}
              markers={markersOwn}
            />
          </div>
          <div className="duos-board-teammate" data-testid={duosTestid.boardTeammate} ref={setMateHost}>
            <Game
              key="board-teammate"
              view={view.teammate}
              legal={NOTHING}
              onAction={ignore}
              replay
              silent
              resultForm="chip"
              turnClock={showing === "teammate" ? chrome?.clock : undefined}
              matchStatus={showing === "teammate" ? chrome?.status : undefined}
              boardRail={showing === "teammate" ? rail : undefined}
            />
            <CardActions
              key="actions-teammate"
              container={mateHost}
              view={view.teammate}
              duel={duelOf(mate)}
              enabled={!over}
              pass={null}
              onPass={sendPass}
              trade={null}
              onPing={onPing}
              markers={markersMate}
            />
          </div>
        </div>
      ) : showing === "own" ? (
        <Game
          key="board-own"
          view={view.own}
          legal={replay ? NOTHING : duelLegal}
          onAction={replay ? ignore : sendDuel}
          replay={replay}
          wearCosmetics
          error={error ?? null}
          autoEndTurn={false}
          resultForm="chip"
          turnClock={chrome?.clock}
          matchStatus={chrome?.status}
          boardRail={rail}
          tradeBar={false}
        />
      ) : (
        <Game
          key="board-teammate"
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
      {replay || showBoth ? null : (
        <CardActions
          key={`actions-${showing}`}
          container={host}
          view={shownView}
          duel={shownDuel}
          enabled={!over}
          pass={showing === "own" ? { legal: passLegal, cost: view.team.passCost, left: view.team.passesLeft[slot] ?? 0 } : null}
          onPass={sendPass}
          trade={showing === "own" ? tradeOfferFor(view.own, duelLegal) : null}
          onTrade={sendTrade}
          onPing={onPing}
          markers={markers}
        />
      )}
      <p className="duos-sr-only" role="status" aria-live="polite">
        {heard === undefined ? "" : t("{seat}: {ping}", { seat: seatLabel(heard.from), ping: t(PING_LABEL[heard.ping]) })}
      </p>
      {over && !boardAfterResult && !replay && !boardBusy ? (
        <ResultPanel
          view={view}
          jloreCapped={jloreCapped}
          economy={economy}
          onShowBoard={() => {
            setBoardAfterResult(true);
          }}
          onWatchReplay={chrome?.onWatchReplay}
          onLeave={chrome?.onLeave}
          leaveRoute={chrome?.leaveRoute}
          playAgain={chrome?.playAgain}
        />
      ) : null}
    </div>
  );
}
