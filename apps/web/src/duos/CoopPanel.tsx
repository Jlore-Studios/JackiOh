// The co-op panel on Play together (#936 S8, B60): make a Duos room against the AI team, share its
// code, join a friend's by its code, and go to the board once both are in.
//
// CLAUDE.md rule 7: the panel proposes and the server rules. The AI's difficulty and the deck or a
// random deal ("More cards from", R924000, R2031000, the device's online pick) are what the player
// chose; the server checks them and answers. The host's room is read every COOP_ROOM_POLL_MS until it
// names its match, and is remembered for the tab (COOP_ROOM_KEY), so a reload picks the wait up.
//
// R915005: a deck naming a custom card is listed off, "(practice only)", and never picked.

import { useEffect, useId, useRef, useState, type FormEvent, type ReactElement } from "react";

import { DIFFICULTIES, type Difficulty } from "@jackioh/engine/config";

import { LeanSetPicker, readPlayLean, writePlayLean } from "../game/LeanNewest.tsx";
import { pageWake } from "../game/net.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { ApiRequestError, getDecks, type SavedDeck } from "../net/api.ts";
import {
  cancelDuosRoom,
  createDuosRoom,
  getDuosLive,
  getDuosRoom,
  joinDuosRoom,
  type DuosDeckChoice,
} from "../net/duos.ts";
import { withSession } from "../net/gate.ts";
import { navigate, paths, useSearch } from "../net/navigate.ts";
import { coopLinkUrl, forgetCoopLink, readCoopLink, sendRoomLink } from "../net/roomLink.ts";
import { sendChat, type Friend } from "../net/social.ts";
import { followInApp } from "../routes/nav.tsx";
import { deckPracticeOnly, practiceOnlyLabel } from "../together/challengeChoice.ts";
import { attempt, messageOf } from "../together/session.ts";
import { COOP_ROOM_KEY, COOP_ROOM_POLL_MS } from "./config.ts";
import { duosTestid } from "./testids.ts";
import "./duos.css";

/** The deck select's value for a dealt deck. */
const RANDOM = "random";

/** R2094000: where this device remembers its last co-op deck pick, so a link join is one press. */
export const COOP_DECK_KEY = "jackioh.coop.deck";

/** R2094000: the remembered deck id, or null when none or storage refuses. */
export function readCoopDeck(): string | null {
  try {
    const raw = window.localStorage.getItem(COOP_DECK_KEY);
    return typeof raw === "string" && raw.length > 0 ? raw : null;
  } catch {
    return null;
  }
}

/** R2094000: remember the pick a co-op room was made or joined with, a random deal included. */
export function writeCoopDeck(id: string): void {
  try {
    window.localStorage.setItem(COOP_DECK_KEY, id);
  } catch {
    // Remembering the pick is a convenience; a refusal costs nothing.
  }
}

/**
 * R2094000: the deck a join starts on: the last co-op pick when still saved (a random deal
 * stays one), else the first saved deck, else none (a random deal). R915005: never a deck that plays
 * only in practice, which a co-op room refuses (§13.9).
 */
export function defaultCoopDeck(decks: readonly SavedDeck[], remembered: string | null): SavedDeck | null {
  if (remembered === RANDOM) return null;
  const online = decks.filter((deck) => !deckPracticeOnly(deck));
  return online.find((deck) => deck.id === remembered) ?? online[0] ?? null;
}

const DIFFICULTY_LABEL: Readonly<Record<Difficulty, string>> = {
  easy: "Easy",
  medium: "Medium",
  hard: "Hard",
};

function readStoredRoom(): string | null {
  try {
    return window.sessionStorage.getItem(COOP_ROOM_KEY);
  } catch {
    return null;
  }
}

function storeRoom(code: string | null): void {
  try {
    if (code === null) window.sessionStorage.removeItem(COOP_ROOM_KEY);
    else window.sessionStorage.setItem(COOP_ROOM_KEY, code);
  } catch {
    // Blocked storage: the wait lasts as long as the page does.
  }
}

/** A refusal (B33's `full` and `own_room`, B34's unknown code, …) in the server's own words. */
function refusalWords(cause: unknown): string {
  return cause instanceof Error ? t(cause.message) : String(cause);
}

/** The account's saved decks by name, for the deck choice, and why they could not be read (none then). */
function useSavedDecks(): { decks: readonly SavedDeck[]; loadError: string | null; retry: () => void } {
  const [decks, setDecks] = useState<readonly SavedDeck[]>([]);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let canceled = false;
    withSession(getDecks).then(
      (answer) => {
        if (!canceled) setDecks(answer.decks);
      },
      (cause: unknown) => {
        if (!canceled) setLoadError(messageOf(cause));
      },
    );
    return () => {
      canceled = true;
    };
  }, [attempt]);
  const retry = (): void => {
    setLoadError(null);
    setAttempt((before) => before + 1);
  };
  return { decks, loadError, retry };
}

/** B42: the co-op game the account is already in (null when none), and why it could not be read. */
function useLiveCoop(): { live: string | null; liveError: string | null; retryLive: () => void } {
  const [live, setLive] = useState<string | null>(null);
  const [liveError, setLiveError] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  useEffect(() => {
    let canceled = false;
    getDuosLive().then(
      (answer) => {
        if (!canceled) setLive(answer.matchId);
      },
      (cause: unknown) => {
        if (!canceled) setLiveError(messageOf(cause));
      },
    );
    return () => {
      canceled = true;
    };
  }, [attempt]);
  const retryLive = (): void => {
    setLiveError(null);
    setAttempt((before) => before + 1);
  };
  return { live, liveError, retryLive };
}

export function CoopPanel({ friends }: { friends: readonly Friend[] }): ReactElement {
  useLocale();
  const headingId = useId();
  const joinId = useId();
  // Each Try again is described by its own alert, so two of them on one panel can be told apart.
  const decksErrorId = useId();
  const liveErrorId = useId();
  const { decks, loadError, retry } = useSavedDecks();
  const [difficulty, setDifficulty] = useState<Difficulty>("easy");
  // R2094000: a co-op link fills the join form once, read here so StrictMode's second call agrees.
  const [coopLink] = useState(readCoopLink);
  // R2094000: a chat Join on this same screen arrives as a new query without remounting.
  const search = useSearch();
  // A remembered id is only taken once the decks show it is still saved, so the deck sent is the one shown.
  const [deck, setDeck] = useState<string>(RANDOM);
  const [lean, setLean] = useState(readPlayLean);
  const [room, setRoom] = useState<string | null>(readStoredRoom);
  const [joinCode, setJoinCode] = useState(coopLink ?? "");
  const [linkSent, setLinkSent] = useState<"shared" | "copied" | null>(null);
  const [picked, setPicked] = useState("");
  // R2094000: the deck settles to the last pick once decks arrive, unless the player touched it.
  const touchedDeck = useRef(false);
  const joinSubmit = useRef<HTMLButtonElement>(null);
  const focusPending = useRef(coopLink !== null);
  // The friend the invite goes to: the one picked, else the first listed, so one friend needs no pick.
  const invitee = friends.some((each) => each.profileId === picked) ? picked : (friends[0]?.profileId ?? "");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const { live, liveError, retryLive } = useLiveCoop();

  // The host's wait: the room names its match once the guest joins (B33), or is gone (B34).
  useEffect(() => {
    if (room === null) return undefined;
    let canceled = false;
    // A wake while a read is out would only repeat it; each effect run counts its own.
    let reading = false;
    const check = (): void => {
      if (reading) return;
      reading = true;
      getDuosRoom(room)
        .then(
          (answer) => {
            if (canceled || answer.matchId === null) return;
            storeRoom(null);
            setRoom(null);
            navigate(paths.duos(answer.matchId));
          },
          (cause: unknown) => {
            if (canceled || !(cause instanceof ApiRequestError) || cause.status !== 404) return;
            storeRoom(null);
            setRoom(null);
            setError(t("Your co-op room closed before anyone joined."));
          },
        )
        .finally(() => {
          reading = false;
        });
    };
    check();
    const timer = window.setInterval(check, COOP_ROOM_POLL_MS);
    // A throttled tab's interval stalls; waking it reads the room again at once.
    const stopWake = pageWake(check);
    return () => {
      canceled = true;
      stopWake();
      window.clearInterval(timer);
    };
  }, [room]);

  useEffect(() => {
    if (touchedDeck.current || decks.length === 0) return;
    const settled = defaultCoopDeck(decks, readCoopDeck())?.id ?? RANDOM;
    setDeck((current) => (current === settled ? current : settled));
  }, [decks]);

  // R2094000: the link is used once: the join form opens filled and Join takes the focus.
  useEffect(() => {
    forgetCoopLink();
  }, []);
  useEffect(() => {
    const code = readCoopLink();
    if (code === null) return;
    setJoinCode(code);
    focusPending.current = true;
  }, [search]);
  useEffect(() => {
    if (!focusPending.current || busy || room !== null) return;
    if (joinSubmit.current === null || joinSubmit.current.disabled) return;
    focusPending.current = false;
    joinSubmit.current.focus();
    forgetCoopLink();
  }, [busy, room, search, joinCode]);

  /** S6: exactly one of a saved deck and a random deal, the deal with its lean when there is one. */
  const choice = (): DuosDeckChoice =>
    deck === RANDOM ? { random: true, ...(lean === null ? {} : { leanSet: lean }) } : { deckId: deck };

  /** One request at a time; its refusal is shown, and a success clears the last one. */
  const run = (work: () => Promise<void>): void => {
    setBusy(true);
    setError(null);
    setStatus(null);
    work().then(
      () => {
        setBusy(false);
      },
      (cause: unknown) => {
        setBusy(false);
        setError(refusalWords(cause));
      },
    );
  };

  const create = (): void => {
    const pickedDeck = deck;
    run(async () => {
      const made = await createDuosRoom({ difficulty, ...choice() });
      writeCoopDeck(pickedDeck);
      storeRoom(made.code);
      setRoom(made.code);
    });
  };

  const cancel = (): void => {
    if (room === null) return;
    const code = room;
    run(async () => {
      await cancelDuosRoom(code);
      storeRoom(null);
      setRoom(null);
    });
  };

  const invite = (): void => {
    const friend = friends.find((each) => each.profileId === invitee);
    if (room === null || friend === undefined) return;
    const code = room;
    run(async () => {
      await sendChat(
        friend.profileId,
        t("Join my co-op game against the AI here: {link} (room code {code}).", { link: coopLinkUrl(code), code }),
      );
      setStatus(t("Invite sent to {name}.", { name: friend.username }));
    });
  };

  const sendLink = (): void => {
    if (room === null) return;
    const code = room;
    // R2094000: sharing failures stay silent, like the 1v1 room's link button.
    attempt(() => sendRoomLink(coopLinkUrl(code))).then(setLinkSent, () => undefined);
  };

  const join = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    const code = joinCode.trim().toUpperCase();
    if (code === "") return;
    const pickedDeck = deck;
    run(async () => {
      const joined = await joinDuosRoom(code, choice());
      writeCoopDeck(pickedDeck);
      navigate(paths.duos(joined.matchId));
    });
  };

  return (
    <section className="lobby-card coop-panel" aria-labelledby={headingId}>
      <h2 id={headingId} className="play-panel__heading">
        {t("Duos co-op")}
      </h2>
      {live === null ? null : (
        <a className="button-primary" data-testid={duosTestid.coopRejoin} href={paths.duos(live)} onClick={followInApp(paths.duos(live))}>
          {t("Back to your co-op game")}
        </a>
      )}
      {liveError === null ? null : (
        <>
          <p id={liveErrorId} className="coop-panel__error" role="alert">
            {t("Your live co-op game could not be checked: {message}", { message: t(liveError) })}
          </p>
          <div className="coop-panel__row">
            <button type="button" data-testid={duosTestid.coopLiveRetry} aria-describedby={liveErrorId} onClick={retryLive}>
              {t("Try again")}
            </button>
          </div>
        </>
      )}

      <div className="coop-panel__row">
        <label>
          {t("AI difficulty")}{" "}
          <select
            data-testid={duosTestid.coopDifficulty}
            value={difficulty}
            disabled={busy || room !== null}
            onChange={(event) => {
              const picked = DIFFICULTIES.find((each) => each === event.target.value);
              if (picked !== undefined) setDifficulty(picked);
            }}
          >
            {DIFFICULTIES.map((each) => (
              <option key={each} value={each}>
                {t(DIFFICULTY_LABEL[each])}
              </option>
            ))}
          </select>
        </label>
        <label>
          {t("Your deck")}{" "}
          <select
            data-testid={duosTestid.coopDeck}
            value={deck}
            disabled={busy || room !== null}
            onChange={(event) => {
              touchedDeck.current = true;
              setDeck(event.target.value);
              writeCoopDeck(event.target.value);
            }}
          >
            <option value={RANDOM}>{t("Random deck")}</option>
            {decks.map((each) =>
              deckPracticeOnly(each) ? (
                <option key={each.id} value={each.id} disabled>
                  {practiceOnlyLabel(each.name)}
                </option>
              ) : (
                <option key={each.id} value={each.id}>
                  {each.name}
                </option>
              ),
            )}
          </select>
        </label>
        {deck === RANDOM ? (
          <LeanSetPicker
            value={lean}
            disabled={busy || room !== null}
            testid={duosTestid.coopLeanNewest}
            onChange={(pick) => {
              setLean(pick);
              writePlayLean(pick);
            }}
          />
        ) : null}
      </div>
      {decks.some(deckPracticeOnly) ? (
        <p className="coop-panel__note" data-testid={duosTestid.coopPracticeOnly}>
          {t("A deck with a custom card plays only in practice, against the AI.")}
        </p>
      ) : null}
      {loadError === null ? null : (
        <>
          <p id={decksErrorId} className="coop-panel__error" role="alert">
            {t("Your decks could not be loaded: {message}", { message: t(loadError) })}
          </p>
          <div className="coop-panel__row">
            <button type="button" data-testid={duosTestid.coopDecksRetry} aria-describedby={decksErrorId} onClick={retry}>
              {t("Try again")}
            </button>
          </div>
        </>
      )}

      {room === null ? (
        <div className="coop-panel__row">
          <button type="button" className="button-primary" data-testid={duosTestid.coopCreate} disabled={busy} onClick={create}>
            {t("Create a co-op room")}
          </button>
        </div>
      ) : (
        <div className="coop-panel__room">
          <p className="coop-panel__row">
            {t("Room code")}{" "}
            <output className="coop-panel__code" data-testid={duosTestid.coopCode}>
              {room}
            </output>
          </p>
          <p className="coop-panel__status" role="status">
            {t("Waiting for your teammate to join…")}
          </p>
          <div className="coop-panel__row">
            <select
              data-testid={duosTestid.coopInviteFriend}
              value={invitee}
              disabled={busy || friends.length === 0}
              aria-label={t("Friend to invite")}
              onChange={(event) => {
                setPicked(event.target.value);
              }}
            >
              {friends.length === 0 ? <option value="">{t("No friends to invite yet")}</option> : null}
              {friends.map((friend) => (
                <option key={friend.profileId} value={friend.profileId}>
                  {friend.username}
                </option>
              ))}
            </select>
            <button type="button" data-testid={duosTestid.coopInvite} disabled={busy || invitee === ""} onClick={invite}>
              {t("Invite")}
            </button>
            <button type="button" data-testid={duosTestid.coopLink} disabled={busy} onClick={sendLink}>
              {linkSent === "shared" ? t("Link shared") : linkSent === "copied" ? t("Link copied") : t("Copy invite link")}
            </button>
            <button type="button" data-testid={duosTestid.coopCancel} disabled={busy} onClick={cancel}>
              {t("Close the room")}
            </button>
          </div>
        </div>
      )}

      <form className="coop-panel__row" onSubmit={join}>
        <label htmlFor={joinId}>{t("Join a co-op room")}</label>
        <input
          id={joinId}
          data-testid={duosTestid.coopJoinInput}
          value={joinCode}
          autoComplete="off"
          spellCheck={false}
          placeholder={t("Code")}
          disabled={busy || room !== null}
          onChange={(event) => {
            setJoinCode(event.target.value);
          }}
        />
        <button
          type="submit"
          ref={joinSubmit}
          data-testid={duosTestid.coopJoinSubmit}
          disabled={busy || room !== null || joinCode.trim() === ""}
        >
          {t("Join")}
        </button>
      </form>

      {status === null ? null : (
        <p className="coop-panel__status" data-testid={duosTestid.coopStatus} role="status">
          {status}
        </p>
      )}
      {error === null ? null : (
        <p className="coop-panel__error" data-testid={duosTestid.coopError} role="alert">
          {error}
        </p>
      )}
    </section>
  );
}
