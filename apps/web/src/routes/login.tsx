// `/login` presents provider-auth forms; `net/auth.ts` and the server decide account state and use
// neutral failures (SPEC §9.2; R160, R192).
// Scrub redirect links before rendering and exchange a PKCE code only with its originating browser (R323, R324).
// Check each link token's account at `/api/auth/me`, never trust its readable address claim; confirmation links
// never sign in (R193). Hold recovery links only after the player types the checked address; never prefill it.
// Renew URL tokens immediately, revoke every unheld link session (R194), and navigate only to fixed `paths` (B35).
// Email-code sign-in (R664), OAuth (R666), and MFA (R665) retain only their proven session; §9.4 still gates it.
// Keep neutral resend/reset notices, per-address cooldowns, an escape from every mode, and notices before their forms.

import { useEffect, useRef, useState, type FormEvent, type ReactElement } from "react";

import { AUTH_EMAIL_RESEND_COOLDOWN_SECONDS, GATE_SLOW_NOTICE_SECONDS } from "@jackioh/server-config";
import Address from "../auth/Address.tsx";
import { addressKey, deadlineAfter, useAddressCooldown } from "../auth/cooldown.ts";
import {
  clearConsumedAuthRedirect,
  consumeAuthRedirect,
  emailFromToken,
  holdRecoverySession,
  type SessionLinkType,
} from "../auth/redirect.ts";
import { loginTestid } from "../auth/testids.ts";
import { emailProblem, newPasswordProblem, normalizeEmail, requiredProblem } from "../auth/validation.ts";
import { ApiRequestError, getMe } from "../net/api.ts";
import {
  AUTH_MESSAGES,
  AUTH_NOTICES,
  AuthError,
  OAUTH_PROVIDER_LABELS,
  adoptSession,
  assuranceLevel,
  authConfig,
  exchangeAuthCode,
  oauthProviders,
  refreshSession,
  requestEmailSignIn,
  requestPasswordReset,
  resendConfirmation,
  revokeSignedOutSession,
  secondFactorFor,
  signIn,
  signInWithUsername,
  signUp,
  startOAuthSignIn,
  verifyEmailCode,
  verifySecondFactor,
  type CodeExchange,
  type OAuthProvider,
} from "../net/auth.ts";
import { forgetVerifier, newestFlow } from "../auth/pkce.ts";
import { loginModeOf, loginReasonOf, navigate, paths } from "../net/navigate.ts";
import {
  forgetPendingEmail,
  pendingEmail,
  pendingEmailEntry,
  pendingReset,
  pendingResetEntry,
  readSession,
  type PendingAddress,
  type Session,
} from "../net/session.ts";
import { signOut, signOutLabel, useSigningOut } from "./account.tsx";
import { BackLink, followInApp } from "./nav.tsx";
import { SiteFooter } from "./SiteFooter.tsx";

import "../auth/auth.css";
import "../auth/tavern.css";

/** Re-exported so every existing `import { loginTestid } from "./login.tsx"` keeps working. */
export { loginTestid };

/** The line under "Create account" that links the privacy policy. */
export const signUpPrivacyTestid = "login-sign-up-privacy";

/**
 * `claimReset`: a recovery link asked for elsewhere, waiting for the player to type their address.
 * `emailCode`: the email sign-in link and code (R664). `mfa`: a session waiting for the code from
 * the account's authenticator app (R665).
 */
type Mode = "signIn" | "signUp" | "forgot" | "claimReset" | "emailCode" | "mfa";

/**
 * R665: a session held for its second step, the factor whose code it needs, and what happens once it
 * has it: `signIn` keeps it as this browser's session; `recovery` hands the raised session back to
 * the recovery link being checked (the link's own state still owns it, so leaving revokes it).
 */
type SecondStep = { session: Session; factorId: string; then: "signIn" | "recovery" };

/** An emailed link that carries a session, waiting for the server to say whose it is. */
type SessionLink =
  | { kind: "session"; session: Session; linkType: SessionLinkType }
  | { kind: "recovery"; session: Session };

/** An emailed link: a session, or (R323) a PKCE code still to exchange for one. */
type PendingLink = SessionLink | { kind: "code"; code: string };

/**
 * The link's session while this screen still answers for it: the link's own until it is renewed,
 * then the renewal. `session` is null once it has been held for the reset screen or revoked.
 */
type LinkState = {
  session: Session | null;
  renewed: boolean;
  /** R323: the one exchange of a link's code, shared by StrictMode's two runs (a code works once). */
  exchange?: Promise<CodeExchange>;
};

/** What renewing a link's session on arrival found. */
type LinkRenewal =
  /** The link's refresh token is spent, and `LinkState.session` is the renewal. */
  | "renewed"
  /** The provider refused the refresh token: the link was opened before (or was never real). */
  | "spent"
  /** No refresh token, or the provider could not be reached: the link's own session stays. */
  | "kept";

/**
 * Renews the link's session once (see A LINK IS RENEWED AS SOON AS IT IS READ). Concurrent calls
 * share one provider request (`refreshSession`), so StrictMode's second run gets the same renewal.
 * A session released meanwhile (the player moved on and it was revoked) is not brought back.
 */
async function renewLinkOnce(state: LinkState): Promise<LinkRenewal> {
  if (state.renewed) return "renewed";
  const session = state.session;
  const refreshToken = session?.refreshToken;
  if (session === null || typeof refreshToken !== "string" || refreshToken.length === 0) return "kept";
  try {
    const next = await refreshSession(refreshToken);
    if (!state.renewed && state.session === session) state.session = next;
    state.renewed = true;
    return "renewed";
  } catch (cause) {
    return cause instanceof AuthError && cause.failure === "sessionEnded" ? "spent" : "kept";
  }
}

/**
 * R323: the link's code for its session, once. The session never passed through a URL, so there is
 * no copy in history to spend: it counts as renewed already.
 */
async function exchangeLinkOnce(state: LinkState, code: string): Promise<CodeExchange> {
  state.exchange ??= exchangeAuthCode(code);
  const exchange = await state.exchange;
  if (exchange.kind === "session" && !state.renewed && state.session === null) {
    state.session = exchange.session;
    state.renewed = true;
  }
  return exchange;
}

/** R665: the factor the link's exchange said its session still needs, or null. */
async function exchangedFactor(state: LinkState): Promise<string | null> {
  const exchange = await state.exchange;
  return exchange?.kind === "session" ? exchange.secondFactor : null;
}

/** The link's session is not kept: revoke it (see A LINK'S SESSION THAT IS NOT KEPT). */
function revokeLink(state: LinkState): void {
  const session = state.session;
  state.session = null;
  if (session !== null) void revokeSignedOutSession(session);
}

/** The link's session, handed over (to the reset screen): this screen no longer answers for it. */
function takeLink(state: LinkState): Session | null {
  const session = state.session;
  state.session = null;
  return session;
}

type Entry = {
  mode: Mode;
  sessionExpired: boolean;
  email: string;
  linkError: boolean;
  /** R666: an OAuth provider sent the player back with an error rather than a code. */
  oauthFailed: boolean;
  link: PendingLink | null;
};

/** What the server said about a link's token (`GET /api/auth/me`, which verifies it). */
type LinkCheck =
  | { kind: "verified"; email: string | null }
  /** The server refused the token: a spent, expired or forged link. */
  | { kind: "refused" }
  /** The server could not be reached or answered something else; nothing was learned. */
  | { kind: "unchecked" };

async function checkLink(accessToken: string): Promise<LinkCheck> {
  try {
    const me = await getMe(accessToken);
    return { kind: "verified", email: me.email };
  } catch (cause) {
    if (cause instanceof ApiRequestError && cause.status === 401) return { kind: "refused" };
    return { kind: "unchecked" };
  }
}

/** What a checked link leaves on this screen, when it does not move on from it. */
type LinkOutcome =
  | "none"
  | "checking"
  /** A confirmation link this browser did not start: the address is confirmed, sign in by hand. */
  | "confirmed"
  /** R663: an email change's confirmation link: the new address is the account's, sign in with it. */
  | "emailChanged"
  /** A recovery link whose account has no address to compare: nothing was held. */
  | "recoveryRefused"
  /** A recovery link asked for elsewhere: held back until the player types its address. */
  | "recoveryClaim"
  /** A dashboard invite: the account needs a password first. */
  | "invited"
  /** A confirmation link the server could not check. */
  | "unchecked"
  /** A recovery link the server could not check: kept, so it can be checked again. */
  | "recoveryUnchecked";

function sameAddress(a: string, b: string): boolean {
  return addressKey(a) === addressKey(b);
}

/** How the screen was opened. Runs once, in a state initializer, before the first render. */
function readEntry(): Entry {
  // The query first (a link is scrubbed with its query, though `/login`'s own never carries one).
  const search = window.location.search;
  const forgot = loginModeOf(search) === "forgot";
  const entry: Entry = {
    mode: forgot ? "forgot" : "signIn",
    sessionExpired: loginReasonOf(search) === "expired",
    // Empty, always: `mode=forgot` is a public link, and the address this browser last asked to
    // reset may be another person's (see NOTHING IS PREFILLED FROM STORAGE).
    email: "",
    linkError: false,
    oauthFailed: false,
    link: null,
  };

  const link = consumeAuthRedirect();
  switch (link.kind) {
    case "none":
      break;
    case "session":
      // Decided once the server has said whose token it is (see A LINK'S ADDRESS IS CHECKED).
      entry.mode = "signIn";
      entry.link = { kind: "session", session: link.session, linkType: link.linkType };
      break;
    case "recovery":
      entry.mode = "signIn";
      entry.link = { kind: "recovery", session: link.session };
      break;
    case "code":
      // Exchanged once mounted (R323), then decided like any other link.
      entry.mode = "signIn";
      entry.link = { kind: "code", code: link.code };
      break;
    case "error":
      entry.mode = "signIn";
      if (newestFlow() === "oauth") {
        // R666: the newest thing this browser asked for was an OAuth sign-in, and the provider sent
        // it back with an error (cancelled, or refused): not an emailed link's failure.
        forgetVerifier("oauth");
        entry.oauthFailed = true;
        break;
      }
      // `linkExpired` and `linkDenied` read the same sentence; the ways forward are the same too.
      entry.linkError = true;
      break;
  }
  return entry;
}

/**
 * The provider's per-address interval that a send this browser remembers (`pendingEmail`,
 * `pendingReset`) started, while it is still running: a reload does not forget it (R192).
 */
function runningInterval(entry: PendingAddress | null): { address: string; deadline: number } | null {
  if (entry === null) return null;
  const deadline = deadlineAfter(AUTH_EMAIL_RESEND_COOLDOWN_SECONDS, entry.at);
  return deadline > Date.now() ? { address: entry.address, deadline } : null;
}

/** Only our own sentences reach the screen; anything that is not an `AuthError` is a fault. */
function messageOf(cause: unknown): string {
  return cause instanceof AuthError ? cause.message : AUTH_MESSAGES.service;
}

/** True once `active` has held for `GATE_SLOW_NOTICE_SECONDS`, as the gate's own slow notice. */
function useRunningLong(active: boolean): boolean {
  const [late, setLate] = useState(false);
  useEffect(() => {
    if (!active) {
      setLate(false);
      return;
    }
    const timer = window.setTimeout(() => {
      setLate(true);
    }, GATE_SLOW_NOTICE_SECONDS * 1000);
    return () => {
      window.clearTimeout(timer);
    };
  }, [active]);
  return active && late;
}

/**
 * The account this browser is signed in as when the screen opens, or null. Its address is the
 * stored token's own claim, shown back only to whoever holds the token; `""` when it names none
 * (the end-to-end fixture sessions).
 */
function signedInAddress(): string | null {
  const session = readSession();
  if (session === null) return null;
  return emailFromToken(session.accessToken) ?? "";
}

export default function LoginRoute(): ReactElement {
  const [entry] = useState(readEntry);
  const [mode, setMode] = useState<Mode>(entry.mode);
  const [email, setEmail] = useState(entry.email);
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [emailError, setEmailError] = useState<string | null>(null);
  const [passwordError, setPasswordError] = useState<string | null>(null);
  const [sessionExpired, setSessionExpired] = useState(entry.sessionExpired);
  const [linkOutcome, setLinkOutcome] = useState<LinkOutcome>(entry.link === null ? "none" : "checking");
  const [linkError, setLinkError] = useState(entry.linkError);
  const [resendOffered, setResendOffered] = useState(entry.linkError);
  const [resendBusy, setResendBusy] = useState(false);
  /** A failed sign-in for the address this browser just signed up with (R160 stands: local state only). */
  const [confirmFirst, setConfirmFirst] = useState(false);
  /** The player has used the form, so a link still being checked no longer decides anything. */
  const acted = useRef(false);
  /** Bumped by "Try again" on a recovery link that could not be checked. */
  const [checkRound, setCheckRound] = useState(0);
  /** The emailed link's session while this screen answers for it (see `LinkState`). */
  const linkState = useRef<LinkState>({
    session: entry.link === null || entry.link.kind === "code" ? null : entry.link.session,
    renewed: false,
  });
  /** The checked address of a recovery link asked for elsewhere (`claimReset`). Never shown. */
  const claimAddress = useRef<string | null>(null);
  const mounted = useRef(false);
  const checkingLong = useRunningLong(linkOutcome === "checking");
  const [signedInAs] = useState(signedInAddress);
  const leaving = useSigningOut();
  const [resendRunning] = useState(() => runningInterval(pendingEmailEntry()));
  const [resetRunning] = useState(() => runningInterval(pendingResetEntry()));
  const resendCooldown = useAddressCooldown(resendRunning);
  const resetCooldown = useAddressCooldown(resetRunning);
  const resendWait = resendCooldown.secondsFor(email);
  const resetWait = resetCooldown.secondsFor(email);
  /** R664: the code from a sign-in email, or (R665) from an authenticator app. */
  const [code, setCode] = useState("");
  const [codeError, setCodeError] = useState<string | null>(null);
  /** R664: the address the sign-in code was mailed to; null until one was asked for. */
  const [codeSentTo, setCodeSentTo] = useState<string | null>(null);
  const codeCooldown = useAddressCooldown();
  const codeWait = codeCooldown.secondsFor(email);
  /** R665: the session waiting for its authenticator code (see `SecondStep`). */
  const secondStep = useRef<SecondStep | null>(null);
  /** R666: the OAuth providers this build offers. */
  const [providers] = useState(oauthProviders);
  const [oauthFailed, setOauthFailed] = useState(entry.oauthFailed);
  const noticeRef = useRef<HTMLParagraphElement>(null);
  /** Set when a notice should take focus once it renders (a sign-up that moved the form on). */
  const focusNotice = useRef(false);

  useEffect(() => {
    if (notice === null || !focusNotice.current) return;
    focusNotice.current = false;
    noticeRef.current?.focus();
  }, [notice]);

  // A link this screen still answers for when it really unmounts (being checked, kept for "Try
  // again", or waiting for its address) is revoked (see A LINK'S SESSION THAT IS NOT KEPT). After
  // the unmount, so StrictMode's rehearsal (which remounts at once) does not.
  useEffect(() => {
    mounted.current = true;
    const state = linkState.current;
    return () => {
      mounted.current = false;
      window.setTimeout(() => {
        if (mounted.current) return;
        revokeLink(state);
        // R665: a sign-in left at its second step is not kept, so it is revoked too.
        releaseSecondStep();
      }, 0);
    };
  }, []);

  // Act on an emailed link once mounted: renew it (spending the copy in history), then act only on
  // what the server says the renewed token is (R193). StrictMode's rehearsal runs this twice; the
  // first run's answer is dropped, and both share one renewal. The cached reading is released when
  // the screen really unmounts.
  useEffect(() => {
    const pending = entry.link;
    const state = linkState.current;
    let cancelled = false;
    const run = async (): Promise<void> => {
      if (pending === null) return;
      let link: SessionLink;
      if (pending.kind === "code") {
        // R323: the code for the link's session first, with the verifier this browser kept.
        let exchange: CodeExchange;
        try {
          exchange = await exchangeLinkOnce(state, pending.code);
        } catch {
          // The provider could not answer. The code is still good, but this screen says what it says
          // of any link it could not check.
          if (cancelled || acted.current) return;
          setLinkOutcome("unchecked");
          setResendOffered(true);
          return;
        }
        if (cancelled) {
          // The screen went for good while the code was exchanged: nobody holds the session now.
          if (!mounted.current) revokeLink(state);
          return;
        }
        if (exchange.kind === "elsewhere") {
          // R324: asked for on another device or browser. The address is confirmed; sign in.
          if (!acted.current) setLinkOutcome("confirmed");
          return;
        }
        if (exchange.kind === "refused") {
          if (acted.current) return;
          setLinkOutcome("none");
          setLinkError(true);
          setResendOffered(true);
          return;
        }
        if (exchange.flow === "magiclink" || exchange.flow === "oauth") {
          // R664, R666: a sign-in THIS browser asked for (only its verifier exchanges the code), so
          // unlike R193's links it signs in, past its second step if the account has one (R665).
          if (acted.current) {
            revokeLink(state);
            return;
          }
          const held = takeLink(state);
          if (held === null) return;
          setLinkOutcome("none");
          signInWith(held, exchange.secondFactor);
          return;
        }
        link =
          exchange.flow === "recovery"
            ? { kind: "recovery", session: exchange.session }
            : {
                kind: "session",
                session: exchange.session,
                linkType: exchange.flow === "email_change" ? "email_change" : "signup",
              };
      } else {
        link = pending;
      }
      const renewal = await renewLinkOnce(state);
      if (cancelled) return;
      const session = state.session;
      // Let go meanwhile: the player moved on, and it has been revoked.
      if (session === null) return;
      if (link.kind === "recovery" && renewal !== "spent" && assuranceLevel(session.accessToken) !== "aal2") {
        // R665: the server refuses an `aal1` token for an account with an authenticator app, so its
        // code comes first. A code's exchange said already; an implicit link's account is asked.
        // An answer that could not be had asks for nothing: the server still has the last word.
        let factor: string | null = null;
        if (pending.kind === "code") factor = await exchangedFactor(state);
        else if (authConfig() !== null) factor = await secondFactorFor(session).catch(() => null);
        if (cancelled) return;
        if (acted.current) {
          revokeLink(state);
          return;
        }
        if (factor !== null) {
          secondStep.current = { session, factorId: factor, then: "recovery" };
          enterSecondStep();
          return;
        }
      }
      const check: LinkCheck = renewal === "spent" ? { kind: "refused" } : await checkLink(session.accessToken);
      if (cancelled) return;
      if (acted.current) {
        // The player moved on before the answer came.
        revokeLink(state);
        return;
      }
      if (check.kind === "refused") {
        // A spent, expired or forged link. A renewal the server still refused is revoked; a link
        // whose own tokens were refused has nothing live to revoke.
        if (renewal === "renewed") revokeLink(state);
        else state.session = null;
        setLinkOutcome("none");
        setLinkError(true);
        setResendOffered(true);
        return;
      }
      if (check.kind === "unchecked") {
        if (link.kind === "recovery") {
          // Our server could not be reached, but the provider accepted this one-time link: keep
          // it on this screen so it can be checked again, rather than spend it on a network blip.
          setLinkOutcome("recoveryUnchecked");
          return;
        }
        revokeLink(state);
        setLinkOutcome("unchecked");
        setResendOffered(true);
        return;
      }
      const verified = check.email;
      if (link.kind === "session") {
        // R193: no link signs this browser in, the sign-up it started included (see A
        // CONFIRMATION link, above): the player signs in with their password.
        revokeLink(state);
        if (link.linkType === "invite") {
          setMode("forgot");
          setLinkOutcome("invited");
          return;
        }
        if (link.linkType === "email_change") {
          // R663: the change is made; like any link, it signs nobody in (R193).
          setLinkOutcome("emailChanged");
          return;
        }
        // The sign-up this browser was waiting on is confirmed: its "confirm first" hint is over.
        const pending = pendingEmail();
        if (verified !== null && pending !== null && sameAddress(verified, pending)) forgetPendingEmail();
        setLinkOutcome("confirmed");
        return;
      }
      if (verified === null) {
        // An account with no address has nothing to compare: nothing is held.
        revokeLink(state);
        setLinkOutcome("recoveryRefused");
        return;
      }
      // R193: a reset this browser asked for is held at once.
      const requested = pendingReset();
      if (requested !== null && sameAddress(verified, requested)) {
        const held = takeLink(state);
        if (held === null) return;
        holdRecoverySession(held, verified);
        navigate(paths.resetPassword, { replace: true });
        return;
      }
      // Asked for elsewhere (another device or browser, or before this browser signed in): held
      // only once the player types the address it was sent to (see A RECOVERY link, above).
      claimAddress.current = verified;
      setMode("claimReset");
      setLinkOutcome("recoveryClaim");
    };
    void run();
    return () => {
      cancelled = true;
      clearConsumedAuthRedirect();
    };
  }, [entry, checkRound]);

  /** R665: a sign-in held for its second step and not finished is revoked. */
  function releaseSecondStep(): void {
    const step = secondStep.current;
    secondStep.current = null;
    // A recovery link's session belongs to the link's state, which revokes it itself.
    if (step !== null && step.then === "signIn") void revokeSignedOutSession(step.session);
  }

  /** Keep a session as this browser's own and go on, as a password sign-in does. */
  function keepSession(session: Session): void {
    // Replaces (and revokes) any other session this browser held (R194).
    adoptSession(session);
    // To the main menu, whichever screen sent the player here (issue #479). The one gate in
    // `main.tsx` handles a pending account on any gated screen, so this file needs no notion of
    // account status. A fixed `paths` value, never a URL from the query (B35).
    navigate(paths.landing, { replace: true });
  }

  /** R665: the session is kept at once, or held for its authenticator code first. */
  function signInWith(session: Session, secondFactor: string | null): void {
    if (secondFactor === null) {
      keepSession(session);
      return;
    }
    secondStep.current = { session, factorId: secondFactor, then: "signIn" };
    enterSecondStep();
  }

  function enterSecondStep(): void {
    setMode("mfa");
    setCode("");
    setCodeError(null);
    setError(null);
    setNotice(null);
    setLinkOutcome("none");
  }

  /** R665: the authenticator code for the held session. */
  function submitSecondStep(): void {
    const step = secondStep.current;
    if (step === null) {
      switchMode("signIn");
      return;
    }
    const problem = requiredProblem(code, "code");
    setCodeError(problem);
    if (problem !== null) {
      document.getElementById("login-code")?.focus();
      return;
    }
    setBusy(true);
    verifySecondFactor(step.session, step.factorId, code)
      .then((raised) => {
        if (secondStep.current !== step) return;
        secondStep.current = null;
        if (step.then === "signIn") {
          keepSession(raised);
          return;
        }
        // The recovery link goes on being checked, now with a token the server accepts.
        linkState.current.session = raised;
        setMode("signIn");
        setLinkOutcome("checking");
        setCheckRound((round) => round + 1);
      })
      .catch((cause: unknown) => {
        setCodeError(messageOf(cause));
        document.getElementById("login-code")?.focus();
      })
      .finally(() => {
        setBusy(false);
      });
  }

  /** R664: mail the sign-in link and code (again). */
  function sendEmailCode(): void {
    const problem = emailProblem(email);
    setEmailError(problem);
    if (problem !== null) {
      document.getElementById("login-email")?.focus();
      return;
    }
    const address = normalizeEmail(email);
    if (codeCooldown.secondsFor(address) > 0) return;
    setBusy(true);
    setError(null);
    setNotice(null);
    requestEmailSignIn(address)
      .then(() => {
        // One sentence whatever the provider answered (R192); `requestEmailSignIn` resolves on all.
        setNotice(AUTH_NOTICES.emailCodeSent);
        setCodeSentTo(address);
        setCode("");
        setCodeError(null);
        codeCooldown.startUntil(address, deadlineAfter(AUTH_EMAIL_RESEND_COOLDOWN_SECONDS, Date.now()));
      })
      .catch((cause: unknown) => {
        setError(messageOf(cause));
      })
      .finally(() => {
        setBusy(false);
      });
  }

  /** R664: the typed sign-in code. */
  function submitEmailCode(address: string): void {
    const problem = requiredProblem(code, "code");
    setCodeError(problem);
    if (problem !== null) {
      document.getElementById("login-code")?.focus();
      return;
    }
    setBusy(true);
    setError(null);
    verifyEmailCode(address, code)
      .then((result) => {
        signInWith(result.session, result.secondFactor);
      })
      .catch((cause: unknown) => {
        setCodeError(messageOf(cause));
        document.getElementById("login-code")?.focus();
      })
      .finally(() => {
        setBusy(false);
      });
  }

  /** R666: leave for an OAuth provider. A refusal before leaving is said here. */
  function continueWith(provider: OAuthProvider): void {
    if (busy) return;
    acted.current = true;
    releaseLink();
    setBusy(true);
    setError(null);
    setOauthFailed(false);
    startOAuthSignIn(provider).catch((cause: unknown) => {
      setError(messageOf(cause));
      setBusy(false);
    });
  }

  /** A link this screen still answers for is let go (revoked) once the player moves on from it. */
  function releaseLink(): void {
    claimAddress.current = null;
    revokeLink(linkState.current);
  }

  function retryLinkCheck(): void {
    if (linkState.current.session === null) return;
    setLinkOutcome("checking");
    setCheckRound((round) => round + 1);
  }

  function switchMode(next: Mode): void {
    acted.current = true;
    releaseLink();
    releaseSecondStep();
    setCode("");
    setCodeError(null);
    setCodeSentTo(null);
    setOauthFailed(false);
    setMode(next);
    setError(null);
    setNotice(null);
    setEmailError(null);
    setPasswordError(null);
    setLinkError(false);
    setSessionExpired(false);
    setLinkOutcome("none");
    setConfirmFirst(false);
  }

  function onResend(): void {
    if (resendBusy || resendWait > 0) return;
    const problem = emailProblem(email);
    if (problem !== null) {
      setEmailError(problem);
      return;
    }
    const address = normalizeEmail(email);
    setResendBusy(true);
    setError(null);
    setNotice(null);
    resendConfirmation(address)
      .then(() => {
        // One sentence whatever the provider answered (R192); `resendConfirmation` resolves on all.
        setNotice(AUTH_NOTICES.resendSent);
        resendCooldown.startUntil(address, deadlineAfter(AUTH_EMAIL_RESEND_COOLDOWN_SECONDS, Date.now()));
      })
      .catch((cause: unknown) => {
        setError(messageOf(cause));
      })
      .finally(() => {
        setResendBusy(false);
      });
  }

  /**
   * `claimReset`: a recovery link asked for elsewhere is held only for the address it was sent to.
   * A typo can be corrected (the link's holder can read its address anyway); leaving revokes it.
   */
  function claimReset(): void {
    const expected = claimAddress.current;
    if (expected === null || linkState.current.session === null) {
      // Nothing left to claim: back to the plain sign-in form, letting go of whatever is left.
      switchMode("signIn");
      return;
    }
    const problem = emailProblem(email);
    const mismatch = problem === null && !sameAddress(normalizeEmail(email), expected);
    const nextEmailError = problem ?? (mismatch ? AUTH_NOTICES.recoveryClaimMismatch : null);
    setEmailError(nextEmailError);
    if (nextEmailError !== null) {
      document.getElementById("login-email")?.focus();
      return;
    }
    const held = takeLink(linkState.current);
    claimAddress.current = null;
    if (held === null) return;
    holdRecoverySession(held, expected);
    navigate(paths.resetPassword, { replace: true });
  }

  function onSubmit(event: FormEvent<HTMLFormElement>): void {
    event.preventDefault();
    if (busy) return;
    if (mode === "claimReset") {
      claimReset();
      return;
    }
    if (mode === "mfa") {
      submitSecondStep();
      return;
    }
    acted.current = true;
    releaseLink();
    if (linkOutcome === "checking") setLinkOutcome("none");
    setConfirmFirst(false);

    const address = normalizeEmail(email);
    let nextEmailError: string | null;
    let nextPasswordError: string | null = null;
    if (mode === "emailCode") {
      if (codeSentTo !== null && sameAddress(address, codeSentTo)) submitEmailCode(codeSentTo);
      else sendEmailCode();
      return;
    }
    if (mode === "signUp") {
      nextEmailError = emailProblem(email);
      nextPasswordError = newPasswordProblem(password);
    } else if (mode === "signIn") {
      // An existing password is whatever the account has: sign-in only asks for something typed.
      nextEmailError = requiredProblem(address, "email or username");
      nextPasswordError = requiredProblem(password, "password");
    } else {
      nextEmailError = emailProblem(email);
    }
    setEmailError(nextEmailError);
    setPasswordError(nextPasswordError);
    if (nextEmailError !== null || nextPasswordError !== null) {
      // The first field to fix, for a keyboard or a screen reader.
      document.getElementById(nextEmailError !== null ? "login-email" : "login-password")?.focus();
      return;
    }

    if (mode === "forgot" && resetWait > 0) return;

    setBusy(true);
    setError(null);
    setNotice(null);
    const done = (): void => {
      setBusy(false);
    };

    if (mode === "signUp") {
      signUp(address, password)
        .then(() => {
          // §9.4 step 1 requires a verified email before a code can be redeemed, so the next step
          // is the inbox, not the game. Said identically whether or not the address was already
          // registered -- see `signUp`'s note on why the provider makes that true for free.
          setMode("signIn");
          setPassword("");
          // The form now reads "Sign in", which is the step after the inbox: the notice says so,
          // above it, and takes focus so it is what is read next.
          focusNotice.current = true;
          setNotice(AUTH_NOTICES.signUpSent);
          setResendOffered(true);
          // The sign-up just mailed this address, which starts the provider's per-address
          // interval: a resend now would be refused, and reported as sent (R192).
          resendCooldown.startUntil(address, deadlineAfter(AUTH_EMAIL_RESEND_COOLDOWN_SECONDS, Date.now()));
        })
        .catch((cause: unknown) => {
          setError(messageOf(cause));
        })
        .finally(done);
      return;
    }

    if (mode === "forgot") {
      requestPasswordReset(address)
        .then(() => {
          // Known address or not, sent or refused: one sentence (OWASP, R192).
          setNotice(AUTH_NOTICES.resetSent);
          resetCooldown.startUntil(address, deadlineAfter(AUTH_EMAIL_RESEND_COOLDOWN_SECONDS, Date.now()));
        })
        .catch((cause: unknown) => {
          setError(messageOf(cause));
        })
        .finally(done);
      return;
    }

    // Read before the attempt: a successful sign-in forgets it.
    const justSignedUp = pendingEmail();
    // R1432: a username never holds @, so what was typed says which door it goes to (R1443).
    const byUsername = !address.includes("@");
    (byUsername ? signInWithUsername(address, password) : signIn(address, password))
      .then((result) => {
        // Kept at once, or (R665) once its authenticator code has been typed.
        signInWith(result.session, result.secondFactor);
      })
      .catch((cause: unknown) => {
        setError(messageOf(cause));
        // The resend link and the confirm-first hint below are for an address.
        if (byUsername) return;
        // An unconfirmed address reads as `credentials` (R160), so the way forward for it is
        // offered after any such refusal rather than only after a telling one. Not after a network
        // failure or a rate limit, which say nothing about the address and which a mailer would not
        // help (and, while rate-limited, would only add another request).
        if (cause instanceof AuthError && cause.failure === "credentials") setResendOffered(true);
        // The address this browser has just signed up with: most likely its confirmation link has
        // not been opened yet. Keyed on this browser's own state, never on the provider's answer.
        if (cause instanceof AuthError && cause.failure === "credentials" && justSignedUp !== null) {
          setConfirmFirst(sameAddress(address, justSignedUp));
        }
      })
      .finally(done);
  }

  const signingUp = mode === "signUp";
  const forgot = mode === "forgot";
  const signingIn = mode === "signIn";
  const claiming = mode === "claimReset";
  const emailCode = mode === "emailCode";
  const secondFactor = mode === "mfa";
  /** R664: the code was mailed to the address in the field, so the field to type it is shown. */
  const codeAsked = emailCode && codeSentTo !== null && sameAddress(normalizeEmail(email), codeSentTo);

  let title: string;
  if (signingUp) title = "Create an account";
  else if (forgot || claiming) title = "Reset your password";
  else if (emailCode) title = "Sign in with an email code";
  else if (secondFactor) title = "Two-step sign-in";
  else title = "Sign in";

  let submitLabel: string;
  if (signingUp) {
    submitLabel = busy ? "Creating…" : "Create account";
  } else if (forgot) {
    // "Send", not "Send again": the interval may have been started before a reload.
    // The wait is said once, in the line under the button (as the confirmation resend says it),
    // so the button keeps its name while it is locked (integration QA: the countdown was twice).
    submitLabel = busy ? "Sending…" : "Send reset link";
  } else if (claiming) {
    submitLabel = "Continue";
  } else if (emailCode) {
    if (codeAsked) submitLabel = busy ? "Signing in…" : "Sign in";
    else submitLabel = busy ? "Sending…" : "Email me a sign-in code";
  } else if (secondFactor) {
    submitLabel = busy ? "Checking…" : "Verify";
  } else {
    submitLabel = busy ? "Signing in…" : "Sign in";
  }

  const forgotButton = (
    <button
      type="button"
      className="link-button"
      data-testid={loginTestid.forgot}
      onClick={() => {
        switchMode("forgot");
      }}
    >
      {claiming ? "Ask for a new link instead" : "Forgot your password?"}
    </button>
  );

  const resendControls = (
    <div className="auth-resend">
      {/* R160 makes an unconfirmed address and a wrong password read alike, so the button says
          who it is for rather than hinting at which one this was. */}
      <span className="auth-hint" data-testid={loginTestid.resendLead}>
        Just signed up? Confirm your email first:
      </span>
      <button
        type="button"
        className="link-button"
        data-testid={loginTestid.resend}
        disabled={resendBusy || resendWait > 0}
        onClick={onResend}
      >
        {resendBusy ? "Sending…" : "Resend the confirmation email"}
      </button>
      {resendWait > 0 ? (
        <span className="auth-cooldown" data-testid={loginTestid.resendCooldown} data-seconds={resendWait}>
          You can send another in {resendWait} s.
        </span>
      ) : null}
    </div>
  );

  return (
    <div className="app-shell auth-screen tavern">
      <BackLink />
      {/* Two parts, so a phone held sideways can set them side by side (auth/tavern.css): what is
          going on (the title and every notice) beside the form. Elsewhere they are one column. */}
      <section className="panel panel--auth panel--split">
        <div className="auth-board__head">
          <div className="brand">
            <h1>JackiOh</h1>
          </div>
          <h2>{title}</h2>

          {secondFactor ? (
            <p className="notice" data-testid={loginTestid.notice} role="status">
              {AUTH_NOTICES.mfaRequired}
            </p>
          ) : null}

          {(signingIn || signingUp) && oauthFailed ? (
            <p className="notice" data-testid={loginTestid.oauthError} role="alert">
              {AUTH_MESSAGES.oauthFailed}
            </p>
          ) : null}

          {signingIn && sessionExpired ? (
            <p className="notice" data-testid={loginTestid.sessionExpired} role="status">
              {AUTH_NOTICES.sessionExpired}
            </p>
          ) : null}

          {signingIn && linkOutcome === "checking" ? (
            <p className="notice" data-testid={loginTestid.checkingLink} role="status">
              {AUTH_NOTICES.checkingLink}
            </p>
          ) : null}

          {signingIn && checkingLong ? (
            <p className="auth-hint" data-testid={loginTestid.checkingSlow}>
              {AUTH_NOTICES.checkingSlow}
            </p>
          ) : null}

          {signingIn && linkOutcome === "confirmed" ? (
            <p className="notice" data-testid={loginTestid.confirmed} role="status">
              {AUTH_NOTICES.emailConfirmed}
            </p>
          ) : null}

          {signingIn && linkOutcome === "emailChanged" ? (
            <p className="notice" data-testid={loginTestid.emailChanged} role="status">
              {AUTH_NOTICES.emailChanged}
            </p>
          ) : null}

          {forgot && linkOutcome === "invited" ? (
            <p className="notice" data-testid={loginTestid.invited} role="status">
              {AUTH_NOTICES.inviteNeedsPassword}
            </p>
          ) : null}

          {claiming && linkOutcome === "recoveryClaim" ? (
            <p className="notice" data-testid={loginTestid.recoveryClaim} role="status">
              {AUTH_NOTICES.recoveryClaim}
            </p>
          ) : null}

          {signingIn && linkOutcome === "recoveryRefused" ? (
            <div className="auth-link-error">
              <p className="notice" data-testid={loginTestid.recoveryRefused} role="alert">
                {AUTH_NOTICES.recoveryElsewhere}
              </p>
              <div className="auth-actions">{forgotButton}</div>
            </div>
          ) : null}

          {signingIn && linkOutcome === "recoveryUnchecked" ? (
            <div className="auth-link-error">
              <p className="notice" data-testid={loginTestid.recoveryUnchecked} role="alert">
                {AUTH_NOTICES.recoveryUnchecked}
              </p>
              <div className="auth-actions">
                <button type="button" data-testid={loginTestid.linkRetry} onClick={retryLinkCheck}>
                  Try again
                </button>
                {forgotButton}
              </div>
            </div>
          ) : null}

          {signingIn && linkOutcome === "unchecked" ? (
            <div className="auth-link-error">
              <p className="notice" data-testid={loginTestid.linkUnchecked} role="alert">
                {AUTH_NOTICES.linkUnchecked}
              </p>
              <div className="auth-actions">
                {resendControls}
                {forgotButton}
              </div>
            </div>
          ) : null}

          {signingIn && linkError ? (
            <div className="auth-link-error">
              {/* Our sentence only: the link's `error_description` is never read (R193). */}
              <p className="notice" data-testid={loginTestid.linkError} role="alert">
                {AUTH_MESSAGES.linkExpired}
              </p>
              {/* Mail scanners open links before the player does: a confirmation link they spent
                  has still confirmed the address, and signing in is all that is left. */}
              <p className="auth-hint" data-testid={loginTestid.linkErrorSignIn}>
                If you already confirmed your email, just sign in below.
              </p>
              <div className="auth-actions">
                {resendControls}
                {forgotButton}
              </div>
            </div>
          ) : null}

          {notice !== null ? (
            <p className="notice" data-testid={loginTestid.notice} role="status" ref={noticeRef} tabIndex={-1}>
              {notice}
            </p>
          ) : null}

          {error !== null ? (
            <p className="notice" data-testid={loginTestid.error} role="alert">
              {error}
            </p>
          ) : null}

          {signingIn && error !== null && confirmFirst ? (
            <p className="auth-hint" data-testid={loginTestid.confirmFirst}>
              {AUTH_NOTICES.confirmFirst}
            </p>
          ) : null}

          {signedInAs !== null && (signingIn || signingUp) ? (
            // Never silently: a sign-in here replaces (and revokes) the session this browser holds.
            <div className="auth-signed-in" data-testid={loginTestid.signedInAs}>
              <p className="auth-hint">
                {signedInAs === "" ? (
                  "This browser is already signed in to an account."
                ) : (
                  <>
                    This browser is signed in as{" "}
                    <strong>
                      <Address value={signedInAs} />
                    </strong>
                    .
                  </>
                )}{" "}
                {signingUp ? "Creating an account does not change that." : "Signing in here signs it out of that account."}
              </p>
              <div className="auth-actions">
                <a
                  className="link-button"
                  href={paths.account}
                  data-testid={loginTestid.signedInAccount}
                  onClick={followInApp(paths.account)}
                >
                  Go to that account
                </a>
                <button
                  type="button"
                  className="link-button"
                  data-testid={loginTestid.signedInSignOut}
                  disabled={leaving}
                  onClick={() => {
                    signOut();
                  }}
                >
                  {signOutLabel(leaving)}
                </button>
              </div>
            </div>
          ) : null}

          {forgot ? (
            <p className="auth-hint">
              Enter the address you signed up with. If it has an account, we&rsquo;ll email a link to
              choose a new password.
            </p>
          ) : null}
          {signingUp ? (
            // Said before the player has an account, not after they have confirmed one.
            <p className="auth-hint" data-testid={loginTestid.inviteOnly}>
              {AUTH_NOTICES.inviteOnly}
            </p>
          ) : null}
        </div>

        <div className="auth-board__body">
          <form
            className="form-card"
            data-testid={loginTestid.form}
            data-mode={mode}
            onSubmit={onSubmit}
            // Our own messages, next to the field, rather than the browser's bubble.
            noValidate
          >
            {secondFactor ? null : (
              <>
                <label htmlFor="login-email">
                  {claiming ? "Your account's email" : signingIn ? "Email or username" : "Email"}
                </label>
                <input
                  id="login-email"
                  data-testid={loginTestid.email}
                  // R1443: sign-in takes a username too, which an email field would refuse.
                  type={signingIn ? "text" : "email"}
                  autoComplete="username"
                  autoCapitalize="none"
                  value={email}
                  aria-invalid={emailError !== null}
                  aria-describedby={emailError !== null ? "login-email-error" : undefined}
                  onChange={(event) => {
                    setEmail(event.target.value);
                    setEmailError(null);
                  }}
                />
                {emailError !== null ? (
                  <p id="login-email-error" className="auth-field-error" data-testid={loginTestid.emailError}>
                    {emailError}
                  </p>
                ) : null}
              </>
            )}

            {codeAsked || secondFactor ? (
              <>
                <label htmlFor="login-code">
                  {secondFactor ? "Code from your authenticator app" : "Code from the email"}
                </label>
                <input
                  id="login-code"
                  data-testid={loginTestid.code}
                  type="text"
                  inputMode="numeric"
                  autoComplete="one-time-code"
                  autoCapitalize="none"
                  autoCorrect="off"
                  spellCheck={false}
                  // Focused when the step opens: it is the one thing to do there.
                  autoFocus={secondFactor}
                  value={code}
                  aria-invalid={codeError !== null}
                  aria-describedby={codeError !== null ? "login-code-error" : undefined}
                  onChange={(event) => {
                    setCode(event.target.value);
                    setCodeError(null);
                  }}
                />
                {codeError !== null ? (
                  <p id="login-code-error" className="auth-field-error" data-testid={loginTestid.codeError}>
                    {codeError}
                  </p>
                ) : null}
              </>
            ) : null}

            {signingIn || signingUp ? (
              <>
                <label htmlFor="login-password">Password</label>
                <div className="input-with-affix">
                  <input
                    id="login-password"
                    data-testid={loginTestid.password}
                    // The whole point of the toggle: a password you cannot read is a password you
                    // cannot check before submitting, which matters most while CREATING one.
                    type={showPassword ? "text" : "password"}
                    autoComplete={signingUp ? "new-password" : "current-password"}
                    // Shown as text, a phone keyboard would otherwise capitalise, correct and
                    // spell-check (remotely, with enhanced spell-check) the password.
                    autoCapitalize="none"
                    autoCorrect="off"
                    spellCheck={false}
                    value={password}
                    aria-invalid={passwordError !== null}
                    aria-describedby={passwordError !== null ? "login-password-error" : undefined}
                    onChange={(event) => {
                      setPassword(event.target.value);
                      setPasswordError(null);
                    }}
                  />
                  <button
                    type="button"
                    className="affix-button"
                    data-testid={loginTestid.togglePassword}
                    // Announced, not just drawn: the icon alone tells a screen reader nothing.
                    aria-label={showPassword ? "Hide password" : "Show password"}
                    aria-pressed={showPassword}
                    onClick={() => {
                      setShowPassword((shown) => !shown);
                    }}
                  >
                    {showPassword ? "Hide" : "Show"}
                  </button>
                </div>
                {passwordError !== null ? (
                  <p
                    id="login-password-error"
                    className="auth-field-error"
                    data-testid={loginTestid.passwordError}
                  >
                    {passwordError}
                  </p>
                ) : null}
              </>
            ) : null}

            <button
              type="submit"
              data-testid={loginTestid.submit}
              disabled={busy || (forgot && resetWait > 0) || (emailCode && !codeAsked && codeWait > 0)}
            >
              {submitLabel}
            </button>
            {codeAsked ? (
              <div className="auth-resend">
                <button
                  type="button"
                  className="link-button"
                  data-testid={loginTestid.codeSend}
                  disabled={busy || codeWait > 0}
                  onClick={sendEmailCode}
                >
                  Send a new code
                </button>
              </div>
            ) : null}
            {emailCode && codeWait > 0 ? (
              // Only the wait: whether a mail went out is the neutral notice's to say (R192).
              <p className="auth-hint" data-testid={loginTestid.codeCooldown} data-seconds={codeWait}>
                You can ask for another in {codeWait} s.
              </p>
            ) : null}
            {signingUp ? (
              <p className="auth-hint" data-testid={signUpPrivacyTestid}>
                By creating an account you agree to the{" "}
                <a href={paths.terms} onClick={followInApp(paths.terms)}>
                  Terms
                </a>{" "}
                and{" "}
                <a href={paths.privacy} onClick={followInApp(paths.privacy)}>
                  Privacy Policy
                </a>
                .
              </p>
            ) : null}
            {forgot && resetWait > 0 ? (
              // Only the wait: whether a mail went out is the neutral notice's to say (R192).
              <p className="auth-hint" data-testid={loginTestid.resetCooldown} data-seconds={resetWait}>
                You can ask for another in {resetWait} s.
              </p>
            ) : null}
          </form>

          {(signingIn || signingUp) && providers.length > 0 ? (
            // R666: only the providers this build names, so none appears before it is set up.
            <div className="auth-oauth" role="group" aria-label="Other ways to sign in">
              <p className="auth-hint">Or continue with</p>
              <div className="auth-actions">
                {providers.map((provider) => (
                  <button
                    key={provider}
                    type="button"
                    className="button-secondary"
                    data-testid={loginTestid.oauth}
                    data-provider={provider}
                    disabled={busy}
                    onClick={() => {
                      continueWith(provider);
                    }}
                  >
                    {OAUTH_PROVIDER_LABELS[provider]}
                  </button>
                ))}
              </div>
            </div>
          ) : null}

          <div className="auth-links">
            {signingIn ? (
              <button
                type="button"
                className="link-button"
                data-testid={loginTestid.emailCodeStart}
                onClick={() => {
                  switchMode("emailCode");
                }}
              >
                Sign in with an email code instead
              </button>
            ) : null}
            {(signingIn &&
              !linkError &&
              linkOutcome !== "recoveryRefused" &&
              linkOutcome !== "unchecked" &&
              linkOutcome !== "recoveryUnchecked") ||
            claiming
              ? forgotButton
              : null}
            {forgot || claiming || emailCode || secondFactor ? (
              <button
                type="button"
                className="link-button"
                data-testid={secondFactor ? loginTestid.mfaCancel : loginTestid.backToSignIn}
                onClick={() => {
                  switchMode("signIn");
                }}
              >
                {secondFactor ? "Cancel and sign in again" : emailCode ? "Sign in with a password" : "Back to sign in"}
              </button>
            ) : (
              <button
                type="button"
                className="link-button"
                data-testid={loginTestid.mode}
                onClick={() => {
                  switchMode(signingUp ? "signIn" : "signUp");
                }}
              >
                {signingUp ? "Already have an account? Sign in" : "No account? Create one"}
              </button>
            )}
          </div>

          {signingIn && resendOffered && !linkError && linkOutcome !== "unchecked" ? resendControls : null}
        </div>
      </section>

      <SiteFooter />
    </div>
  );
}
