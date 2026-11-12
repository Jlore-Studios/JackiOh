// The private lobby screen, `/lobby/:code` (SPEC §9.5, §13.9; R3730000 to R3730006): one screen for a 1v1
// lobby and a co-op lobby. Its seats show who is in, an empty seat invites a friend, each member picks
// their own deck and presses Ready, the host sets the game and presses Start, and a countdown opens the
// board. With no lobby under the code it is the classic room's join card, so an old link still joins.
//
// CLAUDE.md rule 7: the screen sends intent and draws the server's view. Who may enter, what a pick may
// be, whether the Start may go and every seat's state are the server's; a refusal is shown in its words.
// The view holds no card and no 1v1 opponent's deck. It reads again on every `lobby` push over the social
// socket, every LOBBY_POLL_MS while the socket is down, and as the lobby's time runs out.

import { useCallback, useEffect, useId, useRef, useState, type FormEvent, type ReactElement, type RefObject } from "react";

import { DIFFICULTIES, type Difficulty } from "@jackioh/engine/config";
import { LOBBY_START_COUNTDOWN_SECONDS } from "@jackioh/server-config";
import type { LeanPick } from "@jackioh/shared";

import { LeanSetPicker, readPlayLean, writePlayLean } from "../game/LeanNewest.tsx";
import { t, useLocale } from "../i18n/i18n.ts";
import { ApiRequestError, joinRoom, type ModeChoice, type QueueMode, type SavedDeck } from "../net/api.ts";
import { joinDuosRoom } from "../net/duos.ts";
import { MODE_LABEL } from "../net/liveGame.ts";
import { navigate, paths } from "../net/navigate.ts";
import { lobbyLinkUrl, sendRoomLink } from "../net/roomLink.ts";
import { getFriends, type Friend } from "../net/social.ts";
import { attempt, messageOf } from "../net/errors.ts";
import { QUEUE_MODES, Verdict, choiceFor, defaultDeck, defaultTrio, useLobbyData, verdictFor, type LobbyData } from "../routes/play.tsx";
import { BackLink, followInApp } from "../routes/nav.tsx";
import { PAGE_TITLE_ID } from "../routes/PageTitle.tsx";
import { onFriendsPush, onLobbyPush, useSocialPoll } from "../together/socialPoll.ts";
import { STATUS_WORDS } from "../together/words.tsx";
import {
  enterLobby,
  getLobby,
  inviteToLobby,
  leaveLobby,
  pickLobby,
  readyLobby,
  removeFromLobby,
  setLobbySettings,
  startLobby,
  type CoopPick,
  type CoopRoomView,
  type LobbyRead,
  type LobbySeat,
  type LobbyView,
  type RoomView,
} from "./api.ts";
import { LOBBY_COUNTDOWN_TICK_MS, LOBBY_POLL_MS } from "./config.ts";
import { canStart, deckWords, seatsOf, type SeatShown } from "./seats.ts";
import { lobbyTestid } from "./testids.ts";
import "./lobby.css";

/** Unit conversion, not configuration. */
const MS_PER_SECOND = 1000;

/** The co-op deck select's value for a random deck. */
const RANDOM = "random";

export const DIFFICULTY_LABEL: Readonly<Record<Difficulty, string>> = { easy: "Easy", medium: "Medium", hard: "Hard" };

/** The board a started lobby opens: a series for Conquest, a co-op board, or a match (R3730002). */
export function boardOf(view: LobbyView): string | null {
  if (view.seriesId !== null) return paths.series(view.seriesId);
  if (view.matchId === null) return null;
  return view.settings.format === "coop" ? paths.duos(view.matchId) : paths.match(view.matchId);
}

/** A refusal in the server's words. */
function refusalOf(cause: unknown): string {
  return t(messageOf(cause));
}

type Read =
  | { kind: "loading" }
  | { kind: "ready"; value: LobbyRead }
  | { kind: "closed" }
  | { kind: "failed"; message: string };

/** The lobby under `code` as the server shows it, read again on `reload`, a push, a poll and its expiry. */
function useLobbyRead(code: string): [Read, () => void] {
  const [read, setRead] = useState<Read>({ kind: "loading" });
  const [round, setRound] = useState(0);
  const social = useSocialPoll();
  const reload = useCallback((): void => {
    setRound((before) => before + 1);
  }, []);
  useEffect(() => {
    let canceled = false;
    attempt(() => getLobby(code)).then(
      (value) => {
        if (!canceled) setRead({ kind: "ready", value });
      },
      (cause: unknown) => {
        if (canceled) return;
        if (cause instanceof ApiRequestError && cause.status === 404) setRead({ kind: "closed" });
        else setRead({ kind: "failed", message: messageOf(cause) });
      },
    );
    return () => {
      canceled = true;
    };
  }, [code, round]);
  // R3730000: a push names the lobby that changed.
  useEffect(() => onLobbyPush((pushed) => pushed === code && reload()), [code, reload]);
  // While the socket is down, the lobby reads itself at its own pace.
  useEffect(() => {
    if (social.live) return undefined;
    const timer = window.setInterval(reload, LOBBY_POLL_MS);
    return () => {
      window.clearInterval(timer);
    };
  }, [social.live, reload]);
  // R3730004: an idle lobby closes at its expiry, so the screen reads it then.
  const expiresAt = read.kind === "ready" && read.value.kind === "lobby" ? read.value.expiresAt : null;
  useEffect(() => {
    if (expiresAt === null) return undefined;
    const timer = window.setTimeout(reload, Math.max(0, expiresAt - Date.now()) + MS_PER_SECOND);
    return () => {
      window.clearTimeout(timer);
    };
  }, [expiresAt, reload]);
  return [read, reload];
}

export type LobbyScreenProps = { token: string; code: string };

export function LobbyScreen({ token, code }: LobbyScreenProps): ReactElement {
  useLocale();
  const [read, reload] = useLobbyRead(code);
  // R3730004: a member who reads the lobby as a visitor was removed from it.
  const wasMember = useRef(false);
  const view = read.kind === "ready" && read.value.kind === "lobby" ? read.value : null;
  const removed = view !== null && view.you === null && wasMember.current;
  if (view !== null && view.you !== null) wasMember.current = true;

  let body: ReactElement;
  if (read.kind === "loading") {
    body = (
      <p className="lobby-note" role="status">
        {t("Loading the lobby…")}
      </p>
    );
  } else if (read.kind === "closed") {
    body = (
      <section className="lobby-card lobby-closed" data-testid={lobbyTestid.closed}>
        <p>{t("This lobby is closed.")}</p>
        <a className="button-primary" href={paths.play} onClick={followInApp(paths.play)}>
          {t("Back to the lobby")}
        </a>
      </section>
    );
  } else if (read.kind === "failed") {
    body = (
      <section className="lobby-card" role="alert" data-testid={lobbyTestid.error}>
        <p>{t("The lobby could not be read: {message}", { message: t(read.message) })}</p>
        <button type="button" className="button-secondary" onClick={reload}>
          {t("Try again")}
        </button>
      </section>
    );
  } else if (read.value.kind === "lobby") {
    body = <Lobby token={token} view={read.value} removed={removed} reload={reload} />;
  } else {
    body = <RoomJoinCard token={token} room={read.value} />;
  }
  return (
    <div className="lobby-screen" data-testid={lobbyTestid.screen}>
      <BackLink title={t("Private lobby")} />
      {body}
    </div>
  );
}

/** The lobby itself: its code, its seats, its settings, the member's deck, invites and the actions. */
function Lobby({ token, view, removed, reload }: { token: string; view: LobbyView; removed: boolean; reload: () => void }): ReactElement {
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState<"code" | "shared" | "copied" | null>(null);
  const inviteHeading = useRef<HTMLHeadingElement>(null);

  /** One request at a time; its refusal is shown, and every answer reads the lobby again. */
  const run = (work: () => Promise<unknown>): void => {
    setBusy(true);
    setError(null);
    attempt(work)
      .then(
        () => undefined,
        (cause: unknown) => {
          setError(refusalOf(cause));
        },
      )
      .finally(() => {
        setBusy(false);
        reload();
      });
  };

  const host = view.you === "host";
  const member = view.you !== null;
  const full = view.seats.every((seat) => seat.player !== null);
  const started = view.startedAt !== null;

  return (
    <section className="lobby-card lobby" aria-labelledby={PAGE_TITLE_ID} data-format={view.settings.format}>
      <header className="lobby__head">
        <div className="lobby__code">
          <span className="lobby__code-label">{t("Code")}</span>
          <code data-testid={lobbyTestid.code}>{view.code}</code>
          <button
            type="button"
            className="button-secondary"
            data-testid={lobbyTestid.copyCode}
            onClick={() => {
              void attempt(() => navigator.clipboard.writeText(view.code)).then(() => {
                setCopied("code");
              }, () => undefined);
            }}
          >
            {t("Copy")}
          </button>
          <button
            type="button"
            className="button-secondary"
            data-testid={lobbyTestid.copyLink}
            onClick={() => {
              void attempt(() => sendRoomLink(lobbyLinkUrl(view.code))).then(setCopied, () => undefined);
            }}
          >
            {t("Copy link")}
          </button>
          {copied === null ? null : (
            <span className="lobby-note" role="status">
              {copied === "shared" ? t("Link shared.") : t("Copied.")}
            </span>
          )}
        </div>
      </header>

      {removed ? (
        <p className="lobby__notice" role="status" data-testid={lobbyTestid.removed}>
          {t("You were removed from this lobby.")}
        </p>
      ) : null}

      <div className="lobby__body">
        <div className="lobby__seats">
          {seatsOf(view).map((team, index) => (
            <div key={index === 0 ? "a" : "b"} className="lobby__team" data-team={index === 0 ? "a" : "b"}>
              {team.map((shown, place) => (
                <Seat
                  key={`${String(index)}-${String(place)}`}
                  shown={shown}
                  hostView={host}
                  duel={view.settings.format === "duel"}
                  busy={busy || started}
                  onInvite={() => inviteHeading.current?.focus()}
                  onEnter={() => {
                    run(() => enterLobby(view.code));
                  }}
                  onRemove={(profileId) => {
                    run(() => removeFromLobby(view.code, profileId));
                  }}
                />
              ))}
            </div>
          ))}
        </div>

        <div className="lobby__side">
          <Settings view={view} disabled={!host || busy || started} onChange={(change) => {
            run(() => setLobbySettings(view.code, change));
          }} />
          {member ? <YourDeck token={token} view={view} busy={busy || started} run={run} /> : null}
          {member && !full && !started ? <InviteList code={view.code} heading={inviteHeading} run={run} /> : null}
        </div>
      </div>

      {error === null ? null : (
        <p className="lobby__error" role="alert" data-testid={lobbyTestid.error}>
          {error}
        </p>
      )}
      {started ? <Countdown view={view} /> : null}

      <footer className="lobby__actions">
        {member ? (
          <button
            type="button"
            className="button-secondary"
            data-testid={lobbyTestid.leave}
            disabled={busy}
            onClick={() => {
              run(async () => {
                await leaveLobby(view.code);
                navigate(view.settings.format === "coop" ? paths.together : paths.play);
              });
            }}
          >
            {t("Leave")}
          </button>
        ) : null}
        {host ? (
          <button
            type="button"
            className="button-primary"
            data-testid={lobbyTestid.start}
            disabled={busy || !canStart(view)}
            onClick={() => {
              run(() => startLobby(view.code));
            }}
          >
            {t("Start")}
          </button>
        ) : null}
      </footer>
    </section>
  );
}

/** One seat: a player, an empty seat to invite into or take, or one of the AI's. */
function Seat({
  shown,
  hostView,
  duel,
  busy,
  onInvite,
  onEnter,
  onRemove,
}: {
  shown: SeatShown;
  hostView: boolean;
  duel: boolean;
  busy: boolean;
  onInvite: () => void;
  onEnter: () => void;
  onRemove: (profileId: string) => void;
}): ReactElement {
  if (shown.kind === "ai") {
    return (
      <div className="lobby-seat lobby-seat--ai" data-testid={lobbyTestid.seat} data-seat="ai">
        <span className="lobby-seat__name">{t("AI team")}</span>
        <span className="lobby-seat__state">{shown.label}</span>
      </div>
    );
  }
  if (shown.kind === "invite") {
    return (
      <button type="button" className="lobby-seat lobby-seat--empty" data-testid={lobbyTestid.invite} data-seat="invite" disabled={busy} onClick={onInvite}>
        <span aria-hidden="true">+</span> {t("Invite a friend")}
      </button>
    );
  }
  if (shown.kind === "open") {
    return (
      <div className="lobby-seat lobby-seat--empty" data-seat="open">
        <button type="button" className="button-primary" data-testid={lobbyTestid.enter} disabled={busy} onClick={onEnter}>
          {t("Join lobby")}
        </button>
      </div>
    );
  }
  const { seat } = shown;
  const player = seat.player;
  return (
    <div className="lobby-seat" data-testid={lobbyTestid.seat} data-seat={seat.role} data-state={seat.state} data-you={shown.you ? "true" : "false"}>
      <span className="lobby-seat__portrait" aria-hidden="true">
        {player?.username.slice(0, 1) ?? "?"}
      </span>
      <span className="lobby-seat__name">
        {player?.username ?? ""}
        {shown.host ? (
          <span className="lobby-seat__crown" title={t("Host")} aria-label={t("Host")}>
            ♛
          </span>
        ) : null}
      </span>
      {player === null ? null : <span className="lobby-seat__presence">{t(STATUS_WORDS[player.status])}</span>}
      <SeatDeck seat={seat} duel={duel} you={shown.you} />
      {hostView && !shown.host && player !== null ? (
        <button
          type="button"
          className="link-button"
          data-testid={lobbyTestid.remove}
          disabled={busy}
          onClick={() => {
            onRemove(player.profileId);
          }}
        >
          {t("Remove")}
        </button>
      ) : null}
    </div>
  );
}

/** A seat's deck line: its state, and the deck's name where it may be shown. */
function SeatDeck({ seat, duel, you }: { seat: LobbySeat; duel: boolean; you: boolean }): ReactElement {
  const named = seat.state === "ready" && (seat.deck !== null || seat.random) && (you || !duel);
  return (
    <span className="lobby-seat__state" data-state={seat.state}>
      {named ? `${seat.deck ?? t("Random deck")} · ` : null}
      {deckWords(seat)}
    </span>
  );
}

/** R3730003: the host's settings, which everyone sees and only the host changes. */
function Settings({
  view,
  disabled,
  onChange,
}: {
  view: LobbyView;
  disabled: boolean;
  onChange: (change: { mode: QueueMode } | { difficulty: Difficulty }) => void;
}): ReactElement {
  const id = useId();
  const host = view.you === "host";
  if (view.settings.format === "duel") {
    const mode = view.settings.mode;
    return (
      <div className="lobby__setting">
        <label htmlFor={id}>{t("Format")}</label>
        {host ? (
          <select
            id={id}
            className="lobby-select"
            data-testid={lobbyTestid.settings}
            value={mode}
            disabled={disabled}
            onChange={(event) => {
              const picked = QUEUE_MODES.find((each) => each === event.currentTarget.value);
              if (picked !== undefined) onChange({ mode: picked });
            }}
          >
            {QUEUE_MODES.map((each) => (
              <option key={each} value={each}>
                {t("1v1 · {mode}", { mode: t(MODE_LABEL[each]) })}
              </option>
            ))}
          </select>
        ) : (
          <output id={id} data-testid={lobbyTestid.settings}>
            {t("1v1 · {mode}", { mode: t(MODE_LABEL[mode]) })}
          </output>
        )}
      </div>
    );
  }
  const difficulty = view.settings.difficulty;
  return (
    <div className="lobby__setting">
      <label htmlFor={id}>{t("Co-op against the AI team")}</label>
      {host ? (
        <select
          id={id}
          className="lobby-select"
          data-testid={lobbyTestid.settings}
          value={difficulty}
          disabled={disabled}
          onChange={(event) => {
            const picked = DIFFICULTIES.find((each) => each === event.currentTarget.value);
            if (picked !== undefined) onChange({ difficulty: picked });
          }}
        >
          {DIFFICULTIES.map((each) => (
            <option key={each} value={each}>
              {t(DIFFICULTY_LABEL[each])}
            </option>
          ))}
        </select>
      ) : (
        <output id={id} data-testid={lobbyTestid.settings}>
          {t(DIFFICULTY_LABEL[difficulty])}
        </output>
      )}
    </div>
  );
}

/** What the member has picked here: a deck, a trio, or a random deck with its lean. */
type LocalPick = { deckId: string | null; trioId: string | null; random: boolean; lean: LeanPick | null };

/** The pick the screen starts on: the server's when the member made one, else the defaults. */
function startingPick(view: LobbyView, data: LobbyData | null): LocalPick {
  const sent = view.yourPick;
  const lean = sent?.mode === "random" ? (sent.leanSet ?? null) : readPlayLean();
  const deckId = sent?.mode === "bo1" ? sent.deckId : (defaultDeck(data?.decks ?? [], null)?.id ?? null);
  const trioId = sent?.mode === "bo3" ? sent.trioId : (defaultTrio(data?.trios ?? [], null)?.id ?? null);
  const random = view.settings.format === "coop" ? sent?.mode === "random" || (sent === null && deckId === null) : view.settings.mode === "random";
  return { deckId, trioId, random, lean };
}

/** The request a local pick sends: the queue's choice in a duel, a co-op room's in co-op. */
function pickBody(view: LobbyView, pick: LocalPick, data: LobbyData | null): ModeChoice | CoopPick | null {
  if (view.settings.format === "coop") {
    if (pick.random || pick.deckId === null) return pick.lean === null ? { random: true } : { random: true, leanSet: pick.lean };
    return { deckId: pick.deckId };
  }
  const deck = data?.decks.find((each) => each.id === pick.deckId) ?? null;
  const trio = data?.trios.find((each) => each.id === pick.trioId) ?? null;
  return choiceFor(view.settings.mode, deck, trio, pick.lean);
}

/** Whether `body` is what the server already holds as the member's pick. */
function samePick(view: LobbyView, body: ModeChoice | CoopPick): boolean {
  const sent = view.yourPick;
  if (sent === null) return false;
  if ("random" in body) return sent.mode === "random" && (sent.leanSet ?? null) === (body.leanSet ?? null);
  if ("deckId" in body && !("mode" in body)) return sent.mode === "bo1" && sent.deckId === body.deckId;
  return JSON.stringify(sent) === JSON.stringify(body);
}

/** R3730001: the member's own deck, trio or random deck, with the validator's verdict, and Ready. */
function YourDeck({
  token,
  view,
  busy,
  run,
}: {
  token: string;
  view: LobbyView;
  busy: boolean;
  run: (work: () => Promise<unknown>) => void;
}): ReactElement {
  const id = useId();
  const [load, retryLoad] = useLobbyData(token);
  const data = load.kind === "ready" ? load.data : null;
  const [pick, setPick] = useState<LocalPick>(() => startingPick(view, data));
  // The defaults settle once the decks arrive, and again when the host's change takes the pick off.
  const settled = useRef<string | null>(null);
  const settleKey = `${data === null ? "-" : "data"}:${JSON.stringify(view.settings)}:${view.yourPick === null ? "none" : "sent"}`;
  useEffect(() => {
    if (settled.current === settleKey) return;
    settled.current = settleKey;
    setPick(startingPick(view, data));
  }, [settleKey, view, data]);

  const mine = view.seats.find((seat) => seat.role === view.you) ?? null;
  const ready = mine?.state === "ready";
  const body = pickBody(view, pick, data);
  const duelMode = view.settings.format === "duel" ? view.settings.mode : null;
  const deck = data?.decks.find((each) => each.id === pick.deckId) ?? null;
  const trio = data?.trios.find((each) => each.id === pick.trioId) ?? null;
  const verdict =
    data === null || pick.random ? null : verdictFor(duelMode ?? "bo1", duelMode === "bo3" ? null : deck, trio, data);

  const send = (next: LocalPick): void => {
    setPick(next);
    const nextBody = pickBody(view, next, data);
    if (nextBody !== null) run(() => pickLobby(view.code, nextBody));
  };

  return (
    <div className="lobby__deck">
      <label htmlFor={id}>{t("Your deck")}</label>
      {duelMode === "bo3" ? (
        <select
          id={id}
          className="lobby-select"
          data-testid={lobbyTestid.trioSelect}
          value={pick.trioId ?? ""}
          disabled={busy || data === null}
          onChange={(event) => {
            send({ ...pick, trioId: event.currentTarget.value });
          }}
        >
          {(data?.trios ?? []).map((saved) => (
            <option key={saved.id} value={saved.id}>
              {saved.name}
            </option>
          ))}
        </select>
      ) : duelMode === "random" ? (
        <p className="lobby-note">{t("No deck needed: the server deals both of you one when the game starts.")}</p>
      ) : (
        <select
          id={id}
          className="lobby-select"
          data-testid={lobbyTestid.deckSelect}
          value={pick.random ? RANDOM : (pick.deckId ?? "")}
          disabled={busy || data === null}
          onChange={(event) => {
            const value = event.currentTarget.value;
            send(value === RANDOM ? { ...pick, random: true } : { ...pick, random: false, deckId: value });
          }}
        >
          {view.settings.format === "coop" ? <option value={RANDOM}>{t("A random deck")}</option> : null}
          {(data?.decks ?? []).map((saved: SavedDeck) => (
            <option key={saved.id} value={saved.id}>
              {saved.name}
            </option>
          ))}
        </select>
      )}
      {pick.random ? (
        <LeanSetPicker
          testid={lobbyTestid.lean}
          value={pick.lean}
          disabled={busy}
          onChange={(lean) => {
            writePlayLean(lean);
            send({ ...pick, lean });
          }}
        />
      ) : null}
      {load.kind === "failed" && duelMode !== "random" ? <DecksFailed message={load.message} retry={retryLoad} /> : null}
      <Verdict result={verdict} />
      <button
        type="button"
        className="button-secondary lobby__ready"
        data-testid={lobbyTestid.ready}
        data-ready={ready ? "true" : "false"}
        aria-pressed={ready}
        disabled={busy || (!ready && body === null)}
        onClick={() => {
          run(async () => {
            if (ready) {
              await readyLobby(view.code, false);
              return;
            }
            if (body === null) return;
            // A pick shown here that the server does not hold yet is sent first (R3730001).
            if (!samePick(view, body)) await pickLobby(view.code, body);
            await readyLobby(view.code, true);
          });
        }}
      >
        {ready ? t("Not ready") : t("Ready")}
      </button>
    </div>
  );
}

/** #1568: decks that could not be read say why, with Try again, which the alert describes. */
function DecksFailed({ message, retry }: { message: string; retry: () => void }): ReactElement {
  const id = useId();
  return (
    <div className="lobby__failed">
      <p id={id} className="lobby__error" role="alert">
        {t("Your decks could not be loaded: {message}", { message: t(message) })}
      </p>
      <button type="button" className="button-secondary" data-testid={lobbyTestid.decksRetry} aria-describedby={id} onClick={retry}>
        {t("Try again")}
      </button>
    </div>
  );
}

/** R3730005: the member's friends, online first, each one Invite away from the empty seat. */
function InviteList({
  code,
  heading,
  run,
}: {
  code: string;
  heading: RefObject<HTMLHeadingElement | null>;
  run: (work: () => Promise<unknown>) => void;
}): ReactElement {
  const [friends, setFriends] = useState<readonly Friend[] | null>(null);
  const [round, setRound] = useState(0);
  const [sent, setSent] = useState<ReadonlySet<string>>(() => new Set());
  useEffect(() => {
    let canceled = false;
    attempt(() => getFriends()).then(
      (answer) => {
        if (!canceled) setFriends(answer.friends);
      },
      () => {
        if (!canceled) setFriends([]);
      },
    );
    return () => {
      canceled = true;
    };
  }, [round]);
  useEffect(() => onFriendsPush(() => {
    setRound((before) => before + 1);
  }), []);
  const order = { online: 0, inMatch: 1, offline: 2 } as const;
  const listed = [...(friends ?? [])].sort((a, b) => order[a.status] - order[b.status]);
  return (
    <div className="lobby__invites">
      <h2 className="lobby__subhead" tabIndex={-1} ref={heading}>
        {t("Invite")}
      </h2>
      {friends === null ? null : listed.length === 0 ? (
        <p className="lobby-note">{t("Add friends on Play together to invite them here, or share the code.")}</p>
      ) : (
        <ul className="lobby__friends">
          {listed.map((friend) => (
            <li key={friend.profileId} className="lobby__friend" data-status={friend.status}>
              <span className="lobby__friend-name">{friend.username}</span>
              <span className="lobby__friend-status">{t(STATUS_WORDS[friend.status])}</span>
              {friend.status === "offline" ? null : (
                <button
                  type="button"
                  className="button-secondary"
                  data-testid={lobbyTestid.inviteFriend}
                  disabled={sent.has(friend.profileId)}
                  onClick={() => {
                    run(async () => {
                      await inviteToLobby(code, friend.profileId);
                      setSent((before) => new Set(before).add(friend.profileId));
                    });
                  }}
                >
                  {sent.has(friend.profileId) ? t("Invited") : t("Invite")}
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/** R3730002: the countdown from the moment this screen saw the Start, then the board. */
function Countdown({ view }: { view: LobbyView }): ReactElement {
  const [seenAt] = useState(() => Date.now());
  const [now, setNow] = useState(seenAt);
  const board = boardOf(view);
  const left = Math.max(0, Math.ceil((seenAt + LOBBY_START_COUNTDOWN_SECONDS * MS_PER_SECOND - now) / MS_PER_SECOND));
  useEffect(() => {
    const timer = window.setInterval(() => {
      setNow(Date.now());
    }, LOBBY_COUNTDOWN_TICK_MS);
    return () => {
      window.clearInterval(timer);
    };
  }, []);
  useEffect(() => {
    if (left === 0 && board !== null) navigate(board, { force: true });
  }, [left, board]);
  return (
    <p className="lobby__countdown" role="status" data-testid={lobbyTestid.countdown}>
      {board === null ? t("Starting…") : t("The game starts in {seconds}…", { seconds: String(left) })}
    </p>
  );
}

/** R3730006: the classic room under the code, joined at once with a pick for its mode. */
function RoomJoinCard({ token, room }: { token: string; room: RoomView | CoopRoomView }): ReactElement {
  const id = useId();
  const [load, retryLoad] = useLobbyData(token);
  const data = load.kind === "ready" ? load.data : null;
  const [deckId, setDeckId] = useState<string | null>(null);
  const [trioId, setTrioId] = useState<string | null>(null);
  const [lean, setLean] = useState<LeanPick | null>(readPlayLean);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const mode: QueueMode = room.kind === "room" ? room.mode : "bo1";
  const deck = data?.decks.find((each) => each.id === deckId) ?? (room.kind === "room" ? defaultDeck(data?.decks ?? [], null) : null);
  const trio = data?.trios.find((each) => each.id === trioId) ?? defaultTrio(data?.trios ?? [], null);
  const random = room.kind === "coopRoom" ? deck === null : mode === "random";

  const join = (event: FormEvent): void => {
    event.preventDefault();
    setBusy(true);
    setError(null);
    attempt(async () => {
      if (room.kind === "coopRoom") {
        const joined = await joinDuosRoom(room.code, deck === null ? { random: true, ...(lean === null ? {} : { leanSet: lean }) } : { deckId: deck.id });
        navigate(paths.duos(joined.matchId), { force: true });
        return;
      }
      const choice = choiceFor(mode, deck, trio, lean);
      if (choice === null) throw new Error(t("Pick a deck first."));
      const joined = await joinRoom(token, room.code, choice);
      const board = joined.seriesId !== null ? paths.series(joined.seriesId) : joined.matchId !== null ? paths.match(joined.matchId) : paths.play;
      navigate(board, { force: true });
    }).then(
      () => undefined,
      (cause: unknown) => {
        setBusy(false);
        setError(refusalOf(cause));
      },
    );
  };

  return (
    <form className="lobby-card lobby-join-card" data-testid={lobbyTestid.joinCard} onSubmit={join}>
      <h2 className="lobby__title">
        {room.kind === "room"
          ? t("{name}'s room · {mode}", { name: room.host.username, mode: t(MODE_LABEL[mode]) })
          : t("{name}'s co-op room · {difficulty}", { name: room.host.username, difficulty: t(DIFFICULTY_LABEL[room.difficulty]) })}
      </h2>
      <p className="lobby-note">{t("This room starts the moment you join it.")}</p>
      {room.kind === "room" && mode === "bo3" ? (
        <>
          <label htmlFor={id}>{t("Your trio")}</label>
          <select id={id} className="lobby-select" data-testid={lobbyTestid.trioSelect} value={trio?.id ?? ""} disabled={busy} onChange={(event) => {
            setTrioId(event.currentTarget.value);
          }}>
            {(data?.trios ?? []).map((saved) => (
              <option key={saved.id} value={saved.id}>
                {saved.name}
              </option>
            ))}
          </select>
        </>
      ) : room.kind === "room" && mode === "random" ? null : (
        <>
          <label htmlFor={id}>{t("Your deck")}</label>
          <select id={id} className="lobby-select" data-testid={lobbyTestid.deckSelect} value={deck?.id ?? RANDOM} disabled={busy} onChange={(event) => {
            setDeckId(event.currentTarget.value === RANDOM ? null : event.currentTarget.value);
          }}>
            {room.kind === "coopRoom" ? <option value={RANDOM}>{t("A random deck")}</option> : null}
            {(data?.decks ?? []).map((saved) => (
              <option key={saved.id} value={saved.id}>
                {saved.name}
              </option>
            ))}
          </select>
        </>
      )}
      {load.kind === "failed" && !(room.kind === "room" && mode === "random") ? <DecksFailed message={load.message} retry={retryLoad} /> : null}
      {random ? <LeanSetPicker testid={lobbyTestid.lean} value={lean} disabled={busy} onChange={(pick) => {
        setLean(pick);
        writePlayLean(pick);
      }} /> : null}
      {error === null ? null : (
        <p className="lobby__error" role="alert" data-testid={lobbyTestid.error}>
          {error}
        </p>
      )}
      <button type="submit" className="button-primary" data-testid={lobbyTestid.joinRoom} disabled={busy}>
        {t("Join")}
      </button>
    </form>
  );
}

export default LobbyScreen;
