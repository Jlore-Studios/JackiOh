// "Let friends watch my games" (#936, S7, B50): the account's `allowSpectators` from `/api/settings`.
// It is the account's, not the device's, so it is no part of the settings store or its sync
// (`settings/accountSync.ts`) and no slot a tab reset would touch (`settings/SettingsPanel.tsx`).
// It draws nothing until the account's value is in: signed out, a pending account and a server from
// before #936 have no such setting. Who may watch is still the server's (CLAUDE.md rule 7).

import { useEffect, useState, type ReactElement } from "react";

import { t, useLocale } from "../i18n/i18n.ts";
import { apiRequest, getPlayerSettings, type PlayerSettingsAccountCopy } from "../net/api.ts";
import { withSession } from "../net/gate.ts";
import { readSession } from "../net/session.ts";
import { messageOf } from "./session.ts";
import { SPECTATORS_SETTING_TESTID } from "./testids.ts";

import "../settings/settings.css";

/** With no `groups`, the server writes `allowSpectators` alone (`crates/server/src/api/settings.rs`). */
function putAllowSpectators(token: string, allow: boolean): Promise<{ settings: PlayerSettingsAccountCopy }> {
  return apiRequest<{ settings: PlayerSettingsAccountCopy }>("/api/settings", {
    method: "PUT",
    token,
    body: { allowSpectators: allow },
  });
}

export function SpectatorSwitch(): ReactElement | null {
  useLocale();
  const [allowed, setAllowed] = useState<boolean | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (readSession() === null) return;
    let canceled = false;
    withSession(getPlayerSettings).then(
      (settings) => {
        if (!canceled && typeof settings.allowSpectators === "boolean") setAllowed(settings.allowSpectators);
      },
      () => undefined,
    );
    return () => {
      canceled = true;
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
          onChange={(event) => {
            change(event.currentTarget.checked);
          }}
        />
      </label>
      {error === null ? null : (
        <p className="notice" role="alert">
          {t(error)}
        </p>
      )}
    </div>
  );
}
