// The challenge toast (#936, B57): mounted once in the shell (`main.tsx`), it reads
// `GET /api/challenges` every `SOCIAL_POLL_MS` while the tab holds a session, on every screen.
//
// AN INCOMING CHALLENGE is a toast: Accept takes the player to `/play` with the room's code and mode
// filled in, the way a room link does (R767), where they pick their deck and join; the join is what
// accepts it (B49). Decline declines it. Either way the toast goes.
//
// AN OUTGOING CHALLENGE is followed: a challenge is a room, so the challenger's game starts when the
// friend joins it, wherever the challenger is. While the last read held one of theirs (or one was just
// sent from this tab, `follow.ts`), each read also asks `/api/auth/me` for a live game, and once it
// names one the toast announces it as a pairing is announced ("Match found!", R765) and takes them there.
//
// Nothing here is a rule (CLAUDE.md rule 7): whom a player may challenge, and when a challenge lapses,
// are the server's. A session the server refuses outright is not asked again until the tab holds
// another one.

import { useEffect, useRef, useState, type ReactElement } from "react";

import { MATCH_FOUND_NAV_DELAY_MS } from "@jackioh/server-config";

import { t, useLocale } from "../i18n/i18n.ts";
import { getMe } from "../net/api.ts";
import { liveGameOf, type LiveGame } from "../net/liveGame.ts";
import { currentPath, navigate, paths } from "../net/navigate.ts";
import { roomLinkPath } from "../net/roomLink.ts";
import { readSession } from "../net/session.ts";
import { declineChallenge, getChallenges, type Challenge } from "../net/social.ts";
import { QueueFound } from "../routes/GameBanner.tsx";
import { SOCIAL_POLL_MS } from "./config.ts";
import { followingChallenge, stopFollowingChallenge } from "./follow.ts";
import { isRefusedSession, messageOf, withToken } from "./session.ts";
import { challengeToastTestid } from "./testids.ts";
import { MODE_WORDS, Named } from "./words.tsx";

import "./together.css";

/** A challenge as S7 shapes one, as far as the toast reads it. */
function isChallenge(value: unknown): value is Challenge {
  if (typeof value !== "object" || value === null) return false;
  const { challengeId, roomCode, mode, from } = value as { challengeId?: unknown; roomCode?: unknown; mode?: unknown; from?: unknown };
  return (
    typeof challengeId === "string" &&
    typeof roomCode === "string" &&
    (mode === "bo1" || mode === "bo3" || mode === "random") &&
    typeof from === "object" &&
    from !== null &&
    typeof (from as { username?: unknown }).username === "string"
  );
}

/** Adds `id` to a set without changing the one React holds. */
function including(ids: ReadonlySet<string>, id: string): ReadonlySet<string> {
  const next = new Set(ids);
  next.add(id);
  return next;
}

export function ChallengeToast(): ReactElement {
  useLocale();
  const [incoming, setIncoming] = useState<readonly Challenge[]>([]);
  /** Challenges this tab has answered: gone from the toast before the server's next read says so. */
  const [answered, setAnswered] = useState<ReadonlySet<string>>(() => new Set());
  const [declining, setDeclining] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [found, setFound] = useState<LiveGame | null>(null);
  /** The last read held an open challenge of the player's own. */
  const hadOutgoing = useRef(false);

  useEffect(() => {
    let cancelled = false;
    let refused: string | null = null;

    const follow = (): void => {
      withToken(getMe).then(
        (me) => {
          if (cancelled) return;
          const game = liveGameOf(me);
          if (game === null) return;
          stopFollowingChallenge();
          if (currentPath() === game.path) return;
          setFound((shown) => shown ?? game);
        },
        () => undefined,
      );
    };

    const tick = (): void => {
      const token = readSession()?.accessToken ?? null;
      if (token === null || token === refused) {
        hadOutgoing.current = false;
        setIncoming((shown) => (shown.length === 0 ? shown : []));
        return;
      }
      getChallenges().then(
        (answer) => {
          if (cancelled) return;
          // Read as lists of challenges whatever answered: the toast sits over every screen, and a
          // server from before #936 has no such route.
          const theirs = Array.isArray(answer.incoming) ? answer.incoming.filter(isChallenge) : [];
          const ours = Array.isArray(answer.outgoing) ? answer.outgoing.filter(isChallenge) : [];
          setIncoming(theirs);
          // An accepted challenge is no longer open, so the read after the join no longer lists it; one
          // sent since the last read is not listed yet either (`follow.ts`).
          const watching = hadOutgoing.current || ours.length > 0 || followingChallenge();
          hadOutgoing.current = ours.length > 0;
          if (watching) follow();
        },
        (cause: unknown) => {
          if (isRefusedSession(cause)) refused = token;
        },
      );
    };

    tick();
    const handle = window.setInterval(tick, SOCIAL_POLL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(handle);
    };
  }, []);

  // The announcement holds for the pairing's beat, then the player is taken to the game.
  useEffect(() => {
    if (found === null) return;
    const timer = window.setTimeout(() => {
      setFound(null);
      navigate(found.path);
    }, MATCH_FOUND_NAV_DELAY_MS);
    return () => {
      window.clearTimeout(timer);
    };
  }, [found]);

  const accept = (challenge: Challenge): void => {
    setAnswered((ids) => including(ids, challenge.challengeId));
    setError(null);
    const target = roomLinkPath(challenge.roomCode, challenge.mode);
    // `/play` reads a room link as it mounts (R767); a lobby already open reads it on a fresh load.
    if (currentPath() === paths.play) window.location.assign(target);
    else navigate(target);
  };

  const decline = (challenge: Challenge): void => {
    setDeclining(challenge.challengeId);
    setError(null);
    declineChallenge(challenge.challengeId)
      .then(
        () => {
          setAnswered((ids) => including(ids, challenge.challengeId));
        },
        (cause: unknown) => {
          setError(messageOf(cause));
        },
      )
      .finally(() => {
        setDeclining(null);
      });
  };

  const shown = incoming.filter((challenge) => !answered.has(challenge.challengeId));

  return (
    <>
      <div className="challenge-toasts" aria-live="polite">
        {shown.map((challenge) => (
          <section
            key={challenge.challengeId}
            className="notice challenge-toast"
            data-testid={challengeToastTestid.toast}
            data-challenge-id={challenge.challengeId}
            data-room-code={challenge.roomCode}
            data-mode={challenge.mode}
            aria-label={t("Challenge")}
          >
            <p className="challenge-toast__text">
              <Named
                text={t("{name} challenges you to {mode}.", { mode: t(MODE_WORDS[challenge.mode]) })}
                name={challenge.from.username}
              />
            </p>
            <div className="challenge-toast__actions">
              <button
                type="button"
                className="button-primary"
                data-testid={challengeToastTestid.accept}
                disabled={declining === challenge.challengeId}
                onClick={() => {
                  accept(challenge);
                }}
              >
                {t("Accept")}
              </button>
              <button
                type="button"
                data-testid={challengeToastTestid.decline}
                disabled={declining === challenge.challengeId}
                aria-busy={declining === challenge.challengeId}
                onClick={() => {
                  decline(challenge);
                }}
              >
                {t("Decline")}
              </button>
            </div>
          </section>
        ))}
        {error === null || shown.length === 0 ? null : (
          <p className="notice challenge-toast challenge-toast--error" data-testid={challengeToastTestid.error} role="alert">
            {t(error)}
          </p>
        )}
      </div>
      <QueueFound game={found} />
    </>
  );
}
