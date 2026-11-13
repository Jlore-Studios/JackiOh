// A friend's lobby invite (R3730005), mounted once beside the challenge toast (`main.tsx`): the social
// socket's `lobbyInvite` frame shows a toast with Open, which opens the lobby, and Dismiss. R3729002: the tab
// keeps each invite in `inbox/invites.ts`, so Open and Dismiss only send the toast away and the bell's inbox
// still lists the invite (Join, Dismiss there) until it is answered or `INBOX_INVITE_KEEP_MS` has passed; the
// server sends the frame once, so a tab with no socket open still misses it. CLAUDE.md rule 7: Open only goes
// to the lobby; the lobby's own screen enters it, and the server decides.

import { useEffect, type ReactElement } from "react";

import { hideInviteToast, noteInvite, useInvites } from "../inbox/invites.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { navigate, paths } from "../net/navigate.ts";
import { onLobbyInvitePush } from "../together/socialPoll.ts";
import { lobbyTestid } from "./testids.ts";
import "./lobby.css";

export function LobbyInviteToast(): ReactElement | null {
  useLocale();
  const invites = useInvites().filter((invite) => !invite.toastHidden);
  useEffect(() => onLobbyInvitePush((invite) => noteInvite(invite)), []);
  if (invites.length === 0) return null;
  return (
    <div className="lobby-invite-toasts" role="region" aria-label={t("Lobby invites")}>
      {invites.map((invite) => (
        <div key={invite.code} className="lobby-invite-toast" role="status" data-testid={lobbyTestid.inviteToast}>
          <p>
            {invite.format === "coop"
              ? t("{name} invites you to a co-op lobby", { name: invite.from.username })
              : t("{name} invites you to a 1v1 lobby", { name: invite.from.username })}
          </p>
          <div className="lobby-invite-toast__actions">
            <button
              type="button"
              className="button-primary"
              data-testid={lobbyTestid.inviteOpen}
              onClick={() => {
                hideInviteToast(invite.code);
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
                hideInviteToast(invite.code);
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
