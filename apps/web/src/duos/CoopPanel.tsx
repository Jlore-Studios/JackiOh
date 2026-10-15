// The co-op panel on Play together (#936 S8, B60): make a Duos room against the AI team, share its
// code (in a chat message to a friend, or by hand), join a friend's by its code, and go to the board
// once both are in.
//
// CLAUDE.md rule 7: the panel proposes and the server rules. The AI's difficulty, the deck or a random
// deal (and, for a random one, "More cards from the newest set", R1372) are what the player chose; the
// server checks them, makes the room, and answers a join with the match or a refusal shown in a
// player's words. The host waits on the room: it is read every COOP_ROOM_POLL_MS until it names its
// match, and then both players are on the board (the guest goes straight from its join). A room the
// tab opened is remembered for the tab (COOP_ROOM_KEY, try/catch), so a reload picks the wait up.

import { useEffect, useId, useState, type FormEvent, type ReactElement } from "react";

import { DIFFICULTIES, type Difficulty } from "@jackioh/engine/config";

import { t, useLocale } from "../i18n/i18n.ts";
import { ApiRequestError, getDecks, type SavedDeck } from "../net/api.ts";
import {
  cancelDuosRoom,
  createDuosRoom,
  getDuosLive,
  getDuosRoom,
  joinDuosRoom,
} from "../net/duos.ts";
import { withSession } from "../net/gate.ts";
import { navigate, paths } from "../net/navigate.ts";
import { sendChat, type Friend } from "../net/social.ts";
import { followInApp } from "../routes/nav.tsx";
import { COOP_ROOM_KEY, COOP_ROOM_POLL_MS } from "./config.ts";
import { duosTestid } from "./testids.ts";
import "./duos.css";

/** The deck select's value for a dealt deck. */
const RANDOM = "random";

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

/** A refusal (B33's `full` and `own_room`, B34's unknown code, …) in the server's own words, never reworded here. */
function refusalWords(cause: unknown): string {
  return cause instanceof Error ? t(cause.message) : String(cause);
}

/** The account's saved decks by name, for the deck choice; none when they cannot be read. */
function useSavedDecks(): readonly SavedDeck[] {
  const [decks, setDecks] = useState<readonly SavedDeck[]>([]);
  useEffect(() => {
    let cancelled = false;
    withSession(getDecks).then(
      (answer) => {
        if (!cancelled) setDecks(answer.decks);
      },
      () => undefined,
    );
    return () => {
      cancelled = true;
    };
  }, []);
  return decks;
}

export function CoopPanel({ friends }: { friends: readonly Friend[] }): ReactElement {
  useLocale();
  const headingId = useId();
  const joinId = useId();
  const decks = useSavedDecks();
  const [difficulty, setDifficulty] = useState<Difficulty>("easy");
  const [deck, setDeck] = useState<string>(RANDOM);
  const [leanNewest, setLeanNewest] = useState(false);
  const [room, setRoom] = useState<string | null>(readStoredRoom);
  const [joinCode, setJoinCode] = useState("");
  const [picked, setPicked] = useState("");
  // The friend the invite goes to: the one picked, else the first listed, so one friend needs no pick.
  const invitee = friends.some((each) => each.profileId === picked) ? picked : (friends[0]?.profileId ?? "");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [live, setLive] = useState<string | null>(null);

  // B42: a co-op game the account is already in is offered first.
  useEffect(() => {
    let cancelled = false;
    getDuosLive().then(
      (answer) => {
        if (!cancelled) setLive(answer.matchId);
      },
      () => undefined,
    );
    return () => {
      cancelled = true;
    };
  }, []);

  // The host's wait: the room names its match once the guest joins (B33), or is gone (B34).
  useEffect(() => {
    if (room === null) return undefined;
    let cancelled = false;
    const check = (): void => {
      getDuosRoom(room).then(
        (answer) => {
          if (cancelled || answer.matchId === null) return;
          storeRoom(null);
          setRoom(null);
          navigate(paths.duos(answer.matchId));
        },
        (cause: unknown) => {
          if (cancelled || !(cause instanceof ApiRequestError) || cause.status !== 404) return;
          storeRoom(null);
          setRoom(null);
          setError(t("Your co-op room closed before anyone joined."));
        },
      );
    };
    check();
    const timer = window.setInterval(check, COOP_ROOM_POLL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [room]);

  /** S6: exactly one of a saved deck and a random deal. */
  const choice = (): { deckId?: string; random?: boolean; leanNewest?: boolean } =>
    deck === RANDOM ? { random: true, ...(leanNewest ? { leanNewest: true } : {}) } : { deckId: deck };

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
    run(async () => {
      const made = await createDuosRoom({ difficulty, ...choice() });
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
      await sendChat(friend.profileId, t("Join my co-op game against the AI: the room code is {code}.", { code }));
      setStatus(t("Invite sent to {name}.", { name: friend.username }));
    });
  };

  const join = (event: FormEvent<HTMLFormElement>): void => {
    event.preventDefault();
    const code = joinCode.trim().toUpperCase();
    if (code === "") return;
    run(async () => {
      const joined = await joinDuosRoom(code, choice());
      navigate(paths.duos(joined.matchId));
    });
  };

  return (
    <section className="lobby-card coop-panel" aria-labelledby={headingId}>
      <h2 id={headingId} className="play-panel__heading">
        {t("Duos co-op")}
      </h2>
      <p className="lobby-note">
        {t("Team up with a friend against two AI players: your team shares a pool of life, passes cards and ends its turns together.")}
      </p>
      {live === null ? null : (
        <a className="button-primary" data-testid={duosTestid.coopRejoin} href={paths.duos(live)} onClick={followInApp(paths.duos(live))}>
          {t("Back to your co-op game")}
        </a>
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
              setDeck(event.target.value);
            }}
          >
            <option value={RANDOM}>{t("Random deck")}</option>
            {decks.map((each) => (
              <option key={each.id} value={each.id}>
                {each.name}
              </option>
            ))}
          </select>
        </label>
        {deck === RANDOM ? (
          <label>
            <input
              type="checkbox"
              data-testid={duosTestid.coopLeanNewest}
              checked={leanNewest}
              disabled={busy || room !== null}
              onChange={(event) => {
                setLeanNewest(event.currentTarget.checked);
              }}
            />{" "}
            {t("More cards from the newest set")}
          </label>
        ) : null}
      </div>

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
        <button type="submit" data-testid={duosTestid.coopJoinSubmit} disabled={busy || room !== null || joinCode.trim() === ""}>
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

export default CoopPanel;
