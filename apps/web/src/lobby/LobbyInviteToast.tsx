// A friend's lobby invite (R3730005), mounted once beside the challenge toast (`main.tsx`): the social
// socket's `lobbyInvite` frame shows a toast with Open, which opens the lobby, and Dismiss. Nothing keeps
// an invite, so a tab with no socket open misses it. CLAUDE.md rule 7: Open only goes to the lobby; the
// lobby's own screen enters it, and the server decides.

import { useEffect, useState, type ReactElement } from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import { navigate, paths } from "../net/navigate.ts";
import { onLobbyInvitePush, type LobbyInvite } from "../together/socialPoll.ts";
import { lobbyTestid } from "./testids.ts";
import "./lobby.css";

export function LobbyInviteToast(): ReactElement | null {
  useLocale();
  const [invites, setInvites] = useState<readonly LobbyInvite[]>([]);
  useEffect(
    () =>
      onLobbyInvitePush((invite) => {
        setInvites((before) => [...before.filter((each) => each.code !== invite.code), invite]);
      }),
    [],
  );
  if (invites.length === 0) return null;
  const drop = (code: string): void => {
    setInvites((before) => before.filter((each) => each.code !== code));
  };
  return (
    <div className="lobby-invite-toasts" role="region" aria-label={t("Lobby invites")}>
      {invites.map((invite) => (
        <div key={invite.code} className="lobby-invite-toast" role="status" data-testid={lobbyTestid.inviteToast}>
          <p>
            {invite.format === "coop"
              ? t("{name} invites you to a co-op lobby", { name: invite.from.username })
              : invite.format === "2v2"
                ? t("{name} invites you to a 2v2 lobby", { name: invite.from.username })
                : t("{name} invites you to a 1v1 lobby", { name: invite.from.username })}
          </p>
          <div className="lobby-invite-toast__actions">
            <button
              type="button"
              className="button-primary"
              data-testid={lobbyTestid.inviteOpen}
              onClick={() => {
                drop(invite.code);
                navigate(paths.lobby(invite.code));
              }}
            >
              {t("Open")}
            </button>
            <button
              type="button"
              className="button-secondary"
              data-testid={lobbyTestid.inviteDismiss}
              onClick={() => {
                drop(invite.code);
              }}
            >
              {t("Dismiss")}
            </button>
          </div>
        </div>
      ))}
    </div>
  );
}
