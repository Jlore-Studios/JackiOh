// `/invite` — the pending-account code screen (SPEC §9.4, BUILD M6-T1).
// R145/§9.8: render code and account refusals as received; identical code errors are a security property.
// R192/R109/R194: server-provided waits/messages, renew/retry once, and retain known status after other failures.
// R191 reads input without guessing; waits recheck after expiry or wake; config is R79/R104 (CLAUDE.md rule 9).

import { useEffect, useRef, useState } from "react";

import { readCodeInput } from "@jackioh/shared";

import {
  AUTH_EMAIL_RESEND_COOLDOWN_SECONDS,
  CODE_ATTEMPT_WINDOW_SECONDS,
  CODE_STATUS_RECHECK_FLOOR_SECONDS,
} from "@jackioh/server-config";
import Address from "../auth/Address.tsx";
import CodeField from "../auth/CodeField.tsx";
import { deadlineAfter, useAddressCooldown, useSecondsUntil } from "../auth/cooldown.ts";
import { INVITE_CODE_FORMAT, attemptsText, waitInWords } from "../auth/codeInput.ts";
import { inviteTestid } from "../auth/testids.ts";
import {
  ApiRequestError,
  getCodeStatus,
  getMe,
  redeemCode,
  retryAfterMsOf,
  type CodeStatusResponse,
  type MeResponse,
} from "../net/api.ts";
import { AUTH_MESSAGES, AUTH_NOTICES, AuthError, resendConfirmation } from "../net/auth.ts";
import { announceAccountChange, callWithRenewal, useAccount, type Account } from "../net/gate.ts";
import { loginPath, navigate, paths } from "../net/navigate.ts";
import { pendingEmailEntry } from "../net/session.ts";
import { signOut, signOutLabel, useSigningOut } from "./account.tsx";
import { BackLink, followInApp } from "./nav.tsx";

import "../auth/tavern.css";

export { INVITE_CODE_GROUPS, INVITE_CODE_PLACEHOLDER } from "../auth/codeInput.ts";

export const INVITE_CODE_INPUT = inviteTestid.input;
export const INVITE_SUBMIT = inviteTestid.submit;
export const INVITE_ERROR = inviteTestid.error;
/** §9.4's circuit breaker is open: the screen says so instead of guessing after a 503. */
export const INVITE_PAUSED = inviteTestid.paused;
/** An active account reached the code screen; redemption is not for it. */
export const INVITE_NOT_NEEDED = inviteTestid.notNeeded;

/** What the input shows for `raw`: R191's reading, grouped and separated per §9.4. */
export function formatInviteCode(raw: string): string {
  return readCodeInput(raw, INVITE_CODE_FORMAT).formatted;
}

/** `setTimeout`'s own ceiling (a signed 32-bit millisecond count); a longer wait fires at once. */
const LONGEST_TIMER_MS = 2_147_483_647;
/** Unit conversions, not configuration. */
const MS_PER_SECOND = 1000;

/**
 * A rate-limited refusal (R192): the wait it stated (kept for `data-retry-after-ms`), and the
 * deadline that wait makes on the clock, or null for both when it stated none.
 */
type RateLimit = { retryAfterMs: number | null; until: number | null };

function isRateLimited(cause: unknown): cause is ApiRequestError {
  return cause instanceof ApiRequestError && (cause.status === 429 || cause.code === "rate_limited");
}

function isPausedRefusal(cause: unknown): boolean {
  return cause instanceof ApiRequestError && cause.status === 503 && cause.code === "unavailable";
}

/** Names both account and network without revealing which limit refused; the server error says what happened. */
function waitText(leftMs: number | null): string {
  if (leftMs === null) return "This account or network can try again in a few minutes.";
  return `This account or network can try again in ${waitInWords(leftMs)}.`;
}

type StatusRead = { status: CodeStatusResponse; at: number };

function statusDeadlines(read: StatusRead | null): { pausedUntil: number | null; triesBackAt: number | null } {
  if (read === null) return { pausedUntil: null, triesBackAt: null };
  const { status, at } = read;
  const pausedUntil = status.redemptionEnabled ? null : at + Math.max(0, status.retryAfterMs);
  const out = status.attemptsRemaining !== undefined && status.attemptsRemaining <= 0;
  // A server from before R192's wait sends none: the whole window is the upper bound.
  const triesBackAt = out
    ? at + Math.max(0, status.attemptsRetryAfterMs ?? CODE_ATTEMPT_WINDOW_SECONDS * MS_PER_SECOND)
    : null;
  return { pausedUntil, triesBackAt };
}

/**
 * When to read the status again: the soonest deadline it stated, but never sooner than
 * `CODE_STATUS_RECHECK_FLOOR_SECONDS` after it was read, or null when it stated none that matters.
 */
function recheckAt(read: StatusRead | null): number | null {
  const { pausedUntil, triesBackAt } = statusDeadlines(read);
  const due = [pausedUntil, triesBackAt].filter((deadline): deadline is number => deadline !== null);
  if (read === null || due.length === 0) return null;
  return Math.max(Math.min(...due), read.at + CODE_STATUS_RECHECK_FLOOR_SECONDS * MS_PER_SECOND);
}

function msUntil(deadline: number | null, now: number): number | null {
  return deadline === null ? null : Math.max(0, deadline - now);
}

/** Runs when a deadline passes or the page wakes; retry is throttled so a failed re-read is not final. */
function useWhenDue(deadline: number | null, onDue: () => void): void {
  const latest = useRef(onDue);
  latest.current = onDue;
  const lastRun = useRef<number | null>(null);

  useEffect(() => {
    if (deadline === null) return;
    let timer: number | undefined;
    const check = (): void => {
      window.clearTimeout(timer);
      const now = Date.now();
      if (now < deadline) {
        timer = window.setTimeout(check, Math.min(deadline - now, LONGEST_TIMER_MS));
        return;
      }
      if (lastRun.current !== null && now - lastRun.current < CODE_STATUS_RECHECK_FLOOR_SECONDS * MS_PER_SECOND) return;
      lastRun.current = now;
      latest.current();
    };
    const onWake = (): void => {
      if (document.visibilityState !== "hidden") check();
    };
    check();
    document.addEventListener("visibilitychange", onWake);
    window.addEventListener("pageshow", onWake);
    window.addEventListener("focus", onWake);
    return () => {
      window.clearTimeout(timer);
      document.removeEventListener("visibilitychange", onWake);
      window.removeEventListener("pageshow", onWake);
      window.removeEventListener("focus", onWake);
    };
  }, [deadline]);
}

type EmailCheck = "idle" | "checking" | "unchanged" | "failed";

export type InviteRouteProps = {
  account?: { token: string; me: MeResponse };
};

export default function InviteRoute({ account }: InviteRouteProps = {}) {
  // A changed account gets a fresh screen.
  if (account !== undefined) {
    return (
      <InviteScreen
        key={account.me.profile.id}
        account={{ kind: "ready", token: account.token, me: account.me }}
      />
    );
  }
  return <StandaloneInvite />;
}

function StandaloneInvite() {
  const account = useAccount();
  // Do not redraw a screen already in use when its first account read arrives.
  const readyId = account.kind === "ready" ? account.me.profile.id : null;
  const [seen, setSeen] = useState<{ id: string | null; generation: number }>({ id: null, generation: 0 });
  if (readyId !== null && readyId !== seen.id) {
    setSeen({ id: readyId, generation: seen.id === null ? seen.generation : seen.generation + 1 });
  }
  return <InviteScreen key={seen.generation} account={account} />;
}

/** R145's code-dependent refusal: the one sentence for a missing, expired or exhausted code. */
function isCodeRefusal(cause: unknown): boolean {
  return cause instanceof ApiRequestError && cause.code === "invalid_code";
}

/** §9.4 step 1: the account's email is not confirmed (or the server could not tell that it is). */
function isUnverifiedRefusal(cause: unknown): boolean {
  return cause instanceof ApiRequestError && cause.code === "email_unverified";
}

function runningResend(): { address: string; deadline: number } | null {
  const entry = pendingEmailEntry();
  if (entry === null) return null;
  const deadline = deadlineAfter(AUTH_EMAIL_RESEND_COOLDOWN_SECONDS, entry.at);
  return deadline > Date.now() ? { address: entry.address, deadline } : null;
}

function InviteScreen({ account }: { account: Account }) {
  const [code, setCode] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [statusRead, setStatusRead] = useState<StatusRead | null>(null);
  const status = statusRead?.status ?? null;
  const [submitting, setSubmitting] = useState(false);
  const [rateLimit, setRateLimit] = useState<RateLimit | null>(null);
  /** Bumped after every refusal, so the attempts count is read again (B21). */
  const [statusReads, setStatusReads] = useState(0);
  /** The code R145's error just refused: Redeem stays off while the field still holds it. */
  const [refusedCode, setRefusedCode] = useState<string | null>(null);
  /** §9.4 step 1 refused a redemption for this account's unconfirmed email. */
  const [unverifiedRefusal, setUnverifiedRefusal] = useState(false);
  const [resendBusy, setResendBusy] = useState(false);
  const [resendNotice, setResendNotice] = useState<string | null>(null);
  const [initialResend] = useState(runningResend);
  const resendCooldown = useAddressCooldown(initialResend);
  const leaving = useSigningOut();
  const [emailCheck, setEmailCheck] = useState<EmailCheck>("idle");
  const [confirmedHere, setConfirmedHere] = useState(false);
  const codeRef = useRef(code);
  codeRef.current = code;
  /** Set when a refusal should put the caret back in the field once it is unlocked. */
  const focusField = useRef(false);
  const [redeemed, setRedeemed] = useState(false);
  const redeemedRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (redeemed) redeemedRef.current?.focus();
  }, [redeemed]);

  const me = account.kind === "ready" ? account.me : null;
  useEffect(() => {
    setUnverifiedRefusal(false);
    setConfirmedHere(false);
    setEmailCheck((check) => (check === "checking" ? check : "idle"));
  }, [me]);

  useEffect(() => {
    if (submitting || !focusField.current) return;
    focusField.current = false;
    document.getElementById(INVITE_CODE_INPUT)?.focus();
  }, [submitting]);

  useEffect(() => {
    if (account.kind !== "anonymous") return;
    navigate(account.reason === "expired" ? loginPath({ reason: "expired" }) : loginPath(), {
      replace: true,
    });
  }, [account]);

  const token = account.kind === "ready" ? account.token : null;

  useEffect(() => {
    if (token === null) return;
    let cancelled = false;
    // Status-read failure retains known status; an unauthorised read renews once (R194).
    callWithRenewal(token, getCodeStatus)
      .then((outcome) => {
        if (cancelled) return;
        if (outcome.kind === "ok") setStatusRead({ status: outcome.value, at: Date.now() });
        else if (outcome.kind === "signedOut") {
          navigate(outcome.expired ? loginPath({ reason: "expired" }) : loginPath(), { replace: true });
        }
      })
      .catch(() => undefined);
    return () => {
      cancelled = true;
    };
  }, [token, statusReads]);

  // Re-read status when its stated wait expires.
  useWhenDue(recheckAt(statusRead), () => {
    setStatusReads((count) => count + 1);
  });

  useWhenDue(rateLimit?.until ?? null, () => {
    setRateLimit(null);
    setError(null);
    setStatusReads((count) => count + 1);
  });

  const { pausedUntil, triesBackAt } = statusDeadlines(statusRead);
  const lastDeadline = Math.max(pausedUntil ?? 0, triesBackAt ?? 0, rateLimit?.until ?? 0);
  useSecondsUntil(lastDeadline > 0 ? lastDeadline : null);
  const now = Date.now();
  const pausedLeftMs = msUntil(pausedUntil, now);
  const triesBackInMs = msUntil(triesBackAt, now);
  const rateLimitLeftMs = msUntil(rateLimit?.until ?? null, now);

  const complete = readCodeInput(code, INVITE_CODE_FORMAT).complete;
  const paused = status !== null && !status.redemptionEnabled;
  const active = account.kind === "ready" && !account.me.needsInviteCode;
  const done = active || redeemed;
  const attemptsRemaining = status?.attemptsRemaining;
  const outOfAttempts = attemptsRemaining !== undefined && attemptsRemaining <= 0;
  const refusedAgain = refusedCode !== null && code === refusedCode;
  const canSubmit =
    token !== null &&
    complete &&
    !paused &&
    !submitting &&
    !outOfAttempts &&
    rateLimit === null &&
    !done &&
    !refusedAgain;
  const email = me?.email ?? null;
  const unverified =
    me !== null && me.needsInviteCode && ((!me.emailVerified && !confirmedHere) || unverifiedRefusal);
  const resendWait = email === null ? 0 : resendCooldown.secondsFor(email);

  /** A confirmed address announces a fresh gate read; a failed check leaves this screen intact. */
  function checkAgain(): void {
    if (token === null || emailCheck === "checking") return;
    setEmailCheck("checking");
    callWithRenewal(token, getMe)
      .then((outcome) => {
        if (outcome.kind === "signedOut") {
          navigate(outcome.expired ? loginPath({ reason: "expired" }) : loginPath(), { replace: true });
          return;
        }
        if (outcome.kind === "switched") {
          setEmailCheck("idle");
          return;
        }
        if (outcome.value.emailVerified) {
          setConfirmedHere(true);
          setUnverifiedRefusal(false);
          setEmailCheck("idle");
          announceAccountChange();
          return;
        }
        setEmailCheck("unchanged");
      })
      .catch(() => {
        setEmailCheck("failed");
      });
  }

  /** R194 renews an unauthorised token once; never resend after sign-out or account switch. */
  async function redeemWithRenewal(bearer: string, sent: string): Promise<"redeemed" | "stopped"> {
    const outcome = await callWithRenewal(bearer, (token) => redeemCode(token, sent));
    if (outcome.kind === "ok") return "redeemed";
    if (outcome.kind === "signedOut") {
      navigate(outcome.expired ? loginPath({ reason: "expired" }) : loginPath(), { replace: true });
    }
    return "stopped";
  }

  function submit(): void {
    if (token === null || !canSubmit) return;
    const sent = code;
    setSubmitting(true);
    setError(null);
    redeemWithRenewal(token, sent)
      .then((outcome) => {
        if (outcome === "redeemed") setRedeemed(true);
      })
      .catch((cause: unknown) => {
        if (isCodeRefusal(cause)) {
          // Never show a refusal under a different code.
          if (codeRef.current !== sent) {
            setStatusReads((count) => count + 1);
            return;
          }
          setRefusedCode(sent);
          focusField.current = true;
        }
        if (isUnverifiedRefusal(cause)) setUnverifiedRefusal(true);
        // R145/R192 distinctions live in the server's message; only the adjacent wait is ours.
        setError(cause instanceof Error ? cause.message : String(cause));
        if (isRateLimited(cause)) {
          const wait = retryAfterMsOf(cause);
          setRateLimit({ retryAfterMs: wait, until: wait === null ? null : Date.now() + wait });
        }
        if (isPausedRefusal(cause)) {
          setStatusRead((read) => ({
            status: { ...(read?.status ?? { retryAfterMs: 0 }), redemptionEnabled: false, retryAfterMs: 0 },
            at: Date.now(),
          }));
        }
        if (cause instanceof ApiRequestError && cause.status === 409) announceAccountChange();
        setStatusReads((count) => count + 1);
      })
      .finally(() => {
        setSubmitting(false);
      });
  }

  function resend(): void {
    if (email === null || resendBusy || resendWait > 0) return;
    const address = email;
    setResendBusy(true);
    setResendNotice(null);
    resendConfirmation(address)
      .then(() => {
        setResendNotice(AUTH_NOTICES.resendSent);
        resendCooldown.startUntil(address, deadlineAfter(AUTH_EMAIL_RESEND_COOLDOWN_SECONDS, Date.now()));
      })
      .catch((cause: unknown) => {
        setResendNotice(cause instanceof AuthError ? cause.message : AUTH_MESSAGES.service);
      })
      .finally(() => {
        setResendBusy(false);
      });
  }

  const helpId = `${INVITE_CODE_INPUT}-help`;

  return (
    <div className="app-shell invite-screen tavern">
      <div className="invite-screen__top">
        <BackLink>
          <div className="invite-screen__account">
            {account.kind === "ready" ? (
              <span className="invite-screen__who">
                Signed in as{" "}
                <span className="invite-screen__email" data-testid={inviteTestid.accountEmail}>
                  {email ?? "an account with no email address"}
                </span>
              </span>
            ) : null}
            <button
              type="button"
              className="link-button"
              data-testid={inviteTestid.signOut}
              disabled={leaving}
              aria-busy={leaving}
              onClick={() => {
                signOut();
              }}
            >
              {signOutLabel(leaving)}
            </button>
          </div>
        </BackLink>
      </div>

      {/* Two parts, so a phone held sideways can set them side by side (auth/tavern.css): the title
          and what stands in the way beside the code field. Elsewhere they are one column. */}
      <section className="panel panel--auth panel--split">
        <div className="auth-board__head">
          <div className="brand">
            <h1>JackiOh</h1>
          </div>
          <h2>{redeemed ? "You\u2019re in" : active ? "You\u2019re all set" : "Enter your invite code"}</h2>

          {redeemed ? (
            <div className="notice" data-testid={inviteTestid.redeemed} role="status" tabIndex={-1} ref={redeemedRef}>
              <p>Your invite code worked: online play is open. Build your decks, then find a match.</p>
              <a
                className="button-primary"
                href={paths.decks}
                data-testid={inviteTestid.goToDecks}
                onClick={followInApp(paths.decks)}
              >
                Build your decks
              </a>
            </div>
          ) : null}

          {account.kind === "error" ? (
            <div className="notice" role="alert">
              <p>{account.message}</p>
              <button
                type="button"
                onClick={() => {
                  account.retry?.();
                }}
              >
                Try again
              </button>
            </div>
          ) : null}

          {active && !redeemed ? (
            <div className="notice">
              <p data-testid={INVITE_NOT_NEEDED}>This account is already active and needs no invite code.</p>
              <a
                className="button-primary"
                href={paths.decks}
                data-testid={inviteTestid.goToDecks}
                onClick={followInApp(paths.decks)}
              >
                Go to your decks
              </a>
            </div>
          ) : null}

          {paused && !done ? (
            <p
              className="notice"
              data-testid={INVITE_PAUSED}
              data-retry-after-ms={String(status?.retryAfterMs ?? 0)}
            >
              {pausedLeftMs !== null && pausedLeftMs > 0
                ? `Invite code redemption is paused. Try again in ${waitInWords(pausedLeftMs)}.`
                : "Invite code redemption is paused. Please try again later."}
            </p>
          ) : null}

          {unverified && !done ? (
            <div className="notice invite-screen__unverified" data-testid={inviteTestid.unverified} role="status">
              <p>
                Confirm your email address first: open the link we emailed to{" "}
                <strong>{email === null ? "your address" : <Address value={email} />}</strong>, then check again here. If you already have,
                check again in a moment.
              </p>
              <div className="invite-screen__actions">
                <button
                  type="button"
                  data-testid={inviteTestid.checkAgain}
                  disabled={emailCheck === "checking"}
                  aria-busy={emailCheck === "checking"}
                  onClick={checkAgain}
                >
                  {emailCheck === "checking" ? "Checking…" : "Check again"}
                </button>
                <button
                  type="button"
                  className="link-button"
                  data-testid={inviteTestid.resend}
                  disabled={email === null || resendBusy || resendWait > 0}
                  onClick={resend}
                >
                  {resendBusy
                    ? "Sending…"
                    : resendWait > 0
                      ? `Send again in ${String(resendWait)} s`
                      : "Resend the confirmation email"}
                </button>
              </div>
              {emailCheck === "unchanged" || emailCheck === "failed" ? (
                <p data-testid={inviteTestid.checkResult} data-result={emailCheck} role="status">
                  {emailCheck === "unchanged"
                    ? "Still not confirmed. Open the link we emailed, then check again."
                    : "Couldn’t check just now. Try again in a moment."}
                </p>
              ) : null}
              {resendNotice === null ? null : <p data-testid={inviteTestid.resendNotice}>{resendNotice}</p>}
            </div>
          ) : null}
        </div>

        <div className="auth-board__body">
          {done ? null : (
            <form
              className="form-card invite-screen__form"
              noValidate
              onSubmit={(event) => {
                event.preventDefault();
                submit();
              }}
            >
              {/* Named for a screen reader; on screen the title right above already says it. */}
              <label className="invite-screen__label" htmlFor={INVITE_CODE_INPUT}>
                Invite code
              </label>
              <CodeField
                id={INVITE_CODE_INPUT}
                value={code}
                describedBy={helpId}
                // Locked while a code is being redeemed, so the answer is about the code on screen.
                disabled={submitting}
                onChange={(next) => {
                  setCode(next.formatted);
                  // The refusal was about the code as it was; a rate limit's wait still stands.
                  if (rateLimit === null) setError(null);
                  // A rate limit that stated no wait lasts until the player changes the code; the
                  // server still decides, this only stops the button looking dead forever.
                  if (rateLimit !== null && rateLimit.retryAfterMs === null) {
                    setRateLimit(null);
                    setError(null);
                  }
                }}
              />
              {/* One short line between the field and Redeem: the four groups and the count under
                  them already show the shape, so this only says what a paste may carry. */}
              <p className="invite-screen__help" id={helpId} data-testid={inviteTestid.help}>
                Paste it as is: case and dashes don&rsquo;t matter.
              </p>

              {attemptsRemaining === undefined || rateLimit !== null ? null : (
                <p
                  className="invite-screen__attempts"
                  data-testid={inviteTestid.attempts}
                  data-remaining={String(attemptsRemaining)}
                >
                  {attemptsText(
                    attemptsRemaining,
                    // A wait only when the server stated one; the window's upper bound is not said.
                    outOfAttempts && status?.attemptsRetryAfterMs !== undefined ? triesBackInMs : null,
                  )}
                </p>
              )}

              <button
                type="submit"
                data-testid={INVITE_SUBMIT}
                disabled={!canSubmit}
                aria-busy={submitting}
              >
                {submitting ? "Redeeming…" : "Redeem"}
              </button>
            </form>
          )}

          {error === null && rateLimit === null ? null : (
            // One message: what the server said, then (for a rate limit) how long to wait.
            <div className="invite-screen__refusal">
              {error === null ? null : (
                <p className="notice" data-testid={INVITE_ERROR} role="alert">
                  {error}
                </p>
              )}
              {rateLimit === null ? null : (
                <div
                  className="notice notice--error invite-screen__wait"
                  data-testid={inviteTestid.rateLimited}
                  {...(rateLimit.retryAfterMs === null
                    ? {}
                    : { "data-retry-after-ms": String(rateLimit.retryAfterMs) })}
                >
                  {waitText(rateLimitLeftMs)}
                </div>
              )}
            </div>
          )}

          {refusedAgain ? (
            // The client's own sentence beside R145's, the same for every refused code: it says
            // nothing about which of the three kinds of failure this was.
            <p className="invite-screen__help" data-testid={inviteTestid.refused}>
              Change the code to try again.
            </p>
          ) : null}

          {done ? null : (
            // For whoever has no code yet: under Redeem, set apart like the sign-in board's links,
            // so it never stands between the field and the button.
            <p className="invite-screen__where" data-testid={inviteTestid.whereFrom}>
              No code yet? Online play is invite-only for now, and the JackiOh team hands out the
              codes. In the meantime,{" "}
              <a href={paths.practice} data-testid={inviteTestid.playAi} onClick={followInApp(paths.practice)}>
                play vs AI
              </a>
              , which needs no code.
            </p>
          )}
        </div>
      </section>
    </div>
  );
}
