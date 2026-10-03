// The Account tab's one control (issue #129, R633, R634): where the player's settings are kept, and
// whether the account has the latest. It reads the sync's status (`accountSync.ts`) and decides
// nothing: every setting is saved on this device the moment it changes, signed in or not.

import type { ReactElement } from "react";

import { currentPath, navigate, paths } from "../net/navigate.ts";
import { rememberReturnTo } from "../net/return-to.ts";
import { readSession } from "../net/session.ts";
import { retrySettingsSync, useSettingsSyncState } from "./accountSync.ts";

function clockTime(at: number): string {
  try {
    return new Date(at).toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
  } catch {
    return "";
  }
}

export default function AccountSettings(): ReactElement {
  const sync = useSettingsSyncState();
  const signedIn = readSession() !== null;

  let message: string;
  if (sync.phase === "syncing") message = "Saving to your account…";
  else if (sync.phase === "saved") message = "Saved to your account.";
  else if (sync.phase === "error") {
    message = "Couldn't reach your account. Your settings are saved on this device and are sent when it answers.";
  } else if (signedIn) {
    message = "Saved on this device. They reach your account the next time you open Play, Decks or Practice.";
  } else {
    message = "Saved on this device only. Sign in to keep them with your account, on every device you play on.";
  }

  return (
    <div className="settings-account" data-testid="settings-account" data-phase={sync.phase}>
      <p className="settings-account__status" role="status">
        {message}
        {sync.phase === "saved" && sync.savedAt !== null ? (
          <>
            {" "}
            <time dateTime={new Date(sync.savedAt).toISOString()}>{clockTime(sync.savedAt)}</time>
          </>
        ) : null}
      </p>
      {sync.phase === "error" ? (
        <button type="button" className="settings-account__action" data-testid="settings-sync-retry" onClick={retrySettingsSync}>
          Try again
        </button>
      ) : null}
      {sync.phase === "signedOut" && !signedIn ? (
        <a
          className="settings-account__action"
          data-testid="settings-sign-in"
          href={paths.login}
          onClick={(event) => {
            event.preventDefault();
            rememberReturnTo(currentPath());
            navigate(paths.login);
          }}
        >
          Sign in
        </a>
      ) : null}
    </div>
  );
}
