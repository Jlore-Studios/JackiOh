// The way into a private lobby (R3730000), where "Play a friend" stood on `/play` and the co-op panel on
// `/together`: Create a lobby in this screen's format, Join one by its code, and the player's own lobbies
// not started, so a reload finds its lobby again (R3730004). A co-op panel also makes a 2v2 lobby and lists
// the player's (R3731000), and offers the way back to a live co-op game (B42). CLAUDE.md rule 7: the server
// makes the lobby and refuses what it refuses.

import { useEffect, useId, useState, type FormEvent, type ReactElement } from "react";

import { canonicalCode } from "@jackioh/shared";
import { ROOM_CODE_FORMAT, ROOM_CODE_LENGTH } from "@jackioh/server-config";

import { t, useLocale } from "../i18n/i18n.ts";
import type { QueueMode } from "../net/api.ts";
import { getDuosLive } from "../net/duos.ts";
import { attempt, messageOf } from "../net/errors.ts";
import { MODE_LABEL } from "../net/liveGame.ts";
import { navigate, paths } from "../net/navigate.ts";
import { followInApp } from "../routes/nav.tsx";
import { createLobby, getMyLobbies, type LobbySettings, type MyLobby } from "./api.ts";
import { lobbyTestid } from "./testids.ts";
import "./lobby.css";

/** What a lobby of the list is, in words. */
function lobbyWords(lobby: MyLobby): string {
  if (lobby.settings.format === "duel") return t("1v1 · {mode}", { mode: t(MODE_LABEL[lobby.settings.mode]) });
  return lobby.settings.format === "2v2" ? t("2v2") : t("Co-op");
}

/** The screen a panel stands on: Play's 1v1 or Play together's co-op, which makes 2v2 lobbies too (R3731000). */
export type LobbyPanelFormat = "duel" | "coop";

export type LobbyPanelProps = {
  format: LobbyPanelFormat;
  /** A 1v1 lobby's first mode: the one the Play screen has chosen. */
  mode?: QueueMode;
};

export function LobbyPanel({ format, mode }: LobbyPanelProps): ReactElement {
  useLocale();
  const headingId = useId();
  const joinId = useId();
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [code, setCode] = useState("");
  const [mine, setMine] = useState<readonly MyLobby[]>([]);
  const liveErrorId = useId();
  const [live, setLive] = useState<string | null>(null);
  const [liveError, setLiveError] = useState<string | null>(null);
  const [liveRound, setLiveRound] = useState(0);

  useEffect(() => {
    let canceled = false;
    attempt(getMyLobbies).then(
      (answer) => {
        if (!canceled) setMine(answer.lobbies);
      },
      () => undefined,
    );
    return () => {
      canceled = true;
    };
  }, []);

  // B42: a co-op screen offers the way back to the live co-op game; #1900: a failed read says why, with Try again.
  useEffect(() => {
    if (format !== "coop") return undefined;
    let canceled = false;
    attempt(getDuosLive).then(
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
  }, [format, liveRound]);

  const create = (settings: LobbySettings): void => {
    setBusy(true);
    setError(null);
    attempt(() => createLobby(settings)).then(
      (made) => {
        navigate(paths.lobby(made.code));
      },
      (cause: unknown) => {
        setBusy(false);
        setError(t(messageOf(cause)));
      },
    );
  };

  const join = (event: FormEvent): void => {
    event.preventDefault();
    const canonical = canonicalCode(code, ROOM_CODE_FORMAT);
    if (canonical === null) {
      setError(t("A code is {length} letters and digits.", { length: String(ROOM_CODE_LENGTH) }));
      return;
    }
    navigate(paths.lobby(canonical));
  };

  const shown = mine.filter((lobby) => lobby.settings.format === format || (format === "coop" && lobby.settings.format === "2v2"));
  return (
    <section className="lobby-card lobby-panel" aria-labelledby={headingId} data-testid={lobbyTestid.panel} data-format={format}>
      <h2 id={headingId} className="play-panel__heading">
        {format === "duel" ? t("Play a friend") : t("Co-op with a friend")}
      </h2>
      {live === null ? null : (
        <a className="button-secondary" data-testid={lobbyTestid.rejoin} href={paths.duos(live)} onClick={followInApp(paths.duos(live))}>
          {t("Back to your co-op game")}
        </a>
      )}
      {liveError === null ? null : (
        <div className="lobby__failed">
          <p id={liveErrorId} className="lobby__error" role="alert">
            {t("Your live co-op game could not be checked: {message}", { message: t(liveError) })}
          </p>
          <button
            type="button"
            className="button-secondary"
            data-testid={lobbyTestid.liveRetry}
            aria-describedby={liveErrorId}
            onClick={() => {
              setLiveError(null);
              setLiveRound((before) => before + 1);
            }}
          >
            {t("Try again")}
          </button>
        </div>
      )}
      <p className="lobby-note">
        {format === "duel"
          ? t("Make a private lobby, invite a friend, and start when you are both ready.")
          : t("Make a co-op lobby, invite a friend, and take on the AI team together.")}
      </p>
      <button
        type="button"
        className="button-secondary"
        data-testid={lobbyTestid.create}
        disabled={busy}
        onClick={() => {
          create(format === "duel" ? { format, mode: mode ?? "bo1" } : { format, difficulty: "medium" });
        }}
      >
        {t("Create a lobby")}
      </button>
      {format === "coop" ? (
        <button
          type="button"
          className="button-secondary"
          data-testid={lobbyTestid.create2v2}
          disabled={busy}
          onClick={() => {
            create({ format: "2v2" });
          }}
        >
          {t("Create a 2v2 lobby")}
        </button>
      ) : null}
      {shown.length === 0 ? null : (
        <ul className="lobby-panel__mine" data-testid={lobbyTestid.mine}>
          {shown.map((lobby) => (
            <li key={lobby.code}>
              <a href={paths.lobby(lobby.code)} onClick={followInApp(paths.lobby(lobby.code))}>
                {t("Your lobby {code} · {what}", { code: lobby.code, what: lobbyWords(lobby) })}
              </a>
            </li>
          ))}
        </ul>
      )}
      <form className="lobby-panel__join" data-testid={lobbyTestid.joinForm} onSubmit={join}>
        <label htmlFor={joinId}>{t("Join by code")}</label>
        <div className="lobby-panel__row">
          <input
            id={joinId}
            data-testid={lobbyTestid.joinInput}
            value={code}
            autoComplete="off"
            spellCheck={false}
            placeholder={t("Code")}
            onChange={(event) => {
              setCode(event.target.value);
            }}
          />
          <button type="submit" className="button-secondary" data-testid={lobbyTestid.joinSubmit} disabled={code.trim() === ""}>
            {t("Join")}
          </button>
        </div>
      </form>
      {error === null ? null : (
        <p className="lobby__error" role="alert" data-testid={lobbyTestid.error}>
          {error}
        </p>
      )}
    </section>
  );
}
