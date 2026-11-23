// Sign-out, shared by the gate's own panels. The gate offers Sign out on screens that failed to
// load anything else, so this module stays in the entry chunk on purpose and imports nothing
// from `routes/account.tsx`; the account screen itself rides `routeChunks.account`.

import { useSyncExternalStore } from "react";

import { AUTH_SIGN_OUT_WAIT_SECONDS } from "@jackioh/server-config";

import { forgetPushOnSignOut } from "../daily/push.ts";
import { t } from "../i18n/i18n.ts";
import { revokeSignedOutSession, sessionNearExpiry } from "../net/auth.ts";
import { paths } from "../net/navigate.ts";
import { clearSession, forgetLinkAfterSignIn, forgetPendingAddresses, readSession } from "../net/session.ts";

/**
 * Clears the token, revokes it at the provider (R194: a copied refresh token must not keep renewing
 * sessions) and does a REAL navigation: `useAccount` reads the session once on mount, so a pushState
 * would leave mounted gates holding the old account. A live session's revoke goes with `keepalive`,
 * not awaited; an expired one must be renewed before the provider revokes it, so sign-out waits for
 * both, at most `AUTH_SIGN_OUT_WAIT_SECONDS`, then loads the landing page whatever happened.
 */
export function signOut(): void {
  // One at a time: during an expired session's renewal the device's copy is already gone, so a
  // second press would load the page at once and cut off the revocation. It joins the first instead.
  if (signingOut) return;
  const session = readSession();
  // #3727: the server forgets this device's daily push subscription while the token still works.
  if (session !== null) forgetPushOnSignOut(session.accessToken);
  clearSession();
  // Whatever sign-up or reset this browser was waiting on ends with it (R193): an address left
  // armed here would accept the next person's link, or one planted by someone else.
  forgetPendingAddresses();
  // R3147002: and so does a provider waiting to be linked to the account signed in next.
  forgetLinkAfterSignIn();
  let left = false;
  const leave = (): void => {
    if (left || typeof window === "undefined") return;
    left = true;
    window.location.assign(paths.landing);
  };
  if (session === null) {
    leave();
    return;
  }
  const revoking = revokeSignedOutSession(session);
  if (!sessionNearExpiry(session, Date.now())) {
    leave();
    return;
  }
  setSigningOut(true);
  const timer = setTimeout(() => {
    leave();
    setSigningOut(false);
  }, AUTH_SIGN_OUT_WAIT_SECONDS * 1000);
  void revoking.finally(() => {
    clearTimeout(timer);
    leave();
    // The page is leaving; a press in the moment before it goes finds no session and just leaves.
    setSigningOut(false);
  });
}

/** True from a sign-out that has to wait (an expired session) until the page leaves. */
let signingOut = false;
const signingOutListeners = new Set<() => void>();

function setSigningOut(value: boolean): void {
  signingOut = value;
  for (const listener of signingOutListeners) listener();
}

/** Test seam: a sign-out whose page load a test stubbed never ends on its own. */
export function resetSigningOutForTests(): void {
  setSigningOut(false);
}

/** Whether a sign-out is under way: Sign out buttons show "Signing out…", disabled, till the page leaves. */
export function useSigningOut(): boolean {
  return useSyncExternalStore(
    (listener) => {
      signingOutListeners.add(listener);
      return () => {
        signingOutListeners.delete(listener);
      };
    },
    () => signingOut,
    () => false,
  );
}

export function signOutLabel(busy: boolean): string {
  return busy ? t("Signing out…") : t("Sign out");
}
