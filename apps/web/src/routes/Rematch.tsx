// Rematch offers after an online match (SPEC §9.5, R672).
// UI sends server intent and renders server status only (CLAUDE.md rule 7).
// R1372, R924000: All random offers may carry the player's "More cards from" pick; Best-of-1 offers do not.

import { useEffect, useRef, useState, type ReactElement } from "react";

import { SERIES_POLL_SECONDS } from "@jackioh/server-config";
import { pollWhileVisible, type ConnectionState } from "../game/net.ts";
import { LeanSetPicker, readPlayLean, writePlayLean } from "../game/LeanNewest.tsx";
import { t } from "../i18n/i18n.ts";
import { navigate, paths } from "../net/navigate.ts";
import {
  rematchOffer,
  rematchStatus,
  type RematchStatusResponse,
  type RematchStakes,
} from "../net/api.ts";
import { messageOf } from "../net/errors.ts";

export const rematchTestid = {
  offer: "rematch-offer",
  double: "rematch-double",
  incoming: "rematch-incoming",
  status: "rematch-status",
  leanNewest: "rematch-lean-newest",
} as const;

export type RematchButtonsProps = {
  token: string;
  matchId: string;
  connection: ConnectionState;
  ranked: boolean;
  /** R3213000: the match socket's last `rematch` frame (`MatchSnapshot.rematch`); while it is held, nothing is read. */
  pushed?: RematchStatusResponse | null;
};

const MS_PER_SECOND = 1000;

function incomingWords(stakes: RematchStakes): string {
  return stakes === 2 ? t("Your opponent wants double-or-nothing.") : t("Your opponent wants a rematch.");
}

function waitingWords(stakes: RematchStakes): string {
  return stakes === 2
    ? t("You offered double-or-nothing. Waiting for your opponent…")
    : t("You offered a rematch. Waiting for your opponent…");
}

const OFFERING_LABEL = "Offering…";

type Listener = (answer: RematchStatusResponse) => void;

/** One match's poll: whoever shows its status listens, and the last answer waits for a later joiner. */
type Poller = { listeners: Set<Listener>; last: RematchStatusResponse | null; stop: () => void };

/** R3207002: the tab's polls, one per match id, whatever number of components show that match's status. */
const pollers = new Map<string, Poller>();

function startPoller(token: string, matchId: string): Poller {
  let stopped = false;
  // A wake while a read is out would only repeat it.
  let reading = false;
  const poller: Poller = { listeners: new Set(), last: null, stop: () => undefined };
  const read = (): void => {
    if (reading) return;
    reading = true;
    rematchStatus(token, matchId)
      .then(
        (answer) => {
          if (stopped) return;
          poller.last = answer;
          for (const listener of [...poller.listeners]) listener(answer);
        },
        () => undefined,
      )
      .finally(() => {
        reading = false;
      });
  };
  read();
  // R3205000: a hidden tab reads nothing; a throttled tab's interval stalls, and showing the tab (or the
  // network coming back) reads the status again at once.
  const stopPolling = pollWhileVisible(read, SERIES_POLL_SECONDS * MS_PER_SECOND);
  poller.stop = () => {
    stopped = true;
    stopPolling();
  };
  return poller;
}

/**
 * R3207002: listens to the match's poll, starting it when none runs in this tab. A listener that joins
 * after an answer came hears it at once and sends no request; the last to leave stops the poll. It
 * returns what leaves.
 */
function joinRematchStatus(token: string, matchId: string, listener: Listener): () => void {
  let poller = pollers.get(matchId);
  if (poller === undefined) {
    poller = startPoller(token, matchId);
    pollers.set(matchId, poller);
  }
  const joined = poller;
  joined.listeners.add(listener);
  if (joined.last !== null) listener(joined.last);
  return () => {
    joined.listeners.delete(listener);
    if (joined.listeners.size > 0) return;
    joined.stop();
    if (pollers.get(matchId) === joined) pollers.delete(matchId);
  };
}

/**
 * One status poll per match serves buttons and watcher (R3207002), so their navigation cannot drift apart.
 * R3213000: while the match socket holds a pushed status, that is the answer and nothing is read; once it
 * goes back to null (the socket dropped) the poll starts again, one read at once.
 */
function useRematchStatus(
  token: string,
  matchId: string,
  pushed: RematchStatusResponse | null,
  onStatus: (answer: RematchStatusResponse) => void,
): void {
  const latest = useRef(onStatus);
  latest.current = onStatus;
  const live = pushed !== null;
  useEffect(() => {
    if (live) return;
    return joinRematchStatus(token, matchId, (answer) => {
      latest.current(answer);
    });
  }, [token, matchId, live]);
  useEffect(() => {
    if (pushed !== null) latest.current(pushed);
  }, [pushed]);
}

/** Outlives the result panel so matching offers still navigate a player viewing the board. */
export function RematchWatcher({
  token,
  matchId,
  pushed = null,
}: {
  token: string;
  matchId: string;
  /** R3213000: as `RematchButtonsProps.pushed`. */
  pushed?: RematchStatusResponse | null;
}): ReactElement | null {
  useRematchStatus(token, matchId, pushed, (answer) => {
    if (answer.matchId !== null) navigate(paths.match(answer.matchId));
  });
  return null;
}

export default function RematchButtons({ token, matchId, connection, ranked, pushed = null }: RematchButtonsProps): ReactElement | null {
  const [status, setStatus] = useState<RematchStatusResponse | null>(null);
  const [offering, setOffering] = useState<RematchStakes | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [lean, setLean] = useState(readPlayLean);

  useRematchStatus(token, matchId, pushed, (answer) => {
    setStatus(answer);
    if (answer.matchId !== null) navigate(paths.match(answer.matchId));
  });

  if (connection !== "open" || status === null || !status.opponentHere) return null;

  const random = status.mode === "random";

  async function meet(stakes: RematchStakes): Promise<void> {
    if (offering !== null) return;
    setOffering(stakes);
    setError(null);
    try {
      const answer = await (random && lean !== null
        ? rematchOffer(token, matchId, stakes, lean)
        : rematchOffer(token, matchId, stakes));
      if (answer.matchId !== null) navigate(paths.match(answer.matchId));
      // R3213000: a socket that pushes the status pushes the one this offer made.
      else if (pushed === null) {
        const refresh = await rematchStatus(token, matchId).catch(() => null);
        if (refresh !== null) {
          setStatus(refresh);
          if (refresh.matchId !== null) navigate(paths.match(refresh.matchId));
        }
      }
    } catch (cause) {
      setError(messageOf(cause));
    } finally {
      setOffering(null);
    }
  }

  const incoming = status.opponentOffer !== null && status.opponentOffer !== status.youOffered;
  const waiting = status.youOffered !== null && status.matchId === null;

  // Result-panel styles determine the unclassed offers' layout and primary treatment.
  return (
    <div className="rematch">
      {incoming ? (
        <p className="rematch-incoming" data-testid={rematchTestid.incoming}>
          {incomingWords(status.opponentOffer as RematchStakes)}{" "}
          {status.opponentOffer === 2 ? t("Offer double-or-nothing back to play.") : t("Offer a rematch back to play.")}
        </p>
      ) : null}
      <div className="rematch-buttons">
        <button
          type="button"
          data-testid={rematchTestid.offer}
          disabled={offering !== null}
          aria-busy={offering === 1}
          onClick={() => void meet(1)}
        >
          {offering === 1 ? t(OFFERING_LABEL) : t("Rematch")}
        </button>
        <button
          type="button"
          data-testid={rematchTestid.double}
          disabled={offering !== null || !ranked}
          aria-busy={offering === 2}
          title={ranked ? undefined : t("Ranked games only")}
          onClick={() => void meet(2)}
        >
          {offering === 2 ? t(OFFERING_LABEL) : t("Double or nothing")}
        </button>
      </div>
      {ranked ? null : <p className="rematch-note">{t("Double-or-nothing needs a ranked match.")}</p>}
      {random ? (
        <LeanSetPicker
          value={lean}
          disabled={offering !== null}
          testid={rematchTestid.leanNewest}
          onChange={(pick) => {
            setLean(pick);
            writePlayLean(pick);
          }}
        />
      ) : null}
      {waiting && error === null ? (
        <p className="rematch-status" data-testid={rematchTestid.status} role="status">
          {waitingWords(status.youOffered as RematchStakes)}
        </p>
      ) : null}
      {error !== null ? (
        <p className="rematch-status" data-testid={rematchTestid.status} role="alert">
          {t(error)}
        </p>
      ) : null}
    </div>
  );
}
