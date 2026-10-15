// "Let friends watch my games" (#936, S7, B50): the account's `allowSpectators`, read from
// `GET /api/settings` and written with `PUT /api/settings`, which takes it beside the device groups
// (sent empty here, so no group changes). It is the account's, not the device's, so it is no part of
// the settings store or its sync (`settings/accountSync.ts`) and no slot a tab reset would touch: the
// panel's Gameplay tab draws it after its slots (`settings/SettingsPanel.tsx`), and Play together
// shows it too.
//
// It draws nothing until the account's value is in: signed out, a pending account and a server from
// before #936 have no such setting. Who may actually watch is still the server's (CLAUDE.md rule 7).

import { useEffect, useId, useState, type ReactElement } from "react";

import { SPECTATE_DELAY_MS } from "@jackioh/server-config";

import { t, useLocale } from "../i18n/i18n.ts";
import { apiRequest, getPlayerSettings, type PlayerSettingsAccountCopy } from "../net/api.ts";
import { withSession } from "../net/gate.ts";
import { readSession } from "../net/session.ts";
import { MS_PER_SECOND } from "../game/clockConstants.ts";
import { messageOf } from "./session.ts";
import { SPECTATORS_SETTING_TESTID } from "./testids.ts";

import "../settings/settings.css";

/** `PUT /api/settings` with the account's `allowSpectators` and no device group. */
function putAllowSpectators(token: string, allow: boolean): Promise<{ settings: PlayerSettingsAccountCopy }> {
  return apiRequest<{ settings: PlayerSettingsAccountCopy }>("/api/settings", {
    method: "PUT",
    token,
    body: { groups: {}, allowSpectators: allow },
  });
}

export function SpectatorSwitch(): ReactElement | null {
  useLocale();
  const hintId = useId();
  const [allowed, setAllowed] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (readSession() === null) return;
    let cancelled = false;
    withSession(getPlayerSettings).then(
      (settings) => {
        if (!cancelled && typeof settings.allowSpectators === "boolean") setAllowed(settings.allowSpectators);
      },
      () => undefined,
    );
    return () => {
      cancelled = true;
    };
  }, []);

  if (allowed === null) return null;

  const change = (next: boolean): void => {
    setAllowed(next);
    setBusy(true);
    setError(null);
    withSession((token) => putAllowSpectators(token, next))
      .then(
        (answer) => {
          if (typeof answer.settings.allowSpectators === "boolean") setAllowed(answer.settings.allowSpectators);
        },
        (cause: unknown) => {
          setAllowed(!next);
          setError(messageOf(cause));
        },
      )
      .finally(() => {
        setBusy(false);
      });
  };

  return (
    <div className="settings-row">
      <label className="settings-control">
        <span className="settings-label">{t("Let friends watch my games")}</span>
        <input
          type="checkbox"
          role="switch"
          className="settings-switch"
          data-testid={SPECTATORS_SETTING_TESTID}
          checked={allowed}
          disabled={busy}
          aria-describedby={hintId}
          onChange={(event) => {
            change(event.currentTarget.checked);
          }}
        />
      </label>
      <p className="settings-hint" id={hintId}>
        {t("Friends can watch your unranked online games from your seat, {seconds} seconds behind. Ranked games are never watched.", {
          seconds: String(Math.round(SPECTATE_DELAY_MS / MS_PER_SECOND)),
        })}
      </p>
      {error === null ? null : (
        <p className="notice" role="alert">
          {t(error)}
        </p>
      )}
    </div>
  );
}
