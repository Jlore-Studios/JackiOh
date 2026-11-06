// The username prompt (R1435). The gate decides nothing (CLAUDE.md rule 7): the prompt is owed by
// `/api/auth/me`'s `promptOwed`. After a pick or a skip nothing waits on the gate's re-read, so one that
// fails leaves the prompt owed, and the player has answered.
// THE GATE'S FRAME is `ShellPanel`'s markup, written out here because main.tsx imports this file.

import { useState, type ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import { skipUsernamePrompt } from "../net/api.ts";
import { announceAccountChange } from "../net/gate.ts";
import { signOut, signOutLabel, useSigningOut } from "../routes/account.tsx";
import { PageTitle } from "../routes/PageTitle.tsx";
import { Wordmark } from "../routes/Wordmark.tsx";
import { tokenMetadata } from "./claims.ts";
import Username from "./Username.tsx";
import UsernameField from "./UsernameField.tsx";
import { shellTestid, usernameTestid } from "./testids.ts";

import "./tavern.css";

/** A skip that failed for any reason: the server's prompt is still owed, and Skip still works. */
export const USERNAME_SKIP_FAILED = "Couldn't skip just now. Try again in a moment.";

export type UsernamePromptProps = {
  token: string;
  /** The username the account holds now (`Player#n`). */
  name: string;
  /** The server took a pick or a skip: the gate opens its screen. */
  onAnswered: () => void;
};

/** The provider's own username first (OIDC's `preferred_username`, Supabase's `user_name`), then its nickname. */
const PROVIDER_USERNAME_CLAIMS = ["preferred_username", "user_name", "nickname"] as const;

/**
 * R3147003: the username the account's sign-in provider gave it (Roblox's, R3147000), which the prompt
 * starts from. Read from the token's `user_metadata`, the user's own to edit, so it is only a first value:
 * the server's preview and save judge it as any typed name.
 */
export function providerUsername(token: string): string {
  for (const claim of PROVIDER_USERNAME_CLAIMS) {
    const name = tokenMetadata(token, "user_metadata", claim)?.trim();
    if (name !== undefined && name.length > 0) return name;
  }
  return "";
}

export default function UsernamePrompt({ token, name, onAnswered }: UsernamePromptProps): ReactElement {
  const leaving = useSigningOut();
  const [suggested] = useState(() => providerUsername(token));
  const [skipping, setSkipping] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // "You're Player#7 for now. …": split around the name, so English keeps the text nodes it had.
  const [heldBefore = "", heldAfter = ""] = t(
    "You're {name} for now. Pick the name other players will see; you can change it later on your account page.",
  ).split("{name}");

  const skip = (): void => {
    if (skipping) return;
    setSkipping(true);
    setError(null);
    skipUsernamePrompt(token)
      .then(() => {
        onAnswered();
        announceAccountChange();
      })
      .catch(() => {
        setError(USERNAME_SKIP_FAILED);
      })
      .finally(() => {
        setSkipping(false);
      });
  };

  return (
    <div className="app-shell tavern shell-panel" data-testid={usernameTestid.prompt}>
      <section className="panel panel--auth">
        <Wordmark />
        <PageTitle>{t("Choose your username")}</PageTitle>
        <p>
          {heldBefore}
          <Username name={name} />
          {heldAfter}
        </p>
        <UsernameField
          token={token}
          initial={suggested}
          disabled={skipping}
          onSaved={() => {
            onAnswered();
            announceAccountChange();
          }}
        />
        {error !== null ? (
          <p className="notice" data-testid={usernameTestid.promptError} role="alert">
            {t(error)}
          </p>
        ) : null}
        <div className="row">
          <button
            type="button"
            className="link-button"
            data-testid={usernameTestid.promptSkip}
            disabled={skipping}
            aria-busy={skipping}
            onClick={skip}
          >
            {skipping ? t("Skipping…") : t("Skip for now")}
          </button>
          <button
            type="button"
            className="link-button"
            data-testid={shellTestid.signOut}
            disabled={leaving}
            aria-busy={leaving}
            onClick={() => {
              signOut();
            }}
          >
            {signOutLabel(leaving)}
          </button>
        </div>
      </section>
    </div>
  );
}
