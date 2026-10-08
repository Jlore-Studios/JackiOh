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

import { useEffect, useRef, useState, type ReactElement } from "react";

import { SERIES_POLL_SECONDS } from "@jackioh/server-config";
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

/** Our own offer while it waits, so the line says which of the two buttons was pressed. */
function waitingWords(stakes: RematchStakes): string {
  return `${stakes === 2 ? "You offered double-or-nothing." : "You offered a rematch."} Waiting for your opponent…`;
}

/** A button's label while its offer is on its way (the `aria-busy` pattern of the app's other sends). */
const OFFERING_LABEL = "Offering…";

/**
 * The status, now and every `SERIES_POLL_SECONDS`: the opponent's offer, their presence, and
 * the game equal offers made. A created game takes both seats there at once. One
 * implementation for the buttons and the watcher below, so the two never drift apart.
 */
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

/**
 * The half of a rematch that must outlive the result panel. The buttons live in the panel's
 * `actions` (`Result.tsx` renders those only while the panel is open), so a seat that offers
 * and then presses "View the board" unmounts their poller — and is never taken to the game
 * the opponent's matching offer creates, while already sitting in that ranked game. The
 * match route mounts this alongside the board once the game is over, so the navigation
 * survives the fold. It renders nothing and only ever navigates; offering still needs the
 * panel open. Two pollers while the panel is open are harmless: both navigate idempotently
 * to the same game.
 */
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

  useRematchStatus(token, matchId, (answer) => {
    setStatus(answer);
    if (answer.matchId !== null) navigate(paths.match(answer.matchId));
  });

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

  // The markup is the result panel's (Result.tsx `actions`): the two offers are its buttons, the
  // incoming offer a line above them and the wait or the refusal a line below. reveal.css lays the
  // block into the panel's own row of ways on, so the buttons carry no class of their own: the
  // panel sizes them like its others and makes Rematch its primary (animations.css). The shell's
  // `button-primary` is the lobby's blue call to action, not the board's gold one.
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
