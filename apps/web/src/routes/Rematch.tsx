// The death screen's way to play again: rematch offers after an online match (SPEC §9.5, R672).
//
// After a non-series match ends, either seat may offer a rematch — a normal one or, in a ranked
// match, a double-or-nothing — and matching offers start one new game with the finished decks.
// The buttons show only while the opponent is still on the match: this component renders nothing
// unless our own socket is open AND the status says theirs is (`opponentHere`), so leaving,
// logging out or closing the tab takes them away on both sides.
//
// IT ENFORCES NOTHING (CLAUDE.md rule 7). It sends offer intent, polls the status the server
// computes, and navigates to the game the server made. The only numbers it reads are the server's
// (`youOffered`, `opponentOffer`, `opponentHere`, `matchId`).

import { useEffect, useState, type ReactElement } from "react";

import { SERIES_POLL_SECONDS } from "../../../server/src/config.ts";
import type { ConnectionState } from "../game/net.ts";
import { navigate, paths } from "../net/navigate.ts";
import {
  rematchOffer,
  rematchStatus,
  type RematchStatusResponse,
  type RematchStakes,
} from "../net/api.ts";

/** Chrome this component invented, mirroring `matchTestid`'s pattern in `routes/match.tsx`. */
export const rematchTestid = {
  /** The "Rematch" button (a normal game). */
  offer: "rematch-offer",
  /** The "Double or nothing" button (ranked matches only). */
  double: "rematch-double",
  /** The prompt that the opponent already offered, asking to meet it. */
  incoming: "rematch-incoming",
  /** What this component is doing: waiting, or why an offer failed. */
  status: "rematch-status",
} as const;

export type RematchButtonsProps = {
  token: string;
  matchId: string;
  /** Our own socket's state; anything but open hides every button. */
  connection: ConnectionState;
  /** Whether the finished match was ranked: only ranked games go double-or-nothing (R672). */
  ranked: boolean;
};

const MS_PER_SECOND = 1000;

/** The incoming offer in the opponent's words. */
function incomingWords(stakes: RematchStakes): string {
  return stakes === 2 ? "Your opponent wants double-or-nothing." : "Your opponent wants a rematch.";
}

export default function RematchButtons({ token, matchId, connection, ranked }: RematchButtonsProps): ReactElement | null {
  const [status, setStatus] = useState<RematchStatusResponse | null>(null);
  const [offering, setOffering] = useState<RematchStakes | null>(null);
  const [error, setError] = useState<string | null>(null);

  // The status, now and every `SERIES_POLL_SECONDS`: the opponent's offer, their presence, and
  // the game equal offers made. A created game takes both seats there at once.
  useEffect(() => {
    let cancelled = false;
    const read = (): void => {
      rematchStatus(token, matchId).then(
        (answer) => {
          if (cancelled) return;
          setStatus(answer);
          if (answer.matchId !== null) navigate(paths.match(answer.matchId));
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

  // Nothing until the status says the opponent is here — and never on a dead socket. A missing
  // first read reads the same as gone: no buttons on an unknown presence.
  if (connection !== "open" || status === null || !status.opponentHere) return null;

  async function meet(stakes: RematchStakes): Promise<void> {
    if (offering !== null) return;
    setOffering(stakes);
    setError(null);
    try {
      const answer = await rematchOffer(token, matchId, stakes);
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
          className="button-primary"
          data-testid={rematchTestid.offer}
          disabled={offering !== null}
          onClick={() => void meet(1)}
        >
          Rematch
        </button>
        <button
          type="button"
          data-testid={rematchTestid.double}
          disabled={offering !== null || !ranked}
          title={ranked ? undefined : "Ranked games only"}
          onClick={() => void meet(2)}
        >
          Double or nothing
        </button>
      </div>
      {ranked ? null : <p className="rematch-note">Double-or-nothing needs a ranked match.</p>}
      {waiting && error === null ? (
        <p className="rematch-status" data-testid={rematchTestid.status} role="status">
          Waiting for your opponent…
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
