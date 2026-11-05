// `/account`'s other ways to sign in (R3025000): each OAuth provider this build names (R666), linked to
// this account or not. Linking leaves for the provider and comes back through `/login`; unlinking never
// takes the account's last way in.

import { useEffect, useState, type ReactElement } from "react";

import {
  AUTH_MESSAGES,
  OAUTH_PROVIDER_LABELS,
  authConfig,
  authMessageOf,
  listIdentities,
  oauthProviders,
  startIdentityLink,
  unlinkIdentity,
  type LinkedIdentity,
  type OAuthProvider,
} from "../net/auth.ts";
import { t } from "../i18n/i18n.ts";
import { forgetLinkAfterSignIn, linkAfterSignIn } from "../net/session.ts";
import { takeLinkResult } from "./redirect.ts";

import "./two-step.css";

export const linkedTestid = {
  section: "account-linked",
  row: "account-linked-row",
  link: "account-linked-link",
  unlink: "account-linked-unlink",
  retry: "account-linked-retry",
  notice: "account-linked-notice",
  moving: "account-linked-moving",
  error: "account-linked-error",
} as const;

type Phase = { kind: "loading" } | { kind: "ready"; identities: LinkedIdentity[] } | { kind: "unknown" };

/** Nothing on a build that names no provider, or has no auth provider at all. */
export default function LinkedAccounts({ token }: { token: string }): ReactElement | null {
  const [providers] = useState(() => (authConfig() === null ? [] : oauthProviders()));
  return providers.length > 0 ? <LinkedPanel token={token} providers={providers} /> : null;
}

function LinkedPanel({ token, providers }: { token: string; providers: OAuthProvider[] }): ReactElement {
  const [phase, setPhase] = useState<Phase>({ kind: "loading" });
  const [attempt, setAttempt] = useState(0);
  const [busy, setBusy] = useState(false);
  const [result] = useState(takeLinkResult);
  // R3147002: the provider of the new account this device removed, to link here; said once.
  const [moving] = useState(() => providers.find((provider) => provider === linkAfterSignIn()) ?? null);
  useEffect(() => {
    forgetLinkAfterSignIn();
  }, []);
  const [notice, setNotice] = useState<string | null>(result === "linked" ? t("Linked. You can sign in with it now.") : null);
  // A link that came back with an error: the provider gives no reason worth showing.
  const [error, setError] = useState<string | null>(result === "failed" ? t(AUTH_MESSAGES.linkRefused) : null);

  useEffect(() => {
    let canceled = false;
    listIdentities(token)
      .then((identities) => {
        if (!canceled) setPhase({ kind: "ready", identities });
      })
      .catch(() => {
        if (!canceled) setPhase({ kind: "unknown" });
      });
    return () => {
      canceled = true;
    };
  }, [token, attempt]);

  const link = (provider: OAuthProvider): void => {
    setBusy(true);
    setNotice(null);
    setError(null);
    startIdentityLink(provider, token).catch((cause: unknown) => {
      setError(t(authMessageOf(cause)));
      setBusy(false);
    });
  };

  const unlink = (identity: LinkedIdentity): void => {
    setBusy(true);
    setNotice(null);
    setError(null);
    unlinkIdentity(token, identity.id)
      .then(() => {
        setNotice(t("Unlinked. It no longer signs in to this account."));
        setPhase({ kind: "loading" });
        setAttempt((before) => before + 1);
      })
      .catch((cause: unknown) => {
        setError(t(authMessageOf(cause)));
      })
      .finally(() => {
        setBusy(false);
      });
  };

  return (
    <div className="account-two-step" data-testid={linkedTestid.section}>
      <h3 className="account-heading">{t("Other ways to sign in")}</h3>
      {moving !== null && phase.kind === "ready" && !phase.identities.some((entry) => entry.provider === moving) ? (
        <p className="notice" data-testid={linkedTestid.moving} role="status">
          {t("Link {provider} below to finish: from then on it signs in to this account.", {
            provider: OAUTH_PROVIDER_LABELS[moving],
          })}
        </p>
      ) : null}
      {phase.kind === "loading" ? <p>{t("Loading…")}</p> : null}
      {phase.kind === "unknown" ? (
        <div className="account-actions">
          <p>{t("Couldn't load your sign-in methods.")}</p>
          <button
            type="button"
            className="link-button"
            data-testid={linkedTestid.retry}
            onClick={() => {
              setPhase({ kind: "loading" });
              setAttempt((before) => before + 1);
            }}
          >
            {t("Try again")}
          </button>
        </div>
      ) : null}
      {phase.kind === "ready"
        ? providers.map((provider) => {
            const label = OAUTH_PROVIDER_LABELS[provider];
            const identity = phase.identities.find((entry) => entry.provider === provider);
            // The provider refuses the last way in too; this only never offers it.
            const removable = identity !== undefined && phase.identities.length > 1;
            return (
              <div
                key={provider}
                className="account-actions"
                data-testid={linkedTestid.row}
                data-provider={provider}
                data-linked={identity === undefined ? "false" : "true"}
              >
                <p>
                  {identity === undefined
                    ? t("{provider}: not linked.", { provider: label })
                    : t("{provider}: linked. You can sign in with it.", { provider: label })}
                </p>
                {identity === undefined ? (
                  <button
                    type="button"
                    data-testid={linkedTestid.link}
                    data-provider={provider}
                    disabled={busy}
                    onClick={() => {
                      link(provider);
                    }}
                  >
                    {t("Link {provider}", { provider: label })}
                  </button>
                ) : removable ? (
                  <button
                    type="button"
                    className="link-button"
                    data-testid={linkedTestid.unlink}
                    data-provider={provider}
                    disabled={busy}
                    onClick={() => {
                      unlink(identity);
                    }}
                  >
                    {t("Unlink {provider}", { provider: label })}
                  </button>
                ) : null}
              </div>
            );
          })
        : null}
      {notice !== null ? (
        <p className="notice" data-testid={linkedTestid.notice} role="status">
          {notice}
        </p>
      ) : null}
      {error !== null ? (
        <p className="notice" data-testid={linkedTestid.error} role="alert">
          {error}
        </p>
      ) : null}
    </div>
  );
}
