// R3147002: "Already have a JackiOh account?" A sign-in with a provider that matched no account made a new
// one (Roblox names no address, so it never matches one; R3147000). This removes that new account, as "Delete
// my account" does, so the provider is free to be linked to the account the player already had (R3025000):
// the device signs out, `/login` says what to do, and the sign-in after it opens `/account`.
// Only a pending account is offered it (it holds nothing yet), and only when it signed up with a provider
// this build names. The token's `app_metadata` only says which; the server decides the deletion.

import { useState, type ReactElement } from "react";

import { t } from "../i18n/i18n.ts";
import { deleteAccount } from "../net/api.ts";
import { OAUTH_PROVIDER_LABELS, oauthProviders, type OAuthProvider } from "../net/auth.ts";
import { messageOf } from "../net/errors.ts";
import { paths } from "../net/navigate.ts";
import { clearSession, forgetPendingAddresses, rememberLinkAfterSignIn } from "../net/session.ts";
import { tokenMetadata } from "./claims.ts";

export const moveTestid = {
  root: "move-to-existing",
  open: "move-to-existing-open",
  confirm: "move-to-existing-confirm",
  keep: "move-to-existing-keep",
  error: "move-to-existing-error",
} as const;

/** The provider this account signed up with, when this build offers it. */
export function signedUpWith(token: string): OAuthProvider | null {
  const name = tokenMetadata(token, "app_metadata", "provider");
  return oauthProviders().find((provider) => provider === name) ?? null;
}

export default function MoveToExisting({ token }: { token: string }): ReactElement | null {
  const [provider] = useState(() => signedUpWith(token));
  const [asking, setAsking] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  if (provider === null) return null;
  const label = OAUTH_PROVIDER_LABELS[provider];

  const remove = (): void => {
    if (busy) return;
    setBusy(true);
    setError(null);
    deleteAccount(token)
      .then(() => {
        // As after "Delete my account": nothing to revoke, the account is gone.
        clearSession();
        forgetPendingAddresses();
        rememberLinkAfterSignIn(provider);
        window.location.assign(paths.login);
      })
      .catch((cause: unknown) => {
        setBusy(false);
        setError(messageOf(cause));
      });
  };

  return (
    <div className="move-to-existing" data-testid={moveTestid.root} data-provider={provider}>
      {asking ? (
        <>
          <p>
            {t(
              "Signing in with {provider} made this new account. To sign in with {provider} to the account you already have, this new one is removed first. Then sign in to your account and link {provider} on its account page.",
              { provider: label },
            )}
          </p>
          {/* R3221000: the safe choice first, the act last. */}
          <div className="row">
            <button
              type="button"
              className="link-button"
              data-testid={moveTestid.keep}
              disabled={busy}
              onClick={() => {
                setAsking(false);
                setError(null);
              }}
            >
              {t("Keep it")}
            </button>
            <button type="button" data-testid={moveTestid.confirm} disabled={busy} aria-busy={busy} onClick={remove}>
              {busy ? t("Removing…") : t("Remove this new account")}
            </button>
          </div>
          {error !== null ? (
            <p className="notice" data-testid={moveTestid.error} role="alert">
              {t(error)}
            </p>
          ) : null}
        </>
      ) : (
        <button
          type="button"
          className="link-button"
          data-testid={moveTestid.open}
          onClick={() => {
            setAsking(true);
          }}
        >
          {t("Already have a JackiOh account?")}
        </button>
      )}
    </div>
  );
}
