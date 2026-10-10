// Rematch offers after an online match (SPEC §9.5, R672).
// UI sends server intent and renders server status only (CLAUDE.md rule 7).
// R1372: All Random offers may carry the player's `leanNewest`; Best-of-1 offers do not.

import { useEffect, useRef, useState, type ReactElement } from "react";

import { SERIES_POLL_SECONDS } from "@jackioh/server-config";
import type { ConnectionState } from "../game/net.ts";
import { LeanNewestToggle, readPlayLeanNewest, writePlayLeanNewest } from "../game/LeanNewest.tsx";
import { navigate, paths } from "../net/navigate.ts";
import {
  rematchOffer,
  rematchStatus,
  type RematchStatusResponse,
  type RematchStakes,
} from "../net/api.ts";

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
};

const MS_PER_SECOND = 1000;

function incomingWords(stakes: RematchStakes): string {
  return stakes === 2 ? "Your opponent wants double-or-nothing." : "Your opponent wants a rematch.";
}

function waitingWords(stakes: RematchStakes): string {
  return `${stakes === 2 ? "You offered double-or-nothing." : "You offered a rematch."} Waiting for your opponent…`;
}

const OFFERING_LABEL = "Offering…";

/** One status poller serves buttons and watcher so their navigation cannot drift apart. */
function useRematchStatus(token: string, matchId: string, onStatus: (answer: RematchStatusResponse) => void): void {
  const latest = useRef(onStatus);
  latest.current = onStatus;
  useEffect(() => {
    let cancelled = false;
    const read = (): void => {
      rematchStatus(token, matchId).then(
        (answer) => {
          if (!cancelled) latest.current(answer);
        },
        () => undefined,
      );
    };
    read();
    const handle = window.setInterval(read, SERIES_POLL_SECONDS * MS_PER_SECOND);
    return () => {
      cancelled = true;
      window.clearInterval(handle);
    };
  }, [token, matchId]);
}

/** Outlives the result panel so matching offers still navigate a player viewing the board. */
export function RematchWatcher({ token, matchId }: { token: string; matchId: string }): ReactElement | null {
  useRematchStatus(token, matchId, (answer) => {
    if (answer.matchId !== null) navigate(paths.match(answer.matchId));
  });
  return null;
}

export default function RematchButtons({ token, matchId, connection, ranked }: RematchButtonsProps): ReactElement | null {
  const [status, setStatus] = useState<RematchStatusResponse | null>(null);
  const [offering, setOffering] = useState<RematchStakes | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [leanNewest, setLeanNewest] = useState(readPlayLeanNewest);

  useRematchStatus(token, matchId, (answer) => {
    setStatus(answer);
    if (answer.matchId !== null) navigate(paths.match(answer.matchId));
  });

  // Hide offers until the server confirms an open opponent socket.
  if (connection !== "open" || status === null || !status.opponentHere) return null;

  const random = status.mode === "random";

  async function meet(stakes: RematchStakes): Promise<void> {
    if (offering !== null) return;
    setOffering(stakes);
    setError(null);
    try {
      const answer = await (random && leanNewest
        ? rematchOffer(token, matchId, stakes, true)
        : rematchOffer(token, matchId, stakes));
      if (answer.matchId !== null) navigate(paths.match(answer.matchId));
      else {
        const refresh = await rematchStatus(token, matchId).catch(() => null);
        if (refresh !== null) {
          setStatus(refresh);
          if (refresh.matchId !== null) navigate(paths.match(refresh.matchId));
        }
      }
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
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
          {status.opponentOffer === 2 ? "Offer double-or-nothing back to play." : "Offer a rematch back to play."}
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
          {offering === 1 ? OFFERING_LABEL : "Rematch"}
        </button>
        <button
          type="button"
          data-testid={rematchTestid.double}
          disabled={offering !== null || !ranked}
          aria-busy={offering === 2}
          title={ranked ? undefined : "Ranked games only"}
          onClick={() => void meet(2)}
        >
          {offering === 2 ? OFFERING_LABEL : "Double or nothing"}
        </button>
      </div>
      {ranked ? null : <p className="rematch-note">Double-or-nothing needs a ranked match.</p>}
      {random ? (
        <LeanNewestToggle
          checked={leanNewest}
          disabled={offering !== null}
          testid={rematchTestid.leanNewest}
          onChange={(on) => {
            setLeanNewest(on);
            writePlayLeanNewest(on);
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
          {error}
        </p>
      ) : null}
    </div>
  );
}
