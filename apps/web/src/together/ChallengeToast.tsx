// The challenge toast (#936, B57; #1831): mounted once in the shell (`main.tsx`), it reads the open
// challenges on every screen while the tab holds a session (`socialPoll.ts`).
// AN INCOMING CHALLENGE offers the deck or trio last used for its mode in this browser
// (`challengeChoice.ts`), or for All Random "More cards from" (R924000), lazily since the shell mounts
// the toast on every screen (#778). Accept joins the room by its code, which accepts the challenge (B49).
// AN OUTGOING CHALLENGE is followed: a challenge is a room, so the challenger's game starts when the
// friend joins it, wherever the challenger is; the toast announces it as a pairing is ("Match found!",
// R765) and takes them there.
// Nothing here is a rule (CLAUDE.md rule 7): whom a player may challenge, whether a deck may be
// played, and when a challenge lapses, are the server's. A session the server refuses outright is
// not asked again until the tab holds another one.

import { useEffect, useRef, useState, type ReactElement } from "react";

import { MATCH_FOUND_NAV_DELAY_MS } from "@jackioh/server-config";
import type { LeanPick } from "@jackioh/shared";

import { LeanSetPickerLazy } from "../game/LeanSetPickerLazy.tsx";
import { readPlayLean, writePlayLean } from "../game/playLean.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { getDecks, getMe, joinRoom, type ModeChoice, type SavedDeck, type SavedTrio } from "../net/api.ts";
import { withSession } from "../net/gate.ts";
import { liveGameOf, type LiveGame } from "../net/liveGame.ts";
import { currentPath, leaveFirst, navigate, paths } from "../net/navigate.ts";
import { roomLinkPath } from "../net/roomLink.ts";
import { declineChallenge, type Challenge } from "../net/social.ts";
import { QueueFound } from "../routes/GameBanner.tsx";
import { Skeleton } from "../skeleton/Skeleton.tsx";
import { choiceOf, pickDeck, pickTrio, readChallengeChoice, rememberChallengeChoice } from "./challengeChoice.ts";
import { followingChallenge, stopFollowingChallenge } from "./follow.ts";
import { messageOf } from "./session.ts";
import { reloadSocial, useSocialPoll } from "./socialPoll.ts";
import { challengeToastTestid } from "./testids.ts";
import { MODE_WORDS, Named } from "./words.tsx";

import "./together.css";

type Saved = { decks: readonly SavedDeck[]; trios: readonly SavedTrio[] };

/** Adds `id` to a set without changing the one React holds. */
function including(ids: ReadonlySet<string>, id: string): ReadonlySet<string> {
  const next = new Set(ids);
  next.add(id);
  return next;
}

function gameAfterJoin(answer: { matchId: string | null; seriesId: string | null }): string {
  if (typeof answer.matchId === "string" && answer.matchId.length > 0) return paths.match(answer.matchId);
  if (typeof answer.seriesId === "string" && answer.seriesId.length > 0) return paths.series(answer.seriesId);
  return paths.play;
}

export function ChallengeToast(): ReactElement {
  useLocale();
  const social = useSocialPoll();
  /** Challenges this tab has answered: gone from the toast before the server's next read says so. */
  const [answered, setAnswered] = useState<ReadonlySet<string>>(() => new Set());
  const [declining, setDeclining] = useState<string | null>(null);
  const [joining, setJoining] = useState<string | null>(null);
  /** The challenge whose join the server refused, whose `error` it is. */
  const [refusedId, setRefusedId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [found, setFound] = useState<LiveGame | null>(null);
  /** The player's decks and trios; null until read. */
  const [saved, setSaved] = useState<Saved | "failed" | null>(null);
  /** The deck or trio picked in a toast, by challenge; until then the one last used. */
  const [picks, setPicks] = useState<Readonly<Record<string, string>>>({});
  /** R2031000: the "More cards from" pick made in a toast, by challenge; until then the device's online pick. */
  const [leans, setLeans] = useState<Readonly<Record<string, LeanPick | null>>>({});
  /** The last read held an open challenge of the player's own. */
  const hadOutgoing = useRef(false);

  const shown = social.incoming.filter((challenge) => !answered.has(challenge.challengeId));
  const needsDecks = shown.some((challenge) => challenge.mode !== "random");

  // A toast that needs a deck reads the player's decks as it appears, and again for the next one.
  useEffect(() => {
    if (!needsDecks) {
      setSaved(null);
      return;
    }
    let canceled = false;
    withSession(getDecks).then(
      (answer) => {
        if (!canceled) setSaved({ decks: answer.decks, trios: answer.trios });
      },
      () => {
        if (!canceled) setSaved("failed");
      },
    );
    return () => {
      canceled = true;
    };
  }, [needsDecks]);

  // Each read of the challenges, while one of the player's own is open (or was just sent), asks for a
  // game the friend's join may have started. An accepted challenge is no longer open, so the read after
  // the join no longer lists it; one sent since the last read is not listed yet either (`follow.ts`).
  useEffect(() => {
    if (social.challengeReads === 0 || social.refused) {
      hadOutgoing.current = false;
      return;
    }
    const ours = social.outgoing.length > 0;
    const watching = hadOutgoing.current || ours || followingChallenge();
    hadOutgoing.current = ours;
    if (!watching) return;
    let canceled = false;
    withSession(getMe).then(
      (me) => {
        if (canceled) return;
        const game = liveGameOf(me);
        if (game === null) return;
        stopFollowingChallenge();
        if (currentPath() === game.path) return;
        setFound((current) => current ?? game);
      },
      () => undefined,
    );
    return () => {
      canceled = true;
    };
  }, [social.challengeReads, social.outgoing, social.refused]);

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

  /** The id of the deck (Best of 1) or trio (Conquest) picked for `challenge`, null while there is none to pick. */
  const pickedFor = (challenge: Challenge): string | null => {
    if (saved === null || saved === "failed" || challenge.mode === "random") return null;
    const remembered = readChallengeChoice();
    const chosen = picks[challenge.challengeId];
    if (challenge.mode === "bo3") return pickTrio(saved.trios, chosen ?? remembered.trioId)?.id ?? null;
    return pickDeck(saved.decks, chosen ?? remembered.deckId)?.id ?? null;
  };

  /** R2031000: the pick an All Random `challenge` joins with, read afresh until one is made in its toast. */
  const leanFor = (challenge: Challenge): LeanPick | null => {
    const chosen = leans[challenge.challengeId];
    return chosen === undefined ? readPlayLean() : chosen;
  };

  /** Joins with `choice` and takes the player to its game (R1831000). */
  const join = (challenge: Challenge, choice: ModeChoice): void => {
    if (joining !== null) return;
    setJoining(challenge.challengeId);
    setError(null);
    setRefusedId(null);
    withSession((token) => joinRoom(token, challenge.roomCode, choice))
      .then(
        (answer) => {
          rememberChallengeChoice(choice, false);
          setAnswered((ids) => including(ids, challenge.challengeId));
          reloadSocial();
          // The player let any unsaved work go before the join (`leaveFirst`).
          navigate(gameAfterJoin(answer), { force: true });
        },
        (cause: unknown) => {
          setError(messageOf(cause));
          setRefusedId(challenge.challengeId);
        },
      )
      .finally(() => {
        setJoining(null);
      });
  };

  /** The way in when the toast cannot join: `/play` with the room's code and mode filled in (R767). */
  const openInLobby = (challenge: Challenge): void => {
    setError(null);
    setRefusedId(null);
    const target = roomLinkPath(challenge.roomCode, challenge.mode);
    // `/play` reads a room link as it mounts (R767); a lobby already open reads it on a fresh load.
    if (currentPath() === paths.play) {
      setAnswered((ids) => including(ids, challenge.challengeId));
      window.location.assign(target);
      return;
    }
    // R1754000: a page holding unsaved work may hold the move; until the player goes, the toast stays to be accepted again.
    if (navigate(target)) setAnswered((ids) => including(ids, challenge.challengeId));
  };

  const decline = (challenge: Challenge): void => {
    setDeclining(challenge.challengeId);
    setError(null);
    setRefusedId(null);
    declineChallenge(challenge.challengeId)
      .then(
        () => {
          setAnswered((ids) => including(ids, challenge.challengeId));
          reloadSocial();
        },
        (cause: unknown) => {
          setError(messageOf(cause));
        },
      )
      .finally(() => {
        setDeclining(null);
      });
  };

  return (
    <>
      <div className="challenge-toasts" aria-live="polite">
        {shown.map((challenge) => {
          const id = challenge.challengeId;
          const pick = pickedFor(challenge);
          const lean = challenge.mode === "random" ? leanFor(challenge) : null;
          const choice = choiceOf(challenge.mode, pick, pick, lean);
          const list = saved === null || saved === "failed" ? [] : challenge.mode === "bo3" ? saved.trios : saved.decks;
          const busy = joining === id || declining === id;
          const lobby = refusedId === id || (challenge.mode !== "random" && saved !== null && choice === null);
          return (
            <section
              key={id}
              className="notice challenge-toast"
              data-testid={challengeToastTestid.toast}
              data-challenge-id={id}
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
              {challenge.mode !== "random" && saved === null ? (
                <Skeleton variant="panel" label={t("Loading your decks…")} />
              ) : null}
              {challenge.mode !== "random" && list.length > 0 && pick !== null ? (
                <label className="challenge-toast__pick">
                  {challenge.mode === "bo3" ? t("Your trio") : t("Your deck")}
                  <select
                    className="lobby-select"
                    data-testid={challengeToastTestid.deck}
                    value={pick}
                    disabled={busy}
                    onChange={(event) => {
                      const value = event.target.value;
                      setPicks((before) => ({ ...before, [id]: value }));
                    }}
                  >
                    {list.map((entry) => (
                      <option key={entry.id} value={entry.id}>
                        {entry.name}
                      </option>
                    ))}
                  </select>
                </label>
              ) : null}
              {challenge.mode === "random" ? (
                <LeanSetPickerLazy
                  value={lean}
                  disabled={busy}
                  testid={challengeToastTestid.lean}
                  onChange={(next) => {
                    setLeans((before) => ({ ...before, [id]: next }));
                    writePlayLean(next);
                  }}
                />
              ) : null}
              {refusedId === id && error !== null ? (
                <p className="challenge-toast__refusal" data-testid={challengeToastTestid.error} role="alert">
                  {t(error)}
                </p>
              ) : null}
              <div className="challenge-toast__actions">
                {choice === null ? null : (
                  <button
                    type="button"
                    className="button-primary"
                    data-testid={challengeToastTestid.accept}
                    disabled={busy}
                    aria-busy={joining === id}
                    onClick={() => {
                      // R1754000: unsaved work may hold the join; until the player goes, the toast stays.
                      leaveFirst(() => {
                        join(challenge, choice);
                      });
                    }}
                  >
                    {t("Accept")}
                  </button>
                )}
                {lobby ? (
                  <button
                    type="button"
                    data-testid={challengeToastTestid.lobby}
                    data-route={roomLinkPath(challenge.roomCode, challenge.mode)}
                    disabled={busy}
                    onClick={() => {
                      openInLobby(challenge);
                    }}
                  >
                    {t("Open in lobby")}
                  </button>
                ) : null}
                <button
                  type="button"
                  data-testid={challengeToastTestid.decline}
                  disabled={busy}
                  aria-busy={declining === id}
                  onClick={() => {
                    decline(challenge);
                  }}
                >
                  {t("Decline")}
                </button>
              </div>
            </section>
          );
        })}
        {error === null || refusedId !== null || shown.length === 0 ? null : (
          <p className="notice challenge-toast challenge-toast--error" data-testid={challengeToastTestid.error} role="alert">
            {t(error)}
          </p>
        )}
      </div>
      <QueueFound game={found} />
    </>
  );
}
